//! Definition preparse control metadata, following wc.c::conall/conend.
//! Construction never executes a body or decides compiler optimization legality.
use crate::{Error, Result, definition_control::ControlWord as W};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKind {
    Body,
    Test,
    Assert,
    Word(W),
    DoFor,
    BreakFor,
    DoSelect,
    EndSelect,
    SelectNested,
    BreakSelect,
    ContinueSelect,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlJump {
    Index(usize),
    DynamicError,
    Return,
}
/// C's canend status of the previous B-block result, not purity or CFG reachability.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PreviousResult {
    #[default]
    Unresolved,
    CanReturn,
    CannotReturn,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlNode {
    /// Body-relative bytes; word range indexes the corresponding physical line.
    pub span: Range<usize>,
    pub line: usize,
    pub words: Range<usize>,
    pub kind: ControlKind,
    pub go: ControlJump,
    pub assertion: Option<Range<usize>>,
    /// C accepts some packed-code end pairs without the canonical do. structure.
    pub analysis_barrier: bool,
    /// CBBLOCKEND: followed by a non-select end that falls through, with more code.
    pub before_fallthrough_end: bool,
    pub previous_result: PreviousResult,
    /// Raw goto/label suffix, excluding the terminating dot; never name lookup.
    pub named_target: Option<String>,
}

fn invalid(nodes: &[ControlNode], index: usize) -> Error {
    Error::Control.at(nodes[index].span.clone())
}

/// Validate the supported subset and install per-valence control targets.
/// go is C's control/error target, not the sole successful successor of a block.
pub fn audit(nodes: &mut [ControlNode]) -> Result<()> {
    use ControlJump::*;
    use ControlKind::*;
    audit_goto(nodes)?;
    let mut stack: Vec<usize> = Vec::new();
    let mut tests = 0usize;
    let mut loops = 0usize;
    for i in 0..nodes.len() {
        let previous = stack.last().copied();
        let before = stack.iter().rev().nth(1).copied();
        let q = previous.map(|j| nodes[j].kind);
        let r = before.map(|j| nodes[j].kind);
        let next = i + 1;
        match nodes[i].kind {
            Body => {
                if tests > 0 {
                    nodes[i].kind = Test;
                }
            }
            Assert => {}
            Word(W::If | W::While | W::Whilst | W::For) => {
                if nodes[i].kind != Word(W::If) {
                    loops += 1;
                }
                stack.push(i);
                tests += 1;
            }
            Word(W::Select) => {
                if loops > 0
                    && stack
                        .iter()
                        .rev()
                        .take_while(|j| {
                            !matches!(nodes[**j].kind, Word(W::For | W::While | W::Whilst))
                        })
                        .any(|j| nodes[*j].kind == Word(W::Select))
                {
                    nodes[i].kind = SelectNested;
                }
                stack.push(i);
                tests += 1;
            }
            Word(W::Case | W::FCase) => {
                if !matches!(
                    q,
                    Some(Word(W::Select)) | Some(SelectNested) | Some(DoSelect)
                ) {
                    return Err(invalid(nodes, i));
                }
                if q == Some(DoSelect) {
                    tests += 1;
                }
                stack.push(i);
            }
            Word(W::Do) => {
                if !matches!(
                    q,
                    Some(Word(
                        W::If
                            | W::ElseIf
                            | W::While
                            | W::Whilst
                            | W::For
                            | W::Case
                            | W::FCase
                            | W::Select
                    ))
                ) {
                    return Err(invalid(nodes, i));
                }
                if q == Some(Word(W::For)) {
                    nodes[i].kind = DoFor;
                }
                if matches!(q, Some(Word(W::Case | W::FCase))) {
                    nodes[i].kind = DoSelect;
                }
                stack.push(i);
                tests -= 1;
            }
            Word(W::ElseIf | W::Else) => {
                if q != Some(Word(W::Do)) || !matches!(r, Some(Word(W::If | W::ElseIf))) {
                    return Err(invalid(nodes, i));
                }
                let j = previous.unwrap();
                let k = before.unwrap();
                nodes[j].go = Index(next);
                nodes[i].go = Index(if r == Some(Word(W::If)) { 0 } else { k });
                if nodes[i].kind == Word(W::ElseIf) {
                    stack.truncate(stack.len() - 2);
                    stack.push(i);
                    tests += 1;
                } else {
                    let size = stack.len();
                    stack[size - 2] = j;
                    stack[size - 1] = i;
                }
            }
            Word(W::Try | W::Catch | W::CatchD | W::CatchT) => {
                stack.push(i);
            }
            Word(W::End) => {
                if q == Some(DoSelect) {
                    close_select(nodes, &mut stack, i)?;
                    nodes[i].kind = EndSelect;
                    continue;
                }

                if matches!(q, Some(Word(W::Catch | W::CatchD | W::CatchT))) {
                    close_try(nodes, &mut stack, i)?;
                    mark_body_end(nodes, i);
                    continue;
                }
                let (Some(j), Some(k)) = (previous, before) else {
                    return Err(invalid(nodes, i));
                };
                if matches!((r, q), (Some(Word(W::If)), Some(Word(W::Do)))) {
                } else if matches!(
                    (r, q),
                    (Some(Word(W::ElseIf)), Some(Word(W::Do)))
                        | (Some(Word(W::Do)), Some(Word(W::Else)))
                ) {
                    let mut chain = if q == Some(Word(W::Else)) { j } else { k };
                    loop {
                        let Index(back) = nodes[chain].go else {
                            return Err(invalid(nodes, i));
                        };
                        nodes[chain].go = Index(next);
                        if back == 0 {
                            break;
                        }
                        chain = back;
                    }
                } else if packed_loop_end(r, q) {
                    nodes[i].analysis_barrier = !matches!(
                        (r, q),
                        (Some(Word(W::While | W::Whilst)), Some(Word(W::Do)))
                    );
                    if r == Some(Word(W::Whilst)) {
                        nodes[k].go = Index(j + 1);
                    }
                    nodes[i].go = Index(k + 1);
                    if matches!(r, Some(Word(W::While | W::Whilst))) {
                        loops -= 1;
                    }
                    for node in &mut nodes[k + 1..i] {
                        if node.go == DynamicError {
                            if matches!(node.kind, Word(W::Break) | BreakSelect) {
                                node.go = Index(next);
                            } else if matches!(node.kind, Word(W::Continue) | ContinueSelect) {
                                node.go = Index(k + 1);
                            }
                        }
                    }
                } else if r == Some(Word(W::For)) && q == Some(DoFor) {
                    nodes[i].go = Index(j);
                    nodes[k].go = Index(i);
                    loops -= 1;
                    for node in &mut nodes[k + 1..i] {
                        if node.go == DynamicError {
                            if matches!(node.kind, Word(W::Break) | BreakSelect) {
                                node.kind = BreakFor;
                                node.go = Index(next);
                            } else if matches!(node.kind, Word(W::Continue) | ContinueSelect) {
                                node.go = Index(j);
                            }
                        }
                    }
                } else {
                    return Err(invalid(nodes, i));
                }
                nodes[j].go = Index(next);
                stack.truncate(stack.len() - 2);
                mark_body_end(nodes, i);
            }
            Word(W::Break | W::Continue) => {
                if loops == 0 {
                    return Err(invalid(nodes, i));
                }
            }
            Word(W::Return | W::Throw | W::Goto | W::Label) => {}
            Word(W::Assert) | Test | DoFor | BreakFor | DoSelect | EndSelect | SelectNested
            | BreakSelect | ContinueSelect => {
                return Err(Error::Unsupported(
                    "control audit requires unprocessed nodes".into(),
                ));
            }
        }
    }
    if let Some(i) = stack.last() {
        return Err(invalid(nodes, *i));
    }
    fill_previous_result(nodes)
}

fn control_tag(kind: ControlKind) -> u16 {
    use ControlKind::*;
    match kind {
        Body => 1,
        Test => 2,
        Assert => 25,
        DoFor => 18,
        BreakFor => 19,
        DoSelect => 24,
        EndSelect => 26,
        SelectNested => 21,
        BreakSelect => 30,
        ContinueSelect => 31,
        Word(word) => match word {
            W::Do => 3,
            W::If => 4,
            W::While => 5,
            W::End => 6,
            W::Else => 7,
            W::Whilst => 8,
            W::ElseIf => 9,
            W::Try => 10,
            W::Catch => 11,
            W::Break => 12,
            W::Continue => 13,
            W::Label => 14,
            W::Goto => 15,
            W::Return => 16,
            W::For => 17,
            W::Select => 20,
            W::Case => 22,
            W::FCase => 23,
            W::Assert => 25,
            W::Throw => 27,
            W::CatchD => 28,
            W::CatchT => 29,
        },
    }
}
fn packed_loop_end(previous: Option<ControlKind>, current: Option<ControlKind>) -> bool {
    let (Some(r), Some(q)) = (previous, current) else {
        return false;
    };
    let pair = (control_tag(r) << 8) + control_tag(q);
    // wc.c BETWEENC(BOTHASUS(r,q), BOTHASUS(CWHILE,CDO), BOTHASUS(CWHILST,CDO)).
    ((5 << 8) + 3..=(8 << 8) + 3).contains(&pair)
}

fn close_select(nodes: &mut [ControlNode], stack: &mut Vec<usize>, end: usize) -> Result<()> {
    use ControlJump::*;
    use ControlKind::*;
    let original_stack = stack.clone();
    let original_top = stack.len();
    let mut case = end - 1;
    let mut do_index = None;
    let start = loop {
        let Some(index) = stack.pop() else {
            return Err(invalid(nodes, end));
        };
        if matches!(nodes[index].kind, Word(W::Select) | SelectNested) {
            break index;
        }
        if nodes[index].kind == DoSelect {
            do_index = Some(index);
            nodes[index].go = Index(case + 1);
        } else if matches!(nodes[index].kind, Word(W::Case | W::FCase)) {
            case = index;
            nodes[index].go = Index(end);
            if do_index == Some(index + 1) {
                nodes[index + 1].go = Index(index + 2);
            }
            if nodes[index].kind == Word(W::FCase) && stack.len() < original_top - 2 {
                let following = original_stack[stack.len() + 2];
                let following_do = original_stack[stack.len() + 3];
                nodes[following].go = Index(following_do + 1);
            }
        } else {
            return Err(invalid(nodes, end));
        }
    };
    nodes[case].go = Index(case + 1);
    nodes[start].go = Index(end);
    // Match conendsel's half-open scan; the final pre-end entry is excluded.
    for node in &mut nodes[start + 1..end.saturating_sub(1)] {
        if node.go == DynamicError {
            if node.kind == Word(W::Break) {
                node.kind = BreakSelect;
            } else if node.kind == Word(W::Continue) {
                node.kind = ContinueSelect;
            }
        }
    }
    Ok(())
}

fn close_try(nodes: &mut [ControlNode], stack: &mut Vec<usize>, end: usize) -> Result<()> {
    use ControlJump::*;
    use ControlKind::{BreakSelect, ContinueSelect, Word};
    let mut catches: [Option<usize>; 3] = [None; 3];
    let mut chain = vec![end];
    let start = loop {
        let Some(index) = stack.pop() else {
            return Err(invalid(nodes, end));
        };
        let slot = match nodes[index].kind {
            Word(W::Try) => break index,
            Word(W::Catch) => 0,
            Word(W::CatchD) => 1,
            Word(W::CatchT) => 2,
            _ => return Err(invalid(nodes, end)),
        };
        if catches[slot].replace(index).is_some() {
            return Err(invalid(nodes, end));
        }
        chain.push(index);
    };
    if chain.len() == 1 {
        return Err(invalid(nodes, end));
    }
    let first = *chain.last().unwrap();
    nodes[start].go = Index(first);
    for pair in chain.windows(2) {
        nodes[pair[1]].go = Index(pair[0]);
    }
    if let Some(handler) = catches[0].or(catches[1]) {
        for node in &mut nodes[start + 1..first] {
            if node.go == DynamicError
                && !matches!(
                    node.kind,
                    Word(W::Break | W::Continue) | BreakSelect | ContinueSelect
                )
            {
                node.go = Index(handler + 1);
            }
        }
    }
    Ok(())
}

pub fn verify(nodes: &[ControlNode]) -> Result<()> {
    if nodes
        .iter()
        .any(|node| matches!(node.go, ControlJump::Index(target) if target>nodes.len()))
    {
        return Err(Error::Unsupported("control target outside valence".into()));
    }
    for (i, node) in nodes.iter().enumerate() {
        let named = matches!(node.kind, ControlKind::Word(W::Goto | W::Label));
        if named != node.named_target.is_some() {
            return Err(Error::Unsupported(
                "missing or unexpected named control data".into(),
            ));
        }
        if node.kind == ControlKind::Word(W::Goto) {
            let label = match node.go {
                ControlJump::Index(target) => target.checked_sub(1).and_then(|i| nodes.get(i)),
                _ => None,
            };
            if label.is_none_or(|label| {
                label.kind != ControlKind::Word(W::Label) || label.named_target != node.named_target
            }) {
                return Err(Error::Unsupported(
                    "goto does not target its label successor".into(),
                ));
            }
        }
        if node.before_fallthrough_end
            && (node.kind != ControlKind::Body
                || i + 2 >= nodes.len()
                || nodes[i + 1].kind != ControlKind::Word(W::End)
                || nodes[i + 1].go != ControlJump::Index(i + 2))
        {
            return Err(Error::Unsupported(
                "invalid fallthrough-end metadata".into(),
            ));
        }
    }
    Ok(())
}

fn mark_body_end(nodes: &mut [ControlNode], end: usize) {
    if end > 0
        && end + 1 < nodes.len()
        && nodes[end].go == ControlJump::Index(end + 1)
        && nodes[end - 1].kind == ControlKind::Body
    {
        nodes[end - 1].before_fallthrough_end = true;
    }
}

/// Record wc.c::conall's reverse fixed point without executing or optimizing code.
/// Mixed successor outcomes or provisional cycles remain Unresolved.
fn fill_previous_result(nodes: &mut [ControlNode]) -> Result<()> {
    use ControlKind::*;
    verify(nodes)?;
    let n = nodes.len();
    let mut state = vec![0u8; n];
    loop {
        let mut changed = false;
        for i in (0..n).rev() {
            let old = state[i];
            if old & 3 != 0 {
                continue;
            }
            let next = state.get(i + 1).copied().unwrap_or(5);
            let target = match nodes[i].go {
                ControlJump::Index(target) => target,
                ControlJump::DynamicError | ControlJump::Return => n,
            };
            let jump = state.get(target).copied().unwrap_or(5);
            state[i] = match nodes[i].kind {
                Body | Word(W::Throw) => 10,
                Word(W::Return) => 5,
                Word(W::For | W::Select | W::Label) | SelectNested => next,
                Test | Assert => {
                    if i + 1 == n || target >= n {
                        next
                    } else {
                        jump & next
                    }
                }
                Word(W::Try | W::Catch | W::CatchD | W::CatchT | W::Do) | DoFor | DoSelect => {
                    if i + 1 == n || target >= n {
                        5
                    } else {
                        jump & next
                    }
                }
                _ => {
                    if target >= n {
                        5
                    } else if target >= i {
                        jump
                    } else if i + 1 == n {
                        5
                    } else {
                        (next & 12) | ((next & jump) >> 2)
                    }
                }
            };
            changed |= old != state[i];
        }
        if !changed {
            break;
        }
    }
    for (node, value) in nodes.iter_mut().zip(state) {
        node.previous_result = match value & 3 {
            1 => PreviousResult::CanReturn,
            2 => PreviousResult::CannotReturn,
            _ => PreviousResult::Unresolved,
        };
    }
    Ok(())
}

/// congoto runs before conall, so these are the original (unspecialized) kinds.
fn audit_goto(nodes: &mut [ControlNode]) -> Result<()> {
    use ControlKind::Word;
    if !nodes.iter().any(|n| n.kind == Word(W::Goto)) {
        return Ok(()); // Unreferenced duplicate labels are legal in C.
    }
    let mut intervals: Vec<(usize, Option<usize>)> = Vec::new();
    let mut cursor = 0usize;
    for (i, node) in nodes.iter().enumerate() {
        match node.kind {
            Word(W::End) => {
                let Some(interval) = intervals.get_mut(cursor) else {
                    return Err(invalid(nodes, i));
                };
                interval.1 = Some(i);
                while cursor > 0 && intervals[cursor].1.is_some_and(|end| end > 0) {
                    cursor -= 1;
                }
            }
            Word(W::Case | W::Catch | W::Do | W::Else | W::ElseIf | W::FCase) => {
                let Some(interval) = intervals.get_mut(cursor) else {
                    return Err(invalid(nodes, i));
                };
                interval.1 = Some(i);
                intervals.push((i, None));
                cursor = intervals.len() - 1;
            }
            Word(W::For | W::If | W::Select | W::Try | W::While | W::Whilst) => {
                intervals.push((i, None));
                cursor = intervals.len() - 1;
            }
            _ => {}
        }
    }
    for i in 0..nodes.len() {
        if nodes[i].kind != Word(W::Goto) {
            continue;
        }
        let name = nodes[i]
            .named_target
            .as_ref()
            .ok_or_else(|| invalid(nodes, i))?;
        let mut labels = nodes.iter().enumerate().filter(|(_, node)| {
            node.kind == Word(W::Label) && node.named_target.as_ref() == Some(name)
        });
        let Some((target, _)) = labels.next() else {
            return Err(invalid(nodes, i));
        };
        if let Some((duplicate, _)) = labels.next() {
            return Err(invalid(nodes, duplicate));
        }
        let inside = |point: usize, start: usize, end: Option<usize>| {
            point >= start && end.is_none_or(|end| point < end)
        };
        if intervals
            .iter()
            .any(|&(start, end)| inside(target, start, end) && !inside(i, start, end))
        {
            return Err(invalid(nodes, i));
        }
        nodes[i].go = ControlJump::Index(target + 1);
    }
    Ok(())
}
