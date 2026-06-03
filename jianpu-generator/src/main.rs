mod ast;
mod error;
mod grouper;
mod layout;
mod parser;
mod pdf;
mod renderer;
mod utils;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "jianpu", about = "Generate JianPu notation PDFs")]
struct Args {
    /// Path to the .jianpu input file
    input: PathBuf,

    /// Path for the output PDF (default: input filename with .pdf extension)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    let output_path = args.output.unwrap_or_else(|| args.input.with_extension("pdf"));

    let input = match std::fs::read_to_string(&args.input) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("error: could not read {:?}: {}", args.input, e);
            std::process::exit(1);
        }
    };

    let filename = args.input.to_string_lossy().to_string();

    let result = (|| -> Result<(), error::JianPuError> {
        let doc = parser::parse(&input, &filename)?;
        let score = grouper::group(doc)?;
        let cell_size = score.metadata.cell_size;
        let pages = layout::layout(&score, 595.0, 842.0);
        let svgs = renderer::render(&pages, cell_size);
        let pdf_bytes = pdf::write_pdf(&svgs)?;
        std::fs::write(&output_path, &pdf_bytes).map_err(|e| error::JianPuError::new(
            error::Span::new(0, 0),
            format!("could not write output PDF: {}", e),
        ))?;
        println!("written to {:?}", output_path);
        Ok(())
    })();

    if let Err(e) = result {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}
