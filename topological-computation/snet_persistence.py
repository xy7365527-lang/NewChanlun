"""snet_persistence.py -- S_net SQLite 持久化（替代 pickle+gzip 全量序列化）。

解决 snet_cache.py 的三个问题：
  1. 全量序列化 → SQLite 行级存储，支持增量更新
  2. crash 丢失 → WAL 模式 + 增量 INSERT 实时持久化穿越新边
  3. 无查询能力 → SQL 查询

数据库路径: ~/.swarm/persist/snet.db (WAL mode)

认识论等级: L0（序列化/反序列化是无损代数操作，SQLite 是存储引擎替换）

谱系引用: 423号候选5（S_net 持久化）
"""

from __future__ import annotations

import hashlib
import json
import sqlite3
import sys
import time
from pathlib import Path
from typing import Optional

from signifier_net import (
    SNet,
    Signifier,
    SignifierEdge,
    AxisType,
    Morpheme,
    MorphemeStructure,
)


# ---------------------------------------------------------------------------
# 默认路径
# ---------------------------------------------------------------------------

_DEFAULT_DB_DIR = Path.home() / ".swarm" / "persist"
_DEFAULT_DB_PATH = _DEFAULT_DB_DIR / "snet.db"


# ---------------------------------------------------------------------------
# Schema
# ---------------------------------------------------------------------------

_SCHEMA_SQL = """
CREATE TABLE IF NOT EXISTS signifiers (
    id TEXT PRIMARY KEY,
    surface_forms TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT '',
    lang TEXT NOT NULL DEFAULT '',
    domain TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source TEXT NOT NULL,
    target TEXT NOT NULL,
    axis TEXT NOT NULL,
    weight REAL NOT NULL DEFAULT 0.0,
    evidence TEXT NOT NULL DEFAULT '',
    relation TEXT NOT NULL DEFAULT '',
    differential TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source);
CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target);
CREATE UNIQUE INDEX IF NOT EXISTS idx_edges_unique ON edges(source, target, axis);

CREATE TABLE IF NOT EXISTS morphemes (
    signifier_id TEXT PRIMARY KEY,
    morphemes_json TEXT NOT NULL,
    etymology TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"""


# ---------------------------------------------------------------------------
# SNetPersistence
# ---------------------------------------------------------------------------

class SNetPersistence:
    """S_net SQLite 持久化层。

    设计约束：
      - WAL 模式：读写并发安全
      - 边去重：UNIQUE INDEX on (source, target, axis)，INSERT OR IGNORE
      - SNet 类不改动：通过 to_dict/from_dict 等价性保证无损
      - 增量保存 < 1ms/步：单条 INSERT OR IGNORE
      - 全量保存 778K edges：executemany 批量插入

    认识论等级: L0（存储引擎替换，不引入经验假设）
    """

    def __init__(self, db_path: Optional[Path] = None) -> None:
        if db_path is None:
            db_path = _DEFAULT_DB_PATH
        self._db_path = Path(db_path)
        self._db_path.parent.mkdir(parents=True, exist_ok=True)
        self._conn: Optional[sqlite3.Connection] = None
        self._connect()

    def _connect(self) -> None:
        """建立连接并初始化 schema。"""
        self._conn = sqlite3.connect(
            str(self._db_path),
            isolation_level=None,  # autocommit for WAL pragma
        )
        # WAL mode for concurrent read/write safety
        self._conn.execute("PRAGMA journal_mode=WAL")
        # Synchronous NORMAL: good balance of durability and performance
        self._conn.execute("PRAGMA synchronous=NORMAL")
        # Larger cache for bulk operations
        self._conn.execute("PRAGMA cache_size=-64000")  # 64MB
        # Initialize schema
        self._conn.executescript(_SCHEMA_SQL)

    def close(self) -> None:
        """关闭连接。"""
        if self._conn is not None:
            self._conn.close()
            self._conn = None

    @property
    def db_path(self) -> Path:
        return self._db_path

    # ------------------------------------------------------------------
    # 全量保存
    # ------------------------------------------------------------------

    def save_full(self, snet: SNet) -> None:
        """全量保存 SNet 到 SQLite（首次摄入后 / 重建后）。

        策略：
          1. 清空所有表（TRUNCATE 等价）
          2. executemany 批量插入 signifiers, edges, morphemes
          3. 事务保护：全部成功或全部回滚

        性能目标: 778K edges < 10s (executemany + WAL)

        认识论等级: L0（无损序列化）
        """
        conn = self._conn
        t0 = time.time()

        conn.execute("BEGIN")
        try:
            # Clear existing data
            conn.execute("DELETE FROM signifiers")
            conn.execute("DELETE FROM edges")
            conn.execute("DELETE FROM morphemes")

            # Signifiers
            sig_rows = []
            for sid, sig in snet._signifiers.items():
                sig_rows.append((
                    sig.id,
                    json.dumps(list(sig.surface_forms), ensure_ascii=False),
                    sig.source,
                    sig.lang,
                    sig.domain,
                ))
            if sig_rows:
                conn.executemany(
                    "INSERT INTO signifiers (id, surface_forms, source, lang, domain) "
                    "VALUES (?, ?, ?, ?, ?)",
                    sig_rows,
                )

            # Edges
            edge_rows = []
            for e in snet._edges:
                edge_rows.append((
                    e.source,
                    e.target,
                    e.axis.value,
                    e.weight,
                    e.evidence,
                    e.relation,
                    e.differential,
                ))
            if edge_rows:
                conn.executemany(
                    "INSERT OR IGNORE INTO edges "
                    "(source, target, axis, weight, evidence, relation, differential) "
                    "VALUES (?, ?, ?, ?, ?, ?, ?)",
                    edge_rows,
                )

            # Morphemes
            morph_rows = []
            for sid, ms in snet._morphemes.items():
                morph_json = json.dumps(
                    [
                        {
                            "form": m.form,
                            "meaning": m.meaning,
                            "lang": m.lang,
                            "shared_with": list(m.shared_with),
                        }
                        for m in ms.morphemes
                    ],
                    ensure_ascii=False,
                )
                morph_rows.append((
                    ms.signifier_id,
                    morph_json,
                    ms.etymology,
                ))
            if morph_rows:
                conn.executemany(
                    "INSERT INTO morphemes (signifier_id, morphemes_json, etymology) "
                    "VALUES (?, ?, ?)",
                    morph_rows,
                )

            conn.execute("COMMIT")
        except Exception:
            conn.execute("ROLLBACK")
            raise

        elapsed = time.time() - t0
        n_sig = len(sig_rows)
        n_edge = len(edge_rows)
        n_morph = len(morph_rows)
        print(
            f"S_net SQLite save_full: {n_sig} signifiers, {n_edge} edges, "
            f"{n_morph} morphemes ({elapsed:.1f}s) → {self._db_path}",
            file=sys.stderr,
        )

    # ------------------------------------------------------------------
    # 全量加载
    # ------------------------------------------------------------------

    def load_full(self) -> Optional[SNet]:
        """全量加载 SQLite → SNet。

        如果数据库为空（无 signifiers），返回 None。

        性能目标: < 5s

        认识论等级: L0（无损反序列化）
        """
        conn = self._conn
        t0 = time.time()

        # Check if DB has any data
        row = conn.execute("SELECT COUNT(*) FROM signifiers").fetchone()
        if row is None or row[0] == 0:
            return None

        # Load signifiers
        signifiers: dict[str, Signifier] = {}
        cursor = conn.execute(
            "SELECT id, surface_forms, source, lang, domain FROM signifiers"
        )
        for sid, sf_json, source, lang, domain in cursor:
            surface_forms = tuple(json.loads(sf_json))
            signifiers[sid] = Signifier(
                id=sid,
                surface_forms=surface_forms,
                source=source,
                lang=lang,
                domain=domain,
            )

        # Load edges
        edges: list[SignifierEdge] = []
        cursor = conn.execute(
            "SELECT source, target, axis, weight, evidence, relation, differential "
            "FROM edges"
        )
        for src, tgt, axis_str, weight, evidence, relation, differential in cursor:
            edges.append(SignifierEdge(
                source=src,
                target=tgt,
                axis=AxisType(axis_str),
                weight=weight,
                evidence=evidence,
                relation=relation,
                differential=differential,
            ))

        # Load morphemes
        morphemes: dict[str, MorphemeStructure] = {}
        cursor = conn.execute(
            "SELECT signifier_id, morphemes_json, etymology FROM morphemes"
        )
        for sig_id, morph_json, etymology in cursor:
            morph_list = tuple(
                Morpheme(
                    form=m["form"],
                    meaning=m["meaning"],
                    lang=m["lang"],
                    shared_with=tuple(m.get("shared_with", ())),
                )
                for m in json.loads(morph_json)
            )
            morphemes[sig_id] = MorphemeStructure(
                signifier_id=sig_id,
                morphemes=morph_list,
                etymology=etymology,
            )

        snet = SNet(signifiers=signifiers, edges=edges, morphemes=morphemes)
        elapsed = time.time() - t0

        print(
            f"S_net SQLite load_full: {len(signifiers)} signifiers, "
            f"{len(edges)} edges, {len(morphemes)} morphemes ({elapsed:.1f}s)",
            file=sys.stderr,
        )
        return snet

    # ------------------------------------------------------------------
    # 增量保存
    # ------------------------------------------------------------------

    def save_incremental(
        self,
        new_signifiers: Optional[list[Signifier]] = None,
        new_edges: Optional[list[SignifierEdge]] = None,
        new_morphemes: Optional[list[MorphemeStructure]] = None,
    ) -> int:
        """增量保存（穿越步进时调用）。

        使用 INSERT OR IGNORE 实现边去重（UNIQUE INDEX on source, target, axis）。
        signifiers 使用 INSERT OR REPLACE（后来的定义覆盖先前的）。

        返回实际插入的条目数（去重后）。

        性能目标: < 1ms/步（通常只有 0-3 条新边）

        认识论等级: L0（追加操作）
        """
        conn = self._conn
        inserted = 0

        if new_signifiers:
            rows = [
                (
                    sig.id,
                    json.dumps(list(sig.surface_forms), ensure_ascii=False),
                    sig.source,
                    sig.lang,
                    sig.domain,
                )
                for sig in new_signifiers
            ]
            conn.execute("BEGIN")
            try:
                conn.executemany(
                    "INSERT OR REPLACE INTO signifiers "
                    "(id, surface_forms, source, lang, domain) "
                    "VALUES (?, ?, ?, ?, ?)",
                    rows,
                )
                conn.execute("COMMIT")
                inserted += len(rows)
            except Exception:
                conn.execute("ROLLBACK")
                raise

        if new_edges:
            rows = [
                (
                    e.source,
                    e.target,
                    e.axis.value,
                    e.weight,
                    e.evidence,
                    e.relation,
                    e.differential,
                )
                for e in new_edges
            ]
            conn.execute("BEGIN")
            try:
                cursor = conn.executemany(
                    "INSERT OR IGNORE INTO edges "
                    "(source, target, axis, weight, evidence, relation, differential) "
                    "VALUES (?, ?, ?, ?, ?, ?, ?)",
                    rows,
                )
                conn.execute("COMMIT")
                # rowcount from executemany is the count of affected rows
                inserted += cursor.rowcount if cursor.rowcount > 0 else 0
            except Exception:
                conn.execute("ROLLBACK")
                raise

        if new_morphemes:
            rows = [
                (
                    ms.signifier_id,
                    json.dumps(
                        [
                            {
                                "form": m.form,
                                "meaning": m.meaning,
                                "lang": m.lang,
                                "shared_with": list(m.shared_with),
                            }
                            for m in ms.morphemes
                        ],
                        ensure_ascii=False,
                    ),
                    ms.etymology,
                )
                for ms in new_morphemes
            ]
            conn.execute("BEGIN")
            try:
                conn.executemany(
                    "INSERT OR REPLACE INTO morphemes "
                    "(signifier_id, morphemes_json, etymology) "
                    "VALUES (?, ?, ?)",
                    rows,
                )
                conn.execute("COMMIT")
                inserted += len(rows)
            except Exception:
                conn.execute("ROLLBACK")
                raise

        return inserted

    # ------------------------------------------------------------------
    # Manifest（缓存有效性判断）
    # ------------------------------------------------------------------

    def get_manifest_hash(self) -> Optional[str]:
        """获取存储的 manifest hash，用于判断缓存有效性。

        返回 None 表示无 manifest（需要全量重建）。
        """
        conn = self._conn
        row = conn.execute(
            "SELECT value FROM metadata WHERE key = 'manifest_hash'"
        ).fetchone()
        if row is None:
            return None
        return row[0]

    def set_manifest(self, manifest: dict) -> None:
        """保存 manifest 并计算 hash。

        manifest hash = SHA-256(canonical JSON)。
        同时保存完整 manifest JSON 以供调试。
        """
        conn = self._conn
        manifest_json = json.dumps(manifest, sort_keys=True, ensure_ascii=False)
        manifest_hash = hashlib.sha256(manifest_json.encode("utf-8")).hexdigest()

        conn.execute("BEGIN")
        try:
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?, ?)",
                ("manifest_hash", manifest_hash),
            )
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?, ?)",
                ("manifest_json", manifest_json),
            )
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES (?, ?)",
                ("manifest_updated", str(time.time())),
            )
            conn.execute("COMMIT")
        except Exception:
            conn.execute("ROLLBACK")
            raise

    def manifest_valid(self, manifest: dict) -> bool:
        """判断当前 manifest 是否与存储的一致。

        比较逻辑：canonical JSON → SHA-256 hash 比对。
        """
        stored_hash = self.get_manifest_hash()
        if stored_hash is None:
            return False
        manifest_json = json.dumps(manifest, sort_keys=True, ensure_ascii=False)
        current_hash = hashlib.sha256(manifest_json.encode("utf-8")).hexdigest()
        return stored_hash == current_hash

    # ------------------------------------------------------------------
    # 统计 / 诊断
    # ------------------------------------------------------------------

    def stats(self) -> dict:
        """返回数据库统计信息。"""
        conn = self._conn
        n_sig = conn.execute("SELECT COUNT(*) FROM signifiers").fetchone()[0]
        n_edge = conn.execute("SELECT COUNT(*) FROM edges").fetchone()[0]
        n_morph = conn.execute("SELECT COUNT(*) FROM morphemes").fetchone()[0]
        db_size = self._db_path.stat().st_size if self._db_path.exists() else 0
        return {
            "signifiers": n_sig,
            "edges": n_edge,
            "morphemes": n_morph,
            "db_size_mb": round(db_size / (1024 * 1024), 2),
            "db_path": str(self._db_path),
        }

    def __repr__(self) -> str:
        s = self.stats()
        return (
            f"SNetPersistence(signifiers={s['signifiers']}, "
            f"edges={s['edges']}, morphemes={s['morphemes']}, "
            f"db={s['db_size_mb']}MB)"
        )

    def __del__(self) -> None:
        self.close()
