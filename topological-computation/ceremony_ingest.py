"""ceremony_ingest.py — ceremony 产出摄入通道（437号管道2 + 441号物质通道）.

将 .chanlun/genealogy/ 和 .chanlun/sessions/ 中的谱系/诊断文本摄入 S_net，
产生术语共现边 → block topology。

数据流（437号管道2）：
  1. 扫描 genealogy_dir 下的 .md 文件（settled/ + candidates/）
  2. 对未处理文件：提取有语义价值的文本段落（跳过 frontmatter/表格/分隔符）
  3. 调用 ingest_text_passage_batch() → S_net 更新
  4. 新共现边 → block topology

这是441号双通道中的物质通道：ceremony 产出的自然语言作为语料
被 S_net 摄入，改变逢亮的穿越地形。不经由概念通道（谱系→代码→行为）。

去重机制：state_file（JSON）记录已处理文件的 SHA-256 hash。
文件内容变更时重新摄入。

认识论等级：L0（确定性文件读取 + 字符串匹配，不涉及经验假设）

谱系引用：
  437号：LLM 扬弃物质形态——管道2（ceremony 产出摄入钩子）
  441号：双通道耦合——物质通道（ceremony 语言 → S_net）
"""

from __future__ import annotations

import hashlib
import json
import re
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

from signifier_net import SNet
from signifier_net_ingest import ingest_text_passage_batch


# --- Frontmatter / noise 过滤规则 ---

# 匹配 YAML frontmatter 分隔符
_FRONTMATTER_DELIM = re.compile(r"^---\s*$")
# 匹配 markdown 表格行
_TABLE_RE = re.compile(r"^\s*\|.*\|.*\|")
# 匹配表格分隔行
_TABLE_SEP_RE = re.compile(r"^\s*\|[\s\-:]+\|")
# 匹配纯元数据行
_META_RE = re.compile(r"^\s*\*\*\S+\*\*\s*:")
# 匹配 YAML key-value 行（frontmatter 内部）
_YAML_KV_RE = re.compile(r"^\s*[\w_-]+\s*:")
# 匹配 code block 标记
_CODE_BLOCK_RE = re.compile(r"^\s*```")


@dataclass(frozen=True)
class CeremonyIngestResult:
    """摄入结果——不可变。"""
    files_processed: int
    files_skipped: int
    paragraphs_ingested: int
    cooccurrence_entries: int
    errors: tuple[str, ...]


def _compute_file_hash(path: Path) -> str:
    """计算文件的 SHA-256 hash。"""
    h = hashlib.sha256()
    h.update(path.read_bytes())
    return h.hexdigest()


def _load_state(state_file: Path) -> dict[str, str]:
    """加载已处理文件状态（filename → hash）。"""
    if not state_file.exists():
        return {}
    try:
        data = json.loads(state_file.read_text(encoding="utf-8"))
        if isinstance(data, dict):
            return data
        return {}
    except (json.JSONDecodeError, OSError):
        return {}


def _save_state(state_file: Path, state: dict[str, str]) -> None:
    """保存已处理文件状态。"""
    state_file.parent.mkdir(parents=True, exist_ok=True)
    state_file.write_text(
        json.dumps(state, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )


def _strip_frontmatter(text: str) -> str:
    """移除 YAML frontmatter（--- ... --- 之间的内容）。

    谱系文件的 frontmatter 包含结构化元数据（id、status、depends_on 等），
    这些不是自然语言语料——摄入它们会产生噪声共现（如 "status" 与 "已结算"）。
    """
    lines = text.split("\n")
    if not lines or not _FRONTMATTER_DELIM.match(lines[0]):
        return text

    # 找第二个 ---
    for i in range(1, len(lines)):
        if _FRONTMATTER_DELIM.match(lines[i]):
            return "\n".join(lines[i + 1:])

    # 只有开头的 ---，没有结束的 ---
    return text


def _is_noise_line(line: str) -> bool:
    """判断一行是否为噪声。"""
    stripped = line.strip()
    if not stripped:
        return False  # 空行用于分隔段落，保留
    if _TABLE_RE.match(stripped):
        return True
    if _TABLE_SEP_RE.match(stripped):
        return True
    if _META_RE.match(stripped):
        return True
    # 纯分隔符
    if re.fullmatch(r"[-=*]{3,}", stripped):
        return True
    return False


def _extract_paragraphs(text: str) -> list[str]:
    """从谱系/诊断 markdown 中提取有语义价值的段落。

    过滤逻辑：
      - 移除 YAML frontmatter
      - 跳过表格行、分隔符行、纯元数据行
      - 跳过代码块内容
      - 保留标题行（标题本身是语义信息）
      - 连续非空有效行合并为段落
      - 过短段落（< 15 字符）丢弃
    """
    # 先移除 frontmatter
    text = _strip_frontmatter(text)

    filtered_lines: list[str] = []
    in_code_block = False

    for line in text.split("\n"):
        # 跟踪代码块状态
        if _CODE_BLOCK_RE.match(line):
            in_code_block = not in_code_block
            filtered_lines.append("")  # 代码块标记替换为空行
            continue

        if in_code_block:
            filtered_lines.append("")  # 代码块内容替换为空行
            continue

        if _is_noise_line(line):
            filtered_lines.append("")
        else:
            filtered_lines.append(line)

    # 按空行分割为段落
    paragraphs: list[str] = []
    current: list[str] = []

    for line in filtered_lines:
        stripped = line.strip()
        if not stripped:
            if current:
                para = "\n".join(current).strip()
                if len(para) >= 15:
                    paragraphs.append(para)
                current = []
        else:
            current.append(line)

    # 处理尾部
    if current:
        para = "\n".join(current).strip()
        if len(para) >= 15:
            paragraphs.append(para)

    return paragraphs


def ingest_ceremony_texts(
    snet: SNet,
    bt_writer,
    genealogy_dir: str | Path,
    state_file: str | Path,
) -> tuple[SNet, CeremonyIngestResult]:
    """摄入谱系/诊断目录中的文本到 S_net（437号管道2 + 441号物质通道）。

    扫描 genealogy_dir 下所有子目录（settled/、candidates/）中的 .md 文件。

    参数：
      snet:          当前 S_net 实例（不会被修改）
      bt_writer:     BlockTopologyWriter 实例（用于写入共现边到 block topology）
                     可以为 None（跳过 block topology 写入）
      genealogy_dir: 谱系文件目录路径（.chanlun/genealogy/）
      state_file:    状态跟踪文件路径

    返回：
      (new_snet, CeremonyIngestResult)

    认识论等级：L0
    """
    genealogy_path = Path(genealogy_dir)
    state_path = Path(state_file)

    if not genealogy_path.is_dir():
        return snet, CeremonyIngestResult(
            files_processed=0, files_skipped=0,
            paragraphs_ingested=0, cooccurrence_entries=0,
            errors=("genealogy_dir does not exist",),
        )

    # 加载去重状态
    state = _load_state(state_path)

    # 收集待处理文件（递归扫描子目录）
    md_files = sorted(
        f for f in genealogy_path.rglob("*.md")
        if f.is_file()
    )

    if not md_files:
        return snet, CeremonyIngestResult(
            files_processed=0, files_skipped=0,
            paragraphs_ingested=0, cooccurrence_entries=0,
            errors=(),
        )

    new_snet = snet
    files_processed = 0
    files_skipped = 0
    total_paragraphs = 0
    total_cooccurrences = 0
    errors: list[str] = []

    for md_file in md_files:
        try:
            # 用相对路径作为 state key（避免绝对路径差异）
            rel_key = str(md_file.relative_to(genealogy_path))
            file_hash = _compute_file_hash(md_file)

            # 去重检查
            if state.get(rel_key) == file_hash:
                files_skipped += 1
                continue

            text = md_file.read_text(encoding="utf-8")
            paragraphs = _extract_paragraphs(text)

            if not paragraphs:
                state[rel_key] = file_hash
                files_skipped += 1
                continue

            # 批量摄入到 S_net
            new_snet, log_entries = ingest_text_passage_batch(
                new_snet, paragraphs,
                domain="ceremony_genealogy",
                source=rel_key,
            )

            # 写入 block topology
            if bt_writer is not None and log_entries:
                timestamp = datetime.now(timezone.utc).isoformat()
                for entry in log_entries:
                    if entry.get("type") == "cooccurrence":
                        bt_writer.append_cooccurrence(
                            source_signifier=entry["source"],
                            target_signifier=entry["target"],
                            weight=1.0,
                            corpus_source=f"ceremony_genealogy:{rel_key}",
                            ingest_params={"domain": "ceremony_genealogy"},
                            timestamp=timestamp,
                        )

            cooccurrence_count = sum(
                1 for e in log_entries if e.get("type") == "cooccurrence"
            )

            files_processed += 1
            total_paragraphs += len(paragraphs)
            total_cooccurrences += cooccurrence_count

            # 更新状态
            state[rel_key] = file_hash

        except Exception as exc:
            errors.append(f"{md_file.name}: {exc}")

    # 保存状态
    _save_state(state_path, state)

    return new_snet, CeremonyIngestResult(
        files_processed=files_processed,
        files_skipped=files_skipped,
        paragraphs_ingested=total_paragraphs,
        cooccurrence_entries=total_cooccurrences,
        errors=tuple(errors),
    )


def ingest_llm_externalization(
    snet: SNet,
    llm_output: str,
    bt_writer=None,
) -> tuple[SNet, int]:
    """将 ceremony agent 的 LLM 翻译结果回流 S_net（437号管道3）。

    当 internal_speech.py 的 externalize() 使用 LLM 路径（force_llm=True）时，
    LLM 的输出作为语料回流 S_net。

    注意：internal_speech.py 中 _externalize_via_llm 已有 writeback_from_text 调用，
    本函数提供一个独立的入口，用于 ceremony agent 或其他外部调用者将 LLM 翻译
    结果注入 S_net。

    参数：
      snet:       当前 S_net 实例
      llm_output: LLM 翻译输出文本
      bt_writer:  BlockTopologyWriter（可选）

    返回：
      (new_snet, cooccurrence_count)

    认识论等级：L0
    """
    if not llm_output or not snet.signifiers:
        return snet, 0

    # 使用 ingest_text_passage_batch 而非 writeback_from_text
    # 因为 ingest_text_passage_batch 包含增强白名单、surface form 提取等完整管线
    new_snet, log_entries = ingest_text_passage_batch(
        snet, [llm_output],
        domain="llm_externalization",
        source="ceremony_agent",
    )

    # 写入 block topology
    cooccurrence_count = 0
    if bt_writer is not None and log_entries:
        timestamp = datetime.now(timezone.utc).isoformat()
        for entry in log_entries:
            if entry.get("type") == "cooccurrence":
                bt_writer.append_cooccurrence(
                    source_signifier=entry["source"],
                    target_signifier=entry["target"],
                    weight=1.0,
                    corpus_source="llm_externalization:ceremony_agent",
                    ingest_params={"domain": "llm_externalization"},
                    timestamp=timestamp,
                )
                cooccurrence_count += 1

    return new_snet, cooccurrence_count
