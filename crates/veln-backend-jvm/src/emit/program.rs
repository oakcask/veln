use std::collections::{BTreeMap, BTreeSet};

use veln_ir::TypedProgram;

use crate::api::{EntryArgType, JavaProgram, JavaSourceFile, SanitizedOptions};
use crate::emit::function::FunctionEmitter;
use crate::java::{sanitize_identifier_text, unique_java_identifier};

pub(crate) struct ProgramEmitter<'a> {
    program: &'a TypedProgram,
    pub(crate) options: SanitizedOptions,
    function_names: BTreeMap<String, String>,
}

impl<'a> ProgramEmitter<'a> {
    pub(crate) fn new(program: &'a TypedProgram, options: SanitizedOptions) -> Self {
        let mut function_names = BTreeMap::new();
        let mut used_names = BTreeSet::new();
        for function in &program.functions {
            let name = unique_java_identifier(
                &format!("fn_{}", sanitize_identifier_text(&function.name)),
                &mut used_names,
            );
            function_names.insert(function.name.clone(), name);
        }
        Self {
            program,
            options,
            function_names,
        }
    }

    pub(crate) fn emit(&self) -> JavaProgram {
        JavaProgram {
            sources: vec![
                JavaSourceFile {
                    path: format!("{}.java", self.options.program_class),
                    contents: self.emit_program_class(),
                },
                JavaSourceFile {
                    path: format!("{}.java", self.options.runtime_class),
                    contents: self.emit_runtime_class(),
                },
            ],
        }
    }

    pub(crate) fn emit_with_entry(
        &self,
        entry_function: &str,
        entry_arg_types: &[EntryArgType],
    ) -> JavaProgram {
        let mut program = self.emit();
        program.sources.push(JavaSourceFile {
            path: "VelnEntry.java".to_string(),
            contents: self.emit_entry_class(entry_function, entry_arg_types),
        });
        program
    }

    fn emit_program_class(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "public final class {} {{\n",
            self.options.program_class
        ));
        out.push_str(&format!(
            "    private {}() {{}}\n\n",
            self.options.program_class
        ));
        for (index, function) in self.program.functions.iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            let mut function_emitter = FunctionEmitter::new(self, function);
            function_emitter.emit(&mut out);
        }
        out.push_str("}\n");
        out
    }

    fn emit_entry_class(&self, entry_function: &str, entry_arg_types: &[EntryArgType]) -> String {
        let function_name = self.function_name(entry_function);
        let args = entry_arg_types
            .iter()
            .enumerate()
            .map(|(index, ty)| entry_arg_value(*ty, index))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            r#"public final class VelnEntry {{
    private VelnEntry() {{}}

    public static void main(String[] args) {{
        try {{
            {runtime}.setProcessArgs(args);
            Object result = {program}.{function_name}({entry_args});
            if ({runtime}.isErr(result)) {{
                System.err.println({runtime}.format(result));
                System.exit(1);
            }}
        }} catch ({runtime}.ContractFailure error) {{
            {runtime}.recordContractFailure(error);
            System.err.println(error.getMessage());
            System.exit(1);
        }}
    }}

    private static Object argInt(String text, String name) {{
        try {{
            return Long.valueOf(Long.parseLong(text));
        }} catch (NumberFormatException error) {{
            System.err.println("veln: invalid Int argument `" + name + "`: `" + text + "`");
            System.exit(1);
            return null;
        }}
    }}

    private static Object argFloat(String text, String name) {{
        try {{
            return Double.valueOf(Double.parseDouble(text));
        }} catch (NumberFormatException error) {{
            System.err.println("veln: invalid Float argument `" + name + "`: `" + text + "`");
            System.exit(1);
            return null;
        }}
    }}

    private static Object argBool(String text, String name) {{
        if ("true".equals(text)) {{
            return Boolean.TRUE;
        }}
        if ("false".equals(text)) {{
            return Boolean.FALSE;
        }}
        System.err.println("veln: invalid Bool argument `" + name + "`: `" + text + "`");
        System.exit(1);
        return null;
    }}
}}
"#,
            program = self.options.program_class,
            runtime = self.options.runtime_class,
            function_name = function_name,
            entry_args = args,
        )
    }

    fn emit_runtime_class(&self) -> String {
        include_str!("../../runtime/VelnRuntime.java")
            .replace("VelnRuntime", &self.options.runtime_class)
    }

    pub(crate) fn function_name(&self, name: &str) -> String {
        self.function_names
            .get(name)
            .cloned()
            .unwrap_or_else(|| format!("fn_{}", sanitize_identifier_text(name)))
    }

    pub(crate) fn function(&self, name: &str) -> Option<&veln_ir::IrFunction> {
        self.program
            .functions
            .iter()
            .find(|function| function.name == name)
    }
}

fn entry_arg_value(ty: EntryArgType, index: usize) -> String {
    match ty {
        EntryArgType::String => format!("args[{index}]"),
        EntryArgType::Int => format!("argInt(args[{index}], \"{index}\")"),
        EntryArgType::Float => format!("argFloat(args[{index}], \"{index}\")"),
        EntryArgType::Bool => format!("argBool(args[{index}], \"{index}\")"),
    }
}
