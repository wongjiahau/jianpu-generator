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
        render_svgs_from_source_filtered(input, "test.jianpu", Some(&["Soprano".into()])).unwrap();
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

#[test]
fn adjacent_beat_group_underlines_have_gap_between_them() {
    // "2_3=4=" is beat 2 and "6_7_" is beat 3 — both get a level-0 beam underline.
    // The underline for beat 2 must end strictly before the underline for beat 3 starts.
    let source = concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n",
        "[parts]\nS = notes\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n0 2_3=4= 6_7_ 0\n",
    );
    let score = compile(source, "test").unwrap();
    let config = render_config::RenderConfig::from_metadata(&score.metadata);
    let header = grid_layout::types::Header {
        title: score.metadata.title.clone(),
        subtitle: score.metadata.subtitle.clone(),
        author: score.metadata.author.clone(),
    };
    let blocks = compiler::compile(&score);
    let grid_pages = grid_layout::layout(&blocks, &config, &header, 595.0, 842.0);
    let abs = coordinate_resolver::resolve(&grid_pages, config.note_number_width as f32);

    let mut underlines: Vec<(f32, f32)> = abs[0]
        .elements
        .iter()
        .filter_map(|e| {
            if let compositor::types::AbsoluteContent::Underline { width, level: 0 } = &e.content {
                Some((e.x, *width))
            } else {
                None
            }
        })
        .collect();
    underlines.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    assert_eq!(underlines.len(), 2, "expected two level-0 underlines");
    let (x1, w1) = underlines[0];
    let (x2, _) = underlines[1];
    assert!(
        x2 > x1 + w1,
        "underlines should have a gap but they touch: beat2 ends at {:.1}, beat3 starts at {:.1}",
        x1 + w1,
        x2
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
