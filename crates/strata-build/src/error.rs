//! `BuildError` — build(M3)が全件収集して返すエラー種別(sml-build-m3-handoff.md D-B1)。
//!
//! 「全か無か」(D13): `build` はパース診断・解決エラーを1件でも検出したら
//! グラフを返さず、収集した全 `BuildError` を `Err` として返す。

use strata_sml::{Diag, Span};

/// build のエラー種別(D-B1)。
#[derive(Debug, Clone, PartialEq)]
pub enum BuildError {
    /// パーサの診断(`strata_sml::Diag`)をそのまま包む。
    Parse(Diag),
    /// ULID 未付与ブロック(fmt 未実行)。`strata fmt` を先に実行するよう案内する。
    MissingId { span: Span },
    /// 参照ターゲット(エイリアス)がファイル内に存在しない。
    UnresolvedAlias { alias: String, span: Span },
    /// 同名エイリアスが複数ブロックに定義されている(全定義箇所のスパンを持つ)。
    DuplicateAlias { alias: String, spans: Vec<Span> },
    /// `::figure` の属性不足・不正(kind 欠落、chart の data-ref/mark/encode 不足等)。
    BadFigure { span: Span, msg: String },
    /// 数式が tex2math でパースできない(`UnknownCommand` 等)。
    Math { span: Span, msg: String },
    /// インライン参照のスキーム(`table:`/`fig:`/`math:`/`cell:`)が対象ノードの実際の
    /// 型と一致しない(sml-build-m3-handoff.md D-B5: 「不一致は `UnresolvedAlias` では
    /// なく `BadFigure` 相当の新エラーでもよい — 裁量。ただし黙認はしない」を受けて
    /// 新設した variant。**D14(2026-07-14、sml-spec.md §1.2)で正式承認**。
    RefTypeMismatch { span: Span, msg: String },
    /// class タグの字句が `[A-Za-z0-9_-]+` に違反する(D23、sml-spec §1.4)。
    /// build の成否は class の有無に依存しない設計だが、字句だけは検証する
    /// (`WP-X1` の実装ハンドオフ: 「class を検証(字句違反は BuildError)」)。
    BadClass { span: Span, msg: String },
    /// 単一ファイル build(`--workspace` 無し)で doc 修飾参照
    /// (`<文書alias>/<ブロックalias>`、D41/D42)に遭遇した(WP-W1.3、
    /// sml-spec §1.10)。黙って落とさず、`--workspace` の必要性を案内する専用エラー。
    CrossDocRef { doc: String, alias: String, span: Span },
    /// ワークスペース build(`--workspace`)で doc 修飾参照の文書 alias 側が
    /// どのメンバーにも見つからない(WP-W2.3: 「doc 修飾の未解決(文書 alias 不明 /
    /// ブロック alias 不明を区別)」の前者)。
    UnknownDocAlias { doc: String, alias: String, span: Span },
    /// ワークスペース build で doc 修飾参照の文書 alias 側は解決できたが、その文書内に
    /// 該当ブロック alias が無い(WP-W2.3 の後者)。
    UnknownBlockAlias { doc: String, alias: String, span: Span },
    /// 単一ファイル build(`--workspace` 無し)で `doc:` 参照(D53、sml-spec §1.14)が
    /// 自文書以外の文書 alias を指している(自文書 alias のみ解決可)。`CrossDocRef` と
    /// 同型だが対象がブロックでなく文書そのものなのでメッセージ文言が違う専用 variant。
    DocRefNeedsWorkspace { alias: String, span: Span },
    /// ワークスペース build で `doc:<alias>` の alias がどのメンバー文書の frontmatter
    /// alias にも一致しない(D53)。
    UnknownDoc { alias: String, span: Span },
    /// build 後の `strata_core::invariants::validate` が検出した違反。正しい実装では
    /// 出ないはずの build 自体のバグ検出網(D-B5)。D-B1 の列挙には無いが、D-B5 が
    /// 「違反があれば BuildError に変換して返す」と明記しているため追加した variant。
    /// **D14(2026-07-14、sml-spec.md §1.2)で正式承認**。ソース位置(`Span`)を
    /// 持たないため、CLI 表示では `-:-` を使う(strata-cli の `format_build_error`)。
    Invariant(strata_core::invariants::Violation),
}
