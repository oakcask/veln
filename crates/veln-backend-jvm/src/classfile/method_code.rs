use super::*;

pub(super) struct MethodCode {
    constant_pool: Rc<RefCell<ConstantPool>>,
    pub(super) code: Vec<u8>,
    labels: Vec<Option<usize>>,
    branch_patches: Vec<Vec<Patch>>,
    exception_patches: Vec<Vec<usize>>,
    #[cfg(test)]
    branch_patch_visit_count: usize,
    #[cfg(test)]
    exception_patch_visit_count: usize,
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
            branch_patches: Vec::new(),
            exception_patches: Vec::new(),
            #[cfg(test)]
            branch_patch_visit_count: 0,
            #[cfg(test)]
            exception_patch_visit_count: 0,
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
        self.branch_patches.push(Vec::new());
        self.exception_patches.push(Vec::new());
        id
    }

    pub(super) fn bind(&mut self, label: usize) {
        let target = self.code.len();
        self.labels[label] = Some(target);
        for patch in std::mem::take(&mut self.branch_patches[label]) {
            self.apply_branch_patch(patch, target);
        }
        for exception_index in std::mem::take(&mut self.exception_patches[label]) {
            self.exceptions[exception_index].handler_pc = target;
            #[cfg(test)]
            {
                self.exception_patch_visit_count += 1;
            }
        }
    }

    pub(super) fn add_exception_handler_to_label(
        &mut self,
        start_pc: usize,
        end_pc: usize,
        handler_label: usize,
    ) {
        let exception_index = self.exceptions.len();
        self.exceptions.push(ExceptionHandler {
            start_pc,
            end_pc,
            handler_pc: 0,
            catch_type: "java/lang/Throwable".to_string(),
        });
        if let Some(handler_pc) = self.labels[handler_label] {
            self.exceptions[exception_index].handler_pc = handler_pc;
            #[cfg(test)]
            {
                self.exception_patch_visit_count += 1;
            }
        } else {
            self.exception_patches[handler_label].push(exception_index);
        }
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
        self.add_branch_patch(
            Patch {
                pos,
                width: BranchWidth::Short,
            },
            label,
        );
    }

    pub(super) fn branch_wide_to(&mut self, label: usize) {
        let pos = self.code.len();
        self.code.push(0xc8);
        self.code.extend_from_slice(&[0, 0, 0, 0]);
        self.add_branch_patch(
            Patch {
                pos,
                width: BranchWidth::Wide,
            },
            label,
        );
    }

    pub(super) fn branch_wide_from_any_stack_to(&mut self, label: usize) {
        // An exception-handler entry discards the operand stack from the throwing
        // edge. Use a one-instruction synthetic edge so expression-local values
        // cannot reach a shared unwind block with incompatible stack heights.
        self.code.push(0x01);
        let try_start = self.code.len();
        self.code.push(0xbf);
        let handler_pc = self.code.len();
        self.code.push(0x57);
        self.branch_wide_to(label);
        self.exceptions.push(ExceptionHandler {
            start_pc: try_start,
            end_pc: handler_pc,
            handler_pc,
            catch_type: "java/lang/Throwable".to_string(),
        });
    }

    fn add_branch_patch(&mut self, patch: Patch, label: usize) {
        if let Some(target) = self.labels[label] {
            self.apply_branch_patch(patch, target);
        } else {
            self.branch_patches[label].push(patch);
        }
    }

    fn apply_branch_patch(&mut self, patch: Patch, target: usize) {
        let offset = target as isize - patch.pos as isize;
        match patch.width {
            BranchWidth::Short => {
                let bytes = (offset as i16).to_be_bytes();
                self.code[patch.pos + 1] = bytes[0];
                self.code[patch.pos + 2] = bytes[1];
            }
            BranchWidth::Wide => {
                let bytes = (offset as i32).to_be_bytes();
                self.code[patch.pos + 1..patch.pos + 5].copy_from_slice(&bytes);
            }
        }
        #[cfg(test)]
        {
            self.branch_patch_visit_count += 1;
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
    width: BranchWidth,
}

enum BranchWidth {
    Short,
    Wide,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_patch_visits_grow_linearly_with_adjacent_label_counts() {
        fn visits(label_count: usize) -> usize {
            let pool = Rc::new(RefCell::new(ConstantPool::new()));
            let mut code = MethodCode::new(pool);
            let labels = (0..label_count)
                .map(|_| code.new_label())
                .collect::<Vec<_>>();

            for &label in &labels {
                code.branch_wide_to(label);
            }
            for &label in &labels {
                code.bind(label);
                code.branch_wide_to(label);
            }

            assert!(code.branch_patches.iter().all(Vec::is_empty));
            code.branch_patch_visit_count
        }

        let small = visits(1_000);
        let large = visits(2_000);

        assert_eq!(small, 2_000);
        assert_eq!(large, 4_000);
        assert_eq!(large, small * 2);
    }

    #[test]
    fn exception_patches_are_resolved_once_when_their_label_is_bound() {
        let pool = Rc::new(RefCell::new(ConstantPool::new()));
        let mut code = MethodCode::new(pool);
        let handler = code.new_label();

        for index in 0..2_000 {
            code.add_exception_handler_to_label(index, index + 1, handler);
        }
        code.op(0x00);
        code.bind(handler);
        code.add_exception_handler_to_label(2_000, 2_001, handler);

        assert_eq!(code.exception_patch_visit_count, 2_001);
        assert!(code.exception_patches[handler].is_empty());
        assert!(
            code.exceptions
                .iter()
                .all(|exception| exception.handler_pc == code.code.len())
        );
    }
}
