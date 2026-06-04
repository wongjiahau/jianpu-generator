# Interleaved Syntax Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the separated `[score:Name]`/`[lyrics:Name]` section syntax with a single `[score]` section where notes and lyrics rows are interleaved per-measure, with column order declared by a `parts` metadata field.

**Architecture:** The parser's section splitter is simplified (no Lyrics variant, no named Score); metadata gains a `parts: Vec<PartColumn>` field; a new `interleaved_parser` module turns the single `[score]` block + `Vec<PartColumn>` into `Vec<ParsedPart>`. Everything downstream (grouper, combiner, renderer, pdf) is unchanged.

**Tech Stack:** Rust, Cargo; run tests with `cargo test`, build with `cargo build`.

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `src/error.rs` | Modify | Add `Location` enum (`Span` \| `Bar`), update `JianPuError` |
| `src/ast/parsed.rs` | Modify | Add `PartColumn` enum; add `parts` field to `ParsedMetadata` |
| `src/parser/metadata_parser.rs` | Modify | Parse `parts` field into `Vec<PartColumn>` |
| `src/parser/section_splitter.rs` | Modify | Remove `Lyrics` variant; remove `name` from `Score` variant |
| `src/parser/score/interleaved_parser.rs` | Create | Core interleaved parsing: groups → `Vec<ParsedPart>` |
| `src/parser/score/mod.rs` | Modify | Export `interleaved_parser` |
| `src/parser/mod.rs` | Modify | Wiring + update all tests to new syntax |
| `src/combiner.rs` | Modify | Update internal tests that call `parser::parse` with old syntax |
| `tests/integration.rs` | Modify | Update integration test to new syntax |
| `彌勒淨土鄉.jianpu` | Modify | Convert to new interleaved syntax |
| `demo.jianpu` | Modify | Simplify and convert to new syntax |

---

## Task 1: Update `error.rs` — add `Location` enum

**Files:**
- Modify: `src/error.rs`

- [ ] **Step 1: Write a failing test**

Add to `src/error.rs` at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_error_display_with_note() {
        let e = JianPuError::at_bar(3, 2, "too many beats");
        assert_eq!(format!("{}", e), "too many beats (bar 3, note 2)");
    }

    #[test]
    fn bar_error_display_whole_bar() {
        let e = JianPuError::at_bar(5, 0, "incomplete measure");
        assert_eq!(format!("{}", e), "incomplete measure (bar 5)");
    }

    #[test]
    fn span_error_display() {
        let e = JianPuError::new(Span::new(10, 20), "bad token");
        assert_eq!(format!("{}", e), "bad token (at byte 10-20)");
    }
}
```

- [ ] **Step 2: Run to confirm failure**

```
cargo test -p jianpu error::tests
```

Expected: compilation error (types don't exist yet).

- [ ] **Step 3: Rewrite `src/error.rs`**

```rust
#[derive(Debug, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }
}

#[derive(Debug, Clone)]
pub enum Location {
    Span(Span),
    Bar { bar: usize, note: usize },
}

#[derive(Debug)]
pub struct JianPuError {
    pub location: Location,
    pub message: String,
}

impl JianPuError {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self { location: Location::Span(span), message: message.into() }
    }

    pub fn at_bar(bar: usize, note: usize, message: impl Into<String>) -> Self {
        Self { location: Location::Bar { bar, note }, message: message.into() }
    }
}

impl std::fmt::Display for JianPuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.location {
            Location::Span(span) => write!(f, "{} (at byte {}-{})", self.message, span.start, span.end),
            Location::Bar { bar, note: 0 } => write!(f, "{} (bar {})", self.message, bar),
            Location::Bar { bar, note } => write!(f, "{} (bar {}, note {})", self.message, bar, note),
        }
    }
}

impl std::error::Error for JianPuError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_error_display_with_note() {
        let e = JianPuError::at_bar(3, 2, "too many beats");
        assert_eq!(format!("{}", e), "too many beats (bar 3, note 2)");
    }

    #[test]
    fn bar_error_display_whole_bar() {
        let e = JianPuError::at_bar(5, 0, "incomplete measure");
        assert_eq!(format!("{}", e), "incomplete measure (bar 5)");
    }

    #[test]
    fn span_error_display() {
        let e = JianPuError::new(Span::new(10, 20), "bad token");
        assert_eq!(format!("{}", e), "bad token (at byte 10-20)");
    }
}
```

The `thiserror` dependency can be removed from `Cargo.toml` if it was only used for `JianPuError`. Check with `grep -r "thiserror" src/` — if no other uses, remove from `Cargo.toml`.

- [ ] **Step 4: Fix compilation errors**

`JianPuError` previously had `pub span: Span`. Any code accessing `.span` directly needs updating. Search:

```
grep -rn "\.span" src/ --include="*.rs"
```

The only expected direct field accesses are in the old `#[error]` derive (now replaced). Confirm no test accesses `.span` on a `JianPuError` directly. The `Spanned<T>` struct (separate) still has `.span` and is unaffected.

- [ ] **Step 5: Run tests**

```
cargo test
```

Expected: all existing tests pass (or compile errors only from old `thiserror` derive that was removed).

- [ ] **Step 6: Commit**

```bash
git add src/error.rs Cargo.toml
git commit -m "refactor: replace JianPuError.span with Location enum supporting bar positions"
```

---

## Task 2: Add `PartColumn` and update `ParsedMetadata`

**Files:**
- Modify: `src/ast/parsed.rs`

- [ ] **Step 1: Add `PartColumn` enum and `parts` field**

In `src/ast/parsed.rs`, add below the existing `pub struct ParsedMetadata`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PartColumn {
    Notes { name: String },
    Lyrics { name: String },
}
```

Then add `parts` field to `ParsedMetadata`:

```rust
#[derive(Debug)]
pub struct ParsedMetadata {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: String,
    pub row_height: Option<u32>,
    pub max_columns: Option<u32>,
    pub label_width: Option<u32>,
    pub parts: Vec<PartColumn>,   // ← new field
}
```

- [ ] **Step 2: Fix compilation errors**

`ParsedMetadata` is constructed in `src/parser/metadata_parser.rs`. Add `parts: vec![]` as a placeholder so it compiles:

```rust
Ok(ParsedMetadata {
    title: ...,
    subtitle,
    author: ...,
    row_height,
    max_columns,
    label_width,
    parts: vec![],  // ← placeholder; Task 3 fills this properly
})
```

- [ ] **Step 3: Build to confirm no errors**

```
cargo build
```

- [ ] **Step 4: Commit**

```bash
git add src/ast/parsed.rs src/parser/metadata_parser.rs
git commit -m "feat: add PartColumn enum and parts field to ParsedMetadata"
```

---

## Task 3: Parse `parts` field in `metadata_parser.rs`

**Files:**
- Modify: `src/parser/metadata_parser.rs`

- [ ] **Step 1: Write failing tests**

Add to the `tests` module in `src/parser/metadata_parser.rs`:

```rust
#[test]
fn parses_parts_field() {
    use crate::ast::parsed::PartColumn;
    let content = "title = \"t\"\nauthor = \"a\"\nparts = notes:Alto1 lyrics:Alto1 notes:Alto2\n";
    let meta = parse_metadata(content, 0).unwrap();
    assert_eq!(meta.parts, vec![
        PartColumn::Notes { name: "Alto1".to_string() },
        PartColumn::Lyrics { name: "Alto1".to_string() },
        PartColumn::Notes { name: "Alto2".to_string() },
    ]);
}

#[test]
fn parts_defaults_to_single_unnamed_part_when_absent() {
    use crate::ast::parsed::PartColumn;
    let content = "title = \"t\"\nauthor = \"a\"\n";
    let meta = parse_metadata(content, 0).unwrap();
    assert_eq!(meta.parts, vec![
        PartColumn::Notes { name: "".to_string() },
        PartColumn::Lyrics { name: "".to_string() },
    ]);
}

#[test]
fn rejects_invalid_parts_token() {
    let content = "title = \"t\"\nauthor = \"a\"\nparts = invalid:foo\n";
    assert!(parse_metadata(content, 0).is_err());
}
```

- [ ] **Step 2: Run to confirm failure**

```
cargo test parser::metadata_parser::tests
```

Expected: `parses_parts_field` and `parts_defaults_to_single_unnamed_part_when_absent` fail.

- [ ] **Step 3: Implement `parts` parsing**

In `src/parser/metadata_parser.rs`, add a helper function:

```rust
fn parse_parts(value: &str, span: &Span) -> Result<Vec<crate::ast::parsed::PartColumn>, JianPuError> {
    use crate::ast::parsed::PartColumn;
    let mut columns = Vec::new();
    for token in value.split_whitespace() {
        let col = if let Some(name) = token.strip_prefix("notes:") {
            PartColumn::Notes { name: name.to_string() }
        } else if let Some(name) = token.strip_prefix("lyrics:") {
            PartColumn::Lyrics { name: name.to_string() }
        } else {
            return Err(JianPuError::new(
                span.clone(),
                format!("invalid parts token '{}': expected 'notes:<name>' or 'lyrics:<name>'", token),
            ));
        };
        columns.push(col);
    }
    Ok(columns)
}
```

In `parse_metadata`, add a `parts` local variable, handle the `"parts"` key, and set the default:

```rust
pub fn parse_metadata(content: &str, base_offset: usize) -> Result<ParsedMetadata, JianPuError> {
    use crate::ast::parsed::PartColumn;
    let mut title: Option<String> = None;
    let mut subtitle: Option<String> = None;
    let mut author: Option<String> = None;
    let mut row_height: Option<u32> = None;
    let mut max_columns: Option<u32> = None;
    let mut label_width: Option<u32> = None;
    let mut parts: Option<Vec<PartColumn>> = None;
    // ... existing parsing loop ...
    // Inside the match:
    "parts" => {
        parts = Some(parse_parts(value, &line_span)?);
    }
    // ...
    // Default when absent:
    let parts = parts.unwrap_or_else(|| vec![
        PartColumn::Notes  { name: "".to_string() },
        PartColumn::Lyrics { name: "".to_string() },
    ]);

    Ok(ParsedMetadata {
        title: ...,
        subtitle,
        author: ...,
        row_height,
        max_columns,
        label_width,
        parts,
    })
}
```

- [ ] **Step 4: Run tests**

```
cargo test parser::metadata_parser::tests
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/parser/metadata_parser.rs
git commit -m "feat: parse 'parts' metadata field into Vec<PartColumn>"
```

---

## Task 4: Simplify `section_splitter.rs`

**Files:**
- Modify: `src/parser/section_splitter.rs`

- [ ] **Step 1: Update the enum and parser**

Replace the entire `SectionKind` enum:

```rust
#[derive(Debug, PartialEq)]
pub enum SectionKind {
    Metadata,
    Score,
}
```

In `split_sections`, replace the `match` arm for section headers:

```rust
current_kind = Some(match kind_str {
    "metadata" => SectionKind::Metadata,
    "score" => SectionKind::Score,
    _ => {
        return Err(JianPuError::new(
            Span::new(byte_offset, byte_offset + line.len()),
            format!("unknown section: [{}]", kind_str),
        ))
    }
});
```

- [ ] **Step 2: Update existing tests in the file**

Remove or rewrite tests that reference `SectionKind::Lyrics` or `SectionKind::Score { name: ... }`. Replace with:

```rust
#[test]
fn splits_metadata_and_score() {
    let input = "[metadata]\ntitle = \"hi\"\n\n[score]\n1 2 3\n";
    let sections = split_sections(input).unwrap();
    assert_eq!(sections.len(), 2);
    assert_eq!(sections[0].kind, SectionKind::Metadata);
    assert_eq!(sections[1].kind, SectionKind::Score);
    assert_eq!(sections[1].content.trim(), "1 2 3");
}

#[test]
fn rejects_lyrics_section() {
    let input = "[metadata]\ntitle=\"t\"\n[lyrics]\nfoo\n";
    assert!(split_sections(input).is_err());
}

#[test]
fn rejects_named_score_section() {
    let input = "[score:Soprano]\n1 2 3\n";
    assert!(split_sections(input).is_err());
}

#[test]
fn rejects_unknown_section() {
    let input = "[unknown]\nfoo\n";
    assert!(split_sections(input).is_err());
}

#[test]
fn content_offset_points_past_header_line() {
    let input = "[metadata]\ntitle = \"hi\"\n";
    let sections = split_sections(input).unwrap();
    assert_eq!(sections[0].content_offset, 11);
}

#[test]
fn handles_header_with_no_content() {
    let input = "[metadata]\ntitle = \"hi\"\n\n[score]\n";
    let sections = split_sections(input).unwrap();
    assert_eq!(sections.len(), 2);
    assert_eq!(sections[1].kind, SectionKind::Score);
    assert_eq!(sections[1].content.trim(), "");
}
```

- [ ] **Step 3: Run tests**

```
cargo test parser::section_splitter::tests
```

Expected: all pass.

- [ ] **Step 4: Fix compilation errors in `parser/mod.rs`**

`parser/mod.rs` references `SectionKind::Score { name }` and `SectionKind::Lyrics { name }`. These will fail to compile. Leave `parser/mod.rs` broken for now — it will be fixed in Task 7.

- [ ] **Step 5: Commit (with expected compile errors)**

```bash
git add src/parser/section_splitter.rs
git commit -m "refactor: remove Lyrics and named Score variants from SectionKind"
```

---

## Task 5: Create `src/parser/score/interleaved_parser.rs`

**Files:**
- Create: `src/parser/score/interleaved_parser.rs`

This is the core new module. It takes a `[score]` section's raw content plus `&[PartColumn]` and returns `Vec<ParsedPart>`.

- [ ] **Step 1: Write failing tests**

Create the file with only the tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::parsed::PartColumn;

    fn notes_col(name: &str) -> PartColumn { PartColumn::Notes { name: name.to_string() } }
    fn lyrics_col(name: &str) -> PartColumn { PartColumn::Lyrics { name: name.to_string() } }

    #[test]
    fn single_unnamed_part_no_lyrics() {
        let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4\n";
        let parts = vec![notes_col("")];
        let result = parse(content, &parts).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, None);
        assert!(result[0].lyrics.is_none());
        // 3 events: TimeSignatureChange + KeyChange + BpmChange are directives in first part
        // plus 4 notes = 7 events
        assert_eq!(result[0].score.events.len(), 7);
    }

    #[test]
    fn single_part_with_lyrics() {
        let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4\ndo re mi fa\n";
        let parts = vec![notes_col(""), lyrics_col("")];
        let result = parse(content, &parts).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].lyrics.is_some());
        assert_eq!(result[0].lyrics.as_ref().unwrap().syllables.len(), 4);
    }

    #[test]
    fn two_parts_two_bars() {
        let content = concat!(
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "5 6 7 1\n",
            "\n",
            "1 2 3 4\n",
            "5 6 7 1\n",
        );
        let parts = vec![notes_col("Soprano"), notes_col("Alto")];
        let result = parse(content, &parts).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, Some("Soprano".to_string()));
        assert_eq!(result[1].name, Some("Alto".to_string()));
        // Each part: 3 directive events (first bar) + 4+4 notes = 11
        assert_eq!(result[0].score.events.len(), 11);
        assert_eq!(result[1].score.events.len(), 8);
    }

    #[test]
    fn rejects_wrong_line_count_in_group() {
        let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4\n";
        // parts expects 2 lines but group only has 1
        let parts = vec![notes_col(""), lyrics_col("")];
        let err = parse(content, &parts).unwrap_err();
        assert!(matches!(err.location, crate::error::Location::Bar { bar: 1, .. }));
    }

    #[test]
    fn rejects_overfull_measure() {
        let content = "(time=4/4 key=C4 bpm=120)\n1 2 3 4 5\n";
        let parts = vec![notes_col("")];
        let err = parse(content, &parts).unwrap_err();
        assert!(matches!(err.location, crate::error::Location::Bar { bar: 1, note: 5 }));
    }

    #[test]
    fn rejects_underfull_measure() {
        let content = "(time=4/4 key=C4 bpm=120)\n1 2 3\n";
        let parts = vec![notes_col("")];
        let err = parse(content, &parts).unwrap_err();
        assert!(matches!(err.location, crate::error::Location::Bar { bar: 1, note: 0 }));
    }

    #[test]
    fn directive_row_is_optional() {
        // Second bar has no directive row
        let content = concat!(
            "(time=4/4 key=C4 bpm=120)\n1 2 3 4\n",
            "\n",
            "5 6 7 1\n",
        );
        let parts = vec![notes_col("")];
        let result = parse(content, &parts).unwrap();
        // bar 1: 3 directives + 4 notes; bar 2: 4 notes
        assert_eq!(result[0].score.events.len(), 11);
    }

    #[test]
    fn time_sig_change_updates_beat_tracking() {
        let content = concat!(
            "(time=4/4 key=C4 bpm=120)\n1 2 3 4\n",
            "\n",
            "(time=3/4)\n1 2 3\n",
        );
        let parts = vec![notes_col("")];
        let result = parse(content, &parts).unwrap();
        assert!(result[0].score.events.len() > 0);
    }

    #[test]
    fn rejects_unknown_directive() {
        let content = "(foo=bar)\n1 2 3 4\n";
        let parts = vec![notes_col("")];
        assert!(parse(content, &parts).is_err());
    }

    #[test]
    fn key_directive_parses_flat() {
        let content = "(time=4/4 key=Bb4 bpm=120)\n1 2 3 4\n";
        let parts = vec![notes_col("")];
        let result = parse(content, &parts).unwrap();
        use crate::ast::parsed::{Accidental, ScoreEvent};
        let key_event = result[0].score.events.iter().find(|e| matches!(&e.value, ScoreEvent::KeyChange(_)));
        assert!(key_event.is_some());
        if let ScoreEvent::KeyChange(kc) = &key_event.unwrap().value {
            assert_eq!(kc.note.accidental, Accidental::Flat);
        }
    }

    #[test]
    fn key_directive_parses_sharp() {
        let content = "(time=4/4 key=F#3 bpm=120)\n1 2 3 4\n";
        let parts = vec![notes_col("")];
        let result = parse(content, &parts).unwrap();
        use crate::ast::parsed::{Accidental, ScoreEvent};
        let key_event = result[0].score.events.iter().find(|e| matches!(&e.value, ScoreEvent::KeyChange(_)));
        if let ScoreEvent::KeyChange(kc) = &key_event.unwrap().value {
            assert_eq!(kc.note.accidental, Accidental::Sharp);
        }
    }
}
```

- [ ] **Step 2: Run to confirm failure**

```
cargo test parser::score::interleaved_parser::tests
```

Expected: compile error (module doesn't exist yet).

- [ ] **Step 3: Implement the module**

Create `src/parser/score/interleaved_parser.rs` with the following complete implementation:

```rust
use crate::ast::parsed::{
    Accidental, KeyChange, Note, NoteName, ParsedLyrics, ParsedPart, ParsedScore,
    PartColumn, ScoreEvent,
};
use crate::error::{JianPuError, Span, Spanned};
use crate::utils::tokenize_lyrics;
use crate::parser::score::{token_parser, tokenizer};

pub fn parse(content: &str, parts: &[PartColumn]) -> Result<Vec<ParsedPart>, JianPuError> {
    let groups = collect_groups(content);

    // Build ordered list of notes-column names.
    let notes_names: Vec<String> = parts.iter().filter_map(|p| match p {
        PartColumn::Notes { name } => Some(name.clone()),
        _ => None,
    }).collect();

    if notes_names.is_empty() {
        return Err(JianPuError::new(
            Span::new(0, 0),
            "parts declaration has no 'notes:' columns",
        ));
    }

    // For each column in parts, map to its index in notes_names.
    enum ColAction { Notes(usize), Lyrics(usize) }

    let col_actions: Vec<ColAction> = parts.iter().map(|p| match p {
        PartColumn::Notes { name } => {
            let idx = notes_names.iter().position(|n| n == name).unwrap();
            ColAction::Notes(idx)
        }
        PartColumn::Lyrics { name } => {
            let idx = notes_names.iter().position(|n| n == name)
                .unwrap_or_else(|| panic!("lyrics column '{}' has no matching notes column", name));
            ColAction::Lyrics(idx)
        }
    }).collect();

    let mut events_acc: Vec<Vec<Spanned<ScoreEvent>>> = vec![Vec::new(); notes_names.len()];
    // None = this notes-column has no lyrics column declared.
    let mut syllables_acc: Vec<Option<Vec<crate::ast::parsed::Syllable>>> =
        vec![None; notes_names.len()];

    // Mark which notes-columns have a paired lyrics column.
    for p in parts {
        if let PartColumn::Lyrics { name } = p {
            if let Some(idx) = notes_names.iter().position(|n| n == name) {
                syllables_acc[idx] = Some(Vec::new());
            }
        }
    }

    let mut time_num: u8 = 4;
    let mut time_den: u8 = 4;

    for (bar_idx, group_lines) in groups.iter().enumerate() {
        let bar = bar_idx + 1;

        let (directive_events, data_lines) = split_directive(group_lines, bar)?;

        // Update time sig from directives (for beat validation).
        for e in &directive_events {
            if let ScoreEvent::TimeSignatureChange { numerator, denominator } = &e.value {
                time_num = *numerator;
                time_den = *denominator;
            }
        }

        // Validate line count.
        if data_lines.len() != parts.len() {
            return Err(JianPuError::at_bar(bar, 0, format!(
                "expected {} lines (one per parts column), got {}",
                parts.len(), data_lines.len()
            )));
        }

        // Prepend directive events to first notes part.
        if !directive_events.is_empty() {
            events_acc[0].extend(directive_events);
        }

        let beats_expected = beats_per_measure(time_num, time_den);

        for (i, line) in data_lines.iter().enumerate() {
            match col_actions[i] {
                ColAction::Notes(idx) => {
                    let tokens = tokenizer::tokenize(line, 0);
                    let events = token_parser::parse_tokens(tokens)?;
                    validate_beats(&events, beats_expected, bar)?;
                    events_acc[idx].extend(events);
                }
                ColAction::Lyrics(idx) => {
                    let syllables = tokenize_lyrics(line);
                    syllables_acc[idx].as_mut().unwrap().extend(syllables);
                }
            }
        }
    }

    // Assemble output.
    let mut result = Vec::new();
    for (i, name) in notes_names.iter().enumerate() {
        result.push(ParsedPart {
            name: if name.is_empty() { None } else { Some(name.clone()) },
            score: ParsedScore { events: std::mem::take(&mut events_acc[i]) },
            lyrics: syllables_acc[i].take().map(|s| ParsedLyrics { syllables: s }),
        });
    }

    Ok(result)
}

fn collect_groups(content: &str) -> Vec<Vec<String>> {
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            if !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
        } else {
            current.push(trimmed);
        }
    }
    if !current.is_empty() {
        groups.push(current);
    }

    groups
}

fn split_directive(
    lines: &[String],
    bar: usize,
) -> Result<(Vec<Spanned<ScoreEvent>>, &[String]), JianPuError> {
    if lines.first().map(|l| l.starts_with('(')).unwrap_or(false) {
        let directive_line = &lines[0];
        if !directive_line.ends_with(')') {
            return Err(JianPuError::at_bar(bar, 0, "directive row must end with ')'"));
        }
        let events = parse_directive_line(directive_line)?;
        Ok((events, &lines[1..]))
    } else {
        Ok((Vec::new(), lines))
    }
}

fn parse_directive_line(line: &str) -> Result<Vec<Spanned<ScoreEvent>>, JianPuError> {
    let inner = &line[1..line.len() - 1]; // strip ( and )
    let mut events = Vec::new();

    for token in inner.split_whitespace() {
        let span = Span::new(0, token.len()); // approximate; directive errors use span

        let event = if let Some(rest) = token.strip_prefix("bpm=") {
            let bpm = rest.parse::<u32>().map_err(|_| {
                JianPuError::new(span.clone(), format!("invalid bpm value: {}", rest))
            })?;
            ScoreEvent::BpmChange(bpm)
        } else if let Some(rest) = token.strip_prefix("key=") {
            parse_key_value(rest, span.clone())?
        } else if let Some(rest) = token.strip_prefix("time=") {
            parse_time_value(rest, span.clone())?
        } else {
            return Err(JianPuError::new(span, format!("unknown directive: '{}'", token)));
        };

        events.push(Spanned::new(event, span));
    }

    Ok(events)
}

fn parse_key_value(value: &str, span: Span) -> Result<ScoreEvent, JianPuError> {
    let mut chars = value.chars().peekable();

    let name_char = chars.next().ok_or_else(|| {
        JianPuError::new(span.clone(), "expected note name after 'key='".to_string())
    })?;

    let name = match name_char {
        'A' => NoteName::A, 'B' => NoteName::B, 'C' => NoteName::C,
        'D' => NoteName::D, 'E' => NoteName::E, 'F' => NoteName::F,
        'G' => NoteName::G,
        _ => return Err(JianPuError::new(span.clone(), format!("invalid note name: '{}'", name_char))),
    };

    let accidental = match chars.peek() {
        Some('b') => { chars.next(); Accidental::Flat }
        Some('#') => { chars.next(); Accidental::Sharp }
        _ => Accidental::Natural,
    };

    let octave_str: String = chars.collect();
    let octave = octave_str.parse::<u8>().map_err(|_| {
        JianPuError::new(span.clone(), format!("invalid octave in 'key={}': expected number", value))
    })?;

    Ok(ScoreEvent::KeyChange(KeyChange { note: Note { name, octave, accidental } }))
}

fn parse_time_value(value: &str, span: Span) -> Result<ScoreEvent, JianPuError> {
    let parts: Vec<&str> = value.split('/').collect();
    if parts.len() != 2 {
        return Err(JianPuError::new(span.clone(), format!("invalid time signature: '{}'", value)));
    }
    let numerator = parts[0].parse::<u8>().map_err(|_| {
        JianPuError::new(span.clone(), format!("invalid time numerator: '{}'", parts[0]))
    })?;
    let denominator = parts[1].parse::<u8>().map_err(|_| {
        JianPuError::new(span.clone(), format!("invalid time denominator: '{}'", parts[1]))
    })?;
    if denominator == 0 {
        return Err(JianPuError::new(span, "time denominator cannot be zero".to_string()));
    }
    Ok(ScoreEvent::TimeSignatureChange { numerator, denominator })
}

fn beats_per_measure(num: u8, den: u8) -> u32 {
    (num as u32) * (16 / den as u32)
}

fn validate_beats(
    events: &[Spanned<ScoreEvent>],
    expected: u32,
    bar: usize,
) -> Result<(), JianPuError> {
    let mut total = 0u32;
    let mut note_idx = 0usize;

    for e in events {
        let beats = match &e.value {
            ScoreEvent::Note(n) => n.duration,
            ScoreEvent::Rest(r) => r.duration,
            ScoreEvent::Extension => 4,
            _ => 0,
        };
        if beats > 0 {
            note_idx += 1;
            total += beats;
            if total > expected {
                return Err(JianPuError::at_bar(bar, note_idx, format!(
                    "note exceeds measure boundary: measure has {} quarter-beats, cumulative is now {}",
                    expected, total
                )));
            }
        }
    }

    if total < expected {
        return Err(JianPuError::at_bar(bar, 0, format!(
            "incomplete measure: expected {} quarter-beats, got {}",
            expected, total
        )));
    }

    Ok(())
}
```

- [ ] **Step 4: Run tests**

```
cargo test parser::score::interleaved_parser::tests
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/parser/score/interleaved_parser.rs
git commit -m "feat: add interleaved_parser module for new score syntax"
```

---

## Task 6: Export `interleaved_parser` from `score/mod.rs`

**Files:**
- Modify: `src/parser/score/mod.rs`

- [ ] **Step 1: Add the export**

```rust
pub mod interleaved_parser;
pub mod token_parser;
pub mod tokenizer;
```

- [ ] **Step 2: Build**

```
cargo build
```

- [ ] **Step 3: Commit**

```bash
git add src/parser/score/mod.rs
git commit -m "chore: export interleaved_parser from score module"
```

---

## Task 7: Rewire `parser/mod.rs` and update all parser tests

**Files:**
- Modify: `src/parser/mod.rs`
- Modify: `src/combiner.rs` (update internal tests that use `parser::parse` with old syntax)

- [ ] **Step 1: Rewrite `src/parser/mod.rs`**

Replace the entire file:

```rust
use crate::ast::parsed::{ParsedDocument, PartColumn};
use crate::error::{JianPuError, Span};

pub mod lyrics;
pub mod metadata_parser;
pub mod score;
pub mod section_splitter;

pub fn parse(input: &str, filename: &str) -> Result<ParsedDocument, JianPuError> {
    use section_splitter::{split_sections, SectionKind};

    let sections = split_sections(input)?;
    let doc_span = Span::new(0, input.len());

    let mut raw_metadata: Option<(String, usize)> = None;
    let mut raw_score: Option<(String, usize)> = None;

    for section in sections {
        match section.kind {
            SectionKind::Metadata => {
                if raw_metadata.is_some() {
                    return Err(JianPuError::new(doc_span.clone(), "duplicate [metadata] section"));
                }
                raw_metadata = Some((section.content, section.content_offset));
            }
            SectionKind::Score => {
                if raw_score.is_some() {
                    return Err(JianPuError::new(doc_span.clone(), "duplicate [score] section"));
                }
                raw_score = Some((section.content, section.content_offset));
            }
        }
    }

    let (meta_content, meta_offset) = raw_metadata
        .ok_or_else(|| JianPuError::new(doc_span.clone(), "missing [metadata] section"))?;
    let (score_content, _score_offset) = raw_score
        .ok_or_else(|| JianPuError::new(doc_span, "missing [score] section"))?;

    let metadata = metadata_parser::parse_metadata(&meta_content, meta_offset)?;
    let parts_decl = metadata.parts.clone();
    let parts = score::interleaved_parser::parse(&score_content, &parts_decl)?;

    Ok(ParsedDocument {
        filename: filename.to_string(),
        metadata,
        parts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(notes: &str, lyrics: Option<&str>) -> String {
        let parts_line = match lyrics {
            Some(_) => "parts = notes: lyrics:",
            None => "parts = notes:",
        };
        let lyrics_row = lyrics.map(|l| format!("{}\n", l)).unwrap_or_default();
        format!(
            "[metadata]\ntitle = \"t\"\nauthor = \"a\"\n{}\n\n[score]\n(time=4/4 key=C4 bpm=120)\n{}\n{}\n",
            parts_line, notes, lyrics_row
        )
    }

    #[test]
    fn parses_full_document() {
        let input = concat!(
            "[metadata]\ntitle = \"hello world\"\nauthor = \"foo\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 _3 _4\n你好wo rld\n"
        );
        let doc = parse(input, "test.jianpu").unwrap();
        assert_eq!(doc.metadata.title, "hello world");
        assert_eq!(doc.metadata.author, "foo");
        assert_eq!(doc.parts.len(), 1);
        // 3 directive events + 4 notes = 7
        assert_eq!(doc.parts[0].score.events.len(), 7);
        // 4 syllables
        assert_eq!(doc.parts[0].lyrics.as_ref().unwrap().syllables.len(), 4);
    }

    #[test]
    fn rejects_unknown_section() {
        let input = "[unknown]\nfoo\n";
        assert!(parse(input, "test.jianpu").is_err());
    }

    #[test]
    fn rejects_duplicate_score_section() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\n\n",
            "[score]\n5 6 7 1\n",
        );
        assert!(parse(input, "test.jianpu").is_err());
    }

    #[test]
    fn rejects_missing_metadata_section() {
        let input = "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\n";
        assert!(parse(input, "test.jianpu").is_err());
    }

    #[test]
    fn parses_two_named_parts() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes:Soprano notes:Alto\n\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "5 6 7 1\n",
        );
        let doc = parse(input, "test.jianpu").unwrap();
        assert_eq!(doc.parts.len(), 2);
        assert_eq!(doc.parts[0].name, Some("Soprano".to_string()));
        assert_eq!(doc.parts[1].name, Some("Alto".to_string()));
        assert!(doc.parts[0].lyrics.is_none());
        assert!(doc.parts[1].lyrics.is_none());
    }

    #[test]
    fn single_unnamed_part_remains_compatible() {
        let input = concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes: lyrics:\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\na b c d\n"
        );
        let doc = parse(input, "test.jianpu").unwrap();
        assert_eq!(doc.parts.len(), 1);
        assert_eq!(doc.parts[0].name, None);
        assert!(doc.parts[0].lyrics.is_some());
    }
}
```

- [ ] **Step 2: Update `src/combiner.rs` internal tests**

In `src/combiner.rs`, the `make_two_part_score` helper uses the old syntax. Replace it:

```rust
fn make_two_part_score(soprano: &str, alto: &str) -> Vec<MultiPartMeasure> {
    let input = format!(
        concat!(
            "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes:Soprano notes:Alto\n\n",
            "[score]\n(time=4/4 key=C4 bpm=120)\n{}\n{}\n",
        ),
        soprano, alto
    );
    let doc = parser::parse(&input, "test.jianpu").unwrap();
    grouper::group(doc).unwrap().measures
}
```

Also update the `rejects_parts_with_different_measure_counts` test in `combiner.rs`:

```rust
#[test]
fn rejects_parts_with_different_measure_counts() {
    let input = concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\nparts = notes:Soprano notes:Alto\n\n",
        "[score]\n",
        "(time=4/4 key=C4 bpm=120)\n",
        "1 2 3 4\n",
        "5 6 7 1 5 6 7 1\n",  // Alto has 2 measures worth in one line — beat validator will reject this
    );
    // This should fail either at the interleaved parser (beat overflow) or combiner
    let result = parser::parse(input, "test.jianpu");
    assert!(result.is_err());
}
```

Note: in the old test, both parts each had one measure in separate `[score:*]` sections. Now both parts' notes must be in the same group row-by-row. To test mismatched measure counts, use two groups with different part counts — or test at the combiner level differently. The simplest replacement is shown above.

- [ ] **Step 3: Run all tests**

```
cargo test
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/parser/mod.rs src/combiner.rs
git commit -m "feat: wire interleaved_parser into parse() and update all tests to new syntax"
```

---

## Task 8: Update integration test and example files

**Files:**
- Modify: `tests/integration.rs`
- Modify: `彌勒淨土鄉.jianpu`
- Modify: `demo.jianpu`

- [ ] **Step 1: Update integration test**

Replace `tests/integration.rs`:

```rust
use std::fs;
use std::process::Command;

#[test]
fn full_pipeline_produces_pdf() {
    let input = concat!(
        "[metadata]\n",
        "title = \"test score\"\n",
        "author = \"tester\"\n",
        "parts = notes: lyrics:\n",
        "\n",
        "[score]\n",
        "(time=4/4 key=C4 bpm=120)\n",
        "1 2 3 4\n",
        "do re mi fa\n",
    );

    let input_path = "/tmp/test_score.jianpu";
    let output_path = "/tmp/test_score.pdf";

    fs::write(input_path, input).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_jianpu"))
        .arg(input_path)
        .arg("--output")
        .arg(output_path)
        .status()
        .unwrap();

    assert!(status.success(), "jianpu command failed");

    let pdf_bytes = fs::read(output_path).unwrap();
    assert!(pdf_bytes.starts_with(b"%PDF"), "output is not a valid PDF");

    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(output_path);
}
```

- [ ] **Step 2: Run integration test**

```
cargo test --test integration
```

Expected: pass.

- [ ] **Step 3: Update `demo.jianpu`**

Replace `demo.jianpu` with a simplified demo that showcases the new syntax:

```
[metadata]
title = "Feature Demo"
author = "Jianpu Generator"
row height = 20
parts = notes:main lyrics:main

[score]
(time=4/4 key=C4 bpm=100)
1 - 2 0
hold two tie two

3~ 3 2 1
one slur ing up

1~ 3~ 5 2
note do re mi

_1 _2 _3 _4 _5 _6 _7 _1
fa sol la ti do

(key=G4)
5 6 7 .1
up hi dn low

(bpm=60)
1 2 3 4
slow down slow down

(time=3/4)
1 2 3
wal tz time
```

- [ ] **Step 4: Convert `彌勒淨土鄉.jianpu` to new syntax**

The file has 25 measures in 4/4 time. Each measure becomes a blank-line-separated group. The lyrics syllables are distributed per bar (each bar gets exactly the syllables consumed by its non-continuation note events). The conversion below was derived by counting note-consumption per bar:

```
[metadata]
title = "彌勒淨土鄉村"
author = "Jianpu Generator"
row height = 20
parts = notes:main lyrics:main

[score]
(bpm=92 key=C4 time=4/4)
_5 _5 _5 =5 =5 _5 _3 _2 _3~
白陽旗旛在大道盛宏

_3 _1~1~1 _0 =1 =1
昌花花

2~_2 _3 _4 =3 =3 _2~_1
草擺動道音歌-

2~2~2~2
唱

_1 _1 _1 =1 =1 _1 6. _6~
蝴蝶傳播著真理飛

_6 _5~5~5 _0 =3 =2
翔沒有

1.~_1. =1 =6. _3~=3 _2~=2 _1
人應該永遠流

2~2~2~2
浪

_5 _5 _5 =5 =5 _5 =3 =3 _2 _3~
彌勒佛張開口吞掉了骯

_3 _1~1~1 _0 =1 =1
髒煩惱

2~_2 _3 _4 =3 =3 _2~_1
都裝進祂的大肚

2~2~2~2
量

_1 _1 _1 =1 =1 _1 6. _6~
濟公帶領著我們向

_6 _5~5~5 _0 =3 =2
上更高

_1 6. _1 _3 _2 _1~_2
更遠更充滿理-

1~1~1 _0 =3 =5
想告訴

6~_6 =6 =6 _6 _5 =3 =2~_2
你一個神秘的地

3~3~3 _0 =3 =5
方一個

_6 _6 _6 =1 =1 _7 _6 _5~_6
母子相聚的快樂天-

5~5~5 _0 =3 =5
堂跟佛

6~_6 =6 =6 _6 _5 =3 =2~_2
仙一樣自在安-

_6~=6 _6~=6 _5 4~_4 =1 =1
詳有歡有笑當

_5~=5 _5~=5 _1 _5 3~_1
然也會有奔忙

_4 _4 _4 _3 _2~=2 _2~=2 =1 =7.~
我們擁有母娘的慈

_7 _1 - - -
光
```

Verify: syllables per bar = 9+3+7+1+8+3+6+1+10+3+7+1+8+3+7+3+7+3+9+3+7+6+6+8+1 = 130, which matches the original total.

- [ ] **Step 5: Build and run the converted files**

```bash
cargo run -- 彌勒淨土鄉.jianpu --output /tmp/miledianjing.pdf && echo "OK"
cargo run -- demo.jianpu --output /tmp/demo.pdf && echo "OK"
```

Expected: both produce PDFs without errors.

- [ ] **Step 6: Run full test suite**

```
cargo test
```

Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add tests/integration.rs 彌勒淨土鄉.jianpu demo.jianpu
git commit -m "feat: convert all .jianpu files and integration test to new interleaved syntax"
```

---

## Self-Review Checklist

- **Spec coverage:**
  - ✅ `parts` metadata field → Task 3
  - ✅ Blank-line groups = one bar → Task 5 (`collect_groups`)
  - ✅ Optional `(...)` directive row per group → Task 5 (`split_directive`)
  - ✅ `time=`, `key=`, `bpm=` directive syntax → Task 5 (`parse_directive_line`)
  - ✅ Directives global, persist until overridden → Task 5 (`time_num/time_den` tracking)
  - ✅ `PartColumn` struct variants with named fields → Task 2
  - ✅ `BarPosition` error type → Task 1 (`Location::Bar`)
  - ✅ Remove Lyrics/named Score sections → Task 4
  - ✅ No backward compat → Tasks 4, 7, 8 all remove old syntax
  - ✅ Downstream pipeline unchanged → Tasks 5-7 only touch parser layer
- **Types consistent:** `PartColumn::Notes { name }` used uniformly across Tasks 2, 3, 5, 7, 8.
- **No placeholders:** All steps have concrete code.
