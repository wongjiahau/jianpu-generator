# System Bars Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a left vertical bar line and a bottom horizontal bar to every system line (row group) in the SVG output.

**Architecture:** Add `GridContent::HorizontalBar` for the bottom bar (reusing `GridContent::BarLine` for the left bar). Layout emits both at system-line boundaries. Renderer gains one new match arm.

**Tech Stack:** Rust, SVG output via string formatting.

---

### Task 1: Add `HorizontalBar` variant to `GridContent` + renderer placeholder

**Files:**
- Modify: `src/layout/types.rs`
- Modify: `src/renderer.rs`

Adding the variant without a renderer match arm breaks compilation (Rust requires exhaustive matches). The placeholder keeps the build green while the real renderer arm is written in Task 2.

- [ ] **Step 1: Write a compile-check test in `src/layout/mod.rs` (in the `tests` module)**

Add this test inside the existing `#[cfg(test)] mod tests` block:

```rust
#[test]
fn horizontal_bar_variant_exists() {
    let _ = GridContent::HorizontalBar { from_column: 0, to_column: 10 };
}
```

- [ ] **Step 2: Run test — expect compile failure**

```bash
cargo test horizontal_bar_variant_exists 2>&1 | head -20
```

Expected: compile error mentioning `HorizontalBar` is not a known variant.

- [ ] **Step 3: Add the variant to `GridContent` in `src/layout/types.rs`**

In the `GridContent` enum, add after `PartLabel`:

```rust
HorizontalBar { from_column: u32, to_column: u32 },
```

- [ ] **Step 4: Add placeholder match arm in `src/renderer.rs`**

In `render_page`, inside the `match &element.content` block, add after `GridContent::PartLabel { .. }`:

```rust
GridContent::HorizontalBar { .. } => {
    // rendered in Task 2
}
```

- [ ] **Step 5: Run test — expect pass**

```bash
cargo test horizontal_bar_variant_exists 2>&1 | tail -5
```

Expected: `test ... ok`

- [ ] **Step 6: Commit**

```bash
git add src/layout/types.rs src/renderer.rs src/layout/mod.rs
git commit -m "feat: add HorizontalBar GridContent variant with renderer placeholder"
```

---

### Task 2: Implement `HorizontalBar` rendering in `src/renderer.rs`

**Files:**
- Modify: `src/renderer.rs`

- [ ] **Step 1: Write the failing renderer test**

Add this test inside the existing `#[cfg(test)] mod tests` block in `src/renderer.rs`:

```rust
#[test]
fn horizontal_bar_renders_horizontal_line() {
    use crate::layout::types::*;
    use nonempty::nonempty;
    let page = Page {
        header: Header { title: "t".to_string(), subtitle: None, author: "a".to_string() },
        footer: Footer { page: 1, total: 1 },
        page_width_pt: A4_W,
        row_groups: vec![RowGroup {
            height_in_rows: 4,
            width_in_columns: 16,
            elements: nonempty![GridElement {
                position: GridPosition { column: 0, row: 6 },
                horizontal_alignment: HorizontalAlignment::Left,
                vertical_alignment: VerticalAlignment::Top,
                content: GridContent::HorizontalBar { from_column: 0, to_column: 16 },
            }],
        }],
    };
    let svgs = render(&[page], 24);
    // column_width = (595 - 2*25) / 16 = 34.0625
    // x1 = 0*34.0625 + 25 = 25.0, x2 = 16*34.0625 + 25 = 570.0
    // y = PAGE_MARGIN + row*row_height = 25 + 6*24 = 169.0 (VerticalAlignment::Top → y = base_y)
    assert!(
        svgs[0].contains(r#"x1="25.0" y1="169.0" x2="570.0" y2="169.0""#),
        "expected horizontal line at y=169.0 spanning full content width;\nSVG snippet: {}",
        &svgs[0][..svgs[0].len().min(800)]
    );
}
```

- [ ] **Step 2: Run — expect fail**

```bash
cargo test horizontal_bar_renders_horizontal_line 2>&1 | tail -10
```

Expected: FAIL — the placeholder arm emits nothing.

- [ ] **Step 3: Replace the placeholder with the real renderer arm in `src/renderer.rs`**

Replace:
```rust
GridContent::HorizontalBar { .. } => {
    // rendered in Task 2
}
```

With:
```rust
GridContent::HorizontalBar { from_column, to_column } => {
    let _ = x;
    let x1 = *from_column as f32 * column_width + PAGE_MARGIN;
    let x2 = *to_column as f32 * column_width + PAGE_MARGIN;
    let line_y = base_y;
    elements.push_str(&format!(
        r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="black" stroke-width="1"/>"#,
        x1, line_y, x2, line_y
    ));
}
```

Note: `let _ = x;` suppresses the unused-variable warning since we compute x1/x2 from `from_column`/`to_column` directly, mirroring how `DurationUnderlines` ignores `x`. `base_y` is already computed from `element.position.row` and `row_height`.

- [ ] **Step 4: Run — expect pass**

```bash
cargo test horizontal_bar_renders_horizontal_line 2>&1 | tail -5
```

Expected: `test ... ok`

- [ ] **Step 5: Run all tests — expect no regressions**

```bash
cargo test 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/renderer.rs
git commit -m "feat: render HorizontalBar as SVG horizontal line"
```

---

### Task 3: Emit left bar and bottom bar in layout

**Files:**
- Modify: `src/layout/mod.rs`

Three code locations change:
1. Before the main `for measure in &score.measures` loop — hoist `bar_height` to a named constant.
2. Inside the loop, in the `is_line_start` block — emit left `BarLine`.
3. Before each row-group flush (wrap flush + final flush) — emit `HorizontalBar`.

- [ ] **Step 1: Write failing layout tests**

Add these tests inside the existing `#[cfg(test)] mod tests` block in `src/layout/mod.rs`:

```rust
#[test]
fn left_bar_line_emitted_at_start_of_first_system_line() {
    let score = make_score("1 2 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    // label_cols=0 (unnamed single part), header_rows=2 → row = 2+1 = 3
    let left_bars: Vec<_> = pages.iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(&e.content, GridContent::BarLine { .. }) && e.position.column == 0)
        .collect();
    assert_eq!(left_bars.len(), 1, "expected one left bar for a single system line");
    assert_eq!(left_bars[0].position.row, 3, "left bar should be at row header_rows+1 = 3");
}

#[test]
fn left_bar_line_emitted_for_each_system_line_on_wrap() {
    // First measure: 4 (directives) + 16 (notes) + 1 (bar) = 21 cols
    // Second measure: 0 + 16 + 1 = 17 cols; 21+17=38 > 28 → wraps → 2 system lines
    let score = make_score("1 2 3 4 5 6 7 1", "a b c d e f g h");
    let pages = layout(&score, 300.0, A4_HEIGHT);
    let left_bars: Vec<_> = pages.iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(&e.content, GridContent::BarLine { .. }) && e.position.column == 0)
        .collect();
    assert_eq!(left_bars.len(), 2, "expected one left bar per system line");
}

#[test]
fn bottom_bar_emitted_at_end_of_system_line() {
    let score = make_score("1 2 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let bottom_bars: Vec<_> = pages.iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(&e.content, GridContent::HorizontalBar { .. }))
        .collect();
    assert_eq!(bottom_bars.len(), 1, "expected one bottom bar for a single system line");
    // row_group_height = 4*1 = 4; row = header_rows + row_group_height = 2+4 = 6
    assert_eq!(bottom_bars[0].position.row, 6, "bottom bar row should be current_row_offset + row_group_height");
    if let GridContent::HorizontalBar { from_column, to_column } = &bottom_bars[0].content {
        assert_eq!(*from_column, 0);
        // 4 (directives) + 16 (notes) + 1 (bar) = 21
        assert_eq!(*to_column, 21, "to_column should equal current_col at flush time");
    } else {
        panic!("expected HorizontalBar");
    }
}

#[test]
fn bottom_bar_emitted_for_each_system_line_on_wrap() {
    let score = make_score("1 2 3 4 5 6 7 1", "a b c d e f g h");
    let pages = layout(&score, 300.0, A4_HEIGHT);
    let bottom_bars: Vec<_> = pages.iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(&e.content, GridContent::HorizontalBar { .. }))
        .collect();
    assert_eq!(bottom_bars.len(), 2, "expected one bottom bar per system line");
}
```

- [ ] **Step 2: Run — expect all four new tests to fail**

```bash
cargo test "left_bar_line_emitted\|bottom_bar_emitted" 2>&1 | tail -15
```

Expected: all 4 FAIL.

- [ ] **Step 3: Hoist `bar_height` before the loop in `src/layout/mod.rs`**

Find the line (currently inside the `for measure` loop, near the bar line emission):
```rust
let bar_height = 1 + (num_parts - 1) * 4;
```

Remove it from inside the loop. Add it right after `let row_group_height: u32 = 4 * num_parts;` near the top of `layout()`:

```rust
let row_group_height: u32 = 4 * num_parts;
let bar_height: u32 = 1 + (num_parts - 1) * 4;
```

- [ ] **Step 4: Emit the left bar in the `is_line_start` block**

Find the existing `is_line_start` block in `layout/mod.rs`:

```rust
// Emit part labels at start of each system line
if is_line_start && has_named_parts {
```

Add the left bar emission BEFORE this block:

```rust
// Left system bar at start of each system line
if is_line_start {
    current_elements.push(GridElement {
        position: GridPosition { column: label_cols, row: current_row_offset + 1 },
        horizontal_alignment: HorizontalAlignment::Left,
        vertical_alignment: VerticalAlignment::Center,
        content: GridContent::BarLine { height_in_rows: bar_height },
    });
}
// Emit part labels at start of each system line
if is_line_start && has_named_parts {
```

- [ ] **Step 5: Emit the bottom bar before the wrap-triggered flush**

Find the wrap-triggered flush block. It starts with:
```rust
if current_col + prefix_width + measure_width > columns_per_row {
    // Flush open beam buffers for all parts
    for (part_idx, beam_buf) in per_part_beam_buffer.iter_mut().enumerate() {
```

After the beam-buffer flush and tie/chain resets (just before `if let Some(elements) = nonempty::NonEmpty::from_vec(...)`), add:

```rust
// Bottom system bar for the row group being flushed
current_elements.push(GridElement {
    position: GridPosition { column: 0, row: current_row_offset + row_group_height },
    horizontal_alignment: HorizontalAlignment::Left,
    vertical_alignment: VerticalAlignment::Top,
    content: GridContent::HorizontalBar { from_column: 0, to_column: current_col },
});

if let Some(elements) = nonempty::NonEmpty::from_vec(std::mem::take(&mut current_elements)) {
```

- [ ] **Step 6: Emit the bottom bar before the final flush**

Find the final flush at the end of `layout()`:

```rust
// Flush remaining elements
if let Some(elements) = nonempty::NonEmpty::from_vec(std::mem::take(&mut current_elements)) {
```

Replace with:

```rust
// Bottom system bar for the last row group
if !current_elements.is_empty() {
    current_elements.push(GridElement {
        position: GridPosition { column: 0, row: current_row_offset + row_group_height },
        horizontal_alignment: HorizontalAlignment::Left,
        vertical_alignment: VerticalAlignment::Top,
        content: GridContent::HorizontalBar { from_column: 0, to_column: current_col },
    });
}
if let Some(elements) = nonempty::NonEmpty::from_vec(std::mem::take(&mut current_elements)) {
```

- [ ] **Step 7: Run the four new tests — expect pass**

```bash
cargo test "left_bar_line_emitted\|bottom_bar_emitted" 2>&1 | tail -15
```

Expected: all 4 pass.

- [ ] **Step 8: Run all tests — expect no regressions**

```bash
cargo test 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 9: Commit**

```bash
git add src/layout/mod.rs
git commit -m "feat: emit left bar and bottom bar for each system line"
```
