use crate::ast::parsed::JianPuPitch;

#[derive(Debug, Clone)]
pub struct Header {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: String,
}

#[derive(Debug, Clone)]
pub struct Footer {
    pub page: u32,
    pub total: u32,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub header: Header,
    pub footer: Footer,
    pub row_groups: Vec<RowGroup>,
}

#[derive(Debug, Clone)]
pub struct RowGroup {
    pub elements: Vec<GridElement>,
    pub height_in_rows: u32,
}

#[derive(Debug, Clone)]
pub struct GridPosition {
    pub column: u32,
    pub row: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Clone)]
pub struct GridElement {
    pub position: GridPosition,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub content: GridContent,
}

/// The column range covered by one underline level.
#[derive(Debug, Clone, PartialEq)]
pub struct UnderlineSpan {
    pub from_column: u32,
    pub to_column: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GridContent {
    NoteHead { pitch: JianPuPitch, octave: i8 },
    Rest,
    Lyric { text: String, is_cjk: bool },
    TieOrSlurCurve { from_column: u32, to_column: u32 },
    /// Horizontal underlines for a beam group.
    /// `levels[0]` is the topmost line (closest to the note heads), spanning all notes.
    /// Additional entries are drawn below it; each covers one maximal contiguous sub-run
    /// of notes whose underline count is >= that level number.
    DurationUnderlines { levels: Vec<UnderlineSpan> },
    /// Dots indicating a note is one or more octaves below the default octave.
    /// Placed in row 2 with VerticalAlignment::Bottom so they always appear below underlines.
    LowerOctaveDots { count: u32 },
    BarLine,
    Extension,
}
