struct Successor {
    offset: usize,
    frame: Frame,
}

fn execute_instruction(
    pool: &Rc<RefCell<ConstantPool>>,
    code: &[u8],
    offset: usize,
    mut frame: Frame,
) -> Result<(Option<Frame>, Vec<Successor>), JvmBackendError> {
    let opcode = *code
        .get(offset)
        .ok_or_else(|| stack_map_error("instruction offset is outside method code"))?;
    let end = instruction_end(code, offset)?;
    let successors = Vec::new();
    match opcode {
        0x00..=0x14 => execute_constant_instruction(pool, code, offset, opcode, &mut frame)?,
        0x15
        | 0x19
        | 0x1a..=0x1d
        | 0x2a..=0x2d
        | 0x32
        | 0x36
        | 0x3a
        | 0x3b..=0x3e
        | 0x4b..=0x4e
        | 0x53 => execute_local_instruction(code, offset, opcode, &mut frame)?,
        0x57 | 0x59 | 0x5f | 0x84 => execute_stack_instruction(opcode, &mut frame)?,
        0x99 | 0xa2 | 0xa7 | 0xc6 | 0xc7 => {
            return execute_short_branch(code, offset, opcode, frame);
        }
        0xc8 => return execute_wide_branch(code, offset, frame),
        0xac | 0xb0 | 0xb1 => return execute_return(opcode, frame),
        0xb2 | 0xb3 => execute_field_instruction(pool, code, offset, opcode, &mut frame)?,
        0xb6..=0xb8 => execute_invoke_instruction(pool, code, offset, opcode, &mut frame)?,
        0xbb | 0xbd | 0xbe | 0xbf => {
            if execute_object_instruction(pool, code, offset, opcode, &mut frame)? {
                return Ok((None, successors));
            }
        }
        _ => {
            return Err(stack_map_error(&format!(
                "unsupported emitted opcode 0x{opcode:02x} at {offset}"
            )));
        }
    }
    debug_assert!(end <= code.len());
    Ok((Some(frame), successors))
}

fn execute_constant_instruction(
    pool: &Rc<RefCell<ConstantPool>>,
    code: &[u8],
    offset: usize,
    opcode: u8,
    frame: &mut Frame,
) -> Result<(), JvmBackendError> {
    match opcode {
        0x00 => {}
        0x01 => frame.stack.push(VerificationType::Null),
        0x02..=0x08 | 0x10 | 0x11 => frame.stack.push(VerificationType::Integer),
        0x12 => frame
            .stack
            .push(constant_type(&pool.borrow(), u16::from(code[offset + 1]))?),
        0x13 | 0x14 => {
            let index = u16::from_be_bytes([code[offset + 1], code[offset + 2]]);
            frame.stack.push(constant_type(&pool.borrow(), index)?);
        }
        _ => return Err(stack_map_error("unsupported constant instruction")),
    }
    Ok(())
}

fn execute_local_instruction(
    code: &[u8],
    offset: usize,
    opcode: u8,
    frame: &mut Frame,
) -> Result<(), JvmBackendError> {
    match opcode {
        0x19 | 0x2a..=0x2d => {
            let slot = local_slot(code, offset, opcode, 0x2a)?;
            frame.stack.push(local(frame, slot)?.clone());
        }
        0x15 | 0x1a..=0x1d => {
            let slot = local_slot(code, offset, opcode, 0x1a)?;
            let _ = local(frame, slot)?;
            frame.stack.push(VerificationType::Integer);
        }
        0x3a | 0x4b..=0x4e => {
            let slot = local_slot(code, offset, opcode, 0x4b)?;
            let value = pop(frame)?;
            set_local(frame, slot, value)?;
        }
        0x36 | 0x3b..=0x3e => {
            let slot = local_slot(code, offset, opcode, 0x3b)?;
            let _ = pop(frame)?;
            set_local(frame, slot, VerificationType::Integer)?;
        }
        0x32 => {
            pop_n(frame, 2)?;
            frame
                .stack
                .push(VerificationType::Object("java/lang/Object".to_string()));
        }
        0x53 => pop_n(frame, 3)?,
        _ => return Err(stack_map_error("unsupported local instruction")),
    }
    Ok(())
}

fn execute_stack_instruction(opcode: u8, frame: &mut Frame) -> Result<(), JvmBackendError> {
    match opcode {
        0x57 => {
            let _ = pop(frame)?;
        }
        0x59 => {
            let value = pop(frame)?;
            frame.stack.push(value.clone());
            frame.stack.push(value);
        }
        0x5f => {
            let first = pop(frame)?;
            let second = pop(frame)?;
            frame.stack.push(first);
            frame.stack.push(second);
        }
        0x84 => {}
        _ => return Err(stack_map_error("unsupported stack instruction")),
    }
    Ok(())
}

fn execute_short_branch(
    code: &[u8],
    offset: usize,
    opcode: u8,
    mut frame: Frame,
) -> Result<(Option<Frame>, Vec<Successor>), JvmBackendError> {
    match opcode {
        0x99 | 0xc6 | 0xc7 => {
            let _ = pop(&mut frame)?;
        }
        0xa2 => pop_n(&mut frame, 2)?,
        0xa7 => {
            let offset = short_branch_target(code, offset)?;
            return Ok((None, vec![Successor { offset, frame }]));
        }
        _ => return Err(stack_map_error("unsupported short branch")),
    }
    let target = short_branch_target(code, offset)?;
    let successor = Successor {
        offset: target,
        frame: frame.clone(),
    };
    Ok((Some(frame), vec![successor]))
}

fn execute_wide_branch(
    code: &[u8],
    offset: usize,
    frame: Frame,
) -> Result<(Option<Frame>, Vec<Successor>), JvmBackendError> {
    let offset = wide_branch_target(code, offset)?;
    Ok((None, vec![Successor { offset, frame }]))
}

fn execute_return(
    opcode: u8,
    mut frame: Frame,
) -> Result<(Option<Frame>, Vec<Successor>), JvmBackendError> {
    if opcode != 0xb1 {
        let _ = pop(&mut frame)?;
    }
    Ok((None, Vec::new()))
}

fn execute_field_instruction(
    pool: &Rc<RefCell<ConstantPool>>,
    code: &[u8],
    offset: usize,
    opcode: u8,
    frame: &mut Frame,
) -> Result<(), JvmBackendError> {
    let index = cp_index(code, offset)?;
    let descriptor = field_descriptor(&pool.borrow(), index)?;
    let field_type = parse_field_type(&descriptor)?;
    if opcode == 0xb2 {
        frame.stack.push(field_type);
    } else {
        let _ = pop(frame)?;
    }
    Ok(())
}

fn execute_invoke_instruction(
    pool: &Rc<RefCell<ConstantPool>>,
    code: &[u8],
    offset: usize,
    opcode: u8,
    frame: &mut Frame,
) -> Result<(), JvmBackendError> {
    let index = cp_index(code, offset)?;
    let (owner, name, descriptor) = method_reference(&pool.borrow(), index)?;
    let (arguments, result) = parse_method_descriptor(&descriptor)?;
    pop_n(frame, arguments.len())?;
    if opcode != 0xb8 {
        let receiver = pop(frame)?;
        if opcode == 0xb7 && name == "<init>" {
            initialize_receiver(frame, receiver, owner);
        }
    }
    if let Some(result) = result {
        frame.stack.push(result);
    }
    Ok(())
}

fn execute_object_instruction(
    pool: &Rc<RefCell<ConstantPool>>,
    code: &[u8],
    offset: usize,
    opcode: u8,
    frame: &mut Frame,
) -> Result<bool, JvmBackendError> {
    match opcode {
        0xbb => frame.stack.push(VerificationType::Uninitialized(
            u16::try_from(offset).map_err(|_| stack_map_error("new offset exceeds u16"))?,
        )),
        0xbd => {
            let _ = pop(frame)?;
            let index = cp_index(code, offset)?;
            let class = class_name_for(&pool.borrow(), index)?;
            frame
                .stack
                .push(VerificationType::Object(format!("[L{class};")));
        }
        0xbe => {
            let _ = pop(frame)?;
            frame.stack.push(VerificationType::Integer);
        }
        0xbf => {
            let _ = pop(frame)?;
            return Ok(true);
        }
        _ => return Err(stack_map_error("unsupported object instruction")),
    }
    Ok(false)
}
