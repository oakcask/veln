use super::*;

use crate::semantic_model::Type;

#[test]
fn type_environment_collects_effect_function_and_handler_signatures() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Audit\n",
            "  record(value: Int) -> String\n",
            "end\n",
            "\n",
            "pub fn render(value: Int) -> String effects [Audit, Audit]\n",
            "  value::to_string()\n",
            "end\n",
            "\n",
            "pub handler recorder() handles Audit\n",
            "  record(value) => value::to_string()\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let environment = TypeEnvironment::from_module(&module);

    let effect = environment
        .user_effect_path(&["Audit".to_string()], None)
        .expect("effect signature should be present");
    assert_eq!(effect.operations.len(), 1);
    assert_eq!(effect.operations[0].name, "record");
    let function = environment
        .function("render")
        .expect("function signature should be present");
    assert_eq!(function.params, [Type::int()]);
    assert_eq!(function.return_type, Type::string());
    assert_eq!(function.effects, ["Audit"]);
    assert!(matches!(
        environment.handler_path(&["recorder".to_string()], None),
        HandlerPathResolution::Found(_)
    ));
}
