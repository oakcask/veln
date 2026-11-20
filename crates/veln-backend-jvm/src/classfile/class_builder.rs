use super::*;

pub(super) struct ClassBuilder {
    pub(super) name: String,
    pub(super) access_flags: u16,
    pub(super) constant_pool: Rc<RefCell<ConstantPool>>,
    pub(super) fields: Vec<FieldInfo>,
    pub(super) methods: Vec<MethodInfo>,
    pub(super) interfaces: Vec<String>,
}

impl ClassBuilder {
    pub(super) fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            access_flags: 0x0021,
            constant_pool: Rc::new(RefCell::new(ConstantPool::new())),
            fields: Vec::new(),
            methods: Vec::new(),
            interfaces: Vec::new(),
        }
    }

    pub(super) fn add_default_constructor(&mut self, access_flags: u16) {
        let mut code = MethodCode::new(Rc::clone(&self.constant_pool));
        code.aload(0);
        code.invokespecial(JAVA_LANG_OBJECT, "<init>", "()V");
        code.op(0xb1);
        self.add_method(MethodInfo {
            access_flags,
            name: "<init>".to_string(),
            descriptor: "()V".to_string(),
            max_stack: 1,
            max_locals: 1,
            code: code.code,
            exceptions: Vec::new(),
        });
    }

    pub(super) fn add_method(&mut self, method: MethodInfo) {
        self.methods.push(method);
    }

    pub(super) fn add_field(&mut self, field: FieldInfo) {
        self.fields.push(field);
    }

    pub(super) fn finish(self) -> Result<Vec<u8>, JvmBackendError> {
        validate_method_limits(&self.methods)?;
        if let Some(error) = self.constant_pool.borrow().error() {
            return Err(error);
        }
        let class = serialize_class(&self)?;
        class.write(&self.constant_pool)
    }
}

fn validate_method_limits(methods: &[MethodInfo]) -> Result<(), JvmBackendError> {
    if methods
        .iter()
        .any(|method| method.code.len() > usize::from(u16::MAX))
    {
        return Err(JvmBackendError::new("JVM method code exceeds 65535 bytes"));
    }
    if methods
        .iter()
        .any(|method| method.max_stack > usize::from(u16::MAX))
    {
        return Err(JvmBackendError::new(
            "JVM method operand stack exceeds 65535 slots",
        ));
    }
    Ok(())
}

fn serialize_class(class: &ClassBuilder) -> Result<SerializedClass, JvmBackendError> {
    let this_class = class.constant_pool.borrow_mut().class(&class.name);
    let super_class = class.constant_pool.borrow_mut().class(JAVA_LANG_OBJECT);
    let code_name = class.constant_pool.borrow_mut().utf8("Code");
    let stack_map_name = class.constant_pool.borrow_mut().utf8("StackMapTable");
    let fields = serialize_fields(&class.constant_pool, &class.fields);
    let methods = serialize_methods(
        &class.constant_pool,
        &class.name,
        &class.methods,
        code_name,
        stack_map_name,
    )?;
    let interfaces = class
        .interfaces
        .iter()
        .map(|interface| class.constant_pool.borrow_mut().class(interface))
        .collect();
    Ok(SerializedClass {
        access_flags: class.access_flags,
        this_class,
        super_class,
        fields,
        methods,
        interfaces,
    })
}

fn serialize_fields(
    constant_pool: &Rc<RefCell<ConstantPool>>,
    fields: &[FieldInfo],
) -> Vec<SerializedField> {
    fields
        .iter()
        .map(|field| serialize_field(constant_pool, field))
        .collect()
}

fn serialize_field(
    constant_pool: &Rc<RefCell<ConstantPool>>,
    field: &FieldInfo,
) -> SerializedField {
    let name = constant_pool.borrow_mut().utf8(&field.name);
    let descriptor = constant_pool.borrow_mut().utf8(&field.descriptor);
    SerializedField {
        access_flags: field.access_flags,
        name,
        descriptor,
    }
}

fn serialize_methods(
    constant_pool: &Rc<RefCell<ConstantPool>>,
    class_name: &str,
    methods: &[MethodInfo],
    code_name: u16,
    stack_map_name: u16,
) -> Result<Vec<SerializedMethod>, JvmBackendError> {
    methods
        .iter()
        .map(|method| {
            serialize_method(constant_pool, class_name, method, code_name, stack_map_name)
        })
        .collect()
}

fn serialize_method(
    constant_pool: &Rc<RefCell<ConstantPool>>,
    class_name: &str,
    method: &MethodInfo,
    code_name: u16,
    stack_map_name: u16,
) -> Result<SerializedMethod, JvmBackendError> {
    let name = constant_pool.borrow_mut().utf8(&method.name);
    let descriptor = constant_pool.borrow_mut().utf8(&method.descriptor);
    let exceptions = serialize_exception_handlers(constant_pool, &method.exceptions);
    let stack_map = stack_map::stack_map_table(constant_pool, class_name, method)?;
    Ok(SerializedMethod {
        access_flags: method.access_flags,
        name,
        descriptor,
        code_name,
        max_stack: method.max_stack as u16,
        max_locals: method.max_locals,
        code: method.code.clone(),
        exceptions,
        stack_map_name,
        stack_map,
    })
}

fn serialize_exception_handlers(
    constant_pool: &Rc<RefCell<ConstantPool>>,
    handlers: &[ExceptionHandler],
) -> Vec<SerializedExceptionHandler> {
    handlers
        .iter()
        .map(|handler| SerializedExceptionHandler {
            start_pc: handler.start_pc,
            end_pc: handler.end_pc,
            handler_pc: handler.handler_pc,
            catch_type: constant_pool.borrow_mut().class(&handler.catch_type),
        })
        .collect()
}

struct SerializedClass {
    access_flags: u16,
    this_class: u16,
    super_class: u16,
    fields: Vec<SerializedField>,
    methods: Vec<SerializedMethod>,
    interfaces: Vec<u16>,
}

impl SerializedClass {
    fn write(self, constant_pool: &Rc<RefCell<ConstantPool>>) -> Result<Vec<u8>, JvmBackendError> {
        let mut out = Vec::new();
        write_u32(&mut out, 0xcafebabe);
        write_u16(&mut out, 0);
        write_u16(&mut out, PROGRAM_MAJOR_VERSION);
        constant_pool.borrow().write(&mut out)?;
        write_u16(&mut out, self.access_flags);
        write_u16(&mut out, self.this_class);
        write_u16(&mut out, self.super_class);
        write_u16(&mut out, self.interfaces.len() as u16);
        for interface in self.interfaces {
            write_u16(&mut out, interface);
        }
        write_u16(&mut out, self.fields.len() as u16);
        for field in self.fields {
            field.write(&mut out);
        }
        write_u16(&mut out, self.methods.len() as u16);
        for method in self.methods {
            method.write(&mut out);
        }
        write_u16(&mut out, 0);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finish_class_with_dead_goto_chain(block_count: usize) -> Result<Vec<u8>, JvmBackendError> {
        let mut code = Vec::with_capacity(2 + block_count * 3);
        code.push(0xb1);
        for _ in 0..block_count {
            code.extend_from_slice(&[0xa7, 0, 3]);
        }
        code.push(0xb1);

        let mut class = ClassBuilder::new("DeadGotoChain");
        class.add_method(MethodInfo {
            access_flags: 0x0008,
            name: "dead_gotos".to_string(),
            descriptor: "()V".to_string(),
            max_stack: 0,
            max_locals: 0,
            code,
            exceptions: Vec::new(),
        });
        class.finish()
    }

    fn finish_dead_goto_chain_requiring_a_local(
        block_count: usize,
    ) -> Result<Vec<u8>, JvmBackendError> {
        let mut code = Vec::with_capacity(4 + block_count * 3);
        code.push(0xb1);
        for _ in 0..block_count {
            code.extend_from_slice(&[0xa7, 0, 3]);
        }
        code.extend_from_slice(&[0x1a, 0x57, 0xb1]);

        let mut class = ClassBuilder::new("DeadGotoLocalChain");
        class.add_method(MethodInfo {
            access_flags: 0x0008,
            name: "dead_gotos_reading_local".to_string(),
            descriptor: "()V".to_string(),
            max_stack: 1,
            max_locals: 1,
            code,
            exceptions: Vec::new(),
        });
        class.finish()
    }

    fn finish_class_with_many_locals_and_straight_line_expressions(
        expression_count: usize,
    ) -> Result<Vec<u8>, JvmBackendError> {
        const LOCAL_COUNT: u8 = 250;
        let mut code = Vec::with_capacity(usize::from(LOCAL_COUNT) * 3 + expression_count * 4 + 1);
        for slot in 0..LOCAL_COUNT {
            code.extend_from_slice(&[0x03, 0x36, slot]);
        }
        for _ in 0..expression_count {
            code.extend_from_slice(&[0x03, 0x57, 0x00, 0x00]);
        }
        code.push(0xb1);

        let mut class = ClassBuilder::new("StraightLineFrames");
        class.add_method(MethodInfo {
            access_flags: 0x0008,
            name: "straight_line".to_string(),
            descriptor: "()V".to_string(),
            max_stack: 1,
            max_locals: u16::from(LOCAL_COUNT),
            code,
            exceptions: Vec::new(),
        });
        class.finish()
    }

    #[test]
    fn java_25_verifies_widened_frames_through_converging_dead_blocks() {
        let mut class = ClassBuilder::new("VelnEntry");
        class.add_method(MethodInfo {
            access_flags: 0x0009,
            name: "main".to_string(),
            descriptor: "([Ljava/lang/String;)V".to_string(),
            max_stack: 1,
            max_locals: 1,
            code: vec![
                0xb1, // return
                0x03, // iconst_0
                0x99, 0x00, 0x07, // ifeq 9
                0x01, // aconst_null
                0xa7, 0x00, 0x0a, // goto 16
                0xa7, 0x00, 0x03, // goto 12
                0x2a, // aload_0
                0xa7, 0x00, 0x03, // goto 16
                0xa7, 0x00, 0x03, // goto 19
                0x57, // pop
                0xb1, // return
            ],
            exceptions: Vec::new(),
        });
        let program = JvmProgram {
            classes: vec![JvmClassFile {
                path: "VelnEntry.class".to_string(),
                contents: class.finish().expect("class serialization should succeed"),
            }],
        };

        let output = crate::tests::run_jvm_program_with_java_args_when_java_is_available(
            "converging-dead-stack-map-frames",
            &program,
            &["-Xverify:all"],
            &[],
        )
        .expect("the Java 25 test runtime should be available");

        assert!(
            output.status.success(),
            "JVM verification failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn finish_bounds_frames_for_many_locals_and_straight_line_instructions() {
        finish_class_with_many_locals_and_straight_line_expressions(15_000)
            .expect("straight-line analysis should not retain one full frame per instruction");
    }

    #[test]
    fn finish_bounds_frames_for_adjacent_larger_straight_line_input() {
        finish_class_with_many_locals_and_straight_line_expressions(15_100)
            .expect("adjacent straight-line analysis should remain bounded");
    }

    #[test]
    fn finish_preserves_branch_merges_and_exception_handler_frames() {
        let mut class = ClassBuilder::new("SparseFrameControlFlow");
        class.add_method(MethodInfo {
            access_flags: 0x0008,
            name: "branch_merge".to_string(),
            descriptor: "()V".to_string(),
            max_stack: 1,
            max_locals: 0,
            code: vec![0x03, 0x99, 0x00, 0x06, 0x03, 0x57, 0x00, 0xb1],
            exceptions: Vec::new(),
        });
        class.add_method(MethodInfo {
            access_flags: 0x0008,
            name: "exception_handler".to_string(),
            descriptor: "()V".to_string(),
            max_stack: 1,
            max_locals: 0,
            code: vec![0x00, 0xb1, 0x57, 0xb1],
            exceptions: vec![ExceptionHandler {
                start_pc: 0,
                end_pc: 1,
                handler_pc: 2,
                catch_type: "java/lang/Throwable".to_string(),
            }],
        });

        class
            .finish()
            .expect("sparse analysis should retain branch and handler entry frames");
    }

    #[test]
    fn finish_handles_adjacent_smaller_dead_goto_chain() {
        finish_class_with_dead_goto_chain(20_000)
            .expect("dead control-flow traversal should complete without recursive stack growth");
    }

    #[test]
    fn finish_handles_near_limit_dead_goto_chain() {
        finish_class_with_dead_goto_chain(21_000)
            .expect("dead control-flow traversal should complete without recursive stack growth");
    }

    #[test]
    fn finish_bounds_local_completion_for_near_limit_dead_goto_chain() {
        finish_dead_goto_chain_requiring_a_local(21_000)
            .expect("dead local completion should scale with the control-flow chain");
    }

    #[test]
    fn finish_bounds_local_completion_for_adjacent_dead_goto_chain() {
        finish_dead_goto_chain_requiring_a_local(20_000)
            .expect("adjacent dead local completion should remain bounded");
    }

    #[test]
    fn finish_rejects_method_code_above_the_classfile_limit() {
        let mut class = ClassBuilder::new("OversizedMethod");
        class.add_method(MethodInfo {
            access_flags: 0x0008,
            name: "oversized".to_string(),
            descriptor: "()V".to_string(),
            max_stack: 0,
            max_locals: 0,
            code: vec![0; usize::from(u16::MAX) + 1],
            exceptions: Vec::new(),
        });

        let error = class.finish().expect_err("oversized code should fail");

        assert_eq!(error.to_string(), "JVM method code exceeds 65535 bytes");
    }

    #[test]
    fn finish_rejects_operand_stack_above_the_classfile_limit() {
        let mut class = ClassBuilder::new("OversizedOperandStack");
        class.add_method(MethodInfo {
            access_flags: 0x0008,
            name: "oversized".to_string(),
            descriptor: "()V".to_string(),
            max_stack: usize::from(u16::MAX) + 1,
            max_locals: 0,
            code: vec![0xb1],
            exceptions: Vec::new(),
        });

        let error = class
            .finish()
            .expect_err("unrepresentable operand stack should fail");

        assert_eq!(
            error.to_string(),
            "JVM method operand stack exceeds 65535 slots"
        );
    }
}

pub(super) struct FieldInfo {
    pub(super) access_flags: u16,
    pub(super) name: String,
    pub(super) descriptor: String,
}

struct SerializedField {
    access_flags: u16,
    name: u16,
    descriptor: u16,
}

impl SerializedField {
    fn write(&self, out: &mut Vec<u8>) {
        write_u16(out, self.access_flags);
        write_u16(out, self.name);
        write_u16(out, self.descriptor);
        write_u16(out, 0);
    }
}

pub(super) struct MethodInfo {
    pub(super) access_flags: u16,
    pub(super) name: String,
    pub(super) descriptor: String,
    pub(super) max_stack: usize,
    pub(super) max_locals: u16,
    pub(super) code: Vec<u8>,
    pub(super) exceptions: Vec<ExceptionHandler>,
}

struct SerializedMethod {
    access_flags: u16,
    name: u16,
    descriptor: u16,
    code_name: u16,
    max_stack: u16,
    max_locals: u16,
    code: Vec<u8>,
    exceptions: Vec<SerializedExceptionHandler>,
    stack_map_name: u16,
    stack_map: Vec<u8>,
}

impl SerializedMethod {
    pub(super) fn write(&self, out: &mut Vec<u8>) {
        write_u16(out, self.access_flags);
        write_u16(out, self.name);
        write_u16(out, self.descriptor);
        write_u16(out, 1);
        write_u16(out, self.code_name);
        let nested_attribute_length = if self.stack_map.len() > 2 {
            6 + self.stack_map.len()
        } else {
            0
        };
        let attribute_length =
            12 + self.code.len() + self.exceptions.len() * 8 + nested_attribute_length;
        write_u32(out, attribute_length as u32);
        write_u16(out, self.max_stack);
        write_u16(out, self.max_locals);
        write_u32(out, self.code.len() as u32);
        out.extend_from_slice(&self.code);
        write_u16(out, self.exceptions.len() as u16);
        for exception in &self.exceptions {
            write_u16(out, exception.start_pc as u16);
            write_u16(out, exception.end_pc as u16);
            write_u16(out, exception.handler_pc as u16);
            write_u16(out, exception.catch_type);
        }
        if self.stack_map.len() > 2 {
            write_u16(out, 1);
            write_u16(out, self.stack_map_name);
            write_u32(out, self.stack_map.len() as u32);
            out.extend_from_slice(&self.stack_map);
        } else {
            write_u16(out, 0);
        }
    }
}

#[derive(Clone)]
pub(super) struct ExceptionHandler {
    pub(super) start_pc: usize,
    pub(super) end_pc: usize,
    pub(super) handler_pc: usize,
    pub(super) catch_type: String,
}

struct SerializedExceptionHandler {
    start_pc: usize,
    end_pc: usize,
    handler_pc: usize,
    catch_type: u16,
}
