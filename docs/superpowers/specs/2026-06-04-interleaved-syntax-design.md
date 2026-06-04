# Interleaved Syntax Design

**Date:** 2026-06-04

## Summary

Replace the current separated `[score:Name]` / `[lyrics:Name]` section syntax with a single interleaved `[score]` section where notes and lyrics rows alternate in the order declared by a `parts` metadata field.

## Motivation

The old syntax forces the author to maintain two distant sections in sync — one for notes, one for lyrics. The new syntax keeps notes and lyrics for the same bar visually adjacent, making it easier to author and review.

## New Syntax

### Metadata

```
[metadata]
title = "Mary Had a Little Lamb"
author = "Demo"
parts = notes:Soprano lyrics:Soprano notes:Alto
```

`parts` declares the column order for the `[score]` section. Each token is `notes:<name>` or `lyrics:<name>`. Parts with no lyrics simply omit the `lyrics:<name>` entry.

### Score

```
[score]
(time=4/4 key=C4 bpm=92)
3 2 1 2
Ma rry have a
5 4 3 4

3 3 3 -
li ttle lamb
5 5 5 -
```

- Each blank-line-separated group is exactly **one bar (measure)**.
- The first line of a group may optionally be a `(...)` directive row containing `time=`, `key=`, and/or `bpm=` changes. Directives apply to all subsequent bars until overridden.
- The remaining lines map 1:1 to the `parts` declaration in order.
- The `|` bar separator token is no longer used.

### Validation

- A `notes` row must contain note values that sum to exactly one full measure under the current time signature. Error if too few or too many beats.
- A group must have exactly as many non-directive rows as there are entries in `parts`. Error if the count mismatches.

### Error Reporting

Errors from the interleaved parser report position as bar number and note index (both 1-indexed), not byte spans:

> `bar 3, note 2: note value exceeds remaining beats in measure`

Errors from directive parsing (inside `(...)`) continue to use byte spans.

## Affected Components

### `section_splitter.rs`

Remove `SectionKind::Lyrics` and the `name` field from `SectionKind::Score`. Only two variants remain:

```rust
pub enum SectionKind {
    Metadata,
    Score,
}
```

### `metadata_parser.rs`

Add a `parts` field to parsed metadata. Introduce:

```rust
pub enum PartColumn {
    Notes { name: String },
    Lyrics { name: String },
}
```

`parts` is parsed from space-separated `notes:<name>` / `lyrics:<name>` tokens. If absent, defaults to `[Notes { name: "" }, Lyrics { name: "" }]` (unnamed single part, preserving existing behavior for files without a `parts` key).

### `score/interleaved_parser.rs` (new)

Owns the logic of turning a raw `[score]` string + `Vec<PartColumn>` into `Vec<ParsedPart>`.

Algorithm:
1. Split the score text by blank lines into groups.
2. For each group, if the first line matches `(...)`, parse it as directives and consume it.
3. The remaining lines map 1:1 to the `parts` columns in declaration order.
4. Error (with `BarPosition`) if a group has a line count mismatch with `parts`.
5. `Notes { name }` rows are fed to the existing `score::tokenizer` + `score::token_parser`.
6. `Lyrics { name }` rows are fed to the existing `lyrics::tokenizer`.
7. Validate each notes row fills exactly one measure under the current time signature.
8. Accumulate events per part name, yielding one `ParsedPart` per unique `Notes` entry.

### `error.rs`

Add a `BarPosition` location type for interleaved parser errors:

```rust
pub struct BarPosition {
    pub bar: usize,  // 1-indexed
    pub note: usize, // 1-indexed; 0 if error applies to the whole bar
}
```

`JianPuError` gains a new location variant so it can hold either a `Span` or a `BarPosition`.

### `parser/mod.rs`

Updated orchestration:
1. `section_splitter` → `Metadata` + `Score`
2. `metadata_parser` → metadata including `Vec<PartColumn>`
3. `score::interleaved_parser::parse(score_content, &parts)` → `Vec<ParsedPart>`
4. Return `ParsedDocument`

### Unchanged

`combiner`, `grouper`, `layout`, `renderer`, `pdf` — no changes needed.

## Non-Goals

- Backward compatibility with old `[score:Name]` / `[lyrics:Name]` syntax.
- The input grouping (one group = one bar) does not affect rendered layout; the grouper decides row breaks independently.
