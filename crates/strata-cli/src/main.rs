use clap::{Parser, ValueEnum};
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "Strata Document Builder CLI", long_about = None)]
struct Args {
    /// Input YAML document file
    #[arg(short, long)]
    input: PathBuf,

    /// Output file path
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output format. If not specified, inferred from output file extension
    #[arg(short, long, value_enum)]
    format: Option<Format>,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
enum Format {
    Html,
    Typst,
}

fn main() {
    let args = Args::parse();

    // フォーマットと出力パスの決定
    let format = args.format.unwrap_or_else(|| {
        if let Some(ref out) = args.output {
            if let Some(ext) = out.extension() {
                if ext == "typ" {
                    return Format::Typst;
                }
            }
        }
        Format::Html
    });

    let output_path = args.output.unwrap_or_else(|| {
        match format {
            Format::Html => PathBuf::from("output.html"),
            Format::Typst => PathBuf::from("output.typ"),
        }
    });

    println!("Loading document from: {:?}", args.input);
    let yaml_str = match fs::read_to_string(&args.input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read input file: {}", e);
            std::process::exit(1);
        }
    };

    println!("Parsing Strata document...");
    let parser = strata_vault::Parser::new();
    let vault = match parser.parse_vault(&yaml_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            std::process::exit(1);
        }
    };

    // 不変条件の検証
    println!("Validating document invariants...");
    let violations = strata_core::invariants::validate(&vault.graph);
    if !violations.is_empty() {
        eprintln!("Invariants violation detected!");
        for v in violations {
            eprintln!(" - {:?}", v);
        }
        std::process::exit(1);
    }

    println!("Rendering to {:?}...", format);
    let output_content = match format {
        Format::Html => {
            match strata_html::render_to_html(&vault.graph, vault.root_id) {
                Ok(html) => html,
                Err(e) => {
                    eprintln!("HTML rendering error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Format::Typst => {
            match strata_typst::render_to_typst(&vault.graph, vault.root_id) {
                Ok(typ) => typ,
                Err(e) => {
                    eprintln!("Typst rendering error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    };

    println!("Writing output to: {:?}", output_path);
    if let Err(e) = fs::write(&output_path, output_content) {
        eprintln!("Failed to write output file: {}", e);
        std::process::exit(1);
    }

    println!("Done! Successfully compiled Strata document.");
}
