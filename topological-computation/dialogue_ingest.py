"""dialogue_ingest.py — CC session 对话回流通道。

将 .chanlun/sessions/*.md 中的对话文本摄入 S_net，
产生术语共现边 → block topology。

数据流：
  1. 扫描 sessions_dir 下的 .md 文件
  2. 对未处理文件：提取有语义价值的文本段落（跳过表格/分隔符/纯元数据）
  3. 调用 ingest_text_passage_batch() → S_net 更新
  4. 新共现边 → append_cooccurrence() → block topology

去重机制：state_file（JSON）记录已处理文件的 SHA-256 hash。
文件内容变更时重新摄入。

认识论等级：L0（确定性文件读取 + 字符串匹配，不涉及经验假设）

谱系引用：由 v229-swarm/snet-phase15 创建。
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


# --- 段落过滤规则 ---

# 匹配 markdown 表格行：以 | 开头且含多个 |
_TABLE_RE = re.compile(r"^\s*\|.*\|.*\|")
# 匹配表格分隔行：|---|---|
_TABLE_SEP_RE = re.compile(r"^\s*\|[\s\-:]+\|")
# 匹配纯元数据行：**key**: value 格式
_META_RE = re.compile(r"^\s*\*\*\S+\*\*\s*:")
# 匹配 markdown 标题行
_HEADING_RE = re.compile(r"^\s*#{1,6}\s")
# 匹配列表项中的纯引用标记（→ 来源: ...）
_SOURCE_REF_RE = re.compile(r"^\s*→\s*来源:")
# 匹配 checkbox 标记
_CHECKBOX_RE = re.compile(r"^\s*-\s*\[[ x✅⚠️❌]\]")
# 匹配纯 commit hash 行
_COMMIT_RE = re.compile(r"^\s*-\s*[0-9a-f]{7,}\s")


@dataclass(frozen=True)
class IngestResult:
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


def _is_noise_line(line: str) -> bool:
    """判断一行是否为噪声（表格/分隔符/纯元数据）。"""
    stripped = line.strip()
    if not stripped:
        return False  # 空行用于分隔段落，保留
    if _TABLE_RE.match(stripped):
        return True
    if _TABLE_SEP_RE.match(stripped):
        return True
    if _META_RE.match(stripped):
        return True
    if _SOURCE_REF_RE.match(stripped):
        return True
    if _CHECKBOX_RE.match(stripped):
        return True
    if _COMMIT_RE.match(stripped):
        return True
    # 纯分隔符（---、===、***）
    if re.fullmatch(r"[-=*]{3,}", stripped):
        return True
    return False


def _extract_paragraphs(text: str) -> list[str]:
    """从 session markdown 中提取有语义价值的段落。

    过滤逻辑：
      - 跳过表格行、分隔符行、纯元数据行、来源引用行
      - 跳过 commit hash 行
      - 保留标题行（标题本身是语义信息）
      - 连续非空有效行合并为段落
      - 过短段落（< 15 字符）丢弃
    """
    filtered_lines: list[str] = []
    for line in text.split("\n"):
        if _is_noise_line(line):
            # 用空行替换噪声行，保持段落分隔
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


def ingest_sessions(
    snet: SNet,
    bt_writer,
    sessions_dir: str | Path,
    state_file: str | Path,
) -> tuple[SNet, IngestResult]:
    """摄入 session 目录中的对话文本到 S_net。

    参数：
      snet:         当前 S_net 实例（不会被修改）
      bt_writer:    BlockTopologyWriter 实例（用于写入共现边到 block topology）
                    可以为 None（跳过 block topology 写入）
      sessions_dir: session 文件目录路径
      state_file:   状态跟踪文件路径

    返回：
      (new_snet, IngestResult)

    认识论等级：L0
    """
    sessions_path = Path(sessions_dir)
    state_path = Path(state_file)

    if not sessions_path.is_dir():
        return snet, IngestResult(
            files_processed=0, files_skipped=0,
            paragraphs_ingested=0, cooccurrence_entries=0,
            errors=("sessions_dir does not exist",),
        )

    # 加载去重状态
    state = _load_state(state_path)

    # 收集待处理文件
    md_files = sorted(
        f for f in sessions_path.iterdir()
        if f.is_file() and f.suffix.lower() == ".md"
    )

    if not md_files:
        return snet, IngestResult(
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

    # 第一轮：收集所有待处理段落（去重 + 文件读取）
    all_paragraphs: list[str] = []
    processed_keys: list[tuple[str, str]] = []  # (filename, file_hash)
    for md_file in md_files:
        try:
            file_hash = _compute_file_hash(md_file)

            if state.get(md_file.name) == file_hash:
                files_skipped += 1
                continue

            text = md_file.read_text(encoding="utf-8")
            paragraphs = _extract_paragraphs(text)

            if not paragraphs:
                state[md_file.name] = file_hash
                files_skipped += 1
                continue

            all_paragraphs.extend(paragraphs)
            processed_keys.append((md_file.name, file_hash))
            files_processed += 1
            total_paragraphs += len(paragraphs)

        except Exception as exc:
            errors.append(f"{md_file.name}: {exc}")

    # 第二轮：一次性批量摄入（白名单构建一次，SNet 重建一次）
    all_log_entries: list[dict] = []
    if all_paragraphs:
        source = f"cc_session({files_processed} files)"
        new_snet, all_log_entries = ingest_text_passage_batch(
            new_snet, all_paragraphs,
            domain="cc_session",
            source=source,
        )

        if bt_writer is not None and all_log_entries:
            timestamp = datetime.now(timezone.utc).isoformat()
            for entry in all_log_entries:
                if entry.get("type") == "cooccurrence":
                    bt_writer.append_cooccurrence(
                        source_signifier=entry["source"],
                        target_signifier=entry["target"],
                        weight=1.0,
                        corpus_source=f"cc_session:batch",
                        ingest_params={"domain": "cc_session"},
                        timestamp=timestamp,
                    )

        total_cooccurrences = sum(
            1 for e in all_log_entries if e.get("type") == "cooccurrence"
        )

    # 更新 state
    for filename, file_hash in processed_keys:
        state[filename] = file_hash

    # 保存状态
    _save_state(state_path, state)

    return new_snet, IngestResult(
        files_processed=files_processed,
        files_skipped=files_skipped,
        paragraphs_ingested=total_paragraphs,
        cooccurrence_entries=total_cooccurrences,
        errors=tuple(errors),
    )
