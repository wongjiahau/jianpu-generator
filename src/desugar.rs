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
        .map(|group| desugar_group(group, parts))
        .collect()
}

fn desugar_group(
    group: Vec<(String, usize)>,
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

    let directive_lines = group[..directive_count].to_vec();
    let data_lines = &group[directive_count..];

    let mut resolved: Vec<(String, usize)> = Vec::with_capacity(data_lines.len());

    for (i, (line, offset)) in data_lines.iter().enumerate() {
        if line == "\"" {
            // Guard: if i >= parts.len() the interleaved parser will emit a better error.
            if i >= parts.len() {
                resolved.push((line.clone(), *offset));
                continue;
            }
            let col_type = column_type(&parts[i]);
            let source = (0..resolved.len())
                .rev()
                .find(|&j| column_type(&parts[j]) == col_type)
                .map(|j| resolved[j].0.clone());

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
                            col_type_name(&parts[i])
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
}
