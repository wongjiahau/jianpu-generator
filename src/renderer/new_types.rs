use crate::compositor::types::{DominantBaseline, FontFamily, FontWeight, TextAnchor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SvgVariant {
    Text,
    NoteHead,
    Rest,
    /// A rest that fills a part not mentioned in this measure, rendered
    /// with a distinct glyph so it reads as "not written" rather than
    /// "written as silent". See `ParsedRest::implicit_fill`.
    OmittedPartRest,
    MultiMeasureRest,
    ChordSymbol,
    PercussionHit,
    HorizontalLine,
    Underline,
    TieOrSlur,
    TupletBracket,
    BarLine,
    Lyric,
    DirectiveLine,
}

impl SvgVariant {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::NoteHead => "note-head",
            Self::Rest => "rest",
            Self::OmittedPartRest => "omitted-part-rest",
            Self::MultiMeasureRest => "multi-measure-rest",
            Self::ChordSymbol => "chord-symbol",
            Self::PercussionHit => "percussion-hit",
            Self::HorizontalLine => "horizontal-line",
            Self::Underline => "underline",
            Self::TieOrSlur => "tie-or-slur",
            Self::TupletBracket => "tuplet-bracket",
            Self::BarLine => "bar-line",
            Self::Lyric => "lyric",
            Self::DirectiveLine => "directive-line",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransparentRectRole {
    MeasureClickTarget,
    /// Sibling to `MeasureClickTarget` for one measure's own bar number —
    /// see `compositor::types::AbsoluteContent::BarNumberClickTarget` for
    /// why it needs its own role (distinct hover styling) rather than
    /// reusing `MeasureClickTarget`'s.
    BarNumberClickTarget,
    SectionLabelBackground,
    /// Invisible rect spanning the section label's whole directive line (bar
    /// number through the trailing key/bpm/time-signature/navigation-marker
    /// text), drawn underneath `SectionLabelBackground` inside the same
    /// group. Without it, the group's actual clickable area is only the sum
    /// of its children's painted pixels — since Task 2 of
    /// `PLAN-section-label-engraving-quality.md` split those children into
    /// independently-positioned elements with gaps between them, a line
    /// with no trailing spans (e.g. a bare `label="B"` directive) leaves an
    /// unpainted gap between the bar number and the label box that a click
    /// falls through, even though the group still shows `cursor: pointer`
    /// there.
    SectionLabelClickTarget,
    /// Invisible rect layered on top of `PlaybackCursorRect` inside the same
    /// `Tag::Note` group, giving each note/rest a clickable hit
    /// target — `PlaybackCursorRect` itself is `pointer-events: none` since
    /// its `fill` is owned by playback highlighting, not click handling.
    NoteClickTarget,
    /// Invisible rect layered over a part's `RowLabel` text, giving it a
    /// clickable hit target — see `Tag::PartLabel`.
    PartLabelClickTarget,
    /// Invisible rect layered over one lyric syllable, giving it its own
    /// clickable hit target independent of its note's
    /// `NoteClickTarget` — see `Tag::Lyric`. Painted last (after
    /// `PartLabelClickTarget`) so it always wins hit-testing over the wider
    /// `NoteClickTarget` rect that geometrically covers the same lyric row.
    LyricClickTarget,
    /// Invisible rect layered over one verse's `RowLabel` text, giving it a
    /// clickable hit target — see `Tag::LyricLabel`.
    LyricLabelClickTarget,
    /// Invisible rect for one bar line's own click hit target — see
    /// `Tag::BarLine`. Given its own role (rather than reusing
    /// `MeasureClickTarget`'s) purely for distinct hover styling, the same
    /// reason `BarNumberClickTarget` has its own.
    BarLineClickTarget,
}

impl TransparentRectRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MeasureClickTarget => "measure-click-target-rect",
            Self::BarNumberClickTarget => "bar-number-click-target-rect",
            Self::SectionLabelBackground => "section-label-bg",
            Self::SectionLabelClickTarget => "section-label-click-target-rect",
            Self::NoteClickTarget => "note-click-target-rect",
            Self::PartLabelClickTarget => "part-label-click-target-rect",
            Self::LyricClickTarget => "lyric-click-target-rect",
            Self::LyricLabelClickTarget => "lyric-label-click-target-rect",
            Self::BarLineClickTarget => "bar-line-click-target-rect",
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct SvgDocument {
    pub width_pt: f32,
    pub height_pt: f32,
    pub elements: Vec<SvgElement>,
}

#[derive(Debug, PartialEq)]
pub struct SvgElement {
    pub x: f32,
    pub y: f32,
    pub variant: Option<SvgVariant>,
    pub kind: SvgKind,
}

#[derive(Debug, PartialEq)]
pub enum Tag {
    /// `end` is the last original source measure index this click target
    /// represents; equal to `index` except for merged multi-measure rests.
    Measure {
        index: usize,
        end: usize,
    },
    /// Identifies one measure's own bar-number click target — see
    /// `AbsoluteContent::BarNumberClickTarget`. A distinct tag from
    /// `Tag::Measure` (rather than reusing it) so the frontend can hover-style
    /// it independently and so `[data-tag="measure"]`'s existing
    /// one-element-per-measure DOM shape (several e2e tests key off it via
    /// index) stays unaffected. `index`/`end` mean the same thing as
    /// `Tag::Measure`'s.
    BarNumber {
        index: usize,
        end: usize,
    },
    SectionLabel {
        label: String,
    },
    /// Identifies the sounding note/rest a `PlaybackCursorRect` sits behind,
    /// matching `crate::midi::timing::NoteTiming`'s `(source_part_index,
    /// note_id)` key so playback can look up which group(s) to highlight.
    Note {
        source_part_index: usize,
        note_id: usize,
    },
    /// Identifies a part's `RowLabel` click target — see
    /// `AbsoluteContent::PartLabelClickTarget`. `measure_index_start`/
    /// `measure_index_end` scope a click on this label to the whole
    /// system it sits in.
    PartLabel {
        source_part_index: usize,
        measure_index_start: usize,
        measure_index_end: usize,
    },
    /// Identifies a lyric syllable's own click target — see
    /// `AbsoluteContent::LyricClickTarget`. `source_part_index`/`note_id`
    /// match the syllable's underlying note's own `Tag::Note` identity;
    /// `verse` (0-indexed) disambiguates which verse line the syllable
    /// belongs to when a part has more than one.
    Lyric {
        source_part_index: usize,
        note_id: usize,
        verse: usize,
    },
    /// Identifies one verse's `RowLabel` click target — see
    /// `AbsoluteContent::LyricLabelClickTarget`. `measure_index_start`/
    /// `measure_index_end` scope a click on this label to the whole
    /// system it sits in, mirroring `Tag::PartLabel`.
    LyricLabel {
        source_part_index: usize,
        verse: usize,
        measure_index_start: usize,
        measure_index_end: usize,
    },
    /// Identifies one bar line's own click target — see
    /// `AbsoluteContent::BarLineClickTarget`. `measure_index_next`/
    /// `measure_index_prev` name the measure(s) this bar line sits between;
    /// a system's leading bar line has no `prev`, a system's closing bar
    /// line has no `next`. Resolving "which measure does this select" is
    /// `next ?? prev`.
    BarLine {
        measure_index_next: Option<usize>,
        measure_index_prev: Option<usize>,
    },
}

#[derive(Debug, PartialEq)]

pub enum SvgKind {
    Text {
        content: String,
        font_size: f32,
        anchor: TextAnchor,
        baseline: DominantBaseline,
        font: FontFamily,
        weight: FontWeight,
        italic: bool,
        underline: bool,
    },
    Line {
        x2: f32,
        y2: f32,
        stroke_width: f32,
    },
    Circle {
        r: f32,
    },
    Path {
        // Quadratic bezier: x/y from SvgElement; control and end vary
        control_x: f32,
        control_y: f32,
        end_x: f32,
        end_y: f32,
        stroke_width: f32,
    },
    Rect {
        width: f32,
        height: f32,
    },
    /// Red semi-transparent overlay for erroneous measures (15% opacity).
    ErrorRect {
        width: f32,
        height: f32,
    },
    /// Background rect behind a note/rest glyph, rendered `fill="transparent"`
    /// by default; the frontend toggles its fill at playback time to
    /// highlight whichever note/rest is currently sounding for its part. See
    /// `Tag::Note` for the group it renders inside.
    PlaybackCursorRect {
        width: f32,
        height: f32,
    },
    Group {
        children: Vec<SvgElement>,
        tag: Option<Tag>,
    },
    TransparentRect {
        width: f32,
        height: f32,
        role: TransparentRectRole,
    },
    TextWithTspans {
        font_size: f32,
        anchor: TextAnchor,
        baseline: DominantBaseline,
        font: FontFamily,
        spans: Vec<TspanData>,
    },
}

#[derive(Debug, PartialEq)]
pub struct TspanData {
    pub content: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub font_size: Option<f32>,
}
