use crate::ast::parsed::{Accidental, JianPuPitch};
use crate::compiler::types::ArcKind;

#[derive(Debug, Clone)]
pub struct AbsolutePage {
    pub width_pt: f32,
    pub height_pt: f32,
    pub elements: Vec<AbsoluteElement>,
}

#[derive(Debug, Clone)]
pub struct AbsoluteElement {
    pub x: f32,
    pub y: f32,
    pub content: AbsoluteContent,
}

#[derive(Debug, Clone)]
pub enum AbsoluteContent {
    NoteHead {
        pitch: JianPuPitch,
        accidental: Accidental,
        octave: i8,
        dotted: bool,
        double_dotted: bool,
    },
    Rest {
        dotted: bool,
        double_dotted: bool,
        implicit_fill: bool,
    },
    MultiMeasureRest {
        count: u32,
        width: f32,
    },
    ChordSymbol {
        text: String,
        dotted: bool,
        double_dotted: bool,
    },
    NoteDash {
        dotted: bool,
        double_dotted: bool,
    },
    PercussionHit,
    Underline {
        width: f32,
        level: u32,
    },
    TieOrSlur {
        kind: ArcKind,
        width: f32,
    },
    /// Tuplet bracket: short vertical ticks + horizontal line spanning
    /// `width`, with `label` (the tuplet digit, e.g. `"3"`) centered above
    /// the midpoint. See `GridContent::TupletBracket`.
    TupletBracket {
        label: String,
        width: f32,
    },
    BarLine {
        height: f32,
    },
    HorizontalLine {
        width: f32,
    },
    /// `source_part_index`/`note_id`/`verse` identify the syllable's source
    /// note, mirroring `GridContent::LyricSyllable` — used only by the
    /// renderer's `Tag::Lyric` click-target overlay, not by `render_lyric`
    /// itself (which only needs `text`).
    Lyric {
        text: String,
        source_part_index: usize,
        note_id: usize,
        verse: usize,
    },
    /// A standalone `lyrics` part's whole verse line, left-aligned starting
    /// at the element's `x`, same as [`AbsoluteContent::Lyric`] but spanning
    /// the full measure width instead of being positioned per note.
    LyricLine(String),
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
    MeasureHighlight {
        width: f32,
        height: f32,
    },
    /// Red semi-transparent overlay drawn over a measure with recoverable errors.
    ErrorHighlight {
        width: f32,
        height: f32,
    },
    MeasureClickTarget {
        width: f32,
        height: f32,
        measure_index: usize,
        measure_index_end: usize,
    },
    /// Invisible click hit target laid over one measure's own bar
    /// number (see `grid_layout::types::BarNumberClickTarget`), tightly
    /// sized to the digits themselves rather than the whole measure body —
    /// unlike `MeasureClickTarget`, which never sits above the musical rows
    /// where the bar number is actually drawn. Rendered with its own
    /// `TransparentRectRole::BarNumberClickTarget` (not
    /// `MeasureClickTarget`'s) purely so it can get its own hover styling
    /// without also lighting up on every note hover the way a hover rule on
    /// `MeasureClickTarget` would (that rect already spans every note in
    /// the measure) — both still carry `Tag::Measure` so a click resolves
    /// through the same `getMeasureAtPoint` path either way.
    BarNumberClickTarget {
        width: f32,
        height: f32,
        measure_index: usize,
        measure_index_end: usize,
    },
    /// Invisible click hit target for one bar line (see
    /// `grid_layout::types::BarLineClickTarget`), spanning the whole system
    /// vertically but a fixed, narrow width horizontally (baked in at
    /// resolve time — the old TS-side `BAR_LINE_HIT_WIDTH` constant this
    /// replaces). Carries both adjacent measures' indices so the frontend
    /// can resolve "which measure does this bar line select" as `next ??
    /// prev` without any pixel geometry.
    BarLineClickTarget {
        width: f32,
        height: f32,
        measure_index_next: Option<usize>,
        measure_index_prev: Option<usize>,
    },
    /// Background rect behind one part's sounding note/rest, toggled at
    /// playback time by the frontend rather than filled here (see
    /// `renderer::new_types::SvgKind::PlaybackCursorRect`).
    PlaybackCursorTarget {
        width: f32,
        height: f32,
        source_part_index: usize,
        note_id: usize,
    },
    /// Invisible click hit target layered above `PlaybackCursorTarget`
    /// for the same note/rest, since that rect is `pointer-events: none`
    /// (its `fill` is owned exclusively by playback highlighting — see
    /// `renderer::new_types::TransparentRectRole::NoteClickTarget`).
    NoteClickTarget {
        width: f32,
        height: f32,
        source_part_index: usize,
        note_id: usize,
    },
    /// Invisible click hit target laid over a part's `RowLabel` text
    /// (see `grid_layout::types::PartLabelClickTarget`), spanning that
    /// part's own sub-rows within the fixed-width label region. Clicking or
    /// range-selecting it selects every note/rest that part sounds across
    /// `measure_index_start..=measure_index_end` (the whole system the
    /// label sits in).
    PartLabelClickTarget {
        width: f32,
        height: f32,
        source_part_index: usize,
        measure_index_start: usize,
        measure_index_end: usize,
    },
    /// Invisible click hit target for one lyric syllable, independent of
    /// its note's own `NoteClickTarget` — see
    /// `renderer::new_types::Tag::Lyric`.
    LyricClickTarget {
        width: f32,
        height: f32,
        source_part_index: usize,
        note_id: usize,
        verse: usize,
    },
    /// Invisible click hit target laid over one verse's `RowLabel` text
    /// (see `grid_layout::types::LyricLabelClickTarget`) — the lyric-side
    /// mirror of `PartLabelClickTarget`. Clicking or range-selecting it
    /// selects every syllable that verse sings across
    /// `measure_index_start..=measure_index_end` (the whole system the
    /// label sits in).
    LyricLabelClickTarget {
        width: f32,
        height: f32,
        source_part_index: usize,
        verse: usize,
        measure_index_start: usize,
        measure_index_end: usize,
    },
    DirectiveLine {
        /// Bar-number span, rendered as its own text element pinned to the
        /// line's start (offset 0) so it always precedes `label` and
        /// `spans`, regardless of their widths.
        bar_number: Option<TextSpan>,
        /// See `Metadata::measure_number_style`. Meaningless when `bar_number` is `None`.
        bar_number_font_family: FontFamily,
        /// Section-label text, rendered as its own text/box element
        /// independent of `spans` (see `label_x_offset`) rather than as one
        /// of `spans`'s tspans, so it doesn't need to know their combined
        /// rendered width.
        label: Option<String>,
        /// Font size in points of `label` (see `Metadata::section_label_font_size`).
        /// Meaningless when `label` is `None`.
        label_font_size: f32,
        /// See `Metadata::section_label_style`. Meaningless when `label` is `None`.
        label_bold: bool,
        label_italic: bool,
        label_underline: bool,
        /// See `Metadata::section_label_style`. Meaningless when `label` is `None`.
        label_font_family: FontFamily,
        /// Height in points of `label`'s rendered background box (see
        /// `font_metrics::section_label_box_height`), already including
        /// `Metadata::section_label.vertical_padding_pt`. Meaningless when
        /// `label` is `None`.
        label_box_height: f32,
        /// Key/bpm/time-signature spans, i.e. everything on the line except
        /// `bar_number` and `label`.
        spans: Vec<TextSpan>,
        /// Font family for `spans`. `SansSerif` for an ordinary directive
        /// line's key/bpm/time-signature text (not a configurable text-style
        /// kind); `Metadata::sequence.font_family` for the `# sequence`
        /// summary line's spans (see `PostArcGridContent::SequenceLine`).
        spans_font_family: FontFamily,
        /// X offset (in points, from the line's start) where `spans`
        /// begins: right after `bar_number` when there is no `label`, or
        /// past `label`'s bounding box when there is, so the three
        /// elements never overlap regardless of their measured widths.
        spans_x_offset: f32,
        /// X offset (in points, from the line's start) where the
        /// independent `label` text element begins: past `bar_number`'s
        /// measured width when one is present, zero otherwise.
        label_x_offset: f32,
        /// Whether `directive_row_offset` should be applied to this line.
        /// `true` for ordinary directive lines; `false` for the `# sequence`
        /// summary header, which must not move.
        apply_row_offset: bool,
    },
}

#[derive(Debug, Clone)]
pub struct TextSpan {
    pub content: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub font_size: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAnchor {
    Start,
    Middle,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DominantBaseline {
    Middle,
    Hanging,
    Ideographic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontFamily {
    Monospace,
    /// Everything not on `Serif`: the directive line (bar number, section
    /// label, key/bpm/time signature), part legend, and footer. Pinned to
    /// whichever font backs the `sansSerif` role in `fonts/fonts.json`
    /// (currently Source Han Sans SC — see that file's comment on why the
    /// two roles differ). See `DIRECTIVE_LINE_FONT_FAMILY` in
    /// `src/serializer/mod.rs`. The default role for kinds without an
    /// explicit `font_family` override.
    #[default]
    SansSerif,
    /// The song title, subtitle, and author (`Header::title`/`subtitle`/
    /// `author`, via `make_title_row`/`make_subtitle_author_row`) and lyric
    /// syllables/lines (`render_lyric`/`render_lyric_line`) — pinned to
    /// whichever font backs the `serif` role in `fonts/fonts.json`
    /// (currently Zhuque Fangsong, a calligraphic font kept off the part
    /// legend and footer's Latin glyphs — see `SansSerif` above). Despite the
    /// name, this variant isn't title-exclusive; it's named for its original
    /// single use before the other three joined it. See `SERIF_FONT_FAMILY`
    /// in `src/serializer/mod.rs`.
    Serif,
}

/// Which `FontFamily` each of the three glyph-measured kinds — `notes`,
/// `chords`, `note_dash` — renders in, bundled since every caller sets/reads
/// all three together (see `RenderConfig::glyph_font_families`,
/// `coordinate_resolver::resolve::ResolveFontSizes::glyph_font_families`).
/// Its `Default` is `Monospace` for every field, matching those three kinds'
/// real text-style default (`grouper::resolve_simple_text_styles`) rather
/// than the derived per-field `FontFamily::default()` (`SansSerif`) — so
/// `..Default::default()` in a test fixture matches production's default
/// without spelling it out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphFontFamilies {
    pub notes: FontFamily,
    pub chords: FontFamily,
    pub note_dash: FontFamily,
}

impl Default for GlyphFontFamilies {
    fn default() -> Self {
        Self {
            notes: FontFamily::Monospace,
            chords: FontFamily::Monospace,
            note_dash: FontFamily::Monospace,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontWeight {
    Normal,
    Bold,
}
