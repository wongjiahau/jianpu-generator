use crate::compositor::types::{AbsoluteContent, AbsoluteElement};
use crate::renderer::new_types::{SvgElement, SvgKind, Tag, TransparentRectRole};

/// Shared shape behind every `render_*_click_target`/`render_playback_cursor_target`
/// function below: a single-child `SvgKind::Group` (both elements positioned
/// at `elem`'s origin) carrying `tag`, with `rect_kind` as its one child's
/// content. Each caller only needs to build its own rect kind and `Tag`.
fn wrap_click_target(elem: &AbsoluteElement, rect_kind: SvgKind, tag: Tag) -> Vec<SvgElement> {
    vec![SvgElement {
        x: elem.x,
        y: elem.y,
        variant: None,
        kind: SvgKind::Group {
            children: vec![SvgElement {
                x: elem.x,
                y: elem.y,
                variant: None,
                kind: rect_kind,
            }],
            tag: Some(tag),
        },
    }]
}

/// Split out of `render_overlay_element` to keep it under the max function
/// length — [`AbsoluteContent::PartLabelClickTarget`] and
/// [`AbsoluteContent::LyricClickTarget`] are the two lowest-traffic click
/// target variants, so they're grouped here together.
pub(super) fn render_secondary_click_target(
    elem: &AbsoluteElement,
    content: &AbsoluteContent,
) -> Vec<SvgElement> {
    match content {
        AbsoluteContent::PartLabelClickTarget {
            width,
            height,
            source_part_index,
            measure_index_start,
            measure_index_end,
        } => render_part_label_click_target(
            elem,
            *width,
            *height,
            *source_part_index,
            *measure_index_start,
            *measure_index_end,
        ),
        AbsoluteContent::LyricClickTarget {
            width,
            height,
            source_part_index,
            note_id,
            verse,
        } => render_lyric_click_target(elem, *width, *height, *source_part_index, *note_id, *verse),
        AbsoluteContent::LyricLabelClickTarget {
            width,
            height,
            source_part_index,
            verse,
            measure_index_start,
            measure_index_end,
        } => render_lyric_label_click_target(
            elem,
            &LyricLabelClickTargetArgs {
                width: *width,
                height: *height,
                source_part_index: *source_part_index,
                verse: *verse,
                measure_index_start: *measure_index_start,
                measure_index_end: *measure_index_end,
            },
        ),
        AbsoluteContent::BarNumberClickTarget {
            width,
            height,
            measure_index,
            measure_index_end,
        } => render_bar_number_click_target(
            elem,
            *width,
            *height,
            *measure_index,
            *measure_index_end,
        ),
        AbsoluteContent::BarLineClickTarget {
            width,
            height,
            measure_index_next,
            measure_index_prev,
        } => render_bar_line_click_target(
            elem,
            *width,
            *height,
            *measure_index_next,
            *measure_index_prev,
        ),
        _ => Vec::new(),
    }
}

pub(super) fn render_playback_cursor_target(
    elem: &AbsoluteElement,
    width: f32,
    height: f32,
    source_part_index: usize,
    note_id: usize,
) -> Vec<SvgElement> {
    wrap_click_target(
        elem,
        SvgKind::PlaybackCursorRect { width, height },
        Tag::Note {
            source_part_index,
            note_id,
        },
    )
}

/// Sibling group to [`render_playback_cursor_target`] for the same note/rest,
/// giving it a clickable hit target — `PlaybackCursorRect` is
/// `pointer-events: none` since its `fill` is owned exclusively by
/// `usePlaybackCursor.ts`, so a separate transparent rect handles clicks.
/// Carries the same `Tag::Note` `source_part_index`/`note_id` so a click on
/// it resolves to the same note as the playback cursor rect underneath.
pub(super) fn render_note_click_target(
    elem: &AbsoluteElement,
    width: f32,
    height: f32,
    source_part_index: usize,
    note_id: usize,
) -> Vec<SvgElement> {
    wrap_click_target(
        elem,
        SvgKind::TransparentRect {
            width,
            height,
            role: TransparentRectRole::NoteClickTarget,
        },
        Tag::Note {
            source_part_index,
            note_id,
        },
    )
}

fn render_part_label_click_target(
    elem: &AbsoluteElement,
    width: f32,
    height: f32,
    source_part_index: usize,
    measure_index_start: usize,
    measure_index_end: usize,
) -> Vec<SvgElement> {
    wrap_click_target(
        elem,
        SvgKind::TransparentRect {
            width,
            height,
            role: TransparentRectRole::PartLabelClickTarget,
        },
        Tag::PartLabel {
            source_part_index,
            measure_index_start,
            measure_index_end,
        },
    )
}

/// Args for `render_lyric_label_click_target`, bundled to stay under the
/// max-arguments lint (its `AbsoluteContent::LyricLabelClickTarget` source
/// already carries one field more than a plain arg list allows).
struct LyricLabelClickTargetArgs {
    width: f32,
    height: f32,
    source_part_index: usize,
    verse: usize,
    measure_index_start: usize,
    measure_index_end: usize,
}

/// The lyric-side mirror of `render_part_label_click_target`, for one
/// verse's own `RowLabel` text.
fn render_lyric_label_click_target(
    elem: &AbsoluteElement,
    args: &LyricLabelClickTargetArgs,
) -> Vec<SvgElement> {
    wrap_click_target(
        elem,
        SvgKind::TransparentRect {
            width: args.width,
            height: args.height,
            role: TransparentRectRole::LyricLabelClickTarget,
        },
        Tag::LyricLabel {
            source_part_index: args.source_part_index,
            verse: args.verse,
            measure_index_start: args.measure_index_start,
            measure_index_end: args.measure_index_end,
        },
    )
}

/// Sibling overlay to a lyric syllable's own text element, giving it a
/// clickable hit target independent of its note's — see
/// `Tag::Lyric`. Painted after `PartLabelClickTarget` (see
/// `resolve_click_target_elements`'s append order), so its narrow rect wins
/// hit-testing over the wider `NoteClickTarget` that geometrically covers
/// the same lyric row.
fn render_lyric_click_target(
    elem: &AbsoluteElement,
    width: f32,
    height: f32,
    source_part_index: usize,
    note_id: usize,
    verse: usize,
) -> Vec<SvgElement> {
    wrap_click_target(
        elem,
        SvgKind::TransparentRect {
            width,
            height,
            role: TransparentRectRole::LyricClickTarget,
        },
        Tag::Lyric {
            source_part_index,
            note_id,
            verse,
        },
    )
}

pub(super) fn render_measure_click_target(
    elem: &AbsoluteElement,
    width: f32,
    height: f32,
    measure_index: usize,
    measure_index_end: usize,
) -> Vec<SvgElement> {
    wrap_click_target(
        elem,
        SvgKind::TransparentRect {
            width,
            height,
            role: TransparentRectRole::MeasureClickTarget,
        },
        Tag::Measure {
            index: measure_index,
            end: measure_index_end,
        },
    )
}

/// Sibling to [`render_measure_click_target`] for one measure's own bar
/// number — its own `Tag::BarNumber` and `TransparentRectRole` (rather than
/// reusing `Tag::Measure`'s) so it can get its own hover styling without
/// disturbing `[data-tag="measure"]`'s existing one-group-per-measure DOM
/// shape — see `compositor::types::AbsoluteContent::BarNumberClickTarget`.
/// The frontend's `getMeasureAtPoint` (`previewSelection.ts`) queries both
/// tags together, so a click still resolves to the same measure either way.
pub(super) fn render_bar_number_click_target(
    elem: &AbsoluteElement,
    width: f32,
    height: f32,
    measure_index: usize,
    measure_index_end: usize,
) -> Vec<SvgElement> {
    wrap_click_target(
        elem,
        SvgKind::TransparentRect {
            width,
            height,
            role: TransparentRectRole::BarNumberClickTarget,
        },
        Tag::BarNumber {
            index: measure_index,
            end: measure_index_end,
        },
    )
}

/// Sibling click target to one bar line, exactly like `BarNumberClickTarget`
/// is a sibling of `MeasureClickTarget` — a separate group laid over the
/// bar line, never a change to `render_bar_line`/`glyph_renderers.rs`
/// itself. See `compositor::types::AbsoluteContent::BarLineClickTarget`.
pub(super) fn render_bar_line_click_target(
    elem: &AbsoluteElement,
    width: f32,
    height: f32,
    measure_index_next: Option<usize>,
    measure_index_prev: Option<usize>,
) -> Vec<SvgElement> {
    wrap_click_target(
        elem,
        SvgKind::TransparentRect {
            width,
            height,
            role: TransparentRectRole::BarLineClickTarget,
        },
        Tag::BarLine {
            measure_index_next,
            measure_index_prev,
        },
    )
}
