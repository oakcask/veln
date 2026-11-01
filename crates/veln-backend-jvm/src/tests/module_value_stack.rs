use super::*;

#[test]
fn nested_module_record_stack_tracks_the_emitted_operand_shape() {
    let mut source = String::new();
    for (name, depth) in [("boundary", 21), ("beyond", 24)] {
        let record_type = format!("{{ value: {}", "{ value: ".repeat(depth - 1));
        let initializer = format!("{{ value: {}", "{ value: ".repeat(depth - 1));
        let closings = " }".repeat(depth);
        source.push_str(&format!(
            "let {name}: {record_type}Int{closings} = {initializer}1{closings}\n"
        ));
    }
    source.push_str("pub fn main() -> ()\nend\n");

    let program = lower_to_ir(&source);

    assert_eq!(named_module_value_max_stack(&program, "boundary"), 65);
    assert_eq!(named_module_value_max_stack(&program, "beyond"), 74);
}
