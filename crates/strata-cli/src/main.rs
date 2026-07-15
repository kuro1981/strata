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
    /// Apply a declarative view definition to an SML file's canonical graph,
    /// producing template-consumable data files (YAML). See docs/view-def-v1.md.
    View(ViewArgs),
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

    /// Hide blocks (and their contains subtree) carrying this class (D23).
    /// May be repeated: `--hide note --hide actual-name`.
    #[arg(long = "hide")]
    hide: Vec<String>,
}

#[derive(ClapArgs, Debug)]
struct ViewArgs {
    /// SML file to build and apply the view definition to
    file: PathBuf,

    /// View definition YAML (D30〜D34, docs/view-def-v1.md)
    #[arg(long = "view")]
    view: PathBuf,

    /// Directory that output file paths (declared in the view definition) are
    /// relative to. Defaults to the current directory.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Restrict output to files tagged for this profile (D34). Omit to emit
    /// every file the view definition declares (union of all profiles) —
    /// this is what a template build that reads multiple profile-specific
    /// files (e.g. submit/check) in one pass needs.
    #[arg(long)]
    profile: Option<String>,

    /// Dry-run: report unfulfilled manifest slots and unused graph nodes
    /// instead of writing files (D33). Requires the view definition to
    /// declare a `manifest:` path.
    #[arg(long)]
    check: bool,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Fmt(fmt_args) => run_fmt(fmt_args),
        Command::Build(build_args) => run_build(build_args),
        Command::Render(render_args) => run_render(render_args),
        Command::View(view_args) => run_view(view_args),
    }
}

/// `strata-cli view` サブコマンド(D30/D33、docs/view-v1-handoff.md WP-W2)。
///
/// 内部で `strata_build::build` → `strata_view::apply`(または `--check` なら
/// `strata_view::check`)を呼ぶ。exit code は他コマンドと同じ 0/1/2 の慣習:
/// 2 = SML/ビュー定義/マニフェストを読めない・パースできない(全か無か)、
/// 1 = 書き込み失敗、または `--check` で診断あり、0 = 成功/診断なし。
/// Warning は stderr に出しつつ exit 0(D17 と同じ扱い)。
fn run_view(args: ViewArgs) {
    let src = match fs::read_to_string(&args.file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read input file: {}", e);
            std::process::exit(1);
        }
    };

    let view_src = match fs::read_to_string(&args.view) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read view definition file: {}", e);
            std::process::exit(1);
        }
    };

    let out = match strata_build::build(&src) {
        Err(errors) => {
            for e in &errors {
                for line in format_build_error(e, &src) {
                    eprintln!("{}", line);
                }
            }
            std::process::exit(2);
        }
        Ok(out) => out,
    };
    print_warnings(&out.warnings, &src);

    let view = match strata_view::parse_view_def(&view_src) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("-:-: ViewDefError: {}", e);
            std::process::exit(2);
        }
    };

    if args.check {
        run_view_check(&out, &view, &args);
        return;
    }

    let profile = args.profile.as_deref();
    let (files, warnings) = match strata_view::apply(&out, &view, profile) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("-:-: ViewError: {}", e);
            std::process::exit(2);
        }
    };
    for w in &warnings {
        eprintln!("-:-: warning: View: {}", w);
    }

    let outdir = args.output.unwrap_or_else(|| PathBuf::from("."));
    for f in &files {
        let path = outdir.join(&f.path);
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
            && let Err(e) = fs::create_dir_all(parent)
        {
            eprintln!("Failed to create output directory {}: {}", parent.display(), e);
            std::process::exit(1);
        }
        if let Err(e) = write_atomic(&path, &f.yaml) {
            eprintln!("Failed to write output file {}: {}", path.display(), e);
            std::process::exit(1);
        }
    }
    std::process::exit(0);
}

/// `--check`(D33)の dry-run 診断。view 定義の `manifest:` を(view ファイル自身の
/// ディレクトリを基準に)解決して読み込み、`strata_view::check` の結果を
/// 「-:-: 種別: メッセージ」形式で全件 stderr に出す。診断が1件でもあれば exit 1。
fn run_view_check(out: &strata_build::BuildOutput, view: &strata_view::ViewDef, args: &ViewArgs) {
    let Some(manifest_rel) = &view.manifest else {
        eprintln!("-:-: ViewDefError: --check にはビュー定義の `manifest:` 宣言が必要です。");
        std::process::exit(2);
    };
    let manifest_path = args.view.parent().unwrap_or_else(|| Path::new(".")).join(manifest_rel);
    let manifest_src = match fs::read_to_string(&manifest_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read manifest file {}: {}", manifest_path.display(), e);
            std::process::exit(1);
        }
    };
    let manifest = match strata_view::parse_manifest(&manifest_src) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("-:-: ManifestError: {}", e);
            std::process::exit(2);
        }
    };

    let report = strata_view::check(out, view, &manifest);
    for w in &report.warnings {
        eprintln!("-:-: warning: View: {}", w);
    }
    for e in &report.eval_errors {
        eprintln!("-:-: EvalError: {}", e);
    }
    for m in &report.missing_slots {
        eprintln!("-:-: MissingSlot: {}", m);
    }
    for u in &report.unused_nodes {
        eprintln!("-:-: UnusedNode: {}", u);
    }

    std::process::exit(if report.is_clean() { 0 } else { 1 });
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

    // D23: `--hide` 無しでも常に `render_to_typst_with_hide` を経由する(hide が
    // 空なら挙動は従来の `render_to_typst` と完全一致し、warnings も常に空になる)。
    let render_out = match strata_typst::render_to_typst_with_hide(&out.graph, root, fallback_title, &args.hide) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("render error: {}", e);
            std::process::exit(2);
        }
    };

    // D23: 非表示ノードへの Ref を剥がした際の警告。fmt/build の Warning と同じ
    // stderr 出力(exit code には影響しない)。
    for w in &render_out.warnings {
        eprintln!("{}", w);
    }

    match args.output {
        Some(path) => {
            if let Err(e) = write_atomic(&path, &render_out.text) {
                eprintln!("Failed to write output file: {}", e);
                std::process::exit(1);
            }
        }
        None => {
            print!("{}", render_out.text);
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
        E::BadClass { span, msg } => {
            let (line, col) = at(*span, src);
            vec![format!("{}:{}: BadClass: {}", line, col, msg)]
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
