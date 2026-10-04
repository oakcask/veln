use veln_analysis::ProjectAnalysis;
use veln_ast::{Function, FunctionKind, Param};
use veln_backend_jvm::{EntryArgScalar, EntryArgType};

pub(super) fn checked_entry_arg_types(
    analysis: &ProjectAnalysis,
    entry: &str,
    entry_args: &[String],
) -> Option<Vec<EntryArgType>> {
    let Some(entry_function) = find_entry_function(analysis, entry) else {
        eprintln!("veln: run entry `{entry}` was not found");
        return None;
    };
    match validate_entry_args(entry_function, entry_args) {
        Ok(entry_arg_types) => Some(entry_arg_types),
        Err(error) => {
            error.report(entry);
            None
        }
    }
}

fn find_entry_function<'a>(analysis: &'a ProjectAnalysis, entry: &str) -> Option<&'a Function> {
    analysis.module.functions.iter().find(|function| {
        function.kind == FunctionKind::Function && function.name.as_deref() == Some(entry)
    })
}

fn validate_entry_args(
    entry_function: &Function,
    entry_args: &[String],
) -> Result<Vec<EntryArgType>, EntryArgValidationError> {
    let fixed_param_count = entry_function
        .params
        .iter()
        .filter(|param| !param.is_variadic)
        .count();
    let variadic_param = entry_function.params.iter().find(|param| param.is_variadic);
    let count_is_valid = if variadic_param.is_some() {
        entry_args.len() >= fixed_param_count
    } else {
        entry_args.len() == fixed_param_count
    };
    if !count_is_valid {
        return Err(EntryArgValidationError::WrongCount {
            expected: if variadic_param.is_some() {
                EntryArgCount::AtLeast(fixed_param_count)
            } else {
                EntryArgCount::Exact(fixed_param_count)
            },
            actual: entry_args.len(),
        });
    }

    let mut entry_arg_types = checked_fixed_entry_arg_types(entry_function, entry_args)?;
    if let Some(param) = variadic_param {
        entry_arg_types.push(checked_variadic_entry_arg_type(
            param,
            &entry_args[fixed_param_count..],
        )?);
    }
    Ok(entry_arg_types)
}

fn checked_fixed_entry_arg_types(
    entry_function: &Function,
    entry_args: &[String],
) -> Result<Vec<EntryArgType>, EntryArgValidationError> {
    let mut entry_arg_types = Vec::new();
    for (param, raw_arg) in entry_function
        .params
        .iter()
        .filter(|param| !param.is_variadic)
        .zip(entry_args.iter())
    {
        let arg_type = checked_entry_arg_scalar(param, false)?;
        validate_entry_arg(arg_type, &param.name, raw_arg)
            .map_err(EntryArgValidationError::InvalidValue)?;
        entry_arg_types.push(entry_arg_type_from_scalar(arg_type));
    }
    Ok(entry_arg_types)
}

fn checked_variadic_entry_arg_type(
    param: &Param,
    entry_args: &[String],
) -> Result<EntryArgType, EntryArgValidationError> {
    let element_type = checked_entry_arg_scalar(param, true)?;
    for raw_arg in entry_args {
        validate_entry_arg(element_type, &param.name, raw_arg)
            .map_err(EntryArgValidationError::InvalidValue)?;
    }
    Ok(EntryArgType::VariadicList {
        element: element_type,
        count: entry_args.len(),
    })
}

fn checked_entry_arg_scalar(
    param: &Param,
    variadic: bool,
) -> Result<EntryArgScalar, EntryArgValidationError> {
    param
        .ty
        .as_deref()
        .and_then(entry_arg_scalar)
        .ok_or_else(|| EntryArgValidationError::UnsupportedParameter {
            name: param.name.clone(),
            variadic,
        })
}

fn entry_arg_scalar(ty: &str) -> Option<EntryArgScalar> {
    match ty {
        "String" => Some(EntryArgScalar::String),
        "Int" => Some(EntryArgScalar::Int),
        "Float" => Some(EntryArgScalar::Float),
        "Bool" => Some(EntryArgScalar::Bool),
        _ => None,
    }
}

fn entry_arg_type_from_scalar(ty: EntryArgScalar) -> EntryArgType {
    match ty {
        EntryArgScalar::String => EntryArgType::String,
        EntryArgScalar::Int => EntryArgType::Int,
        EntryArgScalar::Float => EntryArgType::Float,
        EntryArgScalar::Bool => EntryArgType::Bool,
    }
}

fn validate_entry_arg(ty: EntryArgScalar, param_name: &str, raw_arg: &str) -> Result<(), String> {
    match ty {
        EntryArgScalar::String => Ok(()),
        EntryArgScalar::Int => raw_arg.parse::<i64>().map(|_| ()).map_err(|_| {
            format!("veln: invalid Int argument for parameter `{param_name}`: `{raw_arg}`")
        }),
        EntryArgScalar::Float => raw_arg.parse::<f64>().map(|_| ()).map_err(|_| {
            format!("veln: invalid Float argument for parameter `{param_name}`: `{raw_arg}`")
        }),
        EntryArgScalar::Bool if raw_arg == "true" || raw_arg == "false" => Ok(()),
        EntryArgScalar::Bool => Err(format!(
            "veln: invalid Bool argument for parameter `{param_name}`: `{raw_arg}`"
        )),
    }
}

enum EntryArgCount {
    Exact(usize),
    AtLeast(usize),
}

enum EntryArgValidationError {
    WrongCount {
        expected: EntryArgCount,
        actual: usize,
    },
    UnsupportedParameter {
        name: String,
        variadic: bool,
    },
    InvalidValue(String),
}

impl EntryArgValidationError {
    fn report(self, entry: &str) {
        match self {
            Self::WrongCount { expected, actual } => {
                let expects = match expected {
                    EntryArgCount::Exact(count) => count.to_string(),
                    EntryArgCount::AtLeast(count) => format!("at least {count}"),
                };
                eprintln!("veln: run entry `{entry}` expects {expects} argument(s), got {actual}");
                eprintln!("veln: note: pass entry arguments after `--`");
            }
            Self::UnsupportedParameter { name, variadic } => {
                let argument = if variadic {
                    "command-line arguments"
                } else {
                    "a command-line argument"
                };
                eprintln!("veln: run entry parameter `{name}` cannot be supplied from {argument}");
                eprintln!(
                    "veln: note: supported entry argument types are String, Int, Float, and Bool"
                );
            }
            Self::InvalidValue(message) => eprintln!("{message}"),
        }
    }
}
