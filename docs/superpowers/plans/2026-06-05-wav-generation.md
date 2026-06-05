# WAV Generation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `generate wav` subcommand that synthesizes a `.jianpu` score to a WAV audio file using an embedded soundfont with Choir Aahs (GM patch 52).

**Architecture:** Reuse `write_midi()` to produce MIDI bytes in-memory, then synthesize to PCM via `oxisynth` (SF2 synthesizer) using the bundled GeneralUser GS soundfont embedded with `include_bytes!`, and encode to WAV with `hound`. The `ProgramChange(0)` event emitted by the MIDI module is patched to `ProgramChange(52)` inside `wav.rs`, keeping `midi.rs` unchanged.

**Tech Stack:** `oxisynth 0.0.5` (SF2 synthesis), `hound 3` (WAV encoding), `midly 0.5` (already in deps — used here for MIDI parsing), GeneralUser GS v1.471.sf2 (bundled soundfont).

---

## File Map

| Action | Path | Responsibility |
|--------|------|---------------|
| Modify | `Cargo.toml` | Add oxisynth, hound |
| Add | `fonts/GeneralUser_GS.sf2` | Bundled soundfont (~30 MB, committed to repo) |
| Create | `src/wav.rs` | MIDI→PCM synthesis + WAV encoding |
| Modify | `src/main.rs` | Add `Wav` variant to `GenerateFormat`, wire to `wav::write_wav` |
| Modify | `tests/integration.rs` | Add `generate_wav_produces_wav` test |

---

## Task 1: Add dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add oxisynth and hound to Cargo.toml**

Open `Cargo.toml` and add to `[dependencies]`:

```toml
oxisynth = "0.0.5"
hound = "3"
```

Final `[dependencies]` block:

```toml
[dependencies]
ariadne = "0.6"
clap = { version = "4", features = ["derive"] }
svg2pdf = "0.12"
pdf-writer = "0.12"
itertools = "0.13"
nonempty = "0.10"
midly = "0.5"
oxisynth = "0.0.5"
hound = "3"
```

- [ ] **Step 2: Verify crates resolve**

```bash
cargo fetch
```

Expected: exits 0 with no errors.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add oxisynth and hound dependencies for WAV generation"
```

---

## Task 2: Commit the soundfont

**Files:**
- Add: `fonts/GeneralUser_GS.sf2`

- [ ] **Step 1: Download GeneralUser GS v1.471**

Download `GeneralUser GS v1.471.sf2` from S. Christian Collins' official distribution and place it at:

```
fonts/GeneralUser_GS.sf2
```

Verify the file is present and has a non-zero size:

```bash
ls -lh fonts/GeneralUser_GS.sf2
```

Expected: a file around 30 MB.

- [ ] **Step 2: Add to .gitignore exclusion — confirm it is NOT ignored**

```bash
git check-ignore -v fonts/GeneralUser_GS.sf2
```

Expected: no output (not ignored). If it is ignored, remove the matching rule from `.gitignore`.

- [ ] **Step 3: Commit**

```bash
git add fonts/GeneralUser_GS.sf2
git commit -m "chore: add GeneralUser GS soundfont for WAV synthesis"
```

---

## Task 3: Create src/wav.rs — helper unit tests

**Files:**
- Create: `src/wav.rs`

- [ ] **Step 1: Create src/wav.rs with stub functions and tests**

```rust
use hound::{SampleFormat, WavSpec, WavWriter};
use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use oxisynth::{MidiEvent, SoundFont, Synth, SynthDescriptor};
use std::io::Cursor;

const SAMPLE_RATE: u32 = 44100;
const CHOIR_AAHS_PROGRAM: u8 = 52;

static SF2_BYTES: &[u8] = include_bytes!("../fonts/GeneralUser_GS.sf2");

pub fn write_wav(midi_bytes: &[u8]) -> Vec<u8> {
    todo!()
}

fn ticks_to_samples(ticks: u32, tpq: u32, micros_per_beat: u32) -> usize {
    todo!()
}

fn render_samples(synth: &mut Synth, n: usize, l: &mut Vec<f32>, r: &mut Vec<f32>) {
    todo!()
}

fn encode_wav(l: &[f32], r: &[f32]) -> Vec<u8> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_to_samples_quarter_note_at_120bpm() {
        // 120 BPM = 500_000 µs/beat, TPQ = 480
        // quarter note = 480 ticks = 0.5 s = 22050 samples @ 44100 Hz
        assert_eq!(ticks_to_samples(480, 480, 500_000), 22050);
    }

    #[test]
    fn ticks_to_samples_half_note_at_120bpm() {
        assert_eq!(ticks_to_samples(960, 480, 500_000), 44100);
    }

    #[test]
    fn encode_wav_has_riff_wave_header() {
        let l = vec![0.0f32; 44100];
        let r = vec![0.0f32; 44100];
        let bytes = encode_wav(&l, &r);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
    }

    #[test]
    fn encode_wav_stereo_16bit_44100() {
        let l = vec![0.0f32; 100];
        let r = vec![0.0f32; 100];
        let bytes = encode_wav(&l, &r);
        // WAV spec chunk: channels=2, sample_rate=44100, bits=16
        // bytes 22-23: channels (little-endian u16)
        assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 2);
        // bytes 24-27: sample rate (little-endian u32)
        assert_eq!(u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]), 44100);
        // bytes 34-35: bits per sample (little-endian u16)
        assert_eq!(u16::from_le_bytes([bytes[34], bytes[35]]), 16);
    }
}
```

- [ ] **Step 2: Register module in main.rs**

In `src/main.rs`, add `mod wav;` after `mod utils;`:

```rust
mod ast;
mod combiner;
mod error;
mod grouper;
mod layout;
mod midi;
mod parser;
mod pdf;
mod renderer;
mod utils;
mod wav;
```

- [ ] **Step 3: Run the tests to confirm they fail**

```bash
cargo test wav
```

Expected: compiles, but tests FAIL with `not yet implemented` panics.

---

## Task 4: Implement helper functions

**Files:**
- Modify: `src/wav.rs`

- [ ] **Step 1: Implement ticks_to_samples**

Replace the `ticks_to_samples` stub:

```rust
fn ticks_to_samples(ticks: u32, tpq: u32, micros_per_beat: u32) -> usize {
    ((ticks as f64 * SAMPLE_RATE as f64 * micros_per_beat as f64)
        / (tpq as f64 * 1_000_000.0)) as usize
}
```

- [ ] **Step 2: Implement encode_wav**

Replace the `encode_wav` stub:

```rust
fn encode_wav(l: &[f32], r: &[f32]) -> Vec<u8> {
    let spec = WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut buf: Vec<u8> = Vec::new();
    let mut writer = WavWriter::new(Cursor::new(&mut buf), spec).unwrap();
    for (ls, rs) in l.iter().zip(r.iter()) {
        writer
            .write_sample((ls.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
            .unwrap();
        writer
            .write_sample((rs.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
            .unwrap();
    }
    writer.finalize().unwrap();
    buf
}
```

- [ ] **Step 3: Implement render_samples**

Replace the `render_samples` stub:

```rust
fn render_samples(synth: &mut Synth, n: usize, l: &mut Vec<f32>, r: &mut Vec<f32>) {
    let prev = l.len();
    l.resize(prev + n, 0.0);
    r.resize(prev + n, 0.0);
    synth.write(&mut l[prev..], &mut r[prev..]);
}
```

- [ ] **Step 4: Run helper tests**

```bash
cargo test wav::tests::ticks_to_samples -- --nocapture
cargo test wav::tests::encode_wav -- --nocapture
```

Expected: all 4 helper tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/wav.rs src/main.rs
git commit -m "feat: implement WAV helper functions (ticks_to_samples, encode_wav, render_samples)"
```

---

## Task 5: Implement write_wav

**Files:**
- Modify: `src/wav.rs`

- [ ] **Step 1: Replace the write_wav stub**

```rust
pub fn write_wav(midi_bytes: &[u8]) -> Vec<u8> {
    let smf = Smf::parse(midi_bytes).expect("invalid MIDI bytes");
    let tpq = match smf.header.timing {
        Timing::Metrical(t) => t.as_int() as u32,
        _ => 480,
    };

    let mut synth = Synth::new(SynthDescriptor {
        sample_rate: SAMPLE_RATE as f32,
        ..Default::default()
    })
    .expect("synth init failed");

    let sf = SoundFont::load(&mut Cursor::new(SF2_BYTES)).expect("soundfont load failed");
    synth.add_font(sf, true);

    let mut micros_per_beat: u32 = 500_000; // default 120 BPM
    let mut all_l: Vec<f32> = Vec::new();
    let mut all_r: Vec<f32> = Vec::new();

    for event in smf.tracks[0].iter() {
        let delta = event.delta.as_int() as u32;
        if delta > 0 {
            let n = ticks_to_samples(delta, tpq, micros_per_beat);
            render_samples(&mut synth, n, &mut all_l, &mut all_r);
        }
        match &event.kind {
            TrackEventKind::Meta(MetaMessage::Tempo(t)) => {
                micros_per_beat = t.as_int();
            }
            TrackEventKind::Midi { channel, message } => {
                let ch = channel.as_int();
                match message {
                    MidiMessage::ProgramChange { program } => {
                        let p = if program.as_int() == 0 {
                            CHOIR_AAHS_PROGRAM
                        } else {
                            program.as_int()
                        };
                        synth
                            .send_event(MidiEvent::ProgramChange {
                                channel: ch,
                                program_id: p,
                            })
                            .ok();
                    }
                    MidiMessage::NoteOn { key, vel } => {
                        synth
                            .send_event(MidiEvent::NoteOn {
                                channel: ch,
                                key: key.as_int(),
                                vel: vel.as_int(),
                            })
                            .ok();
                    }
                    MidiMessage::NoteOff { key, .. } => {
                        synth
                            .send_event(MidiEvent::NoteOff {
                                channel: ch,
                                key: key.as_int(),
                            })
                            .ok();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // Render 1 second of tail so reverb fully decays
    render_samples(&mut synth, SAMPLE_RATE as usize, &mut all_l, &mut all_r);

    encode_wav(&all_l, &all_r)
}
```

- [ ] **Step 2: Build to verify it compiles**

```bash
cargo build 2>&1
```

Expected: compiles without errors. If `synth.write` or `SynthDescriptor::sample_rate` don't exist at this version, run `cargo doc --open --package oxisynth` to find the correct API names and adjust accordingly.

- [ ] **Step 3: Commit**

```bash
git add src/wav.rs
git commit -m "feat: implement write_wav using oxisynth + GeneralUser GS soundfont"
```

---

## Task 6: Wire generate wav subcommand in main.rs

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add Wav variant to GenerateFormat**

Replace the existing `GenerateFormat` enum:

```rust
#[derive(Subcommand)]
enum GenerateFormat {
    Pdf {
        input: PathBuf,
        output: Option<PathBuf>,
        #[arg(long, value_delimiter = ',', num_args = 0..)]
        tracks: Vec<String>,
    },
    Svg {
        input: PathBuf,
        output: Option<PathBuf>,
        #[arg(long, value_delimiter = ',', num_args = 0..)]
        tracks: Vec<String>,
    },
    Midi {
        input: PathBuf,
        output: Option<PathBuf>,
        #[arg(long, value_delimiter = ',', num_args = 0..)]
        tracks: Vec<String>,
    },
    Wav {
        input: PathBuf,
        output: Option<PathBuf>,
        #[arg(long, value_delimiter = ',', num_args = 0..)]
        tracks: Vec<String>,
    },
}
```

- [ ] **Step 2: Add Wav arm to run_generate**

Inside `run_generate`, after the `GenerateFormat::Midi { .. }` arm, add:

```rust
GenerateFormat::Wav {
    input,
    output,
    tracks,
} => {
    let output_path = output.unwrap_or_else(|| input.with_extension("wav"));
    let mut score = parse_and_group(&input)?;
    filter_tracks(&mut score, &tracks);
    let midi_bytes = midi::write_midi(&score);
    let wav_bytes = wav::write_wav(&midi_bytes);
    write_file(&output_path, &wav_bytes)?;
    println!("written to {:?}", output_path);
    Ok(())
}
```

- [ ] **Step 3: Build**

```bash
cargo build 2>&1
```

Expected: exits 0, no errors.

- [ ] **Step 4: Smoke test manually**

```bash
./target/debug/jianpu generate wav demo.jianpu /tmp/demo_test.wav
```

Expected: prints `written to "/tmp/demo_test.wav"`. Open the WAV file in any audio player and confirm it sounds like a choir singing the notes (not piano).

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: add generate wav subcommand"
```

---

## Task 7: Add integration test

**Files:**
- Modify: `tests/integration.rs`

- [ ] **Step 1: Add generate_wav_produces_wav test**

Append to `tests/integration.rs`:

```rust
#[test]
fn generate_wav_produces_wav() {
    let input_path = "/tmp/test_score_wav.jianpu";
    let output_path = "/tmp/test_score.wav";
    fs::write(input_path, basic_jianpu_input()).unwrap();

    let status = jianpu_cmd()
        .args(["generate", "wav", input_path, output_path])
        .status()
        .unwrap();

    assert!(status.success(), "generate wav command failed");
    let bytes = fs::read(output_path).unwrap();
    assert_eq!(&bytes[0..4], b"RIFF", "output is not a valid WAV file");
    assert_eq!(&bytes[8..12], b"WAVE", "output is not a valid WAV file");
    // Expect a meaningful amount of audio data (>100 KB for a few seconds of stereo 16-bit)
    assert!(bytes.len() > 100_000, "WAV output is suspiciously small");

    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(output_path);
}
```

- [ ] **Step 2: Run integration tests**

```bash
cargo test --test integration 2>&1
```

Expected: all 3 integration tests pass. Note: the WAV test will take a few seconds to synthesize audio — this is normal.

- [ ] **Step 3: Run full test suite**

```bash
cargo test 2>&1
```

Expected: all tests pass.

- [ ] **Step 4: Final commit**

```bash
git add tests/integration.rs
git commit -m "test: add integration test for generate wav"
```
