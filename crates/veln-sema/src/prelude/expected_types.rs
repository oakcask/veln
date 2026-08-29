use super::*;

pub(super) struct ExpectedPreludeParts {
    pub(super) direct: Type,
    pub(super) vec_item: Type,
    pub(super) input_vec_item: Type,
    pub(super) list_item: Type,
    pub(super) input_list_item: Type,
    pub(super) option_item: Type,
    pub(super) input_option_item: Type,
    pub(super) result_value: Type,
    pub(super) result_error: Type,
    pub(super) input_result_value: Type,
    pub(super) input_result_error: Type,
    pub(super) dict_key: Type,
    pub(super) dict_value: Type,
    pub(super) input_dict_key: Type,
    pub(super) input_dict_value: Type,
}

impl ExpectedPreludeParts {
    pub(super) fn from_expected_and_input(expected: Option<&Type>, input: Option<&Type>) -> Self {
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
