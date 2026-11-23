use veln_core::CoreType;

use crate::semantic_model::Type;

trait SignatureType: Clone + PartialEq {
    fn unknown() -> Self;
    fn int() -> Self;
    fn bool() -> Self;
    fn unit() -> Self;
    fn named(name: &str, args: Vec<Self>) -> Self;
    fn record(fields: Vec<(String, Self)>) -> Self;
    fn function(params: Vec<Self>, return_type: Self, effects: Vec<String>) -> Self;
    fn named_args(&self, expected_name: &str) -> Option<&[Self]>;
    fn record_field(&self, field_name: &str) -> Option<&Self>;
    fn function_parts(&self) -> Option<(&[Self], &Self)>;
    fn function_effects(&self) -> Option<&[String]>;

    fn named_argument(&self, expected_name: &str) -> Option<&Self> {
        let args = self.named_args(expected_name)?;
        (args.len() == 1).then(|| &args[0])
    }

    fn option(value: Self) -> Self {
        Self::named("Option", vec![value])
    }

    fn option_part(&self) -> Option<&Self> {
        self.named_argument("Option")
    }

    fn result(value: Self, error: Self) -> Self {
        Self::named("Result", vec![value, error])
    }

    fn list(item: Self) -> Self {
        Self::named("List", vec![item])
    }

    fn result_value(&self) -> Option<&Self> {
        let args = self.named_args("Result")?;
        (args.len() == 2).then(|| &args[0])
    }

    fn list_part(&self) -> Option<&Self> {
        self.named_argument("List")
    }
}

macro_rules! common_signature_type_methods {
    () => {
        fn unknown() -> Self {
            Self::Unknown
        }

        fn int() -> Self {
            Self::int()
        }

        fn bool() -> Self {
            Self::bool()
        }

        fn unit() -> Self {
            Self::unit()
        }

        fn named(name: &str, args: Vec<Self>) -> Self {
            Self::named(name, args)
        }

        fn record(fields: Vec<(String, Self)>) -> Self {
            Self::Record(fields)
        }

        fn named_args(&self, expected_name: &str) -> Option<&[Self]> {
            match self {
                Self::Named { name, args, .. } if name == expected_name => Some(args),
                _ => None,
            }
        }

        fn record_field(&self, field_name: &str) -> Option<&Self> {
            Self::record_field(self, field_name)
        }
    };
}

impl SignatureType for Type {
    common_signature_type_methods!();

    fn option(value: Self) -> Self {
        Type::option(value)
    }

    fn result(value: Self, error: Self) -> Self {
        Type::result(value, error)
    }

    fn list(item: Self) -> Self {
        Type::list(item)
    }

    fn option_part(&self) -> Option<&Self> {
        crate::adt::type_operations::option_part(self)
    }

    fn result_value(&self) -> Option<&Self> {
        crate::adt::type_operations::result_parts(self).map(|(value, _)| value)
    }

    fn list_part(&self) -> Option<&Self> {
        crate::adt::type_operations::list_part(self)
    }

    fn function(params: Vec<Self>, return_type: Self, effects: Vec<String>) -> Self {
        Self::function(params, return_type, effects)
    }

    fn function_parts(&self) -> Option<(&[Self], &Self)> {
        Self::function_parts(self)
    }

    fn function_effects(&self) -> Option<&[String]> {
        Self::function_effects(self)
    }
}

impl SignatureType for CoreType {
    common_signature_type_methods!();

    fn function(params: Vec<Self>, return_type: Self, effects: Vec<String>) -> Self {
        Self::Function {
            params,
            variadic: None,
            return_type: Box::new(return_type),
            effects,
        }
    }

    fn function_parts(&self) -> Option<(&[Self], &Self)> {
        match self {
            Self::Function {
                params,
                return_type,
                ..
            } => Some((params, return_type)),
            _ => None,
        }
    }

    fn function_effects(&self) -> Option<&[String]> {
        match self {
            Self::Function { effects, .. } => Some(effects),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ConcurrencyArgumentTypes<'a, T> {
    pub(crate) handle: Option<&'a T>,
    pub(crate) context: Option<&'a T>,
    pub(crate) action: Option<&'a T>,
}

pub(crate) struct ConcurrencySignatureInputs<'a, T> {
    pub(crate) expected: Option<&'a T>,
    pub(crate) arguments: ConcurrencyArgumentTypes<'a, T>,
    pub(crate) explicit_item: Option<&'a T>,
    pub(crate) explicit_context: Option<&'a T>,
    pub(crate) explicit_result: Option<&'a T>,
}

pub(crate) fn concurrency_signature(
    segments: &[String],
    inputs: ConcurrencySignatureInputs<'_, Type>,
) -> Option<(Vec<Type>, Type)> {
    signature(segments, inputs)
}

pub(crate) fn core_concurrency_signature(
    segments: &[String],
    inputs: ConcurrencySignatureInputs<'_, CoreType>,
) -> Option<(Vec<CoreType>, CoreType)> {
    signature(segments, inputs)
}

fn signature<T: SignatureType>(
    segments: &[String],
    inputs: ConcurrencySignatureInputs<'_, T>,
) -> Option<(Vec<T>, T)> {
    let [module, name] = segments else {
        return None;
    };
    match module.as_str() {
        "channel" => channel_signature(
            name,
            inputs.expected,
            inputs.arguments.handle,
            inputs.explicit_item,
        ),
        "task" => task_signature(
            name,
            inputs.expected,
            inputs.arguments.handle,
            inputs.explicit_item,
            inputs.explicit_context,
        ),
        "io_provider" => io_provider_signature(name, inputs),
        _ => None,
    }
}

fn io_provider_signature<T: SignatureType>(
    name: &str,
    inputs: ConcurrencySignatureInputs<'_, T>,
) -> Option<(Vec<T>, T)> {
    match name {
        "state" => Some(io_provider_state_signature(inputs)),
        "update" => Some(io_provider_update_signature(inputs)),
        _ => None,
    }
}

fn io_provider_state_signature<T: SignatureType>(
    inputs: ConcurrencySignatureInputs<'_, T>,
) -> (Vec<T>, T) {
    let item = inputs
        .explicit_item
        .cloned()
        .or_else(|| inputs.arguments.handle.cloned())
        .or_else(|| {
            inputs
                .expected
                .and_then(|ty| provider_state_item(ty).cloned())
        })
        .unwrap_or_else(T::unknown);
    (vec![item.clone()], T::named("IoProviderState", vec![item]))
}

fn io_provider_update_signature<T: SignatureType>(
    inputs: ConcurrencySignatureInputs<'_, T>,
) -> (Vec<T>, T) {
    let action_parts = inputs
        .arguments
        .action
        .and_then(SignatureType::function_parts);
    let action_params = action_parts.map(|(params, _)| params);
    let action_return = action_parts.map(|(_, return_type)| return_type);
    let item = io_provider_update_item(&inputs, action_params);
    let context = io_provider_update_context(&inputs, action_params);
    let result = io_provider_update_result(&inputs, action_return);
    let transition = T::record(vec![
        ("state".to_string(), item.clone()),
        ("result".to_string(), result.clone()),
    ]);
    (
        vec![
            T::named("IoProviderState", vec![item.clone()]),
            context.clone(),
            T::function(vec![item, context], transition, Vec::new()),
        ],
        result,
    )
}

fn io_provider_update_item<T: SignatureType>(
    inputs: &ConcurrencySignatureInputs<'_, T>,
    action_params: Option<&[T]>,
) -> T {
    inputs
        .explicit_item
        .cloned()
        .or_else(|| {
            inputs
                .arguments
                .handle
                .and_then(provider_state_item)
                .filter(|ty| **ty != T::unknown())
                .cloned()
        })
        .or_else(|| action_params.and_then(|params| params.first()).cloned())
        .or_else(|| {
            inputs
                .arguments
                .handle
                .and_then(provider_state_item)
                .cloned()
        })
        .unwrap_or_else(T::unknown)
}

fn io_provider_update_context<T: SignatureType>(
    inputs: &ConcurrencySignatureInputs<'_, T>,
    action_params: Option<&[T]>,
) -> T {
    inputs
        .explicit_context
        .cloned()
        .or_else(|| action_params.and_then(|params| params.get(1)).cloned())
        .or_else(|| inputs.arguments.context.cloned())
        .unwrap_or_else(T::unknown)
}

fn io_provider_update_result<T: SignatureType>(
    inputs: &ConcurrencySignatureInputs<'_, T>,
    action_return: Option<&T>,
) -> T {
    inputs
        .explicit_result
        .cloned()
        .or_else(|| {
            action_return
                .and_then(|ty| ty.record_field("result"))
                .filter(|ty| **ty != T::unknown())
                .cloned()
        })
        .or_else(|| inputs.expected.filter(|ty| **ty != T::unknown()).cloned())
        .or_else(|| inputs.expected.cloned())
        .unwrap_or_else(T::unknown)
}

fn provider_state_item<T: SignatureType>(ty: &T) -> Option<&T> {
    ty.named_argument("IoProviderState")
        .or_else(|| ty.named_argument("State"))
}

fn channel_signature<T: SignatureType>(
    name: &str,
    expected: Option<&T>,
    handle_type: Option<&T>,
    explicit_item: Option<&T>,
) -> Option<(Vec<T>, T)> {
    match name {
        "bounded" => {
            let item = explicit_item
                .cloned()
                .or_else(|| channel_pair_item_type(expected))
                .unwrap_or_else(T::unknown);
            Some((
                vec![T::int()],
                T::record(vec![
                    ("tx".to_string(), T::named("Sender", vec![item.clone()])),
                    ("rx".to_string(), T::named("Receiver", vec![item])),
                ]),
            ))
        }
        "clone" => sender_clone_signature(handle_type),
        "send" => sender_send_signature(handle_type),
        "recv" => receiver_recv_signature(expected, handle_type),
        "select"
        | "select_priority"
        | "select_many_priority"
        | "select_many_timeout"
        | "select_many_timeout_result"
        | "select_many_timeout_cancellable"
        | "select_timeout"
        | "select_timeout_cancellable"
        | "select_result"
        | "select_priority_result"
        | "select_timeout_result" => select_signature(name, expected, handle_type),
        "close" => Some((vec![T::named("Sender", vec![T::unknown()])], T::unit())),
        _ => None,
    }
}

fn sender_clone_signature<T: SignatureType>(handle_type: Option<&T>) -> Option<(Vec<T>, T)> {
    let item = sender_item_type(handle_type);
    Some((
        vec![T::named("Sender", vec![item.clone()])],
        T::named("Sender", vec![item]),
    ))
}

fn sender_send_signature<T: SignatureType>(handle_type: Option<&T>) -> Option<(Vec<T>, T)> {
    let item = sender_item_type(handle_type);
    Some((
        vec![T::named("Sender", vec![item.clone()]), item],
        T::result(T::unit(), T::named("SendError", Vec::new())),
    ))
}

fn receiver_recv_signature<T: SignatureType>(
    expected: Option<&T>,
    handle_type: Option<&T>,
) -> Option<(Vec<T>, T)> {
    let item = expected
        .and_then(SignatureType::option_part)
        .cloned()
        .or_else(|| receiver_item_type(handle_type))
        .unwrap_or_else(T::unknown);
    Some((
        vec![T::named("Receiver", vec![item.clone()])],
        T::option(item),
    ))
}

fn select_signature<T: SignatureType>(
    name: &str,
    expected: Option<&T>,
    handle_type: Option<&T>,
) -> Option<(Vec<T>, T)> {
    let reports_interrupt = name.ends_with("_result")
        || matches!(
            name,
            "select_many_timeout_cancellable" | "select_timeout_cancellable"
        );
    let item = select_item_type(expected, reports_interrupt)
        .or_else(|| select_receiver_item_type(name, handle_type))
        .unwrap_or_else(T::unknown);
    let mut params = if is_many_select(name) {
        vec![T::list(T::named("Receiver", vec![item.clone()]))]
    } else {
        vec![
            T::named("Receiver", vec![item.clone()]),
            T::named("Receiver", vec![item.clone()]),
        ]
    };
    if has_timeout(name) {
        params.push(T::int());
    }
    if is_cancellable(name) {
        params.push(T::named("CancelToken", Vec::new()));
    }
    let output = T::option(select_output_record(item));
    let return_type = if reports_interrupt {
        T::result(output, T::named("SelectError", Vec::new()))
    } else {
        output
    };
    Some((params, return_type))
}

fn task_signature<T: SignatureType>(
    name: &str,
    expected: Option<&T>,
    handle_type: Option<&T>,
    explicit_item: Option<&T>,
    explicit_context: Option<&T>,
) -> Option<(Vec<T>, T)> {
    match name {
        "spawn" => task_spawn_signature(expected, handle_type, explicit_item),
        "spawn_with" => {
            task_spawn_with_signature(expected, handle_type, explicit_item, explicit_context)
        }
        "join" => task_join_signature(handle_type),
        "join_error_is_cancelled" => Some((vec![T::named("JoinError", Vec::new())], T::bool())),
        "cancel" => Some((vec![T::named("Task", vec![T::unknown()])], T::unit())),
        _ => None,
    }
}

fn task_spawn_signature<T: SignatureType>(
    expected: Option<&T>,
    handle_type: Option<&T>,
    explicit_item: Option<&T>,
) -> Option<(Vec<T>, T)> {
    let item = inferred_task_item(expected, handle_type, explicit_item);
    Some((
        vec![T::function(
            Vec::new(),
            item.clone(),
            function_effects(handle_type),
        )],
        T::named("Task", vec![item]),
    ))
}

fn task_spawn_with_signature<T: SignatureType>(
    expected: Option<&T>,
    handle_type: Option<&T>,
    explicit_item: Option<&T>,
    explicit_context: Option<&T>,
) -> Option<(Vec<T>, T)> {
    let arg = explicit_context
        .cloned()
        .or_else(|| {
            handle_type
                .and_then(SignatureType::function_parts)
                .and_then(|(params, _)| params.first())
                .cloned()
        })
        .unwrap_or_else(T::unknown);
    let item = inferred_task_item(expected, handle_type, explicit_item);
    Some((
        vec![
            T::function(
                vec![arg.clone()],
                item.clone(),
                function_effects(handle_type),
            ),
            arg,
        ],
        T::named("Task", vec![item]),
    ))
}

fn inferred_task_item<T: SignatureType>(
    expected: Option<&T>,
    handle_type: Option<&T>,
    explicit_item: Option<&T>,
) -> T {
    explicit_item
        .cloned()
        .or_else(|| expected.and_then(|ty| ty.named_argument("Task")).cloned())
        .or_else(|| {
            handle_type
                .and_then(SignatureType::function_parts)
                .map(|(_, return_type)| return_type.clone())
        })
        .unwrap_or_else(T::unknown)
}

fn task_join_signature<T: SignatureType>(handle_type: Option<&T>) -> Option<(Vec<T>, T)> {
    let item = handle_type
        .and_then(|ty| ty.named_argument("Task"))
        .cloned()
        .unwrap_or_else(T::unknown);
    Some((
        vec![T::named("Task", vec![item.clone()])],
        T::result(item, T::named("JoinError", Vec::new())),
    ))
}

fn function_effects<T: SignatureType>(ty: Option<&T>) -> Vec<String> {
    ty.and_then(SignatureType::function_effects)
        .map_or_else(Vec::new, <[String]>::to_vec)
}

fn sender_item_type<T: SignatureType>(handle_type: Option<&T>) -> T {
    handle_type
        .and_then(|ty| ty.named_argument("Sender"))
        .cloned()
        .unwrap_or_else(T::unknown)
}

fn receiver_item_type<T: SignatureType>(handle_type: Option<&T>) -> Option<T> {
    handle_type
        .and_then(|ty| ty.named_argument("Receiver"))
        .cloned()
}

fn select_receiver_item_type<T: SignatureType>(name: &str, handle_type: Option<&T>) -> Option<T> {
    if is_many_select(name) {
        handle_type
            .and_then(SignatureType::list_part)
            .and_then(|ty| ty.named_argument("Receiver"))
            .cloned()
    } else {
        receiver_item_type(handle_type)
    }
}

fn channel_pair_item_type<T: SignatureType>(expected: Option<&T>) -> Option<T> {
    let tx_item = expected?.record_field("tx")?.named_argument("Sender")?;
    let rx_item = expected?.record_field("rx")?.named_argument("Receiver")?;
    (tx_item == rx_item).then(|| tx_item.clone())
}

fn select_item_type<T: SignatureType>(expected: Option<&T>, reports_interrupt: bool) -> Option<T> {
    let output = if reports_interrupt {
        expected?.result_value()?
    } else {
        expected?
    };
    output.option_part()?.record_field("value").cloned()
}

fn select_output_record<T: SignatureType>(item: T) -> T {
    T::record(vec![
        ("index".to_string(), T::int()),
        ("value".to_string(), item),
    ])
}

fn is_many_select(name: &str) -> bool {
    matches!(
        name,
        "select_many_priority"
            | "select_many_timeout"
            | "select_many_timeout_result"
            | "select_many_timeout_cancellable"
    )
}

fn has_timeout(name: &str) -> bool {
    matches!(
        name,
        "select_many_timeout"
            | "select_many_timeout_result"
            | "select_many_timeout_cancellable"
            | "select_timeout"
            | "select_timeout_cancellable"
            | "select_timeout_result"
    )
}

fn is_cancellable(name: &str) -> bool {
    matches!(
        name,
        "select_many_timeout_cancellable" | "select_timeout_cancellable"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::type_lowering::core_type;

    #[test]
    fn surface_and_core_sender_send_signatures_share_inferred_item_type() {
        let path = ["channel".to_string(), "send".to_string()];
        let handle = Type::named("Sender", vec![Type::int()]);
        let surface = concurrency_signature(
            &path,
            ConcurrencySignatureInputs {
                expected: None,
                arguments: ConcurrencyArgumentTypes {
                    handle: Some(&handle),
                    context: None,
                    action: None,
                },
                explicit_item: None,
                explicit_context: None,
                explicit_result: None,
            },
        )
        .expect("surface signature should resolve");
        let core_handle = core_type(&handle);
        let core = core_concurrency_signature(
            &path,
            ConcurrencySignatureInputs {
                expected: None,
                arguments: ConcurrencyArgumentTypes {
                    handle: Some(&core_handle),
                    context: None,
                    action: None,
                },
                explicit_item: None,
                explicit_context: None,
                explicit_result: None,
            },
        )
        .expect("core signature should resolve");
        let expected_core = (
            surface.0.iter().map(core_type).collect::<Vec<_>>(),
            core_type(&surface.1),
        );

        assert_eq!(core, expected_core);
        assert_eq!(
            surface.0,
            vec![Type::named("Sender", vec![Type::int()]), Type::int()]
        );
    }

    #[test]
    fn provider_state_update_signature_keeps_a_pure_callback_boundary() {
        let path = ["io_provider".to_string(), "update".to_string()];
        let state = Type::named("IoProviderState", vec![Type::int()]);
        let expected = Type::int();
        let action = Type::function(
            vec![Type::int(), Type::unit()],
            Type::Record(vec![
                ("state".to_string(), Type::int()),
                ("result".to_string(), Type::string()),
            ]),
            Vec::new(),
        );
        let surface = concurrency_signature(
            &path,
            ConcurrencySignatureInputs {
                expected: Some(&expected),
                arguments: ConcurrencyArgumentTypes {
                    handle: Some(&state),
                    context: Some(&Type::unit()),
                    action: Some(&action),
                },
                explicit_item: None,
                explicit_context: None,
                explicit_result: None,
            },
        )
        .expect("provider state update signature should resolve");
        let Type::Function {
            params,
            return_type,
            effects,
            ..
        } = &surface.0[2]
        else {
            panic!("provider update action should remain a callback")
        };
        assert_eq!(params, &vec![Type::int(), Type::unit()]);
        assert_eq!(effects, &Vec::<String>::new());
        assert_eq!(surface.1, Type::string());
        assert_eq!(
            return_type.as_ref(),
            &Type::Record(vec![
                ("state".to_string(), Type::int()),
                ("result".to_string(), Type::string()),
            ])
        );

        let core_state = core_type(&state);
        let core_expected = core_type(&expected);
        let core_action = core_type(&action);
        let core = core_concurrency_signature(
            &path,
            ConcurrencySignatureInputs {
                expected: Some(&core_expected),
                arguments: ConcurrencyArgumentTypes {
                    handle: Some(&core_state),
                    context: Some(&CoreType::unit()),
                    action: Some(&core_action),
                },
                explicit_item: None,
                explicit_context: None,
                explicit_result: None,
            },
        )
        .expect("core provider state update signature should resolve");
        assert_eq!(
            core,
            (
                surface.0.iter().map(core_type).collect::<Vec<_>>(),
                core_type(&surface.1),
            )
        );
    }
}
