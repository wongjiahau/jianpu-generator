use crate::ast::parsed::{ParsedTimedTrack, ParsedTrack, PartDecl, PartKind};
use crate::error::JianPuError;

/// Convenience wrapper that calls `parse` and returns only the tracks,
/// discarding the directive-events accumulator. Used in unit tests.
pub(super) fn parse(
    content: &str,
    base_offset: usize,
    declarations: &[PartDecl],
) -> Result<Vec<ParsedTrack>, JianPuError> {
    super::parse(content, base_offset, declarations).map(|(tracks, _)| tracks)
}

/// Like `parse`, but also returns the directive-events-per-measure accumulator.
#[allow(dead_code)]
pub(super) fn parse_with_directives(
    content: &str,
    base_offset: usize,
    declarations: &[PartDecl],
) -> Result<(Vec<ParsedTrack>, super::DirectiveEventsPerMeasure), JianPuError> {
    super::parse(content, base_offset, declarations)
}

pub(super) fn decl(name: &str, kind: PartKind) -> PartDecl {
    PartDecl {
        abbreviation: name.into(),
        display_name: name.into(),
        kind,
    }
}

pub(super) fn timed_track<'a>(tracks: &'a [ParsedTrack], abbrev: &str) -> &'a ParsedTimedTrack {
    tracks
        .iter()
        .find_map(|t| match t {
            ParsedTrack::Timed(n) if n.abbreviation == abbrev => Some(n),
            ParsedTrack::Timed(_) => None,
        })
        .unwrap_or_else(|| panic!("timed track '{abbrev}' not found"))
}

pub(super) fn notes_track<'a>(tracks: &'a [ParsedTrack], abbrev: &str) -> &'a ParsedTimedTrack {
    timed_track(tracks, abbrev)
}

pub(super) fn chord_track<'a>(tracks: &'a [ParsedTrack], abbrev: &str) -> &'a ParsedTimedTrack {
    timed_track(tracks, abbrev)
}
