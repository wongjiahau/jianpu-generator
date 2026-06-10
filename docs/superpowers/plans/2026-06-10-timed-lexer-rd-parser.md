# Timed Lexer + RD Parser Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace whitespace tokenization and hybrid group parsing with a char-by-char typed lexer and recursive descent parser so spaced nested groups like `((1 1) 5 5)` parse correctly and `(…)` groups can span bar lines at any nesting depth.

**Architecture:** `timed_lexer.rs` scans a notes/chord line into `Vec<Spanned<TimedLexToken>>`. `timed_rd_parser.rs` consumes that stream recursively, reusing `NoteHead`, `ChordHead`, and `parse_duration_suffixes` for timed units. `GroupStack` replaces `GroupParseState` and persists per track across bars in `interleaved_parser`. Delete old `parse_timed_token` / whitespace `RawToken` path.

**Tech Stack:** Rust (`cargo test`), existing `TimedUnitHead` trait, `syntax.md` update, WASM rebuild after: `cd web && pnpm run build:wasm`.

**Spec:** `docs/superpowers/specs/2026-06-10-timed-lexer-rd-parser-design.md`

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `src/parser/score/timed_parser/timed_lexer.rs` | **Create** | Char scan → `TimedLexToken` |
| `src/parser/score/timed_parser/timed_lexer_tests.rs` | **Create** | Lexer unit tests (via `#[path]`) |
| `src/parser/score/timed_parser/timed_rd_parser.rs` | **Create** | Recursive descent parser |
| `src/parser/score/timed_parser/timed_rd_parser_tests.rs` | **Create** | RD parser unit tests |
| `src/parser/score/timed_parser/groups.rs` | **Modify** | `GroupStack`, remove `GroupParseState` / `find_closing_paren` |
| `src/parser/score/timed_parser/mod.rs` | **Modify** | Export `parse_timed_line`, remove old engine |
| `src/parser/score/token_parser.rs` | **Modify** | Thin wrappers `parse_notes_line` / `parse_chord_line` |
| `src/parser/score/interleaved_parser.rs` | **Modify** | `GroupStack`, call new API |
| `src/parser/score/tokenizer.rs` | **Delete** | No longer used |
| `src/parser/score/mod.rs` | **Modify** | Remove `tokenizer` module |
| `src/grouping.rs` | **Modify** | Test call sites |
| `src/layout/mod.rs` | **Modify** | Test helper call sites |
| `src/parser/score/timed_parser/chord_head.rs` | **Modify** | Test helpers |
| `syntax.md` | **Modify** | Whitespace semantics |

---

### Task 1: `TimedLexToken` type + lexer skeleton

**Files:**
- Create: `src/parser/score/timed_parser/timed_lexer.rs`
- Modify: `src/parser/score/timed_parser/mod.rs`

- [ ] **Step 1: Add `timed_lexer.rs` with token enum**

```rust
#![allow(clippy::indexing_slicing)]

use crate::ast::parsed::{Accidental, KeyChange, Note, NoteName, ScoreEvent};
use crate::error::{JianPuError, Span, Spanned};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimedLexToken {
    LParen,
    RParen,
    Extension,
    HeadStart { offset: usize },
    Bpm(u32),
    KeyChange(KeyChange),
    TimeSignature { num: u8, den: u8 },
}

pub fn lex_line(line: &str, base_offset: usize) -> Result<Vec<Spanned<TimedLexToken>>, JianPuError> {
    let mut tokens = Vec::new();
    let mut atom_boundary = true;
    let mut i = 0;
    let bytes = line.as_bytes();

    while i < bytes.len() {
        let (c, len) = match line[i..].chars().next() {
            Some(ch) => (ch, ch.len_utf8()),
            None => break,
        };

        if c.is_whitespace() || c == '|' {
            i += len;
            atom_boundary = true;
            continue;
        }

        let start = base_offset + i;

        match c {
            '(' => {
                tokens.push(Spanned::new(TimedLexToken::LParen, Span::new(start, start + len)));
                atom_boundary = true;
                i += len;
            }
            ')' => {
                tokens.push(Spanned::new(TimedLexToken::RParen, Span::new(start, start + len)));
                atom_boundary = true;
                i += len;
            }
            '-' if atom_boundary => {
                tokens.push(Spanned::new(TimedLexToken::Extension, Span::new(start, start + len)));
                atom_boundary = true;
                i += len;
            }
            '0'..='7' => {
                tokens.push(Spanned::new(
                    TimedLexToken::HeadStart { offset: start },
                    Span::new(start, start + len),
                ));
                atom_boundary = false;
                i += len;
            }
            'b' if line[i..].starts_with("bpm=") => {
                let (token, consumed) = lex_bpm(line, i, start)?;
                tokens.push(token);
                atom_boundary = true;
                i += consumed;
            }
            '1' if line[i..].starts_with("1=") => {
                if let Some((token, consumed)) = try_lex_key_change(line, i, start)? {
                    tokens.push(token);
                    atom_boundary = true;
                    i += consumed;
                    continue;
                }
                tokens.push(Spanned::new(
                    TimedLexToken::HeadStart { offset: start },
                    Span::new(start, start + len),
                ));
                atom_boundary = false;
                i += len;
            }
            _ if c.is_ascii_digit() => {
                if let Some((token, consumed)) = try_lex_time_signature(line, i, start)? {
                    tokens.push(token);
                    atom_boundary = true;
                    i += consumed;
                    continue;
                }
                let pos = start;
                return Err(JianPuError::new(
                    Span::new(pos, pos + len),
                    format!("unexpected character: {c}"),
                ));
            }
            _ => {
                return Err(JianPuError::new(
                    Span::new(start, start + len),
                    format!("unexpected character: {c}"),
                ));
            }
        }
    }

    Ok(tokens)
}

fn lex_bpm(line: &str, i: usize, start: usize) -> Result<(Spanned<TimedLexToken>, usize), JianPuError> {
    let rest = &line[i + 4..];
    let end = rest
        .find(|c: char| c.is_whitespace() || c == '|' || c == '(' || c == ')')
        .unwrap_or(rest.len());
    let digits = &rest[..end];
    let bpm = digits.parse::<u32>().map_err(|_| {
        JianPuError::new(
            Span::new(start, start + 4 + end),
            format!("invalid bpm value: {digits}"),
        )
    })?;
    Ok((
        Spanned::new(
            TimedLexToken::Bpm(bpm),
            Span::new(start, start + 4 + end),
        ),
        4 + end,
    ))
}

fn try_lex_key_change(
    line: &str,
    i: usize,
    start: usize,
) -> Result<Option<(Spanned<TimedLexToken>, usize)>, JianPuError> {
    let after_eq = &line[i + 2..];
    let first = match after_eq.chars().next() {
        Some(c) if matches!(c, 'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'G') => c,
        _ => return Ok(None),
    };
    // Reuse token_parser key parsing logic — extract to shared fn or duplicate minimal parser here.
    let consumed = 2 + key_change_lexeme_len(after_eq);
    let text = &line[i..i + consumed];
    let key = parse_key_change_text(text, &Span::new(start, start + consumed))?;
    Ok(Some((
        Spanned::new(TimedLexToken::KeyChange(key), Span::new(start, start + consumed)),
        consumed,
    )))
}

fn try_lex_time_signature(
    line: &str,
    i: usize,
    start: usize,
) -> Result<Option<(Spanned<TimedLexToken>, usize)>, JianPuError> {
    let slice = &line[i..];
    let Some(slash) = slice.find('/') else {
        return Ok(None);
    };
    let num_str = &slice[..slash];
    if !num_str.chars().all(|c| c.is_ascii_digit()) || num_str.is_empty() {
        return Ok(None);
    }
    let after_slash = &slice[slash + 1..];
    let den_len = after_slash
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .map(|c| c.len_utf8())
        .sum::<usize>();
    if den_len == 0 {
        return Ok(None);
    }
    let num = num_str.parse::<u8>().map_err(|_| {
        JianPuError::new(Span::new(start, start + slash), format!("invalid time signature: {slice}"))
    })?;
    let den_str = &after_slash[..den_len];
    let den = den_str.parse::<u8>().map_err(|_| {
        JianPuError::new(Span::new(start, start + slash + 1 + den_len), format!("invalid time signature: {slice}"))
    })?;
    if den == 0 {
        return Err(JianPuError::new(
            Span::new(start, start + slash + 1 + den_len),
            "time signature denominator cannot be zero".to_string(),
        ));
    }
    let consumed = slash + 1 + den_len;
    Ok(Some((
        Spanned::new(
            TimedLexToken::TimeSignature { num, den },
            Span::new(start, start + consumed),
        ),
        consumed,
    )))
}
```

Add helper stubs `key_change_lexeme_len`, `parse_key_change_text` — implement by extracting `parse_key_change` from `token_parser.rs` into `timed_parser/directives.rs` (small shared module) in Step 2.

- [ ] **Step 2: Wire module in `mod.rs`**

```rust
mod timed_lexer;
pub use timed_lexer::{lex_line, TimedLexToken};
```

- [ ] **Step 3: Run `cargo check`**

Run: `cargo check 2>&1 | head -20`
Expected: compiles after helper extraction / stubs filled in.

- [ ] **Step 4: Commit**

```bash
git add src/parser/score/timed_parser/timed_lexer.rs src/parser/score/timed_parser/mod.rs src/parser/score/timed_parser/directives.rs src/parser/score/token_parser.rs
git commit -m "feat: add timed lexer skeleton and directive helpers"
```

---

### Task 2: Lexer tests

**Files:**
- Create: `src/parser/score/timed_parser/timed_lexer_tests.rs`
- Modify: `src/parser/score/timed_parser/mod.rs`

- [ ] **Step 1: Add test module**

In `mod.rs`:

```rust
#[path = "timed_lexer_tests.rs"]
#[cfg(test)]
mod timed_lexer_tests;
```

- [ ] **Step 2: Write lexer tests**

```rust
use super::timed_lexer::{lex_line, TimedLexToken};
use crate::error::Spanned;

fn kinds(line: &str) -> Vec<TimedLexToken> {
    lex_line(line, 0)
        .unwrap()
        .into_iter()
        .map(|t| t.value)
        .collect()
}

#[test]
fn skips_whitespace_and_bar_lines() {
    assert_eq!(
        kinds("1 2 | 3"),
        vec![
            TimedLexToken::HeadStart { offset: 0 },
            TimedLexToken::HeadStart { offset: 2 },
            TimedLexToken::HeadStart { offset: 6 },
        ]
    );
}

#[test]
fn lexes_spaced_nested_groups() {
    assert_eq!(
        kinds("((1 1) 5 5)"),
        vec![
            TimedLexToken::LParen,
            TimedLexToken::LParen,
            TimedLexToken::HeadStart { offset: 2 },
            TimedLexToken::HeadStart { offset: 4 },
            TimedLexToken::RParen,
            TimedLexToken::HeadStart { offset: 8 },
            TimedLexToken::HeadStart { offset: 10 },
            TimedLexToken::RParen,
        ]
    );
}

#[test]
fn extension_vs_suffix_dash() {
    assert_eq!(kinds("2---"), vec![TimedLexToken::HeadStart { offset: 0 }]);
    assert_eq!(
        kinds("2 - - -"),
        vec![
            TimedLexToken::HeadStart { offset: 0 },
            TimedLexToken::Extension,
            TimedLexToken::Extension,
            TimedLexToken::Extension,
        ]
    );
}

#[test]
fn sixteenth_note_not_key_change() {
    assert_eq!(kinds("1=,"), vec![TimedLexToken::HeadStart { offset: 0 }]);
}

#[test]
fn lexes_directives() {
    use TimedLexToken::*;
    let tokens = kinds("bpm=120 4/4");
    assert!(matches!(tokens[0], Bpm(120)));
    assert!(matches!(tokens[1], TimeSignature { num: 4, den: 4 }));
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --lib timed_lexer_tests -- --nocapture`
Expected: all PASS

- [ ] **Step 4: Commit**

```bash
git add src/parser/score/timed_parser/timed_lexer_tests.rs src/parser/score/timed_parser/mod.rs
git commit -m "test: add timed lexer unit tests"
```

---

### Task 3: `GroupStack`

**Files:**
- Modify: `src/parser/score/timed_parser/groups.rs`

- [ ] **Step 1: Replace `GroupParseState` with `GroupStack`**

```rust
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
        if let Some(frame) = self.frames.last_mut() {
            frame.note_count += 1;
        }
    }
}
```

Keep `validate_group_note_count`, `HasGroupDepth`, `apply_closed_group_depth`, `apply_open_group_depth`. **Delete** `find_closing_paren`, `apply_closing_segment_depth`, `GroupParseState`.

- [ ] **Step 2: Update `mod.rs` re-exports**

```rust
pub use groups::{
    apply_closed_group_depth, apply_open_group_depth, validate_group_note_count, GroupStack,
    GroupFrame, HasGroupDepth,
};
```

Remove `GroupParseState`, `find_closing_paren` exports.

- [ ] **Step 3: Run `cargo check`**

Expected: compile errors at old call sites (fixed in later tasks).

- [ ] **Step 4: Commit**

```bash
git add src/parser/score/timed_parser/groups.rs src/parser/score/timed_parser/mod.rs
git commit -m "refactor: replace GroupParseState with GroupStack"
```

---

### Task 4: RD parser — timed units + extensions + directives

**Files:**
- Create: `src/parser/score/timed_parser/timed_rd_parser.rs`
- Modify: `src/parser/score/timed_parser/mod.rs`

- [ ] **Step 1: Create parser struct**

```rust
use super::duration::parse_duration_suffixes;
use super::groups::{apply_open_group_depth, validate_group_note_count, GroupStack, HasGroupDepth};
use super::timed_lexer::TimedLexToken;
use super::TimedUnitHead;
use crate::ast::parsed::ScoreEvent;
use crate::error::{JianPuError, Span, Spanned};

pub struct TimedRdParser<'a, H: TimedUnitHead> {
    source: &'a str,
    base_offset: usize,
    tokens: &'a [Spanned<TimedLexToken>],
    pos: usize,
    stack: &'a mut GroupStack,
    events: Vec<Spanned<ScoreEvent>>,
    _head: std::marker::PhantomData<H>,
}

impl<'a, H: TimedUnitHead> TimedRdParser<'a, H> {
    pub fn parse_line(
        source: &'a str,
        base_offset: usize,
        tokens: &'a [Spanned<TimedLexToken>],
        stack: &'a mut GroupStack,
    ) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
        let mut parser = Self {
            source,
            base_offset,
            tokens,
            pos: 0,
            stack,
            events: Vec::new(),
            _head: std::marker::PhantomData,
        };
        parser.parse_atoms(false)?;
        parser.finalize_open_frames()?;
        Ok(parser.events)
    }

    fn peek(&self) -> Option<&Spanned<TimedLexToken>> {
        self.tokens.get(self.pos)
    }

    fn bump(&mut self) -> Option<Spanned<TimedLexToken>> {
        let tok = self.tokens.get(self.pos).cloned();
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn parse_atoms(&mut self, stop_at_rparen: bool) -> Result<(), JianPuError> {
        loop {
            match self.peek().map(|t| &t.value) {
                None => return Ok(()),
                Some(TimedLexToken::RParen) if stop_at_rparen => return Ok(()),
                Some(TimedLexToken::RParen) => return self.close_group(),
                Some(TimedLexToken::LParen) => self.open_group()?,
                Some(TimedLexToken::Extension) => self.parse_extension()?,
                Some(TimedLexToken::HeadStart { offset }) => {
                    let offset = *offset;
                    self.parse_timed_unit(offset)?;
                }
                Some(TimedLexToken::Bpm(bpm)) => {
                    let span = self.bump().unwrap().span;
                    self.events.push(Spanned::new(ScoreEvent::BpmChange(*bpm), span));
                }
                Some(TimedLexToken::KeyChange(key)) => {
                    let span = self.bump().unwrap().span;
                    self.events
                        .push(Spanned::new(ScoreEvent::KeyChange(key.clone()), span));
                }
                Some(TimedLexToken::TimeSignature { num, den }) => {
                    let span = self.bump().unwrap().span;
                    self.events.push(Spanned::new(
                        ScoreEvent::TimeSignatureChange {
                            numerator: *num,
                            denominator: *den,
                        },
                        span,
                    ));
                }
            }
        }
    }

    fn parse_timed_unit(&mut self, digit_offset: usize) -> Result<(), JianPuError> {
        let rel = digit_offset.saturating_sub(self.base_offset);
        let text = self.source.get(rel..).ok_or_else(|| {
            JianPuError::new(Span::new(digit_offset, digit_offset + 1), "empty timed unit".into())
        })?;
        let chars: Vec<char> = text.chars().collect();
        let span = Span::new(digit_offset, digit_offset + 1);
        let (head, head_end, is_rest) = H::parse_head(&chars, 0, &span)?;
        let duration_meta =
            parse_duration_suffixes::<H>(&chars, 0, head_end, is_rest, &span)?;
        let unit_end = self.base_offset
            + text
                .char_indices()
                .nth(duration_meta.next_index)
                .map(|(i, _)| i)
                .unwrap_or(text.len());
        let event = H::to_event(
            &head,
            duration_meta.duration,
            duration_meta.dotted,
            if duration_meta.octave_up > 0 {
                duration_meta.octave_up
            } else {
                -duration_meta.octave_down
            },
            0,
            0,
        );
        self.events
            .push(Spanned::new(event, Span::new(digit_offset, unit_end)));
        self.stack.increment_note_count();
        // Advance token stream past HeadStart token only (lexer emits one token per digit)
        if matches!(self.peek().map(|t| &t.value), Some(TimedLexToken::HeadStart { .. })) {
            self.bump();
        }
        Ok(())
    }

    fn parse_extension(&mut self) -> Result<(), JianPuError> {
        let span = self.bump().unwrap().span;
        self.events.push(Spanned::new(ScoreEvent::Extension, span));
        Ok(())
    }

    fn open_group(&mut self) -> Result<(), JianPuError> {
        self.bump();
        self.stack.push(self.events.len());
        self.parse_atoms(true)?;
        Ok(())
    }

    fn close_group(&mut self) -> Result<(), JianPuError> {
        let close_span = self.bump().unwrap().span;
        let frame = self.stack.pop().ok_or_else(|| {
            JianPuError::new(close_span.clone(), "unexpected ')'".to_string())
        })?;
        validate_group_note_count(frame.note_count, &close_span)?;
        self.apply_closed_segment(frame.segment_start, false);
        Ok(())
    }

    fn finalize_open_frames(&mut self) -> Result<(), JianPuError> {
        while let Some(frame) = self.stack.frames.last() {
            let start = frame.segment_start;
            self.apply_open_segment(start);
            break;
        }
        Ok(())
    }

    fn apply_closed_segment(&mut self, start: usize, still_open: bool) {
        // Build mutable slice helpers for events in [start..]
        // Use apply_closed_group_depth on a temporary wrapper or inline membership update
        // Mirror existing groups.rs logic on ScoreEvent note/chord/rest fields
        let _ = still_open;
        let _ = start;
        // Implement: extract apply_depth_to_events(&mut self.events[start..], closed: true)
    }

    fn apply_open_segment(&mut self, start: usize) {
        let _ = start;
        // Implement: apply_open_group_depth equivalent on self.events[start..]
    }
}
```

Implement `apply_closed_segment` / `apply_open_segment` by adding `impl HasGroupDepth for Spanned<ScoreEvent>` helper in `groups.rs` or a local adapter that mutates `ParsedNote` / `ParsedChordNote` / `ParsedRest` inside events.

- [ ] **Step 2: Add `parse_timed_line` to `mod.rs`**

```rust
mod timed_rd_parser;
pub use timed_rd_parser::TimedRdParser;

pub fn parse_timed_line<H: TimedUnitHead>(
    line: &str,
    base_offset: usize,
    stack: &mut GroupStack,
) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
    let tokens = lex_line(line, base_offset)?;
    TimedRdParser::<H>::parse_line(line, base_offset, &tokens, stack)
}
```

- [ ] **Step 3: Write failing test for simple note line**

In `timed_rd_parser_tests.rs`:

```rust
#[test]
fn parses_spaced_notes() {
    use super::{parse_timed_line, GroupStack, NoteHead};
    let events = parse_timed_line::<NoteHead>("5 0 5", 0, &mut GroupStack::default()).unwrap();
    assert_eq!(events.len(), 3);
}
```

- [ ] **Step 4: Run test, implement depth helpers, run until PASS**

Run: `cargo test --lib timed_rd_parser_tests::parses_spaced_notes -- --nocapture`

- [ ] **Step 5: Commit**

```bash
git add src/parser/score/timed_parser/timed_rd_parser.rs src/parser/score/timed_parser/timed_rd_parser_tests.rs src/parser/score/timed_parser/groups.rs src/parser/score/timed_parser/mod.rs
git commit -m "feat: add timed RD parser for units, extensions, directives"
```

---

### Task 5: RD parser — groups + cross-bar depth

**Files:**
- Modify: `src/parser/score/timed_parser/timed_rd_parser.rs`
- Modify: `src/parser/score/timed_parser/timed_rd_parser_tests.rs`

- [ ] **Step 1: Write failing group tests**

```rust
#[test]
fn parses_spaced_nested_outer_group() {
    use super::{parse_timed_line, GroupStack, NoteHead};
    use crate::ast::parsed::{JianPuPitch, ScoreEvent};
    let events =
        parse_timed_line::<NoteHead>("((1 1) 5 5)", 0, &mut GroupStack::default()).unwrap();
    assert_eq!(events.len(), 4);
    // outer slur connects all 4; inner connects first 2
}

#[test]
fn rejects_single_note_group() {
    assert!(parse_timed_line::<NoteHead>("(3)", 0, &mut GroupStack::default()).is_err());
}

#[test]
fn cross_bar_nested_groups() {
    use super::{parse_timed_line, GroupStack, NoteHead};
    let mut stack = GroupStack::default();
    parse_timed_line::<NoteHead>("((1 1", 0, &mut stack).unwrap();
    assert!(stack.is_open());
    let events = parse_timed_line::<NoteHead>("5 5))", 0, &mut stack).unwrap();
    assert!(!stack.is_open());
    assert_eq!(events.len(), 2);
}

#[test]
fn cross_bar_outer_and_inner() {
    use super::{parse_timed_line, GroupStack, NoteHead};
    let mut stack = GroupStack::default();
    parse_timed_line::<NoteHead>("(3= (2_", 0, &mut stack).unwrap();
    let events = parse_timed_line::<NoteHead>("1_))", 0, &mut stack).unwrap();
    assert!(!stack.is_open());
    assert_eq!(events.len(), 1);
}
```

- [ ] **Step 2: Implement depth application on `ScoreEvent`**

Add to `groups.rs`:

```rust
pub fn apply_closed_group_depth_to_events(events: &mut [Spanned<ScoreEvent>]) { ... }
pub fn apply_open_group_depth_to_events(events: &mut [Spanned<ScoreEvent>]) { ... }
```

Use inner helper that matches on `ScoreEvent::Note | Rest | Chord` and updates `group_membership` / `group_continuation` / `tie`.

On `finalize_open_frames` at line end: call `apply_open_group_depth_to_events` on `events[frame.segment_start..]` for the innermost open frame (stack persists; segment_start resets to 0 on next line — **reset `segment_start` to 0** at start of each `parse_line` when stack non-empty, and accumulate `note_count` across lines).

- [ ] **Step 3: Fix `HeadStart` token advance**

When timed unit is `3=` or `1_`, lexer emits one `HeadStart` but multiple chars belong to the unit. After `parse_timed_unit`, skip additional `HeadStart` tokens whose offset falls before `unit_end` (shouldn't happen if lexer is correct) — instead ensure lexer only emits `HeadStart` once per unit. **Important:** for concatenated notes `505`, lexer emits three `HeadStart` tokens; each call to `parse_timed_unit` consumes one. Verify `505` and `5 0 5` produce 3 events.

- [ ] **Step 4: Run group tests**

Run: `cargo test --lib timed_rd_parser_tests -- --nocapture`
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add src/parser/score/timed_parser/
git commit -m "feat: RD parser group stack with cross-bar nesting"
```

---

### Task 6: Rewire `token_parser.rs`

**Files:**
- Modify: `src/parser/score/token_parser.rs`

- [ ] **Step 1: Replace old API**

```rust
use crate::parser::score::timed_parser::{parse_timed_line, GroupStack, NoteHead, ChordHead};

pub use crate::parser::score::timed_parser::GroupStack;

pub fn parse_notes_line(
    line: &str,
    base_offset: usize,
    stack: &mut GroupStack,
) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
    parse_timed_line::<NoteHead>(line, base_offset, stack)
}

pub fn parse_chord_line(
    line: &str,
    base_offset: usize,
    stack: &mut GroupStack,
) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
    parse_timed_line::<ChordHead>(line, base_offset, stack)
}
```

Remove `RawToken`, `parse_tokens(Vec<RawToken>)`, `parse_chord_tokens`, `parse_timed_token` imports.

- [ ] **Step 2: Migrate `token_parser` tests**

Replace test helpers:

```rust
fn parse(input: &str) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
    parse_notes_line(input, 0, &mut GroupStack::default())
}

fn parse_with_state(input: &str, state: &mut GroupStack) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
    parse_notes_line(input, 0, state)
}
```

Remove `use crate::parser::score::tokenizer::tokenize`.

Update `GroupParseState` → `GroupStack` in all tests. Remove tests that assert `state.open` — use `state.is_open()`. Remove `GroupParseState { open: true, open_note_count: N }` manual struct literals; use `parse_with_state` to build state.

- [ ] **Step 3: Run tests**

Run: `cargo test --lib parser::score::token_parser -- --nocapture`
Expected: all PASS

- [ ] **Step 4: Commit**

```bash
git add src/parser/score/token_parser.rs
git commit -m "refactor: token_parser wraps parse_notes_line/chord_line"
```

---

### Task 7: Rewire call sites + delete old engine

**Files:**
- Modify: `src/parser/score/interleaved_parser.rs`
- Modify: `src/grouping.rs`
- Modify: `src/layout/mod.rs`
- Modify: `src/parser/score/timed_parser/chord_head.rs`
- Modify: `src/parser/score/timed_parser/mod.rs`
- Delete: `src/parser/score/tokenizer.rs`
- Modify: `src/parser/score/mod.rs`

- [ ] **Step 1: Update `interleaved_parser.rs`**

```rust
use crate::parser::score::token_parser::GroupStack;

// BarGroupContext:
group_states: &'a mut [GroupStack],

// init:
let mut group_states = vec![GroupStack::default(); declarations.len()];

// EOF check:
if state.is_open() { ... }

// Notes slot:
let events = validate_and_pad_beats(
    token_parser::parse_notes_line(line, ctx.base_offset + line_offset, group_state)?,
    ...
)?;

// Chord slot:
token_parser::parse_chord_line(line, ctx.base_offset + line_offset, group_state)?
```

- [ ] **Step 2: Update test call sites in `grouping.rs` and `layout/mod.rs`**

Replace:

```rust
token_parser::parse_tokens(tokenizer::tokenize(bar, 0), &mut state)
```

with:

```rust
token_parser::parse_notes_line(bar, 0, &mut state)
```

- [ ] **Step 3: Update `chord_head.rs` tests** — use `parse_chord_line` instead of `parse_chord_tokens(tokenize(...))`.

- [ ] **Step 4: Delete old `timed_parser/mod.rs` engine**

Remove: `parse_timed_token`, `parse_timed_tokens`, `parse_atoms_from_chars`, `parse_closed_group`, `parse_open_group`, `parse_closing_group_segment`, all helper fns that referenced `RawToken`.

- [ ] **Step 5: Delete `tokenizer.rs`, remove from `score/mod.rs`**

```rust
pub mod interleaved_parser;
pub mod timed_parser;
pub mod token_parser;
// removed: pub mod tokenizer;
```

- [ ] **Step 6: Run full test suite**

Run: `cargo test --lib`
Expected: all PASS

- [ ] **Step 7: Commit**

```bash
git add src/parser/score/ src/grouping.rs src/layout/mod.rs
git rm src/parser/score/tokenizer.rs
git commit -m "refactor: rewire call sites and remove whitespace tokenizer"
```

---

### Task 8: `syntax.md` + WASM + final verification

**Files:**
- Modify: `syntax.md`

- [ ] **Step 1: Update syntax doc**

In the "Note lines" section, replace:

> Note lines are whitespace-separated **tokens**.

with:

> Note lines are a sequence of **atoms** (notes, rests, chords, extensions, groups). Whitespace is optional between atoms and ignored inside `(…)` groups. The `|` character is accepted but ignored (legacy bar separator).

Add example: `((1 1) 5 5)` equivalent to `((11)55)`.

- [ ] **Step 2: Run full tests**

Run: `cargo test`
Expected: all PASS

- [ ] **Step 3: Rebuild WASM**

Run: `cd web && pnpm run build:wasm`
Expected: success

- [ ] **Step 4: Commit**

```bash
git add syntax.md
git commit -m "docs: whitespace optional between timed atoms"
```

---

## Spec Coverage Checklist

| Spec requirement | Task |
|------------------|------|
| Char-by-char typed lexer | Task 1–2 |
| Whitespace insignificant | Task 2, 5, 8 |
| `((1 1) 5 5)` parses | Task 5 |
| Full `GroupStack` cross-bar | Task 3, 5 |
| Recursive descent grammar | Task 4–5 |
| Reuse NoteHead/ChordHead/duration | Task 4 |
| Remove whitespace RawToken | Task 7 |
| `parse_notes_line` / `parse_chord_line` API | Task 6 |
| interleaved_parser GroupStack | Task 7 |
| syntax.md update | Task 8 |
| Error messages preserved | Task 4–5 |
| WASM rebuild | Task 8 |

---

## Out of Scope (do not implement)

- Lyrics tokenization
- Measure directive row parsing changes
- 4/4 grouping validation rule changes
