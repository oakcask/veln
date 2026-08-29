use super::*;

#[derive(Clone)]
pub(super) struct SourcePreludeSignature {
    pub(super) name: String,
    pub(super) params: Vec<Type>,
    pub(super) return_type: Type,
}

static SOURCE_PRELUDE_CALLBACK_SIGNATURES: OnceLock<Vec<SourcePreludeSignature>> = OnceLock::new();

pub(super) fn compiler_adapter_callback_signature(
    descriptor: &StandardSymbolDescriptor,
) -> Option<(Vec<Type>, Type)> {
    source_prelude_callback_signatures()
        .iter()
        .find(|signature| signature.name == descriptor.name)
        .map(|signature| (signature.params.clone(), signature.return_type.clone()))
}

pub(super) fn source_prelude_callback_signatures() -> &'static [SourcePreludeSignature] {
    SOURCE_PRELUDE_CALLBACK_SIGNATURES
        .get_or_init(|| {
            let package = veln_stdlib::package_bundle();
            let source = package
                .files
                .iter()
                .find(|file| file.path == "prelude.veln")
                .expect("standard package should contain prelude.veln");
            source_prelude_callback_signatures_from_text(source.path, source.text)
        })
        .as_slice()
}

pub(super) fn source_prelude_callback_signatures_from_text(
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

pub(super) fn source_prelude_known_type_names(module: &SurfaceModule) -> BTreeSet<String> {
    let mut known = [
        "Bool", "Int", "Float", "String", "Unit", "Option", "Result", "List", "Vec", "Dict",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<BTreeSet<_>>();
    known.extend(module.types.iter().filter_map(|ty| ty.name.clone()));
    known
}

pub(super) fn source_prelude_concrete_type(ty: &Type, known_types: &BTreeSet<String>) -> Type {
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

pub(super) fn source_prelude_type_name_is_known(
    name: &str,
    known_types: &BTreeSet<String>,
) -> bool {
    known_types.contains(name)
        || name
            .rsplit("::")
            .next()
            .is_some_and(|last| known_types.contains(last))
}

pub(super) fn concrete_function_parameter(ty: &Type) -> bool {
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
