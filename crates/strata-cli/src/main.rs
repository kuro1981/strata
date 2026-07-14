use clap::{Args as ClapArgs, Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use strata_sml::Span;

/// トップレベル CLI。`fmt` / `build` / `render` サブコマンドを提供する(旧 YAML→HTML
/// フローは M4 の vault 削除(docs/sml-render-m4-handoff.md D-R1)で撤去済み)。
/// `render` は canonical グラフ(層2)から Typst マークアップを直接出力する
/// (D-R3。中間 JSON は介さない。strata-html への導線は持たない — D19)。
#[derive(Parser, Debug)]
#[command(author, version, about = "Strata Document Builder CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Format an SML file in place (ID re-injection formatter)
    Fmt(FmtArgs),
    /// Build an SML file into a canonical graph (JSON)
    Build(BuildArgs),
    /// Render an SML file into Typst markup (build + render, no intermediate JSON)
    Render(RenderArgs),
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
struct RenderArgs {
    /// SML file to render
    file: PathBuf,

    /// Write the Typst source to this file instead of stdout
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Fmt(fmt_args) => run_fmt(fmt_args),
        Command::Build(build_args) => run_build(build_args),
        Command::Render(render_args) => run_render(render_args),
    }
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
            // D17: Warning は stderr に出しつつ exit 0(全か無かの対象外)。
            print_warnings(&out.warnings, &src);

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
            // D17: Warning は stderr に出しつつ exit 0(全か無かの対象外)。
            print_warnings(&out.warnings, &src);

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

/// `strata-cli render` サブコマンド(docs/sml-render-m4-handoff.md D-R3)。
///
/// 内部で `strata_build::build` → `strata_typst::render_to_typst` を直結する
/// (中間 JSON なし)。exit code: 成功 0 / 読み書き失敗 1 / BuildError・render エラー
/// 2。`root: None`(フロントマター無し)は D21 の案内文言で exit 2。
fn run_render(args: RenderArgs) {
    let src = match fs::read_to_string(&args.file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read input file: {}", e);
            std::process::exit(1);
        }
    };

    let out = match strata_build::build(&src) {
        Err(errors) => {
            // build と同じ表示形式を再利用する(D-R3)。
            for e in &errors {
                for line in format_build_error(e, &src) {
                    eprintln!("{}", line);
                }
            }
            std::process::exit(2);
        }
        Ok(out) => out,
    };

    // D17: Warning は stderr に出しつつ exit 0(全か無かの対象外)。
    print_warnings(&out.warnings, &src);

    let Some(root) = out.root else {
        // D21: フロントマター無し(root: None)は render 対象外。
        eprintln!("フロントマターがありません。`strata fmt` を先に実行してください。");
        std::process::exit(2);
    };

    // D-R2 1.: フォールバック文書名は入力ファイル名(拡張子抜き)を渡す(裁量。
    // Document.title も最初の H1 も無い場合にのみ使われる)。
    let fallback_title = args.file.file_stem().and_then(|s| s.to_str()).unwrap_or("untitled");

    let typst_src = match strata_typst::render_to_typst(&out.graph, root, fallback_title) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("render error: {}", e);
            std::process::exit(2);
        }
    };

    match args.output {
        Some(path) => {
            if let Err(e) = write_atomic(&path, &typst_src) {
                eprintln!("Failed to write output file: {}", e);
                std::process::exit(1);
            }
        }
        None => {
            print!("{}", typst_src);
        }
    }
    std::process::exit(0);
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

/// D17: `Warning` 診断を「行:列: warning: 種別: メッセージ」形式で stderr に出す。
/// `fmt` / `build` の両方で共有する(全か無かの対象外なので exit code には影響しない)。
fn print_warnings(warnings: &[strata_sml::Diag], src: &str) {
    for w in warnings {
        let (line, col) = w.span.line_col(src);
        eprintln!("{}:{}: warning: {:?}: {}", line, col, w.kind, w.msg);
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
