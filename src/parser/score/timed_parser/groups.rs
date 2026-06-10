#![allow(clippy::indexing_slicing)]

use crate::error::{JianPuError, Span};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GroupStack {
    pub frames: Vec<GroupFrame>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupFrame {
    pub note_count: usize,
    pub segment_start: usize,
}

impl GroupStack {
    pub fn is_open(&self) -> bool {
        !self.frames.is_empty()
    }

    pub fn push(&mut self, segment_start: usize) {
        self.frames.push(GroupFrame {
            note_count: 0,
            segment_start,
        });
    }

    pub fn pop(&mut self) -> Option<GroupFrame> {
        self.frames.pop()
    }

    pub fn increment_note_count(&mut self) {
        // Every open frame (at every nesting level) gets the count, because each
        // note belongs to all enclosing groups.
        for frame in self.frames.iter_mut() {
            frame.note_count += 1;
        }
    }
}

pub fn validate_group_note_count(count: usize, span: &Span) -> Result<(), JianPuError> {
    if count < 2 {
        return Err(JianPuError::new(
            span.clone(),
            "tie/slur group `(…)` must contain at least 2 notes".to_string(),
        ));
    }
    Ok(())
}

pub trait HasGroupDepth {
    fn group_membership(&self) -> u8;
    fn group_continuation(&self) -> u8;
    fn set_group_membership(&mut self, value: u8);
    fn set_group_continuation(&mut self, value: u8);
}

pub fn apply_closed_group_depth<T: HasGroupDepth>(atoms: &mut [T]) {
    let continuation_count = atoms.len().saturating_sub(1);
    for atom in atoms.iter_mut() {
        atom.set_group_membership(atom.group_membership().saturating_add(1));
    }
    for atom in atoms.iter_mut().take(continuation_count) {
        atom.set_group_continuation(atom.group_continuation().saturating_add(1));
    }
}

pub fn apply_open_group_depth<T: HasGroupDepth>(atoms: &mut [T]) {
    for atom in atoms.iter_mut() {
        atom.set_group_membership(atom.group_membership().saturating_add(1));
        atom.set_group_continuation(atom.group_continuation().saturating_add(1));
    }
}

pub fn apply_closing_segment_depth<T: HasGroupDepth>(atoms: &mut [T], group_still_open: bool) {
    for atom in atoms.iter_mut() {
        atom.set_group_membership(atom.group_membership().saturating_add(1));
    }
    let continuation_count = if group_still_open {
        atoms.len()
    } else {
        atoms.len().saturating_sub(1)
    };
    for atom in atoms.iter_mut().take(continuation_count) {
        atom.set_group_continuation(atom.group_continuation().saturating_add(1));
    }
}
