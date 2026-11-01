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
        let class = serialize_class(&self);
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

fn serialize_class(class: &ClassBuilder) -> SerializedClass {
    let this_class = class.constant_pool.borrow_mut().class(&class.name);
    let super_class = class.constant_pool.borrow_mut().class(JAVA_LANG_OBJECT);
    let code_name = class.constant_pool.borrow_mut().utf8("Code");
    let fields = serialize_fields(&class.constant_pool, &class.fields);
    let methods = serialize_methods(&class.constant_pool, &class.methods, code_name);
    let interfaces = class
        .interfaces
        .iter()
        .map(|interface| class.constant_pool.borrow_mut().class(interface))
        .collect();
    SerializedClass {
        access_flags: class.access_flags,
        this_class,
        super_class,
        fields,
        methods,
        interfaces,
    }
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
    methods: &[MethodInfo],
    code_name: u16,
) -> Vec<SerializedMethod> {
    methods
        .iter()
        .map(|method| serialize_method(constant_pool, method, code_name))
        .collect()
}

fn serialize_method(
    constant_pool: &Rc<RefCell<ConstantPool>>,
    method: &MethodInfo,
    code_name: u16,
) -> SerializedMethod {
    let name = constant_pool.borrow_mut().utf8(&method.name);
    let descriptor = constant_pool.borrow_mut().utf8(&method.descriptor);
    let exceptions = serialize_exception_handlers(constant_pool, &method.exceptions);
    SerializedMethod {
        access_flags: method.access_flags,
        name,
        descriptor,
        code_name,
        max_stack: method.max_stack as u16,
        max_locals: method.max_locals,
        code: method.code.clone(),
        exceptions,
    }
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
}

impl SerializedMethod {
    pub(super) fn write(&self, out: &mut Vec<u8>) {
        write_u16(out, self.access_flags);
        write_u16(out, self.name);
        write_u16(out, self.descriptor);
        write_u16(out, 1);
        write_u16(out, self.code_name);
        let attribute_length = 12 + self.code.len() + self.exceptions.len() * 8;
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
        write_u16(out, 0);
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
