use super::*;

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
fn ditto_part_label_is_merged_into_source_row_label() {
    let input = concat!(
        "[metadata]\n",
        "title = \"t\"\n",
        "author = \"a\"\n",
        "\n",
        "[parts]\n",
        "Soprano (S) = notes\n",
        "Alto (A) = notes\n",
        "\n",
        "[score]\n",
        "(time=4/4 key=C4 bpm=120)\n",
        "1 2 3 4\n",
        "\"\n",
    );
    let score = compile(input, "test.jianpu").unwrap();
    let blocks = crate::compiler::compile(&score);
    assert_eq!(
        blocks[0].rows.len(),
        1,
        "ditto Alto should produce no separate row"
    );
    assert_eq!(
        blocks[0].rows[0].label, "[ALL]",
        "source row label should be [ALL] when all other parts are ditto"
    );
}

#[test]
fn ditto_part_promoted_to_timed_when_source_is_filtered_out() {
    let input = concat!(
        "[metadata]\n",
        "title = \"t\"\n",
        "author = \"a\"\n",
        "\n",
        "[parts]\n",
        "Soprano (S) = notes\n",
        "Alto (A) = notes\n",
        "\n",
        "[score]\n",
        "(time=4/4 key=C4 bpm=120)\n",
        "1 2 3 4\n",
        "\"\n",
    );
    let mut score = compile(input, "test.jianpu").unwrap();
    // Alto is ditto. Filter to Alto only — Soprano (the source) is removed.
    apply_track_filter(&mut score, Some(&["A".to_string()]));
    assert_eq!(score.measures[0].parts.len(), 1, "only Alto should remain");
    assert!(
        matches!(score.measures[0].parts[0], PartRow::Timed(_)),
        "Alto should be promoted from Ditto to Timed when its source Soprano is filtered out"
    );
}
