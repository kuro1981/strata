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
    /// Build an SML file into a canonical graph (JSON)
    Build(BuildArgs),
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
struct BuildArgs {
    /// SML file to build
    file: PathBuf,

    /// Write the graph JSON to this file instead of stdout
    #[arg(short, long)]
    output: Option<PathBuf>,
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
        Some(Command::Build(build_args)) => run_build(build_args),
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
                // patches はオフセット降順(適用順)で持つが、人間向けの表示は
                // 文書順(行番号昇順)にする(2026-07-13 裁定)。
                for patch in out.patches.iter().rev() {
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

/// `strata-cli build` サブコマンド(docs/sml-build-m3-handoff.md D-B6)。
fn run_build(args: BuildArgs) {
    let src = match fs::read_to_string(&args.file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read input file: {}", e);
            std::process::exit(1);
        }
    };

    match strata_build::build(&src) {
        Err(errors) => {
            // 全件収集済みのエラーを「行:列: 種別: メッセージ」で全件出力する(D13 同様、
            // 最初の1件で止めない)。ファイルには一切触れない。
            for e in &errors {
                for line in format_build_error(e, &src) {
                    eprintln!("{}", line);
                }
            }
            std::process::exit(2);
        }
        Ok(out) => {
            let json = match serde_json::to_string_pretty(&out) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("Failed to serialize build output: {}", e);
                    std::process::exit(1);
                }
            };

            match args.output {
                Some(path) => {
                    if let Err(e) = write_atomic(&path, &json) {
                        eprintln!("Failed to write output file: {}", e);
                        std::process::exit(1);
                    }
                }
                None => {
                    println!("{}", json);
                }
            }
            std::process::exit(0);
        }
    }
}

/// `BuildError` 1件を「行:列: 種別: メッセージ」形式の行(複数になりうる)に変換する。
///
/// `Parse` は既存の `fmt` の Diag 表示と同形式(`{line}:{col}: {kind:?}: {msg}`)。
/// 他の variant は `span` を `line_col` で変換して同じ形式に揃える。
///
/// 2点、仕様(sml-build-m3-handoff.md D-B6)に明記が無く裁量で決めた箇所:
/// - `MissingId` は `strata_build::BuildError` 自体にメッセージを持たないため、
///   ここで「strata fmt を先に実行してください」という案内文言を組み立てている。
/// - `DuplicateAlias` は1エラーに複数 `span`(全定義箇所)を持つ。全定義箇所を
///   stderr に出すため、spans の数だけ行を生成する(1エラー=1行ではなくなる)。
/// - `Invariant` は build 後のグラフ検証結果であり `strata_sml::Span`(ソース位置)を
///   持たない。行:列の代わりに `-:-` を用いる。
fn format_build_error(e: &strata_build::BuildError, src: &str) -> Vec<String> {
    use strata_build::BuildError as E;

    fn at(span: strata_sml::Span, src: &str) -> (usize, usize) {
        span.line_col(src)
    }

    match e {
        E::Parse(diag) => {
            let (line, col) = at(diag.span, src);
            vec![format!("{}:{}: {:?}: {}", line, col, diag.kind, diag.msg)]
        }
        E::MissingId { span } => {
            let (line, col) = at(*span, src);
            vec![format!(
                "{}:{}: MissingId: このブロックには ID が付与されていません。`strata fmt` を先に実行してください。",
                line, col
            )]
        }
        E::UnresolvedAlias { alias, span } => {
            let (line, col) = at(*span, src);
            vec![format!("{}:{}: UnresolvedAlias: 参照先 '{}' が見つかりません。", line, col, alias)]
        }
        E::DuplicateAlias { alias, spans } => spans
            .iter()
            .map(|span| {
                let (line, col) = at(*span, src);
                format!("{}:{}: DuplicateAlias: エイリアス '{}' が複数箇所で定義されています。", line, col, alias)
            })
            .collect(),
        E::BadFigure { span, msg } => {
            let (line, col) = at(*span, src);
            vec![format!("{}:{}: BadFigure: {}", line, col, msg)]
        }
        E::Math { span, msg } => {
            let (line, col) = at(*span, src);
            vec![format!("{}:{}: Math: {}", line, col, msg)]
        }
        E::RefTypeMismatch { span, msg } => {
            let (line, col) = at(*span, src);
            vec![format!("{}:{}: RefTypeMismatch: {}", line, col, msg)]
        }
        E::Invariant(v) => vec![format!("-:-: Invariant: {:?}", v)],
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
