mod ast;
mod combiner;
mod error;
mod error_reporter;
mod grouper;
mod layout;
mod midi;
mod parser;
mod pdf;
mod renderer;
mod utils;

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

fn run_generate(format: GenerateFormat) -> Result<(), error::JianPuError> {
    match format {
        GenerateFormat::Pdf {
            input,
            output,
            tracks,
        } => {
            let output_path = output.unwrap_or_else(|| input.with_extension("pdf"));
            let mut score = parse_and_group(&input)?;
            filter_tracks(&mut score, &tracks);
            let row_height = score.metadata.row_height;
            let note_number_width = score.metadata.note_number_width;
            let pages = layout::layout(&score, 595.0, 842.0);
            let svgs = renderer::render(&pages, row_height, note_number_width);
            let pdf_bytes = pdf::write_pdf(&svgs)?;
            write_file(&output_path, &pdf_bytes)?;
            println!("written to {:?}", output_path);
            Ok(())
        }
        GenerateFormat::Svg {
            input,
            output,
            tracks,
        } => {
            let output_path = output.unwrap_or_else(|| input.with_extension("svg"));
            let mut score = parse_and_group(&input)?;
            filter_tracks(&mut score, &tracks);
            let row_height = score.metadata.row_height;
            let note_number_width = score.metadata.note_number_width;
            let pages = layout::layout(&score, 595.0, 842.0);
            let svgs = renderer::render(&pages, row_height, note_number_width);
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
        } => {
            let output_path = output.unwrap_or_else(|| input.with_extension("mid"));
            let mut score = parse_and_group(&input)?;
            filter_tracks(&mut score, &tracks);
            let midi_bytes = midi::write_midi(&score);
            write_file(&output_path, &midi_bytes)?;
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
        measure
            .parts
            .retain(|part| part.name.as_ref().is_some_and(|name| tracks.contains(name)));
    }
}

fn write_file(path: &Path, data: &[u8]) -> Result<(), error::JianPuError> {
    std::fs::write(path, data).map_err(|e| {
        error::JianPuError::new(
            error::Span::new(0, 0),
            format!("could not write {:?}: {}", path, e),
        )
    })
}
