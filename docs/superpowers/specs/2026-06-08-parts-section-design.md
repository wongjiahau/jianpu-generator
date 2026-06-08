# Parts Section Design

## Summary

Move score structure declaration out of `[metadata]` into a required `[parts]` section. Replace the flat `parts = notes:X lyrics:X …` metadata string and the `PartColumn` internal model with first-class track declarations that carry a display name, optional abbreviation, and content kind.

**Breaking change.** No backward compatibility with `parts = …` in metadata. No default when `[parts]` is missing.

---

## Motivation

Scores like `彌勒淨土鄉.jianpu` currently declare six logical rows in one metadata line:

```
parts = chord:main notes:A1&T lyrics:A1&T notes:A2 lyrics:A2 notes:S1 lyrics:S1 notes:S2 lyrics:S2
```

Problems:

1. **Metadata pollution** — layout fields (`max columns`, `row height`) sit beside score structure.
2. **Abbreviation-only names** — full voice names (`Alto 1 & Tenor`) are lost; only `A1&T` survives.
3. **Legacy column model** — `PartColumn` splits each voice into separate `notes:` and `lyrics:` tokens, forcing name-matching and a combiner that skips `Lyrics` entries.
4. **Split parsed output** — `ParsedDocument` holds separate `parts` and `chord_parts` vectors keyed back together via `PartColumn` ordering.

---

## Goals

1. **Dedicated `[parts]` section** — required; declares all score rows in author-friendly syntax.
2. **Display name + abbreviation** — `Alto 1 & Tenor (A1&T)` stores both; **abbreviation** used for row labels, error messages, and internal keys.
3. **First-class track model** — one declaration line → one `PartDecl` → one `ParsedTrack`; no `PartColumn` desugaring.
4. **Unchanged `[score]` authoring** — interleaved data lines per measure; line count derived from track kinds.
5. **Store display names** for future legend rendering.

## Non-Goals

- Legend / abbreviation key rendering (parse and store only).
- Changes to note, lyric, chord, or ditto token syntax within data lines.
- Mid-measure line omission rules (trailing implicit ditto unchanged).

---

## File Structure

A `.jianpu` file has **three required sections** in fixed order:

```
[metadata]
…

[parts]
…

[score]
…
```

| Section | Required | Content |
|---------|----------|---------|
| `[metadata]` | yes | `title`, `author`, optional layout fields |
| `[parts]` | yes | Track declarations (score structure) |
| `[score]` | yes | Interleaved measure groups |

### Section ordering

Sections must appear in order: `[metadata]` → `[parts]` → `[score]`. A `[parts]` section after `[score]` is an error. Duplicate sections are errors (same as today for metadata/score).

### Errors

| Situation | Result |
|-----------|--------|
| Missing `[parts]` | Parse error |
| Missing `[metadata]` or `[score]` | Parse error (unchanged) |
| `parts = …` in `[metadata]` | Parse error: `unknown metadata field: parts` |
| Wrong section order | Parse error |

---

## `[parts]` Syntax

One track per line. Blank lines are ignored.

```
<display-name> [(<abbreviation>)] = <column> [<column>…]
```

### Left-hand side

| Form | `display_name` | `abbreviation` |
|------|----------------|----------------|
| `Alto 1 & Tenor (A1&T)` | `Alto 1 & Tenor` | `A1&T` |
| `Melody` | `Melody` | `Melody` |
| `main` | `main` | `main` |

- Parentheses denote the **abbreviation** (short label). Text before `(` (trimmed) is the display name.
- When parentheses are omitted, `abbreviation` equals the full display name.
- `&`, spaces, CJK characters are allowed in display names.

### Right-hand side

Space-separated column tokens:

| Token | `PartKind` | Score lines per measure |
|-------|------------|-------------------------|
| `chord` | `Chord` | 1 |
| `notes` | `Notes` | 1 |
| `lyrics` | (modifier) | +1 when paired with `notes` on the same line |

Valid RHS patterns:

| Pattern | `PartKind` |
|---------|------------|
| `chord` | `Chord` |
| `notes` | `Notes` |
| `notes lyrics` | `NotesWithLyrics` |

### Validation rules

- `lyrics` without `notes` on the same line → error.
- Unknown RHS token → error.
- Duplicate `abbreviation` values across tracks → error.
- Empty abbreviation (e.g. `Name () = notes`) → error.
- Empty display name → error.
- At least one track must be declared (typically at least one `notes` or `notes lyrics` track).

### Examples

```
[parts]
main = chord
Alto 1 & Tenor (A1&T) = notes lyrics
Alto 2 (A2) = notes lyrics
Soprano 1 (S1) = notes lyrics
Soprano 2 (S2) = notes lyrics
```

```
[parts]
Melody = notes lyrics
```

```
[parts]
Violin = notes
Cello = notes
```

---

## Score Line Mapping

`[score]` measure groups are unchanged: optional directive line, then data lines in declaration order.

Each track expands to one or more **score lines** per measure:

| `PartKind` | Lines |
|------------|-------|
| `Chord` | chord |
| `Notes` | notes |
| `NotesWithLyrics` | notes, then lyrics |

Total data lines per measure = sum of score lines across all tracks.

For `彌勒淨土鄉`, five tracks → nine data lines per measure (1 chord + 4 × (notes + lyrics)).

### Positional mapping

```
(track 0: Chord)           → line 0
(track 1: NotesWithLyrics) → lines 1–2 (notes, lyrics)
(track 2: NotesWithLyrics) → lines 3–4
…
```

### Ditto and implicit padding

Ditto (`"`) and implicit trailing padding resolve by **score line role** (chord / notes / lyrics), not by flat `PartColumn` type tokens.

The parser builds a transient `ScoreLineSlot` list from declarations:

```rust
struct ScoreLineSlot {
    track_index: usize,
    role: ScoreLineRole,  // Chord | Notes | Lyrics
}
```

- Ditto copies from the closest preceding line with the same `ScoreLineRole` in the measure group.
- Implicit padding applies to trailing slots using the same role-based rules as today.
- Lyrics-specific rules (`_` no-lyrics marker, no implicit empty lyrics) unchanged.

---

## Internal Model

### Declaration

```rust
pub struct PartDecl {
    pub abbreviation: String,
    pub display_name: String,
    pub kind: PartKind,
}

pub enum PartKind {
    Chord,
    Notes,
    NotesWithLyrics,
}
```

Parsed directly from `[parts]` — no intermediate `PartColumn` list.

### Parsed document

```rust
pub struct ParsedDocument {
    pub filename: String,
    pub metadata: ParsedMetadata,
    pub declarations: Vec<PartDecl>,
    pub tracks: Vec<ParsedTrack>,
}

pub enum ParsedTrack {
    Chord(ParsedChordTrack),
    Notes(ParsedNotesTrack),
}

pub struct ParsedChordTrack {
    pub abbreviation: String,
    pub display_name: String,
    pub events_per_measure: Vec<Vec<ParsedChordEvent>>,
}

pub struct ParsedNotesTrack {
    pub abbreviation: String,
    pub display_name: String,
    pub score: ParsedScore,
    pub lyrics: Option<ParsedLyrics>,
}
```

`declarations.len() == tracks.len()`. Order matches `[parts]` declaration order.

### Metadata

```rust
pub struct ParsedMetadata {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: String,
    pub row_height: Option<u32>,
    pub max_columns: Option<u32>,
    pub label_width: Option<u32>,
    pub note_number_width: Option<u32>,
    // `parts` field REMOVED
}
```

### Deleted types / fields

- `PartColumn` enum
- `ParsedMetadata.parts`
- `ParsedDocument.chord_parts` (merged into `tracks`)
- `parse_parts()` in metadata parser
- Default `notes:` + `lyrics:` when parts absent

### Row labels (rendering)

Each score row shows the track **`abbreviation`** in `PartLabel` — never the full `display_name`.

| Track declaration | Row label |
|-------------------|-----------|
| `Alto 1 & Tenor (A1&T) = notes lyrics` | `A1&T` |
| `main = chord` | `main` |
| `Melody = notes lyrics` | `Melody` |

`display_name` is stored but not used in layout until legend rendering is implemented.

---

## Parser Architecture

`[parts]` parsing lives in a **dedicated module** — do not extend `metadata_parser.rs` or fold this logic into `interleaved_parser.rs`.

```
src/parser/
  mod.rs              ← orchestration only: split sections, call parsers, assemble document
  section_splitter.rs ← section headers only ([metadata], [parts], [score])
  metadata_parser.rs  ← [metadata] key = value fields only
  parts_parser.rs     ← [parts] track declarations only   ← NEW
  score/              ← [score] measure groups only
```

### Module boundaries

| Module | Parses | Must NOT |
|--------|--------|----------|
| `section_splitter.rs` | Section headers and raw section bodies | Parse field syntax inside sections |
| `metadata_parser.rs` | `title`, `author`, layout fields | Know about tracks, abbreviations, or `PartKind` |
| `parts_parser.rs` | Track lines → `Vec<PartDecl>` | Parse score data lines, metadata fields, or section headers |
| `score/interleaved_parser.rs` | Measure groups → `Vec<ParsedTrack>` | Parse `[parts]` declaration syntax |
| `parser/mod.rs` | — | Contain parsing logic beyond wiring the above together |

`parser/mod.rs` should only: split sections → call `parse_metadata` → call `parse_parts` → call `parse` (score) → build `ParsedDocument`.

Rationale: the old `parts = …` metadata string already bloated `metadata_parser.rs`. A separate `parts_parser.rs` keeps score-structure syntax isolated and testable on its own.

---

## Affected Components

### `section_splitter.rs`

Add `SectionKind::Parts`. Enforce section order: metadata, parts, score. Return raw `[parts]` body only — no field parsing here.

### `parts_parser.rs` (new, dedicated)

**Single responsibility:** parse `[parts]` section body → `Vec<PartDecl>`.

Responsibilities:

- Parse `Name (abbreviation) = columns` lines
- Validate RHS patterns and duplicate abbreviations
- Return structured `PartDecl` list

Unit tests for `[parts]` syntax belong in `parts_parser.rs` tests, not in metadata or score parser tests.

### `metadata_parser.rs`

Remove `parts` field handling and `parse_parts()`. Reject `parts` as unknown metadata key. No track-related types or imports.

### `parser/mod.rs`

Orchestration only:

- Require all three sections
- Call `parts_parser::parse_parts(content, offset)` → `declarations`
- Pass `declarations` to score parser
- Assemble `ParsedDocument` with `declarations` + `tracks`

### `desugar.rs`

Replace `parts: &[PartColumn]` with `declarations: &[PartDecl]`. Flatten to `ScoreLineSlot` list internally for ditto/padding. Error messages reference track `abbreviation` and role.

### `interleaved_parser.rs`

- Input: `&[PartDecl]`
- Output: `Vec<ParsedTrack>` (not split `Vec<ParsedPart>` + `Vec<ParsedChordPart>`)
- Column actions keyed by `track_index` + `ScoreLineRole`

### `grouper.rs`

Group each `ParsedTrack` independently. No `doc.metadata.parts` / `parts_ordering`.

### `combiner.rs`

Walk `declarations` and grouped tracks 1:1 to build `PartRow` per measure. Remove `parts_ordering: &[PartColumn]` and the loop that skips `Lyrics` entries. Pass `abbreviation` (not `display_name`) into `PartSlice.name` for layout.

### `syntax.md`

Update file structure, remove metadata `parts` field, document `[parts]` section and abbreviation syntax.

### Tests / fixtures

Migrate all inline test documents from `parts = …` in metadata to `[parts]` section. Update `彌勒淨土鄉.jianpu`.

---

## Complete Example

```
[metadata]
title = "彌勒淨土鄉"
subtitle = "調寄：快樂天堂"
author = "天然師尊 慈賜"
max columns = 42
row height = 20
note number width = 8

[parts]
main = chord
Alto 1 & Tenor (A1&T) = notes lyrics
Alto 2 (A2) = notes lyrics
Soprano 1 (S1) = notes lyrics
Soprano 2 (S2) = notes lyrics

[score]

(bpm=92 key=C4 time=4/4 label="Verse 1")
1 - - -
_5 _5 _5 =5 =5 _5 _3 _2 _3~
白陽旗旛在大道盛宏

6m - - -
_3 _1~1 - _0 =1 =1
昌花花
```

Rendered row labels: `main`, `A1&T`, `A2`, `S1`, `S2` — not the full display names.

---

## Future: Legend Rendering

`PartDecl.display_name` and `PartDecl.abbreviation` will drive a printed legend, e.g.:

```
A1&T — Alto 1 & Tenor
A2   — Alto 2
…
```

Not implemented in this change.

---

## Migration

All existing `.jianpu` files and test fixtures using `parts = …` in `[metadata]` must be updated to the new `[parts]` section. No deprecation period.
