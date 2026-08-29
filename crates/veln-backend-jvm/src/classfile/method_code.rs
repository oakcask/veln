use super::*;

pub(super) struct MethodCode {
    constant_pool: Rc<RefCell<ConstantPool>>,
    pub(super) code: Vec<u8>,
    labels: Vec<Option<usize>>,
    patches: Vec<Patch>,
    pub(super) max_stack: u16,
    pub(super) max_locals: u16,
    pub(super) exceptions: Vec<ExceptionHandler>,
}

impl MethodCode {
    pub(super) fn new(constant_pool: Rc<RefCell<ConstantPool>>) -> Self {
        Self {
            constant_pool,
            code: Vec::new(),
            labels: Vec::new(),
            patches: Vec::new(),
            max_stack: 64,
            max_locals: 0,
            exceptions: Vec::new(),
        }
    }

    pub(super) fn op(&mut self, op: u8) {
        self.code.push(op);
    }

    pub(super) fn mark(&self) -> usize {
        self.code.len()
    }

    pub(super) fn new_label(&mut self) -> usize {
        let id = self.labels.len();
        self.labels.push(None);
        id
    }

    pub(super) fn bind(&mut self, label: usize) {
        self.labels[label] = Some(self.code.len());
        self.patch_bound_labels();
    }

    pub(super) fn branch(&mut self, op: u8) -> usize {
        let label = self.new_label();
        self.branch_to(op, label);
        label
    }

    pub(super) fn branch_to(&mut self, op: u8, label: usize) {
        let pos = self.code.len();
        self.code.push(op);
        self.code.extend_from_slice(&[0, 0]);
        self.patches.push(Patch { pos, label });
    }

    pub(super) fn patch_bound_labels(&mut self) {
        for patch in &self.patches {
            if let Some(target) = self.labels[patch.label] {
                let offset = target as isize - patch.pos as isize;
                let bytes = (offset as i16).to_be_bytes();
                self.code[patch.pos + 1] = bytes[0];
                self.code[patch.pos + 2] = bytes[1];
            }
        }
    }

    pub(super) fn aload(&mut self, slot: u16) {
        self.max_locals = self.max_locals.max(slot + 1);
        match slot {
            0..=3 => self.code.push(0x2a + slot as u8),
            _ if slot <= u8::MAX as u16 => {
                self.code.push(0x19);
                self.code.push(slot as u8);
            }
            _ => panic!("too many JVM locals"),
        }
    }

    pub(super) fn astore(&mut self, slot: u16) {
        self.max_locals = self.max_locals.max(slot + 1);
        match slot {
            0..=3 => self.code.push(0x4b + slot as u8),
            _ if slot <= u8::MAX as u16 => {
                self.code.push(0x3a);
                self.code.push(slot as u8);
            }
            _ => panic!("too many JVM locals"),
        }
    }

    pub(super) fn iload(&mut self, slot: u16) {
        self.max_locals = self.max_locals.max(slot + 1);
        match slot {
            0..=3 => self.code.push(0x1a + slot as u8),
            _ if slot <= u8::MAX as u16 => {
                self.code.push(0x15);
                self.code.push(slot as u8);
            }
            _ => panic!("too many JVM locals"),
        }
    }

    pub(super) fn istore(&mut self, slot: u16) {
        self.max_locals = self.max_locals.max(slot + 1);
        match slot {
            0..=3 => self.code.push(0x3b + slot as u8),
            _ if slot <= u8::MAX as u16 => {
                self.code.push(0x36);
                self.code.push(slot as u8);
            }
            _ => panic!("too many JVM locals"),
        }
    }

    pub(super) fn iinc(&mut self, slot: u16, value: i8) {
        self.max_locals = self.max_locals.max(slot + 1);
        if slot <= u8::MAX as u16 {
            self.code.push(0x84);
            self.code.push(slot as u8);
            self.code.push(value as u8);
        } else {
            panic!("too many JVM locals");
        }
    }

    pub(super) fn push_i32(&mut self, value: i32) {
        match value {
            -1 => self.code.push(0x02),
            0..=5 => self.code.push(0x03 + value as u8),
            -128..=127 => {
                self.code.push(0x10);
                self.code.push(value as i8 as u8);
            }
            -32768..=32767 => {
                self.code.push(0x11);
                self.code.extend_from_slice(&(value as i16).to_be_bytes());
            }
            _ => panic!("integer constant out of JVM push range"),
        }
    }

    pub(super) fn ldc_string(&mut self, value: &str) {
        let index = self.constant_pool.borrow_mut().string(value);
        self.ldc_index(index);
    }

    pub(super) fn ldc_long(&mut self, value: i64) {
        let index = self.constant_pool.borrow_mut().long(value);
        self.code.push(0x14);
        write_u16(&mut self.code, index);
    }

    pub(super) fn ldc_double(&mut self, value: f64) {
        let index = self.constant_pool.borrow_mut().double(value);
        self.code.push(0x14);
        write_u16(&mut self.code, index);
    }

    pub(super) fn ldc_index(&mut self, index: u16) {
        if index <= u8::MAX as u16 {
            self.code.push(0x12);
            self.code.push(index as u8);
        } else {
            self.code.push(0x13);
            write_u16(&mut self.code, index);
        }
    }

    pub(super) fn getstatic(&mut self, class: &str, name: &str, descriptor: &str) {
        let index = self
            .constant_pool
            .borrow_mut()
            .fieldref(class, name, descriptor);
        self.code.push(0xb2);
        write_u16(&mut self.code, index);
    }

    pub(super) fn new_class(&mut self, class: &str) {
        let index = self.constant_pool.borrow_mut().class(class);
        self.code.push(0xbb);
        write_u16(&mut self.code, index);
    }

    pub(super) fn anewarray(&mut self, class: &str) {
        let index = self.constant_pool.borrow_mut().class(class);
        self.code.push(0xbd);
        write_u16(&mut self.code, index);
    }

    pub(super) fn invokestatic(&mut self, class: &str, name: &str, descriptor: &str) {
        let index = self
            .constant_pool
            .borrow_mut()
            .methodref(class, name, descriptor);
        self.code.push(0xb8);
        write_u16(&mut self.code, index);
    }

    pub(super) fn invokespecial(&mut self, class: &str, name: &str, descriptor: &str) {
        let index = self
            .constant_pool
            .borrow_mut()
            .methodref(class, name, descriptor);
        self.code.push(0xb7);
        write_u16(&mut self.code, index);
    }

    pub(super) fn invokevirtual(&mut self, class: &str, name: &str, descriptor: &str) {
        let index = self
            .constant_pool
            .borrow_mut()
            .methodref(class, name, descriptor);
        self.code.push(0xb6);
        write_u16(&mut self.code, index);
    }
}

struct Patch {
    pos: usize,
    label: usize,
}
