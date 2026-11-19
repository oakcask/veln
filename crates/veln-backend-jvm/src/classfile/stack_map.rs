use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
enum VerificationType {
    Top,
    Integer,
    Float,
    Double,
    Long,
    Null,
    UninitializedThis,
    Object(String),
    Uninitialized(u16),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Frame {
    locals: Vec<VerificationType>,
    stack: Vec<VerificationType>,
}

pub(super) fn stack_map_table(
    pool: &Rc<RefCell<ConstantPool>>,
    class_name: &str,
    method: &MethodInfo,
) -> Result<Vec<u8>, JvmBackendError> {
    let initial = initial_frame(class_name, method)?;
    let mut states = vec![None; method.code.len()];
    if method.code.is_empty() {
        return Ok(Vec::new());
    }
    states[0] = Some(initial);
    let mut queue = VecDeque::from([0usize]);
    let mut control_targets = branch_targets(&method.code)?;
    let exception_handlers = method
        .exceptions
        .iter()
        .map(|exception| exception.handler_pc)
        .collect::<BTreeSet<_>>();
    for exception in &method.exceptions {
        control_targets.insert(exception.handler_pc);
    }
    let mut frame_offsets = control_targets.clone();
    let mut dead_frames = BTreeMap::new();

    while let Some(offset) = queue.pop_front() {
        let frame = states[offset]
            .clone()
            .ok_or_else(|| stack_map_error("missing reachable frame"))?;
        let dead_locals = frame.locals.clone();
        for exception in method
            .exceptions
            .iter()
            .filter(|exception| offset >= exception.start_pc && offset < exception.end_pc)
        {
            let handler = Frame {
                locals: frame.locals.clone(),
                stack: vec![VerificationType::Object(exception.catch_type.clone())],
            };
            merge_state(&mut states, exception.handler_pc, handler, &mut queue)?;
        }

        let (next, successors) = execute_instruction(pool, &method.code, offset, frame)?;
        for successor in successors {
            frame_offsets.insert(successor.offset);
            merge_state(&mut states, successor.offset, successor.frame, &mut queue)?;
        }
        if let Some(frame) = next {
            let next_offset = instruction_end(&method.code, offset)?;
            if next_offset < method.code.len() {
                merge_state(&mut states, next_offset, frame, &mut queue)?;
            }
        } else {
            let next_offset = instruction_end(&method.code, offset)?;
            if next_offset < method.code.len()
                && states.get(next_offset).is_some_and(Option::is_none)
                && !dead_frames.contains_key(&next_offset)
                && !exception_handlers.contains(&next_offset)
            {
                frame_offsets.insert(next_offset);
                analyze_dead_block(
                    pool,
                    method,
                    next_offset,
                    dead_frame(&method.code, next_offset, dead_locals)?,
                    &control_targets,
                    &mut states,
                    &mut frame_offsets,
                    &mut dead_frames,
                )?;
            }
        }
    }

    let mut frame_map = BTreeMap::new();
    for offset in frame_offsets {
        let Some(frame) = states
            .get(offset)
            .and_then(Option::as_ref)
            .or_else(|| dead_frames.get(&offset))
        else {
            continue;
        };
        frame_map.insert(offset, frame.clone());
    }
    loop {
        let snapshot = frame_map.clone();
        let mut next_map = snapshot.clone();
        for (offset, frame) in &mut next_map {
            if states.get(*offset).is_some_and(Option::is_some) {
                continue;
            }
            for (slot, required) in
                dead_block_required_locals(&method.code, *offset, &control_targets)?
            {
                if frame.locals[slot] == VerificationType::Top {
                    frame.locals[slot] = required;
                }
            }
            let Some(target) = dead_block_target(&method.code, *offset, &control_targets)? else {
                continue;
            };
            let Some(target_frame) = snapshot.get(&target) else {
                continue;
            };
            let written = dead_block_written_locals(&method.code, *offset, &control_targets)?;
            for (slot, (local, target_local)) in frame
                .locals
                .iter_mut()
                .zip(&target_frame.locals)
                .enumerate()
            {
                if written.contains(&slot) {
                    continue;
                }
                if *local == VerificationType::Top && *target_local != VerificationType::Top {
                    *local = target_local.clone();
                }
            }
        }
        for exception in &method.exceptions {
            if states.get(exception.start_pc).is_some_and(Option::is_some) {
                continue;
            }
            let Some(handler) = snapshot.get(&exception.handler_pc) else {
                continue;
            };
            let Some(source) = next_map.get_mut(&exception.start_pc) else {
                continue;
            };
            for (local, required) in source.locals.iter_mut().zip(&handler.locals) {
                if *local == VerificationType::Top && *required != VerificationType::Top {
                    *local = required.clone();
                }
            }
        }
        if next_map == snapshot {
            frame_map = next_map;
            break;
        }
        frame_map = next_map;
    }
    let frames = frame_map.into_iter().collect::<Vec<_>>();
    serialize_frames(pool, &frames)
}

fn dead_block_written_locals(
    code: &[u8],
    start: usize,
    control_targets: &BTreeSet<usize>,
) -> Result<BTreeSet<usize>, JvmBackendError> {
    let mut written = BTreeSet::new();
    let mut offset = start;
    while offset < code.len() {
        match code[offset] {
            0x36 | 0x3a => {
                written.insert(usize::from(code[offset + 1]));
            }
            0x3b..=0x3e => {
                written.insert(usize::from(code[offset] - 0x3b));
            }
            0x4b..=0x4e => {
                written.insert(usize::from(code[offset] - 0x4b));
            }
            0xa7 | 0xc8 | 0xac | 0xb0 | 0xb1 | 0xbf => break,
            _ => {}
        }
        offset = instruction_end(code, offset)?;
        if control_targets.contains(&offset) {
            break;
        }
    }
    Ok(written)
}

fn dead_block_required_locals(
    code: &[u8],
    start: usize,
    control_targets: &BTreeSet<usize>,
) -> Result<BTreeMap<usize, VerificationType>, JvmBackendError> {
    let mut written = BTreeSet::new();
    let mut required = BTreeMap::new();
    let mut offset = start;
    while offset < code.len() {
        let opcode = code[offset];
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
        if let Some((slot, ty)) = read
            && !written.contains(&slot)
        {
            required.entry(slot).or_insert(ty);
        }
        match opcode {
            0x36 | 0x3a => {
                written.insert(usize::from(code[offset + 1]));
            }
            0x3b..=0x3e => {
                written.insert(usize::from(opcode - 0x3b));
            }
            0x4b..=0x4e => {
                written.insert(usize::from(opcode - 0x4b));
            }
            0x84 => {
                written.insert(usize::from(code[offset + 1]));
            }
            0xa7 | 0xc8 | 0xac | 0xb0 | 0xb1 | 0xbf => break,
            _ => {}
        }
        offset = instruction_end(code, offset)?;
        if control_targets.contains(&offset) {
            break;
        }
    }
    Ok(required)
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
        for exception in method
            .exceptions
            .iter()
            .filter(|exception| offset >= exception.start_pc && offset < exception.end_pc)
        {
            frame_offsets.insert(exception.handler_pc);
            let handler = Frame {
                locals: frame.locals.clone(),
                stack: vec![VerificationType::Object(exception.catch_type.clone())],
            };
            if states
                .get(exception.handler_pc)
                .is_some_and(Option::is_some)
            {
                continue;
            } else if let Some(existing) = dead_frames.get_mut(&exception.handler_pc) {
                merge_frame(existing, handler)?;
            } else {
                analyze_dead_block(
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
        }
        let next_dead_locals = frame.locals.clone();
        let (next, successors) = execute_instruction(pool, &method.code, offset, frame)?;
        for successor in successors {
            frame_offsets.insert(successor.offset);
            if states.get(successor.offset).is_some_and(Option::is_some) {
                continue;
            } else if let Some(existing) = dead_frames.get_mut(&successor.offset) {
                merge_frame(existing, successor.frame)?;
            } else {
                analyze_dead_block(
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
        }
        let Some(next_frame) = next else {
            let next_offset = instruction_end(&method.code, offset)?;
            if next_offset < method.code.len()
                && states.get(next_offset).is_some_and(Option::is_none)
                && !dead_frames.contains_key(&next_offset)
                && !method
                    .exceptions
                    .iter()
                    .any(|exception| exception.handler_pc == next_offset)
            {
                frame_offsets.insert(next_offset);
                let next_frame = dead_frame(&method.code, next_offset, next_dead_locals)?;
                analyze_dead_block(
                    pool,
                    method,
                    next_offset,
                    next_frame,
                    control_targets,
                    states,
                    frame_offsets,
                    dead_frames,
                )?;
            }
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
            if let Some(existing) = dead_frames.get_mut(&next_offset) {
                merge_frame(existing, next_frame)?;
            } else {
                frame_offsets.insert(next_offset);
                analyze_dead_block(
                    pool,
                    method,
                    next_offset,
                    next_frame,
                    control_targets,
                    states,
                    frame_offsets,
                    dead_frames,
                )?;
            }
            break;
        }
        offset = next_offset;
        frame = next_frame;
    }
    Ok(())
}

fn dead_block_target(
    code: &[u8],
    start: usize,
    control_targets: &BTreeSet<usize>,
) -> Result<Option<usize>, JvmBackendError> {
    let mut offset = start;
    while offset < code.len() {
        match code[offset] {
            0xa7 => return Ok(Some(short_branch_target(code, offset)?)),
            0xc8 => return Ok(Some(wide_branch_target(code, offset)?)),
            0xac | 0xb0 | 0xb1 | 0xbf => return Ok(Some(instruction_end(code, offset)?)),
            _ => {
                offset = instruction_end(code, offset)?;
                if control_targets.contains(&offset) {
                    return Ok(Some(offset));
                }
            }
        }
    }
    Ok(None)
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

fn branch_targets(code: &[u8]) -> Result<BTreeSet<usize>, JvmBackendError> {
    let mut targets = BTreeSet::new();
    let mut offset = 0;
    while offset < code.len() {
        match code[offset] {
            0x99 | 0xa2 | 0xa7 | 0xc6 | 0xc7 => {
                targets.insert(short_branch_target(code, offset)?);
            }
            0xc8 => {
                targets.insert(wide_branch_target(code, offset)?);
            }
            _ => {}
        }
        offset = instruction_end(code, offset)?;
    }
    Ok(targets)
}

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
    let mut successors = Vec::new();
    match opcode {
        0x00 => {}
        0x01 => frame.stack.push(VerificationType::Null),
        0x02..=0x08 | 0x10 | 0x11 => frame.stack.push(VerificationType::Integer),
        0x12 | 0x13 => {
            let index = if opcode == 0x12 {
                u16::from(code[offset + 1])
            } else {
                u16::from_be_bytes([code[offset + 1], code[offset + 2]])
            };
            frame.stack.push(constant_type(&pool.borrow(), index)?);
        }
        0x14 => {
            let index = u16::from_be_bytes([code[offset + 1], code[offset + 2]]);
            frame.stack.push(constant_type(&pool.borrow(), index)?);
        }
        0x19 | 0x2a..=0x2d => {
            let slot = local_slot(code, offset, opcode, 0x2a)?;
            frame.stack.push(local(&frame, slot)?.clone());
        }
        0x15 | 0x1a..=0x1d => {
            let slot = local_slot(code, offset, opcode, 0x1a)?;
            let _ = local(&frame, slot)?;
            frame.stack.push(VerificationType::Integer);
        }
        0x3a | 0x4b..=0x4e => {
            let slot = local_slot(code, offset, opcode, 0x4b)?;
            let value = pop(&mut frame)?;
            set_local(&mut frame, slot, value)?;
        }
        0x36 | 0x3b..=0x3e => {
            let slot = local_slot(code, offset, opcode, 0x3b)?;
            let _ = pop(&mut frame)?;
            set_local(&mut frame, slot, VerificationType::Integer)?;
        }
        0x32 => {
            pop_n(&mut frame, 2)?;
            frame
                .stack
                .push(VerificationType::Object("java/lang/Object".to_string()));
        }
        0x53 => {
            pop_n(&mut frame, 3)?;
        }
        0x57 => {
            let _ = pop(&mut frame)?;
        }
        0x59 => {
            let value = pop(&mut frame)?;
            frame.stack.push(value.clone());
            frame.stack.push(value);
        }
        0x5f => {
            let first = pop(&mut frame)?;
            let second = pop(&mut frame)?;
            frame.stack.push(first);
            frame.stack.push(second);
        }
        0x84 => {}
        0x99 | 0xc6 | 0xc7 => {
            let _ = pop(&mut frame)?;
            let target = short_branch_target(code, offset)?;
            successors.push(Successor {
                offset: target,
                frame: frame.clone(),
            });
        }
        0xa2 => {
            pop_n(&mut frame, 2)?;
            let target = short_branch_target(code, offset)?;
            successors.push(Successor {
                offset: target,
                frame: frame.clone(),
            });
        }
        0xa7 => {
            let target = short_branch_target(code, offset)?;
            successors.push(Successor {
                offset: target,
                frame,
            });
            return Ok((None, successors));
        }
        0xc8 => {
            let target = wide_branch_target(code, offset)?;
            successors.push(Successor {
                offset: target,
                frame,
            });
            return Ok((None, successors));
        }
        0xac | 0xb0 => {
            let _ = pop(&mut frame)?;
            return Ok((None, successors));
        }
        0xb1 => return Ok((None, successors)),
        0xb2 | 0xb3 => {
            let index = cp_index(code, offset)?;
            let descriptor = field_descriptor(&pool.borrow(), index)?;
            let field_type = parse_field_type(&descriptor)?;
            if opcode == 0xb2 {
                frame.stack.push(field_type);
            } else {
                let _ = pop(&mut frame)?;
            }
        }
        0xb6..=0xb8 => {
            let index = cp_index(code, offset)?;
            let (owner, name, descriptor) = method_reference(&pool.borrow(), index)?;
            let (arguments, result) = parse_method_descriptor(&descriptor)?;
            pop_n(&mut frame, arguments.len())?;
            if opcode != 0xb8 {
                let receiver = pop(&mut frame)?;
                if opcode == 0xb7 && name == "<init>" {
                    initialize_receiver(&mut frame, receiver, owner);
                }
            }
            if let Some(result) = result {
                frame.stack.push(result);
            }
        }
        0xbb => frame.stack.push(VerificationType::Uninitialized(
            u16::try_from(offset).map_err(|_| stack_map_error("new offset exceeds u16"))?,
        )),
        0xbd => {
            let _ = pop(&mut frame)?;
            let index = cp_index(code, offset)?;
            let class = class_name_for(&pool.borrow(), index)?;
            frame
                .stack
                .push(VerificationType::Object(format!("[L{class};")));
        }
        0xbe => {
            let _ = pop(&mut frame)?;
            frame.stack.push(VerificationType::Integer);
        }
        0xbf => {
            let _ = pop(&mut frame)?;
            return Ok((None, successors));
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

fn initial_frame(class_name: &str, method: &MethodInfo) -> Result<Frame, JvmBackendError> {
    let mut locals = vec![VerificationType::Top; usize::from(method.max_locals)];
    let mut slot = 0;
    if method.access_flags & 0x0008 == 0 {
        locals[slot] = if method.name == "<init>" {
            VerificationType::UninitializedThis
        } else {
            VerificationType::Object(class_name.to_string())
        };
        slot += 1;
    }
    let (arguments, _) = parse_method_descriptor(&method.descriptor)?;
    for argument in arguments {
        if slot >= locals.len() {
            return Err(stack_map_error("method arguments exceed max_locals"));
        }
        let width = matches!(argument, VerificationType::Long | VerificationType::Double)
            .then_some(2)
            .unwrap_or(1);
        locals[slot] = argument;
        slot += width;
    }
    Ok(Frame {
        locals,
        stack: Vec::new(),
    })
}

fn merge_state(
    states: &mut [Option<Frame>],
    offset: usize,
    incoming: Frame,
    queue: &mut VecDeque<usize>,
) -> Result<(), JvmBackendError> {
    let state = states
        .get_mut(offset)
        .ok_or_else(|| stack_map_error("branch target is outside method code"))?;
    match state {
        None => {
            *state = Some(incoming);
            queue.push_back(offset);
        }
        Some(existing) => {
            let changed = merge_frame(existing, incoming)?;
            if changed {
                queue.push_back(offset);
            }
        }
    }
    Ok(())
}

fn merge_frame(existing: &mut Frame, incoming: Frame) -> Result<bool, JvmBackendError> {
    if existing.stack.len() != incoming.stack.len()
        || existing.locals.len() != incoming.locals.len()
    {
        return Err(stack_map_error(
            "incompatible frame shapes at control-flow merge",
        ));
    }
    let mut changed = false;
    for (left, right) in existing
        .locals
        .iter_mut()
        .zip(incoming.locals)
        .chain(existing.stack.iter_mut().zip(incoming.stack))
    {
        let merged = merge_type(left, &right)?;
        if *left != merged {
            *left = merged;
            changed = true;
        }
    }
    Ok(changed)
}

fn merge_type(
    left: &VerificationType,
    right: &VerificationType,
) -> Result<VerificationType, JvmBackendError> {
    if left == right {
        return Ok(left.clone());
    }
    match (left, right) {
        (VerificationType::Top, _) | (_, VerificationType::Top) => Ok(VerificationType::Top),
        (VerificationType::Null, VerificationType::Object(name))
        | (VerificationType::Object(name), VerificationType::Null) => {
            Ok(VerificationType::Object(name.clone()))
        }
        (VerificationType::Object(_), VerificationType::Object(_)) => {
            Ok(VerificationType::Object("java/lang/Object".to_string()))
        }
        _ => Err(stack_map_error(
            "incompatible verification types at control-flow merge",
        )),
    }
}

fn initialize_receiver(frame: &mut Frame, receiver: VerificationType, owner: String) {
    let initialized = VerificationType::Object(owner);
    for value in frame.locals.iter_mut().chain(frame.stack.iter_mut()) {
        if *value == receiver {
            *value = initialized.clone();
        }
    }
}

fn serialize_frames(
    pool: &Rc<RefCell<ConstantPool>>,
    frames: &[(usize, Frame)],
) -> Result<Vec<u8>, JvmBackendError> {
    let mut out = Vec::new();
    write_u16(
        &mut out,
        u16::try_from(frames.len()).map_err(|_| stack_map_error("too many stack-map frames"))?,
    );
    let mut previous = None;
    for (offset, frame) in frames {
        out.push(255);
        let delta = match previous {
            None => *offset,
            Some(previous) => offset
                .checked_sub(previous + 1)
                .ok_or_else(|| stack_map_error("stack-map offsets are not increasing"))?,
        };
        write_u16(
            &mut out,
            u16::try_from(delta).map_err(|_| stack_map_error("stack-map offset exceeds u16"))?,
        );
        let locals = trimmed_locals(&frame.locals);
        write_u16(
            &mut out,
            u16::try_from(locals.len()).map_err(|_| stack_map_error("too many frame locals"))?,
        );
        for value in locals {
            write_verification_type(pool, &mut out, value);
        }
        write_u16(
            &mut out,
            u16::try_from(frame.stack.len())
                .map_err(|_| stack_map_error("too many frame stack values"))?,
        );
        for value in &frame.stack {
            write_verification_type(pool, &mut out, value);
        }
        previous = Some(*offset);
    }
    Ok(out)
}

fn trimmed_locals(locals: &[VerificationType]) -> &[VerificationType] {
    let length = locals
        .iter()
        .rposition(|value| *value != VerificationType::Top)
        .map_or(0, |index| index + 1);
    &locals[..length]
}

fn write_verification_type(
    pool: &Rc<RefCell<ConstantPool>>,
    out: &mut Vec<u8>,
    value: &VerificationType,
) {
    match value {
        VerificationType::Top => out.push(0),
        VerificationType::Integer => out.push(1),
        VerificationType::Float => out.push(2),
        VerificationType::Double => out.push(3),
        VerificationType::Long => out.push(4),
        VerificationType::Null => out.push(5),
        VerificationType::UninitializedThis => out.push(6),
        VerificationType::Object(name) => {
            out.push(7);
            write_u16(out, pool.borrow_mut().class(name));
        }
        VerificationType::Uninitialized(offset) => {
            out.push(8);
            write_u16(out, *offset);
        }
    }
}

fn parse_method_descriptor(
    descriptor: &str,
) -> Result<(Vec<VerificationType>, Option<VerificationType>), JvmBackendError> {
    let bytes = descriptor.as_bytes();
    if bytes.first() != Some(&b'(') {
        return Err(stack_map_error("invalid method descriptor"));
    }
    let mut index = 1;
    let mut arguments = Vec::new();
    while bytes.get(index) != Some(&b')') {
        arguments.push(parse_descriptor_type(descriptor, &mut index)?);
    }
    index += 1;
    let result = if bytes.get(index) == Some(&b'V') {
        None
    } else {
        Some(parse_descriptor_type(descriptor, &mut index)?)
    };
    Ok((arguments, result))
}

fn parse_field_type(descriptor: &str) -> Result<VerificationType, JvmBackendError> {
    parse_descriptor_type(descriptor, &mut 0)
}

fn parse_descriptor_type(
    descriptor: &str,
    index: &mut usize,
) -> Result<VerificationType, JvmBackendError> {
    let bytes = descriptor.as_bytes();
    let start = *index;
    let byte = *bytes
        .get(*index)
        .ok_or_else(|| stack_map_error("truncated descriptor"))?;
    *index += 1;
    match byte {
        b'B' | b'C' | b'I' | b'S' | b'Z' => Ok(VerificationType::Integer),
        b'F' => Ok(VerificationType::Float),
        b'D' => Ok(VerificationType::Double),
        b'J' => Ok(VerificationType::Long),
        b'L' => {
            let end = descriptor[*index..]
                .find(';')
                .map(|relative| *index + relative)
                .ok_or_else(|| stack_map_error("unterminated object descriptor"))?;
            let name = descriptor[*index..end].to_string();
            *index = end + 1;
            Ok(VerificationType::Object(name))
        }
        b'[' => {
            while bytes.get(*index) == Some(&b'[') {
                *index += 1;
            }
            if bytes.get(*index) == Some(&b'L') {
                let _ = parse_descriptor_type(descriptor, index)?;
            } else {
                *index += 1;
            }
            Ok(VerificationType::Object(
                descriptor[start..*index].to_string(),
            ))
        }
        _ => Err(stack_map_error("unsupported descriptor type")),
    }
}

fn instruction_end(code: &[u8], offset: usize) -> Result<usize, JvmBackendError> {
    let opcode = *code
        .get(offset)
        .ok_or_else(|| stack_map_error("truncated instruction"))?;
    let length = match opcode {
        0x10 | 0x12 | 0x15 | 0x19 | 0x36 | 0x3a => 2,
        0x11 | 0x13 | 0x14 | 0x84 | 0x99 | 0xa2 | 0xa7 | 0xb2 | 0xb3 | 0xb6 | 0xb7 | 0xb8
        | 0xbb | 0xbd | 0xc6 | 0xc7 => 3,
        0xc8 => 5,
        _ => 1,
    };
    offset
        .checked_add(length)
        .filter(|end| *end <= code.len())
        .ok_or_else(|| stack_map_error("truncated instruction operands"))
}

fn local_slot(
    code: &[u8],
    offset: usize,
    opcode: u8,
    compact_base: u8,
) -> Result<usize, JvmBackendError> {
    if opcode >= compact_base {
        Ok(usize::from(opcode - compact_base))
    } else {
        code.get(offset + 1)
            .copied()
            .map(usize::from)
            .ok_or_else(|| stack_map_error("truncated local instruction"))
    }
}

fn local(frame: &Frame, slot: usize) -> Result<&VerificationType, JvmBackendError> {
    frame
        .locals
        .get(slot)
        .ok_or_else(|| stack_map_error("local slot exceeds max_locals"))
}

fn set_local(
    frame: &mut Frame,
    slot: usize,
    value: VerificationType,
) -> Result<(), JvmBackendError> {
    let local = frame
        .locals
        .get_mut(slot)
        .ok_or_else(|| stack_map_error("local slot exceeds max_locals"))?;
    *local = value;
    Ok(())
}

fn pop(frame: &mut Frame) -> Result<VerificationType, JvmBackendError> {
    frame
        .stack
        .pop()
        .ok_or_else(|| stack_map_error("operand stack underflow"))
}

fn pop_n(frame: &mut Frame, count: usize) -> Result<(), JvmBackendError> {
    for _ in 0..count {
        let _ = pop(frame)?;
    }
    Ok(())
}

fn cp_index(code: &[u8], offset: usize) -> Result<u16, JvmBackendError> {
    let bytes = code
        .get(offset + 1..offset + 3)
        .ok_or_else(|| stack_map_error("truncated constant-pool instruction"))?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn short_branch_target(code: &[u8], offset: usize) -> Result<usize, JvmBackendError> {
    let bytes = code
        .get(offset + 1..offset + 3)
        .ok_or_else(|| stack_map_error("truncated short branch"))?;
    branch_target(offset, i32::from(i16::from_be_bytes([bytes[0], bytes[1]])))
}

fn wide_branch_target(code: &[u8], offset: usize) -> Result<usize, JvmBackendError> {
    let bytes = code
        .get(offset + 1..offset + 5)
        .ok_or_else(|| stack_map_error("truncated wide branch"))?;
    branch_target(
        offset,
        i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
    )
}

fn branch_target(offset: usize, relative: i32) -> Result<usize, JvmBackendError> {
    let target = i64::try_from(offset)
        .ok()
        .and_then(|offset| offset.checked_add(i64::from(relative)))
        .and_then(|target| usize::try_from(target).ok())
        .ok_or_else(|| stack_map_error("branch target is not representable"))?;
    Ok(target)
}

fn constant_type(pool: &ConstantPool, index: u16) -> Result<VerificationType, JvmBackendError> {
    match pool.key(index) {
        Some(CpKey::Integer(_)) => Ok(VerificationType::Integer),
        Some(CpKey::Long(_)) => Ok(VerificationType::Long),
        Some(CpKey::Double(_)) => Ok(VerificationType::Double),
        Some(CpKey::String(_)) => Ok(VerificationType::Object("java/lang/String".to_string())),
        _ => Err(stack_map_error("unsupported ldc constant type")),
    }
}

fn field_descriptor(pool: &ConstantPool, index: u16) -> Result<String, JvmBackendError> {
    match pool.key(index) {
        Some(CpKey::Fieldref(_, _, descriptor)) => Ok(descriptor.clone()),
        _ => Err(stack_map_error(
            "field instruction references a non-field constant",
        )),
    }
}

fn method_reference(
    pool: &ConstantPool,
    index: u16,
) -> Result<(String, String, String), JvmBackendError> {
    match pool.key(index) {
        Some(CpKey::Methodref(owner, name, descriptor)) => {
            Ok((owner.clone(), name.clone(), descriptor.clone()))
        }
        _ => Err(stack_map_error(
            "invoke instruction references a non-method constant",
        )),
    }
}

fn class_name_for(pool: &ConstantPool, index: u16) -> Result<String, JvmBackendError> {
    match pool.key(index) {
        Some(CpKey::Class(name)) => Ok(name.clone()),
        _ => Err(stack_map_error(
            "class instruction references a non-class constant",
        )),
    }
}

fn stack_map_error(_message: &str) -> JvmBackendError {
    JvmBackendError::new("JVM stack-map generation failed")
}
