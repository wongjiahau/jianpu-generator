use std::fs;
use std::process::Command;

fn jianpu_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_jianpu"))
}

fn basic_jianpu_input() -> &'static str {
    concat!(
        "[metadata]\n",
        "title = \"test score\"\n",
        "author = \"tester\"\n",
        "\n",
        "[parts]\n",
        "Melody = notes lyrics\n",
        "\n",
        "[score]\n",
        "(time=4/4 key=C4 bpm=120)\n",
        "1 2 3 4\n",
        "do re mi fa\n",
    )
}

#[test]
fn generate_pdf_produces_pdf() {
    let input_path = "/tmp/test_pdf_basic.jianpu";
    let output_stem_arg = "/tmp/test_pdf_basic";
    let output_path = "/tmp/test_pdf_basic.pdf";
    fs::write(input_path, basic_jianpu_input()).unwrap();

    let status = jianpu_cmd()
        .args(["generate", "pdf", input_path, "--output", output_stem_arg])
        .status()
        .unwrap();

    assert!(status.success(), "generate pdf command failed");
    let bytes = fs::read(output_path).unwrap();
    assert!(bytes.starts_with(b"%PDF"), "output is not a valid PDF");

    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(output_path);
}

#[test]
fn generate_midi_produces_midi() {
    let input_path = "/tmp/test_score_midi.jianpu";
    let output_stem_arg = "/tmp/test_score_midi_out";
    let output_path = "/tmp/test_score_midi_out.mid";
    fs::write(input_path, basic_jianpu_input()).unwrap();

    let status = jianpu_cmd()
        .args(["generate", "midi", input_path, "--output", output_stem_arg])
        .status()
        .unwrap();

    assert!(status.success(), "generate midi command failed");
    let bytes = fs::read(output_path).unwrap();
    // MIDI files start with "MThd"
    assert!(
        bytes.starts_with(b"MThd"),
        "output is not a valid MIDI file"
    );

    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(output_path);
}

#[test]
fn output_stem_appends_extension() {
    let input_path = "/tmp/test_stem.jianpu";
    let output_stem = "/tmp/test_stem_out";
    let expected_output = "/tmp/test_stem_out.pdf";
    fs::write(input_path, basic_jianpu_input()).unwrap();
    let _ = fs::remove_file(expected_output);

    let status = jianpu_cmd()
        .args(["generate", "pdf", input_path, "--output", output_stem])
        .status()
        .unwrap();

    assert!(status.success(), "generate pdf command failed");
    let bytes = fs::read(expected_output).expect("output file not found at expected stem path");
    assert!(bytes.starts_with(b"%PDF"), "output is not a valid PDF");

    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(expected_output);
}

#[test]
fn generate_wav_produces_wav() {
    let input_path = "/tmp/test_score_wav.jianpu";
    let output_stem_arg = "/tmp/test_score_wav_out";
    let output_path = "/tmp/test_score_wav_out.wav";
    fs::write(input_path, basic_jianpu_input()).unwrap();

    let status = jianpu_cmd()
        .args(["generate", "wav", input_path, "--output", output_stem_arg])
        .status()
        .unwrap();

    assert!(status.success(), "generate wav command failed");
    let bytes = fs::read(output_path).unwrap();
    assert_eq!(&bytes[0..4], b"RIFF", "output is not a valid WAV file");
    assert_eq!(&bytes[8..12], b"WAVE", "output is not a valid WAV file");
    // Expect meaningful audio data (>100 KB for a few seconds of stereo 16-bit 44100 Hz)
    assert!(bytes.len() > 100_000, "WAV output is suspiciously small");

    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(output_path);
}

fn multi_track_jianpu_input() -> &'static str {
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
fn split_tracks_generates_one_pdf_per_track() {
    let input_path = "/tmp/test_split.jianpu";
    let s1_path = "/tmp/test_split - S1.pdf";
    let s2_path = "/tmp/test_split - S2.pdf";
    fs::write(input_path, multi_track_jianpu_input()).unwrap();
    let _ = fs::remove_file(s1_path);
    let _ = fs::remove_file(s2_path);

    let status = jianpu_cmd()
        .args(["generate", "pdf", input_path, "--split-tracks"])
        .status()
        .unwrap();

    assert!(
        status.success(),
        "generate pdf --split-tracks command failed"
    );

    let s1_bytes = fs::read(s1_path).expect("S1 output file not found");
    assert!(
        s1_bytes.starts_with(b"%PDF"),
        "S1 output is not a valid PDF"
    );

    let s2_bytes = fs::read(s2_path).expect("S2 output file not found");
    assert!(
        s2_bytes.starts_with(b"%PDF"),
        "S2 output is not a valid PDF"
    );

    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(s1_path);
    let _ = fs::remove_file(s2_path);
}

#[test]
fn split_tracks_with_output_stem() {
    let input_path = "/tmp/test_split_out.jianpu";
    let s1_path = "/tmp/split_out - S1.pdf";
    let s2_path = "/tmp/split_out - S2.pdf";
    fs::write(input_path, multi_track_jianpu_input()).unwrap();
    let _ = fs::remove_file(s1_path);
    let _ = fs::remove_file(s2_path);

    let status = jianpu_cmd()
        .args([
            "generate",
            "pdf",
            input_path,
            "--output",
            "/tmp/split_out",
            "--split-tracks",
        ])
        .status()
        .unwrap();

    assert!(
        status.success(),
        "generate pdf --output --split-tracks command failed"
    );

    let s1_bytes = fs::read(s1_path).expect("S1 output file not found");
    assert!(
        s1_bytes.starts_with(b"%PDF"),
        "S1 output is not a valid PDF"
    );

    let s2_bytes = fs::read(s2_path).expect("S2 output file not found");
    assert!(
        s2_bytes.starts_with(b"%PDF"),
        "S2 output is not a valid PDF"
    );

    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(s1_path);
    let _ = fs::remove_file(s2_path);
}
