use crate::error::IrrecoverableError;
use crate::parser::parts_parser::InstrumentInfo;
use crate::{apply_track_filter, compile};

/// Parse, group, optionally filter tracks, and synthesize WAV bytes.
///
/// When `enabled_tracks` is `None`, all parts are included.
/// When `Some(tracks)` is empty, no parts are included.
#[cfg(feature = "wav")]
pub fn write_wav_from_source_filtered(
    source: &str,
    filename: &str,
    enabled_tracks: Option<&[String]>,
    sf2_bytes: &[u8],
    instruments: &[InstrumentInfo],
) -> Result<Vec<u8>, IrrecoverableError> {
    let mut score = compile(source, filename, instruments)?;
    apply_track_filter(&mut score, enabled_tracks);
    let score = crate::midi::expand_navigation(&score)?;
    let midi_bytes = crate::midi::write_midi(&score)?;
    crate::wav::write_wav(&midi_bytes, sf2_bytes, None)
}

/// Parse, group, optionally filter tracks, and synthesize MP3 bytes.
///
/// When `enabled_tracks` is `None`, all parts are included.
/// When `Some(tracks)` is empty, no parts are included.
#[cfg(feature = "mp3")]
pub fn write_mp3_from_source_filtered(
    source: &str,
    filename: &str,
    enabled_tracks: Option<&[String]>,
    sf2_bytes: &[u8],
    instruments: &[InstrumentInfo],
) -> Result<Vec<u8>, IrrecoverableError> {
    let mut score = compile(source, filename, instruments)?;
    apply_track_filter(&mut score, enabled_tracks);
    let score = crate::midi::expand_navigation(&score)?;
    let midi_bytes = crate::midi::write_midi(&score)?;
    crate::wav::write_mp3(&midi_bytes, sf2_bytes, None)
}

/// A measure range to synthesize, plus how its end measure should be
/// resolved when it recurs later in the performance (due to a repeat/jump).
///
/// When `extend_to_last_occurrence` is `true`, the end measure is extended
/// through its last occurrence — this is what the web app's "play from
/// current measure" (which always passes the score's literal last written
/// measure as the range end) needs to follow the repeat to the true end.
/// When `false`, the range stops at the end measure's first occurrence at or
/// after the start — what an exact range selection (e.g. "play current
/// measure") needs to avoid overrunning into a later repeat/jump pass.
///
/// When `respect_sequence` is `false`, D.C./D.S. markers and `# sequence`
/// (and any part omissions it applies) are ignored: the range plays exactly
/// as written. This is what "play current measure" needs — it always plays
/// what is written, regardless of how `# sequence` might otherwise reorder
/// or omit parts for that measure's occurrence(s). "Play from current
/// measure" passes `true` so it follows `# sequence` to the true end.
///
/// `sequence_entry_range`, when `Some`, names the exact `# sequence`
/// entry/entries `range` refers to by their 0-based index into
/// `score.sequence` (the order entries are written in `# sequence`) rather
/// than leaving `range`'s written measure indices to be resolved by
/// earliest/last-occurrence search. This is what the sequence-jump
/// toolbar's "play selected sequence range" needs: without it, a repeated
/// label (e.g. `A, B(-x), B`) can't be disambiguated, since every
/// occurrence of `B` shares the same written measure range and search
/// always finds the first one.
pub struct MeasureRangeSelection {
    pub range: std::ops::RangeInclusive<usize>,
    pub extend_to_last_occurrence: bool,
    pub respect_sequence: bool,
    pub sequence_entry_range: Option<std::ops::RangeInclusive<usize>>,
}

/// Track-mute and clip-trim options for
/// [`write_wav_for_measure_range_from_source`], grouped into one param to
/// stay under the crate's argument-count limit.
#[cfg(feature = "wav")]
pub struct MeasureRangeAudioOptions<'a> {
    pub enabled_tracks: Option<&'a [String]>,
    /// When `Some`, narrows the returned clip down to that elapsed-seconds
    /// window (plus a short release tail, sample-accurately faded at the
    /// cut points — see [`crate::wav::TrimWindow`]) instead of playing the
    /// range's full boundary measures — what the web app's "play
    /// selection" needs to play only the range-selected notes' actual time
    /// span. The boundary measures are still synthesized in full either
    /// way, since that's needed for correct tempo/key context and to leave
    /// room for the trimmed clip's own release tail.
    pub trim: Option<crate::wav::TrimWindow>,
}

/// Parse, group, optionally filter tracks, and synthesize WAV for a consecutive measure range.
///
/// BPM and key context is accumulated from all measures before the range's start.
#[cfg(feature = "wav")]
pub fn write_wav_for_measure_range_from_source(
    source: &str,
    filename: &str,
    selection: &MeasureRangeSelection,
    options: &MeasureRangeAudioOptions,
    sf2_bytes: &[u8],
    instruments: &[InstrumentInfo],
) -> Result<Vec<u8>, IrrecoverableError> {
    let mut score = compile(source, filename, instruments)?;
    apply_track_filter(&mut score, options.enabled_tracks);
    let (score, start, end) = crate::midi::expand_for_measure_range(
        &score,
        *selection.range.start(),
        *selection.range.end(),
        selection.extend_to_last_occurrence,
        selection.respect_sequence,
        selection.sequence_entry_range.clone(),
    )?;
    let midi_bytes = crate::midi::write_midi_for_measure_range(&score, start, end)?;
    crate::wav::write_wav(&midi_bytes, sf2_bytes, options.trim)
}

/// Parse, group, optionally filter tracks, and synthesize MP3 for a consecutive measure range.
///
/// BPM and key context is accumulated from all measures before the range's start.
#[cfg(feature = "mp3")]
pub fn write_mp3_for_measure_range_from_source(
    source: &str,
    filename: &str,
    selection: &MeasureRangeSelection,
    options: &MeasureRangeAudioOptions,
    sf2_bytes: &[u8],
    instruments: &[InstrumentInfo],
) -> Result<Vec<u8>, IrrecoverableError> {
    let mut score = compile(source, filename, instruments)?;
    apply_track_filter(&mut score, options.enabled_tracks);
    let (score, start, end) = crate::midi::expand_for_measure_range(
        &score,
        *selection.range.start(),
        *selection.range.end(),
        selection.extend_to_last_occurrence,
        selection.respect_sequence,
        selection.sequence_entry_range.clone(),
    )?;
    let midi_bytes = crate::midi::write_midi_for_measure_range(&score, start, end)?;
    crate::wav::write_mp3(&midi_bytes, sf2_bytes, options.trim)
}

/// Parse, group, optionally filter tracks, and compute the elapsed-seconds
/// offset of each measure boundary in playback order (length = playback
/// measure count + 1; the last entry is the total duration), following
/// `# sequence`/D.C.-al-Coda-Fine navigation the same way the actual audio
/// does (see [`crate::midi::expand_navigation`]).
#[cfg(feature = "midi")]
pub fn measure_start_times_from_source(
    source: &str,
    filename: &str,
    enabled_tracks: Option<&[String]>,
    instruments: &[InstrumentInfo],
) -> Result<Vec<f64>, IrrecoverableError> {
    let mut score = compile(source, filename, instruments)?;
    apply_track_filter(&mut score, enabled_tracks);
    let score = crate::midi::expand_navigation(&score)?;
    crate::midi::measure_start_times_seconds(&score)
}

/// Parse, group, optionally filter tracks, and compute the elapsed-seconds
/// start/end of every sounding note/rest, keyed by `(source_part_index,
/// note_id)`. Used to drive the SVG preview's per-part, per-note playback
/// cursor (see [`crate::midi::NoteTiming`]).
///
/// Follows the same `# sequence`/D.C.-al-Coda-Fine playback order as the
/// actual audio (see [`crate::midi::note_timings_seconds`]), so a repeated
/// or reordered written note is highlighted every time it's actually heard,
/// not just its first pass.
///
/// `visible_tracks` must be the part-visibility toggle's own state (the same
/// set physically removed before the rendered SVG's own `compile()` call —
/// see [`crate::filters::apply_track_filter`]), so `note_id`s and block
/// structure (e.g. a `MultiMeasureRest` run only created once a sibling
/// part's notes are hidden) agree with the rendered SVG's `data-note-id`.
/// `enabled_tracks` separately mutes playback down to a further, possibly
/// narrower subset of those visible parts for this one call only (e.g. the
/// web app's note range-select playback) without affecting `source_part_index`
/// or block structure — see [`crate::midi::note_timings_seconds`].
#[cfg(feature = "midi")]
pub fn note_timings_from_source(
    source: &str,
    filename: &str,
    visible_tracks: Option<&[String]>,
    enabled_tracks: Option<&[String]>,
    instruments: &[InstrumentInfo],
) -> Result<Vec<crate::midi::NoteTiming>, IrrecoverableError> {
    let score = compile(source, filename, instruments)?;
    // `score` is deliberately left unfiltered here: `note_timings_seconds`
    // applies `visible_tracks`/`enabled_tracks` itself, resolving each kept
    // note's `source_part_index` against the visibility-filtered score (see
    // `visible_score`) rather than the fully unfiltered written one.
    crate::midi::note_timings_seconds(&score, visible_tracks, enabled_tracks)
}

/// Same as [`note_timings_from_source`], but scoped to a measure range and
/// relative to the start of that range (`start_s`/`end_s` are seconds from
/// the start of the clip [`write_wav_for_measure_range_from_source`]
/// produces for the same range, not from the start of the whole piece).
///
/// See [`MeasureRangeSelection`] for `extend_to_last_occurrence` and
/// `respect_sequence`, and [`note_timings_from_source`] for how
/// `visible_tracks` differs from `enabled_tracks`.
#[cfg(feature = "midi")]
pub fn note_timings_for_range_from_source(
    source: &str,
    filename: &str,
    selection: &MeasureRangeSelection,
    visible_tracks: Option<&[String]>,
    enabled_tracks: Option<&[String]>,
    instruments: &[InstrumentInfo],
) -> Result<Vec<crate::midi::NoteTiming>, IrrecoverableError> {
    let score = compile(source, filename, instruments)?;
    // As in `note_timings_from_source`, `score` stays unfiltered — filtering
    // happens inside `note_timings_seconds_for_range`/`_for_literal_range` so
    // `source_part_index` keeps its true visibility-filtered value.
    let (_, start_pos, end_pos) = crate::midi::expand_for_measure_range(
        &score,
        *selection.range.start(),
        *selection.range.end(),
        selection.extend_to_last_occurrence,
        selection.respect_sequence,
        selection.sequence_entry_range.clone(),
    )?;
    if !selection.respect_sequence {
        return crate::midi::note_timings_seconds_for_literal_range(
            &score,
            start_pos,
            end_pos,
            visible_tracks,
            enabled_tracks,
        );
    }
    crate::midi::note_timings_seconds_for_range(
        &score,
        start_pos,
        end_pos,
        visible_tracks,
        enabled_tracks,
    )
}

/// Parse, group, optionally filter tracks, and generate MIDI (SMF) bytes.
///
/// When `enabled_tracks` is `None`, all parts are included.
/// When `Some(tracks)` is empty, no parts are included.
#[cfg(feature = "midi")]
pub fn write_midi_from_source_filtered(
    source: &str,
    filename: &str,
    enabled_tracks: Option<&[String]>,
    instruments: &[InstrumentInfo],
) -> Result<Vec<u8>, IrrecoverableError> {
    let mut score = compile(source, filename, instruments)?;
    apply_track_filter(&mut score, enabled_tracks);
    let score = crate::midi::expand_navigation(&score)?;
    crate::midi::write_midi(&score)
}
