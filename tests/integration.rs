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
        "parts = notes: lyrics:\n",
        "\n",
        "[score]\n",
        "(time=4/4 key=C4 bpm=120)\n",
        "1 2 3 4\n",
        "do re mi fa\n",
    )
}

#[test]
fn generate_pdf_produces_pdf() {
    let input_path = "/tmp/test_score.jianpu";
    let output_path = "/tmp/test_score.pdf";
    fs::write(input_path, basic_jianpu_input()).unwrap();

    let status = jianpu_cmd()
        .args(["generate", "pdf", input_path, output_path])
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
    let output_path = "/tmp/test_score.mid";
    fs::write(input_path, basic_jianpu_input()).unwrap();

    let status = jianpu_cmd()
        .args(["generate", "midi", input_path, output_path])
        .status()
        .unwrap();

    assert!(status.success(), "generate midi command failed");
    let bytes = fs::read(output_path).unwrap();
    // MIDI files start with "MThd"
    assert!(bytes.starts_with(b"MThd"), "output is not a valid MIDI file");

    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(output_path);
}
