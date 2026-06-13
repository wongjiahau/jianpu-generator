use crate::ast::parsed::{
    Accidental, BassDegree, Extension, JianPuPitch, KeyChange, Syllable, TriadQuality,
};
use crate::error::Span;

// ── Public final types ────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Metadata {
    pub title: String,
    pub subtitle: Option<String>,
    pub author: String,
    /// Row height in points. Controls font sizes, dot radii, and all vertical spacing. Default: 24.
    pub row_height: u32,
    /// Maximum logical columns per row before wrapping. Default: 28.
    pub max_columns: u32,
    /// Left margin reserved for part labels in points. Default: 40.
    pub label_width: u32,
    /// Estimated rendered width of a single digit note number (0–9) in points. Default: 8.
    pub note_number_width: u32,
}

#[derive(Clone)]
pub struct Notes {
    pub events: Vec<NoteEvent>,
}

#[derive(Clone)]
pub struct Lyrics {
    pub syllables: Vec<Syllable>,
}

#[derive(Clone)]
pub struct PartSlice {
    pub name: Option<String>,
    pub kind: crate::ast::parsed::PartKind,
    pub notes: Notes,
    pub lyrics: Option<Lyrics>,
}

#[derive(Clone)]
pub struct MultiPartMeasure {
    pub time_signature: Option<TimeSignature>,
    pub bpm: Option<u32>,
    // TODO: key-change rendering (1=X label) is not yet implemented in layout/renderer
    pub key: Option<KeyChange>,
    pub label: Option<String>,
    pub parts: Vec<PartRow>,
    /// Byte range of this measure's note events in the original source.
    /// Used to map editor cursor position to a measure index.
    pub source_span: Span,
}

#[derive(Clone)]
pub enum PartRow {
    Timed(PartSlice),
    /// All of this part's input lines were `"` (or implicit trailing omission)
    /// in this measure. Carries the resolved content so audio output still
    /// includes it, but the renderer skips the row entirely.
    Ditto(PartSlice),
}

impl PartRow {
    pub fn name(&self) -> Option<&String> {
        self.slice().name.as_ref()
    }

    /// The resolved content, whether rendered or not.
    pub fn slice(&self) -> &PartSlice {
        match self {
            PartRow::Timed(s) | PartRow::Ditto(s) => s,
        }
    }

    pub fn slice_mut(&mut self) -> &mut PartSlice {
        match self {
            PartRow::Timed(s) | PartRow::Ditto(s) => s,
        }
    }

    /// Content to render; `None` for ditto rows.
    pub fn rendered_slice(&self) -> Option<&PartSlice> {
        match self {
            PartRow::Timed(s) => Some(s),
            PartRow::Ditto(_) => None,
        }
    }

    pub fn is_ditto(&self) -> bool {
        matches!(self, PartRow::Ditto(_))
    }
}

pub(crate) enum GroupedTrack {
    Timed(GroupedPart),
}

impl GroupedTrack {
    pub(crate) fn measure_count(&self) -> usize {
        match self {
            GroupedTrack::Timed(part) => part.measures.len(),
        }
    }

    pub(crate) fn track_name(&self) -> &Option<String> {
        match self {
            GroupedTrack::Timed(part) => &part.name,
        }
    }
}

#[derive(Clone)]
pub struct Score {
    pub metadata: Metadata,
    pub measures: Vec<MultiPartMeasure>,
}

// ── Intermediate grouper types (not part of the public API) ─────────────────

#[allow(dead_code)]
pub(crate) struct MeasureDirectives {
    pub(crate) time_signature: Option<TimeSignature>,
    pub(crate) bpm: Option<u32>,
    pub(crate) key: Option<KeyChange>,
    pub(crate) label: Option<String>,
}

#[allow(dead_code)]
pub(crate) struct GroupedScore {
    pub(crate) measure_directives: Vec<MeasureDirectives>,
    pub(crate) parts: Vec<GroupedTrack>,
}

pub(crate) struct GroupedMeasure {
    pub(crate) notes: Notes,
    pub(crate) source_span: Span,
}

pub(crate) struct GroupedPart {
    pub(crate) name: Option<String>,
    pub(crate) kind: crate::ast::parsed::PartKind,
    pub(crate) measures: Vec<GroupedMeasure>,
    /// Flat lyrics list. `None` means no [lyrics] section was provided.
    pub(crate) lyrics: Option<Vec<Syllable>>,
    /// Per-measure flag: true when every input line of this part in that
    /// measure was a `"` ditto (explicit or implicit trailing omission).
    pub(crate) ditto_measures: Vec<bool>,
    /// Per-measure flag: true when this part's lyric line in that measure
    /// was a `"` ditto. The copied lyrics duplicate the part above, so the
    /// lyric row is not rendered and its space is reclaimed.
    pub(crate) lyrics_ditto_measures: Vec<bool>,
}

// ── Shared note types ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct TimeSignature {
    pub numerator: u8,
    pub denominator: u8,
}

#[derive(Clone)]
pub enum NoteEvent {
    Note(GroupedNote),
    Rest(GroupedRest),
    Chord(GroupedChordNote),
}

#[derive(Clone)]
pub struct GroupedChordNote {
    pub degree: JianPuPitch,
    pub accidental: Accidental,
    pub triad: TriadQuality,
    pub extension: Option<Extension>,
    pub bass: Option<BassDegree>,
    pub duration: u32,
    pub tie: bool,
    pub group_membership: u8,
    pub group_continuation: u8,
    pub dotted: bool,
    pub slur_group_close_at_duration: Option<u32>,
}

#[derive(Clone)]
pub struct GroupedNote {
    pub pitch: JianPuPitch,
    pub octave: i8,
    /// Duration in quarter-beats, including any beats added by `-` extensions.
    pub duration: u32,
    /// True if this note is tied/slurred to the next note.
    pub tie: bool,
    /// Number of nested `(…)` groups this note belongs to.
    pub group_membership: u8,
    /// Number of those groups that continue past this note.
    pub group_continuation: u8,
    /// True if this note was written with `*` (dotted duration).
    pub dotted: bool,
    pub slur_group_close_at_duration: Option<u32>,
}

impl GroupedChordNote {
    pub fn format_symbol(&self) -> String {
        use crate::ast::parsed::{Accidental, Extension, JianPuPitch, TriadQuality};

        let degree = match self.degree {
            JianPuPitch::One => '1',
            JianPuPitch::Two => '2',
            JianPuPitch::Three => '3',
            JianPuPitch::Four => '4',
            JianPuPitch::Five => '5',
            JianPuPitch::Six => '6',
            JianPuPitch::Seven => '7',
        };
        let accidental = match self.accidental {
            Accidental::Sharp => "♯",
            Accidental::Flat => "♭",
            Accidental::Natural => "",
        };
        let triad = match self.triad {
            TriadQuality::Major => "",
            TriadQuality::Minor => "m",
            TriadQuality::Diminished => "°",
            TriadQuality::Augmented => "⁺",
        };
        let extension = match &self.extension {
            Some(Extension::DominantSeventh) => "⁷",
            Some(Extension::MajorSeventh) => "△⁷",
            None => "",
        };
        let mut result = format!("{degree}{accidental}{triad}{extension}");

        if let Some(bass) = &self.bass {
            let bass_degree = match bass.degree {
                JianPuPitch::One => '1',
                JianPuPitch::Two => '2',
                JianPuPitch::Three => '3',
                JianPuPitch::Four => '4',
                JianPuPitch::Five => '5',
                JianPuPitch::Six => '6',
                JianPuPitch::Seven => '7',
            };
            let bass_acc = match bass.accidental {
                Accidental::Sharp => "♯",
                Accidental::Flat => "♭",
                Accidental::Natural => "",
            };
            result.push('/');
            result.push(bass_degree);
            result.push_str(bass_acc);
        }

        result
    }
}

#[derive(Clone)]
pub struct GroupedRest {
    /// Duration in quarter-beats, including any beats added by `-` extensions.
    pub duration: u32,
    /// True if this rest was written with `*` (dotted duration). Reserved for future use.
    #[allow(dead_code)]
    pub dotted: bool,
    pub group_membership: u8,
    pub group_continuation: u8,
}
