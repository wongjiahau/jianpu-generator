use crate::compiler::types::MeasureBlock;
use crate::grid_layout::highlight::compute_measure_highlights_for_range;
use crate::grid_layout::types::MeasureRange;
use std::collections::HashMap;

use super::{no_header, simple_block};

/// A `# sequence` chain selection (e.g. range-selecting from "C" to a later
/// repeat of "A" across "A, B, C, A") highlights several disjoint measures
/// at once — `compute_measure_highlights_for_range` must highlight every measure
/// covered by any of the given ranges, and none of the measures in between.
#[test]
fn disjoint_ranges_highlight_only_their_own_measures() {
    let page_systems: Vec<Vec<Vec<MeasureBlock>>> = vec![vec![vec![
        simple_block(4),
        simple_block(4),
        simple_block(4),
    ]]];
    let highlights = compute_measure_highlights_for_range(
        &page_systems,
        &HashMap::new(),
        &[
            MeasureRange { start: 0, end: 0 },
            MeasureRange { start: 2, end: 2 },
        ],
        &no_header(),
        20.0,
        false,
    );
    assert_eq!(
        highlights.len(),
        2,
        "measures 0 and 2 should each produce one highlight, measure 1 none"
    );
    let mut iter = highlights.into_iter();
    let (_, first) = iter.next().expect("measure 0's highlight");
    let (_, second) = iter.next().expect("measure 2's highlight");
    assert_eq!(
        first.column_start, 1.0,
        "measure 0 is the system's first block"
    );
    assert_eq!(
        second.column_start, 11.5,
        "measure 2 is a mid-system block: its leading bar line is centered \
         in its own column, not flush like the system-leading one"
    );
}
