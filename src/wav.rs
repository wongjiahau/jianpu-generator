use hound::{SampleFormat, WavSpec, WavWriter};
use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use oxisynth::{MidiEvent, SoundFont, Synth, SynthDescriptor};
use std::io::Cursor;

const SAMPLE_RATE: u32 = 44100;
const CHOIR_AAHS_PROGRAM: u8 = 52;

static SF2_BYTES: &[u8] = include_bytes!("../fonts/GeneralUser_GS.sf2");

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
        let delta = event.delta.as_int();
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

fn ticks_to_samples(ticks: u32, tpq: u32, micros_per_beat: u32) -> usize {
    ((ticks as f64 * SAMPLE_RATE as f64 * micros_per_beat as f64) / (tpq as f64 * 1_000_000.0))
        as usize
}

fn render_samples(synth: &mut Synth, n: usize, l: &mut Vec<f32>, r: &mut Vec<f32>) {
    let prev = l.len();
    l.resize(prev + n, 0.0);
    r.resize(prev + n, 0.0);
    synth.write_f32(n, &mut l[prev..], 0, 1, &mut r[prev..], 0, 1);
}

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
        assert_eq!(
            u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
            44100
        );
        // bytes 34-35: bits per sample (little-endian u16)
        assert_eq!(u16::from_le_bytes([bytes[34], bytes[35]]), 16);
    }
}
