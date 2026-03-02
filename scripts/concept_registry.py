"""concept_registry.py — 概念注册表导出（穿越基础设施组件四）

从区块拓扑的 relations.jsonl 中提取所有 defines 关系，
构建概念注册表并导出为单文件 JSON。

ceremony 后全量重导。
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from block_topology import DEFAULT_BASE


@dataclass
class ConceptEntry:
    """注册表中的单个概念条目。"""

    term: str
    authoritative: bool
    defining_blocks: list[str]
    reference_count: int = 0


@dataclass
class ConceptRegistry:
    """概念注册表——concept_id 到 ConceptEntry 的映射。"""

    entries: dict[str, ConceptEntry] = field(default_factory=dict)

    def to_dict(self) -> dict[str, dict[str, Any]]:
        """序列化为 JSON-compatible dict，按 term 排序。"""
        return {
            cid: {
                "term": e.term,
                "authoritative": e.authoritative,
                "defining_blocks": e.defining_blocks,
                "reference_count": e.reference_count,
            }
            for cid, e in sorted(self.entries.items(), key=lambda x: x[1].term)
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ConceptRegistry:
        """从 JSON dict 反序列化。"""
        entries: dict[str, ConceptEntry] = {}
        for cid, d in data.items():
            entries[cid] = ConceptEntry(
                term=d["term"],
                authoritative=d["authoritative"],
                defining_blocks=d["defining_blocks"],
                reference_count=d.get("reference_count", 0),
            )
        return cls(entries=entries)


REGISTRY_PATH = Path(".chanlun/concept_registry.json")


def build_concept_registry(base: Path = DEFAULT_BASE) -> ConceptRegistry:
    """从区块拓扑构建概念注册表。全量重算。

    数据来源：relations.jsonl
    - defines 边：concept_term, concept_definition, from (block_id), to (concept_id)
    - references 边：from (block_id) → to (block_id)

    authoritative 判定：defines 边包含非空 concept_definition 字段。
    reference_count：该概念的定义区块被 references 边指向的总次数。
    """
    relations_path = base / "relations.jsonl"
    if not relations_path.exists():
        return ConceptRegistry()

    # concept_id → accumulated data
    concept_data: dict[str, dict[str, Any]] = {}
    # block_id → 被 references 边指向的次数
    referenced_blocks: dict[str, int] = {}

    with open(relations_path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            rel = json.loads(line)
            rtype = rel.get("relation")

            if rtype == "defines":
                concept_id = rel["to"]
                from_id = rel["from"]
                term = rel.get("concept_term", "")
                has_definition = bool(rel.get("concept_definition"))

                if concept_id not in concept_data:
                    concept_data[concept_id] = {
                        "term": term,
                        "blocks": [],
                        "authoritative": False,
                    }

                entry = concept_data[concept_id]
                if from_id not in entry["blocks"]:
                    entry["blocks"].append(from_id)

                # 任何一条 defines 边有 concept_definition → authoritative
                if has_definition:
                    entry["authoritative"] = True

                # 优先保留非空 term
                if term and not entry["term"]:
                    entry["term"] = term

            elif rtype == "references":
                to_id = rel["to"]
                referenced_blocks[to_id] = referenced_blocks.get(to_id, 0) + 1

    # 组装注册表
    registry = ConceptRegistry()
    for cid, data in concept_data.items():
        ref_count = sum(
            referenced_blocks.get(bid, 0) for bid in data["blocks"]
        )
        registry.entries[cid] = ConceptEntry(
            term=data["term"],
            authoritative=data["authoritative"],
            defining_blocks=data["blocks"],
            reference_count=ref_count,
        )

    return registry


def export_registry(
    registry: ConceptRegistry,
    output_path: Path = REGISTRY_PATH,
) -> Path:
    """导出注册表为 JSON 文件。"""
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        json.dumps(registry.to_dict(), ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    return output_path


def load_registry(path: Path = REGISTRY_PATH) -> ConceptRegistry:
    """从 JSON 文件加载注册表。文件不存在时返回空注册表。"""
    if not path.exists():
        return ConceptRegistry()
    data = json.loads(path.read_text(encoding="utf-8"))
    return ConceptRegistry.from_dict(data)
