// PR1915@7be051e3bf59148f6d39f0917513ed82c2ca6e19 scripts/wayfinder_engine.mts:181-272 原样协议片段。
import { createHash } from "node:crypto";
interface ChildIssue {number: number; body: string; comments: {author: string; body: string; createdAt: string}[]}
export const SPEC_RULING_HEAD = "SPEC 独审裁定";

const REJECT_RE = /不批准|不通过|打回|驳回|重写|reject|deny|nack|暂不/i;

/** SPEC 正文摘要：GitHub 返回的 body，统一 CRLF 并去掉末尾空白后取 sha256。 */
export function specBodyDigest(body: string): string {
  return createHash("sha256").update(body.replace(/\r\n/g, "\n").trimEnd(), "utf8").digest("hex");
}

export interface SpecReview {
  specNumber: number;
  verdict: "PASS" | "BLOCK";
  /** 起草者身份（模型或会话）。 */
  author: string;
  /** 独审者身份；必须不同于起草者。 */
  reviewer: string;
  /** 独审时所审 SPEC 正文的 specBodyDigest。 */
  digest: string;
}

export interface SpecRuling extends SpecReview {
  /** 独审原件位置（路径或链接）。 */
  evidence: string;
}

/** 去掉行首 Markdown 前缀（引用、标题、列表、强调）与行尾强调；正文里的下划线保留（路径常含 _）。 */
const strip = (line: string) => line.replace(/^(?:\s*(?:>|#{1,6}|[-*]\s|\*{1,3}|_{1,3}))+\s*/, "").replace(/(?:\*{1,3}|_{1,3})\s*$/, "").trim();
const field = (lines: string[], name: string) => {
  for (const l of lines) {
    const m = new RegExp(`^${name}\\s*[：:]\\s*(.*)$`).exec(strip(l));
    if (m) return m[1]!.replace(/^`|`$/g, "").trim();
  }
  return "";
};

/** 解析结构化 SPEC 裁定评论；首行不是「SPEC 独审裁定：PASS|BLOCK」返回 null。 */
export function parseSpecRuling(body: string): SpecRuling | null {
  const lines = body.trim().split("\n");
  const m = new RegExp(`^${SPEC_RULING_HEAD}(?:\\*{1,3}|_{1,3})?\\s*[：:]\\s*(?:\\*{1,3}|_{1,3})?(PASS|BLOCK)\\b`).exec(strip(lines[0] ?? ""));
  if (!m) return null;
  const rest = lines.slice(1);
  return {
    specNumber: Number(field(rest, "SPEC")),
    verdict: m[1] as SpecRuling["verdict"],
    author: field(rest, "作者"),
    reviewer: field(rest, "独审"),
    digest: field(rest, "正文 sha256").toLowerCase(),
    evidence: field(rest, "证据"),
  };
}

/** 审查绑定同时用于原件发布和评论消费；BLOCK 原件也须绑定所审正文。 */
function specReviewProblem(r: SpecReview, specNumber: number, specBody: string): string | null {
  if (!Number.isSafeInteger(r.specNumber) || r.specNumber <= 0 || r.specNumber !== specNumber) return "SPEC 票号不匹配";
  if (r.verdict !== "PASS" && r.verdict !== "BLOCK") return "结论只能是 PASS 或 BLOCK";
  if (![r.author, r.reviewer].every((v) => typeof v === "string" && v.trim() && !/[\r\n]/.test(v))) return "缺作者或独审单行身份";
  if (r.author.trim().toLowerCase() === r.reviewer.trim().toLowerCase()) return "独审与作者相同";
  if (typeof r.digest !== "string" || !/^[a-f0-9]{64}$/.test(r.digest)) return "缺正文 sha256";
  if (r.digest !== specBodyDigest(specBody)) return "正文已在独审后改动";
  return null;
}

/** 只有绑定当前 SPEC 的非作者 PASS 才能放行。 */
export function specRulingProblem(r: SpecRuling, specNumber: number, specBody: string): string | null {
  if (r.verdict !== "PASS") return "裁定为 BLOCK";
  if (!r.evidence.trim() || /[\r\n]/.test(r.evidence)) return "缺独审原件位置";
  return specReviewProblem(r, specNumber, specBody);
}

/** 结构化裁定评论正文（给主控发评论用）。 */
export function specRulingBody(r: SpecRuling): string {
  return [`**${SPEC_RULING_HEAD}：${r.verdict}**`, `SPEC：${r.specNumber}`, `作者：${r.author}`, `独审：${r.reviewer}`, `正文 sha256：${r.digest}`, `证据：${r.evidence}`].join("\n") + "\n";
}

/**
 * spec 是否已裁定放行。只认 orchestrator 的评论，按时间序推进：
 *   - 结构化「SPEC 独审裁定」：PASS 且有效（非作者、证据齐、摘要等于当前正文）→ 放行；BLOCK 或无效 PASS → 不放行；
 *   - 其他评论含驳回词 → 不放行（REJECT 先判，避免「不通过」被「通过」子串误命中）；
 *   - 自由文本批准不放行，不区分票或评论的创建日期。
 */
export function isSpecApproved(spec: ChildIssue, orchestrator: string): boolean {
  const mine = spec.comments
    .filter((c) => c.author === orchestrator)
    .sort((a, b) => (a.createdAt < b.createdAt ? -1 : a.createdAt > b.createdAt ? 1 : 0));
  let approved = false;
  for (const c of mine) {
    const ruling = parseSpecRuling(c.body);
    if (ruling) approved = specRulingProblem(ruling, spec.number, spec.body) === null;
    else if (REJECT_RE.test(c.body)) approved = false;
  }
  return approved;
}
