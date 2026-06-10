#![allow(clippy::indexing_slicing)]

mod chord_head;
mod directives;
mod duration;
mod groups;
mod note_head;
mod timed_lexer;

#[path = "timed_lexer_tests.rs"]
#[cfg(test)]
mod timed_lexer_tests;

pub use timed_lexer::{lex_line, TimedLexToken};

pub use chord_head::ChordHead;
pub use note_head::NoteHead;

pub use duration::{parse_duration_suffixes, DurationParse};
pub use groups::{
    apply_closed_group_depth, apply_closing_segment_depth, apply_open_group_depth,
    find_closing_paren, validate_group_note_count, GroupParseState, HasGroupDepth,
};

use crate::ast::parsed::ScoreEvent;
use crate::error::{JianPuError, Span, Spanned};
use crate::parser::score::tokenizer::RawToken;

pub trait TimedUnitHead: Sized {
    /// Parse one head starting at `chars[start]`. Returns (head, index after head, is_rest).
    fn parse_head(
        chars: &[char],
        start: usize,
        span: &Span,
    ) -> Result<(Self, usize, bool), JianPuError>;

    /// True when the next atom should start (note: next digit 0-7; chord: always after suffixes end).
    fn head_boundary(chars: &[char], i: usize) -> bool;

    fn allows_octave_suffixes() -> bool {
        true
    }

    fn to_event(
        head: &Self,
        duration: u32,
        dotted: bool,
        octave: i8,
        group_membership: u8,
        group_continuation: u8,
    ) -> ScoreEvent;
}

struct TimedAtom<H: TimedUnitHead> {
    head: H,
    duration: u32,
    octave: i8,
    dotted: bool,
    group_membership: u8,
    group_continuation: u8,
}

impl<H: TimedUnitHead> HasGroupDepth for TimedAtom<H> {
    fn group_membership(&self) -> u8 {
        self.group_membership
    }

    fn group_continuation(&self) -> u8 {
        self.group_continuation
    }

    fn set_group_membership(&mut self, value: u8) {
        self.group_membership = value;
    }

    fn set_group_continuation(&mut self, value: u8) {
        self.group_continuation = value;
    }
}

pub fn parse_timed_tokens<H: TimedUnitHead>(
    tokens: Vec<RawToken>,
    group_state: &mut GroupParseState,
) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
    let mut events = Vec::new();

    for token in tokens {
        let span = Span::new(token.offset, token.offset + token.text.len());
        let parsed = parse_timed_token::<H>(&token.text, span.clone(), group_state)?;
        for event in parsed {
            events.push(Spanned::new(event, span.clone()));
        }
    }

    Ok(events)
}

pub fn parse_timed_token<H: TimedUnitHead>(
    text: &str,
    span: Span,
    group_state: &mut GroupParseState,
) -> Result<Vec<ScoreEvent>, JianPuError> {
    if text.is_empty() {
        return Err(JianPuError::new(span, "empty timed token".to_string()));
    }

    let mut events = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();

    if group_state.open {
        if text.contains(')') {
            i = parse_closing_group_segment::<H>(&mut events, &chars, text, i, &span, group_state)?;
        } else {
            let added = parse_open_group_continuation::<H>(&mut events, &chars, text, i, &span)?;
            group_state.open_note_count += added;
            return Ok(events);
        }
    }

    while i < chars.len() {
        if chars[i] == '(' {
            let inner_start = i + 1;
            if let Some(inner_end) = find_closing_paren(&chars, inner_start) {
                let inner: String = chars[inner_start..inner_end].iter().collect();
                events.extend(parse_closed_group::<H>(&inner, &span)?);
                i = inner_end + 1;
            } else {
                let inner: String = chars[inner_start..].iter().collect();
                let group_events = parse_open_group::<H>(&inner, &span)?;
                group_state.open_note_count = group_events.len();
                events.extend(group_events);
                group_state.open = true;
                break;
            }
            continue;
        }

        let (head, head_end, is_rest) = H::parse_head(&chars, i, &span)?;
        let (atom, next_i) = parse_one_atom::<H>(head, head_end, is_rest, &chars, i, &span)?;
        events.push(atom_to_event(&atom));
        i = next_i;
    }

    Ok(events)
}

fn parse_closing_group_segment<H: TimedUnitHead>(
    events: &mut Vec<ScoreEvent>,
    chars: &[char],
    text: &str,
    mut i: usize,
    span: &Span,
    group_state: &mut GroupParseState,
) -> Result<usize, JianPuError> {
    // Spaced groups may end with a token like "-)" where leading dashes are extensions.
    while i < chars.len() && chars[i] == '-' {
        let next_is_close = i + 1 < chars.len() && chars[i + 1] == ')';
        let next_is_head = i + 1 < chars.len() && H::head_boundary(chars, i + 1);
        if !next_is_close && next_is_head {
            break;
        }
        events.push(ScoreEvent::Extension);
        i += 1;
    }

    let (mut atoms, next_i) = parse_atoms_from_chars::<H>(chars, text, span, i, true)?;
    i = next_i;
    let closes_group = i < chars.len() && chars[i] == ')';
    apply_closing_segment_depth(&mut atoms, !closes_group);
    let atom_count = atoms.len();
    events.extend(atoms.iter().map(atom_to_event));

    if closes_group {
        validate_group_note_count(group_state.open_note_count + atom_count, span)?;
        group_state.open = false;
        group_state.open_note_count = 0;
        i += 1;
    } else {
        group_state.open_note_count += atom_count;
        group_state.open = true;
    }

    Ok(i)
}

fn parse_open_group_continuation<H: TimedUnitHead>(
    events: &mut Vec<ScoreEvent>,
    chars: &[char],
    text: &str,
    i: usize,
    span: &Span,
) -> Result<usize, JianPuError> {
    let (mut atoms, _) = parse_atoms_from_chars::<H>(chars, text, span, i, false)?;
    apply_open_group_depth(&mut atoms);
    let added = atoms.len();
    events.extend(atoms.iter().map(atom_to_event));

    Ok(added)
}

fn parse_atoms_from_text<H: TimedUnitHead>(
    text: &str,
    span: &Span,
) -> Result<Vec<TimedAtom<H>>, JianPuError> {
    let chars: Vec<char> = text.chars().collect();
    let (atoms, _) = parse_atoms_from_chars::<H>(&chars, text, span, 0, false)?;
    Ok(atoms)
}

fn parse_atoms_from_chars<H: TimedUnitHead>(
    chars: &[char],
    _text: &str,
    span: &Span,
    mut i: usize,
    stop_at_close: bool,
) -> Result<(Vec<TimedAtom<H>>, usize), JianPuError> {
    let mut atoms = Vec::new();

    while i < chars.len() {
        if stop_at_close && chars[i] == ')' {
            return Ok((atoms, i));
        }

        if chars[i] == '(' {
            let inner_start = i + 1;
            let inner_end = find_closing_paren(chars, inner_start)
                .ok_or_else(|| JianPuError::new(span.clone(), "unclosed '(' group".to_string()))?;
            let inner: String = chars[inner_start..inner_end].iter().collect();
            let mut inner_atoms = parse_atoms_from_text::<H>(&inner, span)?;
            validate_group_note_count(inner_atoms.len(), span)?;
            apply_closed_group_depth(&mut inner_atoms);
            atoms.extend(inner_atoms);
            i = inner_end + 1;
            continue;
        }

        let (head, head_end, is_rest) = H::parse_head(chars, i, span)?;
        let (atom, next_i) = parse_one_atom::<H>(head, head_end, is_rest, chars, i, span)?;
        atoms.push(atom);
        i = next_i;
    }

    Ok((atoms, i))
}

fn parse_closed_group<H: TimedUnitHead>(
    inner: &str,
    span: &Span,
) -> Result<Vec<ScoreEvent>, JianPuError> {
    let mut atoms = parse_atoms_from_text::<H>(inner, span)?;
    validate_group_note_count(atoms.len(), span)?;
    apply_closed_group_depth(&mut atoms);
    Ok(atoms.iter().map(atom_to_event).collect())
}

fn parse_open_group<H: TimedUnitHead>(
    inner: &str,
    span: &Span,
) -> Result<Vec<ScoreEvent>, JianPuError> {
    let mut atoms = parse_atoms_from_text::<H>(inner, span)?;
    apply_open_group_depth(&mut atoms);
    Ok(atoms.iter().map(atom_to_event).collect())
}

fn parse_one_atom<H: TimedUnitHead>(
    head: H,
    head_end: usize,
    is_rest: bool,
    chars: &[char],
    start: usize,
    span: &Span,
) -> Result<(TimedAtom<H>, usize), JianPuError> {
    let duration_meta = parse_duration_suffixes::<H>(chars, start, head_end, is_rest, span)?;

    let octave = if duration_meta.octave_up > 0 {
        duration_meta.octave_up
    } else {
        -duration_meta.octave_down
    };

    Ok((
        TimedAtom {
            head,
            duration: duration_meta.duration,
            octave,
            dotted: duration_meta.dotted,
            group_membership: 0,
            group_continuation: 0,
        },
        duration_meta.next_index,
    ))
}

fn atom_to_event<H: TimedUnitHead>(atom: &TimedAtom<H>) -> ScoreEvent {
    H::to_event(
        &atom.head,
        atom.duration,
        atom.dotted,
        atom.octave,
        atom.group_membership,
        atom.group_continuation,
    )
}

pub fn byte_offset_at_char_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(b, _)| b)
        .unwrap_or(text.len())
}
