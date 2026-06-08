use crate::ast::parsed::PartColumn;
use crate::error::{JianPuError, Span};

/// Resolves `"` ditto lines within each measure group.
///
/// A `"` on a data line means "same content as the closest preceding line of
/// the same column type in this group." The directive line (starts with `(`)
/// is never a ditto source or target.
///
/// `parts` maps each data-line position to its column type.
pub fn desugar_groups(
    groups: Vec<Vec<(String, usize)>>,
    parts: &[PartColumn],
) -> Result<Vec<Vec<(String, usize)>>, JianPuError> {
    groups
        .into_iter()
        .map(|group| {
            let padded = pad_implicit_ditto_group(&group, parts)?;
            desugar_group(&padded, parts)
        })
        .collect()
}

/// Pads omitted trailing data lines with implicit `"` ditto markers.
fn pad_implicit_ditto_group(
    group: &[(String, usize)],
    parts: &[PartColumn],
) -> Result<Vec<(String, usize)>, JianPuError> {
    let directive_count = if group
        .first()
        .map(|(l, _)| l.starts_with('('))
        .unwrap_or(false)
    {
        1
    } else {
        0
    };

    let directive_lines = group.get(..directive_count).unwrap_or(&[]);
    let data_lines = group.get(directive_count..).unwrap_or(&[]);

    let span = data_lines
        .last()
        .or(group.last())
        .map(|(_, off)| Span::new(*off, *off + 1))
        .unwrap_or(Span::new(0, 1));

    if data_lines.is_empty() {
        return Err(JianPuError::new(
            span,
            "expected at least one data line in measure group".to_string(),
        ));
    }

    if data_lines.len() > parts.len() {
        return Err(JianPuError::new(
            span,
            format!(
                "expected at most {} lines (one per parts column), got {}",
                parts.len(),
                data_lines.len()
            ),
        ));
    }

    let pad_offset = data_lines.last().map(|(_, off)| *off).unwrap_or(0);
    let mut result_data: Vec<(String, usize)> = data_lines.to_vec();

    for i in data_lines.len()..parts.len() {
        let col = parts.get(i).ok_or_else(|| {
            JianPuError::new(
                Span::new(0, 0),
                "internal invariant: part column missing for implicit ditto padding",
            )
        })?;
        let col_type = column_type(col);
        let has_precedent = (0..result_data.len()).any(|j| {
            parts
                .get(j)
                .map(|p| column_type(p) == col_type)
                .unwrap_or(false)
        });

        if has_precedent {
            result_data.push(("\"".to_string(), pad_offset));
        } else {
            let name = part_display_name(col);
            let hint = if matches!(col, PartColumn::Lyrics { .. }) {
                "write content, '\"' ditto, or '_' for no lyrics"
            } else {
                "write content or '\"' ditto"
            };
            return Err(JianPuError::new(
                Span::new(pad_offset, pad_offset + 1),
                format!("expected {} line for '{name}'; {hint}", col_type_name(col)),
            ));
        }
    }

    let mut result = directive_lines.to_vec();
    result.extend(result_data);
    Ok(result)
}

fn desugar_group(
    group: &[(String, usize)],
    parts: &[PartColumn],
) -> Result<Vec<(String, usize)>, JianPuError> {
    // Directive line (starts with `(`) is never a ditto target — pass it through.
    let directive_count = if group
        .first()
        .map(|(l, _)| l.starts_with('('))
        .unwrap_or(false)
    {
        1
    } else {
        0
    };

    let directive_lines = group.get(..directive_count).unwrap_or(&[]).to_vec();
    let data_lines = group.get(directive_count..).unwrap_or(&[]);

    let mut resolved: Vec<(String, usize)> = Vec::with_capacity(data_lines.len());

    for (i, (line, offset)) in data_lines.iter().enumerate() {
        if line == "\"" {
            // Guard: if i >= parts.len() the interleaved parser will emit a better error.
            if i >= parts.len() {
                resolved.push((line.clone(), *offset));
                continue;
            }
            let col_type = parts.get(i).map(column_type).ok_or_else(|| {
                JianPuError::new(
                    Span::new(0, 0),
                    "internal invariant: part column missing for ditto line",
                )
            })?;
            let source = (0..resolved.len())
                .rev()
                .find(|&j| {
                    parts
                        .get(j)
                        .map(|p| column_type(p) == col_type)
                        .unwrap_or(false)
                })
                .and_then(|j| resolved.get(j).map(|r| r.0.clone()));

            match source {
                Some(src_content) => {
                    // Keep the ditto's own byte offset so error spans point here.
                    resolved.push((src_content, *offset));
                }
                None => {
                    return Err(JianPuError::new(
                        Span::new(*offset, *offset + 1),
                        format!(
                            "ditto '\"' has no preceding {} line in this measure group",
                            parts.get(i).map(col_type_name).unwrap_or("unknown")
                        ),
                    ));
                }
            }
        } else {
            resolved.push((line.clone(), *offset));
        }
    }

    let mut result = directive_lines;
    result.extend(resolved);
    Ok(result)
}

#[derive(PartialEq)]
enum ColType {
    Notes,
    Lyrics,
    Chord,
}

fn column_type(col: &PartColumn) -> ColType {
    match col {
        PartColumn::Notes { .. } => ColType::Notes,
        PartColumn::Lyrics { .. } => ColType::Lyrics,
        PartColumn::Chord { .. } => ColType::Chord,
    }
}

fn col_type_name(col: &PartColumn) -> &'static str {
    match col {
        PartColumn::Notes { .. } => "notes",
        PartColumn::Lyrics { .. } => "lyrics",
        PartColumn::Chord { .. } => "chord",
    }
}

fn part_display_name(col: &PartColumn) -> &str {
    match col {
        PartColumn::Notes { name } | PartColumn::Lyrics { name } | PartColumn::Chord { name } => {
            name
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notes(name: &str) -> PartColumn {
        PartColumn::Notes {
            name: name.to_string(),
        }
    }
    fn lyrics(name: &str) -> PartColumn {
        PartColumn::Lyrics {
            name: name.to_string(),
        }
    }
    fn chord(name: &str) -> PartColumn {
        PartColumn::Chord {
            name: name.to_string(),
        }
    }

    fn group(lines: &[&str]) -> Vec<(String, usize)> {
        lines
            .iter()
            .enumerate()
            .map(|(i, l)| (l.to_string(), i * 10))
            .collect()
    }

    #[test]
    fn notes_ditto_copies_preceding_notes_line() {
        let groups = vec![group(&["1 2 3 4", "\""])];
        let parts = vec![notes("A"), notes("B")];
        let result = desugar_groups(groups, &parts).unwrap();
        assert_eq!(result[0][1].0, "1 2 3 4");
    }

    #[test]
    fn lyrics_ditto_copies_preceding_lyrics_line() {
        let groups = vec![group(&["1 2 3 4", "hello world", "5 6 7 1", "\""])];
        let parts = vec![notes("A"), lyrics("A"), notes("B"), lyrics("B")];
        let result = desugar_groups(groups, &parts).unwrap();
        assert_eq!(result[0][3].0, "hello world");
    }

    #[test]
    fn chord_ditto_copies_preceding_chord_line() {
        let groups = vec![group(&["1 - - -", "1 2 3 4", "\"", "5 6 7 1"])];
        let parts = vec![chord("main"), notes("A"), chord("main2"), notes("B")];
        let result = desugar_groups(groups, &parts).unwrap();
        assert_eq!(result[0][2].0, "1 - - -");
    }

    #[test]
    fn notes_ditto_does_not_copy_lyrics_line() {
        // A `"` on a notes line must NOT match a preceding lyrics line.
        // Only the notes line before it counts.
        let groups = vec![group(&["1 2 3 4", "hello world", "\""])];
        let parts = vec![notes("A"), lyrics("A"), notes("B")];
        let result = desugar_groups(groups, &parts).unwrap();
        assert_eq!(result[0][2].0, "1 2 3 4");
    }

    #[test]
    fn chained_ditto_resolves_transitively() {
        // Third line dittos the second, which dittos the first.
        let groups = vec![group(&["1 2 3 4", "\"", "\""])];
        let parts = vec![notes("A"), notes("B"), notes("C")];
        let result = desugar_groups(groups, &parts).unwrap();
        assert_eq!(result[0][1].0, "1 2 3 4");
        assert_eq!(result[0][2].0, "1 2 3 4");
    }

    #[test]
    fn ditto_with_no_preceding_line_is_an_error() {
        let groups = vec![group(&["\""])];
        let parts = vec![notes("A")];
        let err = desugar_groups(groups, &parts).unwrap_err();
        assert!(
            err.message.contains("no preceding notes line"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn ditto_with_no_preceding_line_of_same_type_is_an_error() {
        // lyrics `"` with only a notes line before it — no preceding lyrics.
        let groups = vec![group(&["1 2 3 4", "\""])];
        let parts = vec![notes("A"), lyrics("A")];
        let err = desugar_groups(groups, &parts).unwrap_err();
        assert!(
            err.message.contains("no preceding lyrics line"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn directive_line_is_not_a_ditto_target() {
        // Group starts with a directive — ditto on the first data line
        // should still error (no preceding notes line in data lines).
        let groups = vec![group(&["(time=4/4)", "\""])];
        let parts = vec![notes("A")];
        let err = desugar_groups(groups, &parts).unwrap_err();
        assert!(
            err.message.contains("no preceding notes line"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn directive_line_is_not_a_ditto_source() {
        // The ditto should copy from the notes line, not the directive line.
        let groups = vec![group(&["(time=4/4)", "1 2 3 4", "\""])];
        let parts = vec![notes("A"), notes("B")];
        let result = desugar_groups(groups, &parts).unwrap();
        // directive passes through unchanged
        assert_eq!(result[0][0].0, "(time=4/4)");
        // ditto resolves to the notes line
        assert_eq!(result[0][2].0, "1 2 3 4");
    }

    #[test]
    fn non_ditto_lines_are_passed_through_unchanged() {
        let groups = vec![group(&["1 2 3 4", "hello"])];
        let parts = vec![notes("A"), lyrics("A")];
        let result = desugar_groups(groups, &parts).unwrap();
        assert_eq!(result[0][0].0, "1 2 3 4");
        assert_eq!(result[0][1].0, "hello");
    }

    #[test]
    fn multiple_groups_are_desugared_independently() {
        // Ditto in group 2 must NOT copy from group 1.
        let groups = vec![group(&["1 2 3 4"]), group(&["\""])];
        let parts = vec![notes("A")];
        let err = desugar_groups(groups, &parts).unwrap_err();
        assert!(
            err.message.contains("no preceding notes line"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn omitted_trailing_notes_line_is_padded_as_implicit_ditto() {
        let groups = vec![group(&["1 2 3 4"])];
        let parts = vec![notes("A"), notes("B")];
        let result = desugar_groups(groups, &parts).unwrap();
        assert_eq!(result[0][0].0, "1 2 3 4");
        assert_eq!(result[0][1].0, "1 2 3 4");
    }

    #[test]
    fn omitted_trailing_lines_pad_as_ditto_when_precedent_exists() {
        let groups = vec![group(&["1 - - -", "1 2 3 4", "hello"])];
        let parts = vec![
            chord("main"),
            notes("A"),
            lyrics("A"),
            notes("B"),
            lyrics("B"),
        ];
        let result = desugar_groups(groups, &parts).unwrap();
        assert_eq!(result[0][3].0, "1 2 3 4");
        assert_eq!(result[0][4].0, "hello");
    }

    #[test]
    fn omitted_trailing_lyrics_without_precedent_is_an_error() {
        let groups = vec![group(&["1 2 3 4"])];
        let parts = vec![notes("A"), lyrics("A")];
        let err = desugar_groups(groups, &parts).unwrap_err();
        assert!(
            err.message.contains("expected lyrics line"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn ditto_can_copy_underscore_no_lyrics_marker() {
        let groups = vec![group(&["1 2 3 4", "_", "\""])];
        let parts = vec![notes("A"), lyrics("A"), lyrics("B")];
        let result = desugar_groups(groups, &parts).unwrap();
        assert_eq!(result[0][2].0, "_");
    }
}
