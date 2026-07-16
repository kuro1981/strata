// ノードの短縮ラベル(グラフペインのノード表示・関係パネルのジャンプ先表示に使う)。
// D49: 「ラベルは見出し/先頭テキストの短縮」。
//
// G1.7(表示ラベルの統一): ユーザー目視フィードバック「日本語表記と英語表記(alias)
// 両方でてくるけど同じ場所をしめしてたりするのが奇妙」への対応。方針:
// 1. 主ラベルは常に人間可読テキスト。alias はテキストが空のノードだけのフォールバック
//    (`withFallback` に集約。個別ケースで `n.alias || "..."` を書かない)。
// 2. alias を補助表記として出す場合は常に `hoverTitle`(hover tooltip)か
//    `ALIAS_BADGE_CLASS` の小さな等幅バッジ(選択中ノードの詳細・関係パネルの行に限定)
//    のどちらか一貫した1形式のみ。グラフノードのラベル本体・パンくず・俯瞰の行ラベル
//    には出さない。

import type { GraphIndex } from "./graphIndex";
import type { Inline, SiteRoot, StrataNode } from "@/types/graph";

const MAX_LEN = 28;

/** ラベルの切り詰め(既定 28 文字)。グラフペイン(local グラフ)は更に短い max を
 * 明示的に渡して使う(G1.6 #2: hop2 やノード数が多いリングでの重なり対策)。 */
export function truncate(s: string, max = MAX_LEN): string {
  const trimmed = s.trim().replace(/\s+/g, " ");
  return trimmed.length > max ? trimmed.slice(0, max - 1) + "…" : trimmed;
}

// 全角相当(CJK 統合漢字・かな・全角記号など)とみなす大まかなコードポイント帯域。
// SVG textLength を実測せず(DOM 計測なしで同心円半径を先に決めたい)、文字種から
// 概算するヒューリスティック(G1.6 #2)。
const WIDE_RANGES: [number, number][] = [
  [0x1100, 0x115f], // ハングル字母
  [0x2e80, 0xa4cf], // CJK 部首・かな・ハングル互換字母・CJK 統合漢字 等
  [0xac00, 0xd7a3], // ハングル音節
  [0xf900, 0xfaff], // CJK 互換漢字
  [0xff00, 0xff60], // 全角英数・記号
  [0xffe0, 0xffe6],
  [0x20000, 0x3fffd], // CJK 拡張
];

function isWideChar(code: number): boolean {
  return WIDE_RANGES.some(([lo, hi]) => code >= lo && code <= hi);
}

/** 概算の描画幅(px)。全角文字は fontSize、半角は fontSize*0.56 として加算する。
 * ローカルグラフの同心円半径の動的算出・ラベル背景プレートの幅に使う(G1.6 #2)。 */
export function estimateLabelWidth(s: string, fontSize = 11): number {
  let w = 0;
  for (const ch of s) {
    w += isWideChar(ch.codePointAt(0) ?? 0) ? fontSize : fontSize * 0.56;
  }
  return w;
}

/** インライン列から表示用プレーンテキストを合成する(ネストした強調・数式・参照も
 * 表示テキストだけ拾う。ロスレス原則は payload 側の話であって、ここは要約表示)。 */
export function inlineToText(inline: Inline[] | undefined): string {
  if (!inline) return "";
  return inline.map(inlineNodeToText).join("");
}

function inlineNodeToText(i: Inline): string {
  switch (i.t) {
    case "text":
      return i.s;
    case "emph":
      return inlineToText(i.children);
    case "math":
      return "[math]";
    case "ref":
      return i.text || "[ref]";
    case "term":
      return i.text || "[term]";
    case "anchor":
      return "";
    case "link":
      return i.text || i.url;
    case "image":
      return i.alt || "[image]";
    default:
      return "";
  }
}

const TYPE_LABEL: Record<string, string> = {
  document: "文書",
  section: "見出し",
  para: "段落",
  list: "リスト",
  record: "record",
  table: "表",
  math: "数式",
  code: "コード",
  term: "用語",
  anchor: "anchor",
  value: "値",
  quote: "引用",
  thematic_break: "水平線",
  figure: "図",
};

/** ノード種別の日本語表示名(未知 type はそのまま type 文字列を出す)。 */
export function typeLabel(type: string): string {
  return TYPE_LABEL[type] ?? type;
}

/** ULID 短縮(alias も無い、正真正銘テキストが空のノード向け最終フォールバック)。
 * 先頭側(タイムスタンプ相当)より末尾側(ランダム性が高い)の方が同時期に作られた
 * ノード同士を見分けやすいので末尾6文字を使う。 */
function shortId(id: string): string {
  return id.length > 6 ? id.slice(-6) : id;
}

/** 表示テキストが得られない場合の統一フォールバック順: 明示テキスト → alias →
 * ULID 短縮 → 型ごとの最終プレースホルダ。alias を「主ラベルとして使ってよい」
 * 唯一の場面(テキストが空のとき)をここに集約する(G1.7 方針1)。 */
function withFallback(text: string | undefined, n: StrataNode, placeholder: string): string {
  const t = text?.trim();
  if (t) return t;
  if (n.alias) return n.alias;
  return shortId(n.id) || placeholder;
}

/** document ノードの表示名(title → alias → ULID 短縮のフォールバック、G1.7)。
 * `deriveLabel` と違って切り詰めない(文書ペインの `<h1>` は本文の主見出しなので
 * グラフペインのラベル用の MAX_LEN=28 で削るべきではない。document ラベルの
 * フォールバック規則そのものは1箇所(ここ)に持ち、`deriveLabel` はこれを
 * truncate して使う)。 */
export function documentTitle(n: StrataNode): string {
  return withFallback((n as { title?: string }).title, n, "(文書)");
}

/** グラフペイン・関係パネルで使う短縮ラベル。 */
export function deriveLabel(n: StrataNode): string {
  switch (n.type) {
    case "document":
      return truncate(documentTitle(n));
    case "section":
      return truncate(withFallback(inlineToText((n as { heading?: Inline[] }).heading), n, "(見出し)"));
    case "para":
      return truncate(withFallback(inlineToText((n as { inline?: Inline[] }).inline), n, "(空の段落)"));
    case "list":
      // リスト自体に見出しテキストは無いが、この合成文字列は常に非空なので
      // withFallback の対象外(alias に置き換わることはない、が値自体は持っていれば
      // 別途 alias バッジ側で見える)。
      return `リスト${(n as { ordered?: boolean }).ordered ? "(順序)" : ""}`;
    case "record":
      return truncate(withFallback(undefined, n, "record"));
    case "table":
      return truncate(withFallback(inlineToText((n as { caption?: Inline[] }).caption), n, "表"));
    case "math":
      return truncate(withFallback(undefined, n, "数式"));
    case "code": {
      // lang はノード固有の識別子ではない(同じ言語の別スニペットと同一になる)ので
      // alias の後段・shortId の前段に置く(裁量: 汎用の3段チェーンに lang を挟んだ
      // code 専用チェーン)。
      const lang = (n as { lang?: string }).lang;
      return truncate(n.alias || lang || shortId(n.id) || "コード");
    }
    case "term":
      return truncate(withFallback((n as { name?: string }).name, n, "用語"));
    case "anchor":
      return truncate(withFallback(inlineToText((n as { inline?: Inline[] }).inline), n, "anchor"));
    case "value": {
      const v = (n as { scalar?: unknown }).scalar;
      return truncate(String(v ?? ""));
    }
    case "quote":
      return "引用";
    case "thematic_break":
      return "───";
    case "figure":
      return truncate(withFallback(inlineToText((n as { caption?: Inline[] }).caption), n, "図"));
    default:
      return truncate(withFallback(undefined, n, typeLabel(n.type)));
  }
}

/** alias 補助バッジの共通スタイル(G1.7 方針2: 小さな等幅バッジを1形式に統一。
 * ノード詳細(選択時)・関係パネルの行に限定して使う)。 */
export const ALIAS_BADGE_CLASS = "font-mono text-[10px] text-muted-foreground/70";

/** hover tooltip 用のフルテキスト(G1.7 方針2: alias 表示を許す3箇所目)。
 * `deriveLabel` が既に alias にフォールバックしている場合(テキストが空のノード)は
 * 同じ文字列を2回出さない。 */
export function hoverTitle(n: StrataNode): string {
  const label = deriveLabel(n);
  const bits = [label];
  if (n.alias && n.alias !== label) bits.push(`#${n.alias}`);
  return `${bits.join(" · ")}(${typeLabel(n.type)})`;
}

/** 文書(SiteRoot)の表示ラベル: document ノードの title を最優先し、無ければ alias、
 * それも無ければファイルパス(裁量: 通常のノードと違い、文書は ULID 短縮より
 * ファイルパスの方が手がかりになるので最終段だけ差し替える)。GraphPane の docBand
 * ラベルと DocumentPane のタブラベルを共通化する(G1.7: 同じ文書が場所によって
 * 和文タイトル/英字 alias/パスとバラバラに出ていたのを統一)。 */
export function docRootLabel(idx: GraphIndex, root: SiteRoot): string {
  const node = root.root ? idx.nodes.get(root.root) : undefined;
  const title = node ? (node as { title?: string }).title : undefined;
  return truncate(title || root.alias || root.path, 40);
}
