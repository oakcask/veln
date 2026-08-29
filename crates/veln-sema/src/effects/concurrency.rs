use veln_core::CoreType;

use crate::semantic_model::Type;

trait SignatureType: Clone + PartialEq {
    fn unknown() -> Self;
    fn int() -> Self;
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

    fn result_value(&self) -> Option<&Self> {
        let args = self.named_args("Result")?;
        (args.len() == 2).then(|| &args[0])
    }
}

impl SignatureType for Type {
    fn unknown() -> Self {
        Self::Unknown
    }

    fn int() -> Self {
        Self::int()
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

    fn function(params: Vec<Self>, return_type: Self, effects: Vec<String>) -> Self {
        Self::function(params, return_type, effects)
    }

    fn named_args(&self, expected_name: &str) -> Option<&[Self]> {
        match self {
            Self::Named { name, args } if name == expected_name => Some(args),
            _ => None,
        }
    }

    fn record_field(&self, field_name: &str) -> Option<&Self> {
        Self::record_field(self, field_name)
    }

    fn function_parts(&self) -> Option<(&[Self], &Self)> {
        Self::function_parts(self)
    }

    fn function_effects(&self) -> Option<&[String]> {
        Self::function_effects(self)
    }
}

impl SignatureType for CoreType {
    fn unknown() -> Self {
        Self::Unknown
    }

    fn int() -> Self {
        Self::int()
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

    fn function(params: Vec<Self>, return_type: Self, effects: Vec<String>) -> Self {
        Self::Function {
            params,
            variadic: None,
            return_type: Box::new(return_type),
            effects,
        }
    }

    fn named_args(&self, expected_name: &str) -> Option<&[Self]> {
        match self {
            Self::Named { name, args } if name == expected_name => Some(args),
            _ => None,
        }
    }

    fn record_field(&self, field_name: &str) -> Option<&Self> {
        Self::record_field(self, field_name)
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

pub(crate) fn concurrency_signature(
    segments: &[String],
    expected: Option<&Type>,
    handle_type: Option<&Type>,
    explicit_item: Option<&Type>,
    explicit_context: Option<&Type>,
) -> Option<(Vec<Type>, Type)> {
    signature(
        segments,
        expected,
        handle_type,
        explicit_item,
        explicit_context,
    )
}

pub(crate) fn core_concurrency_signature(
    segments: &[String],
    expected: Option<&CoreType>,
    handle_type: Option<&CoreType>,
    explicit_item: Option<&CoreType>,
    explicit_context: Option<&CoreType>,
) -> Option<(Vec<CoreType>, CoreType)> {
    signature(
        segments,
        expected,
        handle_type,
        explicit_item,
        explicit_context,
    )
}

fn signature<T: SignatureType>(
    segments: &[String],
    expected: Option<&T>,
    handle_type: Option<&T>,
    explicit_item: Option<&T>,
    explicit_context: Option<&T>,
) -> Option<(Vec<T>, T)> {
    let [module, name] = segments else {
        return None;
    };
    match module.as_str() {
        "channel" => channel_signature(name, expected, handle_type, explicit_item),
        "task" => task_signature(name, expected, handle_type, explicit_item, explicit_context),
        _ => None,
    }
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
        vec![T::named(
            "List",
            vec![T::named("Receiver", vec![item.clone()])],
        )]
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
            .and_then(|ty| ty.named_argument("List"))
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
        let surface = concurrency_signature(&path, None, Some(&handle), None, None)
            .expect("surface signature should resolve");
        let core_handle = core_type(&handle);
        let core = core_concurrency_signature(&path, None, Some(&core_handle), None, None)
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
}
