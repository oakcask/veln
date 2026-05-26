#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreType {
    Unknown,
    Named {
        name: String,
        args: Vec<CoreType>,
    },
    Record(Vec<(String, CoreType)>),
    Function {
        params: Vec<CoreType>,
        return_type: Box<CoreType>,
        effects: Vec<String>,
    },
}

impl CoreType {
    pub fn named(name: impl Into<String>, args: Vec<CoreType>) -> Self {
        Self::Named {
            name: name.into(),
            args,
        }
    }

    pub fn bool() -> Self {
        Self::named("Bool", Vec::new())
    }

    pub fn int() -> Self {
        Self::named("Int", Vec::new())
    }

    pub fn float() -> Self {
        Self::named("Float", Vec::new())
    }

    pub fn string() -> Self {
        Self::named("String", Vec::new())
    }

    pub fn unit() -> Self {
        Self::named("Unit", Vec::new())
    }

    pub fn result(value: CoreType, error: CoreType) -> Self {
        Self::named("Result", vec![value, error])
    }

    pub fn option(value: CoreType) -> Self {
        Self::named("Option", vec![value])
    }

    pub fn list(value: CoreType) -> Self {
        Self::named("List", vec![value])
    }

    pub fn dict(key: CoreType, value: CoreType) -> Self {
        Self::named("Dict", vec![key, value])
    }

    pub fn result_parts(&self) -> Option<(&CoreType, &CoreType)> {
        match self {
            Self::Named { name, args } if name == "Result" && args.len() == 2 => {
                Some((&args[0], &args[1]))
            }
            _ => None,
        }
    }

    pub fn option_part(&self) -> Option<&CoreType> {
        match self {
            Self::Named { name, args } if name == "Option" && args.len() == 1 => Some(&args[0]),
            _ => None,
        }
    }

    pub fn list_part(&self) -> Option<&CoreType> {
        match self {
            Self::Named { name, args } if name == "List" && args.len() == 1 => Some(&args[0]),
            _ => None,
        }
    }

    pub fn dict_parts(&self) -> Option<(&CoreType, &CoreType)> {
        match self {
            Self::Named { name, args } if name == "Dict" && args.len() == 2 => {
                Some((&args[0], &args[1]))
            }
            _ => None,
        }
    }

    pub fn record_field(&self, field_name: &str) -> Option<&CoreType> {
        match self {
            Self::Record(fields) => fields
                .iter()
                .find_map(|(name, ty)| (name == field_name).then_some(ty)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_type_constructors_use_canonical_names_without_args() {
        assert_eq!(
            CoreType::bool(),
            CoreType::Named {
                name: "Bool".to_string(),
                args: Vec::new()
            }
        );
        assert_eq!(
            CoreType::int(),
            CoreType::Named {
                name: "Int".to_string(),
                args: Vec::new()
            }
        );
        assert_eq!(
            CoreType::float(),
            CoreType::Named {
                name: "Float".to_string(),
                args: Vec::new()
            }
        );
        assert_eq!(
            CoreType::string(),
            CoreType::Named {
                name: "String".to_string(),
                args: Vec::new()
            }
        );
        assert_eq!(
            CoreType::unit(),
            CoreType::Named {
                name: "Unit".to_string(),
                args: Vec::new()
            }
        );
    }

    #[test]
    fn generic_type_constructors_preserve_nested_arguments() {
        let result = CoreType::result(
            CoreType::option(CoreType::int()),
            CoreType::list(CoreType::string()),
        );

        assert_eq!(
            result,
            CoreType::Named {
                name: "Result".to_string(),
                args: vec![
                    CoreType::Named {
                        name: "Option".to_string(),
                        args: vec![CoreType::int()]
                    },
                    CoreType::Named {
                        name: "List".to_string(),
                        args: vec![CoreType::string()]
                    }
                ]
            }
        );
    }

    #[test]
    fn result_parts_accept_only_result_with_two_arguments() {
        let ok = CoreType::result(CoreType::int(), CoreType::string());
        assert_eq!(
            ok.result_parts(),
            Some((&CoreType::int(), &CoreType::string()))
        );

        let wrong_name = CoreType::named("Outcome", vec![CoreType::int(), CoreType::string()]);
        assert_eq!(wrong_name.result_parts(), None);

        let missing_error = CoreType::named("Result", vec![CoreType::int()]);
        assert_eq!(missing_error.result_parts(), None);

        let extra_arg = CoreType::named(
            "Result",
            vec![CoreType::int(), CoreType::string(), CoreType::bool()],
        );
        assert_eq!(extra_arg.result_parts(), None);
    }

    #[test]
    fn option_part_accepts_only_option_with_one_argument() {
        let some_type = CoreType::option(CoreType::bool());
        assert_eq!(some_type.option_part(), Some(&CoreType::bool()));

        let wrong_name = CoreType::named("Maybe", vec![CoreType::bool()]);
        assert_eq!(wrong_name.option_part(), None);

        let missing_arg = CoreType::named("Option", Vec::new());
        assert_eq!(missing_arg.option_part(), None);

        let extra_arg = CoreType::named("Option", vec![CoreType::bool(), CoreType::string()]);
        assert_eq!(extra_arg.option_part(), None);
    }

    #[test]
    fn list_part_accepts_only_list_with_one_argument() {
        let list_type = CoreType::list(CoreType::float());
        assert_eq!(list_type.list_part(), Some(&CoreType::float()));

        let wrong_name = CoreType::named("Array", vec![CoreType::float()]);
        assert_eq!(wrong_name.list_part(), None);

        let missing_arg = CoreType::named("List", Vec::new());
        assert_eq!(missing_arg.list_part(), None);

        let extra_arg = CoreType::named("List", vec![CoreType::float(), CoreType::int()]);
        assert_eq!(extra_arg.list_part(), None);
    }

    #[test]
    fn dict_parts_accept_only_dict_with_two_arguments() {
        let dict_type = CoreType::dict(CoreType::string(), CoreType::int());
        assert_eq!(
            dict_type.dict_parts(),
            Some((&CoreType::string(), &CoreType::int()))
        );

        let wrong_name = CoreType::named("Map", vec![CoreType::string(), CoreType::int()]);
        assert_eq!(wrong_name.dict_parts(), None);

        let missing_value = CoreType::named("Dict", vec![CoreType::string()]);
        assert_eq!(missing_value.dict_parts(), None);

        let extra_arg = CoreType::named(
            "Dict",
            vec![CoreType::string(), CoreType::int(), CoreType::bool()],
        );
        assert_eq!(extra_arg.dict_parts(), None);
    }

    #[test]
    fn record_field_finds_the_first_matching_field() {
        let first_count = CoreType::int();
        let second_count = CoreType::string();
        let record = CoreType::Record(vec![
            ("ready".to_string(), CoreType::bool()),
            ("count".to_string(), first_count.clone()),
            ("count".to_string(), second_count),
        ]);

        assert_eq!(record.record_field("ready"), Some(&CoreType::bool()));
        assert_eq!(record.record_field("count"), Some(&first_count));
        assert_eq!(record.record_field("missing"), None);
    }

    #[test]
    fn record_field_rejects_non_record_types() {
        assert_eq!(CoreType::Unknown.record_field("anything"), None);
        assert_eq!(CoreType::int().record_field("anything"), None);
        assert_eq!(
            CoreType::Function {
                params: vec![CoreType::int()],
                return_type: Box::new(CoreType::bool()),
                effects: vec!["io".to_string()],
            }
            .record_field("anything"),
            None
        );
    }
}
