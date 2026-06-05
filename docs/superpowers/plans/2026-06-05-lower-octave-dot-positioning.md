# Lower Octave Dot Positioning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix lower octave dots to render below duration underlines using a shared vertical slot system, following JianPu convention.

**Architecture:** Add `underline_count: u8` to the `LowerOctaveDots` GridContent variant so the renderer knows which vertical slot to start from. Layout populates this field from the note's duration. The renderer replaces its ad-hoc dot-y formula with the same slot formula already used by `DurationUnderlines`.

**Tech Stack:** Rust, SVG rendering. Run tests with `cargo test`.

---

## File Map

| File | Change |
|------|--------|
| `src/layout/types.rs` | Add `underline_count: u8` to `LowerOctaveDots` variant |
| `src/layout/mod.rs` | Populate `underline_count` from note duration; fix `VerticalAlignment` from `Bottom` to `Top`; update existing test |
| `src/renderer.rs` | Replace dot-y formula with slot formula; update existing test; add quarter-beat test |

---

## Task 1: Add `underline_count` to the type and fix all construction sites

**Files:**
- Modify: `src/layout/types.rs`
- Modify: `src/layout/mod.rs`
- Modify: `src/renderer.rs`

This is a breaking type change. All three files must be updated together so the codebase compiles. We use `0` as the placeholder value in layout and renderer — correct values come in Tasks 2 and 3.

- [ ] **Step 1: Update the `LowerOctaveDots` variant**

In `src/layout/types.rs`, change:
```rust
LowerOctaveDots { count: u32 },
```
to:
```rust
LowerOctaveDots { count: u32, underline_count: u8 },
```

- [ ] **Step 2: Fix the construction site in `layout/mod.rs`**

In `src/layout/mod.rs`, find the `LowerOctaveDots` construction (around line 320). Change:
```rust
content: GridContent::LowerOctaveDots { count: (-note.octave) as u32 },
```
to (placeholder `underline_count: 0` for now):
```rust
content: GridContent::LowerOctaveDots { count: (-note.octave) as u32, underline_count: 0 },
```

- [ ] **Step 3: Fix the match in `renderer.rs`**

In `src/renderer.rs`, find the `GridContent::LowerOctaveDots { count }` match arm (around line 106). Change:
```rust
GridContent::LowerOctaveDots { count } => {
    let dot_radius = row_height * 0.08;
    let dot_spacing = dot_radius * 3.0;
    for i in 0..*count {
        let dot_y = base_y + dot_radius + (i as f32) * dot_spacing;
        elements.push_str(&format!(
            r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="black"/>"#,
            x, dot_y, dot_radius
        ));
    }
}
```
to (placeholder formula, still using old logic for now):
```rust
GridContent::LowerOctaveDots { count, underline_count } => {
    let dot_radius = row_height * 0.08;
    let dot_spacing = dot_radius * 3.0;
    for i in 0..*count {
        let dot_y = base_y + dot_radius + (i as f32) * dot_spacing;
        elements.push_str(&format!(
            r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="black"/>"#,
            x, dot_y, dot_radius
        ));
    }
    let _ = underline_count; // silenced until Task 3
}
```

- [ ] **Step 4: Fix the pattern match in the layout test**

In `src/layout/mod.rs`, find the `lower_octave_note_emits_lower_octave_dots_element` test (around line 985). Change:
```rust
if let GridContent::LowerOctaveDots { count } = &lower_dots[0].content {
    assert_eq!(*count, 1);
}
```
to:
```rust
if let GridContent::LowerOctaveDots { count, underline_count } = &lower_dots[0].content {
    assert_eq!(*count, 1);
    let _ = underline_count; // checked in Task 2
}
```

- [ ] **Step 5: Verify compilation and existing tests still pass**

```
cargo test
```
Expected: all existing tests pass (the placeholder `underline_count: 0` doesn't change visual output yet).

- [ ] **Step 6: Commit**

```bash
git add src/layout/types.rs src/layout/mod.rs src/renderer.rs
git commit -m "refactor: add underline_count field to LowerOctaveDots (placeholder)"
```

---

## Task 2: Populate `underline_count` correctly in layout

**Files:**
- Modify: `src/layout/mod.rs`

- [ ] **Step 1: Update the test to assert correct values**

In `src/layout/mod.rs`, replace the `lower_octave_note_emits_lower_octave_dots_element` test body (around line 974–991):

```rust
#[test]
fn lower_octave_note_emits_lower_octave_dots_element() {
    // "1." = pitch 1, 1-beat note (duration=4), octave -1
    // underline_count for duration=4 is 0
    let score = make_score("1. 2 3 4", "a b c d");
    let pages = layout(&score, A4_WIDTH, A4_HEIGHT);
    let all_elements: Vec<_> = pages[0].row_groups.iter()
        .flat_map(|rg| rg.elements.iter())
        .collect();
    let lower_dots: Vec<_> = all_elements.iter()
        .filter(|e| matches!(e.content, GridContent::LowerOctaveDots { .. }))
        .collect();
    assert_eq!(lower_dots.len(), 1, "expected one LowerOctaveDots element");
    if let GridContent::LowerOctaveDots { count, underline_count } = &lower_dots[0].content {
        assert_eq!(*count, 1);
        assert_eq!(*underline_count, 0, "1-beat note has 0 underlines");
    }
    assert_eq!(lower_dots[0].position.row, 4, "LowerOctaveDots must be in absolute row 4");
    assert_eq!(lower_dots[0].vertical_alignment, VerticalAlignment::Top);
}
```

- [ ] **Step 2: Run the test to confirm it fails**

```
cargo test lower_octave_note_emits_lower_octave_dots_element
```
Expected: FAIL — `underline_count` is still `0` (passes that assert) but `vertical_alignment` is `Bottom` not `Top`.

- [ ] **Step 3: Fix the layout construction site**

In `src/layout/mod.rs`, replace the `LowerOctaveDots` construction block (around line 318–326). First, compute `underline_count` from the note's duration (the `underline_count` variable for normal notes is already computed a few lines below — see line 341). Replace the block:

```rust
// Lower octave dots (row +2)
if note.octave < 0 {
    current_elements.push(GridElement {
        position: GridPosition { column: col, row: part_row + 2 },
        horizontal_alignment: HorizontalAlignment::Center,
        vertical_alignment: VerticalAlignment::Bottom,
        content: GridContent::LowerOctaveDots { count: (-note.octave) as u32, underline_count: 0 },
    });
}
```

with:

```rust
// Lower octave dots (row +2)
if note.octave < 0 {
    let dot_underline_count: u8 = match note.duration {
        1 => 2,
        2 => 1,
        _ => 0,
    };
    current_elements.push(GridElement {
        position: GridPosition { column: col, row: part_row + 2 },
        horizontal_alignment: HorizontalAlignment::Center,
        vertical_alignment: VerticalAlignment::Top,
        content: GridContent::LowerOctaveDots { count: (-note.octave) as u32, underline_count: dot_underline_count },
    });
}
```

- [ ] **Step 4: Run the test to confirm it passes**

```
cargo test lower_octave_note_emits_lower_octave_dots_element
```
Expected: PASS.

- [ ] **Step 5: Run all tests**

```
cargo test
```
Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add src/layout/mod.rs
git commit -m "feat: populate underline_count in LowerOctaveDots from note duration"
```

---

## Task 3: Fix the renderer to use the slot formula

**Files:**
- Modify: `src/renderer.rs`

The slot formula places each item (underlines or dots) at:
```
y_slot(n) = base_y + row_height * 0.1 + n * (row_height * 0.15)
```

Dots occupy slots `underline_count … underline_count + count - 1`.

With default `row_height=24`, `PAGE_MARGIN=25`:
- `base_y` for row 4 = `25 + 4 * 24 = 121.0`
- **1-beat lower-octave note** (underline_count=0, slot 0): `dot_y = 121.0 + 2.4 + 0 = 123.4`
- **Half-beat lower-octave note** (underline_count=1, slot 1): `dot_y = 121.0 + 2.4 + 3.6 = 127.0`
- **Quarter-beat lower-octave note** (underline_count=2, slot 2): `dot_y = 121.0 + 2.4 + 7.2 = 130.6`

- [ ] **Step 1: Update the existing renderer test to assert the exact y position**

In `src/renderer.rs`, replace the `lower_octave_note_renders_dot_below_note` test:

```rust
#[test]
fn lower_octave_note_renders_dot_below_note() {
    // "1." = 1-beat note, octave -1 → underline_count=0, slot 0
    // row_height=24, PAGE_MARGIN=25, row=4 → base_y=121.0
    // dot_y = 121.0 + 24*0.1 + 0*(24*0.15) = 123.4
    let svgs = render_score("1. 2 3 4", "a b c d");
    assert!(svgs[0].contains(r#"cy="123.4""#), "1-beat lower-octave dot must be at slot 0 (cy=123.4)");
}
```

- [ ] **Step 2: Add a new test for a quarter-beat lower-octave note**

Add this test after `lower_octave_note_renders_dot_below_note` in `src/renderer.rs`:

```rust
#[test]
fn quarter_beat_lower_octave_dot_is_below_two_underlines() {
    // "=1." = quarter-beat note (duration=1), octave -1 → underline_count=2, slot 2
    // row_height=24, PAGE_MARGIN=25, row=4 → base_y=121.0
    // dot_y = 121.0 + 24*0.1 + 2*(24*0.15) = 130.6
    // Need 16 quarter-beat notes to fill a 4/4 bar.
    let score_str = "=1. =1 =1 =1 =1 =1 =1 =1 =1 =1 =1 =1 =1 =1 =1 =1";
    let lyrics_str = "a b c d e f g h i j k l m n o p";
    let svgs = render_score(score_str, lyrics_str);
    assert!(svgs[0].contains(r#"cy="130.6""#), "quarter-beat lower-octave dot must be at slot 2 (cy=130.6)");
}
```

- [ ] **Step 3: Run tests to confirm they fail**

```
cargo test lower_octave_note_renders_dot_below_note quarter_beat_lower_octave_dot_is_below_two_underlines
```
Expected: both FAIL (current formula produces wrong cy values).

- [ ] **Step 4: Fix the renderer formula**

In `src/renderer.rs`, replace the `LowerOctaveDots` match arm:

```rust
GridContent::LowerOctaveDots { count, underline_count } => {
    let dot_radius = row_height * 0.08;
    for i in 0..*count {
        let slot = *underline_count as f32 + i as f32;
        let dot_y = base_y + row_height * 0.1 + slot * (row_height * 0.15);
        elements.push_str(&format!(
            r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="black"/>"#,
            x, dot_y, dot_radius
        ));
    }
}
```

- [ ] **Step 5: Run the two new/updated tests**

```
cargo test lower_octave_note_renders_dot_below_note quarter_beat_lower_octave_dot_is_below_two_underlines
```
Expected: both PASS.

- [ ] **Step 6: Run all tests**

```
cargo test
```
Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add src/renderer.rs
git commit -m "fix: render lower octave dots below underlines using slot formula"
```
