# Multi-Part Score Support

## Overview

Add support for multiple named parts (voices) in a single `.jianpu` file. Parts are stacked vertically on the same system with aligned bar lines — conventional choral score layout. Each part has its own notes row and optional lyrics row. A part label appears on the left of each system line.

## Syntax

### Named sections

Parts are identified by name in the section header:

```
[score:Soprano]
bpm=120 1=C4 4/4
1 2 3 4 |

[lyrics:Soprano]
do re mi fa

[score:Alto]
1 7 6 5 |

[lyrics:Alto]
do ti la sol
```

Plain `[score]` / `[lyrics]` (no name) remain valid — treated as a single unnamed part. Existing files are fully compatible except for the `cell_size` → `cell size` rename (see Metadata).

### Pairing rules

- A `[lyrics:X]` paired with the preceding `[score:X]` of the same name.
- A `[lyrics:X]` with no matching `[score:X]` → **error: orphan lyrics section**.
- A `[score:X]` with no `[lyrics:X]` is valid (lyrics are optional per part).
- Duplicate `[score:X]` for the same name → error.
- Duplicate `[lyrics:X]` for the same name → error.

### Directives

Directives (`bpm=`, `1=`, time signature) are only valid in the first part's score section. Any directive found in a subsequent part → **parse error**. When a directive is present on a measure, the layout emits it on every part's row group for that measure (conventional choral practice).

### Metadata

`cell_size` renamed to `cell size` (space case, consistent with new fields). New optional field:

```
label width = 60
```

Default: 40pt. Labels wider than the margin are cropped. No text measurement — fixed margin by design.

## Pipeline

```
Parser  →  ParsedDocument (Vec<ParsedPart>)
Grouper →  Vec<GroupedPart>  (per-part, independent)
Combiner → Score { metadata, measures: Vec<MultiPartMeasure> }
Layout  →  Vec<Page>
Renderer → PDF
```

## Data Structures

### Parser layer (`ast/parsed.rs`)

```rust
pub struct ParsedScore {
    pub events: Vec<Spanned<ScoreEvent>>,
}

pub struct ParsedLyrics {
    pub syllables: Vec<Syllable>,
}

pub struct ParsedPart {
    pub name: Option<String>,
    pub score: ParsedScore,
    pub lyrics: Option<ParsedLyrics>,
}

pub struct ParsedDocument {
    pub filename: String,
    pub metadata: ParsedMetadata,
    pub parts: Vec<ParsedPart>,
}

pub struct ParsedMetadata {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: String,
    pub cell_size: Option<u32>,
    pub label_width: Option<u32>,
}
```

### Section splitter (`parser/section_splitter.rs`)

```rust
pub enum SectionKind {
    Metadata,
    Score { name: Option<String> },
    Lyrics { name: Option<String> },
}
```

The splitter parses the name from the header line only. All pairing/matching/orphan-detection logic lives in `parser/mod.rs`.

### Grouper output (intermediate, not in `ast/grouped.rs`)

The grouper produces this intermediate type (can live in `grouper.rs`):

```rust
pub struct GroupedMeasure {
    pub time_signature: Option<TimeSignature>,
    pub bpm: Option<u32>,
    pub key: Option<KeyChange>,
    pub notes: Notes,
}

pub struct GroupedPart {
    pub name: Option<String>,
    pub measures: Vec<GroupedMeasure>,
    pub lyrics: Lyrics,
}
```

Lyrics remain a flat list at this stage. The combiner distributes syllables into `PartSlice.lyrics` by counting note events per measure and slicing accordingly.

### Combiner output (`ast/grouped.rs`)

```rust
pub struct Notes {
    pub events: Vec<NoteEvent>,
}

pub struct Lyrics {
    pub syllables: Vec<Syllable>,
}

pub struct PartSlice {
    pub name: Option<String>,
    pub notes: Notes,
    pub lyrics: Option<Lyrics>,
}

pub struct MultiPartMeasure {
    pub time_signature: Option<TimeSignature>,  // Some if user wrote it on this measure
    pub bpm: Option<u32>,                       // Some if user wrote it on this measure
    pub key: Option<KeyChange>,                 // Some if user wrote it on this measure
    pub parts: Vec<PartSlice>,
}

pub struct Score {
    pub metadata: Metadata,
    pub measures: Vec<MultiPartMeasure>,
}
```

Directives are `Option` — `Some` when the user explicitly wrote them on that measure, `None` otherwise. No deduplication: user intent is preserved verbatim.

### Metadata in grouped AST (`ast/grouped.rs`)

```rust
pub struct Metadata {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: String,
    pub cell_size: u32,        // default 24
    pub label_width: u32,      // default 40
}
```

## Layer Responsibilities

| Layer | Responsibility |
|---|---|
| Section splitter | Split input into tagged sections; parse name from header. Nothing else. |
| Parser | Pair score/lyrics by name; detect orphan lyrics; build `Vec<ParsedPart>`; parse metadata. |
| Grouper | Process each `ParsedPart` independently into `GroupedPart { name, measures, lyrics }`. Validate no directives in non-first parts. |
| Combiner | Zip `Vec<GroupedPart>` → `Vec<MultiPartMeasure>` by measure index. Structural transformation only. Error if parts have different measure counts. |
| Layout | Compute per-measure column width as max across all parts. Wrap all parts at the same measure boundaries. Emit directive elements into every part's row group when directive is present. Emit `PartLabel` elements using `label_width` as fixed indent. |
| Renderer | Draw `GridElement`s verbatim. Handles new `PartLabel` grid content variant. No cross-part awareness. |

## New GridContent Variant

```rust
pub enum GridContent {
    // ... existing variants ...
    PartLabel { text: String },
}
```

Emitted by layout at the start of each system line per part, left-aligned, vertically centered across the part's rows. Only emitted when the part has a name (`Some`).

## Backward Compatibility

- Existing single-part files work unchanged except `cell_size` → `cell size` (breaking rename).
- Unnamed single part: no `PartLabel` emitted, no `label_width` margin applied.
