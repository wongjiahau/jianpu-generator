use crate::error::Spanned;

#[derive(Debug)]
pub struct ParsedScore {
    pub events: Vec<Spanned<ScoreEvent>>,
}

#[derive(Debug)]
pub struct ParsedLyrics {
    pub syllables: Vec<Syllable>,
}

#[derive(Debug)]
pub struct ParsedPart {
    pub name: Option<String>,
    pub score: ParsedScore,
    pub lyrics: Option<ParsedLyrics>,
}

#[derive(Debug)]
pub struct ParsedDocument {
    pub filename: String,
    pub metadata: ParsedMetadata,
    pub parts: Vec<ParsedPart>,
}

#[derive(Debug)]
pub struct ParsedMetadata {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: String,
    pub row_height: Option<u32>,
    pub max_columns: Option<u32>,
    pub label_width: Option<u32>,
}

#[derive(Debug)]
pub enum ScoreEvent {
    Note(ParsedNote),
    Rest(ParsedRest),
    BpmChange(u32),
    KeyChange(KeyChange),
    TimeSignatureChange { numerator: u8, denominator: u8 },
    /// The `-` token: extends the previous note/rest by one full beat (4 quarter-beats).
    Extension,
}

#[derive(Debug)]
pub struct ParsedNote {
    pub pitch: JianPuPitch,
    /// Octave offset from the default octave. 0 = default, positive = up, negative = down.
    pub octave: i8,
    /// Duration in quarter-beats. Token parser produces only 1 (=), 2 (_), or 4 (no prefix).
    pub duration: u32,
    /// Whether `~` follows this note (tie or slur, determined later by pitch comparison).
    pub tie: bool,
}

#[derive(Debug)]
pub struct ParsedRest {
    /// Duration in quarter-beats. Token parser produces only 1, 2, or 4.
    pub duration: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JianPuPitch {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
}

#[derive(Debug, Clone)]
pub struct KeyChange {
    pub note: Note,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub name: NoteName,
    pub octave: u8,
    pub accidental: Accidental,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NoteName { A, B, C, D, E, F, G }

#[derive(Debug, Clone, PartialEq)]
pub enum Accidental { Flat, Sharp, Natural }

#[derive(Debug, Clone, PartialEq)]
pub struct Syllable {
    pub text: String,
    /// True if `-` follows this syllable in the lyrics section.
    pub held: bool,
}
