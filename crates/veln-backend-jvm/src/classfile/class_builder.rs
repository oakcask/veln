use super::*;

pub(super) struct ClassBuilder {
    pub(super) name: String,
    pub(super) access_flags: u16,
    pub(super) constant_pool: Rc<RefCell<ConstantPool>>,
    pub(super) methods: Vec<MethodInfo>,
    pub(super) interfaces: Vec<String>,
}

impl ClassBuilder {
    pub(super) fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            access_flags: 0x0021,
            constant_pool: Rc::new(RefCell::new(ConstantPool::new())),
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

    pub(super) fn finish(self) -> Vec<u8> {
        let this_class = self.constant_pool.borrow_mut().class(&self.name);
        let super_class = self.constant_pool.borrow_mut().class(JAVA_LANG_OBJECT);
        let code_name = self.constant_pool.borrow_mut().utf8("Code");
        let mut methods = Vec::new();
        for method in &self.methods {
            let name = self.constant_pool.borrow_mut().utf8(&method.name);
            let descriptor = self.constant_pool.borrow_mut().utf8(&method.descriptor);
            let mut exceptions = Vec::new();
            for handler in &method.exceptions {
                exceptions.push(SerializedExceptionHandler {
                    start_pc: handler.start_pc,
                    end_pc: handler.end_pc,
                    handler_pc: handler.handler_pc,
                    catch_type: self.constant_pool.borrow_mut().class(&handler.catch_type),
                });
            }
            methods.push(SerializedMethod {
                access_flags: method.access_flags,
                name,
                descriptor,
                code_name,
                max_stack: method.max_stack,
                max_locals: method.max_locals,
                code: method.code.clone(),
                exceptions,
            });
        }
        let mut interfaces = Vec::new();
        for interface in &self.interfaces {
            interfaces.push(self.constant_pool.borrow_mut().class(interface));
        }

        let mut out = Vec::new();
        write_u32(&mut out, 0xcafebabe);
        write_u16(&mut out, 0);
        write_u16(&mut out, PROGRAM_MAJOR_VERSION);
        self.constant_pool.borrow().write(&mut out);
        write_u16(&mut out, self.access_flags);
        write_u16(&mut out, this_class);
        write_u16(&mut out, super_class);
        write_u16(&mut out, interfaces.len() as u16);
        for interface in interfaces {
            write_u16(&mut out, interface);
        }
        write_u16(&mut out, 0);
        write_u16(&mut out, methods.len() as u16);
        for method in methods {
            method.write(&mut out);
        }
        write_u16(&mut out, 0);
        out
    }
}

pub(super) struct MethodInfo {
    pub(super) access_flags: u16,
    pub(super) name: String,
    pub(super) descriptor: String,
    pub(super) max_stack: u16,
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
