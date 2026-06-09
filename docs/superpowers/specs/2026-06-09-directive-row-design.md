# Directive Row Design

**Date:** 2026-06-09
**Status:** Approved

## Overview

Move time signature and BPM rendering from the notes part rows to a dedicated **directive row** that sits **above** the section-label / bar-number row. The directive row appears only when time or BPM changes on a measure (same emission rules as today), and only when at least one measure on the current system line has such a change.

No input syntax changes — this is a layout and rendering change only.

## Motivation

Today, time signature and BPM labels are emitted on every notes part row within a measure. This duplicates directives in multi-part scores and places them inline with note heads rather than in a clear metadata band above the music.

The desired layout separates tempo/meter metadata from note content:

```
┌─────────────────────────────────────────┐
│  [directive row]  4/4   ♩=120           │  only when time/BPM change on this line
│  [meta row]       Verse 1  (or bar "1") │  section label or bar number
│  Soprano │ 1  2  3  4                   │  part rows
│  Alto    │ 5  6  7  1                   │
└─────────────────────────────────────────┘
```

When no time/BPM change appears on a system line, the directive row is omitted entirely (no blank row).

## Current Behavior

| Element | Row (relative to row-group start) | Notes |
|---------|-----------------------------------|-------|
| Section label (`label=`) | `+0` | Replaces bar number when present |
| Bar number | `+0` | Suppressed when section label present |
| Time signature | `+1` per notes part | Repeated on each notes row in multi-part scores |
| BPM | `+1` per notes part | Repeated on each notes row in multi-part scores |
| Part labels | `+1` per part row, column 0 | — |
| Notes / lyrics / chords | `+1` per part row | Start after directive columns |

## New Behavior

### Row tiers

Each system line (row group) may have up to three tiers above part content:

| Tier | Row offset (when directive row present) | Row offset (when absent) | Contents |
|------|----------------------------------------|--------------------------|----------|
| Directive row | `+0` | *(omitted)* | Time signature, BPM |
| Meta row | `+1` | `+0` | Section label or bar number |
| Part rows | `+2` onward | `+1` onward | Part labels, bar line, notes, lyrics, chords |

### Emission rules (unchanged semantics)

- Time signature emitted only when `measure.time_signature.is_some()` (grouper `time_sig_changed`).
- BPM emitted only when `measure.bpm.is_some()` (grouper `bpm_changed`).
- Unchanged values are not repeated on subsequent measures.
- Unchanged values are not repeated after line wrap.
- Mid-line changes (e.g. `(time=3/4)` on bar 5) still emit a label at that measure's column.

### Multi-part scores

Time signature and BPM are emitted **once per measure** on the directive row, not once per notes part row. The test `two_part_layout_emits_directives_on_both_parts_rows` will be updated to expect a single emission.

### Horizontal placement (unchanged)

- Time signature: 2 columns, centered stack (numerator / rule / denominator).
- BPM: next 2 columns, `♩=N` text.
- Position: note-start column (`label_cols + 1`), same as today.
- `compute_prefix_width` logic unchanged — notes still start after directive columns.

### Left bar line

The vertical bar line continues to start at the first part row (`part_row_base`). Its height spans `effective_row_group_height - 1` rows (directive row is outside the bar line span).

## Lookahead Algorithm

A system line may contain multiple measures. A mid-line time/BPM change requires the directive row for the **entire** line (the row exists at the changing measure's column; other measures' columns in that row are empty).

At each system-line start (`is_line_start == true`):

1. Walk measures from the current index, accumulating `prefix_width + measure_width` until wrap would occur.
2. Set `line_has_directive_row` if any measure in that range has `time_signature.is_some() || bpm.is_some()`.
3. Compute offsets:
   - `directive_row_offset = current_row_offset` when `line_has_directive_row`, else unused.
   - `meta_row_offset = current_row_offset + (if line_has_directive_row { 1 } else { 0 })`.
   - `part_row_base = meta_row_offset + 1`.
4. `effective_row_group_height = row_group_height + (if line_has_directive_row { 1 } else { 0 })`.

Use `effective_row_group_height` for:

- Row-group commit (`height_in_rows`)
- `current_row_offset` advancement after line wrap
- Bottom system bar row position
- Page-break decisions

## Pagination

Replace the fixed `row_groups_per_page` estimate with a running row count per page:

- When committing a row group, add `effective_row_group_height` to the page's used-row tally.
- Start a new page when the next row group would exceed the usable row budget (`usable_height / row_height - header_rows - footer_rows`).

This handles variable row-group heights correctly when some lines have a directive row and others do not.

## Implementation

### `src/layout/layout_engine.rs`

Primary changes:

1. Add `line_has_directive_row` state, computed via lookahead at line start.
2. Refactor `emit_measure_directives`:
   - Emit on `directive_row_offset` (not per notes part row).
   - Emit once per measure (remove per-part loop for directives).
3. Shift `emit_section_label` and bar number to `meta_row_offset`.
4. Shift part labels, bar line, and note content to `part_row_base`.
5. Use `effective_row_group_height` in wrap, commit, and page-break logic.

### `src/layout/mod.rs`

Update layout tests:

- Time/BPM row positions (directive row vs notes row).
- Single emission in multi-part scores.
- Mid-line time signature change with directive row on the line.
- Unchanged directives after wrap (no repeat, no extra row if no changes on wrapped line).
- Section label + time/BPM on same measure: directive row above meta row.
- Bar number / bar line positions with and without directive row.

### `src/renderer.rs`

No renderer changes expected — Y coordinates derive from grid row indices.

### `syntax.md`

Add a brief rendering note under [Directive lines](#directive-lines) describing the directive row placement. No syntax rule changes.

## Visual Examples

### First measure with all directives

```
(bpm=92 key=C4 time=4/4 label="Verse 1")
```

```
  4/4  ♩=92          ← directive row
  Verse 1            ← meta row (bar number suppressed)
A1&T │ 1 2 3 4       ← part rows
```

### Continuation measure (unchanged time/BPM)

```
1 2 3 4
```

```
  2                  ← meta row (bar number only; no directive row)
A1&T │ 1 2 3 4
```

### Mid-line time change

```
(time=4/4 …)  1 2 3 4  |  (time=3/4)  1 2 3
```

```
  4/4  ♩=120    3/4           ← directive row spans the line
  1             2              ← meta row
A1&T │ 1 2 3 4  │ 1 2 3
```

## Non-Goals

- Key signature (`key=C4`) is not rendered in the directive row.
- No changes to `(...)` directive line syntax or parsing.
- No label wrapping or truncation.
- No MIDI or audio effects.

## Testing Checklist

- [ ] Time/BPM on directive row when changed on first measure of a line.
- [ ] No directive row when no time/BPM changes appear on a system line.
- [ ] Mid-line `(time=3/4)` — directive row present for whole line, label at correct column.
- [ ] Multi-part: one time/BPM emission per change, not per notes part.
- [ ] Line wrap: unchanged directives do not repeat; wrapped line omits directive row when no changes on that line.
- [ ] Section label + time/BPM: directive row above section label row.
- [ ] Bar line height and note head positions correct with and without directive row.
- [ ] Renderer SVG tests still pass (content unchanged, positions may shift).
