use crate::ast::parsed::JianPuPitch;
use nonempty::NonEmpty;

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
    /// Page width in SVG/PDF points (same value passed to layout()).
    pub page_width_pt: f32,
}

#[derive(Debug, Clone)]
pub struct RowGroup {
    pub elements: NonEmpty<GridElement>,
    pub height_in_rows: u32,
    /// Number of grid columns actually used by this row group.
    pub width_in_columns: u32,
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
    /// Exclusive end (last note head column + last note duration). Used for the left endpoint.
    pub to_column: u32,
    /// Column of the last note head in this span. Used for the right endpoint of the underline.
    pub last_head_column: u32,
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
    BarLine { height_in_rows: u32 },
    Extension,
    TimeSignatureLabel { numerator: u8, denominator: u8 },
    BpmLabel { bpm: u32 },
    PartLabel { text: String },
    HorizontalBar { from_column: u32, to_column: u32 },
    BarNumber { number: u32 },
    SectionLabel { text: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_number_variant_exists() {
        let c = GridContent::BarNumber { number: 5 };
        match c {
            GridContent::BarNumber { number } => assert_eq!(number, 5),
            _ => panic!("unexpected variant"),
        }
    }
}
