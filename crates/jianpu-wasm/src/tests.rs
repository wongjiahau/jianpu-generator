use super::*;
use types::DiagnosticSeverity;

#[test]
fn ok_response_has_svgs() {
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
    let resp = render_response(input, None, None);
    match resp {
        RenderResponse::Ok { svgs } => {
            assert_eq!(svgs.len(), 1);
            assert!(svgs[0].starts_with("<svg"));
        }
        RenderResponse::Err { .. } => panic!("expected ok"),
    }
}

#[test]
fn list_parts_response_returns_declarations() {
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
    let resp = list_parts_response(input);
    match resp {
        ListPartsResponse::Ok { parts } => {
            assert_eq!(parts.len(), 2);
            assert_eq!(parts[0].abbreviation, "Soprano");
            assert_eq!(parts[1].abbreviation, "Alto");
        }
        ListPartsResponse::Err { diagnostics } => {
            panic!("expected ok: {}", diagnostics[0].message);
        }
    }
}

#[test]
fn render_with_disabled_lyrics_hides_lyrics_for_part() {
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
    let all = match render_response(input, None, None) {
        RenderResponse::Ok { svgs } => svgs,
        RenderResponse::Err { .. } => panic!("expected ok"),
    };
    let alto_lyrics_hidden = match render_response(input, None, Some(vec!["Alto".into()])) {
        RenderResponse::Ok { svgs } => svgs,
        RenderResponse::Err { .. } => panic!("expected ok"),
    };
    assert!(all[0].contains("sop"));
    assert!(all[0].contains("alt"));
    assert!(alto_lyrics_hidden[0].contains("sop"));
    assert!(!alto_lyrics_hidden[0].contains("alt"));
}

#[test]
fn render_with_enabled_tracks_filters_parts() {
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
    let all = match render_response(input, None, None) {
        RenderResponse::Ok { svgs } => svgs,
        RenderResponse::Err { .. } => panic!("expected ok"),
    };
    let soprano_only = match render_response(input, Some(vec!["Soprano".into()]), None) {
        RenderResponse::Ok { svgs } => svgs,
        RenderResponse::Err { .. } => panic!("expected ok"),
    };
    assert_ne!(all[0], soprano_only[0]);
}

#[test]
fn err_response_has_structured_diagnostic() {
    let resp = render_response("not valid jianpu", None, None);
    match resp {
        RenderResponse::Err { diagnostics } => {
            assert!(!diagnostics.is_empty());
            let d = &diagnostics[0];
            assert_eq!(d.severity, DiagnosticSeverity::Error);
            assert!(!d.message.is_empty());
            assert!(d.report.as_ref().is_some_and(|r| !r.is_empty()));
        }
        RenderResponse::Ok { .. } => panic!("expected err"),
    }
}

#[test]
fn demo_jianpu_renders() {
    let source = include_str!("../../../demo.jianpu");
    let resp = render_response(source, None, None);
    match resp {
        RenderResponse::Ok { svgs } => {
            assert!(
                !svgs.is_empty(),
                "demo.jianpu should render in the wasm path used by the web editor"
            );
        }
        RenderResponse::Err { diagnostics } => {
            panic!(
                "demo.jianpu failed in wasm render path: {}",
                diagnostics[0].message
            );
        }
    }
}

#[cfg(feature = "pdf")]
#[test]
fn demo_jianpu_generates_pdf() {
    let source = include_str!("../../../demo.jianpu");
    let resp = generate_pdf_response(source, None, None);
    match resp {
        GeneratePdfResponse::Ok { pdf } => {
            assert!(pdf.len() > 4);
            assert_eq!(&pdf[0..4], b"%PDF");
        }
        GeneratePdfResponse::Err { diagnostics } => {
            panic!(
                "demo.jianpu failed in wasm pdf path: {}",
                diagnostics[0].message
            );
        }
    }
}

#[cfg(feature = "pdf")]
#[test]
fn demo_jianpu_generates_split_pdf_zip() {
    use std::io::Read;
    use zip::ZipArchive;

    let source = include_str!("../../../demo.jianpu");
    let resp = generate_split_pdfs_response(source, "demo");
    match resp {
        GenerateSplitPdfsResponse::Ok { zip } => {
            assert!(zip.len() > 4);
            assert_eq!(&zip[0..2], b"PK");
            let cursor = std::io::Cursor::new(zip);
            let mut archive = ZipArchive::new(cursor).unwrap();
            assert!(archive.len() >= 1);
            for i in 0..archive.len() {
                let mut file = archive.by_index(i).unwrap();
                let name = file.name().to_string();
                assert!(
                    name.starts_with("demo - ") && name.ends_with(".pdf"),
                    "unexpected zip entry: {name}"
                );
                let mut buf = [0u8; 4];
                file.read_exact(&mut buf).unwrap();
                assert_eq!(&buf, b"%PDF");
            }
        }
        GenerateSplitPdfsResponse::Err { diagnostics } => {
            panic!(
                "demo.jianpu failed in wasm split pdf path: {}",
                diagnostics[0].message
            );
        }
    }
}

#[test]
fn get_measure_at_offset_ok_for_note_in_measure() {
    let source = concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\n",
    );
    let byte_offset = source.find("1 2 3 4").unwrap();
    let resp = get_measure_at_offset_response(source, byte_offset);
    match resp {
        MeasureAtOffsetResponse::Ok { measure_index } => assert_eq!(measure_index, 0),
        MeasureAtOffsetResponse::NotInMeasure => panic!("expected Ok"),
    }
}

#[test]
fn get_measure_at_offset_not_in_measure_for_header() {
    let source = concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\n",
    );
    let resp = get_measure_at_offset_response(source, 0);
    assert!(
        matches!(resp, MeasureAtOffsetResponse::NotInMeasure),
        "expected NotInMeasure"
    );
}

#[cfg(feature = "wav")]
#[test]
fn generate_wav_for_measure_response_returns_riff_wav() {
    let source = concat!(
        "[metadata]\ntitle=\"t\"\nauthor=\"a\"\n\n[parts]\nMelody = notes\n\n",
        "[score]\n(time=4/4 key=C4 bpm=120)\n1 2 3 4\n",
    );
    let resp = generate_wav_for_measure_response(source, 0, None);
    match resp {
        GenerateWavResponse::Ok { wav } => {
            assert!(wav.len() > 4);
            assert_eq!(&wav[0..4], b"RIFF");
        }
        GenerateWavResponse::Err { diagnostics } => {
            panic!("expected Ok: {}", diagnostics[0].message);
        }
    }
}

#[cfg(feature = "wav")]
#[test]
fn demo_jianpu_generates_wav() {
    let source = include_str!("../../../demo.jianpu");
    let resp = generate_wav_response(source, None);
    match resp {
        GenerateWavResponse::Ok { wav } => {
            assert!(wav.len() > 4);
            assert_eq!(&wav[0..4], b"RIFF");
        }
        GenerateWavResponse::Err { diagnostics } => {
            panic!(
                "demo.jianpu failed in wasm wav path: {}",
                diagnostics[0].message
            );
        }
    }
}

#[test]
fn err_span_is_utf8_byte_offset() {
    let source = concat!(
        "[metadata]\n",
        "title = \"你好\"\n",
        "author = \"a\"\n",
        "\n",
        "[parts]\n",
        "Melody = notes lyrics\n",
        "\n",
        "[score]\n",
        "(time=4/4 key=C4 bpm=120)\n",
        "1 2 x 4\n",
        "a b c d\n",
    );
    let token_byte_start = source.find('x').expect("error token in source");
    let resp = render_response(source, None, None);
    let RenderResponse::Err { diagnostics } = resp else {
        panic!("expected err");
    };
    assert_eq!(diagnostics[0].span.start, token_byte_start);
    assert!(
        token_byte_start > 4,
        "span is absolute in source, not line-local"
    );
}
