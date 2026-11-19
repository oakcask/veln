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
    if method.code.is_empty() {
        return Ok(Vec::new());
    }
    let analysis = analyze_frames(pool, class_name, method)?;
    let frame_map = complete_dead_frames(method, &analysis)?;
    let frames = frame_map.into_iter().collect::<Vec<_>>();
    serialize_frames(pool, &frames)
}

struct FrameAnalysis {
    states: Vec<Option<Frame>>,
    reachable: Vec<bool>,
    control_targets: BTreeSet<usize>,
    frame_offsets: BTreeSet<usize>,
    dead_frames: BTreeMap<usize, Frame>,
}

fn analyze_frames(
    pool: &Rc<RefCell<ConstantPool>>,
    class_name: &str,
    method: &MethodInfo,
) -> Result<FrameAnalysis, JvmBackendError> {
    let initial = initial_frame(class_name, method)?;
    let mut states = vec![None; method.code.len()];
    let mut reachable = vec![false; method.code.len()];
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

    while let Some(start) = queue.pop_front() {
        let mut offset = start;
        let mut frame = states[start]
            .clone()
            .ok_or_else(|| stack_map_error("missing reachable frame"))?;
        loop {
            reachable[offset] = true;
            merge_reachable_handlers(method, offset, &frame, &mut states, &mut queue)?;
            let dead_locals =
                matches!(method.code[offset], 0xa7 | 0xac | 0xb0 | 0xb1 | 0xbf | 0xc8)
                    .then(|| frame.locals.clone());
            let (next, successors) = execute_instruction(pool, &method.code, offset, frame)?;
            merge_reachable_successors(successors, &mut states, &mut queue, &mut frame_offsets)?;
            let next_offset = instruction_end(&method.code, offset)?;
            let Some(next_frame) = next else {
                start_dead_fallthrough(
                    pool,
                    method,
                    next_offset,
                    dead_locals.ok_or_else(|| stack_map_error("missing terminal locals"))?,
                    &exception_handlers,
                    &control_targets,
                    &mut states,
                    &mut frame_offsets,
                    &mut dead_frames,
                )?;
                break;
            };
            if next_offset >= method.code.len() {
                break;
            }
            if control_targets.contains(&next_offset) {
                merge_state(&mut states, next_offset, next_frame, &mut queue)?;
                break;
            }
            offset = next_offset;
            frame = next_frame;
        }
    }
    Ok(FrameAnalysis {
        states,
        reachable,
        control_targets,
        frame_offsets,
        dead_frames,
    })
}

fn merge_reachable_handlers(
    method: &MethodInfo,
    offset: usize,
    frame: &Frame,
    states: &mut [Option<Frame>],
    queue: &mut VecDeque<usize>,
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
        merge_state(states, exception.handler_pc, handler, queue)?;
    }
    Ok(())
}

fn merge_reachable_successors(
    successors: Vec<Successor>,
    states: &mut [Option<Frame>],
    queue: &mut VecDeque<usize>,
    frame_offsets: &mut BTreeSet<usize>,
) -> Result<(), JvmBackendError> {
    for successor in successors {
        frame_offsets.insert(successor.offset);
        merge_state(states, successor.offset, successor.frame, queue)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn start_dead_fallthrough(
    pool: &Rc<RefCell<ConstantPool>>,
    method: &MethodInfo,
    next_offset: usize,
    locals: Vec<VerificationType>,
    exception_handlers: &BTreeSet<usize>,
    control_targets: &BTreeSet<usize>,
    states: &mut [Option<Frame>],
    frame_offsets: &mut BTreeSet<usize>,
    dead_frames: &mut BTreeMap<usize, Frame>,
) -> Result<(), JvmBackendError> {
    if next_offset >= method.code.len()
        || states.get(next_offset).is_some_and(Option::is_some)
        || dead_frames.contains_key(&next_offset)
        || exception_handlers.contains(&next_offset)
    {
        return Ok(());
    }
    frame_offsets.insert(next_offset);
    analyze_dead_block(
        pool,
        method,
        next_offset,
        dead_frame(&method.code, next_offset, locals)?,
        control_targets,
        states,
        frame_offsets,
        dead_frames,
    )
}

fn complete_dead_frames(
    method: &MethodInfo,
    analysis: &FrameAnalysis,
) -> Result<BTreeMap<usize, Frame>, JvmBackendError> {
    let mut frame_map = initial_frame_map(analysis);
    let dead_facts = collect_dead_block_facts(method, analysis, &frame_map)?;
    loop {
        let snapshot = frame_map.clone();
        let mut next_map = snapshot.clone();
        complete_dead_block_locals(analysis, &dead_facts, &snapshot, &mut next_map)?;
        complete_dead_exception_locals(method, analysis, &snapshot, &mut next_map);
        if next_map == snapshot {
            return Ok(next_map);
        }
        frame_map = next_map;
    }
}

fn initial_frame_map(analysis: &FrameAnalysis) -> BTreeMap<usize, Frame> {
    let mut frame_map = BTreeMap::new();
    for &offset in &analysis.frame_offsets {
        let Some(frame) = analysis
            .states
            .get(offset)
            .and_then(Option::as_ref)
            .or_else(|| analysis.dead_frames.get(&offset))
        else {
            continue;
        };
        frame_map.insert(offset, frame.clone());
    }
    frame_map
}

fn collect_dead_block_facts(
    method: &MethodInfo,
    analysis: &FrameAnalysis,
    frame_map: &BTreeMap<usize, Frame>,
) -> Result<BTreeMap<usize, DeadBlockFacts>, JvmBackendError> {
    let mut dead_facts = BTreeMap::new();
    for &offset in frame_map.keys() {
        if !analysis.reachable.get(offset).copied().unwrap_or(false) {
            dead_facts.insert(
                offset,
                dead_block_facts(&method.code, offset, &analysis.control_targets)?,
            );
        }
    }
    Ok(dead_facts)
}

fn complete_dead_block_locals(
    analysis: &FrameAnalysis,
    dead_facts: &BTreeMap<usize, DeadBlockFacts>,
    snapshot: &BTreeMap<usize, Frame>,
    next_map: &mut BTreeMap<usize, Frame>,
) -> Result<(), JvmBackendError> {
    for (offset, frame) in next_map {
        if analysis.reachable.get(*offset).copied().unwrap_or(false) {
            continue;
        }
        let facts = dead_facts
            .get(offset)
            .ok_or_else(|| stack_map_error("missing dead block facts"))?;
        for (&slot, required) in &facts.required {
            if frame.locals[slot] == VerificationType::Top {
                frame.locals[slot] = required.clone();
            }
        }
        let Some(target_frame) = facts.target.and_then(|target| snapshot.get(&target)) else {
            continue;
        };
        for (slot, (local, target_local)) in frame
            .locals
            .iter_mut()
            .zip(&target_frame.locals)
            .enumerate()
        {
            if facts.written.contains(&slot) {
                continue;
            }
            if *local == VerificationType::Top && *target_local != VerificationType::Top {
                *local = target_local.clone();
            }
        }
    }
    Ok(())
}

fn complete_dead_exception_locals(
    method: &MethodInfo,
    analysis: &FrameAnalysis,
    snapshot: &BTreeMap<usize, Frame>,
    next_map: &mut BTreeMap<usize, Frame>,
) {
    for exception in &method.exceptions {
        if analysis
            .reachable
            .get(exception.start_pc)
            .copied()
            .unwrap_or(false)
        {
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
}

include!("stack_map/dead_blocks.rs");

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

include!("stack_map/instructions.rs");

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

include!("stack_map/serialization.rs");

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
