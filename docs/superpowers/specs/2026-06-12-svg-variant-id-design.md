# SVG Variant ID Design

**Date:** 2026-06-12

## Goal

Attach a `data-variant` attribute to every generated SVG element to identify which musical content type produced it. Aids debugging by making SVG elements inspectable in browser devtools.

## Approach

Add a `variant: &'static str` field to `SvgElement`. The renderer sets it inline at each construction site. The serializer emits it as a `data-variant` attribute on every tag. Always included — no flag needed.

## Data Model Change

```rust
// renderer/new_types.rs
pub struct SvgElement {
    pub x: f32,
    pub y: f32,
    pub variant: &'static str,  // new
    pub kind: SvgKind,
}
```

## Variant Name Mapping

| `AbsoluteContent` variant | `variant` value   |
|---------------------------|-------------------|
| `NoteHead`                | `"note-head"`     |
| `Rest`                    | `"rest"`          |
| `ChordSymbol`             | `"chord-symbol"`  |
| `Underline`               | `"underline"`     |
| `TieOrSlur`               | `"tie-or-slur"`   |
| `BarLine`                 | `"bar-line"`      |
| `HorizontalLine`          | `"horizontal-line"` |
| `Lyric`                   | `"lyric"`         |
| `Text`                    | `"text"`          |

## Renderer Change

Every `SvgElement { x, y, kind }` construction in `renderer/new_renderer.rs` gains `variant: "<name>"`. Each renderer function handles exactly one `AbsoluteContent` arm, so each function has a single literal — no conditional branching required.

## Serializer Change

Every SVG tag gains `data-variant="{}"`. Example output:

```svg
<text x="42.0" y="100.0" data-variant="note-head" font-size="14.4" ...>3</text>
<circle cx="42.0" cy="95.0" data-variant="note-head" r="2.5" fill="black"/>
<line x1="10.0" y1="50.0" data-variant="bar-line" x2="10.0" y2="80.0" stroke="black" stroke-width="0.5"/>
<path d="M 10.0 60.0 Q 30.0 45.0 50.0 60.0" data-variant="tie-or-slur" fill="none" stroke="black" stroke-width="1.0"/>
```

## Testing

- Existing serializer unit tests construct `SvgElement` literals — add `variant` field to each.
- Extend each serializer test to assert the `data-variant` attribute appears in the output string.
