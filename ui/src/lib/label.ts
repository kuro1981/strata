// ノードの短縮ラベル(グラフペインのノード表示・関係パネルのジャンプ先表示に使う)。
// D49: 「ラベルは見出し/先頭テキストの短縮」。

import type { Inline, StrataNode } from "@/types/graph";

const MAX_LEN = 28;

function truncate(s: string, max = MAX_LEN): string {
  const trimmed = s.trim().replace(/\s+/g, " ");
  return trimmed.length > max ? trimmed.slice(0, max - 1) + "…" : trimmed;
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

/** グラフペイン・関係パネルで使う短縮ラベル。 */
export function deriveLabel(n: StrataNode): string {
  switch (n.type) {
    case "document":
      return truncate((n as { title?: string }).title || n.alias || "(文書)");
    case "section":
      return truncate(inlineToText((n as { heading?: Inline[] }).heading) || "(見出し)");
    case "para": {
      const text = inlineToText((n as { inline?: Inline[] }).inline);
      return truncate(text || "(空の段落)");
    }
    case "list":
      return `リスト${(n as { ordered?: boolean }).ordered ? "(順序)" : ""}`;
    case "record":
      return truncate(n.alias || "record");
    case "table":
      return truncate(inlineToText((n as { caption?: Inline[] }).caption) || n.alias || "表");
    case "math":
      return truncate(n.alias || "数式");
    case "code":
      return truncate(n.alias || (n as { lang?: string }).lang || "コード");
    case "term":
      return truncate((n as { name?: string }).name || "用語");
    case "anchor":
      return truncate(inlineToText((n as { inline?: Inline[] }).inline) || "anchor");
    case "value": {
      const v = (n as { scalar?: unknown }).scalar;
      return truncate(String(v ?? ""));
    }
    case "quote":
      return "引用";
    case "thematic_break":
      return "───";
    case "figure":
      return truncate(inlineToText((n as { caption?: Inline[] }).caption) || n.alias || "図");
    default:
      return truncate(n.alias || typeLabel(n.type));
  }
}
