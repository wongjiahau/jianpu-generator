pub(super) use super::*;
pub(super) use crate::grouper;
pub(super) use crate::parser;

mod bars;
mod directive;
mod notes;
mod section;
mod slur_tie;

pub(super) fn syllables_to_line(syllables: &[crate::ast::parsed::Syllable]) -> String {
    syllables
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Build a single-part score with lyrics from bar-separated notes (use `|` to separate bars).
/// `lyrics_str` syllables are allocated per bar to match each bar's lyric-slot count.
pub(super) fn make_score(score_str: &str, lyrics_str: &str) -> Score {
    use crate::parser::score::token_parser;
    use crate::utils::{count_lyric_slots_in_events, LyricTieState};

    let bars: Vec<&str> = score_str
        .split('|')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let all_syllables = crate::utils::tokenize_lyrics(lyrics_str);
    let mut syllable_idx = 0usize;
    let mut tie_state = LyricTieState::default();
    let mut group_state = token_parser::GroupStack::default();
    let mut score_content = String::new();
    score_content.push_str("(time=4/4 key=C4 bpm=120)\n");
    for bar in bars {
        score_content.push_str(bar);
        score_content.push('\n');
        let events =
            token_parser::parse_notes_line(bar, 0, &mut group_state).expect("test score tokens");
        let slots = count_lyric_slots_in_events(&events, &mut tie_state) as usize;
        if slots == 0 {
            score_content.push_str("_\n");
        } else {
            let end = (syllable_idx + slots).min(all_syllables.len());
            let bar_syllables = &all_syllables[syllable_idx..end];
            assert_eq!(
                bar_syllables.len(),
                slots,
                "make_score: not enough lyrics for bar {bar:?} (need {slots})"
            );
            syllable_idx = end;
            score_content.push_str(&syllables_to_line(bar_syllables));
            score_content.push('\n');
        }
        score_content.push('\n'); // blank line separating bar groups
    }
    let input = format!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n[score]\n{score_content}"
    );
    let doc = parser::parse(&input, "test.jianpu").unwrap();
    grouper::group(doc).unwrap()
}

/// Build a score from a pre-formatted score_content string (new interleaved format).
/// score_section must be the full content after `[score]\n` in new interleaved syntax.
pub(super) fn make_score_raw(score_section: &str, lyrics_str: &str) -> Score {
    // score_section is already in new interleaved format passed by callers.
    // lyrics_str is ignored here as it's embedded in score_section.
    let _ = lyrics_str; // lyrics are inline in score_section
    let input = format!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes lyrics\n\n[score]\n{score_section}"
    );
    let doc = parser::parse(&input, "test.jianpu").unwrap();
    grouper::group(doc).unwrap()
}

pub(super) fn collect_time_sig_labels(pages: &[Page]) -> Vec<&GridElement> {
    pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(e.content, GridContent::TimeSignatureLabel { .. }))
        .collect()
}

pub(super) fn collect_bpm_labels(pages: &[Page]) -> Vec<&GridElement> {
    pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter(|e| matches!(e.content, GridContent::BpmLabel { .. }))
        .collect()
}

pub(super) fn collect_curves(pages: &[Page]) -> Vec<(u32, u32)> {
    pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter_map(|e| match &e.content {
            GridContent::TieOrSlurCurve {
                from_column,
                to_column,
            } => Some((*from_column, *to_column)),
            _ => None,
        })
        .collect()
}

pub(super) fn collect_lyric_positions(pages: &[Page]) -> Vec<(u32, String)> {
    pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter_map(|e| match &e.content {
            GridContent::Lyric { text, .. } => Some((e.position.column, text.clone())),
            _ => None,
        })
        .collect()
}

pub(super) fn collect_underline_levels(pages: &[Page]) -> Vec<Vec<UnderlineSpan>> {
    pages
        .iter()
        .flat_map(|p| p.row_groups.iter())
        .flat_map(|rg| rg.elements.iter())
        .filter_map(|e| match &e.content {
            GridContent::DurationUnderlines { levels } => Some(levels.clone()),
            _ => None,
        })
        .collect()
}

pub(super) fn make_two_part_score(s_notes: &str, a_notes: &str) -> Score {
    let input = format!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nSoprano = notes\nAlto = notes\n\n[score]\n(time=4/4 key=C4 bpm=120)\n{s_notes}\n{a_notes}\n"
    );
    let doc = parser::parse(&input, "test.jianpu").unwrap();
    grouper::group(doc).unwrap()
}

pub(super) fn parse_and_layout(input: &str) -> Vec<Page> {
    let doc = parser::parse(input, "test.jianpu").unwrap();
    let score = grouper::group(doc).unwrap();
    layout(&score, A4_WIDTH, A4_HEIGHT)
}

pub(super) const A4_WIDTH: f32 = 595.0; // points
pub(super) const A4_HEIGHT: f32 = 842.0; // points
