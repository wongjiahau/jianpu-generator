pub mod ast;
pub mod combiner;
pub mod compiler;
pub mod compositor;
pub mod desugar;
pub mod error;
pub mod error_reporter;
pub mod grouper;
pub mod grouping;
pub mod layout;
pub mod parser;
pub mod render_config;
pub mod renderer;
pub mod serializer;
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
    use layout::new_types::Header;
    let config = render_config::RenderConfig::from_metadata(&score.metadata);
    let header = Header {
        title: score.metadata.title.clone(),
        subtitle: score.metadata.subtitle.clone(),
        author: score.metadata.author.clone(),
    };
    let blocks = compiler::compile(score);
    let pages = layout::new_layout::layout_new(&blocks, &config, &header, 595.0, 842.0);
    let abs = compositor::compose(&pages, &config);
    let docs = renderer::new_renderer::render_new(&abs, &config);
    serializer::serialize(&docs)
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
        // If the source part was filtered out, a leading ditto has no row to
        // merge into and its content would silently disappear. Promote the
        // first ditto part to Timed so it renders independently.
        if let Some(first) = measure.parts.first_mut() {
            if let PartRow::Ditto(slice) = first {
                *first = PartRow::Timed(slice.clone());
            }
        }
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
mod tests;
