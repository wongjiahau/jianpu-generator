pub mod ast;
pub mod combiner;
pub mod desugar;
pub mod error;
pub mod error_reporter;
pub mod grouper;
pub mod grouping;
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

use ast::grouped::Score;
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
            let part_slice = part.slice_mut();
            if part_slice
                .name
                .as_ref()
                .is_some_and(|name| tracks.contains(name))
            {
                part_slice.lyrics = None;
                if part_slice.kind == PartKind::NotesWithLyrics {
                    part_slice.kind = PartKind::Notes;
                }
            }
        }
    }
}

/// Sanitize a track name for use in filenames (mirrors CLI).
pub fn sanitize_track_name(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "-")
}

/// Abbreviation → display name from `[parts]` declarations.
pub fn part_display_name_map(
    source: &str,
    filename: &str,
) -> Result<std::collections::HashMap<String, String>, JianPuError> {
    Ok(list_parts_from_source(source, filename)?
        .into_iter()
        .map(|part| (part.abbreviation, part.display_name))
        .collect())
}

/// Resolve the filename label for a track (display name when declared, else abbreviation).
pub fn split_track_label(
    display_names: &std::collections::HashMap<String, String>,
    abbreviation: &str,
) -> String {
    display_names
        .get(abbreviation)
        .cloned()
        .unwrap_or_else(|| abbreviation.to_string())
}

/// Build a split-track filename: `{base_name} - {label}.{extension}`.
pub fn split_track_filename(base_name: &str, label: &str, extension: &str) -> String {
    format!(
        "{} - {}.{}",
        base_name,
        sanitize_track_name(label),
        extension
    )
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

/// Build a split-track PDF filename: `{base_name} - {label}.pdf`.
pub fn split_pdf_filename(base_name: &str, label: &str) -> String {
    split_track_filename(base_name, label, "pdf")
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
    let display_names = part_display_name_map(source, filename)?;
    let mut entries = Vec::with_capacity(track_names.len());
    for track in track_names {
        let mut score_clone = score.clone();
        filter_tracks(&mut score_clone, std::slice::from_ref(&track));
        let svgs = render_svgs(&score_clone);
        let pdf = pdf::write_pdf(&svgs)?;
        let label = split_track_label(&display_names, &track);
        entries.push(SplitPdfEntry {
            track_name: track.clone(),
            filename: split_pdf_filename(base_name, &label),
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
            writer
                .write_all(&entry.pdf)
                .map_err(|e| JianPuError::new(error::Span::new(0, 0), format!("zip write: {e}")))?;
        }
        writer
            .finish()
            .map_err(|e| JianPuError::new(error::Span::new(0, 0), format!("zip finish: {e}")))?;
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
    use ast::grouped::PartRow;

    // ── Output-ditto tests ────────────────────────────────────────────────────

    #[test]
    fn explicit_ditto_part_is_marked_as_ditto_in_score() {
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
            "\"\n",
        );
        let score = compile(input, "test.jianpu").unwrap();
        assert!(
            matches!(score.measures[0].parts[1], PartRow::Ditto(_)),
            "Alto part written as `\"` ditto should be PartRow::Ditto"
        );
    }

    #[test]
    fn implicit_ditto_part_is_marked_as_ditto_in_score() {
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
            // Alto line omitted — implicit ditto
        );
        let score = compile(input, "test.jianpu").unwrap();
        assert!(
            matches!(score.measures[0].parts[1], PartRow::Ditto(_)),
            "Alto part from implicit trailing omission should be PartRow::Ditto"
        );
    }

    #[test]
    fn non_ditto_part_is_timed_in_score() {
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
        let score = compile(input, "test.jianpu").unwrap();
        assert!(
            matches!(score.measures[0].parts[0], PartRow::Timed(_)),
            "Soprano with explicit notes should be PartRow::Timed"
        );
        assert!(
            matches!(score.measures[0].parts[1], PartRow::Timed(_)),
            "Alto with explicit notes should be PartRow::Timed"
        );
    }

    #[test]
    fn ditto_parts_produce_smaller_svg_than_non_ditto() {
        let with_ditto = concat!(
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
            "\"\n",
        );
        let without_ditto = concat!(
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
        let svgs_ditto = render_svgs_from_source(with_ditto, "test.jianpu").unwrap();
        let svgs_no_ditto = render_svgs_from_source(without_ditto, "test.jianpu").unwrap();
        assert!(
            svgs_ditto[0].len() < svgs_no_ditto[0].len(),
            "SVG with ditto Alto should be smaller than SVG with explicit Alto notes"
        );
    }

    #[test]
    fn measures_with_different_ditto_patterns_go_on_separate_rows() {
        // Measure 1: both Soprano and Alto active.
        // Measure 2: Soprano active, Alto ditto'd.
        // Both fit in 28 columns but must be on separate rows because
        // their active-part sets differ.
        let input = concat!(
            "[metadata]\n",
            "title = \"t\"\n",
            "author = \"a\"\n",
            "max columns = 60\n",
            "\n",
            "[parts]\n",
            "Soprano = notes\n",
            "Alto = notes\n",
            "\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "5 6 7 1\n",
            "\n",
            "1 2 3 4\n",
            "\"\n",
        );
        let score = compile(input, "test.jianpu").unwrap();
        let pages = layout::layout(&score, 595.0, 842.0);
        let total_row_groups: usize = pages.iter().map(|p| p.row_groups.len()).sum();
        assert_eq!(
            total_row_groups, 2,
            "measures with different ditto patterns should be forced onto separate rows"
        );
    }

    #[test]
    fn measures_with_same_ditto_pattern_can_share_a_row() {
        // Both measures have Alto ditto'd — they should share a single row.
        let input = concat!(
            "[metadata]\n",
            "title = \"t\"\n",
            "author = \"a\"\n",
            "max columns = 60\n",
            "\n",
            "[parts]\n",
            "Soprano = notes\n",
            "Alto = notes\n",
            "\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "\"\n",
            "\n",
            "5 6 7 1\n",
            "\"\n",
        );
        let score = compile(input, "test.jianpu").unwrap();
        let pages = layout::layout(&score, 595.0, 842.0);
        let total_row_groups: usize = pages.iter().map(|p| p.row_groups.len()).sum();
        assert_eq!(
            total_row_groups, 1,
            "two measures with the same ditto pattern should share a single row"
        );
    }

    #[test]
    fn same_ditto_count_but_different_parts_still_forces_line_break() {
        // Measure 1: Alto ditto'd (S + T active).
        // Measure 2: Tenor ditto'd (S + A active).
        // Both rows would have identical heights — the break decision must
        // compare WHICH parts are active, not how many.
        let input = concat!(
            "[metadata]\n",
            "title = \"t\"\n",
            "author = \"a\"\n",
            "max columns = 60\n",
            "\n",
            "[parts]\n",
            "Soprano = notes\n",
            "Alto = notes\n",
            "Tenor = notes\n",
            "\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "\"\n",
            "5 6 7 1\n",
            "\n",
            "1 2 3 4\n",
            "5 6 7 1\n",
            "\"\n",
        );
        let score = compile(input, "test.jianpu").unwrap();
        let pages = layout::layout(&score, 595.0, 842.0);
        let total_row_groups: usize = pages.iter().map(|p| p.row_groups.len()).sum();
        assert_eq!(
            total_row_groups, 2,
            "same ditto count but different ditto'd parts must not share a row"
        );
    }

    #[test]
    fn alternating_ditto_patterns_force_break_at_every_change() {
        // Patterns: [S,A] → [S] → [S,A]. Each change forces a break,
        // including returning to a previously seen pattern.
        let input = concat!(
            "[metadata]\n",
            "title = \"t\"\n",
            "author = \"a\"\n",
            "max columns = 60\n",
            "\n",
            "[parts]\n",
            "Soprano = notes\n",
            "Alto = notes\n",
            "\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "5 6 7 1\n",
            "\n",
            "1 2 3 4\n",
            "\"\n",
            "\n",
            "5 6 7 1\n",
            "1 2 3 4\n",
        );
        let score = compile(input, "test.jianpu").unwrap();
        let pages = layout::layout(&score, 595.0, 842.0);
        let total_row_groups: usize = pages.iter().map(|p| p.row_groups.len()).sum();
        assert_eq!(
            total_row_groups, 3,
            "every ditto-pattern change should start a new row"
        );
    }

    #[test]
    fn width_wrapping_still_applies_within_same_ditto_pattern() {
        // Many measures sharing one ditto pattern must still wrap when the
        // row runs out of columns — pattern-matching must not disable
        // ordinary width-based wrapping.
        let mut input = String::from(concat!(
            "[metadata]\n",
            "title = \"t\"\n",
            "author = \"a\"\n",
            "max columns = 28\n",
            "\n",
            "[parts]\n",
            "Soprano = notes\n",
            "Alto = notes\n",
            "\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
        ));
        for _ in 0..6 {
            input.push_str("1 2 3 4\n\"\n\n");
        }
        let score = compile(&input, "test.jianpu").unwrap();
        let pages = layout::layout(&score, 595.0, 842.0);
        let total_row_groups: usize = pages.iter().map(|p| p.row_groups.len()).sum();
        assert!(
            total_row_groups > 1,
            "six 16-beat measures cannot fit one 28-column row; width wrapping must still occur"
        );
    }

    #[test]
    fn partially_ditto_part_counts_as_active_for_line_breaking() {
        // Alto's notes line is ditto but its lyrics line is explicit —
        // the part still renders, so its pattern matches a fully-active
        // measure and the two can share a row.
        let input = concat!(
            "[metadata]\n",
            "title = \"t\"\n",
            "author = \"a\"\n",
            "max columns = 60\n",
            "\n",
            "[parts]\n",
            "Soprano = notes lyrics\n",
            "Alto = notes lyrics\n",
            "\n",
            "[score]\n",
            "(time=4/4 key=C4 bpm=120)\n",
            "1 2 3 4\n",
            "do re mi fa\n",
            "5 6 7 1\n",
            "la la la la\n",
            "\n",
            "1 2 3 4\n",
            "do re mi fa\n",
            "\"\n",
            "ah ah ah ah\n",
        );
        let score = compile(input, "test.jianpu").unwrap();
        assert!(
            matches!(score.measures[1].parts[1], PartRow::Timed(_)),
            "part with explicit lyrics over ditto notes must stay Timed"
        );
        let pages = layout::layout(&score, 595.0, 842.0);
        let total_row_groups: usize = pages.iter().map(|p| p.row_groups.len()).sum();
        assert_eq!(
            total_row_groups, 1,
            "partially-ditto part is active, so both measures share one row"
        );
    }

    #[test]
    fn ditto_row_group_is_shorter_than_fully_active_row_group() {
        // A row where Alto is ditto'd should be shorter than one where both are active.
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
            "\n",
            "1 2 3 4\n",
            "\"\n",
        );
        let score = compile(input, "test.jianpu").unwrap();
        let pages = layout::layout(&score, 595.0, 842.0);
        let heights: Vec<u32> = pages
            .iter()
            .flat_map(|p| p.row_groups.iter())
            .map(|rg| rg.height_in_rows)
            .collect();
        assert_eq!(heights.len(), 2);
        assert!(
            heights[0] > heights[1],
            "row with both parts active (height={}) should be taller than row with Alto ditto'd (height={})",
            heights[0],
            heights[1]
        );
    }

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
        assert_eq!(
            split_pdf_filename("song", "Alto 1 & Tenor"),
            "song - Alto 1 & Tenor.pdf"
        );
        assert_eq!(
            split_pdf_filename("song", "bad/name"),
            "song - bad-name.pdf"
        );
    }

    #[test]
    fn apply_lyrics_filter_downgrades_kind_to_notes() {
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
            "do re mi fa\n",
            "5 6 7 1\n",
            "alt alt alt alt\n",
        );
        let mut score = compile(input, "test.jianpu").unwrap();
        apply_lyrics_filter(&mut score, Some(&["Soprano".into()]));
        let part_slice = score.measures[0].parts[0].slice();
        assert_eq!(
            part_slice.kind,
            PartKind::Notes,
            "apply_lyrics_filter should downgrade kind to Notes when lyrics are hidden"
        );
        let alto_slice = score.measures[0].parts[1].slice();
        assert_eq!(
            alto_slice.kind,
            PartKind::NotesWithLyrics,
            "apply_lyrics_filter should leave untouched parts as NotesWithLyrics"
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
            let entries =
                write_split_pdfs_from_source(multi_track_input(), "test.jianpu", "test_split", &[])
                    .unwrap();
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0].track_name, "S1");
            assert_eq!(entries[0].filename, "test_split - Soprano 1.pdf");
            assert_eq!(entries[1].track_name, "S2");
            assert_eq!(entries[1].filename, "test_split - Soprano 2.pdf");
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
            let entries = write_split_pdfs_from_source(input, "test.jianpu", "song", &[]).unwrap();
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
            let entries =
                write_split_pdfs_from_source(multi_track_input(), "test.jianpu", "test_split", &[])
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
                    "test_split - Soprano 1.pdf".to_string(),
                    "test_split - Soprano 2.pdf".to_string()
                ]
            );

            let mut first = archive.by_name("test_split - Soprano 1.pdf").unwrap();
            let mut buf = Vec::new();
            first.read_to_end(&mut buf).unwrap();
            assert_eq!(&buf[0..4], b"%PDF");
        }
    }
}
