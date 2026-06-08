pub mod ast;
pub mod combiner;
pub mod desugar;
pub mod error;
pub mod error_reporter;
pub mod grouper;
pub mod layout;
pub mod parser;
pub mod renderer;
pub mod utils;

#[cfg(feature = "midi")]
pub mod midi;
#[cfg(feature = "pdf")]
pub mod pdf;
#[cfg(feature = "wav")]
pub mod wav;

use ast::grouped::{PartRow, Score};
use ast::parsed::PartKind;
use error::JianPuError;

/// A part declared in the `[parts]` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartInfo {
    /// Abbreviation used in score row labels and `--tracks` filtering.
    pub abbreviation: String,
    /// Full display name from the declaration left-hand side.
    pub display_name: String,
    /// Whether the part declaration includes a lyrics column.
    pub has_lyrics: bool,
}

/// Parse and group a `.jianpu` source string into a [`Score`].
pub fn compile(source: &str, filename: &str) -> Result<Score, JianPuError> {
    let doc = parser::parse(source, filename)?;
    grouper::group(doc)
}

/// Layout and render a [`Score`] into one SVG string per page.
pub fn render_svgs(score: &Score) -> Vec<String> {
    let row_height = score.metadata.row_height;
    let note_number_width = score.metadata.note_number_width;
    let pages = layout::layout(score, 595.0, 842.0);
    renderer::render(&pages, row_height, note_number_width)
}

/// Parse, group, and render a `.jianpu` source string into SVG page strings.
pub fn render_svgs_from_source(source: &str, filename: &str) -> Result<Vec<String>, JianPuError> {
    render_svgs_from_source_filtered(source, filename, None)
}

/// List part declarations from a `.jianpu` source string.
pub fn list_parts_from_source(source: &str, filename: &str) -> Result<Vec<PartInfo>, JianPuError> {
    let doc = parser::parse(source, filename)?;
    Ok(doc
        .declarations
        .into_iter()
        .map(|d| PartInfo {
            abbreviation: d.abbreviation,
            display_name: d.display_name,
            has_lyrics: d.kind == PartKind::NotesWithLyrics,
        })
        .collect())
}

/// Parse, group, optionally filter tracks, and render SVG page strings.
///
/// When `enabled_tracks` is `None`, all parts are rendered.
/// When `Some(tracks)` is empty, no parts are rendered.
pub fn render_svgs_from_source_filtered(
    source: &str,
    filename: &str,
    enabled_tracks: Option<&[String]>,
) -> Result<Vec<String>, JianPuError> {
    render_svgs_from_source_filtered_with_lyrics(source, filename, enabled_tracks, None)
}

/// Parse, group, optionally filter tracks and lyrics, and render SVG page strings.
///
/// When `enabled_tracks` is `None`, all parts are rendered.
/// When `Some(tracks)` is empty, no parts are rendered.
/// When `disabled_lyrics` lists part abbreviations, lyrics are hidden for those parts.
pub fn render_svgs_from_source_filtered_with_lyrics(
    source: &str,
    filename: &str,
    enabled_tracks: Option<&[String]>,
    disabled_lyrics: Option<&[String]>,
) -> Result<Vec<String>, JianPuError> {
    let mut score = compile(source, filename)?;
    apply_track_filter(&mut score, enabled_tracks);
    apply_lyrics_filter(&mut score, disabled_lyrics);
    Ok(render_svgs(&score))
}

/// Retain only parts whose names appear in `enabled_tracks`.
///
/// `None` keeps every part. `Some([])` removes every part.
pub fn apply_track_filter(score: &mut Score, enabled_tracks: Option<&[String]>) {
    let Some(tracks) = enabled_tracks else {
        return;
    };
    for measure in &mut score.measures {
        measure.parts.retain(|part| {
            part.name()
                .as_ref()
                .is_some_and(|name| tracks.contains(name))
        });
    }
}

/// Retain only parts whose names appear in `tracks`. No-op when `tracks` is empty.
pub fn filter_tracks(score: &mut Score, tracks: &[String]) {
    if tracks.is_empty() {
        return;
    }
    apply_track_filter(score, Some(tracks));
}

/// Hide lyrics on parts whose abbreviations appear in `disabled_lyrics`.
///
/// `None` and `Some([])` keep every lyric line.
pub fn apply_lyrics_filter(score: &mut Score, disabled_lyrics: Option<&[String]>) {
    let Some(tracks) = disabled_lyrics else {
        return;
    };
    if tracks.is_empty() {
        return;
    }
    for measure in &mut score.measures {
        for part in &mut measure.parts {
            if let PartRow::Notes(part_slice) = part {
                if part_slice
                    .name
                    .as_ref()
                    .is_some_and(|name| tracks.contains(name))
                {
                    part_slice.lyrics = None;
                }
            }
        }
    }
}

/// Parse, group, optionally filter tracks, and synthesize WAV bytes.
///
/// When `enabled_tracks` is `None`, all parts are included.
/// When `Some(tracks)` is empty, no parts are included.
#[cfg(feature = "wav")]
pub fn write_wav_from_source_filtered(
    source: &str,
    filename: &str,
    enabled_tracks: Option<&[String]>,
) -> Result<Vec<u8>, JianPuError> {
    let mut score = compile(source, filename)?;
    apply_track_filter(&mut score, enabled_tracks);
    let midi_bytes = midi::write_midi(&score)?;
    wav::write_wav(&midi_bytes)
}

/// Parse, group, optionally filter tracks, and write PDF bytes.
///
/// When `enabled_tracks` is `None`, all parts are included.
/// When `Some(tracks)` is empty, no parts are included.
#[cfg(feature = "pdf")]
pub fn write_pdf_from_source_filtered(
    source: &str,
    filename: &str,
    enabled_tracks: Option<&[String]>,
) -> Result<Vec<u8>, JianPuError> {
    write_pdf_from_source_filtered_with_lyrics(source, filename, enabled_tracks, None)
}

/// Parse, group, optionally filter tracks and lyrics, and write PDF bytes.
#[cfg(feature = "pdf")]
pub fn write_pdf_from_source_filtered_with_lyrics(
    source: &str,
    filename: &str,
    enabled_tracks: Option<&[String]>,
    disabled_lyrics: Option<&[String]>,
) -> Result<Vec<u8>, JianPuError> {
    let mut score = compile(source, filename)?;
    apply_track_filter(&mut score, enabled_tracks);
    apply_lyrics_filter(&mut score, disabled_lyrics);
    let svgs = render_svgs(&score);
    pdf::write_pdf(&svgs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_parts_from_source_returns_declarations() {
        let input = concat!(
            "[metadata]\n",
            "title = \"t\"\n",
            "author = \"a\"\n",
            "\n",
            "[parts]\n",
            "main = chord\n",
            "Alto 1 & Tenor (A1&T) = notes lyrics\n",
            "\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1m\n",
            "1 2 3 4\n",
            "a b c d\n",
        );
        let parts = list_parts_from_source(input, "test.jianpu").unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].abbreviation, "main");
        assert_eq!(parts[0].display_name, "main");
        assert_eq!(parts[1].abbreviation, "A1&T");
        assert_eq!(parts[1].display_name, "Alto 1 & Tenor");
        assert!(!parts[0].has_lyrics);
        assert!(parts[1].has_lyrics);
    }

    #[test]
    fn hidden_lyrics_do_not_reserve_lyric_row_space() {
        let input = concat!(
            "[metadata]\n",
            "title = \"t\"\n",
            "author = \"a\"\n",
            "\n",
            "[parts]\n",
            "Soprano = notes lyrics\n",
            "Alto = notes lyrics\n",
            "\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "sop sop sop sop\n",
            "5 6 7 1\n",
            "alt alt alt alt\n",
        );
        let all = render_svgs_from_source(input, "test.jianpu").unwrap();
        let alto_lyrics_hidden = render_svgs_from_source_filtered_with_lyrics(
            input,
            "test.jianpu",
            None,
            Some(&["Alto".into()]),
        )
        .unwrap();
        assert_ne!(
            all[0].len(),
            alto_lyrics_hidden[0].len(),
            "hiding one part's lyrics should change rendered SVG size"
        );
    }

    #[test]
    fn render_svgs_from_source_filtered_can_hide_lyrics_per_part() {
        let input = concat!(
            "[metadata]\n",
            "title = \"t\"\n",
            "author = \"a\"\n",
            "\n",
            "[parts]\n",
            "Soprano = notes lyrics\n",
            "Alto = notes lyrics\n",
            "\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "sop sop sop sop\n",
            "5 6 7 1\n",
            "alt alt alt alt\n",
        );
        let all = render_svgs_from_source(input, "test.jianpu").unwrap();
        let alto_lyrics_hidden = render_svgs_from_source_filtered_with_lyrics(
            input,
            "test.jianpu",
            None,
            Some(&["Alto".into()]),
        )
        .unwrap();
        assert!(all[0].contains("sop"));
        assert!(all[0].contains("alt"));
        assert!(alto_lyrics_hidden[0].contains("sop"));
        assert!(!alto_lyrics_hidden[0].contains("alt"));
    }

    #[test]
    fn render_svgs_from_source_filtered_can_hide_parts() {
        let input = concat!(
            "[metadata]\n",
            "title = \"t\"\n",
            "author = \"a\"\n",
            "\n",
            "[parts]\n",
            "Soprano = notes\n",
            "Alto = notes\n",
            "\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "5 6 7 1\n",
        );
        let all = render_svgs_from_source(input, "test.jianpu").unwrap();
        let soprano_only =
            render_svgs_from_source_filtered(input, "test.jianpu", Some(&["Soprano".into()]))
                .unwrap();
        assert_ne!(all[0], soprano_only[0]);
    }

    #[test]
    fn render_svgs_from_source_smoke() {
        let input = concat!(
            "[metadata]\n",
            "title = \"t\"\n",
            "author = \"a\"\n",
            "\n",
            "[parts]\n",
            "Melody = notes lyrics\n",
            "\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "a b c d\n",
        );
        let svgs = render_svgs_from_source(input, "test.jianpu").unwrap();
        assert_eq!(svgs.len(), 1);
        assert!(svgs[0].starts_with("<svg"));
        assert!(svgs[0].ends_with("</svg>"));
    }
}
