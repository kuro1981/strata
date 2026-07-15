use clap::{Args as ClapArgs, Parser, Subcommand};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use strata_sml::Span;

/// トップレベル CLI。`fmt` / `build` / `render` サブコマンドを提供する(旧 YAML→HTML
/// フローは M4 の vault 削除(docs/sml-render-m4-handoff.md D-R1)で撤去済み)。
/// `render` は canonical グラフ(層2)から Typst マークアップ(既定)または
/// 素の Markdown/GFM(`--format md`、sml-spec.md §1.8 D38)を直接出力する
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
    /// Render an SML file into Typst markup or plain Markdown/GFM (--format,
    /// D38). Build + render, no intermediate JSON.
    Render(RenderArgs),
    /// Apply a declarative view definition to an SML file's canonical graph,
    /// producing template-consumable data files (YAML). See docs/view-def-v1.md.
    View(ViewArgs),
    /// Serialize an SML file's canonical graph into an AI-readable context view
    /// (ULID-addressable Markdown + semantic edge listing, D36). See
    /// docs/context-m5a-handoff.md.
    Context(ContextArgs),
    /// Build a self-contained static site: graph.json (build output, normalised to a
    /// uniform shape for both single-file and --workspace) + the pre-built graph-UI
    /// SPA (`ui/dist`), combined into one output directory (G1 WS-B, sml-spec.md
    /// §1.13 D49/D50). See docs/graph-ui-g1-handoff.md.
    Site(SiteArgs),
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
    /// SML file to build (omit when using --workspace)
    file: Option<PathBuf>,

    /// Build a workspace instead of a single file: path to a strata.toml
    /// (D41/D43, docs/sml-spec.md §1.10). Mutually exclusive with `file`.
    #[arg(long)]
    workspace: Option<PathBuf>,

    /// Write the graph JSON to this file instead of stdout
    #[arg(short, long)]
    output: Option<PathBuf>,
}

/// `render --format` の選択肢(D19 改定、D38)。既定は `typst`(一次レンダラ)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum RenderFormat {
    Typst,
    Md,
}

#[derive(ClapArgs, Debug)]
struct RenderArgs {
    /// SML file to render (omit when using --workspace)
    file: Option<PathBuf>,

    /// Render a workspace instead of a single file: path to a strata.toml
    /// (D44, sml-spec.md §1.11). Mutually exclusive with `file`. In this mode
    /// `-o` is always an output *directory* (default: current directory) —
    /// every rendered member is written as `<member file stem>.<ext>` inside it.
    #[arg(long)]
    workspace: Option<PathBuf>,

    /// Restrict `--workspace` rendering to a single member (its frontmatter
    /// `alias`, D41). Omit to render every member. Only valid with `--workspace`.
    #[arg(long = "doc")]
    doc: Option<String>,

    /// Write the rendered source to this file instead of stdout (single-file
    /// mode only). The file extension is taken verbatim from what the caller
    /// passes (not inferred from --format). In `--workspace` mode this is
    /// instead the output *directory* (see `--workspace`).
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output format: `typst` (default, primary renderer) or `md` (plain
    /// Markdown / GFM, D38).
    #[arg(long = "format", value_enum, default_value_t = RenderFormat::Typst)]
    format: RenderFormat,

    /// Hide blocks (and their contains subtree) carrying this class (D23).
    /// May be repeated: `--hide note --hide actual-name`.
    #[arg(long = "hide")]
    hide: Vec<String>,
}

#[derive(ClapArgs, Debug)]
struct ViewArgs {
    /// SML file to build and apply the view definition to (omit when using
    /// --workspace)
    file: Option<PathBuf>,

    /// Apply the view to a workspace instead of a single file: path to a
    /// strata.toml (WP-W3, D43). Mutually exclusive with `file`.
    #[arg(long)]
    workspace: Option<PathBuf>,

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

#[derive(ClapArgs, Debug)]
struct ContextArgs {
    /// SML file to build and serialize into a context view (omit when using
    /// --workspace)
    file: Option<PathBuf>,

    /// Serialize a workspace instead of a single file: path to a strata.toml
    /// (D44, sml-spec.md §1.11). Mutually exclusive with `file`.
    #[arg(long)]
    workspace: Option<PathBuf>,

    /// Restrict the (no --node/--class) full-document scope to a single
    /// workspace member (its frontmatter `alias`, D41). Omit to concatenate
    /// every member. Only valid with `--workspace`.
    #[arg(long = "doc")]
    doc: Option<String>,

    /// Write the context Markdown to this file instead of stdout
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Restrict the chunk to this node's `contains` subtree (alias or ULID). May be
    /// repeated. Combined with `--class`, only class-matching blocks within the
    /// requested subtree(s) are listed (AND, D36 scope 2+3 の併用は裁量).
    #[arg(long = "node")]
    node: Vec<String>,

    /// Semantic-edge hop count for the `--node` neighbor summary (D36 既定 1).
    #[arg(long, default_value_t = 1)]
    hops: u32,

    /// Cross-document class listing (D23 class tag, e.g. `note`).
    #[arg(long)]
    class: Option<String>,
}

#[derive(ClapArgs, Debug)]
struct SiteArgs {
    /// SML file to build (omit when using --workspace)
    file: Option<PathBuf>,

    /// Build a workspace instead of a single file: path to a strata.toml.
    /// Mutually exclusive with `file`.
    #[arg(long)]
    workspace: Option<PathBuf>,

    /// Output directory. `graph.json` and the built UI assets (`ui/dist`) are
    /// written here together (self-contained: open `<output>/index.html` directly
    /// or serve the directory — no server-side logic required, D50).
    #[arg(short, long)]
    output: PathBuf,

    /// Override the built UI assets directory to copy from (default:
    /// `<repo root>/ui/dist`, resolved relative to this crate at compile time —
    /// see docs/graph-ui-g1-handoff.md WS-B). Mainly useful for testing against a
    /// UI build that lives elsewhere.
    #[arg(long = "ui-dist")]
    ui_dist: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Fmt(fmt_args) => run_fmt(fmt_args),
        Command::Build(build_args) => run_build(build_args),
        Command::Render(render_args) => run_render(render_args),
        Command::View(view_args) => run_view(view_args),
        Command::Context(context_args) => run_context(context_args),
        Command::Site(site_args) => run_site(site_args),
    }
}

/// `strata-cli context` サブコマンド(M5-A、D36、docs/context-m5a-handoff.md WP-A2)。
///
/// 内部で `strata_build::build` → `strata_context::render_context` を直結する。
/// exit code は他コマンドと同じ 0/1/2 の慣習: 2 = SML を読めない・パースできない、
/// または `--node` の alias/ULID が解決できない・`--class` 単独/無指定なのに
/// フロントマターが無い(全文書スコープの前提が崩れる)、1 = 書き込み失敗、
/// 0 = 成功。Warning は他コマンドと同じく stderr に出しつつ exit 0 を妨げない。
fn run_context(args: ContextArgs) {
    match resolve_input_mode(args.file.clone(), args.workspace.clone()) {
        InputMode::Workspace(members) => run_context_workspace(members, args),
        InputMode::Single(file) => run_context_single(file, args),
    }
}

fn run_context_single(file: PathBuf, args: ContextArgs) {
    if args.doc.is_some() {
        eprintln!("--doc は --workspace と併用してください。");
        std::process::exit(2);
    }

    let src = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read input file: {}", e);
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

    let opts = strata_context::ContextOptions { nodes: args.node, hops: args.hops, class: args.class };
    let text = match strata_context::render_context(&out, &opts) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("-:-: ContextError: {}", e);
            std::process::exit(2);
        }
    };

    write_context_output(&text, args.output);
}

/// `strata-cli context --workspace`(WP-Z1、D44)。`strata_build::build_workspace` →
/// `strata_context::render_context_workspace` を直結する(単一文書版と同じ
/// exit code の慣習)。
fn run_context_workspace(members: Vec<strata_build::Member>, args: ContextArgs) {
    let sources: BTreeMap<String, String> =
        members.iter().map(|m| (m.path.clone(), m.src.clone())).collect();

    let ws = match strata_build::build_workspace(&members) {
        Err(errors) => {
            for e in &errors {
                for line in format_workspace_error(e, &sources) {
                    eprintln!("{}", line);
                }
            }
            std::process::exit(2);
        }
        Ok(ws) => ws,
    };
    for w in &ws.warnings {
        let (line, col) = w.diag.span.line_col(sources.get(&w.path).map(String::as_str).unwrap_or(""));
        eprintln!("{}:{}:{}: warning: {:?}: {}", w.path, line, col, w.diag.kind, w.diag.msg);
    }

    let opts = strata_context::ContextOptions { nodes: args.node, hops: args.hops, class: args.class };
    let text = match strata_context::render_context_workspace(&ws, &opts, args.doc.as_deref()) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("-:-: ContextError: {}", e);
            std::process::exit(2);
        }
    };

    write_context_output(&text, args.output);
}

fn write_context_output(text: &str, output: Option<PathBuf>) -> ! {
    match output {
        Some(path) => {
            if let Err(e) = write_atomic(&path, text) {
                eprintln!("Failed to write output file: {}", e);
                std::process::exit(1);
            }
        }
        None => {
            print!("{}", text);
        }
    }
    std::process::exit(0);
}

/// `strata-cli view` サブコマンド(D30/D33、docs/view-v1-handoff.md WP-W2、
/// `--workspace` は M7 WP-W3)。
///
/// 内部で `strata_build::build`(または `--workspace` なら `build_workspace`)→
/// `strata_view::apply`/`apply_workspace`(または `--check` なら
/// `check`/`check_workspace`)を呼ぶ。exit code は他コマンドと同じ 0/1/2 の慣習:
/// 2 = SML/ビュー定義/マニフェストを読めない・パースできない(全か無か)、
/// 1 = 書き込み失敗、または `--check` で診断あり、0 = 成功/診断なし。
/// Warning は stderr に出しつつ exit 0(D17 と同じ扱い)。
fn run_view(args: ViewArgs) {
    let view_src = match fs::read_to_string(&args.view) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read view definition file: {}", e);
            std::process::exit(1);
        }
    };
    let view = match strata_view::parse_view_def(&view_src) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("-:-: ViewDefError: {}", e);
            std::process::exit(2);
        }
    };

    match resolve_input_mode(args.file.clone(), args.workspace.clone()) {
        InputMode::Workspace(members) => run_view_workspace(members, view, &args),
        InputMode::Single(file) => run_view_single(file, view, &args),
    }
}

fn run_view_single(file: PathBuf, view: strata_view::ViewDef, args: &ViewArgs) {
    let src = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read input file: {}", e);
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

    if args.check {
        run_view_check(&out, &view, args);
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
    write_view_outputs(&files, args.output.clone());
}

/// `strata view --workspace`(WP-W3.1)。`strata build --workspace` と同じ
/// エラー表示(ファイル名付き)を再利用する。
fn run_view_workspace(members: Vec<strata_build::Member>, view: strata_view::ViewDef, args: &ViewArgs) {
    let sources: BTreeMap<String, String> =
        members.iter().map(|m| (m.path.clone(), m.src.clone())).collect();

    let out = match strata_build::build_workspace(&members) {
        Err(errors) => {
            for e in &errors {
                for line in format_workspace_error(e, &sources) {
                    eprintln!("{}", line);
                }
            }
            std::process::exit(2);
        }
        Ok(out) => out,
    };
    for w in &out.warnings {
        let (line, col) = w.diag.span.line_col(sources.get(&w.path).map(String::as_str).unwrap_or(""));
        eprintln!("{}:{}:{}: warning: {:?}: {}", w.path, line, col, w.diag.kind, w.diag.msg);
    }

    if args.check {
        run_view_check_workspace(&out, &view, args);
        return;
    }

    let profile = args.profile.as_deref();
    let (files, warnings) = match strata_view::apply_workspace(&out, &view, profile) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("-:-: ViewError: {}", e);
            std::process::exit(2);
        }
    };
    for w in &warnings {
        eprintln!("-:-: warning: View: {}", w);
    }
    write_view_outputs(&files, args.output.clone());
}

fn write_view_outputs(files: &[strata_view::OutputFile], output: Option<PathBuf>) {
    let outdir = output.unwrap_or_else(|| PathBuf::from("."));
    for f in files {
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
    let manifest = load_manifest_for_check(view, args);
    let report = strata_view::check(out, view, &manifest);
    report_check_and_exit(&report);
}

/// `run_view_check` のワークスペース版(WP-W3.1)。
fn run_view_check_workspace(out: &strata_build::WorkspaceBuildOutput, view: &strata_view::ViewDef, args: &ViewArgs) {
    let manifest = load_manifest_for_check(view, args);
    let report = strata_view::check_workspace(out, view, &manifest);
    report_check_and_exit(&report);
}

fn load_manifest_for_check(view: &strata_view::ViewDef, args: &ViewArgs) -> strata_view::Manifest {
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
    match strata_view::parse_manifest(&manifest_src) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("-:-: ManifestError: {}", e);
            std::process::exit(2);
        }
    }
}

fn report_check_and_exit(report: &strata_view::CheckReport) -> ! {
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

/// `strata.toml` の `[workspace]` テーブル(WP-W2.1)。TOML クレートの選定は裁量
/// (`toml` — serde 経由でこの最小 struct にそのまま写せるため、素朴な TOML には
/// これで十分。裁量、最終報告参照)。
#[derive(serde::Deserialize)]
struct StrataToml {
    workspace: WorkspaceToml,
}

#[derive(serde::Deserialize)]
struct WorkspaceToml {
    /// strata.toml からの相対パス。グロブ可(`glob` クレート、WP-W2.1)。
    members: Vec<String>,
}

/// `strata.toml` を読み、`members` を展開して全メンバーのソースを読み込む
/// (WP-W2.1)。グロブ文字(`*`/`?`/`[`)を含まないエントリはグロブを経由せず直接
/// 読む(存在しなければ明確な I/O エラーになる — グロブに通すと0件マッチが
/// 「黙って空」になりかねないため、D40 の教訓どおり黙って落とさない)。
/// `Member::path` は診断表示用の相対パス文字列(strata.toml のディレクトリ基準)。
fn load_workspace_members(toml_path: &Path) -> Result<Vec<strata_build::Member>, String> {
    let toml_src = fs::read_to_string(toml_path)
        .map_err(|e| format!("Failed to read {}: {}", toml_path.display(), e))?;
    let parsed: StrataToml =
        toml::from_str(&toml_src).map_err(|e| format!("Failed to parse {}: {}", toml_path.display(), e))?;
    let base = toml_path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));

    let mut resolved: Vec<PathBuf> = Vec::new();
    for pattern in &parsed.workspace.members {
        if pattern.contains(['*', '?', '[']) {
            let full_pattern = base.join(pattern);
            let matches_iter = glob::glob(&full_pattern.to_string_lossy())
                .map_err(|e| format!("strata.toml の members グロブ '{pattern}' が不正です: {e}"))?;
            let mut matches: Vec<PathBuf> = matches_iter.filter_map(Result::ok).collect();
            if matches.is_empty() {
                return Err(format!("strata.toml の members グロブ '{pattern}' に一致するファイルがありません"));
            }
            matches.sort();
            resolved.extend(matches);
        } else {
            resolved.push(base.join(pattern));
        }
    }
    resolved.sort();
    resolved.dedup();

    let mut members = Vec::with_capacity(resolved.len());
    for path in resolved {
        let src =
            fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        let display_path = path.strip_prefix(base).unwrap_or(&path).to_string_lossy().into_owned();
        members.push(strata_build::Member { path: display_path, src });
    }
    Ok(members)
}

/// `BuildArgs`/`ViewArgs` 共通: `file`/`--workspace` の組み合わせを検証し、
/// ワークスペースモードなら展開済みメンバー列、単一ファイルモードならそのファイルの
/// パスを返す。両方指定・両方省略はエラー(exit 2)。
enum InputMode {
    Single(PathBuf),
    Workspace(Vec<strata_build::Member>),
}

fn resolve_input_mode(file: Option<PathBuf>, workspace: Option<PathBuf>) -> InputMode {
    match (file, workspace) {
        (Some(_), Some(_)) => {
            eprintln!("file と --workspace は同時に指定できません。");
            std::process::exit(2);
        }
        (None, None) => {
            eprintln!("file か --workspace のいずれかが必要です。");
            std::process::exit(2);
        }
        (Some(f), None) => InputMode::Single(f),
        (None, Some(w)) => match load_workspace_members(&w) {
            Ok(members) => InputMode::Workspace(members),
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(2);
            }
        },
    }
}

/// `strata-cli build` サブコマンド(docs/sml-build-m3-handoff.md D-B6、
/// `--workspace` は WP-W2.2)。
fn run_build(args: BuildArgs) {
    if let InputMode::Workspace(members) = resolve_input_mode(args.file.clone(), args.workspace.clone()) {
        run_build_workspace(members, args.output);
        return;
    }
    let file = args.file.expect("resolve_input_mode returned Single only when file is Some");

    let src = match fs::read_to_string(&file) {
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

/// `strata-cli build --workspace` サブコマンド(WP-W2.2)。全メンバーを一括パースし、
/// 横断参照を解決した単一の統合グラフ(`WorkspaceBuildOutput`)を出力する。
fn run_build_workspace(members: Vec<strata_build::Member>, output: Option<PathBuf>) {
    let sources: BTreeMap<String, String> =
        members.iter().map(|m| (m.path.clone(), m.src.clone())).collect();

    match strata_build::build_workspace(&members) {
        Err(errors) => {
            for e in &errors {
                for line in format_workspace_error(e, &sources) {
                    eprintln!("{}", line);
                }
            }
            std::process::exit(2);
        }
        Ok(out) => {
            for w in &out.warnings {
                let (line, col) = w.diag.span.line_col(sources.get(&w.path).map(String::as_str).unwrap_or(""));
                eprintln!("{}:{}:{}: warning: {:?}: {}", w.path, line, col, w.diag.kind, w.diag.msg);
            }

            let json = match serde_json::to_string_pretty(&out) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("Failed to serialize workspace build output: {}", e);
                    std::process::exit(1);
                }
            };

            match output {
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

/// `WorkspaceError` 1件を「ファイル:行:列: 種別: メッセージ」形式の行(複数になりうる)
/// に変換する(WP-W2.3: 診断にファイル名を含める。裁量: 「行:列:」の前に「ファイル:」を
/// 前置する体裁を選んだ)。`Member` はファイル固有の `BuildError` を
/// `format_build_error` に委譲しつつ先頭にファイル名を足すだけ。`DuplicateDocAlias`/
/// `UlidCollision` はソース位置を持たないため、関係する全ファイルぶん「ファイル:-:-:」
/// の行を1つずつ出す。
fn format_workspace_error(e: &strata_build::WorkspaceError, sources: &BTreeMap<String, String>) -> Vec<String> {
    use strata_build::WorkspaceError as WE;
    match e {
        WE::DuplicateDocAlias { alias, paths } => paths
            .iter()
            .map(|p| {
                format!(
                    "{p}:-:-: DuplicateDocAlias: 文書 alias '{alias}' が複数ファイルで宣言されています({})",
                    paths.join(", ")
                )
            })
            .collect(),
        WE::UlidCollision { id, paths } => paths
            .iter()
            .map(|p| {
                format!(
                    "{p}:-:-: UlidCollision: ID '{}' が複数ファイルで宣言されています({})",
                    id.0,
                    paths.join(", ")
                )
            })
            .collect(),
        WE::Member { path, error } => {
            let label = if path.is_empty() { "(workspace)" } else { path.as_str() };
            let src = sources.get(path).map(String::as_str).unwrap_or("");
            format_build_error(error, src).into_iter().map(|line| format!("{label}:{line}")).collect()
        }
    }
}

/// `strata-cli render` サブコマンド(docs/sml-render-m4-handoff.md D-R3、
/// `--format md` は sml-spec.md §1.8 D38・docs/md-render-handoff.md WP-M3)。
///
/// 内部で `strata_build::build` → `strata_typst::render_to_typst_with_hide` /
/// `strata_md::render_to_md_with_hide` のいずれかを直結する(中間 JSON なし)。
/// exit code: 成功 0 / 読み書き失敗 1 / BuildError・render エラー 2。
/// `root: None`(フロントマター無し)は D21 の案内文言で exit 2(両フォーマット共通)。
fn run_render(args: RenderArgs) {
    match resolve_input_mode(args.file.clone(), args.workspace.clone()) {
        InputMode::Workspace(members) => run_render_workspace(members, args),
        InputMode::Single(file) => run_render_single(file, args),
    }
}

fn run_render_single(file: PathBuf, args: RenderArgs) {
    if args.doc.is_some() {
        eprintln!("--doc は --workspace と併用してください。");
        std::process::exit(2);
    }

    let src = match fs::read_to_string(&file) {
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
    // Document.title も最初の H1 も無い場合にのみ使われる。`--format md` は現状
    // これを本文に出さない — strata_md::render_to_md_with_hide のドキュメント参照)。
    let fallback_title = file.file_stem().and_then(|s| s.to_str()).unwrap_or("untitled");

    // D23: `--hide` 無しでも常に `*_with_hide` を経由する(hide が空なら挙動は
    // 従来の `render_to_typst`/`render_to_md` と完全一致し、warnings も常に空になる)。
    // 両フォーマットの `RenderOutput`(strata_typst / strata_md)は独立した型だが
    // 形(text/warnings)が同じなので、ここで `(String, Vec<String>)` に揃えて以降を
    // フォーマット非依存の共通コードにする。
    let (text, warnings): (String, Vec<String>) = match args.format {
        RenderFormat::Typst => match strata_typst::render_to_typst_with_hide(&out.graph, root, fallback_title, &args.hide) {
            Ok(o) => (o.text, o.warnings),
            Err(e) => {
                eprintln!("render error: {}", e);
                std::process::exit(2);
            }
        },
        RenderFormat::Md => match strata_md::render_to_md_with_hide(&out.graph, root, fallback_title, &args.hide) {
            Ok(o) => (o.text, o.warnings),
            Err(e) => {
                eprintln!("render error: {}", e);
                std::process::exit(2);
            }
        },
    };

    // D23: 非表示ノードへの Ref を剥がした際の警告。fmt/build の Warning と同じ
    // stderr 出力(exit code には影響しない)。
    for w in &warnings {
        eprintln!("{}", w);
    }

    match args.output {
        Some(path) => {
            if let Err(e) = write_atomic(&path, &text) {
                eprintln!("Failed to write output file: {}", e);
                std::process::exit(1);
            }
        }
        None => {
            print!("{}", text);
        }
    }
    std::process::exit(0);
}

/// `strata-cli render --workspace`(WP-Z1、D44)。cross-doc 参照を含む文書の
/// 再生成不能(M7 の既知副作用)を解消する。`--doc` 省略時は全メンバーを、指定時は
/// 当該文書のみを出力する。出力は常に `-o <outdir>`(既定カレントディレクトリ)配下に
/// `<メンバーのファイル名 stem>.<拡張子>` として書く。
fn run_render_workspace(members: Vec<strata_build::Member>, args: RenderArgs) {
    let sources: BTreeMap<String, String> =
        members.iter().map(|m| (m.path.clone(), m.src.clone())).collect();

    let ws = match strata_build::build_workspace(&members) {
        Err(errors) => {
            for e in &errors {
                for line in format_workspace_error(e, &sources) {
                    eprintln!("{}", line);
                }
            }
            std::process::exit(2);
        }
        Ok(ws) => ws,
    };
    for w in &ws.warnings {
        let (line, col) = w.diag.span.line_col(sources.get(&w.path).map(String::as_str).unwrap_or(""));
        eprintln!("{}:{}:{}: warning: {:?}: {}", w.path, line, col, w.diag.kind, w.diag.msg);
    }

    let selected: Vec<&strata_build::DocRoot> = match &args.doc {
        Some(alias) => match ws.roots.iter().find(|r| r.alias.as_deref() == Some(alias.as_str())) {
            Some(r) => vec![r],
            None => {
                eprintln!("指定された文書 alias '{}' がワークスペースに見つかりません。", alias);
                std::process::exit(2);
            }
        },
        None => ws.roots.iter().collect(),
    };

    // D44: cross-doc `Ref` の描画に使うワークスペース全体の付帯情報。
    // - doc_of: ノード → 所属文書(`DocRoot::path`)
    // - doc_titles: 文書 → 退化テキストに添える表示名(Document.title、無ければファイル名 stem)
    // - doc_stems (md のみ): 文書 → 相対リンクに使う出力ファイル名 stem
    let doc_of = strata_build::doc_ownership(&ws.graph, &ws.roots);
    let doc_titles: HashMap<String, String> = ws
        .roots
        .iter()
        .map(|r| {
            let title = r
                .root
                .and_then(|root| ws.graph.nodes.get(&root))
                .and_then(|n| match &n.payload {
                    strata_core::NodePayload::Document(d) => d.title.clone(),
                    _ => None,
                })
                .unwrap_or_else(|| file_stem(&r.path));
            (r.path.clone(), title)
        })
        .collect();
    let doc_stems: HashMap<String, String> = ws.roots.iter().map(|r| (r.path.clone(), file_stem(&r.path))).collect();
    let cross_anchors = if matches!(args.format, RenderFormat::Md) {
        let doc_root_ids: Vec<strata_core::NodeId> = ws.roots.iter().filter_map(|r| r.root).collect();
        strata_md::compute_workspace_anchors(&ws.graph, &doc_root_ids)
    } else {
        HashMap::new()
    };

    let outdir = args.output.clone().unwrap_or_else(|| PathBuf::from("."));
    if let Err(e) = fs::create_dir_all(&outdir) {
        eprintln!("Failed to create output directory {}: {}", outdir.display(), e);
        std::process::exit(1);
    }

    for r in selected {
        let Some(root) = r.root else {
            eprintln!("{}: フロントマターがありません。`strata fmt` を先に実行してください。", r.path);
            std::process::exit(2);
        };
        let stem = file_stem(&r.path);

        let (text, warnings): (String, Vec<String>) = match args.format {
            RenderFormat::Typst => {
                let ctx = strata_typst::WorkspaceRenderCtx {
                    doc_of: &doc_of,
                    doc_titles: &doc_titles,
                    current_doc: &r.path,
                };
                match strata_typst::render_to_typst_workspace_with_hide(&ws.graph, root, &stem, &args.hide, Some(ctx)) {
                    Ok(o) => (o.text, o.warnings),
                    Err(e) => {
                        eprintln!("render error ({}): {}", r.path, e);
                        std::process::exit(2);
                    }
                }
            }
            RenderFormat::Md => {
                let ctx = strata_md::WorkspaceRenderCtx {
                    doc_of: &doc_of,
                    doc_stems: &doc_stems,
                    doc_titles: &doc_titles,
                    current_doc: &r.path,
                    cross_anchors: &cross_anchors,
                };
                match strata_md::render_to_md_workspace_with_hide(&ws.graph, root, &stem, &args.hide, Some(ctx)) {
                    Ok(o) => (o.text, o.warnings),
                    Err(e) => {
                        eprintln!("render error ({}): {}", r.path, e);
                        std::process::exit(2);
                    }
                }
            }
        };
        for w in &warnings {
            eprintln!("{}", w);
        }

        let ext = match args.format {
            RenderFormat::Typst => "typ",
            RenderFormat::Md => "md",
        };
        let out_path = outdir.join(format!("{stem}.{ext}"));
        if let Err(e) = write_atomic(&out_path, &text) {
            eprintln!("Failed to write output file {}: {}", out_path.display(), e);
            std::process::exit(1);
        }
    }
    std::process::exit(0);
}

/// メンバーの `DocRoot::path`(strata.toml からの相対パス)からファイル名 stem を
/// 取り出す(D44: workspace render の出力ファイル名・退化テキストのフォールバック名)。
fn file_stem(path: &str) -> String {
    Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or(path).to_string()
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
        // WP-W1.3: 単一ファイル build で doc 修飾参照(`<文書alias>/<ブロックalias>`)に
        // 遭遇した。黙って落とさず `--workspace` の必要性を案内する(D42)。
        E::CrossDocRef { doc, alias, span } => {
            let (line, col) = at(*span, src);
            vec![format!(
                "{}:{}: CrossDocRef: 参照 '{}/{}' はワークスペース対応コマンド(`strata build --workspace <strata.toml>` または `strata render --workspace <strata.toml>`)が必要です。",
                line, col, doc, alias
            )]
        }
        // WP-W2.3: ワークスペース build 中の doc 修飾参照の未解決(文書 alias 側)。
        E::UnknownDocAlias { doc, alias, span } => {
            let (line, col) = at(*span, src);
            vec![format!(
                "{}:{}: UnknownDocAlias: 参照 '{}/{}' — 文書 alias '{}' を持つメンバーがワークスペースにありません。",
                line, col, doc, alias, doc
            )]
        }
        // WP-W2.3: ワークスペース build 中の doc 修飾参照の未解決(ブロック alias 側)。
        E::UnknownBlockAlias { doc, alias, span } => {
            let (line, col) = at(*span, src);
            vec![format!(
                "{}:{}: UnknownBlockAlias: 参照 '{}/{}' — 文書 '{}' にブロック alias '{}' がありません。",
                line, col, doc, alias, doc, alias
            )]
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

// --- `strata site`(G1 WS-B、docs/graph-ui-g1-handoff.md、sml-spec.md §1.13 D49/D50) ---

/// `strata site` が書き出す graph.json のトップレベル形。単一ファイル/ワークスペース
/// どちらの build 結果でも同じ形(`graph`/`roots`/`doc_aliases`)に正規化する(裁量:
/// `strata_build::BuildOutput` と `WorkspaceBuildOutput` は形が違う — WP-W2.4 の
/// `DocRoot` 相当を単一ファイルでも合成することで、UI 側(ui/src/types/graph.ts)は
/// 常に1つの TypeScript 型だけを相手にできる)。`warnings` は含めない(CLI が既に
/// stderr に出している。graph.json は UI 用の静的資産に徹する)。
#[derive(serde::Serialize)]
struct SiteGraphJson {
    graph: strata_core::Graph,
    roots: Vec<SiteRoot>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    doc_aliases: BTreeMap<String, BTreeMap<String, strata_core::NodeId>>,
}

/// `strata_build::workspace::DocRoot` と同じ形(`path`/`alias`/`root`)の裁量による
/// 単純写し。単一ファイル build 用にも同じ形で合成する。
#[derive(serde::Serialize)]
struct SiteRoot {
    path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    alias: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    root: Option<strata_core::NodeId>,
}

/// `strata site` サブコマンド。内部で `strata_build::build`(または `--workspace` なら
/// `build_workspace`)→ `graph.json` 書き出し → ビルド済み UI 資産(`ui/dist`)を
/// 出力ディレクトリへ合成する。exit code: 0 成功 / 1 I/O 失敗 / 2 SML を読めない・
/// パースできない、または UI 資産(`ui/dist`)が無い(前提条件エラーとして他の
/// usage エラーと同じ 2 に揃えた。裁量)。
fn run_site(args: SiteArgs) {
    let ui_dist = args.ui_dist.clone().unwrap_or_else(default_ui_dist);
    if !ui_dist.is_dir() {
        eprintln!(
            "UI 資産が見つかりません: {}\n`ui/` で `pnpm install && pnpm build` を先に実行してください(docs/graph-ui-g1-handoff.md WS-B)。",
            ui_dist.display()
        );
        std::process::exit(2);
    }

    let site_graph = match resolve_input_mode(args.file.clone(), args.workspace.clone()) {
        InputMode::Workspace(members) => build_site_graph_workspace(members),
        InputMode::Single(file) => build_site_graph_single(file),
    };

    if let Err(e) = fs::create_dir_all(&args.output) {
        eprintln!("Failed to create output directory {}: {}", args.output.display(), e);
        std::process::exit(1);
    }

    if let Err(e) = copy_dir_recursive(&ui_dist, &args.output) {
        eprintln!("Failed to copy UI assets from {} to {}: {}", ui_dist.display(), args.output.display(), e);
        std::process::exit(1);
    }

    let json = match serde_json::to_string_pretty(&site_graph) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("Failed to serialize graph JSON: {}", e);
            std::process::exit(1);
        }
    };
    if let Err(e) = write_atomic(&args.output.join("graph.json"), &json) {
        eprintln!("Failed to write graph.json: {}", e);
        std::process::exit(1);
    }

    std::process::exit(0);
}

/// 単一ファイル build から `SiteGraphJson` を合成する。文書 alias
/// (`SiteRoot::alias`)は root ノード自身の解決済み alias(D26)をそのまま使う —
/// ワークスペース側の `DocRoot::alias`(フロントマターの alias)と同じ意味になる。
fn build_site_graph_single(file: PathBuf) -> SiteGraphJson {
    let src = match fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read input file: {}", e);
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

    let alias = out.root.and_then(|id| out.graph.nodes.get(&id)).and_then(|n| n.alias.clone());
    let path = file.to_string_lossy().into_owned();

    SiteGraphJson {
        graph: out.graph,
        roots: vec![SiteRoot { path, alias, root: out.root }],
        doc_aliases: BTreeMap::new(),
    }
}

/// ワークスペース build から `SiteGraphJson` を合成する(`WorkspaceBuildOutput` を
/// ほぼそのまま写すだけ — `warnings` を落とす点のみが違う)。
fn build_site_graph_workspace(members: Vec<strata_build::Member>) -> SiteGraphJson {
    let sources: BTreeMap<String, String> = members.iter().map(|m| (m.path.clone(), m.src.clone())).collect();

    let out = match strata_build::build_workspace(&members) {
        Err(errors) => {
            for e in &errors {
                for line in format_workspace_error(e, &sources) {
                    eprintln!("{}", line);
                }
            }
            std::process::exit(2);
        }
        Ok(out) => out,
    };
    for w in &out.warnings {
        let (line, col) = w.diag.span.line_col(sources.get(&w.path).map(String::as_str).unwrap_or(""));
        eprintln!("{}:{}:{}: warning: {:?}: {}", w.path, line, col, w.diag.kind, w.diag.msg);
    }

    SiteGraphJson {
        graph: out.graph,
        roots: out.roots.into_iter().map(|r| SiteRoot { path: r.path, alias: r.alias, root: r.root }).collect(),
        doc_aliases: out.doc_aliases,
    }
}

/// `ui/dist` の既定位置: このクレート(`crates/strata-cli`)からリポジトリルート経由で
/// `ui/dist` へ(裁量: コンパイル時の `CARGO_MANIFEST_DIR` に焼き込む。このモノレポの
/// 開発用途では常に正しく解決できる一方、切り離して配布するバイナリでは通用しない
/// — 既知の制限として最終報告に明記、G2 で作り直す前提)。
fn default_ui_dist() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("ui").join("dist")
}

/// `ui/dist` の中身を出力ディレクトリへ再帰コピーする(シンボリックリンクは
/// vite のビルド成果物に現れない想定なので扱わない)。
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            fs::create_dir_all(&target)?;
            copy_dir_recursive(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
