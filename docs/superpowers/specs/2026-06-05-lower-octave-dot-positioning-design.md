# Lower Octave Dot Positioning

**Date:** 2026-06-05

## Problem

Lower octave dots and duration underlines are both rendered starting from `base_y` (the top of row +2), causing them to visually overlap. Additionally, the `VerticalAlignment::Bottom` on `LowerOctaveDots` elements is set but ignored by the renderer, which always uses `base_y` directly.

The correct JianPu convention is: lower octave dots appear **below** any underlines, in the same vertical slot system, stacked vertically.

## Slot System

Both underlines and lower octave dots share a common vertical slot formula:

```
y_slot(n) = base_y + row_height * 0.1 + n * (row_height * 0.15)
```

This is the same formula already used by `DurationUnderlines`. A note with `u` underlines and `d` lower-octave dots occupies:

- Underline slots: `0 … u-1`
- Dot slots: `u … u+d-1`

### Examples

| Duration | Underlines (u) | Octave dots (d) | Dot slot(s) |
|----------|---------------|-----------------|-------------|
| 1 beat   | 0             | 1               | 0           |
| 1 beat   | 0             | 2               | 0, 1        |
| Half     | 1             | 1               | 1           |
| Half     | 1             | 2               | 1, 2        |
| Quarter  | 2             | 1               | 2           |
| Quarter  | 2             | 2               | 2, 3        |

Worst case (2 underlines + 2 dots) lands at slot 3 = `base_y + 0.55 * row_height` — within row +2's height of `1.0 * row_height`.

## Changes

### `layout/types.rs`

Add `underline_count: u8` to `LowerOctaveDots`:

```rust
LowerOctaveDots { count: u32, underline_count: u8 },
```

### `layout/mod.rs`

When emitting `LowerOctaveDots`, populate `underline_count` from the note's duration:

```rust
let underline_count = match note.duration {
    1 => 2,
    2 => 1,
    _ => 0,
};
```

Change `VerticalAlignment` on `LowerOctaveDots` elements from `Bottom` to `Top` (since positioning is computed from `base_y`, not the bottom of the row).

### `renderer.rs`

Replace the current dot y formula:

```rust
let dot_y = base_y + dot_radius + (i as f32) * dot_spacing;
```

With the slot formula:

```rust
let slot = underline_count as f32 + i as f32;
let dot_y = base_y + row_height * 0.1 + slot * (row_height * 0.15);
```

Remove `dot_spacing` (no longer needed in this branch; `dot_radius` is still used for the circle `r` attribute).

## Tests

- Update `lower_octave_note_renders_dot_below_note` in `renderer.rs` to assert the correct y position using the slot formula.
- Update `lower_octave_note_emits_lower_octave_dots_element` in `layout/mod.rs` to assert `underline_count` on the emitted element.
- Add a renderer test for a quarter-beat lower-octave note asserting the dot is at slot 2.
