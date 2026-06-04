# System Bars Design

## Summary

Add a left vertical bar line and a bottom horizontal bar to every system line (row group) in the rendered output. This applies to all scores regardless of part count.

## Motivation

Currently each system line has only right-side bar lines (one per measure). Adding a left bar and a bottom bar gives each system line a clear visual boundary, matching conventional sheet music practice.

## Changes

### 1. `src/layout/types.rs` — new `GridContent` variant

```rust
HorizontalBar { from_column: u32, to_column: u32 },
```

Mirrors the `DurationUnderlines` pattern: the renderer ignores the element's `column` position and uses `from_column`/`to_column` directly to compute absolute x coordinates.

### 2. `src/layout/mod.rs` — two new emission points

**Left bar** — inside the existing `is_line_start` block, emit a `BarLine` at:
- `column = label_cols`
- `row = current_row_offset + 1`
- `height_in_rows = bar_height` (same value used for measure bar lines)

Reuses the existing `BarLine` variant; no new type needed.

**Bottom bar** — just before each row group flush (wrap-triggered and final end-of-loop flush), push:
- `GridContent::HorizontalBar { from_column: 0, to_column: current_col }`
- `row = current_row_offset + row_group_height`
- `vertical_alignment = VerticalAlignment::Top` so `y = base_y` lands at the exact bottom of the row group

### 3. `src/renderer.rs` — one new match arm

```rust
GridContent::HorizontalBar { from_column, to_column } => {
    let x1 = *from_column as f32 * column_width + PAGE_MARGIN;
    let x2 = *to_column as f32 * column_width + PAGE_MARGIN;
    let y = base_y;
    elements.push_str(&format!(
        r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="black" stroke-width="1"/>"#,
        x1, y, x2, y
    ));
}
```

## Scope

- No changes to the AST, parser, grouper, combiner, or PDF layers.
- No new metadata fields or config options.
- Single-part and multi-part scores both get the bars unconditionally.
