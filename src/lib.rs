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

/// Sanitize a track name for use in filenames (mirrors CLI).
pub fn sanitize_track_name(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "-")
}

/// Collect unique part names from score measures (order of first appearance).
pub fn collect_track_names(score: &Score) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut names = Vec::new();
    for measure in &score.measures {
        for part in &measure.parts {
            if let Some(name) = part.name() {
                if seen.insert(name.clone()) {
                    names.push(name.clone());
                }
            }
        }
    }
    names
}

/// Build a split-track PDF filename: `{base_name} - {track}.pdf`.
pub fn split_pdf_filename(base_name: &str, track: &str) -> String {
    format!("{} - {}.pdf", base_name, sanitize_track_name(track))
}

/// Track list for split export. Empty `tracks_filter` → all score tracks;
/// falls back to `[parts]` declaration abbreviations when score has no named parts.
pub fn split_track_names(
    source: &str,
    filename: &str,
    score: &Score,
    tracks_filter: &[String],
) -> Result<Vec<String>, JianPuError> {
    let mut names = if tracks_filter.is_empty() {
        collect_track_names(score)
    } else {
        tracks_filter.to_vec()
    };
    if names.is_empty() {
        names = list_parts_from_source(source, filename)?
            .into_iter()
            .map(|part| part.abbreviation)
            .collect();
    }
    Ok(names)
}

/// One PDF produced by split-track export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitPdfEntry {
    pub track_name: String,
    pub filename: String,
    pub pdf: Vec<u8>,
}

/// Parse once, render one PDF per track (CLI `--split-tracks` semantics).
///
/// `tracks_filter`: empty → all tracks; non-empty → only listed abbreviations.
/// Lyrics are always included (no lyrics filter).
#[cfg(feature = "pdf")]
pub fn write_split_pdfs_from_source(
    source: &str,
    filename: &str,
    base_name: &str,
    tracks_filter: &[String],
) -> Result<Vec<SplitPdfEntry>, JianPuError> {
    let score = compile(source, filename)?;
    let track_names = split_track_names(source, filename, &score, tracks_filter)?;
    let mut entries = Vec::with_capacity(track_names.len());
    for track in track_names {
        let mut score_clone = score.clone();
        filter_tracks(&mut score_clone, std::slice::from_ref(&track));
        let svgs = render_svgs(&score_clone);
        let pdf = pdf::write_pdf(&svgs)?;
        entries.push(SplitPdfEntry {
            track_name: track.clone(),
            filename: split_pdf_filename(base_name, &track),
            pdf,
        });
    }
    Ok(entries)
}

#[cfg(feature = "pdf")]
pub fn zip_split_pdfs(entries: &[SplitPdfEntry]) -> Result<Vec<u8>, JianPuError> {
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    let mut buffer = Vec::new();
    {
        let mut writer = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for entry in entries {
            writer.start_file(&entry.filename, options).map_err(|e| {
                JianPuError::new(error::Span::new(0, 0), format!("zip start_file: {e}"))
            })?;
            writer.write_all(&entry.pdf).map_err(|e| {
                JianPuError::new(error::Span::new(0, 0), format!("zip write: {e}"))
            })?;
        }
        writer.finish().map_err(|e| {
            JianPuError::new(error::Span::new(0, 0), format!("zip finish: {e}"))
        })?;
    }
    Ok(buffer)
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

    #[test]
    fn split_track_names_falls_back_to_part_declarations() {
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
        let score = compile(input, "test.jianpu").unwrap();
        let names = split_track_names(input, "test.jianpu", &score, &[]).unwrap();
        assert_eq!(names, vec!["Melody"]);
    }

    #[test]
    fn split_pdf_filename_sanitizes_track_name() {
        assert_eq!(split_pdf_filename("song", "A1&T"), "song - A1&T.pdf");
        assert_eq!(
            split_pdf_filename("song", "bad/name"),
            "song - bad-name.pdf"
        );
    }

    #[cfg(feature = "pdf")]
    mod split_pdf_tests {
        use super::*;
        use std::io::Read;
        use zip::ZipArchive;

        fn multi_track_input() -> &'static str {
            concat!(
                "[metadata]\n",
                "title = \"test score\"\n",
                "author = \"tester\"\n",
                "\n",
                "[parts]\n",
                "Soprano 1 (S1) = notes lyrics\n",
                "Soprano 2 (S2) = notes lyrics\n",
                "\n",
                "[score]\n",
                "(time=4/4 key=C4 bpm=120)\n",
                "1 2 3 4\n",
                "do re mi fa\n",
                "5 6 7 1\n",
                "sol la ti do\n",
            )
        }

        #[test]
        fn write_split_pdfs_from_source_produces_one_pdf_per_track() {
            let entries = write_split_pdfs_from_source(
                multi_track_input(),
                "test.jianpu",
                "test_split",
                &[],
            )
            .unwrap();
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0].track_name, "S1");
            assert_eq!(entries[0].filename, "test_split - S1.pdf");
            assert_eq!(entries[1].track_name, "S2");
            assert_eq!(entries[1].filename, "test_split - S2.pdf");
            assert_eq!(&entries[0].pdf[0..4], b"%PDF");
            assert_eq!(&entries[1].pdf[0..4], b"%PDF");
        }

        #[test]
        fn write_split_pdfs_from_source_single_part_uses_split_naming() {
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
            let entries =
                write_split_pdfs_from_source(input, "test.jianpu", "song", &[]).unwrap();
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].filename, "song - Melody.pdf");
            assert_eq!(&entries[0].pdf[0..4], b"%PDF");
        }

        #[test]
        fn write_split_pdfs_from_source_invalid_source_errors() {
            let err =
                write_split_pdfs_from_source("not valid", "test.jianpu", "song", &[]).unwrap_err();
            assert!(!err.message.is_empty());
        }

        #[test]
        fn zip_split_pdfs_contains_named_entries() {
            let entries = write_split_pdfs_from_source(
                multi_track_input(),
                "test.jianpu",
                "test_split",
                &[],
            )
            .unwrap();
            let zip_bytes = zip_split_pdfs(&entries).unwrap();
            assert_eq!(&zip_bytes[0..2], b"PK");

            let cursor = std::io::Cursor::new(zip_bytes);
            let mut archive = ZipArchive::new(cursor).unwrap();
            assert_eq!(archive.len(), 2);
            let mut names: Vec<String> = (0..archive.len())
                .map(|i| archive.by_index(i).unwrap().name().to_string())
                .collect();
            names.sort();
            assert_eq!(
                names,
                vec![
                    "test_split - S1.pdf".to_string(),
                    "test_split - S2.pdf".to_string()
                ]
            );

            let mut first = archive.by_name("test_split - S1.pdf").unwrap();
            let mut buf = Vec::new();
            first.read_to_end(&mut buf).unwrap();
            assert_eq!(&buf[0..4], b"%PDF");
        }
    }
}
