fn serialize_frames(
    pool: &Rc<RefCell<ConstantPool>>,
    frames: &[(usize, Frame)],
) -> Result<Vec<u8>, JvmBackendError> {
    let mut out = Vec::new();
    write_u16(
        &mut out,
        u16::try_from(frames.len()).map_err(|_| stack_map_error("too many stack-map frames"))?,
    );
    let mut previous = None;
    for (offset, frame) in frames {
        serialize_frame(pool, &mut out, *offset, frame, previous)?;
        previous = Some(*offset);
    }
    Ok(out)
}
fn serialize_frame(
    pool: &Rc<RefCell<ConstantPool>>,
    out: &mut Vec<u8>,
    offset: usize,
    frame: &Frame,
    previous: Option<usize>,
) -> Result<(), JvmBackendError> {
    out.push(255);
    let delta = match previous {
        None => offset,
        Some(previous) => offset
            .checked_sub(previous + 1)
            .ok_or_else(|| stack_map_error("stack-map offsets are not increasing"))?,
    };
    write_u16(
        out,
        u16::try_from(delta).map_err(|_| stack_map_error("stack-map offset exceeds u16"))?,
    );
    write_verification_types(
        pool,
        out,
        trimmed_locals(&frame.locals),
        "too many frame locals",
    )?;
    write_verification_types(pool, out, &frame.stack, "too many frame stack values")
}

fn write_verification_types(
    pool: &Rc<RefCell<ConstantPool>>,
    out: &mut Vec<u8>,
    values: &[VerificationType],
    overflow_message: &str,
) -> Result<(), JvmBackendError> {
    write_u16(
        out,
        u16::try_from(values.len()).map_err(|_| stack_map_error(overflow_message))?,
    );
    for value in values {
        write_verification_type(pool, out, value);
    }
    Ok(())
}

fn trimmed_locals(locals: &[VerificationType]) -> &[VerificationType] {
    let length = locals
        .iter()
        .rposition(|value| *value != VerificationType::Top)
        .map_or(0, |index| index + 1);
    &locals[..length]
}

fn write_verification_type(
    pool: &Rc<RefCell<ConstantPool>>,
    out: &mut Vec<u8>,
    value: &VerificationType,
) {
    match value {
        VerificationType::Top => out.push(0),
        VerificationType::Integer => out.push(1),
        VerificationType::Float => out.push(2),
        VerificationType::Double => out.push(3),
        VerificationType::Long => out.push(4),
        VerificationType::Null => out.push(5),
        VerificationType::UninitializedThis => out.push(6),
        VerificationType::Object(name) => {
            out.push(7);
            write_u16(out, pool.borrow_mut().class(name));
        }
        VerificationType::Uninitialized(offset) => {
            out.push(8);
            write_u16(out, *offset);
        }
    }
}

fn parse_method_descriptor(
    descriptor: &str,
) -> Result<(Vec<VerificationType>, Option<VerificationType>), JvmBackendError> {
    let bytes = descriptor.as_bytes();
    if bytes.first() != Some(&b'(') {
        return Err(stack_map_error("invalid method descriptor"));
    }
    let mut index = 1;
    let mut arguments = Vec::new();
    while bytes.get(index) != Some(&b')') {
        arguments.push(parse_descriptor_type(descriptor, &mut index)?);
    }
    index += 1;
    let result = if bytes.get(index) == Some(&b'V') {
        None
    } else {
        Some(parse_descriptor_type(descriptor, &mut index)?)
    };
    Ok((arguments, result))
}

fn parse_field_type(descriptor: &str) -> Result<VerificationType, JvmBackendError> {
    parse_descriptor_type(descriptor, &mut 0)
}

fn parse_descriptor_type(
    descriptor: &str,
    index: &mut usize,
) -> Result<VerificationType, JvmBackendError> {
    let bytes = descriptor.as_bytes();
    let start = *index;
    let byte = *bytes
        .get(*index)
        .ok_or_else(|| stack_map_error("truncated descriptor"))?;
    *index += 1;
    match byte {
        b'B' | b'C' | b'I' | b'S' | b'Z' => Ok(VerificationType::Integer),
        b'F' => Ok(VerificationType::Float),
        b'D' => Ok(VerificationType::Double),
        b'J' => Ok(VerificationType::Long),
        b'L' => {
            let end = descriptor[*index..]
                .find(';')
                .map(|relative| *index + relative)
                .ok_or_else(|| stack_map_error("unterminated object descriptor"))?;
            let name = descriptor[*index..end].to_string();
            *index = end + 1;
            Ok(VerificationType::Object(name))
        }
        b'[' => {
            while bytes.get(*index) == Some(&b'[') {
                *index += 1;
            }
            if bytes.get(*index) == Some(&b'L') {
                let _ = parse_descriptor_type(descriptor, index)?;
            } else {
                *index += 1;
            }
            Ok(VerificationType::Object(
                descriptor[start..*index].to_string(),
            ))
        }
        _ => Err(stack_map_error("unsupported descriptor type")),
    }
}
