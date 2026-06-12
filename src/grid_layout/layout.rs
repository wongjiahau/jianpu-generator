use crate::compiler::types::{Decoration, ElementContent, MeasureBlock, MeasureRow, RowId};
use crate::grid_layout::types::Header;
use crate::grid_layout::types::{GridContent, GridElement, GridPage, GridRow, HAlign, VAlign};
use crate::render_config::RenderConfig;

// ── Row classification ────────────────────────────────────────────────────────

pub(crate) fn is_lyric_row(row: &MeasureRow) -> bool {
    let has_lyric = row
        .elements
        .iter()
        .any(|e| matches!(e.content, ElementContent::Lyric(_)));
    let has_note = row.elements.iter().any(|e| {
        matches!(
            e.content,
            ElementContent::NoteHead { .. } | ElementContent::Rest { .. }
        )
    });
    has_lyric && !has_note
}

fn has_lyrics(row: &MeasureRow) -> bool {
    row.elements
        .iter()
        .any(|e| matches!(e.content, ElementContent::Lyric(_)))
}

pub(crate) fn is_chord_only_row(row: &MeasureRow) -> bool {
    if is_lyric_row(row) {
        return false;
    }
    let has_note = row.elements.iter().any(|e| {
        matches!(
            e.content,
            ElementContent::NoteHead { .. } | ElementContent::Rest { .. }
        )
    });
    !has_note
        && row
            .elements
            .iter()
            .any(|e| matches!(e.content, ElementContent::ChordSymbol(_)))
}

// ── Sub-row heights ───────────────────────────────────────────────────────────

/// Returns the 6 sub-row heights for a Note/Chord part, in order:
/// [arc, above_dot, note_head, below_dot, half_ul, quarter_ul]
pub(crate) fn note_part_sub_row_heights(base: f32) -> [f32; 6] {
    [
        base * 0.30, // tie/slur arc
        base * 0.25, // above-octave dots
        base,        // note head (main)
        base * 0.25, // below-octave dots
        base * 0.15, // half-beat underline
        base * 0.15, // quarter-beat underline
    ]
}

/// Returns the 4 sub-row heights for a Chord-symbol-only part, in order:
/// [arc, chord_main, half_ul, quarter_ul]
pub(crate) fn chord_part_sub_row_heights(base: f32) -> [f32; 4] {
    [
        base * 0.30, // tie/slur arc
        base * 0.75, // chord symbol (main)
        base * 0.15, // half-beat underline
        base * 0.15, // quarter-beat underline
    ]
}

pub(crate) fn lyric_row_height(base: f32) -> f32 {
    base * 0.50
}

pub(crate) fn decoration_row_height(base: f32) -> f32 {
    base * 0.50
}

pub(crate) fn separator_row_height() -> f32 {
    4.0
}

pub(crate) fn header_title_row_height(base: f32) -> f32 {
    base * 0.80
}

pub(crate) fn header_subtitle_author_row_height(base: f32) -> f32 {
    base * 0.50
}

pub(crate) fn footer_row_height(base: f32) -> f32 {
    base * 0.40
}

// ── Column width helper ───────────────────────────────────────────────────────

/// Number of columns in a MeasureBlock (BarLine column + 1).
pub(crate) fn block_column_width(block: &MeasureBlock) -> u32 {
    block
        .rows
        .first()
        .and_then(|row| {
            row.elements
                .iter()
                .find(|e| e.content == ElementContent::BarLine)
        })
        .map(|e| e.column + 1)
        .unwrap_or(1)
}

/// Total height in points for all musical sub-rows in a system
/// (sum over all non-lyric part rows).
pub(crate) fn system_musical_height_pt(block: &MeasureBlock, base: f32) -> f32 {
    block
        .rows
        .iter()
        .filter(|r| !is_lyric_row(r))
        .map(|r| {
            if is_chord_only_row(r) {
                chord_part_sub_row_heights(base).iter().sum::<f32>()
            } else {
                note_part_sub_row_heights(base).iter().sum::<f32>()
            }
        })
        .sum()
}

/// Total height in points for lyric rows in a system.
pub(crate) fn system_lyric_height_pt(block: &MeasureBlock, base: f32) -> f32 {
    block.rows.iter().filter(|r| has_lyrics(r)).count() as f32 * lyric_row_height(base)
}

// ── System packing ───────────────────────────────────────────────────────────

fn row_ids(block: &MeasureBlock) -> Vec<&RowId> {
    block.rows.iter().map(|r| &r.id).collect()
}

pub(crate) const LABEL_COLS: u32 = 4;

/// Break `blocks` into systems. Each system is a `Vec<MeasureBlock>`.
pub(crate) fn pack_into_systems(
    blocks: &[MeasureBlock],
    config: &RenderConfig,
) -> Vec<Vec<MeasureBlock>> {
    let mut systems: Vec<Vec<MeasureBlock>> = Vec::new();
    let mut current: Vec<MeasureBlock> = Vec::new();
    let mut current_cols: u32 = 0;

    for block in blocks {
        let col_w = block_column_width(block);
        let needs_new = if let Some(first) = current.first() {
            current_cols + col_w > config.max_columns || row_ids(block) != row_ids(first)
        } else {
            false
        };

        if needs_new && !current.is_empty() {
            systems.push(std::mem::take(&mut current));
            current_cols = 0;
        }

        current_cols += col_w;
        current.push(block.clone());
    }

    if !current.is_empty() {
        systems.push(current);
    }

    systems
}

fn compute_bar_height(first: &MeasureBlock, base: f32) -> f32 {
    first
        .rows
        .iter()
        .filter(|r| !is_lyric_row(r))
        .map(|r| {
            if is_chord_only_row(r) {
                chord_part_sub_row_heights(base).iter().sum::<f32>()
            } else {
                note_part_sub_row_heights(base).iter().sum::<f32>()
            }
        })
        .sum()
}

fn expand_lyric_part(
    system: &[MeasureBlock],
    part_idx: usize,
    base: f32,
    column_count: u32,
) -> GridRow {
    let mut row = GridRow {
        height_pt: lyric_row_height(base),
        column_count,
        elements: vec![],
    };
    let mut measure_col_offset: u32 = 0;
    for block in system {
        let col_w = block_column_width(block);
        if let Some(part_row) = block.rows.get(part_idx) {
            for el in &part_row.elements {
                if let ElementContent::Lyric(text) = &el.content {
                    row.elements.push(GridElement {
                        column: LABEL_COLS + measure_col_offset + el.column,
                        column_span: 1,
                        halign: HAlign::Center,
                        valign: VAlign::Center,
                        content: GridContent::LyricSyllable(text.clone()),
                    });
                }
            }
        }
        measure_col_offset += col_w;
    }
    row
}

#[allow(clippy::indexing_slicing)]
fn expand_note_part(
    system: &[MeasureBlock],
    part_template: &MeasureRow,
    part_idx: usize,
    base: f32,
    column_count: u32,
    bar_height: f32,
) -> Vec<GridRow> {
    let (sub_heights, sub_count): (Vec<f32>, usize) = if is_chord_only_row(part_template) {
        (chord_part_sub_row_heights(base).to_vec(), 4)
    } else {
        (note_part_sub_row_heights(base).to_vec(), 6)
    };
    let mut sub_rows: Vec<GridRow> = sub_heights
        .iter()
        .map(|&h| GridRow {
            height_pt: h,
            column_count,
            elements: vec![],
        })
        .collect();
    let head_sub = if is_chord_only_row(part_template) {
        1
    } else {
        2
    };
    if !part_template.label.is_empty() {
        sub_rows[head_sub].elements.push(GridElement {
            column: 0,
            column_span: LABEL_COLS,
            halign: HAlign::Center,
            valign: VAlign::Center,
            content: GridContent::RowLabel(part_template.label.clone()),
        });
    }
    if part_idx == 0 {
        sub_rows[0].elements.push(GridElement {
            column: LABEL_COLS,
            column_span: 1,
            halign: HAlign::Start,
            valign: VAlign::Top,
            content: GridContent::BarLine {
                height_pt: bar_height,
            },
        });
    }
    let mut measure_col_offset: u32 = 0;
    for block in system {
        let col_w = block_column_width(block);
        if let Some(part_row) = block.rows.get(part_idx) {
            crate::grid_layout::expand::expand_measure_elements(
                part_row,
                measure_col_offset,
                head_sub,
                sub_count,
                bar_height,
                part_idx,
                &mut sub_rows,
            );
        }
        measure_col_offset += col_w;
    }
    sub_rows
}

/// Convert a system's measures into flat GridRows.
/// Does not include decoration, separator, header, or footer rows.
pub(crate) fn expand_system_to_rows(system: &[MeasureBlock], base: f32) -> Vec<GridRow> {
    let Some(first) = system.first() else {
        return vec![];
    };
    let total_musical_cols: u32 = system.iter().map(block_column_width).sum();
    let column_count = LABEL_COLS + total_musical_cols;
    let bar_height = compute_bar_height(first, base);
    let mut all_rows: Vec<GridRow> = Vec::new();
    for (part_idx, part_template) in first.rows.iter().enumerate() {
        if is_lyric_row(part_template) {
            all_rows.push(expand_lyric_part(system, part_idx, base, column_count));
        } else {
            all_rows.extend(expand_note_part(
                system,
                part_template,
                part_idx,
                base,
                column_count,
                bar_height,
            ));
            if has_lyrics(part_template) {
                all_rows.push(expand_lyric_part(system, part_idx, base, column_count));
            }
        }
    }
    all_rows
}

fn has_any_decoration(block: &MeasureBlock) -> bool {
    !block.decorations.is_empty()
}

fn make_decoration_row(system: &[MeasureBlock], base: f32) -> GridRow {
    let Some(first) = system.first() else {
        return GridRow {
            height_pt: decoration_row_height(base),
            column_count: 1,
            elements: vec![],
        };
    };
    let total_musical_cols: u32 = system.iter().map(block_column_width).sum();
    let column_count = LABEL_COLS + total_musical_cols;
    let mut elements: Vec<GridElement> = Vec::new();

    for dec in &first.decorations {
        match dec {
            Decoration::Bpm(bpm) => elements.push(GridElement {
                column: LABEL_COLS,
                column_span: column_count - LABEL_COLS,
                halign: HAlign::Start,
                valign: VAlign::Center,
                content: GridContent::Bpm(*bpm),
            }),
            Decoration::TimeSignature {
                numerator,
                denominator,
            } => elements.push(GridElement {
                column: LABEL_COLS,
                column_span: column_count - LABEL_COLS,
                halign: HAlign::Start,
                valign: VAlign::Center,
                content: GridContent::TimeSignature {
                    numerator: *numerator,
                    denominator: *denominator,
                },
            }),
            Decoration::SectionLabel(s) => elements.push(GridElement {
                column: LABEL_COLS,
                column_span: column_count - LABEL_COLS,
                halign: HAlign::Start,
                valign: VAlign::Center,
                content: GridContent::SectionLabel(s.clone()),
            }),
            Decoration::BarNumber(n) => elements.push(GridElement {
                column: LABEL_COLS,
                column_span: column_count - LABEL_COLS,
                halign: HAlign::Start,
                valign: VAlign::Bottom,
                content: GridContent::BarNumber(*n),
            }),
        }
    }

    GridRow {
        height_pt: decoration_row_height(base),
        column_count,
        elements,
    }
}

fn make_separator_row() -> GridRow {
    GridRow {
        height_pt: separator_row_height(),
        column_count: 1,
        elements: vec![GridElement {
            column: 0,
            column_span: 1,
            halign: HAlign::Start,
            valign: VAlign::Center,
            content: GridContent::HorizontalLine,
        }],
    }
}

fn make_header_rows(header: &Header, base: f32) -> Vec<GridRow> {
    let title_row = GridRow {
        height_pt: header_title_row_height(base),
        column_count: 1,
        elements: vec![GridElement {
            column: 0,
            column_span: 1,
            halign: HAlign::Center,
            valign: VAlign::Center,
            content: GridContent::Text {
                content: header.title.clone(),
                font_size: base * 1.5,
                bold: false,
                italic: false,
            },
        }],
    };

    let mut subtitle_author_elements: Vec<GridElement> = Vec::new();
    if let Some(subtitle) = &header.subtitle {
        subtitle_author_elements.push(GridElement {
            column: 0,
            column_span: 1,
            halign: HAlign::Center,
            valign: VAlign::Center,
            content: GridContent::Text {
                content: subtitle.clone(),
                font_size: base * 0.8,
                bold: false,
                italic: true,
            },
        });
    }
    subtitle_author_elements.push(GridElement {
        column: 0,
        column_span: 1,
        halign: HAlign::End,
        valign: VAlign::Center,
        content: GridContent::Text {
            content: header.author.clone(),
            font_size: base * 0.6,
            bold: false,
            italic: false,
        },
    });
    let subtitle_author_row = GridRow {
        height_pt: header_subtitle_author_row_height(base),
        column_count: 1,
        elements: subtitle_author_elements,
    };

    vec![title_row, subtitle_author_row]
}

fn make_footer_row(page_num: u32, total_pages: u32, base: f32) -> GridRow {
    GridRow {
        height_pt: footer_row_height(base),
        column_count: 1,
        elements: vec![GridElement {
            column: 0,
            column_span: 1,
            halign: HAlign::Center,
            valign: VAlign::Center,
            content: GridContent::Text {
                content: format!("{page_num} / {total_pages}"),
                font_size: base * 0.6,
                bold: false,
                italic: false,
            },
        }],
    }
}

fn system_total_height(system: &[MeasureBlock], base: f32) -> f32 {
    let Some(first) = system.first() else {
        return 0.0;
    };
    let musical = system_musical_height_pt(first, base);
    let lyric = system_lyric_height_pt(first, base);
    let deco = if has_any_decoration(first) {
        decoration_row_height(base)
    } else {
        0.0
    };
    musical + lyric + deco
}

fn build_page_rows(systems: &[Vec<MeasureBlock>], header: &Header, base: f32) -> Vec<GridRow> {
    let mut rows: Vec<GridRow> = make_header_rows(header, base);
    for (sys_idx, system) in systems.iter().enumerate() {
        if sys_idx > 0 {
            rows.push(make_separator_row());
        }
        let Some(first) = system.first() else {
            continue;
        };
        if has_any_decoration(first) {
            rows.push(make_decoration_row(system, base));
        }
        rows.extend(expand_system_to_rows(system, base));
    }
    rows
}

/// Public entry point: convert compiler blocks to GridPages.
pub fn layout(
    blocks: &[MeasureBlock],
    config: &RenderConfig,
    header: &Header,
    page_width_pt: f32,
    page_height_pt: f32,
) -> Vec<GridPage> {
    let base = config.row_height as f32;
    let systems = pack_into_systems(blocks, config);

    let header_h: f32 = make_header_rows(header, base)
        .iter()
        .map(|r| r.height_pt)
        .sum();
    let footer_h = footer_row_height(base);
    let usable_h = page_height_pt - 2.0 * super::PAGE_MARGIN - header_h - footer_h;

    let mut page_systems: Vec<Vec<Vec<MeasureBlock>>> = Vec::new();
    let mut current_page: Vec<Vec<MeasureBlock>> = Vec::new();
    let mut used_h: f32 = 0.0;

    for system in systems {
        let sys_h = system_total_height(&system, base);
        let gap = if current_page.is_empty() {
            0.0
        } else {
            separator_row_height()
        };
        if !current_page.is_empty() && used_h + gap + sys_h > usable_h {
            page_systems.push(std::mem::take(&mut current_page));
            used_h = 0.0;
        }
        used_h += gap + sys_h;
        current_page.push(system);
    }
    page_systems.push(current_page);

    let total_pages = page_systems.len() as u32;
    page_systems
        .into_iter()
        .enumerate()
        .map(|(page_idx, systems)| {
            let mut rows = build_page_rows(&systems, header, base);
            rows.push(make_footer_row(page_idx as u32 + 1, total_pages, base));
            GridPage {
                width_pt: page_width_pt,
                height_pt: page_height_pt,
                rows,
            }
        })
        .collect()
}
