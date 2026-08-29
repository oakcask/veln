use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use veln_ast::{BinaryOp, ContractKind, PrefixOp};
use veln_ir::{
    ContractObligationStatus, IrCallTarget, IrContract, IrDictEntry, IrExpr, IrExprKind,
    IrFunction, IrHandlerProvider, IrMatchArm, IrPattern, IrPatternField, IrPatternKind,
    IrRecordField, IrSchemaDecodeSpec, IrStmt, IrStmtKind, TypedProgram,
};
use veln_literals::parse_integer_literal;

use crate::java::{sanitize_identifier_text, unique_java_identifier, veln_string_literal_value};
use crate::model::{EntryArgScalar, EntryArgType, JvmClassFile, JvmProgram, SanitizedOptions};
use crate::runtime::{
    binary_method, concurrency_method, prelude_method, standard_library_method, stdio_method,
};

mod call_emission;
mod class_builder;
mod constant_pool;
mod contract_emission;
mod contracts_and_tail_recursion;
mod function_flow;
mod method_code;
mod pattern_and_value_emission;
mod schema_invocation;
mod schema_metadata;

use class_builder::*;
use constant_pool::*;
use contracts_and_tail_recursion::{
    ValueRef, contract_integer_value, object_method_descriptor, parse_contract_call,
    runtime_classes, strip_contract_outer_parens,
};
pub(crate) use contracts_and_tail_recursion::{classify_tail_recursion, split_contract_binary};
pub(crate) use function_flow::TailRecursionEligibility;
use function_flow::{ContractCheckPosition, FunctionBytecodeEmitter};
use method_code::*;

const PROGRAM_MAJOR_VERSION: u16 = 49;
const JAVA_LANG_OBJECT: &str = "java/lang/Object";
const VELN_ENTRY: &str = "VelnEntry";

pub(crate) struct ClassfileEmitter<'a> {
    program: &'a TypedProgram,
    options: SanitizedOptions,
    function_names: BTreeMap<String, String>,
}

impl<'a> ClassfileEmitter<'a> {
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

    pub(crate) fn emit(
        &self,
        entry_function: &str,
        entry_arg_types: &[EntryArgType],
    ) -> JvmProgram {
        self.emit_with_entry_class(self.emit_entry_class(entry_function, entry_arg_types))
    }

    pub(crate) fn emit_test_entries(&self, entry_functions: &[String]) -> JvmProgram {
        self.emit_with_entry_class(self.emit_test_entry_class(entry_functions))
    }

    fn emit_with_entry_class(&self, entry_class: Vec<u8>) -> JvmProgram {
        let mut classes = runtime_classes();
        classes.push(JvmClassFile {
            path: format!("{}.class", self.options.program_class),
            contents: self.emit_program_class(),
        });
        classes.push(JvmClassFile {
            path: "VelnEntry.class".to_string(),
            contents: entry_class,
        });
        for function in &self.program.functions {
            classes.push(JvmClassFile {
                path: format!(
                    "{}${}.class",
                    self.options.program_class,
                    self.function_name(&function.name)
                ),
                contents: self.emit_function_adapter(function),
            });
        }
        JvmProgram { classes }
    }

    fn emit_program_class(&self) -> Vec<u8> {
        let mut class = ClassBuilder::new(&self.options.program_class);
        class.access_flags = 0x0031;
        class.add_default_constructor(0x0002);
        for function in &self.program.functions {
            let mut method = MethodCode::new(Rc::clone(&class.constant_pool));
            let mut emitter = FunctionBytecodeEmitter::new(self, function);
            emitter.emit(&mut method);
            class.add_method(MethodInfo {
                access_flags: 0x0008,
                name: self.function_name(&function.name),
                descriptor: object_method_descriptor(function.params.len()),
                max_stack: method.max_stack,
                max_locals: method.max_locals,
                code: method.code,
                exceptions: method.exceptions,
            });
        }
        class.finish()
    }

    fn emit_function_adapter(&self, function: &IrFunction) -> Vec<u8> {
        let adapter = format!(
            "{}${}",
            self.options.program_class,
            self.function_name(&function.name)
        );
        let mut class = ClassBuilder::new(&adapter);
        class.access_flags = 0x0031;
        class.interfaces.push(self.runtime_nested("Fn"));
        class.add_default_constructor(0x0001);

        let mut code = MethodCode::new(Rc::clone(&class.constant_pool));
        for index in 0..function.params.len() {
            code.aload(1);
            code.push_i32(index as i32);
            code.op(0x32);
        }
        code.invokestatic(
            &self.options.program_class,
            &self.function_name(&function.name),
            &object_method_descriptor(function.params.len()),
        );
        code.op(0xb0);
        class.add_method(MethodInfo {
            access_flags: 0x0081,
            name: "call".to_string(),
            descriptor: "([Ljava/lang/Object;)Ljava/lang/Object;".to_string(),
            max_stack: code.max_stack,
            max_locals: 2,
            code: code.code,
            exceptions: Vec::new(),
        });
        class.finish()
    }

    fn emit_entry_class(&self, entry_function: &str, entry_arg_types: &[EntryArgType]) -> Vec<u8> {
        let mut class = ClassBuilder::new(VELN_ENTRY);
        class.access_flags = 0x0031;
        class.add_default_constructor(0x0002);

        let mut code = MethodCode::new(Rc::clone(&class.constant_pool));
        let try_start = code.mark();
        code.aload(0);
        code.invokestatic(
            &self.options.runtime_class,
            "setProcessArgs",
            "([Ljava/lang/String;)V",
        );
        self.emit_entry_argument_conversions(&mut code, entry_arg_types);
        self.emit_entry_invocation_and_result(&mut code, entry_function, entry_arg_types);
        let try_end = code.mark();
        code.op(0xb1);

        self.emit_entry_contract_failure_handler(&mut code, try_start, try_end);
        self.emit_entry_runtime_failure_handler(&mut code, try_start, try_end);
        self.add_entry_main_method(&mut class, code);
        class.finish()
    }

    fn emit_test_entry_class(&self, entry_functions: &[String]) -> Vec<u8> {
        let mut class = ClassBuilder::new(VELN_ENTRY);
        class.access_flags = 0x0031;
        class.add_default_constructor(0x0002);

        let mut code = MethodCode::new(Rc::clone(&class.constant_pool));
        let try_start = code.mark();
        code.push_i32(0);
        code.istore(3);
        let loop_start = code.new_label();
        let loop_done = code.new_label();
        code.bind(loop_start);
        code.iload(3);
        code.aload(0);
        code.op(0xbe);
        code.branch_to(0xa2, loop_done);

        let dispatched = code.new_label();
        for entry_function in entry_functions {
            code.aload(0);
            code.iload(3);
            code.op(0x32);
            code.ldc_string(entry_function);
            code.invokevirtual("java/lang/String", "equals", "(Ljava/lang/Object;)Z");
            let next = code.branch(0x99);
            code.invokestatic(
                &self.options.program_class,
                &self.function_name(entry_function),
                &object_method_descriptor(0),
            );
            code.astore(1);
            self.emit_entry_result(&mut code);
            code.branch_to(0xa7, dispatched);
            code.bind(next);
        }
        code.getstatic("java/lang/System", "err", "Ljava/io/PrintStream;");
        code.ldc_string("unknown test entry");
        code.invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V");
        code.push_i32(1);
        code.invokestatic("java/lang/System", "exit", "(I)V");
        code.op(0xb1);

        code.bind(dispatched);
        code.iinc(3, 1);
        code.branch_to(0xa7, loop_start);
        code.bind(loop_done);
        let try_end = code.mark();
        code.op(0xb1);

        self.emit_entry_contract_failure_handler(&mut code, try_start, try_end);
        self.emit_entry_runtime_failure_handler(&mut code, try_start, try_end);
        self.add_entry_main_method(&mut class, code);
        class.finish()
    }

    fn emit_entry_argument_conversions(
        &self,
        code: &mut MethodCode,
        entry_arg_types: &[EntryArgType],
    ) {
        let mut raw_index = 0usize;
        for ty in entry_arg_types {
            match ty {
                EntryArgType::String => {
                    self.emit_entry_scalar_argument(code, raw_index, EntryArgScalar::String);
                    raw_index += 1;
                }
                EntryArgType::Int => {
                    self.emit_entry_scalar_argument(code, raw_index, EntryArgScalar::Int);
                    raw_index += 1;
                }
                EntryArgType::Float => {
                    self.emit_entry_scalar_argument(code, raw_index, EntryArgScalar::Float);
                    raw_index += 1;
                }
                EntryArgType::Bool => {
                    self.emit_entry_scalar_argument(code, raw_index, EntryArgScalar::Bool);
                    raw_index += 1;
                }
                EntryArgType::VariadicList { element, count } => {
                    self.emit_entry_variadic_list(code, raw_index, *element, *count);
                    raw_index += count;
                }
            }
        }
    }

    fn emit_entry_scalar_argument(
        &self,
        code: &mut MethodCode,
        raw_index: usize,
        ty: EntryArgScalar,
    ) {
        code.aload(0);
        code.push_i32(raw_index as i32);
        code.op(0x32);
        self.emit_entry_argument_conversion(code, ty);
    }

    fn emit_entry_argument_conversion(&self, code: &mut MethodCode, ty: EntryArgScalar) {
        match ty {
            EntryArgScalar::String => {}
            EntryArgScalar::Int => {
                code.invokestatic("java/lang/Long", "parseLong", "(Ljava/lang/String;)J");
                code.invokestatic("java/lang/Long", "valueOf", "(J)Ljava/lang/Long;");
            }
            EntryArgScalar::Float => {
                code.invokestatic("java/lang/Double", "parseDouble", "(Ljava/lang/String;)D");
                code.invokestatic("java/lang/Double", "valueOf", "(D)Ljava/lang/Double;");
            }
            EntryArgScalar::Bool => {
                code.invokestatic(
                    "java/lang/Boolean",
                    "valueOf",
                    "(Ljava/lang/String;)Ljava/lang/Boolean;",
                );
            }
        }
    }

    fn emit_entry_variadic_list(
        &self,
        code: &mut MethodCode,
        raw_start: usize,
        element: EntryArgScalar,
        count: usize,
    ) {
        code.invokestatic(
            &self.options.runtime_class,
            "listNil",
            "()Ljava/lang/Object;",
        );
        for index in (0..count).rev() {
            self.emit_entry_scalar_argument(code, raw_start + index, element);
            code.op(0x5f);
            code.invokestatic(
                &self.options.runtime_class,
                "listCons",
                "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            );
        }
    }

    fn emit_entry_invocation_and_result(
        &self,
        code: &mut MethodCode,
        entry_function: &str,
        entry_arg_types: &[EntryArgType],
    ) {
        code.invokestatic(
            &self.options.program_class,
            &self.function_name(entry_function),
            &object_method_descriptor(entry_arg_types.len()),
        );
        code.astore(1);
        self.emit_entry_result(code);
    }

    fn emit_entry_result(&self, code: &mut MethodCode) {
        code.aload(1);
        code.invokestatic(
            &self.options.runtime_class,
            "isErr",
            "(Ljava/lang/Object;)Z",
        );
        let result_ok = code.branch(0x99);
        self.emit_entry_failure(code, 1);
        code.bind(result_ok);

        self.emit_projected_entry_failure(code, "isEncodeStepInvalid", "encodeStepInvalidAsErr");
        self.emit_projected_entry_failure(code, "isDecodeStepInvalid", "decodeStepInvalidAsErr");
        self.emit_projected_entry_failure(code, "isDecodeStepNeedMore", "decodeStepNeedMoreAsErr");
    }

    fn emit_projected_entry_failure(
        &self,
        code: &mut MethodCode,
        predicate: &str,
        projection: &str,
    ) {
        code.aload(1);
        code.invokestatic(
            &self.options.runtime_class,
            predicate,
            "(Ljava/lang/Object;)Z",
        );
        let ok = code.branch(0x99);
        code.aload(1);
        code.invokestatic(
            &self.options.runtime_class,
            projection,
            "(Ljava/lang/Object;)Ljava/lang/Object;",
        );
        code.astore(2);
        self.emit_entry_failure(code, 2);
        code.bind(ok);
    }

    fn emit_entry_failure(&self, code: &mut MethodCode, local: u16) {
        code.aload(local);
        code.invokestatic(
            &self.options.runtime_class,
            "recordResultFailure",
            "(Ljava/lang/Object;)V",
        );
        code.getstatic("java/lang/System", "err", "Ljava/io/PrintStream;");
        code.aload(local);
        code.invokestatic(
            &self.options.runtime_class,
            "format",
            "(Ljava/lang/Object;)Ljava/lang/String;",
        );
        code.invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V");
        code.push_i32(1);
        code.invokestatic("java/lang/System", "exit", "(I)V");
    }

    fn emit_entry_contract_failure_handler(
        &self,
        code: &mut MethodCode,
        try_start: usize,
        try_end: usize,
    ) {
        let handler = code.mark();
        code.astore(2);
        code.aload(2);
        code.invokestatic(
            &self.options.runtime_class,
            "recordContractFailure",
            &format!("(L{}$ContractFailure;)V", self.options.runtime_class),
        );
        code.getstatic("java/lang/System", "err", "Ljava/io/PrintStream;");
        code.aload(2);
        code.invokevirtual("java/lang/Throwable", "getMessage", "()Ljava/lang/String;");
        code.invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V");
        code.push_i32(1);
        code.invokestatic("java/lang/System", "exit", "(I)V");
        code.op(0xb1);
        code.exceptions.push(ExceptionHandler {
            start_pc: try_start,
            end_pc: try_end,
            handler_pc: handler,
            catch_type: self.runtime_nested("ContractFailure"),
        });
    }

    fn emit_entry_runtime_failure_handler(
        &self,
        code: &mut MethodCode,
        try_start: usize,
        try_end: usize,
    ) {
        let handler = code.mark();
        code.astore(2);
        code.getstatic("java/lang/System", "err", "Ljava/io/PrintStream;");
        code.aload(2);
        code.invokevirtual("java/lang/Throwable", "getMessage", "()Ljava/lang/String;");
        code.invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V");
        code.push_i32(1);
        code.invokestatic("java/lang/System", "exit", "(I)V");
        code.op(0xb1);
        code.exceptions.push(ExceptionHandler {
            start_pc: try_start,
            end_pc: try_end,
            handler_pc: handler,
            catch_type: self.runtime_nested("RuntimeFailure"),
        });
    }

    fn add_entry_main_method(&self, class: &mut ClassBuilder, code: MethodCode) {
        class.add_method(MethodInfo {
            access_flags: 0x0009,
            name: "main".to_string(),
            descriptor: "([Ljava/lang/String;)V".to_string(),
            max_stack: code.max_stack,
            max_locals: code.max_locals.max(3),
            code: code.code,
            exceptions: code.exceptions,
        });
    }

    fn function_name(&self, name: &str) -> String {
        self.function_names
            .get(name)
            .cloned()
            .unwrap_or_else(|| format!("fn_{}", sanitize_identifier_text(name)))
    }

    fn runtime_nested(&self, nested: &str) -> String {
        format!("{}${nested}", self.options.runtime_class)
    }
}
