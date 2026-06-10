# Timed Lexer + Recursive Descent Parser Design

## Overview

Replace the whitespace-based score tokenizer and hybrid group parser with a char-by-char **typed lexer** and **recursive descent parser** for notes and chord lines. Whitespace becomes insignificant between atoms; nested `(…)` groups parse correctly regardless of spacing; and a **full group stack** persists across bar lines so groups can span measures at any nesting depth.

**Motivation:** Input like `((1 1) 5 5)` fails today because whitespace splitting produces tokens `((1`, `1)`, `5`, `5)`, and the inner parser requires balanced parens within a single whitespace chunk. The syntax spec already states whitespace inside groups is insignificant — the implementation must match.

**Decisions (confirmed):**

- Typed lexical tokens (not a merged char cursor).
- Full `GroupStack` across bar lines (nested groups may span measures at any depth).
- Separate lexer + RD parser modules (Approach 1); reuse existing `NoteHead`, `ChordHead`, and duration suffix logic.

---

## Section 1: Architecture

### Pipeline (notes/chord lines)

```
line text
  → timed_lexer::lex_line(text, offset) → Vec<Spanned<TimedLexToken>>
  → timed_rd_parser::parse_line(tokens, head, &mut GroupStack) → Vec<Spanned<ScoreEvent>>
```

`interleaved_parser` calls this per notes/chord slot, passing the same `GroupStack` across bars for that track. At score EOF, any non-empty stack produces `"unclosed '(' group at end of score in part '…'"`.

### Lexer (`src/parser/score/timed_parser/timed_lexer.rs`)

Char-by-char scan. **Whitespace and `|` are skipped** (never produce tokens).

| Token | When emitted |
|-------|--------------|
| `LParen` | `(` |
| `RParen` | `)` |
| `Extension` | `-` at an **atom boundary** (line start, after skipped whitespace, after `(` or `)`) |
| `HeadStart` | `0`–`7` starting a timed unit; carries byte offset of the digit |
| `Bpm(u32)` | `bpm=<digits>` |
| `KeyChange(...)` | `1=<NoteName><octave>` where character after `=` is A–G |
| `TimeSignature { num, den }` | `<n>/<d>` where both sides are digit sequences |

**Atom boundary for `-`:** The lexer tracks whether the cursor is at the start of a new atom. At such a position, `-` emits `Extension`. Otherwise `-` is left in the source and consumed by the duration sub-parser when handling a `HeadStart`.

**Directive disambiguation:** `1=,` (sixteenth note) emits `HeadStart`, not `KeyChange`. Key change requires A–G immediately after `=`. Time signatures require digit/digit form; chord slash chords like `6m/5` are handled inside `ChordHead` maximal munch, not by the time-signature lexer rule (chord lines do not emit standalone `TimeSignature` tokens from ambiguous patterns — the chord head parser owns `/` within a timed unit).

### Parser (`src/parser/score/timed_parser/timed_rd_parser.rs`)

Recursive descent grammar:

```
line       ::= atoms
atoms      ::= atom*
atom       ::= group | timed_unit | extension | directive
group      ::= LParen atoms RParen
timed_unit ::= HeadStart → (NoteHead|ChordHead + parse_duration_suffixes)
extension  ::= Extension
directive  ::= Bpm | KeyChange | TimeSignature
```

Notes and chords share the same RD parser; only the `TimedUnitHead` impl differs (`NoteHead` vs `ChordHead`).

### GroupStack (replaces `GroupParseState`)

```rust
pub struct GroupStack {
    frames: Vec<GroupFrame>,
}

struct GroupFrame {
    note_count: usize,  // notes emitted inside this frame; used for ≥2 validation on close
}
```

Behavior:

- **`LParen`** → push a new frame with `note_count = 0`, parse inner `atoms` until `RParen` or line end. If line ends first, the frame stays open for the next line on the same track.
- **`RParen`** → pop the innermost frame, validate `note_count >= 2`, apply group depth (`group_membership`, `group_continuation`, tie flags) to notes in that segment. If stack is empty, error with `unexpected ')'`.
- **Line end with open frames** → parsing succeeds; stack persists into the next bar line for that track.
- **Score EOF with open frames** → error (existing message, per-part label).
- **Nesting depth** → `frames.len()` after push drives `group_membership`; tie/continuation uses refactored helpers from `groups.rs`.

Cross-bar examples enabled by full stack:

- Bar 1: `((1 1` → two open frames; bar 2: `5 5))` → closes inner then outer.
- Bar 1: `(3= (2_` → outer + inner open; bar 2: `1_))` → inner closes then outer.

### Removed

- Whitespace-based `RawToken` string chunks for timed lines.
- `parse_timed_token`, `parse_atoms_from_chars`, `find_closing_paren`, and open/close segment helpers in `timed_parser/mod.rs`.
- `GroupParseState { open, open_note_count }`.

### Unchanged

- Lyrics tokenization (`tokenize_lyrics`).
- Measure directive rows `(bpm=… key=…)` via `interleaved_directives`.
- Beat padding, 4/4 grouping validation, layout, MIDI — same `ScoreEvent` output shape.

---

## Section 2: Error Handling & Spans

### Span assignment

- Structural tokens (`LParen`, `RParen`, `Extension`, directives): span covers the lexeme bytes.
- `HeadStart`: span starts at the digit; the RD parser extends the emitted event span to the full timed unit (head + suffixes), consistent with current per-chunk span behavior.

### Error messages

| Condition | Message (existing where noted) |
|-----------|-------------------------------|
| Score EOF, non-empty stack | `unclosed '(' group at end of score in part '…'` |
| `RParen` with empty stack | `unexpected ')'` |
| Frame closed with `< 2` notes | `tie/slur group '(…)' must contain at least 2 notes` |
| Unknown character at lex time | `unexpected character: …` |
| Head/duration validation | Reuse `JianPuError` from `NoteHead`, `ChordHead`, `duration.rs` |

### Whitespace semantics

Whitespace is **never significant** on notes/chord lines:

- `505` ≡ `5 0 5`
- `((1 1) 5 5)` ≡ `((11)55)`
- `(1 - 6m -)` ≡ `(1-6m-)`

**`syntax.md` update (same commit as implementation):** Replace “whitespace-separated tokens” with “whitespace is optional between atoms; ignored inside `(…)` groups.”

---

## Section 3: Public API & Call Sites

### Exports (`timed_parser/mod.rs`)

```rust
pub struct GroupStack { ... }

pub fn parse_timed_line<H: TimedUnitHead>(
    line: &str,
    base_offset: usize,
    stack: &mut GroupStack,
) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError>
```

### Wrappers (`token_parser.rs`)

Keep familiar entry points; change signature to take line text + offset instead of `Vec<RawToken>`:

```rust
pub fn parse_notes_line(line: &str, base_offset: usize, stack: &mut GroupStack) -> ...
pub fn parse_chord_line(line: &str, base_offset: usize, stack: &mut GroupStack) -> ...
```

Inline directives (`bpm=`, `1=C4`, `4/4`) and standalone `-` handling remain in the wrapper or RD parser as today.

### Call site updates

| File | Change |
|------|--------|
| `interleaved_parser.rs` | `GroupStack` per track; call `parse_notes_line` / `parse_chord_line` with line text |
| `grouping.rs` (tests) | Same |
| `layout/mod.rs` (test helper) | Same |
| `timed_parser/chord_head.rs` (tests) | Same |

### `tokenizer.rs`

Remove from notes/chord path. Delete `tokenizer::tokenize` if no remaining callers; otherwise keep file only for any legacy use found during migration (grep before delete).

---

## Section 4: Testing

### Lexer unit tests

- Skips whitespace and `|`.
- `((1 1) 5 5)` → `LParen LParen HeadStart(1) HeadStart(1) RParen HeadStart(5) HeadStart(5) RParen`.
- `2---` → single `HeadStart`; `2 - - -` → `HeadStart Extension Extension Extension`.
- `1=,` → `HeadStart`, not key change; `1=C4` → `KeyChange`.
- `bpm=120`, `4/4` directives.

### RD parser unit tests

- Migrate all existing `token_parser` and `timed_parser` group tests.
- **New:** spaced nested outer group `((1 1) 5 5)`.
- **New:** cross-bar nested `((1 1` / `5 5))`.
- **New:** cross-bar outer + inner `(3= (2_` / `1_))`.
- Chord: `(1 - 6m -)`, nested chord groups, cross-bar chord groups.

### Integration

- Existing `interleaved_parser`, grouping validation, and layout tests pass (call-site updates only unless behavior intentionally expands).

---

## Section 5: Implementation Order

1. Add `TimedLexToken`, `timed_lexer.rs`, lexer tests.
2. Add `GroupStack`, refactor `groups.rs` depth helpers for stack model.
3. Add `timed_rd_parser.rs`, wire `NoteHead`/`ChordHead`/duration sub-parsers.
4. Rewire `token_parser.rs` and `interleaved_parser.rs`.
5. Update `grouping.rs`, `layout/mod.rs` tests.
6. Remove dead code from `timed_parser/mod.rs`; remove or narrow `tokenizer.rs`.
7. Update `syntax.md`.
8. `cargo test`; rebuild WASM (`cd web && pnpm run build:wasm`).

---

## Section 6: Out of Scope

- Lyrics tokenization changes.
- Measure-level directive row parsing.
- 4/4 grouping validation rule changes.
- Renderer or MIDI behavior changes beyond what identical `ScoreEvent` output requires.
