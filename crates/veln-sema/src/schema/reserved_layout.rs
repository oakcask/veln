use veln_ast::SchemaDecl;

use super::primitives::{
    canonical_schema_primitive_is, exact_width_schema_primitive_bit_width,
    exact_width_schema_primitive_little_endian, reserved_bits_schema_primitive,
};

pub(crate) fn supported_encode_reserved_bits(
    fields: &[veln_ast::SchemaField],
    index: usize,
    reserved: (i64, i64),
) -> Option<(u8, i64)> {
    ReservedBitsEncodeContext {
        fields,
        index,
        bit_width: reserved.0,
        expected_value: reserved.1,
    }
    .is_supported()
    .then_some((reserved.0 as u8, reserved.1))
}

struct ReservedBitsEncodeContext<'a> {
    fields: &'a [veln_ast::SchemaField],
    index: usize,
    bit_width: i64,
    expected_value: i64,
}

impl ReservedBitsEncodeContext<'_> {
    fn is_supported(&self) -> bool {
        supported_bit_packed_reserved_group(self.fields, self.index)
            || supported_byte_interleaved_reserved_group(
                self.fields,
                self.index,
                self.bit_width,
                self.expected_value,
            )
            || self.supports_forward_layout()
            || self.supports_backward_layout()
            || self.supports_middle_layout()
            || self.supports_standalone_layout()
    }

    fn supports_forward_layout(&self) -> bool {
        let next = self.next();
        (self.bit_width == 1
            && self.expected_value == 0
            && next.is_some_and(|field| canonical_schema_primitive_is(&field.ty, "UInt31be")))
            || supported_reserved_byte_prefix(self.bit_width, self.expected_value, next)
            || self.supports_packed_prefix(next)
            || next.zip(self.next_next()).is_some_and(|(first, second)| {
                supported_prefix_reserved_group(first, second, self.bit_width, self.expected_value)
            })
    }

    fn supports_packed_prefix(&self, next: Option<&veln_ast::SchemaField>) -> bool {
        packed_reserved_storage_bit_width(self.bit_width).is_some_and(|storage_bit_width| {
            next.and_then(|field| exact_width_schema_primitive_bit_width(&field.ty))
                .is_some_and(|next_bit_width| {
                    i64::from(next_bit_width) + self.bit_width == storage_bit_width
                })
                && self
                    .maximum_value()
                    .is_some_and(|max_value| self.expected_value <= max_value)
        })
    }

    fn supports_backward_layout(&self) -> bool {
        let previous = self.previous();
        self.supports_packed_suffix(previous)
            || previous.is_some_and(|field| {
                supported_byte_visible_reserved_suffix(field, (self.bit_width, self.expected_value))
            })
            || self
                .previous_previous()
                .zip(previous)
                .is_some_and(|(first, second)| {
                    supported_suffix_reserved_group(
                        first,
                        second,
                        self.bit_width,
                        self.expected_value,
                    )
                })
    }

    fn supports_packed_suffix(&self, previous: Option<&veln_ast::SchemaField>) -> bool {
        suffix_packed_reserved_storage_bit_width(self.bit_width).is_some_and(|storage_bit_width| {
            !self.previous_previous().is_some_and(|field| {
                previous.is_some_and(|visible| supported_packed_reserved_prefix(field, visible))
            }) && previous
                .and_then(|field| exact_width_schema_primitive_bit_width(&field.ty))
                .is_some_and(|previous_bit_width| {
                    i64::from(previous_bit_width) + self.bit_width == storage_bit_width
                })
                && self
                    .maximum_value()
                    .is_some_and(|max_value| self.expected_value <= max_value)
        })
    }

    fn supports_middle_layout(&self) -> bool {
        self.previous()
            .zip(self.next())
            .is_some_and(|(previous, next)| {
                supported_middle_reserved_bits(previous, next, self.bit_width, self.expected_value)
            })
    }

    fn supports_standalone_layout(&self) -> bool {
        self.bit_width > 0
            && self.bit_width <= 32
            && self.bit_width % 8 == 0
            && self
                .maximum_value()
                .is_some_and(|max_value| self.expected_value <= max_value)
    }

    fn maximum_value(&self) -> Option<i64> {
        if self.bit_width == 32 {
            Some(0xffff_ffff)
        } else {
            reserved_bits_max_value(self.bit_width)
        }
    }

    fn previous_previous(&self) -> Option<&veln_ast::SchemaField> {
        self.index
            .checked_sub(2)
            .and_then(|index| self.fields.get(index))
    }

    fn previous(&self) -> Option<&veln_ast::SchemaField> {
        self.index
            .checked_sub(1)
            .and_then(|index| self.fields.get(index))
    }

    fn next(&self) -> Option<&veln_ast::SchemaField> {
        self.fields.get(self.index + 1)
    }

    fn next_next(&self) -> Option<&veln_ast::SchemaField> {
        self.fields.get(self.index + 2)
    }
}

fn supported_bit_packed_reserved_group(fields: &[veln_ast::SchemaField], index: usize) -> bool {
    for start in 0..=index {
        let mut total_bit_width = 0_i64;
        let mut has_reserved = false;
        let mut has_visible = false;
        for (offset, field) in fields[start..].iter().enumerate() {
            let Some(bit_width) = bit_packed_group_field_width(field) else {
                break;
            };
            total_bit_width += bit_width;
            has_reserved |= reserved_bits_schema_primitive(&field.ty).is_some();
            has_visible |= reserved_bits_schema_primitive(&field.ty).is_none();
            if matches!(total_bit_width, 8 | 16 | 24 | 32 | 40 | 48 | 56 | 64) {
                let end = start + offset;
                if has_reserved && has_visible && start <= index && index <= end {
                    return true;
                }
                break;
            }
            if total_bit_width > 64 {
                break;
            }
        }
    }
    false
}

fn bit_packed_group_field_width(field: &veln_ast::SchemaField) -> Option<i64> {
    if let Some((bit_width, expected_value)) = reserved_bits_schema_primitive(&field.ty) {
        if bit_width <= 0 || bit_width >= 64 || bit_width % 8 == 0 {
            return None;
        }
        let max_value = reserved_bits_max_value(bit_width)?;
        return (expected_value <= max_value).then_some(bit_width);
    }
    if exact_width_schema_primitive_little_endian(&field.ty) {
        return None;
    }
    let bit_width = i64::from(exact_width_schema_primitive_bit_width(&field.ty)?);
    (bit_width % 8 != 0).then_some(bit_width)
}

fn reserved_bits_max_value(bit_width: i64) -> Option<i64> {
    if !(1..=63).contains(&bit_width) {
        return None;
    }
    if bit_width == 63 {
        return Some(i64::MAX);
    }
    Some((1_i64 << bit_width) - 1)
}

fn supported_prefix_reserved_group(
    first_visible_field: &veln_ast::SchemaField,
    second_visible_field: &veln_ast::SchemaField,
    bit_width: i64,
    expected_value: i64,
) -> bool {
    if bit_width <= 0 || bit_width > 57 {
        return false;
    }
    if exact_width_schema_primitive_little_endian(&first_visible_field.ty)
        || exact_width_schema_primitive_little_endian(&second_visible_field.ty)
    {
        return false;
    }
    let Some(first_bit_width) = exact_width_schema_primitive_bit_width(&first_visible_field.ty)
    else {
        return false;
    };
    let Some(second_bit_width) = exact_width_schema_primitive_bit_width(&second_visible_field.ty)
    else {
        return false;
    };
    if first_bit_width > 8 || second_bit_width > 8 {
        return false;
    }
    let total_bit_width = bit_width + i64::from(first_bit_width) + i64::from(second_bit_width);
    let supported_one_byte_group = bit_width % 8 != 0
        && (bit_width + i64::from(first_bit_width)) % 8 != 0
        && total_bit_width == 8;
    let supported_two_byte_group = total_bit_width == 16;
    let supported_three_byte_group = (17..=23).contains(&bit_width) && total_bit_width == 24;
    let supported_four_byte_group = (25..=31).contains(&bit_width) && total_bit_width == 32;
    let supported_five_byte_group = bit_width == 33 && total_bit_width == 40;
    let supported_six_byte_group = bit_width == 41 && total_bit_width == 48;
    let supported_seven_byte_group = bit_width == 49 && total_bit_width == 56;
    let supported_eight_byte_group = bit_width == 57 && total_bit_width == 64;
    (supported_one_byte_group
        || supported_two_byte_group
        || supported_three_byte_group
        || supported_four_byte_group
        || supported_five_byte_group
        || supported_six_byte_group
        || supported_seven_byte_group
        || supported_eight_byte_group)
        && expected_value < (1_i64 << bit_width)
}

fn supported_suffix_reserved_group(
    first_visible_field: &veln_ast::SchemaField,
    second_visible_field: &veln_ast::SchemaField,
    bit_width: i64,
    expected_value: i64,
) -> bool {
    if bit_width <= 0 || bit_width > 7 {
        return false;
    }
    if exact_width_schema_primitive_little_endian(&first_visible_field.ty)
        || exact_width_schema_primitive_little_endian(&second_visible_field.ty)
    {
        return false;
    }
    let Some(first_bit_width) = exact_width_schema_primitive_bit_width(&first_visible_field.ty)
    else {
        return false;
    };
    let Some(second_bit_width) = exact_width_schema_primitive_bit_width(&second_visible_field.ty)
    else {
        return false;
    };
    first_bit_width <= 8
        && second_bit_width == 8
        && i64::from(first_bit_width) + i64::from(second_bit_width) + bit_width == 16
        && expected_value < (1_i64 << bit_width)
}

fn supported_byte_visible_reserved_suffix(
    visible_field: &veln_ast::SchemaField,
    reserved: (i64, i64),
) -> bool {
    let (bit_width, expected_value) = reserved;
    if bit_width <= 8 || bit_width >= 56 || bit_width % 8 == 0 {
        return false;
    }
    if !canonical_schema_primitive_is(&visible_field.ty, "UInt8") {
        return false;
    }
    let storage_bit_width = ((8 + bit_width + 7) / 8) * 8;
    storage_bit_width > 16
        && storage_bit_width <= 64
        && reserved_bits_max_value(bit_width).is_some_and(|max_value| expected_value <= max_value)
}

fn supported_packed_reserved_prefix(
    reserved_field: &veln_ast::SchemaField,
    visible_field: &veln_ast::SchemaField,
) -> bool {
    let Some((bit_width, expected_value)) = reserved_bits_schema_primitive(&reserved_field.ty)
    else {
        return false;
    };
    packed_reserved_storage_bit_width(bit_width).is_some_and(|storage_bit_width| {
        exact_width_schema_primitive_bit_width(&visible_field.ty).is_some_and(|visible_bit_width| {
            i64::from(visible_bit_width) + bit_width == storage_bit_width
        }) && expected_value < (1_i64 << bit_width)
    })
}

fn supported_reserved_byte_prefix(
    bit_width: i64,
    expected_value: i64,
    visible_field: Option<&veln_ast::SchemaField>,
) -> bool {
    bit_width > 0
        && bit_width <= 56
        && bit_width % 8 != 0
        && reserved_bits_max_value(bit_width)
            .is_some_and(|max_value| (0..=max_value).contains(&expected_value))
        && visible_field.is_some_and(|field| canonical_schema_primitive_is(&field.ty, "UInt8"))
}

pub(crate) fn schema_payload_has_generalized_reserved_byte_prefix(schema: &SchemaDecl) -> bool {
    schema.fields.iter().enumerate().any(|(index, field)| {
        let Some((bit_width, expected_value)) = reserved_bits_schema_primitive(&field.ty) else {
            return false;
        };
        schema_field_uses_generalized_reserved_byte_prefix(
            &schema.fields,
            index,
            (bit_width, expected_value),
        )
    })
}

pub(crate) fn schema_field_uses_generalized_reserved_byte_prefix(
    fields: &[veln_ast::SchemaField],
    index: usize,
    reserved: (i64, i64),
) -> bool {
    let (bit_width, expected_value) = reserved;
    supported_reserved_byte_prefix(bit_width, expected_value, fields.get(index + 1))
        && !matches!((bit_width, expected_value), (1, 0) | (2, 0) | (9, 0))
}

fn supported_middle_reserved_bits(
    previous_field: &veln_ast::SchemaField,
    next_field: &veln_ast::SchemaField,
    bit_width: i64,
    expected_value: i64,
) -> bool {
    if bit_width <= 0 || bit_width > 32 {
        return false;
    }
    if exact_width_schema_primitive_little_endian(&previous_field.ty)
        || exact_width_schema_primitive_little_endian(&next_field.ty)
    {
        return false;
    }
    let Some(previous_bit_width) = exact_width_schema_primitive_bit_width(&previous_field.ty)
    else {
        return false;
    };
    let Some(next_bit_width) = exact_width_schema_primitive_bit_width(&next_field.ty) else {
        return false;
    };
    let total_bit_width = i64::from(previous_bit_width) + bit_width + i64::from(next_bit_width);
    previous_bit_width % 8 != 0
        && (i64::from(previous_bit_width) + bit_width) % 8 != 0
        && matches!(total_bit_width, 8 | 16 | 24 | 32)
        && expected_value < (1_i64 << bit_width)
}

fn supported_byte_interleaved_reserved_group(
    fields: &[veln_ast::SchemaField],
    index: usize,
    bit_width: i64,
    expected_value: i64,
) -> bool {
    if bit_width <= 0 || bit_width > 7 {
        return false;
    }
    let Some(first_field) = index
        .checked_sub(1)
        .and_then(|previous| fields.get(previous))
    else {
        return false;
    };
    let (Some(byte_field), Some(last_field)) = (fields.get(index + 1), fields.get(index + 2))
    else {
        return false;
    };
    if [first_field, byte_field, last_field]
        .iter()
        .any(|field| exact_width_schema_primitive_little_endian(&field.ty))
    {
        return false;
    }
    let Some(first_bit_width) = exact_width_schema_primitive_bit_width(&first_field.ty) else {
        return false;
    };
    let Some(byte_bit_width) = exact_width_schema_primitive_bit_width(&byte_field.ty) else {
        return false;
    };
    let Some(last_bit_width) = exact_width_schema_primitive_bit_width(&last_field.ty) else {
        return false;
    };
    first_bit_width < 8
        && byte_bit_width == 8
        && last_bit_width < 8
        && i64::from(first_bit_width) + bit_width + 8 + i64::from(last_bit_width) == 16
        && (i64::from(first_bit_width) + bit_width) % 8 != 0
        && expected_value < (1_i64 << bit_width)
}

fn packed_reserved_storage_bit_width(bit_width: i64) -> Option<i64> {
    if (1..=7).contains(&bit_width) {
        Some(8)
    } else if (9..=15).contains(&bit_width) {
        Some(16)
    } else if (17..=23).contains(&bit_width) {
        Some(24)
    } else if (25..=31).contains(&bit_width) {
        Some(32)
    } else {
        None
    }
}

fn suffix_packed_reserved_storage_bit_width(bit_width: i64) -> Option<i64> {
    packed_reserved_storage_bit_width(bit_width).or_else(|| {
        if (33..=39).contains(&bit_width) {
            Some(40)
        } else if (41..=47).contains(&bit_width) {
            Some(48)
        } else if (49..=55).contains(&bit_width) {
            Some(56)
        } else if (57..=63).contains(&bit_width) {
            Some(64)
        } else {
            None
        }
    })
}
