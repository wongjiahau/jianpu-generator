# Parts Section Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace metadata `parts = …` with a required `[parts]` section, first-class `PartDecl`/`ParsedTrack` model, and dedicated `parts_parser.rs` — breaking change, no defaults.

**Architecture:** Three-section document (`[metadata]` → `[parts]` → `[score]`). `parts_parser.rs` parses track declarations only. Score parsing consumes `&[PartDecl]`, flattening to transient `ScoreLineSlot`s for ditto/padding. Grouper and combiner walk tracks in declaration order; row labels use `abbreviation` only.

**Tech Stack:** Rust. Run tests with `cargo test`. Spec: `docs/superpowers/specs/2026-06-08-parts-section-design.md`.

---

## File Map

| File | Change |
|------|--------|
| `src/ast/parsed.rs` | Add `PartDecl`, `PartKind`, `ParsedTrack`, `ParsedNotesTrack`, `ParsedChordTrack`, `ScoreLineRole`, `ScoreLineSlot`; remove `PartColumn`, `ParsedPart`, `ParsedChordPart`; reshape `ParsedDocument` |
| `src/parser/section_splitter.rs` | Add `SectionKind::Parts`; validate section order |
| `src/parser/parts_parser.rs` | **New.** Dedicated `[parts]` parser + unit tests |
| `src/parser/metadata_parser.rs` | Remove `parts` field + `parse_parts()` |
| `src/parser/mod.rs` | Orchestration: wire three sections; export `parts_parser` |
| `src/desugar.rs` | Take `&[PartDecl]` + flattened slots; drop `PartColumn` |
| `src/parser/score/interleaved_parser.rs` | Input `&[PartDecl]` → output `Vec<ParsedTrack>` |
| `src/grouper.rs` | Group `ParsedTrack` → `GroupedTrack`; drop `metadata.parts` |
| `src/ast/grouped.rs` | Add `GroupedTrack` enum (optional crate-private) |
| `src/combiner.rs` | Combine aligned `GroupedTrack` slices; use `abbreviation` for labels |
| `src/layout/mod.rs`, `src/layout/layout_engine.rs` | Update test fixtures only (unless compile errors) |
| `src/renderer.rs`, `src/pdf.rs`, `src/main.rs` | Fix any `doc.parts` / `doc.chord_parts` references |
| `tests/integration.rs` | Migrate `basic_jianpu_input()` |
| `demo.jianpu`, `彌勒淨土鄉.jianpu` | Migrate to `[parts]` section |
| `syntax.md` | Document new syntax |

**Do NOT** add `[parts]` parsing logic to `metadata_parser.rs` or `interleaved_parser.rs`.

---

### Task 1: New AST types in `parsed.rs`

**Files:**
- Modify: `src/ast/parsed.rs`

- [ ] **Step 1: Add new types and remove legacy ones**

Replace `PartColumn`, `ParsedPart`, `ParsedChordPart`, and reshape `ParsedDocument`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct PartDecl {
    pub abbreviation: String,
    pub display_name: String,
    pub kind: PartKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PartKind {
    Chord,
    Notes,
    NotesWithLyrics,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScoreLineRole {
    Chord,
    Notes,
    Lyrics,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScoreLineSlot {
    pub track_index: usize,
    pub role: ScoreLineRole,
}

impl PartDecl {
    pub fn score_line_roles(&self) -> &'static [ScoreLineRole] {
        match self.kind {
            PartKind::Chord => &[ScoreLineRole::Chord],
            PartKind::Notes => &[ScoreLineRole::Notes],
            PartKind::NotesWithLyrics => &[ScoreLineRole::Notes, ScoreLineRole::Lyrics],
        }
    }
}

pub fn flatten_score_line_slots(declarations: &[PartDecl]) -> Vec<ScoreLineSlot> {
    let mut slots = Vec::new();
    for (track_index, decl) in declarations.iter().enumerate() {
        for &role in decl.score_line_roles() {
            slots.push(ScoreLineSlot { track_index, role });
        }
    }
    slots
}

pub struct ParsedNotesTrack {
    pub abbreviation: String,
    pub display_name: String,
    pub score: ParsedScore,
    pub lyrics: Option<ParsedLyrics>,
}

pub struct ParsedChordTrack {
    pub abbreviation: String,
    pub display_name: String,
    pub events_per_measure: Vec<Vec<ParsedChordEvent>>,
}

pub enum ParsedTrack {
    Chord(ParsedChordTrack),
    Notes(ParsedNotesTrack),
}

pub struct ParsedDocument {
    pub filename: String,
    pub metadata: ParsedMetadata,
    pub declarations: Vec<PartDecl>,
    pub tracks: Vec<ParsedTrack>,
}
```

Remove from `ParsedMetadata`: `pub parts: Vec<PartColumn>`.

Keep `ParsedChordEvent`, `ParsedChordSymbol`, etc. — only remove `ParsedChordPart` and `ParsedPart`.

- [ ] **Step 2: Run check**

```bash
cargo check 2>&1 | head -40
```

Expected: compile errors in files still referencing removed types — that's OK for this step.

- [ ] **Step 3: Commit**

```bash
git add src/ast/parsed.rs
git commit -m "refactor: replace PartColumn with PartDecl and ParsedTrack AST"
```

---

### Task 2: Section splitter — add `[parts]`

**Files:**
- Modify: `src/parser/section_splitter.rs`

- [ ] **Step 1: Write failing tests**

Add to `section_splitter.rs` tests module:

```rust
#[test]
fn splits_metadata_parts_and_score() {
    let input = "[metadata]\ntitle=\"t\"\n\n[parts]\nMelody = notes lyrics\n\n[score]\n1 2\n";
    let sections = split_sections(input).unwrap();
    assert_eq!(sections.len(), 3);
    assert_eq!(sections[0].kind, SectionKind::Metadata);
    assert_eq!(sections[1].kind, SectionKind::Parts);
    assert_eq!(sections[2].kind, SectionKind::Score);
    assert_eq!(sections[1].content.trim(), "Melody = notes lyrics");
}

#[test]
fn rejects_parts_after_score() {
    let input = "[metadata]\nt\n[score]\n1\n[parts]\nMelody = notes\n";
    assert!(split_sections(input).is_err());
}
```

- [ ] **Step 2: Run tests to verify failure**

```bash
cargo test section_splitter:: 2>&1
```

Expected: FAIL — `SectionKind::Parts` missing or order check missing.

- [ ] **Step 3: Implement**

```rust
pub enum SectionKind {
    Metadata,
    Parts,
    Score,
}

// In split_sections, match "parts" => SectionKind::Parts

// After building sections vec, validate order:
fn validate_section_order(sections: &[RawSection]) -> Result<(), JianPuError> {
    let expected = [SectionKind::Metadata, SectionKind::Parts, SectionKind::Score];
    if sections.len() != expected.len() {
        return Err(JianPuError::new(
            Span::new(0, 0),
            format!(
                "expected exactly 3 sections ([metadata], [parts], [score]), got {}",
                sections.len()
            ),
        ));
    }
    for (section, exp) in sections.iter().zip(expected.iter()) {
        if &section.kind != exp {
            return Err(JianPuError::new(
                Span::new(0, 0),
                "sections must appear in order: [metadata], [parts], [score]".to_string(),
            ));
        }
    }
    Ok(())
}
```

Call `validate_section_order` before returning from `split_sections`. Update existing two-section tests to include `[parts]`.

- [ ] **Step 4: Run tests**

```bash
cargo test section_splitter:: 2>&1
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/parser/section_splitter.rs
git commit -m "feat: add [parts] section to section splitter"
```

---

### Task 3: Dedicated `parts_parser.rs`

**Files:**
- Create: `src/parser/parts_parser.rs`
- Modify: `src/parser/mod.rs` (add `pub mod parts_parser;`)

- [ ] **Step 1: Write failing tests**

Create `src/parser/parts_parser.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::parsed::{PartDecl, PartKind};

    #[test]
    fn parses_abbreviated_track() {
        let content = "Alto 1 & Tenor (A1&T) = notes lyrics\n";
        let decls = parse_parts(content, 0).unwrap();
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].display_name, "Alto 1 & Tenor");
        assert_eq!(decls[0].abbreviation, "A1&T");
        assert_eq!(decls[0].kind, PartKind::NotesWithLyrics);
    }

    #[test]
    fn parses_chord_track() {
        let content = "main = chord\n";
        let decls = parse_parts(content, 0).unwrap();
        assert_eq!(decls[0].abbreviation, "main");
        assert_eq!(decls[0].display_name, "main");
        assert_eq!(decls[0].kind, PartKind::Chord);
    }

    #[test]
    fn omits_parens_uses_name_as_abbreviation() {
        let content = "Melody = notes lyrics\n";
        let decls = parse_parts(content, 0).unwrap();
        assert_eq!(decls[0].abbreviation, "Melody");
        assert_eq!(decls[0].display_name, "Melody");
    }

    #[test]
    fn rejects_duplicate_abbreviations() {
        let content = "A (x) = notes\nB (x) = notes\n";
        assert!(parse_parts(content, 0).is_err());
    }

    #[test]
    fn rejects_lyrics_without_notes() {
        let content = "X = lyrics\n";
        assert!(parse_parts(content, 0).is_err());
    }

    #[test]
    fn rejects_parts_in_metadata_is_not_this_modules_job() {
        // sanity: parts_parser only receives section body
        let content = "title = \"t\"\n";
        assert!(parse_parts(content, 0).is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify failure**

```bash
cargo test parts_parser:: 2>&1
```

Expected: FAIL — module/function not found.

- [ ] **Step 3: Implement `parse_parts`**

```rust
use crate::ast::parsed::{PartDecl, PartKind};
use crate::error::{JianPuError, Span};

pub fn parse_parts(content: &str, base_offset: usize) -> Result<Vec<PartDecl>, JianPuError> {
    let mut declarations = Vec::new();
    let mut seen_abbreviations = std::collections::HashSet::new();
    let mut byte_offset = base_offset;

    for line in content.lines() {
        let trimmed = line.trim();
        byte_offset += line.len() + 1;
        if trimmed.is_empty() {
            continue;
        }
        let line_span = Span::new(byte_offset - line.len() - 1, byte_offset - 1);

        let (lhs, rhs) = trimmed.split_once('=').ok_or_else(|| {
            JianPuError::new(line_span.clone(), format!("expected track declaration, got: {trimmed}"))
        })?;
        let lhs = lhs.trim();
        let rhs = rhs.trim();

        let (display_name, abbreviation) = parse_lhs(lhs, &line_span)?;
        if !seen_abbreviations.insert(abbreviation.clone()) {
            return Err(JianPuError::new(
                line_span.clone(),
                format!("duplicate abbreviation: {abbreviation}"),
            ));
        }

        let kind = parse_rhs(rhs, &line_span)?;
        declarations.push(PartDecl {
            abbreviation,
            display_name,
            kind,
        });
    }

    if declarations.is_empty() {
        return Err(JianPuError::new(
            Span::new(base_offset, base_offset + content.len().max(1)),
            "expected at least one track in [parts] section",
        ));
    }

    Ok(declarations)
}

fn parse_lhs(lhs: &str, span: &Span) -> Result<(String, String), JianPuError> {
    if let Some(open) = lhs.rfind('(') {
        if lhs.ends_with(')') {
            let display_name = lhs[..open].trim().to_string();
            let abbreviation = lhs[open + 1..lhs.len() - 1].trim().to_string();
            if display_name.is_empty() {
                return Err(JianPuError::new(span.clone(), "display name cannot be empty".to_string()));
            }
            if abbreviation.is_empty() {
                return Err(JianPuError::new(span.clone(), "abbreviation cannot be empty".to_string()));
            }
            return Ok((display_name, abbreviation));
        }
    }
    let name = lhs.trim().to_string();
    if name.is_empty() {
        return Err(JianPuError::new(span.clone(), "track name cannot be empty".to_string()));
    }
    Ok((name.clone(), name))
}

fn parse_rhs(rhs: &str, span: &Span) -> Result<PartKind, JianPuError> {
    let tokens: Vec<&str> = rhs.split_whitespace().collect();
    match tokens.as_slice() {
        ["chord"] => Ok(PartKind::Chord),
        ["notes"] => Ok(PartKind::Notes),
        ["notes", "lyrics"] => Ok(PartKind::NotesWithLyrics),
        _ => Err(JianPuError::new(
            span.clone(),
            format!(
                "invalid track columns '{rhs}': expected 'chord', 'notes', or 'notes lyrics'"
            ),
        )),
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test parts_parser:: 2>&1
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/parser/parts_parser.rs src/parser/mod.rs
git commit -m "feat: add dedicated parts_parser for [parts] section"
```

---

### Task 4: Strip `parts` from metadata parser

**Files:**
- Modify: `src/parser/metadata_parser.rs`

- [ ] **Step 1: Update tests — remove parts tests, add rejection test**

Remove: `parses_parts_field`, `parts_defaults_to_single_unnamed_part_when_absent`, `parses_chord_column_in_parts`, `rejects_invalid_parts_token`, `rejects_invalid_parts_token_includes_chord_hint`.

Add:

```rust
#[test]
fn rejects_parts_field_in_metadata() {
    let content = "title = \"t\"\nauthor = \"a\"\nparts = notes: lyrics:\n";
    let err = parse_metadata(content, 0).unwrap_err();
    assert!(err.message.contains("unknown metadata field: parts"));
}
```

- [ ] **Step 2: Remove `parse_parts` function and `"parts"` match arm**

Delete `parts` variable, default parts vec, and `ParsedMetadata.parts` assignment.

- [ ] **Step 3: Run tests**

```bash
cargo test metadata_parser:: 2>&1
```

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/parser/metadata_parser.rs
git commit -m "refactor: remove parts field from metadata parser"
```

---

### Task 5: Wire orchestration in `parser/mod.rs`

**Files:**
- Modify: `src/parser/mod.rs`

- [ ] **Step 1: Update tests to three-section format**

Example helper for tests:

```rust
fn minimal_input(score: &str) -> String {
    format!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n[score]\n{score}"
    )
}
```

Update `parses_full_document`, `parses_two_named_parts`, etc. to use `[parts]` and assert on `doc.declarations` / `doc.tracks`.

- [ ] **Step 2: Implement parse wiring**

```rust
use section_splitter::{split_sections, SectionKind};

// Collect raw_metadata, raw_parts, raw_score from sections
// Error if any missing

let metadata = metadata_parser::parse_metadata(&meta_content, meta_offset)?;
let declarations = parts_parser::parse_parts(&parts_content, parts_offset)?;
let tracks = score::interleaved_parser::parse(&score_content, score_offset, &declarations)?;

Ok(ParsedDocument {
    filename: filename.to_string(),
    metadata,
    declarations,
    tracks,
})
```

- [ ] **Step 3: Run tests (expect failures in interleaved_parser until Task 7)**

```bash
cargo test parser:: 2>&1
```

- [ ] **Step 4: Commit**

```bash
git add src/parser/mod.rs
git commit -m "feat: wire [parts] section into document parser"
```

---

### Task 6: Refactor `desugar.rs` for `PartDecl`

**Files:**
- Modify: `src/desugar.rs`

- [ ] **Step 1: Update test helpers**

Replace `notes()`, `lyrics()`, `chord()` helpers with `PartDecl` builders:

```rust
fn decl(name: &str, kind: PartKind) -> PartDecl {
    PartDecl {
        abbreviation: name.to_string(),
        display_name: name.to_string(),
        kind,
    }
}
```

Update all tests to pass `&[PartDecl]` instead of `&[PartColumn]`.

- [ ] **Step 2: Refactor implementation**

```rust
use crate::ast::parsed::{flatten_score_line_slots, PartDecl, ScoreLineRole};

pub fn desugar_groups(
    groups: Vec<Vec<(String, usize)>>,
    declarations: &[PartDecl],
) -> Result<Vec<Vec<(String, usize)>>, JianPuError> {
    let slots = flatten_score_line_slots(declarations);
    // pad_implicit_ditto_group and desugar_group take slots.len() instead of parts.len()
    // column_type lookups use slot.role instead of PartColumn variant
    // error messages use declarations[slot.track_index].abbreviation
}
```

Replace `column_type(col: &PartColumn)` with `slot.role`. Ditto still matches by `ScoreLineRole` (same behavior as old notes/lyrics/chord column types).

- [ ] **Step 3: Run tests**

```bash
cargo test desugar:: 2>&1
```

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/desugar.rs
git commit -m "refactor: desugar using PartDecl score line slots"
```

---

### Task 7: Refactor `interleaved_parser.rs`

**Files:**
- Modify: `src/parser/score/interleaved_parser.rs`

- [ ] **Step 1: Change signature and context**

```rust
pub fn parse(
    content: &str,
    base_offset: usize,
    declarations: &[PartDecl],
) -> Result<Vec<ParsedTrack>, JianPuError>
```

Replace `parts: &[PartColumn]` in `BarGroupContext` with `declarations: &[PartDecl]` and `slots: &[ScoreLineSlot]`.

- [ ] **Step 2: Replace column action model**

Instead of separate notes/chord index vectors matched by name:

```rust
enum SlotAction {
    Chord { track_index: usize },
    Notes { track_index: usize },
    Lyrics { track_index: usize },
}
```

Build `slot_actions: Vec<SlotAction>` from `flatten_score_line_slots(declarations)`.

Accumulators: one entry per track index, storing either chord events or notes+syllables.

- [ ] **Step 3: Build `Vec<ParsedTrack>` at end**

```rust
declarations.iter().enumerate().map(|(i, decl)| match decl.kind {
    PartKind::Chord => ParsedTrack::Chord(ParsedChordTrack {
        abbreviation: decl.abbreviation.clone(),
        display_name: decl.display_name.clone(),
        events_per_measure: chord_acc[i].clone(),
    }),
    PartKind::Notes | PartKind::NotesWithLyrics => ParsedTrack::Notes(ParsedNotesTrack {
        abbreviation: decl.abbreviation.clone(),
        display_name: decl.display_name.clone(),
        score: ParsedScore { events: notes_acc[i].clone() },
        lyrics: lyrics_acc[i].clone().map(|s| ParsedLyrics { syllables: s }),
    }),
}).collect()
```

- [ ] **Step 4: Update all interleaved_parser tests**

Replace `notes_col`/`lyrics_col`/`chord_col` with `PartDecl` declarations. Example:

```rust
let declarations = vec![
    decl("main", PartKind::Chord),
    decl("Soprano", PartKind::Notes),
];
```

- [ ] **Step 5: Run tests**

```bash
cargo test interleaved_parser:: 2>&1
```

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/parser/score/interleaved_parser.rs
git commit -m "refactor: interleaved parser outputs ParsedTrack from PartDecl"
```

---

### Task 8: Refactor grouper

**Files:**
- Modify: `src/grouper.rs`
- Modify: `src/ast/grouped.rs` (add `GroupedTrack`)

- [ ] **Step 1: Add `GroupedTrack` to grouped.rs**

```rust
pub(crate) enum GroupedTrack {
    Chord(GroupedChordPart),
    Notes(GroupedPart),
}
```

- [ ] **Step 2: Refactor `group()`**

```rust
pub fn group(doc: ParsedDocument) -> Result<Score, JianPuError> {
    let metadata = doc.metadata;
    let mut grouped_tracks = Vec::new();
    for track in doc.tracks {
        grouped_tracks.push(match track {
            ParsedTrack::Notes(p) => GroupedTrack::Notes(group_notes_track(p)?),
            ParsedTrack::Chord(p) => GroupedTrack::Chord(group_chord_track(p)?),
        });
    }
    let measures = combiner::combine(&grouped_tracks)?;
    // ...
}
```

Rename `group_part` → `group_notes_track`, `group_chord_part` → `group_chord_track`. Use `abbreviation` for `GroupedPart.name: Option<String>` (Some unless empty string).

- [ ] **Step 3: Update grouper test fixtures**

Every inline document string needs `[parts]` section. Example migration pattern:

```rust
// Before:
"[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n"

// After:
"[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n"
```

- [ ] **Step 4: Run tests**

```bash
cargo test grouper:: 2>&1
```

- [ ] **Step 5: Commit**

```bash
git add src/grouper.rs src/ast/grouped.rs
git commit -m "refactor: grouper processes ParsedTrack aligned with declarations"
```

---

### Task 9: Refactor combiner

**Files:**
- Modify: `src/combiner.rs`

- [ ] **Step 1: Simplify combine signature**

```rust
pub fn combine(grouped_tracks: &[GroupedTrack]) -> Result<Vec<MultiPartMeasure>, JianPuError>
```

Remove `parts_ordering: &[PartColumn]` and separate `parts`/`chord_parts` vecs.

- [ ] **Step 2: Build PartRows 1:1 from grouped tracks**

```rust
for measure_idx in 0..expected_len {
    let mut part_rows = Vec::new();
    for track in grouped_tracks {
        match track {
            GroupedTrack::Notes(part) => {
                let measure = &part.measures[measure_idx];
                let syllables = /* distribute lyrics */;
                part_rows.push(PartRow::Notes(PartSlice {
                    name: part.name.clone(), // abbreviation only
                    notes: Notes { events: measure.notes.events.clone() },
                    lyrics: /* ... */,
                }));
            }
            GroupedTrack::Chord(part) => {
                part_rows.push(PartRow::Chord(part.measures[measure_idx].clone()));
            }
        }
    }
}
```

Ensure `PartSlice.name` receives `Some(abbreviation)` — never `display_name`.

- [ ] **Step 3: Update combiner tests**

Migrate fixtures to `[parts]` section format.

- [ ] **Step 4: Run tests**

```bash
cargo test combiner:: 2>&1
```

- [ ] **Step 5: Commit**

```bash
git add src/combiner.rs
git commit -m "refactor: combiner walks GroupedTrack in declaration order"
```

---

### Task 10: Fix remaining compile errors

**Files:**
- Modify: `src/layout/mod.rs`, `src/layout/layout_engine.rs`, `src/renderer.rs`, `src/pdf.rs`, `src/main.rs` (as needed)

- [ ] **Step 1: Search and fix references**

```bash
rg 'doc\.parts|doc\.chord_parts|metadata\.parts|PartColumn|ParsedPart|ParsedChordPart' src/
```

Update any remaining references. Layout/renderer tests: migrate inline `[metadata]…parts = …` strings to `[parts]` format.

- [ ] **Step 2: Run full test suite**

```bash
cargo test 2>&1
```

Expected: PASS (or only fixture migration failures remaining)

- [ ] **Step 3: Commit**

```bash
git add -A src/
git commit -m "refactor: update downstream code for ParsedTrack document model"
```

---

### Task 11: Migrate fixture files

**Files:**
- Modify: `demo.jianpu`
- Modify: `彌勒淨土鄉.jianpu`
- Modify: `tests/integration.rs`

- [ ] **Step 1: Update `demo.jianpu`**

```jianpu
[metadata]
title = "Feature Demo"
author = "Jianpu Generator"
row height = 20

[parts]
Melody = notes lyrics

[score]
...
```

- [ ] **Step 2: Update `彌勒淨土鄉.jianpu`**

```jianpu
[parts]
main = chord
Alto 1 & Tenor (A1&T) = notes lyrics
Alto 2 (A2) = notes lyrics
Soprano 1 (S1) = notes lyrics
Soprano 2 (S2) = notes lyrics
```

Remove `parts = …` line from metadata.

- [ ] **Step 3: Update `tests/integration.rs`**

```rust
fn basic_jianpu_input() -> &'static str {
    concat!(
        "[metadata]\n",
        "title = \"test score\"\n",
        "author = \"tester\"\n",
        "\n",
        "[parts]\n",
        "Melody = notes lyrics\n",
        "\n",
        "[score]\n",
        "(time=4/4 key=C4 bpm=120)\n",
        "1 2 3 4\n",
        "do re mi fa\n",
    )
}
```

- [ ] **Step 4: Run integration tests**

```bash
cargo test --test integration 2>&1
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add demo.jianpu 彌勒淨土鄉.jianpu tests/integration.rs
git commit -m "chore: migrate fixture files to [parts] section syntax"
```

---

### Task 12: Update `syntax.md`

**Files:**
- Modify: `syntax.md`

- [ ] **Step 1: Rewrite file structure section**

Document three required sections, `[parts]` syntax, abbreviation parentheses, validation rules, and that row labels use abbreviations.

Remove metadata `parts` field documentation.

- [ ] **Step 2: Update examples**

Replace all `parts = …` examples with `[parts]` blocks. Include `彌勒淨土鄉`-style multi-part example.

- [ ] **Step 3: Commit**

```bash
git add syntax.md
git commit -m "docs: document [parts] section syntax"
```

---

### Task 13: Final verification

- [ ] **Step 1: Full test suite**

```bash
cargo test 2>&1
```

Expected: all tests PASS

- [ ] **Step 2: Smoke-test CLI on migrated score**

```bash
cargo build --release
cargo run -- generate pdf 彌勒淨土鄉.jianpu --output /tmp/mile-test 2>&1
```

Expected: success; PDF starts with `%PDF`

- [ ] **Step 3: Run GitNexus change detection**

```bash
npx gitnexus detect_changes 2>&1
```

Review affected symbols match expected parser/grouper/combiner scope.

- [ ] **Step 4: Commit any remaining fixes**

```bash
git status
```

---

## Spec Coverage Checklist

| Spec requirement | Task |
|------------------|------|
| Required `[parts]` section | Task 2, 5 |
| Section order enforced | Task 2 |
| Dedicated `parts_parser.rs` | Task 3 |
| No parsing in metadata_parser | Task 4 |
| `PartDecl` with abbreviation + display_name | Task 1, 3 |
| Remove `PartColumn` | Task 1, 6, 7 |
| `ParsedTrack` unified model | Task 1, 7 |
| Score line slots for ditto | Task 1, 6 |
| Abbreviation used for row labels | Task 9 |
| Breaking: no metadata `parts` | Task 4 |
| No defaults when missing | Task 2, 3, 5 |
| Migrate fixtures | Task 11 |
| Update syntax.md | Task 12 |
| Legend deferred | N/A |
