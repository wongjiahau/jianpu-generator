mod ast;
mod combiner;
mod desugar;
mod error;
mod error_reporter;
mod grouper;
mod layout;
mod midi;
mod parser;
mod pdf;
mod renderer;
mod utils;
mod wav;

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "jianpu", about = "Generate JianPu notation files")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Generate {
        #[command(subcommand)]
        format: GenerateFormat,
    },
}

#[derive(Subcommand)]
enum GenerateFormat {
    Pdf {
        input: PathBuf,
        #[arg(long, help = "Output file stem (extension is added automatically)")]
        output: Option<PathBuf>,
        #[arg(long, value_delimiter = ',', num_args = 0.., help = "Comma-separated list of track names to include (e.g. --tracks S1,S2)")]
        tracks: Vec<String>,
        #[arg(
            long,
            help = "Generate one file per track instead of a single combined file"
        )]
        split_tracks: bool,
    },
    Svg {
        input: PathBuf,
        #[arg(long, help = "Output file stem (extension is added automatically)")]
        output: Option<PathBuf>,
        #[arg(long, value_delimiter = ',', num_args = 0.., help = "Comma-separated list of track names to include (e.g. --tracks S1,S2)")]
        tracks: Vec<String>,
        #[arg(
            long,
            help = "Generate one file per track instead of a single combined file"
        )]
        split_tracks: bool,
    },
    Midi {
        input: PathBuf,
        #[arg(long, help = "Output file stem (extension is added automatically)")]
        output: Option<PathBuf>,
        #[arg(long, value_delimiter = ',', num_args = 0.., help = "Comma-separated list of track names to include (e.g. --tracks S1,S2)")]
        tracks: Vec<String>,
        #[arg(
            long,
            help = "Generate one file per track instead of a single combined file"
        )]
        split_tracks: bool,
    },
    Wav {
        input: PathBuf,
        #[arg(long, help = "Output file stem (extension is added automatically)")]
        output: Option<PathBuf>,
        #[arg(long, value_delimiter = ',', num_args = 0.., help = "Comma-separated list of track names to include (e.g. --tracks S1,S2)")]
        tracks: Vec<String>,
        #[arg(
            long,
            help = "Generate one file per track instead of a single combined file"
        )]
        split_tracks: bool,
    },
}

fn main() {
    let args = Args::parse();

    let result = match args.command {
        Commands::Generate { format } => run_generate(format),
    };

    if let Err(e) = result {
        error_reporter::render(&e);
        std::process::exit(1);
    }
}

fn output_stem(input: &Path, tracks: &[String], output: Option<&Path>) -> PathBuf {
    match output {
        Some(out) => out.with_extension(""),
        None => {
            let stem = input.file_stem().unwrap_or_default().to_string_lossy();
            let suffix = if tracks.is_empty() {
                stem.into_owned()
            } else {
                format!("{} - {}", stem, tracks.join("&"))
            };
            input.with_file_name(suffix)
        }
    }
}

fn sanitize_track_name(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "-")
}

fn run_generate(format: GenerateFormat) -> Result<(), error::JianPuError> {
    match format {
        GenerateFormat::Pdf {
            input,
            output,
            tracks,
            split_tracks,
        } => {
            let mut score = parse_and_group(&input)?;
            if split_tracks {
                let effective_tracks = if !tracks.is_empty() {
                    tracks.clone()
                } else {
                    collect_track_names(&score)
                };
                if effective_tracks.is_empty() {
                    eprintln!("warning: --split-tracks given but score has no named tracks; generating single file");
                } else {
                    let base = output_stem(&input, &[], output.as_deref());
                    let base_name = base
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    for track in &effective_tracks {
                        let mut score_clone = score.clone();
                        filter_tracks(&mut score_clone, std::slice::from_ref(track));
                        let row_height = score_clone.metadata.row_height;
                        let note_number_width = score_clone.metadata.note_number_width;
                        let pages = layout::layout(&score_clone, 595.0, 842.0);
                        let svgs = renderer::render(&pages, row_height, note_number_width);
                        let pdf_bytes = pdf::write_pdf(&svgs)?;
                        let safe_track = sanitize_track_name(track);
                        let track_path = base
                            .with_file_name(format!("{} - {}", base_name, safe_track))
                            .with_extension("pdf");
                        write_file(&track_path, &pdf_bytes)?;
                        println!("written to {:?}", track_path);
                    }
                    return Ok(());
                }
            }
            filter_tracks(&mut score, &tracks);
            let row_height = score.metadata.row_height;
            let note_number_width = score.metadata.note_number_width;
            let pages = layout::layout(&score, 595.0, 842.0);
            let svgs = renderer::render(&pages, row_height, note_number_width);
            let pdf_bytes = pdf::write_pdf(&svgs)?;
            let output_path = output_stem(&input, &tracks, output.as_deref()).with_extension("pdf");
            write_file(&output_path, &pdf_bytes)?;
            println!("written to {:?}", output_path);
            Ok(())
        }
        GenerateFormat::Svg {
            input,
            output,
            tracks,
            split_tracks,
        } => {
            let mut score = parse_and_group(&input)?;
            if split_tracks {
                let effective_tracks = if !tracks.is_empty() {
                    tracks.clone()
                } else {
                    collect_track_names(&score)
                };
                if effective_tracks.is_empty() {
                    eprintln!("warning: --split-tracks given but score has no named tracks; generating single file");
                } else {
                    let base = output_stem(&input, &[], output.as_deref());
                    let base_name = base
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    for track in &effective_tracks {
                        let mut score_clone = score.clone();
                        filter_tracks(&mut score_clone, std::slice::from_ref(track));
                        let row_height = score_clone.metadata.row_height;
                        let note_number_width = score_clone.metadata.note_number_width;
                        let pages = layout::layout(&score_clone, 595.0, 842.0);
                        let svgs = renderer::render(&pages, row_height, note_number_width);
                        let safe_track = sanitize_track_name(track);
                        let track_base =
                            base.with_file_name(format!("{} - {}", base_name, safe_track));
                        for (i, svg) in svgs.iter().enumerate() {
                            let path = if svgs.len() == 1 {
                                track_base.with_extension("svg")
                            } else {
                                track_base.with_extension(format!("{}.svg", i + 1))
                            };
                            write_file(&path, svg.as_bytes())?;
                            println!("written to {:?}", path);
                        }
                    }
                    return Ok(());
                }
            }
            filter_tracks(&mut score, &tracks);
            let row_height = score.metadata.row_height;
            let note_number_width = score.metadata.note_number_width;
            let pages = layout::layout(&score, 595.0, 842.0);
            let svgs = renderer::render(&pages, row_height, note_number_width);
            let output_path = output_stem(&input, &tracks, output.as_deref()).with_extension("svg");
            for (i, svg) in svgs.iter().enumerate() {
                let path = if svgs.len() == 1 {
                    output_path.clone()
                } else {
                    output_path.with_extension(format!("{}.svg", i + 1))
                };
                write_file(&path, svg.as_bytes())?;
                println!("written to {:?}", path);
            }
            Ok(())
        }
        GenerateFormat::Midi {
            input,
            output,
            tracks,
            split_tracks,
        } => {
            let mut score = parse_and_group(&input)?;
            if split_tracks {
                let effective_tracks = if !tracks.is_empty() {
                    tracks.clone()
                } else {
                    collect_track_names(&score)
                };
                if effective_tracks.is_empty() {
                    eprintln!("warning: --split-tracks given but score has no named tracks; generating single file");
                } else {
                    let base = output_stem(&input, &[], output.as_deref());
                    let base_name = base
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    for track in &effective_tracks {
                        let mut score_clone = score.clone();
                        filter_tracks(&mut score_clone, std::slice::from_ref(track));
                        let midi_bytes = midi::write_midi(&score_clone);
                        let safe_track = sanitize_track_name(track);
                        let track_path = base
                            .with_file_name(format!("{} - {}", base_name, safe_track))
                            .with_extension("mid");
                        write_file(&track_path, &midi_bytes)?;
                        println!("written to {:?}", track_path);
                    }
                    return Ok(());
                }
            }
            filter_tracks(&mut score, &tracks);
            let midi_bytes = midi::write_midi(&score);
            let output_path = output_stem(&input, &tracks, output.as_deref()).with_extension("mid");
            write_file(&output_path, &midi_bytes)?;
            println!("written to {:?}", output_path);
            Ok(())
        }
        GenerateFormat::Wav {
            input,
            output,
            tracks,
            split_tracks,
        } => {
            let mut score = parse_and_group(&input)?;
            if split_tracks {
                let effective_tracks = if !tracks.is_empty() {
                    tracks.clone()
                } else {
                    collect_track_names(&score)
                };
                if effective_tracks.is_empty() {
                    eprintln!("warning: --split-tracks given but score has no named tracks; generating single file");
                } else {
                    let base = output_stem(&input, &[], output.as_deref());
                    let base_name = base
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    for track in &effective_tracks {
                        let mut score_clone = score.clone();
                        filter_tracks(&mut score_clone, std::slice::from_ref(track));
                        let midi_bytes = midi::write_midi(&score_clone);
                        let wav_bytes = wav::write_wav(&midi_bytes);
                        let safe_track = sanitize_track_name(track);
                        let track_path = base
                            .with_file_name(format!("{} - {}", base_name, safe_track))
                            .with_extension("wav");
                        write_file(&track_path, &wav_bytes)?;
                        println!("written to {:?}", track_path);
                    }
                    return Ok(());
                }
            }
            filter_tracks(&mut score, &tracks);
            let midi_bytes = midi::write_midi(&score);
            let wav_bytes = wav::write_wav(&midi_bytes);
            let output_path = output_stem(&input, &tracks, output.as_deref()).with_extension("wav");
            write_file(&output_path, &wav_bytes)?;
            println!("written to {:?}", output_path);
            Ok(())
        }
    }
}

fn parse_and_group(input: &Path) -> Result<ast::grouped::Score, error::JianPuError> {
    let content = std::fs::read_to_string(input).map_err(|e| {
        error::JianPuError::new(
            error::Span::new(0, 0),
            format!("could not read {:?}: {}", input, e),
        )
    })?;
    let filename = input.to_string_lossy().to_string();
    let doc = parser::parse(&content, &filename).map_err(|e| e.with_path(input))?;
    grouper::group(doc).map_err(|e| e.with_path(input))
}

fn filter_tracks(score: &mut ast::grouped::Score, tracks: &[String]) {
    if tracks.is_empty() {
        return;
    }
    for measure in &mut score.measures {
        measure.parts.retain(|part| {
            part.name()
                .as_ref()
                .is_some_and(|name| tracks.contains(name))
        });
    }
}

fn collect_track_names(score: &ast::grouped::Score) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut names = Vec::new();
    for measure in &score.measures {
        for part in &measure.parts {
            if let Some(name) = part.name() {
                if seen.insert(name.clone()) {
                    names.push(name.clone());
                }
            }
        }
    }
    names
}

fn write_file(path: &Path, data: &[u8]) -> Result<(), error::JianPuError> {
    std::fs::write(path, data).map_err(|e| {
        error::JianPuError::new(
            error::Span::new(0, 0),
            format!("could not write {:?}: {}", path, e),
        )
    })
}
