#![allow(clippy::disallowed_macros)]
use crate::note_highlight_identity::{
    multi_part_source_with_ties_rests_and_chord, note_ids_in_svg,
};
use jianpu_generator::{
    measure_start_times_from_source, note_timings_for_range_from_source, note_timings_from_source,
    render_svgs_from_source, render_svgs_from_source_filtered, MeasureRangeSelection,
};
use std::collections::HashSet;

/// Regression test: filtering playback down to a subset of parts (e.g. the
/// web app's note range-select playback, which mutes every part outside the
/// selection) must not renumber `source_part_index`. `apply_track_filter`
/// physically drops the unselected parts from `score.measures[].parts`
/// before `note_timings_seconds` walks it, so without correction the
/// remaining parts' timings get reindexed from 0 by their new position in
/// the filtered vec — disagreeing with the full, unfiltered render's
/// `data-part-index`, which is what the SVG playback cursor (`usePlaybackCursor`)
/// looks up by. Here `Alto` and `Chords` are written parts 1 and 2 (after
/// `Soprano`, part 0); filtering down to just them must still report
/// `source_part_index` 1 and 2, not 0 and 1.
#[test]
fn note_timings_with_track_filter_keep_original_part_indices() {
    let source = multi_part_source_with_ties_rests_and_chord();
    let filename = "note_highlight_identity_track_filter.jianpu";

    let render_output = render_svgs_from_source(source, filename, &[]).unwrap();
    let svg_ids = note_ids_in_svg(&render_output.svgs);

    let enabled_tracks = ["A".to_string(), "C".to_string()];
    let filtered_timings =
        note_timings_from_source(source, filename, None, Some(&enabled_tracks), &[]).unwrap();

    assert!(
        !filtered_timings.is_empty(),
        "expected at least one note timing for the filtered parts"
    );

    let filtered_ids: HashSet<(usize, usize)> = filtered_timings
        .iter()
        .map(|t| (t.source_part_index, t.note_id))
        .collect();
    assert!(
        filtered_ids.is_subset(&svg_ids),
        "filtered note timings must use the same (source_part_index, note_id) identity as the \
         full-score render's data-note-id: {filtered_ids:?} not a subset of {svg_ids:?}"
    );

    let source_part_indices: HashSet<usize> = filtered_timings
        .iter()
        .map(|t| t.source_part_index)
        .collect();
    assert_eq!(
        source_part_indices,
        HashSet::from([1, 2]),
        "Alto (written part 1) and Chords (written part 2) must keep their original \
         source_part_index after filtering, not be renumbered to 0 and 1 by their position \
         in the filtered parts list"
    );
}

/// One part, four measures: notes, then two consecutive all-rest measures
/// (which `merge_rest_runs` collapses into a single `MultiMeasureRest` glyph
/// since `MIN_REST_RUN_LENGTH` is 2), then notes again.
fn score_with_merged_rest_run() -> &'static str {
    concat!(
        "# metadata\n",
        "title = \"note highlight identity merged rest\"\n",
        "\n",
        "# parts\n",
        "Melody [M] = notes\n",
        "\n",
        "# score\n",
        "time=4/4 key=C4 bpm=120\n",
        "[M] 1 2 3 4\n",
        "\n",
        "[M] 0 0 0 0\n",
        "\n",
        "[M] 0 0 0 0\n",
        "\n",
        "[M] 5 6 7 1\n",
    )
}

#[test]
fn note_ids_match_between_grid_layout_and_midi_timing_with_merged_rest_run() {
    let source = score_with_merged_rest_run();

    let render_output =
        render_svgs_from_source(source, "note_highlight_identity_merged_rest.jianpu", &[]).unwrap();
    let svg_ids = note_ids_in_svg(&render_output.svgs);

    let note_timings = note_timings_from_source(
        source,
        "note_highlight_identity_merged_rest.jianpu",
        None,
        None,
        &[],
    )
    .unwrap();
    let timing_ids: HashSet<(usize, usize)> = note_timings
        .iter()
        .map(|t| (t.source_part_index, t.note_id))
        .collect();

    assert!(
        !svg_ids.is_empty(),
        "expected at least one note group in the rendered SVG"
    );
    assert_eq!(
        svg_ids, timing_ids,
        "grid-layout note highlight targets and MIDI note timings must agree on every \
         (source_part_index, note_id) pair, even when consecutive all-rest measures are \
         merged into a single MultiMeasureRest glyph"
    );

    // 4 notes + 1 merged-rest glyph (standing in for both rest measures) + 4
    // notes = 9 sounding entries, not 12 as if the two rest measures had each
    // kept their own NoteTiming.
    assert_eq!(
        note_timings.len(),
        9,
        "the two consecutive all-rest measures should collapse into one NoteTiming"
    );

    let merged_rest = note_timings
        .iter()
        .find(|t| t.note_id == 4)
        .expect("expected a single NoteTiming for the merged rest glyph's note_id");
    let quarter_note_duration = note_timings[1].start_s - note_timings[0].start_s;
    assert!(
        (merged_rest.end_s - merged_rest.start_s - quarter_note_duration * 8.0).abs() < 1e-6,
        "merged rest should span both underlying measures (8 quarter notes' worth of time), \
         got {} vs {}",
        merged_rest.end_s - merged_rest.start_s,
        quarter_note_duration * 8.0
    );
}

/// Two parts, Melody and Harmony: Harmony alone has notes in the first two
/// measures (Melody implicitly rests there), then Melody plays in the third
/// measure (Harmony implicitly rests there). Hiding Harmony turns Melody's
/// leading two measures into a genuinely all-rest run *only once Harmony is
/// filtered out* — in the full, unfiltered score they aren't all-rest, since
/// Harmony has real notes there. This is the filter-induced merge the
/// full-score compile in `note_timings_seconds` used to miss entirely (see
/// the "playback cursor rest collapse with hidden part" e2e regression).
fn score_with_filter_induced_merged_rest_run() -> &'static str {
    concat!(
        "# metadata\n",
        "title = \"note highlight identity filter induced merged rest\"\n",
        "\n",
        "# parts\n",
        "Melody [M] = notes\n",
        "Harmony [H] = notes\n",
        "\n",
        "# score\n",
        "time=4/4 key=C4 bpm=120\n",
        "[H] 1 2 3 4\n",
        "\n",
        "[H] 5 6 7 1\n",
        "\n",
        "[M] 1 2 3 4\n",
    )
}

/// Regression test: hiding Harmony (the part-visibility toggle, e.g.
/// `render_svgs_from_source_filtered`'s `enabled_tracks`, physically removed
/// before `compile()`) turns Melody's first two measures into an all-rest
/// run that only the *filtered* render's `compile()` call ever sees — the
/// full, unfiltered score still has Harmony's real notes there, so a naive
/// `note_timings_seconds` call over the unfiltered score would never collapse
/// that span into one `MultiMeasureRest` block/`note_id`, disagreeing with
/// what the filtered render actually shows (and permanently desyncing the
/// playback cursor, since `usePlaybackCursor` looks elements up by that
/// `note_id`). Passing the same visibility filter into `note_timings_from_source`
/// must make the two agree exactly.
#[test]
fn note_timings_with_visibility_filter_match_filter_induced_merged_rest() {
    let source = score_with_filter_induced_merged_rest_run();
    let filename = "note_highlight_identity_visibility_filter_merged_rest.jianpu";

    let visible_tracks = ["M".to_string()];
    let render_output =
        render_svgs_from_source_filtered(source, filename, Some(&visible_tracks), &[]).unwrap();
    let svg_ids = note_ids_in_svg(&render_output.svgs);

    let note_timings =
        note_timings_from_source(source, filename, Some(&visible_tracks), None, &[]).unwrap();
    let timing_ids: HashSet<(usize, usize)> = note_timings
        .iter()
        .map(|t| (t.source_part_index, t.note_id))
        .collect();

    assert!(
        !svg_ids.is_empty(),
        "expected at least one note group in the rendered (Harmony-hidden) SVG"
    );
    assert_eq!(
        svg_ids, timing_ids,
        "with Harmony hidden, the filtered render's (source_part_index, note_id) identities \
         must match note_timings_from_source's exactly, including the filter-induced merged \
         rest spanning Melody's first two measures: {svg_ids:?} vs {timing_ids:?}"
    );

    // Melody's two leading rest measures collapse into one merged-rest
    // NoteTiming, plus its 4 real notes in measure 2 = 5 total, not 6 (which
    // would mean the filter-induced merge never happened).
    assert_eq!(
        note_timings.len(),
        5,
        "Melody's two filter-induced all-rest measures should collapse into one NoteTiming, \
         matching the filtered render's single MultiMeasureRest glyph"
    );
}

/// Playing from a non-first measure (e.g. the web app's "play from here")
/// must report note timings relative to the *clip*'s own start, not the
/// whole score's, and must still agree with the full-score render's
/// `data-note-id`s (see `note_timings_seconds_for_range`).
#[test]
fn note_timings_for_range_start_at_zero_and_match_full_score_ids() {
    let source = multi_part_source_with_ties_rests_and_chord();
    let filename = "note_highlight_identity_range.jianpu";

    let render_output = render_svgs_from_source(source, filename, &[]).unwrap();
    let svg_ids = note_ids_in_svg(&render_output.svgs);

    let full_timings = note_timings_from_source(source, filename, None, None, &[]).unwrap();
    let measure_boundaries = measure_start_times_from_source(source, filename, None, &[]).unwrap();

    // Play from the second (last) written measure through the end.
    let start_measure_index = 1;
    let range_timings = note_timings_for_range_from_source(
        source,
        filename,
        &MeasureRangeSelection {
            range: start_measure_index..=start_measure_index,
            extend_to_last_occurrence: false,
            respect_sequence: true,
            sequence_entry_range: None,
        },
        None,
        None,
        &[],
    )
    .unwrap();

    assert!(
        !range_timings.is_empty(),
        "expected at least one note timing for the ranged measure"
    );

    let range_ids: HashSet<(usize, usize)> = range_timings
        .iter()
        .map(|t| (t.source_part_index, t.note_id))
        .collect();
    assert!(
        range_ids.is_subset(&svg_ids),
        "ranged note timings must use the same (source_part_index, note_id) identity as the \
         full-score render's data-note-id: {range_ids:?} not a subset of {svg_ids:?}"
    );

    // The written measure's identity should match exactly the full-score
    // timings occurring at or after that measure's boundary (start_s in the
    // full score, before rebasing).
    let full_score_offset = measure_boundaries[start_measure_index];
    let expected_ids: HashSet<(usize, usize)> = full_timings
        .iter()
        .filter(|t| t.start_s >= full_score_offset - 1e-6)
        .map(|t| (t.source_part_index, t.note_id))
        .collect();
    assert_eq!(
        range_ids, expected_ids,
        "ranged note timings should cover exactly the same notes as the full-score timings \
         occurring at or after the ranged measure's boundary"
    );

    let first_start_s = range_timings
        .iter()
        .map(|t| t.start_s)
        .fold(f64::INFINITY, f64::min);
    assert!(
        first_start_s.abs() < 1e-6,
        "the first note of a range starting mid-score should be offset to ~0 seconds relative \
         to the clip, got {first_start_s}"
    );
}

/// A `# sequence` that reorders/repeats labeled spans (`A, B, B, C`), so a
/// written measure's position in playback order differs from its literal
/// written index — `C` is written measure 2 but plays at position 3.
fn sequence_source_with_reordered_spans() -> &'static str {
    concat!(
        "# metadata\n",
        "title = \"note highlight identity respect_sequence false\"\n",
        "\n",
        "# parts\n",
        "Soprano [S] = notes\n",
        "\n",
        "# sequence\n",
        "A, B, B, C\n",
        "\n",
        "# score\n",
        "time=4/4 key=C4 bpm=120 label=\"A\"\n",
        "[S] 1\n",
        "\n",
        "label=\"B\"\n",
        "[S] 2\n",
        "\n",
        "label=\"C\"\n",
        "[S] 3\n",
    )
}

/// Regression test: "play current measure" (`respect_sequence: false`) on a
/// written measure that a `# sequence` also plays at a *different* position
/// elsewhere (here, `C` is written index 2 but plays at position 3, since `B`
/// occupies position 2) must still report `C`'s own `note_id`, not whichever
/// measure occupies that position in the `# sequence`-expanded timeline.
#[test]
fn note_timings_for_range_ignore_sequence_use_written_index_identity() {
    let source = sequence_source_with_reordered_spans();
    let filename = "note_highlight_identity_ignore_sequence.jianpu";

    let render_output = render_svgs_from_source(source, filename, &[]).unwrap();
    let svg_ids = note_ids_in_svg(&render_output.svgs);

    // Written measure 2 is section "C".
    let range_timings = note_timings_for_range_from_source(
        source,
        filename,
        &MeasureRangeSelection {
            range: 2..=2,
            extend_to_last_occurrence: false,
            respect_sequence: false,
            sequence_entry_range: None,
        },
        None,
        None,
        &[],
    )
    .unwrap();

    assert_eq!(
        range_timings.len(),
        1,
        "expected exactly one note timing for the single-note measure C"
    );
    let timing = &range_timings[0];
    assert!(
        svg_ids.contains(&(timing.source_part_index, timing.note_id)),
        "range timing {timing:?} must use an (source_part_index, note_id) pair that exists in \
         the full-score render's data-note-id"
    );
    assert_eq!(
        timing.note_id, 2,
        "measure C is the third written note (note_id 2, after A's 0 and B's 1); \
         `respect_sequence: false` must not resolve it against the # sequence-expanded \
         playback position (which would land on B's note_id 1)"
    );
}
