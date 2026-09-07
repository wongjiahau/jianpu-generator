use crate::error::Span;

use super::{Accidental, JianPuPitch, KeyChange};

#[derive(Debug, Clone, PartialEq)]
pub enum TriadQuality {
    Major,
    Minor,
    Augmented,
    Diminished,
    Sus2,
    Sus4,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Extension {
    DominantSeventh,
    MajorSeventh,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BassDegree {
    pub degree: JianPuPitch,
    pub accidental: Accidental,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScoreEvent {
    Note(ParsedNote),
    Chord(ParsedChordNote),
    PercussionHit(ParsedPercussionHit),
    Rest(ParsedRest),
    BpmChange(u32),
    KeyChange(KeyChange),
    TimeSignatureChange {
        numerator: u8,
        denominator: u8,
    },
    /// Internal or explicit padding: extends the previous note by one beat — one full beat
    /// (4 quarter-beats), one dotted beat (6 quarter-beats, written `-.`) for compound
    /// meters, or one double-dotted beat (7 quarter-beats, written `-..`).
    Extension {
        dotted: bool,
        double_dotted: bool,
    },
    /// Legacy tie marker retained for lyric-slot counting paths; use `(…)` groups in input.
    TieMarker,
    LabelChange(String),
    /// `merge_duplicate_measures_across_parts=` — in effect from this measure onward
    /// until the next occurrence.
    MergeDuplicateMeasuresAcrossPartsChange(bool),
    /// `hide_resting_parts=` — in effect from this measure onward until the next
    /// occurrence.
    HideRestingPartsChange(bool),
    /// `break` — forces a new system to start at this measure. Applies only
    /// to the measure it's written on; does not persist to later measures.
    SystemBreak,
}

/// Tuplet ratio tag attached to a parsed note/chord/rest/percussion-hit that falls inside
/// an open `{N:...}`/`{N:M:...}` bracket: `num` notes take the time of `den` notes of the
/// same written value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TupletInfo {
    pub num: u32,
    pub den: u32,
    /// Identifies which `{...}` bracket this tag came from, distinguishing
    /// directly-adjacent brackets that share the same `num`/`den` ratio (e.g.
    /// `3:{3 6 1} 3:{3 6 1}`) so they don't merge into a single tuplet span/bracket.
    /// Unique per opened bracket within a line; not meaningful beyond identity/equality.
    pub id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedNote {
    pub pitch: JianPuPitch,
    pub accidental: Accidental,
    /// Octave offset from the default octave. 0 = default, positive = up, negative = down.
    pub octave: i8,
    /// Duration in quarter-beats. For dotted notes this already includes the added half-value.
    pub duration: u32,
    /// Whether this note is tied/slurred to the next note (from a `(…)` group).
    pub slur: bool,
    /// Source span of the `~` suffix when this note is tied to the next note.
    pub tie_to_next_span: Option<Span>,
    /// Number of nested `(…)` groups this note belongs to.
    pub group_membership: u8,
    /// Number of those groups that continue past this note.
    pub group_continuation: u8,
    /// Whether `.` was present as a dotted-note suffix.
    pub dotted: bool,
    /// Whether `..` was present as a double-dotted-note suffix. Only ever `true` when
    /// `dotted` is also `true`.
    pub double_dotted: bool,
    /// When the slur group closes on an extension within this note (e.g. `(5 -)`),
    /// this holds the offset in quarter-beats from the note's start where the slur arc
    /// should end. `None` means the slur closes at the note's head position (normal case).
    pub slur_group_close_at_duration: Option<u32>,
    /// The innermost `{...}` tuplet bracket this note belongs to, if any.
    pub tuplet: Option<TupletInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedChordNote {
    pub degree: JianPuPitch,
    pub accidental: Accidental,
    pub triad: TriadQuality,
    pub extension: Option<Extension>,
    pub bass: Option<BassDegree>,
    pub duration: u32,
    pub slur: bool,
    pub tie_to_next_span: Option<Span>,
    pub group_membership: u8,
    pub group_continuation: u8,
    pub dotted: bool,
    pub double_dotted: bool,
    pub slur_group_close_at_duration: Option<u32>,
    /// The innermost `{...}` tuplet bracket this chord note belongs to, if any.
    pub tuplet: Option<TupletInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedPercussionHit {
    /// Duration in quarter-beats. For dotted hits this already includes the added half-value.
    pub duration: u32,
    /// Whether this hit is tied/slurred to the next hit (from a `(…)` group).
    pub slur: bool,
    /// Source span of the `~` suffix when this hit is tied to the next hit.
    pub tie_to_next_span: Option<Span>,
    /// Number of nested `(…)` groups this hit belongs to.
    pub group_membership: u8,
    /// Number of those groups that continue past this hit.
    pub group_continuation: u8,
    /// Whether `.` was present as a dotted-hit suffix.
    pub dotted: bool,
    /// Whether `..` was present as a double-dotted-hit suffix.
    pub double_dotted: bool,
    pub slur_group_close_at_duration: Option<u32>,
    /// The innermost `{...}` tuplet bracket this hit belongs to, if any.
    pub tuplet: Option<TupletInfo>,
}

impl ParsedPercussionHit {
    pub fn tie_to_next(&self) -> bool {
        self.tie_to_next_span.is_some()
    }
}

impl ParsedNote {
    pub fn tie_to_next(&self) -> bool {
        self.tie_to_next_span.is_some()
    }
}

impl ParsedChordNote {
    pub fn tie_to_next(&self) -> bool {
        self.tie_to_next_span.is_some()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedRest {
    /// Duration in quarter-beats. For dotted rests this already includes the added half-value.
    pub duration: u32,
    /// Whether `.` was present as a dotted-rest suffix.
    pub dotted: bool,
    /// Whether `..` was present as a double-dotted-rest suffix.
    pub double_dotted: bool,
    pub group_membership: u8,
    pub group_continuation: u8,
    /// The innermost `{...}` tuplet bracket this rest belongs to, if any.
    pub tuplet: Option<TupletInfo>,
    /// True when this rest was synthesized to fill a part not mentioned in
    /// this measure (see "Not-mentioned parts" in syntax.md), rather than
    /// written by the composer as an explicit `0`. Rendered with a distinct
    /// glyph — see `render_rest` in `src/renderer/new_renderer/glyph_renderers.rs`.
    pub implicit_fill: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Syllable {
    pub text: String,
    /// True if `-` follows this syllable in the lyrics section.
    pub held: bool,
    /// Source byte range of this syllable's own token, absolute within the
    /// whole document — lets the SVG preview map a clicked lyric
    /// syllable back to its source text, mirroring how `ParsedNote::event_span`
    /// does the same for notes (see `note_spans.rs`).
    pub span: Span,
}
