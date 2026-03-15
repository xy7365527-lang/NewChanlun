"""cooccurrence_hyperedge.py — 共现超边数据结构。

基本事件不是成对边，而是"段落 P 包含 {A, B, C, D, ...}"。
CooccurrenceHyperedge 将一个段落中同时在场的所有术语记录为一个超边，
避免展开为 C(N,2) 成对边导致的 O(n^2) 性能瓶颈和信息丢失。

成对边是超边的派生物（lazy），不是基本事件。

认识论等级：L0（纯数据结构定义，无经验假设）

谱系引用：v243-swarm/hyperedge-core
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class CooccurrenceHyperedge:
    """共现超边：一个段落中同时在场的所有术语。

    vertices:          参与术语的 signifier IDs（frozenset，无序、不重复）
    source:            来源标记（如 "phenomenology_of_spirit.md"）
    domain:            领域标记（如 "hegel"）
    timestamp:         摄入时间戳
    ingest_param_refs: 指向 snet_param:xxx 概念层节点的引用（不是值副本！442号对齐）
    evidence_tag:      语料引用标记（如 "corpus:hegel:phenomenology_of_spirit.md"）
    """
    vertices: frozenset[str]
    source: str
    domain: str
    timestamp: str
    ingest_param_refs: tuple[str, ...] = ()
    evidence_tag: str = ""


def hyperedge_to_dict(he: CooccurrenceHyperedge) -> dict:
    """将超边序列化为纯 Python dict。

    认识论等级：L0（无损序列化）
    """
    return {
        "vertices": sorted(he.vertices),
        "source": he.source,
        "domain": he.domain,
        "timestamp": he.timestamp,
        "ingest_param_refs": list(he.ingest_param_refs),
        "evidence_tag": he.evidence_tag,
    }


def hyperedge_from_dict(data: dict) -> CooccurrenceHyperedge:
    """从 dict 反序列化超边。

    认识论等级：L0（无损反序列化）
    """
    return CooccurrenceHyperedge(
        vertices=frozenset(data["vertices"]),
        source=data.get("source", ""),
        domain=data.get("domain", ""),
        timestamp=data.get("timestamp", ""),
        ingest_param_refs=tuple(data.get("ingest_param_refs", ())),
        evidence_tag=data.get("evidence_tag", ""),
    )
