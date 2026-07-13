use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};
use std::fs;
use std::path::{Path, PathBuf};
use strata_sml::Span;

/// トップレベル CLI。既存の YAML→HTML/Typst フロー(`-i/-o/-f`)を無変更で温存しつつ、
/// `fmt` サブコマンドを追加する(docs/sml-fmt-m2-handoff.md D-F4)。
///
/// `args_conflicts_with_subcommands` でトップレベル引数とサブコマンドの併用を禁止し、
/// `subcommand_negates_reqs` で `fmt` 使用時に `--input` の必須制約を外す。
#[derive(Parser, Debug)]
#[command(author, version, about = "Strata Document Builder CLI", long_about = None)]
#[command(args_conflicts_with_subcommands = true, subcommand_negates_reqs = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    args: Args,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Format an SML file in place (ID re-injection formatter)
    Fmt(FmtArgs),
}

#[derive(ClapArgs, Debug)]
struct FmtArgs {
    /// SML file to format
    file: PathBuf,

    /// Do not write; exit 1 if the file needs formatting, 0 otherwise
    #[arg(long)]
    check: bool,
}

#[derive(ClapArgs, Debug)]
struct Args {
    /// Input YAML document file
    #[arg(short, long, required = true)]
    input: Option<PathBuf>,

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
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Fmt(fmt_args)) => run_fmt(fmt_args),
        None => run_legacy(cli.args),
    }
}

/// 既存の YAML→HTML/Typst フロー。挙動(ログ・exit code)は M1 時点から一切変更しない。
fn run_legacy(args: Args) {
    // `--input` は `required = true` により、サブコマンド未指定時は clap が必ず埋める。
    let input = args.input.expect("--input is required when no subcommand is given (enforced by clap)");

    // フォーマットと出力パスの決定
    let format = args.format.unwrap_or_else(|| {
        if let Some(ref out) = args.output
            && let Some(ext) = out.extension()
            && ext == "typ"
        {
            return Format::Typst;
        }
        Format::Html
    });

    let output_path = args.output.unwrap_or_else(|| match format {
        Format::Html => PathBuf::from("output.html"),
        Format::Typst => PathBuf::from("output.typ"),
    });

    println!("Loading document from: {:?}", input);
    let yaml_str = match fs::read_to_string(&input) {
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
        Format::Html => match strata_html::render_to_html(&vault.graph, vault.root_id) {
            Ok(html) => html,
            Err(e) => {
                eprintln!("HTML rendering error: {}", e);
                std::process::exit(1);
            }
        },
        Format::Typst => match strata_typst::render_to_typst(&vault.graph, vault.root_id) {
            Ok(typ) => typ,
            Err(e) => {
                eprintln!("Typst rendering error: {}", e);
                std::process::exit(1);
            }
        },
    };

    println!("Writing output to: {:?}", output_path);
    if let Err(e) = fs::write(&output_path, output_content) {
        eprintln!("Failed to write output file: {}", e);
        std::process::exit(1);
    }

    println!("Done! Successfully compiled Strata document.");
}

/// `strata-cli fmt` サブコマンド(docs/sml-fmt-m2-handoff.md D-F4)。
fn run_fmt(args: FmtArgs) {
    let src = match fs::read_to_string(&args.file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read input file: {}", e);
            std::process::exit(1);
        }
    };

    match strata_sml::format(&src) {
        Err(diags) => {
            // パースエラー: ファイルには一切触れず、全診断を「行:列: 種別: メッセージ」で出力。
            for diag in &diags {
                let (line, col) = diag.span.line_col(&src);
                eprintln!("{}:{}: {:?}: {}", line, col, diag.kind, diag.msg);
            }
            std::process::exit(2);
        }
        Ok(out) => {
            if out.patches.is_empty() {
                // 変更不要: --check の有無によらず exit 0。
                std::process::exit(0);
            }

            if args.check {
                for patch in &out.patches {
                    let (line, col) = Span::new(patch.at, patch.at + patch.delete).line_col(&src);
                    println!(
                        "{}:{}: delete {} byte(s), insert {:?}",
                        line, col, patch.delete, patch.insert
                    );
                }
                std::process::exit(1);
            }

            if let Err(e) = write_atomic(&args.file, &out.text) {
                eprintln!("Failed to write formatted file: {}", e);
                std::process::exit(1);
            }
            std::process::exit(0);
        }
    }
}

/// 同一ディレクトリに一時ファイルを書いてから rename する原子的な書き込み。
fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "input path has no file name")
    })?;
    let tmp_path = dir.join(format!(".{}.tmp.{}", file_name.to_string_lossy(), std::process::id()));
    fs::write(&tmp_path, contents)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}
