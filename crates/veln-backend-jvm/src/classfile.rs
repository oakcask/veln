use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;

use veln_ast::{BinaryOp, ContractKind, PredicateRemainder, PrefixOp};
use veln_ir::{
    ContractObligationStatus, CoreContractCall, CoreModuleValueIdentity, CoreType, IrCallTarget,
    IrCallbackTarget, IrContract, IrDeferredBlock, IrDictEntry, IrExpr, IrExprKind, IrFunction,
    IrHandlerProvider, IrMatchArm, IrPattern, IrPatternField, IrPatternKind, IrRecordField,
    IrSchemaDecodeDispatchCase, IrSchemaDecodeSpec, IrStmt, IrStmtKind, TypedProgram,
};
use veln_literals::parse_integer_literal;

use crate::java::{sanitize_identifier_text, unique_java_identifier, veln_string_literal_value};
use crate::model::{
    EntryArgScalar, EntryArgType, JvmBackendError, JvmClassFile, JvmProgram, SanitizedOptions,
};
use crate::runtime::{
    binary_method, concurrency_method, prelude_method, standard_library_method, stdio_method,
};

mod call_emission;
mod class_builder;
mod constant_pool;
mod contract_emission;
mod contract_value;
mod contracts_and_tail_recursion;
mod function_flow;
mod local_bindings;
mod method_code;
mod pattern_and_value_emission;
mod schema_invocation;
mod schema_metadata;

use class_builder::*;
use constant_pool::*;
#[cfg(test)]
pub(crate) use contract_value::split_contract_binary;
use contract_value::{
    ContractScalar, ContractValue, parse_contract_value, strip_contract_outer_parens,
};
pub(crate) use contracts_and_tail_recursion::classify_tail_recursion;
use contracts_and_tail_recursion::{ValueRef, object_method_descriptor, runtime_classes};
pub(crate) use function_flow::TailRecursionEligibility;
use function_flow::{ContractCheckPosition, FunctionBytecodeEmitter};
use method_code::*;

const PROGRAM_MAJOR_VERSION: u16 = 49;
const JAVA_LANG_OBJECT: &str = "java/lang/Object";
const VELN_ENTRY: &str = "VelnEntry";
const MODULE_VALUE_DESCRIPTOR: &str = "Ljava/lang/Object;";
const MODULE_VALUES_PER_CLASS: usize = 1_000;

fn collect_module_value_dependencies(
    expr: &IrExpr,
    dependencies: &mut BTreeSet<CoreModuleValueIdentity>,
) {
    let mut pending = vec![expr];
    while let Some(expr) = pending.pop() {
        match &expr.kind {
            IrExprKind::ModuleValue(identity) => {
                dependencies.insert(identity.clone());
            }
            IrExprKind::Record(fields) => {
                pending.extend(fields.iter().rev().map(|field| &field.value));
            }
            IrExprKind::AdtVariant { payloads, .. } => {
                pending.extend(payloads.iter().rev());
            }
            IrExprKind::OptionSome(value)
            | IrExprKind::ResultOk(value)
            | IrExprKind::ResultErr(value)
            | IrExprKind::Prefix { expr: value, .. } => pending.push(value),
            IrExprKind::ListCons { head, tail } => {
                pending.push(tail);
                pending.push(head);
            }
            IrExprKind::BoolLiteral(_)
            | IrExprKind::StringLiteral(_)
            | IrExprKind::IntLiteral(_)
            | IrExprKind::OptionNone
            | IrExprKind::ListNil
            | IrExprKind::Unit => {}
            kind => {
                unreachable!("unsupported module value initializer reached JVM emission: {kind:?}")
            }
        }
    }
}

fn module_value_initializer_max_stack(expr: &IrExpr) -> usize {
    enum Work<'a> {
        Visit(&'a IrExpr),
        Unary,
        ListCons,
        Record(usize),
        AdtVariant(usize),
    }

    let mut work = vec![Work::Visit(expr)];
    let mut values = Vec::new();
    while let Some(item) = work.pop() {
        match item {
            Work::Visit(expr) => match &expr.kind {
                IrExprKind::StringLiteral(_)
                | IrExprKind::BoolLiteral(_)
                | IrExprKind::Unit
                | IrExprKind::ModuleValue(_)
                | IrExprKind::OptionNone
                | IrExprKind::ListNil => values.push(1usize),
                IrExprKind::IntLiteral(_) => values.push(2usize),
                IrExprKind::Prefix {
                    op: PrefixOp::Negate,
                    expr,
                }
                | IrExprKind::ResultOk(expr)
                | IrExprKind::ResultErr(expr)
                | IrExprKind::OptionSome(expr) => {
                    work.push(Work::Unary);
                    work.push(Work::Visit(expr));
                }
                IrExprKind::ListCons { head, tail } => {
                    work.push(Work::ListCons);
                    work.push(Work::Visit(tail));
                    work.push(Work::Visit(head));
                }
                IrExprKind::Record(fields) => {
                    work.push(Work::Record(fields.len()));
                    work.extend(fields.iter().rev().map(|field| Work::Visit(&field.value)));
                }
                IrExprKind::AdtVariant { payloads, .. } => {
                    work.push(Work::AdtVariant(payloads.len()));
                    work.extend(payloads.iter().rev().map(Work::Visit));
                }
                kind => unreachable!(
                    "unsupported module value initializer reached JVM emission: {kind:?}"
                ),
            },
            Work::Unary => {
                debug_assert!(!values.is_empty());
            }
            Work::ListCons => {
                let tail = values.pop().expect("module value list tail stack");
                let head = values.pop().expect("module value list head stack");
                values.push(head.max(1usize.saturating_add(tail)));
            }
            Work::Record(count) => {
                let start = values.len() - count;
                let maximum = values.drain(start..).fold(1usize, |maximum, value| {
                    maximum.max(3usize.saturating_add(value))
                });
                values.push(maximum);
            }
            Work::AdtVariant(count) => {
                let start = values.len() - count;
                let maximum = values.drain(start..).fold(2usize, |maximum, value| {
                    maximum.max(4usize.saturating_add(value))
                });
                values.push(maximum);
            }
        }
    }

    values.pop().expect("module value initializer stack")
}

fn emit_module_value_initializer(
    emitter: &mut FunctionBytecodeEmitter<'_, '_>,
    code: &mut MethodCode,
    initializer: &IrExpr,
) -> Result<(), JvmBackendError> {
    enum Work<'a> {
        Expr(&'a IrExpr),
        PushInt(i32),
        String(&'a str),
        AdtName(&'a [String]),
        NewObjectArray(usize),
        Dup,
        ArrayStore,
        InvokeStatic {
            method: &'static str,
            descriptor: &'static str,
        },
        InvokeRuntime {
            method: &'static str,
            descriptor: &'static str,
        },
    }

    let runtime_class = emitter.program.options.runtime_class.clone();
    let mut work = vec![Work::Expr(initializer)];
    while let Some(item) = work.pop() {
        match item {
            Work::Expr(expr) => match &expr.kind {
                IrExprKind::ModuleValue(identity) => code.getstatic(
                    emitter.program.module_value_owner(identity),
                    emitter.program.module_value_name(identity),
                    MODULE_VALUE_DESCRIPTOR,
                ),
                IrExprKind::BoolLiteral(value) => emitter.emit_bool_literal(code, *value),
                IrExprKind::StringLiteral(value) => emitter.emit_string_literal(code, value),
                IrExprKind::IntLiteral(value) => emitter.emit_int_literal(code, value),
                IrExprKind::Unit => emitter.emit_unit(code),
                IrExprKind::OptionNone => emitter.emit_option_none(code),
                IrExprKind::ListNil => emitter.emit_list_nil(code),
                IrExprKind::Prefix {
                    op: PrefixOp::Negate,
                    expr,
                } => {
                    work.push(Work::InvokeRuntime {
                        method: "negate",
                        descriptor: "(Ljava/lang/Object;)Ljava/lang/Object;",
                    });
                    work.push(Work::Expr(expr));
                }
                IrExprKind::ResultOk(value) => {
                    work.push(Work::InvokeRuntime {
                        method: "ok",
                        descriptor: "(Ljava/lang/Object;)LVelnRuntime$Result;",
                    });
                    work.push(Work::Expr(value));
                }
                IrExprKind::ResultErr(value) => {
                    work.push(Work::InvokeRuntime {
                        method: "err",
                        descriptor: "(Ljava/lang/Object;)LVelnRuntime$Result;",
                    });
                    work.push(Work::Expr(value));
                }
                IrExprKind::OptionSome(value) => {
                    work.push(Work::InvokeRuntime {
                        method: "some",
                        descriptor: "(Ljava/lang/Object;)LVelnRuntime$Option;",
                    });
                    work.push(Work::Expr(value));
                }
                IrExprKind::ListCons { head, tail } => {
                    work.push(Work::InvokeStatic {
                        method: "listCons",
                        descriptor: "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                    });
                    work.push(Work::Expr(tail));
                    work.push(Work::Expr(head));
                }
                IrExprKind::Record(fields) => {
                    work.push(Work::InvokeStatic {
                        method: "record",
                        descriptor: "([Ljava/lang/Object;)Ljava/util/Map;",
                    });
                    for (index, field) in fields.iter().enumerate().rev() {
                        work.push(Work::ArrayStore);
                        work.push(Work::Expr(&field.value));
                        work.push(Work::PushInt((index * 2 + 1) as i32));
                        work.push(Work::Dup);
                        work.push(Work::ArrayStore);
                        work.push(Work::String(&field.name));
                        work.push(Work::PushInt((index * 2) as i32));
                        work.push(Work::Dup);
                    }
                    work.push(Work::NewObjectArray(fields.len() * 2));
                }
                IrExprKind::AdtVariant { name, payloads } => {
                    work.push(Work::InvokeStatic {
                        method: "adt",
                        descriptor: "(Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/Object;",
                    });
                    for (index, payload) in payloads.iter().enumerate().rev() {
                        work.push(Work::ArrayStore);
                        work.push(Work::Expr(payload));
                        work.push(Work::PushInt(index as i32));
                        work.push(Work::Dup);
                    }
                    work.push(Work::NewObjectArray(payloads.len()));
                    work.push(Work::AdtName(name));
                }
                kind => unreachable!(
                    "unsupported module value initializer reached JVM emission: {kind:?}"
                ),
            },
            Work::PushInt(value) => code.push_i32(value),
            Work::String(value) => code.ldc_string(value),
            Work::AdtName(name) => code.ldc_string(&name.join("::")),
            Work::NewObjectArray(len) => {
                code.push_i32(len as i32);
                code.anewarray(JAVA_LANG_OBJECT);
            }
            Work::Dup => code.op(0x59),
            Work::ArrayStore => code.op(0x53),
            Work::InvokeStatic { method, descriptor }
            | Work::InvokeRuntime { method, descriptor } => {
                code.invokestatic(&runtime_class, method, descriptor);
            }
        }
        if code.code.len() > usize::from(u16::MAX) {
            return Err(JvmBackendError::new("JVM method code exceeds 65535 bytes"));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn named_module_value_max_stack(program: &TypedProgram, name: &str) -> usize {
    let declaration = program
        .module_values
        .iter()
        .find(|declaration| declaration.identity.name == name)
        .expect("module value for maximum JVM stack");
    module_value_initializer_max_stack(&declaration.initializer)
}

#[cfg(test)]
pub(crate) fn local_binding_retention(program: &TypedProgram, function_name: &str) -> usize {
    let options = SanitizedOptions {
        program_class: "VelnProgram".to_string(),
        runtime_class: "VelnRuntime".to_string(),
    };
    let emitter = ClassfileEmitter::new(program, options);
    let function = program
        .functions
        .iter()
        .find(|function| function.name == function_name)
        .expect("function for local binding retention");
    let class = ClassBuilder::new(&emitter.options.program_class);
    let mut code = MethodCode::new(Rc::clone(&class.constant_pool));
    let mut function_emitter = FunctionBytecodeEmitter::new(&emitter, function);
    function_emitter.emit(&mut code);
    function_emitter.locals.peak_retained_entry_count()
}

#[cfg(test)]
pub(crate) fn function_code_footprint(program: &TypedProgram, function_name: &str) -> usize {
    let options = SanitizedOptions {
        program_class: "VelnProgram".to_string(),
        runtime_class: "VelnRuntime".to_string(),
    };
    let emitter = ClassfileEmitter::new(program, options);
    let function = program
        .functions
        .iter()
        .find(|function| function.name == function_name)
        .expect("function for bytecode footprint");
    let class = ClassBuilder::new(&emitter.options.program_class);
    let mut code = MethodCode::new(Rc::clone(&class.constant_pool));
    let mut function_emitter = FunctionBytecodeEmitter::new(&emitter, function);
    function_emitter.emit(&mut code);
    code.code.len() + code.exceptions.len() * 8
}

#[cfg(test)]
pub(crate) fn function_max_stack(program: &TypedProgram, function_name: &str) -> usize {
    let options = SanitizedOptions {
        program_class: "VelnProgram".to_string(),
        runtime_class: "VelnRuntime".to_string(),
    };
    let emitter = ClassfileEmitter::new(program, options);
    let function = program
        .functions
        .iter()
        .find(|function| function.name == function_name)
        .expect("function for maximum JVM stack");
    let class = ClassBuilder::new(&emitter.options.program_class);
    let mut code = MethodCode::new(Rc::clone(&class.constant_pool));
    let mut function_emitter = FunctionBytecodeEmitter::new(&emitter, function);
    function_emitter.emit(&mut code);
    code.max_stack
}

#[cfg(test)]
pub(crate) fn contract_call_metadata_work(program: &TypedProgram, function_name: &str) -> usize {
    let options = SanitizedOptions {
        program_class: "VelnProgram".to_string(),
        runtime_class: "VelnRuntime".to_string(),
    };
    let emitter = ClassfileEmitter::new(program, options);
    let function = program
        .functions
        .iter()
        .find(|function| function.name == function_name)
        .expect("function for contract call metadata work");
    let class = ClassBuilder::new(&emitter.options.program_class);
    let mut code = MethodCode::new(Rc::clone(&class.constant_pool));
    let mut function_emitter = FunctionBytecodeEmitter::new(&emitter, function);
    function_emitter.emit(&mut code);
    code.contract_call_metadata_work
}

#[cfg(test)]
pub(crate) fn contract_remainder_emission_work(
    program: &TypedProgram,
    function_name: &str,
) -> usize {
    let options = SanitizedOptions {
        program_class: "VelnProgram".to_string(),
        runtime_class: "VelnRuntime".to_string(),
    };
    let emitter = ClassfileEmitter::new(program, options);
    let function = program
        .functions
        .iter()
        .find(|function| function.name == function_name)
        .expect("function for contract remainder emission work");
    let class = ClassBuilder::new(&emitter.options.program_class);
    let mut code = MethodCode::new(Rc::clone(&class.constant_pool));
    let mut function_emitter = FunctionBytecodeEmitter::new(&emitter, function);
    function_emitter.emit(&mut code);
    code.contract_remainder_emission_work
}

#[cfg(test)]
pub(crate) fn contract_remainder_scan_work(program: &TypedProgram, function_name: &str) -> usize {
    let options = SanitizedOptions {
        program_class: "VelnProgram".to_string(),
        runtime_class: "VelnRuntime".to_string(),
    };
    let emitter = ClassfileEmitter::new(program, options);
    let function = program
        .functions
        .iter()
        .find(|function| function.name == function_name)
        .expect("function for contract remainder scan work");
    let class = ClassBuilder::new(&emitter.options.program_class);
    let mut code = MethodCode::new(Rc::clone(&class.constant_pool));
    let mut function_emitter = FunctionBytecodeEmitter::new(&emitter, function);
    function_emitter.emit(&mut code);
    code.contract_remainder_scan_work
}

pub(crate) struct ClassfileEmitter<'a> {
    program: &'a TypedProgram,
    options: SanitizedOptions,
    function_names: BTreeMap<String, String>,
    module_value_names: BTreeMap<CoreModuleValueIdentity, String>,
    module_value_owners: BTreeMap<CoreModuleValueIdentity, String>,
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
        let mut module_value_names = BTreeMap::new();
        for declaration in &program.module_values {
            let identity = &declaration.identity;
            let name = unique_java_identifier(
                &format!(
                    "value_{}_{}_{}",
                    sanitize_identifier_text(identity.package.as_deref().unwrap_or("current")),
                    sanitize_identifier_text(identity.module.as_deref().unwrap_or("module")),
                    sanitize_identifier_text(&identity.name),
                ),
                &mut used_names,
            );
            module_value_names.insert(identity.clone(), name);
        }
        let mut emitter = Self {
            program,
            options,
            function_names,
            module_value_names,
            module_value_owners: BTreeMap::new(),
        };
        for (position, declaration_index) in emitter
            .module_values_in_dependency_order()
            .into_iter()
            .enumerate()
        {
            let identity = emitter.program.module_values[declaration_index]
                .identity
                .clone();
            let owner = emitter.module_value_class(position / MODULE_VALUES_PER_CLASS);
            emitter.module_value_owners.insert(identity, owner);
        }
        emitter
    }

    fn function_callsite(&self, name: &str) -> bool {
        self.program
            .functions
            .iter()
            .find(|function| function.name == name)
            .is_some_and(|function| function.callsite)
    }

    fn module_value_name(&self, identity: &CoreModuleValueIdentity) -> &str {
        self.module_value_names
            .get(identity)
            .unwrap_or_else(|| panic!("missing module value declaration for {identity:?}"))
    }

    fn module_value_owner(&self, identity: &CoreModuleValueIdentity) -> &str {
        self.module_value_owners
            .get(identity)
            .unwrap_or_else(|| panic!("missing module value declaration for {identity:?}"))
    }

    fn module_value_class(&self, class_index: usize) -> String {
        format!("{}$ModuleValues{class_index}", self.options.program_class)
    }

    fn module_values_in_dependency_order(&self) -> Vec<usize> {
        let indexes = self
            .program
            .module_values
            .iter()
            .enumerate()
            .map(|(index, declaration)| (declaration.identity.clone(), index))
            .collect::<BTreeMap<_, _>>();
        let mut dependencies = Vec::with_capacity(self.program.module_values.len());
        let mut dependents = vec![Vec::new(); self.program.module_values.len()];

        for (index, declaration) in self.program.module_values.iter().enumerate() {
            let mut identities = BTreeSet::new();
            collect_module_value_dependencies(&declaration.initializer, &mut identities);
            let declaration_dependencies = identities
                .into_iter()
                .map(|identity| {
                    indexes.get(&identity).copied().unwrap_or_else(|| {
                        panic!("missing module value declaration for {identity:?}")
                    })
                })
                .collect::<BTreeSet<_>>();
            for dependency in &declaration_dependencies {
                dependents[*dependency].push(index);
            }
            dependencies.push(declaration_dependencies);
        }

        let mut remaining_dependencies = dependencies.iter().map(BTreeSet::len).collect::<Vec<_>>();
        let mut ready = remaining_dependencies
            .iter()
            .enumerate()
            .filter_map(|(index, count)| (*count == 0).then_some(index))
            .collect::<BTreeSet<_>>();
        let mut ordered = Vec::with_capacity(self.program.module_values.len());
        while let Some(index) = ready.iter().next().copied() {
            ready.remove(&index);
            ordered.push(index);
            for dependent in &dependents[index] {
                remaining_dependencies[*dependent] -= 1;
                if remaining_dependencies[*dependent] == 0 {
                    ready.insert(*dependent);
                }
            }
        }
        assert_eq!(
            ordered.len(),
            self.program.module_values.len(),
            "module value initializers must be acyclic before JVM emission"
        );
        ordered
    }

    pub(crate) fn emit(
        &self,
        entry_function: &str,
        entry_arg_types: &[EntryArgType],
    ) -> Result<JvmProgram, JvmBackendError> {
        self.emit_with_entry_class(self.emit_entry_class(entry_function, entry_arg_types)?)
    }

    pub(crate) fn emit_test_entries(
        &self,
        entry_functions: &[String],
    ) -> Result<JvmProgram, JvmBackendError> {
        self.emit_with_entry_class(self.emit_test_entry_class(entry_functions)?)
    }

    fn emit_with_entry_class(&self, entry_class: Vec<u8>) -> Result<JvmProgram, JvmBackendError> {
        let mut classes = runtime_classes();
        classes.push(JvmClassFile {
            path: format!("{}.class", self.options.program_class),
            contents: self.emit_program_class()?,
        });
        classes.push(JvmClassFile {
            path: "VelnEntry.class".to_string(),
            contents: entry_class,
        });
        let ordered_values = self.module_values_in_dependency_order();
        for (class_index, declarations) in
            ordered_values.chunks(MODULE_VALUES_PER_CLASS).enumerate()
        {
            let class_name = self.module_value_class(class_index);
            classes.push(JvmClassFile {
                path: format!("{class_name}.class"),
                contents: self.emit_module_value_class(&class_name, declarations)?,
            });
        }
        for function in &self.program.functions {
            classes.push(JvmClassFile {
                path: format!(
                    "{}${}.class",
                    self.options.program_class,
                    self.function_name(&function.name)
                ),
                contents: self.emit_function_adapter(function)?,
            });
        }
        Ok(JvmProgram { classes })
    }

    fn emit_program_class(&self) -> Result<Vec<u8>, JvmBackendError> {
        let mut class = ClassBuilder::new(&self.options.program_class);
        class.access_flags = 0x0031;
        class.add_default_constructor(0x0002);
        let ordered_values = self.module_values_in_dependency_order();
        if !ordered_values.is_empty() {
            let mut method = MethodCode::new(Rc::clone(&class.constant_pool));
            for class_index in 0..ordered_values.len().div_ceil(MODULE_VALUES_PER_CLASS) {
                method.invokestatic(&self.module_value_class(class_index), "initialize", "()V");
            }
            method.op(0xb1);
            class.add_method(MethodInfo {
                access_flags: 0x0008,
                name: "<clinit>".to_string(),
                descriptor: "()V".to_string(),
                max_stack: method.max_stack,
                max_locals: 0,
                code: method.code,
                exceptions: method.exceptions,
            });
        }
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

    fn emit_module_value_class(
        &self,
        class_name: &str,
        declarations: &[usize],
    ) -> Result<Vec<u8>, JvmBackendError> {
        let mut class = ClassBuilder::new(class_name);
        class.access_flags = 0x0031;
        let function = self
            .program
            .functions
            .first()
            .expect("JVM program requires an entry or test function");

        for declaration_index in declarations {
            let declaration = &self.program.module_values[*declaration_index];
            class.add_field(FieldInfo {
                access_flags: 0x0008,
                name: self.module_value_name(&declaration.identity).to_string(),
                descriptor: MODULE_VALUE_DESCRIPTOR.to_string(),
            });
        }
        for declaration_index in declarations {
            let declaration = &self.program.module_values[*declaration_index];
            let mut method = MethodCode::with_max_stack(
                Rc::clone(&class.constant_pool),
                module_value_initializer_max_stack(&declaration.initializer),
            );
            let mut emitter = FunctionBytecodeEmitter::new(self, function);
            emit_module_value_initializer(&mut emitter, &mut method, &declaration.initializer)?;
            method.putstatic(
                class_name,
                self.module_value_name(&declaration.identity),
                MODULE_VALUE_DESCRIPTOR,
            );
            method.op(0xb1);
            class.add_method(MethodInfo {
                access_flags: 0x000a,
                name: format!("init_{}", self.module_value_name(&declaration.identity)),
                descriptor: "()V".to_string(),
                max_stack: method.max_stack,
                max_locals: 0,
                code: method.code,
                exceptions: method.exceptions,
            });
        }

        let mut method = MethodCode::new(Rc::clone(&class.constant_pool));
        for declaration_index in declarations {
            let declaration = &self.program.module_values[*declaration_index];
            method.invokestatic(
                class_name,
                &format!("init_{}", self.module_value_name(&declaration.identity)),
                "()V",
            );
        }
        method.op(0xb1);
        class.add_method(MethodInfo {
            access_flags: 0x0008,
            name: "initialize".to_string(),
            descriptor: "()V".to_string(),
            max_stack: method.max_stack,
            max_locals: 0,
            code: method.code,
            exceptions: method.exceptions,
        });
        class.finish()
    }

    fn emit_function_adapter(&self, function: &IrFunction) -> Result<Vec<u8>, JvmBackendError> {
        let adapter = format!(
            "{}${}",
            self.options.program_class,
            self.function_name(&function.name)
        );
        let mut class = ClassBuilder::new(&adapter);
        class.access_flags = 0x0031;
        class
            .interfaces
            .push(self.runtime_nested(if function.callsite {
                "CallsiteFn"
            } else {
                "Fn"
            }));
        class.add_default_constructor(0x0001);

        let mut code = MethodCode::new(Rc::clone(&class.constant_pool));
        let source_param_count = function.params.len() - usize::from(function.callsite);
        for index in 0..source_param_count {
            code.aload(1);
            code.push_i32(index as i32);
            code.op(0x32);
        }
        if function.callsite {
            code.aload(2);
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
            descriptor: if function.callsite {
                "([Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;".to_string()
            } else {
                "([Ljava/lang/Object;)Ljava/lang/Object;".to_string()
            },
            max_stack: code.max_stack,
            max_locals: if function.callsite { 3 } else { 2 },
            code: code.code,
            exceptions: Vec::new(),
        });
        class.finish()
    }

    fn emit_entry_class(
        &self,
        entry_function: &str,
        entry_arg_types: &[EntryArgType],
    ) -> Result<Vec<u8>, JvmBackendError> {
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

        self.finish_entry_class(class, code, try_start, try_end)
    }

    fn emit_test_entry_class(
        &self,
        entry_functions: &[String],
    ) -> Result<Vec<u8>, JvmBackendError> {
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
        self.emit_test_entry_dispatch(&mut code, entry_functions, dispatched);

        code.bind(dispatched);
        code.iinc(3, 1);
        code.branch_to(0xa7, loop_start);
        code.bind(loop_done);
        let try_end = code.mark();
        code.op(0xb1);

        self.finish_entry_class(class, code, try_start, try_end)
    }

    fn emit_test_entry_dispatch(
        &self,
        code: &mut MethodCode,
        entry_functions: &[String],
        dispatched: usize,
    ) {
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
            self.emit_entry_result(code);
            code.branch_to(0xa7, dispatched);
            code.bind(next);
        }
        code.getstatic("java/lang/System", "err", "Ljava/io/PrintStream;");
        code.ldc_string("unknown test entry");
        code.invokevirtual("java/io/PrintStream", "println", "(Ljava/lang/String;)V");
        code.push_i32(1);
        code.invokestatic("java/lang/System", "exit", "(I)V");
        code.op(0xb1);
    }

    fn finish_entry_class(
        &self,
        mut class: ClassBuilder,
        mut code: MethodCode,
        try_start: usize,
        try_end: usize,
    ) -> Result<Vec<u8>, JvmBackendError> {
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
            "formatResultFailure",
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
        self.emit_entry_failure_handler(code, try_start, try_end, "ContractFailure", true);
    }

    fn emit_entry_runtime_failure_handler(
        &self,
        code: &mut MethodCode,
        try_start: usize,
        try_end: usize,
    ) {
        self.emit_entry_failure_handler(code, try_start, try_end, "RuntimeFailure", false);
    }

    fn emit_entry_failure_handler(
        &self,
        code: &mut MethodCode,
        try_start: usize,
        try_end: usize,
        failure_class: &str,
        record_contract_failure: bool,
    ) {
        let handler = code.mark();
        code.astore(2);
        if record_contract_failure {
            code.aload(2);
            code.invokestatic(
                &self.options.runtime_class,
                "recordContractFailure",
                &format!("(L{}$ContractFailure;)V", self.options.runtime_class),
            );
        }
        code.aload(2);
        code.invokestatic(
            &self.options.runtime_class,
            "printFailure",
            "(Ljava/lang/Throwable;)V",
        );
        code.push_i32(1);
        code.invokestatic("java/lang/System", "exit", "(I)V");
        code.op(0xb1);
        code.exceptions.push(ExceptionHandler {
            start_pc: try_start,
            end_pc: try_end,
            handler_pc: handler,
            catch_type: self.runtime_nested(failure_class),
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
