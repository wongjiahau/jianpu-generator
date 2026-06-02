use crate::error::Spanned;

pub struct ParsedDocument {
    pub filename: String,
    pub metadata: ParsedMetadata,
    pub score_events: Vec<Spanned<ScoreEvent>>,
    pub lyrics: Vec<Syllable>,
}

pub struct ParsedMetadata {
    pub title: String,
    pub author: String,
    pub cell_size: Option<u32>,
}

pub enum ScoreEvent {
    Note(ParsedNote),
    Rest(ParsedRest),
    BpmChange(u32),
    KeyChange(KeyChange),
    TimeSignatureChange { numerator: u8, denominator: u8 },
    /// The `-` token: extends the previous note/rest by one full beat (4 quarter-beats).
    Extension,
}

pub struct ParsedNote {
    pub pitch: JianPuPitch,
    /// Octave offset from the default octave. 0 = default, positive = up, negative = down.
    pub octave: i8,
    /// Duration in quarter-beats. Token parser produces only 1 (=), 2 (_), or 4 (no prefix).
    pub duration: u32,
    /// Whether `~` follows this note (tie or slur, determined later by pitch comparison).
    pub tie: bool,
}

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

pub struct KeyChange {
    pub note: Note,
}

pub struct Note {
    pub name: NoteName,
    pub octave: u8,
    pub accidental: Accidental,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NoteName { A, B, C, D, E, F, G }

#[derive(Debug, Clone, PartialEq)]
pub enum Accidental { Flat, Sharp, Natural }

pub struct Syllable {
    pub text: String,
    /// True if `-` follows this syllable in the lyrics section.
    pub held: bool,
}
