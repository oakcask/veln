use std::{collections::BTreeSet, sync::OnceLock};

use veln_ast::{BinaryOp, PrefixOp, SurfaceModule, lower_surface_ast};
use veln_core::{CoreCallTarget, CoreType};
use veln_source::SourceFile;
use veln_syntax::parse;

use crate::adt;
use crate::standard_symbols::{StandardSymbolDescriptor, prelude_symbol};
use crate::types::{Type, core_type, parse_type_or_unknown};

pub(crate) const PRELUDE_MODULE: &str = "prelude";
const PRELUDE_BUILTIN_MODULE: &str = "prelude_builtin";

pub(crate) fn prelude_signature(name: &str, expected: Option<&Type>) -> Option<(Vec<Type>, Type)> {
    prelude_signature_with_input(name, expected, None)
}

pub(crate) fn prelude_signature_with_input(
    name: &str,
    expected: Option<&Type>,
    input: Option<&Type>,
) -> Option<(Vec<Type>, Type)> {
    let descriptor = prelude_symbol(name)?;
    let expected = ExpectedPreludeParts::from_expected_and_input(expected, input);
    prelude_float_signature(descriptor.name)
        .or_else(|| prelude_byte_signature(descriptor.name))
        .or_else(|| prelude_string_signature(descriptor.name))
        .or_else(|| prelude_vec_signature(descriptor.name, &expected))
        .or_else(|| prelude_list_signature(descriptor.name, &expected))
        .or_else(|| prelude_dict_signature(descriptor.name, &expected))
        .or_else(|| prelude_option_signature(descriptor.name, &expected))
        .or_else(|| prelude_result_signature(descriptor.name, &expected))
        .or_else(|| source_backed_prelude_callback_signature(descriptor))
}

pub(crate) fn qualified_prelude_builtin_signature_with_input(
    segments: &[String],
    expected: Option<&Type>,
    input: Option<&Type>,
) -> Option<(String, Vec<Type>, Type)> {
    let [module, name] = segments else {
        return None;
    };
    if module != PRELUDE_BUILTIN_MODULE {
        return None;
    }
    let (params, return_type) = prelude_signature_with_input(name, expected, input)?;
    Some((name.clone(), params, return_type))
}

pub(crate) fn qualified_prelude_signature(
    segments: &[String],
    expected: Option<&Type>,
) -> Option<(String, Vec<Type>, Type)> {
    qualified_prelude_signature_with_input(segments, expected, None)
}

pub(crate) fn qualified_prelude_signature_with_input(
    segments: &[String],
    expected: Option<&Type>,
    input: Option<&Type>,
) -> Option<(String, Vec<Type>, Type)> {
    let [module, name] = segments else {
        return None;
    };
    if module != PRELUDE_MODULE {
        return None;
    }
    let (params, return_type) = prelude_signature_with_input(name, expected, input)?;
    Some((name.clone(), params, return_type))
}

#[derive(Clone)]
struct SourcePreludeSignature {
    name: String,
    params: Vec<Type>,
    return_type: Type,
}

static SOURCE_PRELUDE_CALLBACK_SIGNATURES: OnceLock<Vec<SourcePreludeSignature>> = OnceLock::new();

fn source_backed_prelude_callback_signature(
    descriptor: &StandardSymbolDescriptor,
) -> Option<(Vec<Type>, Type)> {
    descriptor.source?;
    source_prelude_callback_signatures()
        .iter()
        .find(|signature| signature.name == descriptor.name)
        .map(|signature| (signature.params.clone(), signature.return_type.clone()))
}

fn source_prelude_callback_signatures() -> &'static [SourcePreludeSignature] {
    SOURCE_PRELUDE_CALLBACK_SIGNATURES
        .get_or_init(|| {
            let source = veln_stdlib::prelude_source("");
            source_prelude_callback_signatures_from_text(source.path, source.text)
        })
        .as_slice()
}

fn source_prelude_callback_signatures_from_text(
    path: &'static str,
    text: &'static str,
) -> Vec<SourcePreludeSignature> {
    let file = SourceFile::new(path, text);
    let parsed = parse(&file);
    if !parsed.diagnostics.is_empty() {
        return Vec::new();
    }
    let module = lower_surface_ast(&parsed.tree);
    let known_types = source_prelude_known_type_names(&module);

    module
        .functions
        .iter()
        .filter_map(|function| {
            let name = function.name.clone()?;
            let params = function
                .params
                .iter()
                .map(|param| {
                    source_prelude_concrete_type(
                        &parse_type_or_unknown(param.ty.as_deref()),
                        &known_types,
                    )
                })
                .collect::<Vec<_>>();
            if !params.iter().any(concrete_function_parameter) {
                return None;
            }
            Some(SourcePreludeSignature {
                name,
                params,
                return_type: source_prelude_concrete_type(
                    &parse_type_or_unknown(function.return_type.as_deref()),
                    &known_types,
                ),
            })
        })
        .collect()
}

fn source_prelude_known_type_names(module: &SurfaceModule) -> BTreeSet<String> {
    let mut known = [
        "Bool", "Int", "Float", "String", "Unit", "Option", "Result", "List", "Vec", "Dict",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<BTreeSet<_>>();
    known.extend(module.types.iter().filter_map(|ty| ty.name.clone()));
    known
}

fn source_prelude_concrete_type(ty: &Type, known_types: &BTreeSet<String>) -> Type {
    match ty {
        Type::Unknown => Type::Unknown,
        Type::Named { name, args } if source_prelude_type_name_is_known(name, known_types) => {
            Type::named(
                name.clone(),
                args.iter()
                    .map(|arg| source_prelude_concrete_type(arg, known_types))
                    .collect(),
            )
        }
        Type::Named { .. } => Type::Unknown,
        Type::Record(fields) => Type::Record(
            fields
                .iter()
                .map(|(name, ty)| (name.clone(), source_prelude_concrete_type(ty, known_types)))
                .collect(),
        ),
        Type::Function {
            params,
            variadic,
            return_type,
            effects,
        } => Type::Function {
            params: params
                .iter()
                .map(|param| source_prelude_concrete_type(param, known_types))
                .collect(),
            variadic: variadic
                .as_deref()
                .map(|ty| Box::new(source_prelude_concrete_type(ty, known_types))),
            return_type: Box::new(source_prelude_concrete_type(return_type, known_types)),
            effects: effects.clone(),
        },
    }
}

fn source_prelude_type_name_is_known(name: &str, known_types: &BTreeSet<String>) -> bool {
    known_types.contains(name)
        || name
            .rsplit("::")
            .next()
            .is_some_and(|last| known_types.contains(last))
}

fn concrete_function_parameter(ty: &Type) -> bool {
    matches!(ty, Type::Function { .. }) && !prelude_type_has_unknown(ty)
}

fn prelude_type_has_unknown(ty: &Type) -> bool {
    match ty {
        Type::Unknown => true,
        Type::Named { args, .. } => args.iter().any(prelude_type_has_unknown),
        Type::Record(fields) => fields.iter().any(|(_, ty)| prelude_type_has_unknown(ty)),
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            params.iter().any(prelude_type_has_unknown)
                || variadic.as_deref().is_some_and(prelude_type_has_unknown)
                || prelude_type_has_unknown(return_type)
        }
    }
}

struct ExpectedPreludeParts {
    direct: Type,
    vec_item: Type,
    input_vec_item: Type,
    list_item: Type,
    input_list_item: Type,
    option_item: Type,
    input_option_item: Type,
    result_value: Type,
    result_error: Type,
    input_result_value: Type,
    input_result_error: Type,
    dict_key: Type,
    dict_value: Type,
    input_dict_key: Type,
    input_dict_value: Type,
}

impl ExpectedPreludeParts {
    fn from_expected_and_input(expected: Option<&Type>, input: Option<&Type>) -> Self {
        let (result_value, result_error) = expected
            .and_then(adt::result_parts)
            .map_or((Type::Unknown, Type::Unknown), |(value, error)| {
                (value.clone(), error.clone())
            });
        let (input_result_value, input_result_error) = input
            .and_then(adt::result_parts)
            .map_or((Type::Unknown, Type::Unknown), |(value, error)| {
                (value.clone(), error.clone())
            });
        let (dict_key, dict_value) = expected
            .and_then(Type::dict_parts)
            .map_or((Type::Unknown, Type::Unknown), |(key, value)| {
                (key.clone(), value.clone())
            });
        let (input_dict_key, input_dict_value) = input
            .and_then(Type::dict_parts)
            .map_or((Type::Unknown, Type::Unknown), |(key, value)| {
                (key.clone(), value.clone())
            });
        Self {
            direct: expected.cloned().unwrap_or(Type::Unknown),
            vec_item: expected
                .and_then(Type::vec_part)
                .cloned()
                .unwrap_or(Type::Unknown),
            input_vec_item: input
                .and_then(Type::vec_part)
                .cloned()
                .unwrap_or(Type::Unknown),
            list_item: expected
                .and_then(adt::list_part)
                .cloned()
                .unwrap_or(Type::Unknown),
            input_list_item: input
                .and_then(adt::list_part)
                .cloned()
                .unwrap_or(Type::Unknown),
            option_item: expected
                .and_then(adt::option_part)
                .cloned()
                .unwrap_or(Type::Unknown),
            input_option_item: input
                .and_then(adt::option_part)
                .cloned()
                .unwrap_or(Type::Unknown),
            result_value,
            result_error,
            input_result_value,
            input_result_error,
            dict_key,
            dict_value,
            input_dict_key,
            input_dict_value,
        }
    }
}

fn prelude_float_signature(name: &str) -> Option<(Vec<Type>, Type)> {
    match name {
        "float_negate" => Some((vec![Type::float()], Type::float())),
        "float_add" | "float_subtract" | "float_multiply" | "float_divide" => {
            Some((vec![Type::float(), Type::float()], Type::float()))
        }
        "float_less" | "float_less_equal" | "float_greater" | "float_greater_equal" => {
            Some((vec![Type::float(), Type::float()], Type::bool()))
        }
        _ => None,
    }
}

fn prelude_byte_signature(name: &str) -> Option<(Vec<Type>, Type)> {
    byte_prelude_signature::<Type>(name)
}

type ByteSignature<T> = (Vec<T>, T);

trait BytePreludeType: Clone {
    fn named(name: &'static str) -> Self;
    fn record(fields: Vec<(&'static str, Self)>) -> Self;
    fn bool() -> Self;
    fn int() -> Self;
    fn string() -> Self;
    fn unit() -> Self;
    fn vec(item: Self) -> Self;
    fn list(item: Self) -> Self;
    fn option(item: Self) -> Self;
    fn result(value: Self, error: Self) -> Self;
}

impl BytePreludeType for Type {
    fn named(name: &'static str) -> Self {
        Type::named(name, Vec::new())
    }

    fn record(fields: Vec<(&'static str, Self)>) -> Self {
        Type::Record(
            fields
                .into_iter()
                .map(|(name, ty)| (name.to_string(), ty))
                .collect(),
        )
    }

    fn bool() -> Self {
        Type::bool()
    }

    fn int() -> Self {
        Type::int()
    }

    fn string() -> Self {
        Type::string()
    }

    fn unit() -> Self {
        Type::unit()
    }

    fn vec(item: Self) -> Self {
        Type::vec(item)
    }

    fn list(item: Self) -> Self {
        adt::list_type(item)
    }

    fn option(item: Self) -> Self {
        adt::option_type(item)
    }

    fn result(value: Self, error: Self) -> Self {
        adt::result_type(value, error)
    }
}

impl BytePreludeType for CoreType {
    fn named(name: &'static str) -> Self {
        CoreType::named(name, Vec::new())
    }

    fn record(fields: Vec<(&'static str, Self)>) -> Self {
        CoreType::Record(
            fields
                .into_iter()
                .map(|(name, ty)| (name.to_string(), ty))
                .collect(),
        )
    }

    fn bool() -> Self {
        CoreType::bool()
    }

    fn int() -> Self {
        CoreType::int()
    }

    fn string() -> Self {
        CoreType::string()
    }

    fn unit() -> Self {
        CoreType::unit()
    }

    fn vec(item: Self) -> Self {
        CoreType::vec(item)
    }

    fn list(item: Self) -> Self {
        adt::core_list_type(item)
    }

    fn option(item: Self) -> Self {
        adt::core_option_type(item)
    }

    fn result(value: Self, error: Self) -> Self {
        adt::core_result_type(value, error)
    }
}

struct BytePreludeTypes<T> {
    byte: T,
    flag8: T,
    flag16be: T,
    flag16le: T,
    flag24be: T,
    flag24le: T,
    flag32be: T,
    flag32le: T,
    flag40be: T,
    flag40le: T,
    flag48be: T,
    flag48le: T,
    flag56be: T,
    flag56le: T,
    flag64be: T,
    flag64le: T,
    byte_chunk: T,
    byte_view: T,
    byte_count: T,
    byte_offset: T,
}

impl<T: BytePreludeType> BytePreludeTypes<T> {
    fn new() -> Self {
        Self {
            byte: T::named("Byte"),
            flag8: T::named("Flag8"),
            flag16be: T::named("Flag16be"),
            flag16le: T::named("Flag16le"),
            flag24be: T::named("Flag24be"),
            flag24le: T::named("Flag24le"),
            flag32be: T::named("Flag32be"),
            flag32le: T::named("Flag32le"),
            flag40be: T::named("Flag40be"),
            flag40le: T::named("Flag40le"),
            flag48be: T::named("Flag48be"),
            flag48le: T::named("Flag48le"),
            flag56be: T::named("Flag56be"),
            flag56le: T::named("Flag56le"),
            flag64be: T::named("Flag64be"),
            flag64le: T::named("Flag64le"),
            byte_chunk: T::named("ByteChunk"),
            byte_view: T::named("ByteView"),
            byte_count: T::named("ByteCount"),
            byte_offset: T::named("ByteOffset"),
        }
    }

    fn flags(&self) -> [(&'static str, &T); 15] {
        [
            ("flag8", &self.flag8),
            ("flag16be", &self.flag16be),
            ("flag16le", &self.flag16le),
            ("flag24be", &self.flag24be),
            ("flag24le", &self.flag24le),
            ("flag32be", &self.flag32be),
            ("flag32le", &self.flag32le),
            ("flag40be", &self.flag40be),
            ("flag40le", &self.flag40le),
            ("flag48be", &self.flag48be),
            ("flag48le", &self.flag48le),
            ("flag56be", &self.flag56be),
            ("flag56le", &self.flag56le),
            ("flag64be", &self.flag64be),
            ("flag64le", &self.flag64le),
        ]
    }
}

fn byte_prelude_signature<T: BytePreludeType>(name: &str) -> Option<ByteSignature<T>> {
    let types = BytePreludeTypes::new();
    byte_constructor_signature(name, &types)
        .or_else(|| byte_flag_signature(name, &types))
        .or_else(|| byte_chunk_signature(name, &types))
        .or_else(|| byte_view_signature(name, &types))
        .or_else(|| byte_chunk_list_signature(name, &types))
        .or_else(|| byte_decode_sample_signature(name, &types))
        .or_else(|| http2_protocol_preface_signature(name, &types))
        .or_else(|| http2_protocol_frame_signature(name, &types))
        .or_else(|| http2_peer_limit_signature(name, &types))
        .or_else(|| hpack_fixture_signature(name, &types))
        .or_else(|| byte_numeric_signature(name, &types))
}

fn result_string<T: BytePreludeType>(value: T) -> T {
    T::result(value, T::string())
}

fn unit_runtime_diagnostic_result<T: BytePreludeType>() -> T {
    T::result(T::unit(), T::named("RuntimeDiagnostic"))
}

fn byte_constructor_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "byte" => Some((vec![T::int()], result_string(types.byte.clone()))),
        "byte_to_int" => Some((vec![types.byte.clone()], T::int())),
        _ => None,
    }
}

fn byte_flag_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    for (prefix, flag) in types.flags() {
        let Some(suffix) = name.strip_prefix(prefix) else {
            continue;
        };
        return match suffix {
            "_is_set" => Some((vec![flag.clone(), T::int()], result_string(T::bool()))),
            "_set" => Some((vec![flag.clone(), T::int()], result_string(flag.clone()))),
            "_bits" => Some((vec![flag.clone()], T::int())),
            "_from_bits" => Some((vec![T::int()], result_string(flag.clone()))),
            _ => None,
        };
    }
    None
}

fn byte_chunk_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "byte_chunk" => Some((vec![T::vec(types.byte.clone())], types.byte_chunk.clone())),
        "byte_chunk_count" => Some((vec![types.byte_chunk.clone()], types.byte_count.clone())),
        "byte_append" => Some((
            vec![types.byte_chunk.clone(), types.byte_chunk.clone()],
            types.byte_chunk.clone(),
        )),
        "byte_chunk_from_hex" => Some((vec![T::string()], result_string(types.byte_chunk.clone()))),
        "byte_chunk_to_visible_ascii_string" => {
            Some((vec![types.byte_chunk.clone()], result_string(T::string())))
        }
        "hpack_fixture_huffman_bytes_label" => Some((vec![types.byte_chunk.clone()], T::string())),
        "hpack_fixture_huffman_label_bytes" => {
            Some((vec![T::string()], T::option(types.byte_chunk.clone())))
        }
        "byte_chunk_from_visible_ascii_string" => {
            Some((vec![T::string()], result_string(types.byte_chunk.clone())))
        }
        "byte_take" | "byte_drop" => Some((
            vec![types.byte_chunk.clone(), types.byte_count.clone()],
            result_string(types.byte_chunk.clone()),
        )),
        _ => None,
    }
}

fn byte_view_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "byte_view" => Some((
            vec![
                types.byte_chunk.clone(),
                types.byte_offset.clone(),
                types.byte_count.clone(),
            ],
            result_string(types.byte_view.clone()),
        )),
        "byte_view_to_chunk" => Some((vec![types.byte_view.clone()], types.byte_chunk.clone())),
        "byte_view_count" => Some((vec![types.byte_view.clone()], types.byte_count.clone())),
        "byte_view_take" | "byte_view_drop" => Some((
            vec![types.byte_view.clone(), types.byte_count.clone()],
            result_string(types.byte_view.clone()),
        )),
        "byte_view_slice" => Some((
            vec![
                types.byte_view.clone(),
                types.byte_count.clone(),
                types.byte_count.clone(),
            ],
            result_string(types.byte_view.clone()),
        )),
        _ => None,
    }
}

fn byte_chunk_list_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "byte_chunks_empty" => Some((Vec::new(), T::list(types.byte_chunk.clone()))),
        "byte_chunks_one" => Some((
            vec![types.byte_chunk.clone()],
            T::list(types.byte_chunk.clone()),
        )),
        "byte_chunks_append" => Some((
            vec![
                T::list(types.byte_chunk.clone()),
                T::list(types.byte_chunk.clone()),
            ],
            T::list(types.byte_chunk.clone()),
        )),
        _ => None,
    }
}

fn byte_decode_sample_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "byte_expect_fixed_u8_be" => Some((
            vec![types.byte_view.clone(), T::int(), T::string(), T::string()],
            result_string(T::int()),
        )),
        "byte_decode_http2_frame" => Some((
            vec![types.byte_view.clone()],
            result_string(http2_frame_type()),
        )),
        "byte_decode_schema_width_sample" => Some((
            vec![types.byte_view.clone()],
            result_string(schema_width_sample_type()),
        )),
        "byte_decode_schema_validation_sample" => Some((
            vec![types.byte_view.clone()],
            result_string(schema_validation_sample_type()),
        )),
        _ => None,
    }
}

fn http2_protocol_preface_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "http2_protocol_closed_with_pending" => Some((
            vec![T::int(), T::int(), T::string(), types.byte_view.clone()],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_partial_preface" => Some((
            vec![T::int(), T::int(), types.byte_view.clone()],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_invalid_preface" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_continuation_expected" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        _ => None,
    }
}

fn http2_protocol_frame_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "http2_protocol_invalid_frame_kind" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_invalid_stream_id" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_invalid_payload_length" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_invalid_window_update_increment" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_invalid_data_padding" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_content_length_mismatch" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_invalid_request_header_list"
        | "http2_protocol_invalid_response_header_list" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                T::string(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_unexpected_settings_ack" => Some((
            vec![T::int(), T::string(), T::string(), types.byte_view.clone()],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_invalid_priority_dependency" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_stream_after_goaway" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        _ => None,
    }
}

fn http2_peer_limit_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "http2_peer_limit_frame_size_exceeded" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_peer_limit_header_list_size_exceeded" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_peer_limit_header_table_size_exceeded" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_peer_limit_flow_control_window_exceeded" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_peer_limit_concurrent_streams_exceeded" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_peer_limit_settings_value_out_of_range" => Some((
            vec![
                T::int(),
                T::int(),
                T::string(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        _ => None,
    }
}

fn hpack_fixture_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "hpack_fixture_unsupported_header_block"
        | "hpack_fixture_unsupported_static_index"
        | "hpack_fixture_malformed_string_length"
        | "hpack_fixture_malformed_raw_string_value"
        | "hpack_fixture_malformed_huffman_padding"
        | "hpack_fixture_huffman_eos_symbol"
        | "hpack_fixture_huffman_non_visible_value"
        | "hpack_fixture_table_size_update_malformed" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "hpack_fixture_dynamic_index_out_of_range"
        | "hpack_fixture_dynamic_name_continuation_missing"
        | "hpack_fixture_dynamic_name_continuation_malformed"
        | "hpack_fixture_dynamic_name_continuation_out_of_range" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "hpack_fixture_table_size_update_not_at_start"
        | "hpack_fixture_table_size_update_trailing_bytes" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        _ => None,
    }
}

fn byte_numeric_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "byte_read_u8_be" | "byte_read_u16_be" | "byte_read_u24_be" | "byte_read_u31_be"
        | "byte_read_u32_be" | "byte_read_u40_be" | "byte_read_u48_be" | "byte_read_u64_be"
        | "byte_read_u16_le" | "byte_read_u24_le" | "byte_read_u31_le" | "byte_read_u32_le"
        | "byte_read_u40_le" | "byte_read_u48_le" | "byte_read_u64_le" => {
            Some((vec![types.byte_view.clone()], result_string(T::int())))
        }
        "byte_write_u8_be" | "byte_write_u16_be" | "byte_write_u24_be" | "byte_write_u31_be"
        | "byte_write_u32_be" | "byte_write_u40_be" | "byte_write_u48_be" | "byte_write_u64_be"
        | "byte_write_u16_le" | "byte_write_u24_le" | "byte_write_u31_le" | "byte_write_u32_le"
        | "byte_write_u40_le" | "byte_write_u48_le" | "byte_write_u64_le" => {
            Some((vec![T::int()], result_string(types.byte_chunk.clone())))
        }
        "byte_count" => Some((vec![T::int()], result_string(types.byte_count.clone()))),
        "byte_count_to_int" => Some((vec![types.byte_count.clone()], T::int())),
        "byte_offset" => Some((vec![T::int()], result_string(types.byte_offset.clone()))),
        "byte_offset_to_int" => Some((vec![types.byte_offset.clone()], T::int())),
        _ => None,
    }
}

fn http2_frame_type<T: BytePreludeType>() -> T {
    T::record(vec![
        ("length", T::int()),
        ("kind", T::int()),
        ("flags", T::int()),
        ("stream_id", T::int()),
        ("payload", T::named("ByteView")),
    ])
}

fn schema_width_sample_type<T: BytePreludeType>() -> T {
    T::record(vec![("short_value", T::int()), ("wide_value", T::int())])
}

fn schema_validation_sample_type<T: BytePreludeType>() -> T {
    T::record(vec![("length", T::int()), ("padding_length", T::int())])
}

fn prelude_string_signature(name: &str) -> Option<(Vec<Type>, Type)> {
    match name {
        "string_split_once" => Some((
            vec![Type::string(), Type::string()],
            adt::option_type(Type::Record(vec![
                ("left".to_string(), Type::string()),
                ("right".to_string(), Type::string()),
            ])),
        )),
        "string_parse_int" => Some((
            vec![Type::string()],
            adt::result_type(Type::int(), Type::string()),
        )),
        "int_to_string" => Some((vec![Type::int()], Type::string())),
        _ => None,
    }
}

fn prelude_vec_signature(name: &str, expected: &ExpectedPreludeParts) -> Option<(Vec<Type>, Type)> {
    prelude_vec_basic_signature(name, &expected.vec_item)
        .or_else(|| prelude_vec_callback_signature(name, expected))
}

fn prelude_vec_basic_signature(name: &str, vec_item: &Type) -> Option<(Vec<Type>, Type)> {
    match name {
        "vec_len" => Some((vec![Type::vec(Type::Unknown)], Type::int())),
        "vec_is_empty" => Some((vec![Type::vec(Type::Unknown)], Type::bool())),
        "vec_push" => Some((
            vec![Type::vec(vec_item.clone()), vec_item.clone()],
            Type::vec(vec_item.clone()),
        )),
        "vec_concat" => Some((
            vec![Type::vec(vec_item.clone()), Type::vec(vec_item.clone())],
            Type::vec(vec_item.clone()),
        )),
        _ => None,
    }
}

fn prelude_vec_callback_signature(
    name: &str,
    expected: &ExpectedPreludeParts,
) -> Option<(Vec<Type>, Type)> {
    let vec_item = &expected.vec_item;
    let input_vec_item = &expected.input_vec_item;
    let same_vec_item = prefer_known(input_vec_item, vec_item);
    match name {
        "vec_map" => Some((
            vec![
                Type::vec(input_vec_item.to_owned()),
                Type::Function {
                    params: vec![input_vec_item.to_owned()],
                    variadic: None,
                    return_type: Box::new(vec_item.clone()),
                    effects: Vec::new(),
                },
            ],
            Type::vec(vec_item.clone()),
        )),
        "vec_filter" => Some((
            vec![
                Type::vec(same_vec_item.clone()),
                Type::Function {
                    params: vec![same_vec_item.clone()],
                    variadic: None,
                    return_type: Box::new(Type::bool()),
                    effects: Vec::new(),
                },
            ],
            Type::vec(same_vec_item),
        )),
        "vec_fold" => Some((
            vec![
                Type::vec(input_vec_item.to_owned()),
                expected.direct.clone(),
                Type::Function {
                    params: vec![expected.direct.clone(), input_vec_item.to_owned()],
                    variadic: None,
                    return_type: Box::new(expected.direct.clone()),
                    effects: Vec::new(),
                },
            ],
            expected.direct.clone(),
        )),
        "vec_try_map" => Some(vec_try_map_signature(
            &expected.result_value,
            expected.result_error.clone(),
            input_vec_item,
            false,
        )),
        "vec_try_map_with" => Some(vec_try_map_signature(
            &expected.result_value,
            expected.result_error.clone(),
            input_vec_item,
            true,
        )),
        _ => None,
    }
}

fn prelude_list_signature(
    name: &str,
    expected: &ExpectedPreludeParts,
) -> Option<(Vec<Type>, Type)> {
    prelude_list_basic_signature(name, &expected.list_item)
        .or_else(|| prelude_list_callback_signature(name, expected))
}

fn prelude_list_basic_signature(name: &str, list_item: &Type) -> Option<(Vec<Type>, Type)> {
    match name {
        "list_nil" => Some((Vec::new(), adt::list_type(list_item.clone()))),
        "list_cons" => Some((
            vec![list_item.clone(), adt::list_type(list_item.clone())],
            adt::list_type(list_item.clone()),
        )),
        "list_is_empty" => Some((vec![adt::list_type(Type::Unknown)], Type::bool())),
        "list_reverse" => Some((
            vec![adt::list_type(list_item.clone())],
            adt::list_type(list_item.clone()),
        )),
        _ => None,
    }
}

fn prelude_list_callback_signature(
    name: &str,
    expected: &ExpectedPreludeParts,
) -> Option<(Vec<Type>, Type)> {
    let list_item = &expected.list_item;
    let input_list_item = &expected.input_list_item;
    let same_list_item = prefer_known(input_list_item, list_item);
    match name {
        "list_map" => Some((
            vec![
                adt::list_type(input_list_item.to_owned()),
                Type::Function {
                    params: vec![input_list_item.to_owned()],
                    variadic: None,
                    return_type: Box::new(list_item.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::list_type(list_item.clone()),
        )),
        "list_filter" => Some((
            vec![
                adt::list_type(same_list_item.clone()),
                Type::Function {
                    params: vec![same_list_item.clone()],
                    variadic: None,
                    return_type: Box::new(Type::bool()),
                    effects: Vec::new(),
                },
            ],
            adt::list_type(same_list_item),
        )),
        "list_fold" => Some((
            vec![
                adt::list_type(input_list_item.to_owned()),
                expected.direct.clone(),
                Type::Function {
                    params: vec![expected.direct.clone(), input_list_item.to_owned()],
                    variadic: None,
                    return_type: Box::new(expected.direct.clone()),
                    effects: Vec::new(),
                },
            ],
            expected.direct.clone(),
        )),
        "list_try_map" => Some(list_try_map_signature(
            &expected.result_value,
            expected.result_error.clone(),
            input_list_item,
        )),
        _ => None,
    }
}

fn prelude_dict_signature(
    name: &str,
    expected: &ExpectedPreludeParts,
) -> Option<(Vec<Type>, Type)> {
    let dict_key = prefer_known(&expected.dict_key, &expected.input_dict_key);
    let dict_value = prefer_known(&expected.dict_value, &expected.input_dict_value);
    let option_item = prefer_known(&expected.option_item, &expected.input_dict_value);
    match name {
        "dict_get" => Some((
            vec![Type::dict(dict_key.clone(), option_item.clone()), dict_key],
            adt::option_type(option_item.clone()),
        )),
        "dict_contains" => Some((
            vec![Type::dict(dict_key.clone(), dict_value.clone()), dict_key],
            Type::bool(),
        )),
        "dict_insert" => Some((
            vec![
                Type::dict(dict_key.clone(), dict_value.clone()),
                dict_key.clone(),
                dict_value.clone(),
            ],
            Type::dict(dict_key.clone(), dict_value.clone()),
        )),
        "dict_remove" => Some((
            vec![
                Type::dict(dict_key.clone(), dict_value.clone()),
                dict_key.clone(),
            ],
            Type::dict(dict_key.clone(), dict_value.clone()),
        )),
        "dict_map" | "dict_map_with" => {
            let with_context = name == "dict_map_with";
            let result_dict_key = prefer_known(&expected.input_dict_key, &expected.dict_key);
            let result_dict_value = expected.dict_value.clone();
            Some((
                prelude_callback_params_with_context(
                    with_context,
                    Type::dict(result_dict_key.clone(), expected.input_dict_value.clone()),
                    Type::Function {
                        params: prelude_callback_args_with_context(
                            with_context,
                            vec![result_dict_key.clone(), expected.input_dict_value.clone()],
                        ),
                        variadic: None,
                        return_type: Box::new(result_dict_value.clone()),
                        effects: Vec::new(),
                    },
                ),
                Type::dict(result_dict_key, result_dict_value),
            ))
        }
        "dict_filter" | "dict_filter_with" => {
            let with_context = name == "dict_filter_with";
            let same_dict_key = prefer_known(&expected.input_dict_key, &expected.dict_key);
            let same_dict_value = prefer_known(&expected.input_dict_value, &expected.dict_value);
            Some((
                prelude_callback_params_with_context(
                    with_context,
                    Type::dict(same_dict_key.clone(), same_dict_value.clone()),
                    Type::Function {
                        params: prelude_callback_args_with_context(
                            with_context,
                            vec![same_dict_key.clone(), same_dict_value.clone()],
                        ),
                        variadic: None,
                        return_type: Box::new(Type::bool()),
                        effects: Vec::new(),
                    },
                ),
                Type::dict(same_dict_key, same_dict_value),
            ))
        }
        "dict_fold" | "dict_fold_with" => {
            let with_context = name == "dict_fold_with";
            Some((
                prelude_fold_params_with_context(
                    with_context,
                    Type::dict(
                        expected.input_dict_key.clone(),
                        expected.input_dict_value.clone(),
                    ),
                    expected.direct.clone(),
                    Type::Function {
                        params: prelude_callback_args_with_context(
                            with_context,
                            vec![
                                expected.direct.clone(),
                                expected.input_dict_key.clone(),
                                expected.input_dict_value.clone(),
                            ],
                        ),
                        variadic: None,
                        return_type: Box::new(expected.direct.clone()),
                        effects: Vec::new(),
                    },
                ),
                expected.direct.clone(),
            ))
        }
        "dict_try_map" | "dict_try_map_with" => {
            let with_context = name == "dict_try_map_with";
            let (result_dict_key, result_dict_value) = expected
                .result_value
                .dict_parts()
                .map_or((Type::Unknown, Type::Unknown), |(key, value)| {
                    (key.clone(), value.clone())
                });
            let output_key = prefer_known(&expected.input_dict_key, &result_dict_key);
            Some((
                prelude_callback_params_with_context(
                    with_context,
                    Type::dict(output_key.clone(), expected.input_dict_value.clone()),
                    Type::Function {
                        params: prelude_callback_args_with_context(
                            with_context,
                            vec![output_key.clone(), expected.input_dict_value.clone()],
                        ),
                        variadic: None,
                        return_type: Box::new(adt::result_type(
                            result_dict_value.clone(),
                            expected.result_error.clone(),
                        )),
                        effects: Vec::new(),
                    },
                ),
                adt::result_type(
                    Type::dict(output_key, result_dict_value),
                    expected.result_error.clone(),
                ),
            ))
        }
        _ => None,
    }
}

fn prelude_callback_params_with_context(
    with_context: bool,
    input: Type,
    callback: Type,
) -> Vec<Type> {
    let mut params = Vec::new();
    if with_context {
        params.push(Type::Unknown);
    }
    params.push(input);
    params.push(callback);
    params
}

fn prelude_fold_params_with_context(
    with_context: bool,
    input: Type,
    initial: Type,
    callback: Type,
) -> Vec<Type> {
    let mut params = Vec::new();
    if with_context {
        params.push(Type::Unknown);
    }
    params.push(input);
    params.push(initial);
    params.push(callback);
    params
}

fn prelude_callback_args_with_context(with_context: bool, args: Vec<Type>) -> Vec<Type> {
    if with_context {
        std::iter::once(Type::Unknown).chain(args).collect()
    } else {
        args
    }
}

fn prelude_option_signature(
    name: &str,
    expected: &ExpectedPreludeParts,
) -> Option<(Vec<Type>, Type)> {
    let option_item = &expected.option_item;
    let input_option_item = &expected.input_option_item;
    match name {
        "option_map" => Some((
            vec![
                adt::option_type(input_option_item.clone()),
                Type::Function {
                    params: vec![input_option_item.clone()],
                    variadic: None,
                    return_type: Box::new(option_item.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::option_type(option_item.clone()),
        )),
        "option_and_then" => Some((
            vec![
                adt::option_type(input_option_item.clone()),
                Type::Function {
                    params: vec![input_option_item.clone()],
                    variadic: None,
                    return_type: Box::new(adt::option_type(option_item.clone())),
                    effects: Vec::new(),
                },
            ],
            adt::option_type(option_item.clone()),
        )),
        "option_unwrap_or" => Some((
            vec![
                adt::option_type(expected.direct.clone()),
                expected.direct.clone(),
            ],
            expected.direct.clone(),
        )),
        _ => None,
    }
}

fn prelude_result_signature(
    name: &str,
    expected: &ExpectedPreludeParts,
) -> Option<(Vec<Type>, Type)> {
    let result_value = &expected.result_value;
    let result_error = &expected.result_error;
    let input_result_value = &expected.input_result_value;
    let input_result_error = &expected.input_result_error;
    let carried_result_value = prefer_known(result_value, &expected.input_result_value);
    let carried_result_error = prefer_known(result_error, &expected.input_result_error);
    match name {
        "result_map" => Some((
            vec![
                adt::result_type(input_result_value.clone(), carried_result_error.clone()),
                Type::Function {
                    params: vec![input_result_value.clone()],
                    variadic: None,
                    return_type: Box::new(result_value.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::result_type(result_value.clone(), carried_result_error),
        )),
        "result_map_err" => Some((
            vec![
                adt::result_type(carried_result_value.clone(), input_result_error.clone()),
                Type::Function {
                    params: vec![input_result_error.clone()],
                    variadic: None,
                    return_type: Box::new(result_error.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::result_type(carried_result_value, result_error.clone()),
        )),
        "result_and_then" => Some((
            vec![
                adt::result_type(input_result_value.clone(), carried_result_error.clone()),
                Type::Function {
                    params: vec![input_result_value.clone()],
                    variadic: None,
                    return_type: Box::new(adt::result_type(
                        result_value.clone(),
                        carried_result_error.clone(),
                    )),
                    effects: Vec::new(),
                },
            ],
            adt::result_type(result_value.clone(), carried_result_error),
        )),
        _ => None,
    }
}

fn vec_try_map_signature(
    result_value: &Type,
    result_error: Type,
    input_vec_item: &Type,
    with_context: bool,
) -> (Vec<Type>, Type) {
    let mapped_item = result_value.vec_part().cloned().unwrap_or(Type::Unknown);
    let input_item = input_vec_item.clone();
    let mut params = Vec::new();
    let mut callback_params = Vec::new();

    if with_context {
        params.push(Type::Unknown);
        callback_params.push(Type::Unknown);
    }

    params.push(Type::vec(input_item.clone()));
    callback_params.push(input_item);
    params.push(Type::Function {
        params: callback_params,
        variadic: None,
        return_type: Box::new(adt::result_type(mapped_item.clone(), result_error.clone())),
        effects: Vec::new(),
    });

    (
        params,
        adt::result_type(Type::vec(mapped_item), result_error),
    )
}

fn list_try_map_signature(
    result_value: &Type,
    result_error: Type,
    input_list_item: &Type,
) -> (Vec<Type>, Type) {
    let mapped_item = adt::list_part(result_value)
        .cloned()
        .unwrap_or(Type::Unknown);
    let input_item = input_list_item.clone();
    (
        vec![
            adt::list_type(input_item.clone()),
            Type::Function {
                params: vec![input_item],
                variadic: None,
                return_type: Box::new(adt::result_type(mapped_item.clone(), result_error.clone())),
                effects: Vec::new(),
            },
        ],
        adt::result_type(adt::list_type(mapped_item), result_error),
    )
}

fn prefer_known(primary: &Type, fallback: &Type) -> Type {
    if primary == &Type::Unknown {
        fallback.clone()
    } else {
        primary.clone()
    }
}

pub(crate) fn float_prefix_prelude_name(op: PrefixOp) -> Option<&'static str> {
    match op {
        PrefixOp::Negate => Some("float_negate"),
        _ => None,
    }
}

pub(crate) fn float_arithmetic_prelude_name(op: BinaryOp) -> Option<&'static str> {
    match op {
        BinaryOp::Add => Some("float_add"),
        BinaryOp::Subtract => Some("float_subtract"),
        BinaryOp::Multiply => Some("float_multiply"),
        BinaryOp::Divide => Some("float_divide"),
        _ => None,
    }
}

pub(crate) fn float_comparison_prelude_name(op: BinaryOp) -> Option<&'static str> {
    match op {
        BinaryOp::Less => Some("float_less"),
        BinaryOp::LessEqual => Some("float_less_equal"),
        BinaryOp::Greater => Some("float_greater"),
        BinaryOp::GreaterEqual => Some("float_greater_equal"),
        _ => None,
    }
}

fn core_vec_try_map_signature(
    result_value: &CoreType,
    result_error: CoreType,
    with_context: bool,
) -> (Vec<CoreType>, CoreType) {
    let mapped_item = result_value
        .vec_part()
        .cloned()
        .unwrap_or(CoreType::Unknown);
    let mut params = Vec::new();
    let mut callback_params = Vec::new();

    if with_context {
        params.push(CoreType::Unknown);
        callback_params.push(CoreType::Unknown);
    }

    params.push(CoreType::vec(CoreType::Unknown));
    callback_params.push(CoreType::Unknown);
    params.push(CoreType::Function {
        params: callback_params,
        variadic: None,
        return_type: Box::new(adt::core_result_type(
            mapped_item.clone(),
            result_error.clone(),
        )),
        effects: Vec::new(),
    });

    (
        params,
        adt::core_result_type(CoreType::vec(mapped_item), result_error),
    )
}

pub(crate) fn core_prelude_signature(
    name: &str,
    expected: Option<&CoreType>,
) -> Option<(CoreCallTarget, Vec<CoreType>, CoreType)> {
    let descriptor = prelude_symbol(name)?;
    let expected = ExpectedCorePreludeParts::from_expected(expected);
    let signature = core_prelude_float_signature(descriptor.name)
        .or_else(|| core_prelude_byte_signature(descriptor.name))
        .or_else(|| core_prelude_string_signature(descriptor.name))
        .or_else(|| core_prelude_vec_signature(descriptor.name, &expected))
        .or_else(|| core_prelude_list_signature(descriptor.name, &expected))
        .or_else(|| core_prelude_dict_signature(descriptor.name, &expected))
        .or_else(|| core_prelude_option_signature(descriptor.name, &expected))
        .or_else(|| core_prelude_result_signature(descriptor.name, &expected))
        .or_else(|| source_backed_core_prelude_callback_signature(descriptor))?;
    Some((
        CoreCallTarget::PreludeBuiltin(descriptor.name.to_string()),
        signature.0,
        signature.1,
    ))
}

fn source_backed_core_prelude_callback_signature(
    descriptor: &StandardSymbolDescriptor,
) -> Option<(Vec<CoreType>, CoreType)> {
    let (params, return_type) = source_backed_prelude_callback_signature(descriptor)?;
    Some((
        params.iter().map(core_type).collect(),
        core_type(&return_type),
    ))
}

pub(crate) fn qualified_core_prelude_builtin_signature(
    segments: &[String],
    expected: Option<&CoreType>,
) -> Option<(CoreCallTarget, Vec<CoreType>, CoreType)> {
    let [module, name] = segments else {
        return None;
    };
    if module != PRELUDE_BUILTIN_MODULE {
        return None;
    }
    core_prelude_signature(name, expected)
}

pub(crate) fn qualified_core_prelude_signature(
    segments: &[String],
    expected: Option<&CoreType>,
) -> Option<(CoreCallTarget, Vec<CoreType>, CoreType)> {
    let [module, name] = segments else {
        return None;
    };
    if module != PRELUDE_MODULE {
        return None;
    }
    core_prelude_signature(name, expected)
}

struct ExpectedCorePreludeParts {
    direct: CoreType,
    vec_item: CoreType,
    list_item: CoreType,
    option_item: CoreType,
    result_value: CoreType,
    result_error: CoreType,
    dict_key: CoreType,
    dict_value: CoreType,
}

impl ExpectedCorePreludeParts {
    fn from_expected(expected: Option<&CoreType>) -> Self {
        let (result_value, result_error) = expected
            .and_then(adt::core_result_parts)
            .map_or((CoreType::Unknown, CoreType::Unknown), |(value, error)| {
                (value.clone(), error.clone())
            });
        let (dict_key, dict_value) = expected
            .and_then(CoreType::dict_parts)
            .map_or((CoreType::Unknown, CoreType::Unknown), |(key, value)| {
                (key.clone(), value.clone())
            });
        Self {
            direct: expected.cloned().unwrap_or(CoreType::Unknown),
            vec_item: expected
                .and_then(CoreType::vec_part)
                .cloned()
                .unwrap_or(CoreType::Unknown),
            list_item: expected
                .and_then(adt::core_list_part)
                .cloned()
                .unwrap_or(CoreType::Unknown),
            option_item: expected
                .and_then(adt::core_option_part)
                .cloned()
                .unwrap_or(CoreType::Unknown),
            result_value,
            result_error,
            dict_key,
            dict_value,
        }
    }
}

fn core_prelude_float_signature(name: &str) -> Option<(Vec<CoreType>, CoreType)> {
    match name {
        "float_negate" => Some((vec![CoreType::float()], CoreType::float())),
        "float_add" | "float_subtract" | "float_multiply" | "float_divide" => Some((
            vec![CoreType::float(), CoreType::float()],
            CoreType::float(),
        )),
        "float_less" | "float_less_equal" | "float_greater" | "float_greater_equal" => {
            Some((vec![CoreType::float(), CoreType::float()], CoreType::bool()))
        }
        _ => None,
    }
}

fn core_prelude_byte_signature(name: &str) -> Option<(Vec<CoreType>, CoreType)> {
    byte_prelude_signature::<CoreType>(name)
}

fn core_prelude_string_signature(name: &str) -> Option<(Vec<CoreType>, CoreType)> {
    match name {
        "string_split_once" => Some((
            vec![CoreType::string(), CoreType::string()],
            adt::core_option_type(CoreType::Record(vec![
                ("left".to_string(), CoreType::string()),
                ("right".to_string(), CoreType::string()),
            ])),
        )),
        "string_parse_int" => Some((
            vec![CoreType::string()],
            adt::core_result_type(CoreType::int(), CoreType::string()),
        )),
        "int_to_string" => Some((vec![CoreType::int()], CoreType::string())),
        _ => None,
    }
}

fn core_prelude_vec_signature(
    name: &str,
    expected: &ExpectedCorePreludeParts,
) -> Option<(Vec<CoreType>, CoreType)> {
    core_prelude_vec_basic_signature(name, &expected.vec_item)
        .or_else(|| core_prelude_vec_callback_signature(name, expected))
}

fn core_prelude_vec_basic_signature(
    name: &str,
    vec_item: &CoreType,
) -> Option<(Vec<CoreType>, CoreType)> {
    match name {
        "vec_len" => Some((vec![CoreType::vec(CoreType::Unknown)], CoreType::int())),
        "vec_is_empty" => Some((vec![CoreType::vec(CoreType::Unknown)], CoreType::bool())),
        "vec_push" => Some((
            vec![CoreType::vec(vec_item.clone()), vec_item.clone()],
            CoreType::vec(vec_item.clone()),
        )),
        "vec_concat" => Some((
            vec![
                CoreType::vec(vec_item.clone()),
                CoreType::vec(vec_item.clone()),
            ],
            CoreType::vec(vec_item.clone()),
        )),
        _ => None,
    }
}

fn core_prelude_vec_callback_signature(
    name: &str,
    expected: &ExpectedCorePreludeParts,
) -> Option<(Vec<CoreType>, CoreType)> {
    let vec_item = &expected.vec_item;
    match name {
        "vec_map" => Some((
            vec![
                CoreType::vec(CoreType::Unknown),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    variadic: None,
                    return_type: Box::new(vec_item.clone()),
                    effects: Vec::new(),
                },
            ],
            CoreType::vec(vec_item.clone()),
        )),
        "vec_filter" => Some((
            vec![
                CoreType::vec(vec_item.clone()),
                CoreType::Function {
                    params: vec![vec_item.clone()],
                    variadic: None,
                    return_type: Box::new(CoreType::bool()),
                    effects: Vec::new(),
                },
            ],
            CoreType::vec(vec_item.clone()),
        )),
        "vec_fold" => Some((
            vec![
                CoreType::vec(CoreType::Unknown),
                expected.direct.clone(),
                CoreType::Function {
                    params: vec![expected.direct.clone(), CoreType::Unknown],
                    variadic: None,
                    return_type: Box::new(expected.direct.clone()),
                    effects: Vec::new(),
                },
            ],
            expected.direct.clone(),
        )),
        "vec_try_map" => Some(core_vec_try_map_signature(
            &expected.result_value,
            expected.result_error.clone(),
            false,
        )),
        "vec_try_map_with" => Some(core_vec_try_map_signature(
            &expected.result_value,
            expected.result_error.clone(),
            true,
        )),
        _ => None,
    }
}

fn core_prelude_list_signature(
    name: &str,
    expected: &ExpectedCorePreludeParts,
) -> Option<(Vec<CoreType>, CoreType)> {
    core_prelude_list_basic_signature(name, &expected.list_item)
        .or_else(|| core_prelude_list_callback_signature(name, expected))
}

fn core_prelude_list_basic_signature(
    name: &str,
    list_item: &CoreType,
) -> Option<(Vec<CoreType>, CoreType)> {
    match name {
        "list_nil" => Some((Vec::new(), adt::core_list_type(list_item.clone()))),
        "list_cons" => Some((
            vec![list_item.clone(), adt::core_list_type(list_item.clone())],
            adt::core_list_type(list_item.clone()),
        )),
        "list_is_empty" => Some((
            vec![adt::core_list_type(CoreType::Unknown)],
            CoreType::bool(),
        )),
        "list_reverse" => Some((
            vec![adt::core_list_type(list_item.clone())],
            adt::core_list_type(list_item.clone()),
        )),
        _ => None,
    }
}

fn core_prelude_list_callback_signature(
    name: &str,
    expected: &ExpectedCorePreludeParts,
) -> Option<(Vec<CoreType>, CoreType)> {
    let list_item = &expected.list_item;
    match name {
        "list_map" => Some((
            vec![
                adt::core_list_type(CoreType::Unknown),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    variadic: None,
                    return_type: Box::new(list_item.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::core_list_type(list_item.clone()),
        )),
        "list_filter" => Some((
            vec![
                adt::core_list_type(list_item.clone()),
                CoreType::Function {
                    params: vec![list_item.clone()],
                    variadic: None,
                    return_type: Box::new(CoreType::bool()),
                    effects: Vec::new(),
                },
            ],
            adt::core_list_type(list_item.clone()),
        )),
        "list_fold" => Some((
            vec![
                adt::core_list_type(CoreType::Unknown),
                expected.direct.clone(),
                CoreType::Function {
                    params: vec![expected.direct.clone(), CoreType::Unknown],
                    variadic: None,
                    return_type: Box::new(expected.direct.clone()),
                    effects: Vec::new(),
                },
            ],
            expected.direct.clone(),
        )),
        "list_try_map" => Some(core_list_try_map_signature(
            &expected.result_value,
            expected.result_error.clone(),
        )),
        _ => None,
    }
}

fn core_prelude_dict_signature(
    name: &str,
    expected: &ExpectedCorePreludeParts,
) -> Option<(Vec<CoreType>, CoreType)> {
    let dict_key = &expected.dict_key;
    let dict_value = &expected.dict_value;
    let option_item = &expected.option_item;
    match name {
        "dict_get" => Some((
            vec![
                CoreType::dict(dict_key.clone(), option_item.clone()),
                dict_key.clone(),
            ],
            adt::core_option_type(option_item.clone()),
        )),
        "dict_contains" => Some((
            vec![
                CoreType::dict(dict_key.clone(), dict_value.clone()),
                dict_key.clone(),
            ],
            CoreType::bool(),
        )),
        "dict_insert" => Some((
            vec![
                CoreType::dict(dict_key.clone(), dict_value.clone()),
                dict_key.clone(),
                dict_value.clone(),
            ],
            CoreType::dict(dict_key.clone(), dict_value.clone()),
        )),
        "dict_remove" => Some((
            vec![
                CoreType::dict(dict_key.clone(), dict_value.clone()),
                dict_key.clone(),
            ],
            CoreType::dict(dict_key.clone(), dict_value.clone()),
        )),
        "dict_map" | "dict_map_with" => {
            let with_context = name == "dict_map_with";
            Some((
                core_prelude_callback_params_with_context(
                    with_context,
                    CoreType::dict(dict_key.clone(), CoreType::Unknown),
                    CoreType::Function {
                        params: core_prelude_callback_args_with_context(
                            with_context,
                            vec![dict_key.clone(), CoreType::Unknown],
                        ),
                        variadic: None,
                        return_type: Box::new(dict_value.clone()),
                        effects: Vec::new(),
                    },
                ),
                CoreType::dict(dict_key.clone(), dict_value.clone()),
            ))
        }
        "dict_filter" | "dict_filter_with" => {
            let with_context = name == "dict_filter_with";
            Some((
                core_prelude_callback_params_with_context(
                    with_context,
                    CoreType::dict(dict_key.clone(), dict_value.clone()),
                    CoreType::Function {
                        params: core_prelude_callback_args_with_context(
                            with_context,
                            vec![dict_key.clone(), dict_value.clone()],
                        ),
                        variadic: None,
                        return_type: Box::new(CoreType::bool()),
                        effects: Vec::new(),
                    },
                ),
                CoreType::dict(dict_key.clone(), dict_value.clone()),
            ))
        }
        "dict_fold" | "dict_fold_with" => {
            let with_context = name == "dict_fold_with";
            Some((
                core_prelude_fold_params_with_context(
                    with_context,
                    CoreType::dict(CoreType::Unknown, CoreType::Unknown),
                    expected.direct.clone(),
                    CoreType::Function {
                        params: core_prelude_callback_args_with_context(
                            with_context,
                            vec![
                                expected.direct.clone(),
                                CoreType::Unknown,
                                CoreType::Unknown,
                            ],
                        ),
                        variadic: None,
                        return_type: Box::new(expected.direct.clone()),
                        effects: Vec::new(),
                    },
                ),
                expected.direct.clone(),
            ))
        }
        "dict_try_map" | "dict_try_map_with" => {
            let with_context = name == "dict_try_map_with";
            let (result_dict_key, result_dict_value) = expected
                .result_value
                .dict_parts()
                .map_or((CoreType::Unknown, CoreType::Unknown), |(key, value)| {
                    (key.clone(), value.clone())
                });
            Some((
                core_prelude_callback_params_with_context(
                    with_context,
                    CoreType::dict(result_dict_key.clone(), CoreType::Unknown),
                    CoreType::Function {
                        params: core_prelude_callback_args_with_context(
                            with_context,
                            vec![result_dict_key.clone(), CoreType::Unknown],
                        ),
                        variadic: None,
                        return_type: Box::new(adt::core_result_type(
                            result_dict_value.clone(),
                            expected.result_error.clone(),
                        )),
                        effects: Vec::new(),
                    },
                ),
                adt::core_result_type(
                    CoreType::dict(result_dict_key, result_dict_value),
                    expected.result_error.clone(),
                ),
            ))
        }
        _ => None,
    }
}

fn core_prelude_callback_params_with_context(
    with_context: bool,
    input: CoreType,
    callback: CoreType,
) -> Vec<CoreType> {
    let mut params = Vec::new();
    if with_context {
        params.push(CoreType::Unknown);
    }
    params.push(input);
    params.push(callback);
    params
}

fn core_prelude_fold_params_with_context(
    with_context: bool,
    input: CoreType,
    initial: CoreType,
    callback: CoreType,
) -> Vec<CoreType> {
    let mut params = Vec::new();
    if with_context {
        params.push(CoreType::Unknown);
    }
    params.push(input);
    params.push(initial);
    params.push(callback);
    params
}

fn core_prelude_callback_args_with_context(
    with_context: bool,
    args: Vec<CoreType>,
) -> Vec<CoreType> {
    if with_context {
        std::iter::once(CoreType::Unknown).chain(args).collect()
    } else {
        args
    }
}

fn core_prelude_option_signature(
    name: &str,
    expected: &ExpectedCorePreludeParts,
) -> Option<(Vec<CoreType>, CoreType)> {
    let option_item = &expected.option_item;
    match name {
        "option_map" => Some((
            vec![
                adt::core_option_type(CoreType::Unknown),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    variadic: None,
                    return_type: Box::new(option_item.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::core_option_type(option_item.clone()),
        )),
        "option_and_then" => Some((
            vec![
                adt::core_option_type(CoreType::Unknown),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    variadic: None,
                    return_type: Box::new(adt::core_option_type(option_item.clone())),
                    effects: Vec::new(),
                },
            ],
            adt::core_option_type(option_item.clone()),
        )),
        "option_unwrap_or" => Some((
            vec![
                adt::core_option_type(expected.direct.clone()),
                expected.direct.clone(),
            ],
            expected.direct.clone(),
        )),
        _ => None,
    }
}

fn core_prelude_result_signature(
    name: &str,
    expected: &ExpectedCorePreludeParts,
) -> Option<(Vec<CoreType>, CoreType)> {
    let result_value = &expected.result_value;
    let result_error = &expected.result_error;
    match name {
        "result_map" => Some((
            vec![
                adt::core_result_type(CoreType::Unknown, result_error.clone()),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    variadic: None,
                    return_type: Box::new(result_value.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::core_result_type(result_value.clone(), result_error.clone()),
        )),
        "result_map_err" => Some((
            vec![
                adt::core_result_type(result_value.clone(), CoreType::Unknown),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    variadic: None,
                    return_type: Box::new(result_error.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::core_result_type(result_value.clone(), result_error.clone()),
        )),
        "result_and_then" => Some((
            vec![
                adt::core_result_type(CoreType::Unknown, result_error.clone()),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    variadic: None,
                    return_type: Box::new(adt::core_result_type(
                        result_value.clone(),
                        result_error.clone(),
                    )),
                    effects: Vec::new(),
                },
            ],
            adt::core_result_type(result_value.clone(), result_error.clone()),
        )),
        _ => None,
    }
}

fn core_list_try_map_signature(
    result_value: &CoreType,
    result_error: CoreType,
) -> (Vec<CoreType>, CoreType) {
    let mapped_item = adt::core_list_part(result_value)
        .cloned()
        .unwrap_or(CoreType::Unknown);
    (
        vec![
            adt::core_list_type(CoreType::Unknown),
            CoreType::Function {
                params: vec![CoreType::Unknown],
                variadic: None,
                return_type: Box::new(adt::core_result_type(
                    mapped_item.clone(),
                    result_error.clone(),
                )),
                effects: Vec::new(),
            },
        ],
        adt::core_result_type(adt::core_list_type(mapped_item), result_error),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prelude_signature_is_gated_by_standard_symbol_descriptor() {
        let (params, return_type) =
            prelude_signature("vec_len", None).expect("standard helper signature");

        assert_eq!(params, vec![Type::vec(Type::Unknown)]);
        assert_eq!(return_type, Type::int());
        assert!(prelude_signature("unknown_helper", None).is_none());
    }

    #[test]
    fn dictionary_prelude_signatures_are_first_order() {
        let expected_dict = Type::dict(Type::string(), Type::int());
        let expected_option = adt::option_type(Type::int());

        for (name, expected, params, return_type) in [
            (
                "dict_get",
                expected_option,
                vec![expected_dict.clone(), Type::string()],
                adt::option_type(Type::int()),
            ),
            (
                "dict_contains",
                Type::bool(),
                vec![expected_dict.clone(), Type::string()],
                Type::bool(),
            ),
            (
                "dict_insert",
                expected_dict.clone(),
                vec![expected_dict.clone(), Type::string(), Type::int()],
                expected_dict.clone(),
            ),
            (
                "dict_remove",
                expected_dict.clone(),
                vec![expected_dict.clone(), Type::string()],
                expected_dict.clone(),
            ),
        ] {
            let (actual_params, actual_return_type) =
                prelude_signature_with_input(name, Some(&expected), Some(&expected_dict))
                    .expect("dictionary helper signature");

            assert_eq!(actual_params, params, "{name}");
            assert_eq!(actual_return_type, return_type, "{name}");
            assert!(
                actual_params
                    .iter()
                    .all(|param| !matches!(param, Type::Function { .. })),
                "{name} should not be treated as a callback helper"
            );
        }
    }

    #[test]
    fn source_backed_prelude_fallback_uses_concrete_callback_parameter() {
        let signatures = source_prelude_callback_signatures_from_text(
            "prelude.veln",
            concat!(
                "pub fn future_apply(value: Int, callback: fn(Int, String) -> Bool) -> Bool\n",
                "  callback(value, \"ok\")\n",
                "end\n",
            ),
        );

        let signature = signatures
            .iter()
            .find(|signature| signature.name == "future_apply")
            .expect("future source-backed callback helper should have a fallback signature");

        assert_eq!(
            signature.params,
            vec![
                Type::int(),
                Type::function(vec![Type::int(), Type::string()], Type::bool(), Vec::new())
            ]
        );
        assert_eq!(signature.return_type, Type::bool());
    }

    #[test]
    fn source_backed_prelude_fallback_rejects_non_concrete_callback_parameter() {
        let signatures = source_prelude_callback_signatures_from_text(
            "prelude.veln",
            concat!(
                "pub fn future_generic(value: A, callback: fn(A, Int) -> Bool) -> Bool\n",
                "  callback(value, 1)\n",
                "end\n",
            ),
        );

        assert!(
            signatures
                .iter()
                .all(|signature| signature.name != "future_generic"),
            "generic callback parameter should stay outside the fallback"
        );
    }
}
