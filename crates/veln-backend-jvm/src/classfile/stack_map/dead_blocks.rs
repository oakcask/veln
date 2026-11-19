struct DeadBlockFacts {
    required: BTreeMap<usize, VerificationType>,
    written: BTreeSet<usize>,
    target: Option<usize>,
}
fn dead_block_facts(
    code: &[u8],
    start: usize,
    control_targets: &BTreeSet<usize>,
) -> Result<DeadBlockFacts, JvmBackendError> {
    let mut written = BTreeSet::new();
    let mut required = BTreeMap::new();
    let mut offset = start;
    let mut target = None;
    while offset < code.len() {
        let opcode = code[offset];
        if let Some((slot, ty)) = dead_local_read(code, offset, opcode)?
            && !written.contains(&slot)
        {
            required.entry(slot).or_insert(ty);
        }
        if let Some(slot) = dead_local_write(code, offset, opcode) {
            written.insert(slot);
        }
        if let Some(end) = dead_block_end(code, offset, opcode)? {
            target = Some(end);
            break;
        }
        offset = instruction_end(code, offset)?;
        if control_targets.contains(&offset) {
            target = Some(offset);
            break;
        }
    }
    Ok(DeadBlockFacts {
        required,
        written,
        target,
    })
}

fn dead_local_read(
    code: &[u8],
    offset: usize,
    opcode: u8,
) -> Result<Option<(usize, VerificationType)>, JvmBackendError> {
    let read = match opcode {
        0x15 | 0x1a..=0x1d => Some((
            local_slot(code, offset, opcode, 0x1a)?,
            VerificationType::Integer,
        )),
        0x19 | 0x2a..=0x2d => Some((
            local_slot(code, offset, opcode, 0x2a)?,
            VerificationType::Object("java/lang/Object".to_string()),
        )),
        0x84 => Some((usize::from(code[offset + 1]), VerificationType::Integer)),
        _ => None,
    };
    Ok(read)
}

fn dead_local_write(code: &[u8], offset: usize, opcode: u8) -> Option<usize> {
    match opcode {
        0x36 | 0x3a | 0x84 => Some(usize::from(code[offset + 1])),
        0x3b..=0x3e => Some(usize::from(opcode - 0x3b)),
        0x4b..=0x4e => Some(usize::from(opcode - 0x4b)),
        _ => None,
    }
}

fn dead_block_end(
    code: &[u8],
    offset: usize,
    opcode: u8,
) -> Result<Option<usize>, JvmBackendError> {
    let target = match opcode {
        0xa7 => Some(short_branch_target(code, offset)?),
        0xc8 => Some(wide_branch_target(code, offset)?),
        0xac | 0xb0 | 0xb1 | 0xbf => Some(instruction_end(code, offset)?),
        _ => None,
    };
    Ok(target)
}

#[allow(clippy::too_many_arguments)]
fn analyze_dead_block(
    pool: &Rc<RefCell<ConstantPool>>,
    method: &MethodInfo,
    start: usize,
    mut frame: Frame,
    control_targets: &BTreeSet<usize>,
    states: &mut [Option<Frame>],
    frame_offsets: &mut BTreeSet<usize>,
    dead_frames: &mut BTreeMap<usize, Frame>,
) -> Result<(), JvmBackendError> {
    dead_frames.insert(start, frame.clone());
    let mut offset = start;
    loop {
        analyze_dead_handlers(
            pool,
            method,
            offset,
            &frame,
            control_targets,
            states,
            frame_offsets,
            dead_frames,
        )?;
        let next_dead_locals = frame.locals.clone();
        let (next, successors) = execute_instruction(pool, &method.code, offset, frame)?;
        for successor in successors {
            merge_or_analyze_dead_frame(
                pool,
                method,
                successor.offset,
                successor.frame,
                control_targets,
                states,
                frame_offsets,
                dead_frames,
            )?;
        }
        let Some(next_frame) = next else {
            analyze_dead_fallthrough(
                pool,
                method,
                offset,
                next_dead_locals,
                control_targets,
                states,
                frame_offsets,
                dead_frames,
            )?;
            break;
        };
        let next_offset = instruction_end(&method.code, offset)?;
        if next_offset >= method.code.len() {
            break;
        }
        if next_offset != start && control_targets.contains(&next_offset) {
            if states.get(next_offset).is_some_and(Option::is_some) {
                break;
            }
            merge_or_analyze_dead_frame(
                pool,
                method,
                next_offset,
                next_frame,
                control_targets,
                states,
                frame_offsets,
                dead_frames,
            )?;
            break;
        }
        offset = next_offset;
        frame = next_frame;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn analyze_dead_handlers(
    pool: &Rc<RefCell<ConstantPool>>,
    method: &MethodInfo,
    offset: usize,
    frame: &Frame,
    control_targets: &BTreeSet<usize>,
    states: &mut [Option<Frame>],
    frame_offsets: &mut BTreeSet<usize>,
    dead_frames: &mut BTreeMap<usize, Frame>,
) -> Result<(), JvmBackendError> {
    for exception in method
        .exceptions
        .iter()
        .filter(|exception| offset >= exception.start_pc && offset < exception.end_pc)
    {
        let handler = Frame {
            locals: frame.locals.clone(),
            stack: vec![VerificationType::Object(exception.catch_type.clone())],
        };
        merge_or_analyze_dead_frame(
            pool,
            method,
            exception.handler_pc,
            handler,
            control_targets,
            states,
            frame_offsets,
            dead_frames,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn analyze_dead_fallthrough(
    pool: &Rc<RefCell<ConstantPool>>,
    method: &MethodInfo,
    offset: usize,
    locals: Vec<VerificationType>,
    control_targets: &BTreeSet<usize>,
    states: &mut [Option<Frame>],
    frame_offsets: &mut BTreeSet<usize>,
    dead_frames: &mut BTreeMap<usize, Frame>,
) -> Result<(), JvmBackendError> {
    let next_offset = instruction_end(&method.code, offset)?;
    if next_offset >= method.code.len()
        || states.get(next_offset).is_some_and(Option::is_some)
        || dead_frames.contains_key(&next_offset)
        || method
            .exceptions
            .iter()
            .any(|exception| exception.handler_pc == next_offset)
    {
        return Ok(());
    }
    let next_frame = dead_frame(&method.code, next_offset, locals)?;
    merge_or_analyze_dead_frame(
        pool,
        method,
        next_offset,
        next_frame,
        control_targets,
        states,
        frame_offsets,
        dead_frames,
    )
}

#[allow(clippy::too_many_arguments)]
fn merge_or_analyze_dead_frame(
    pool: &Rc<RefCell<ConstantPool>>,
    method: &MethodInfo,
    offset: usize,
    frame: Frame,
    control_targets: &BTreeSet<usize>,
    states: &mut [Option<Frame>],
    frame_offsets: &mut BTreeSet<usize>,
    dead_frames: &mut BTreeMap<usize, Frame>,
) -> Result<(), JvmBackendError> {
    frame_offsets.insert(offset);
    if states.get(offset).is_some_and(Option::is_some) {
        return Ok(());
    }
    if let Some(existing) = dead_frames.get_mut(&offset) {
        let _ = merge_frame(existing, frame)?;
        return Ok(());
    }
    analyze_dead_block(
        pool,
        method,
        offset,
        frame,
        control_targets,
        states,
        frame_offsets,
        dead_frames,
    )
}

fn dead_frame(
    code: &[u8],
    offset: usize,
    mut locals: Vec<VerificationType>,
) -> Result<Frame, JvmBackendError> {
    let opcode = code[offset];
    if matches!(opcode, 0x19 | 0x2a..=0x2d) {
        let slot = local_slot(code, offset, opcode, 0x2a)?;
        let next = instruction_end(code, offset)?;
        if matches!(code.get(next), Some(0xb0 | 0xbf)) {
            let local = locals
                .get_mut(slot)
                .ok_or_else(|| stack_map_error("dead athrow local exceeds max_locals"))?;
            *local = VerificationType::Object("java/lang/Throwable".to_string());
        }
    }
    let stack = match opcode {
        0x36 | 0x3b..=0x3e => vec![VerificationType::Integer],
        0x3a | 0x4b..=0x4e | 0x57 | 0xb0 | 0xbf => {
            vec![VerificationType::Object("java/lang/Throwable".to_string())]
        }
        _ => Vec::new(),
    };
    Ok(Frame { locals, stack })
}
