use super::*;
use crate::semantic_model::Type;
use crate::type_relations::is_assignable;

fn rigid_owner_identities(
    package_name: Option<&str>,
    module_name: Option<&str>,
) -> (String, String) {
    let source = SourceFile::new("owner.veln", "fn owner<T>(value: T) -> T\n  value\nend\n");
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let mut module = lower_surface_ast(&parsed.tree);
    let function = &mut module.functions[0];
    function.package_name = package_name.map(str::to_string);
    function.module_name = module_name.map(str::to_string);

    let signature = crate::types::TypeParameterIdentityContext::function_parameters(function);
    let annotations = crate::types::TypeParameterIdentityContext::for_function(function);
    (
        signature[0].identity.clone(),
        annotations
            .identity("T")
            .expect("annotation binder identity"),
    )
}

fn assert_rigid_owners_are_distinct(
    left_owner: (Option<&str>, Option<&str>),
    right_owner: (Option<&str>, Option<&str>),
) {
    let left = rigid_owner_identities(left_owner.0, left_owner.1);
    let right = rigid_owner_identities(right_owner.0, right_owner.1);
    assert_eq!(left.0, left.1);
    assert_eq!(right.0, right.1);
    assert_ne!(left.0, right.0);

    let left_type = Type::resolved_named("T", left.0, Vec::new());
    let right_type = Type::resolved_named("T", right.0, Vec::new());
    assert!(!is_assignable(&left_type, &right_type));
    assert!(!is_assignable(&right_type, &left_type));
}

#[test]
fn rigid_parameter_owner_keys_preserve_option_boundaries() {
    assert_rigid_owners_are_distinct(
        (None, Some("subject")),
        (Some("workspace"), Some("subject")),
    );
}

#[test]
fn rigid_parameter_owner_keys_preserve_component_boundaries() {
    assert_rigid_owners_are_distinct(
        (Some("pkg:api"), Some("part")),
        (Some("pkg"), Some("api:part")),
    );
}
