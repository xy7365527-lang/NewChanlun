#!/usr/bin/env python3
"""#1488 S2（0055/F9）：会话库的“主库文件 + 逻辑内容”摘要。

只比主库文件 sha 会漏掉未 checkpoint 的 WAL（两跑 WAL 盐不同，字节必然不同）。本工具给出：
  main_file_sha256 / wal_bytes   原样（未 checkpoint）主库文件与 WAL 大小
  checkpointed_sha256            wal_checkpoint(TRUNCATE) 后主库文件 sha（WAL 并入，跨跑可比）
  logical_dump_sha256            iterdump() 全库逻辑 SQL 文本 sha（与页面布局无关）
  table_rows                     各表行数

默认在副本上做（--in-place 才改原库，驱动在服务停止后使用）；不启动产品。
用法：python3 db_logical_digest.py <session.sqlite> [--in-place]
"""
import hashlib
import json
import os
import shutil
import sqlite3
import sys
import tempfile


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def digest(db, in_place=False):
    wal = db + "-wal"
    out = {"db": db, "main_file_sha256": sha256_file(db), "wal_bytes": os.path.getsize(wal) if os.path.exists(wal) else 0}
    tmp = None
    if not in_place:
        tmp = tempfile.mkdtemp(prefix="dbdigest-")
        target = os.path.join(tmp, "session.sqlite")
        shutil.copy2(db, target)
        if os.path.exists(wal):
            shutil.copy2(wal, target + "-wal")
        db = target
    try:
        c = sqlite3.connect(db)
        busy, log, ckpt = c.execute("PRAGMA wal_checkpoint(TRUNCATE)").fetchone()
        out["checkpoint"] = {"busy": busy, "log_frames": log, "checkpointed_frames": ckpt}
        h = hashlib.sha256()
        for line in c.iterdump():
            h.update(line.encode("utf-8"))
            h.update(b"\n")
        out["logical_dump_sha256"] = h.hexdigest()
        tables = [r[0] for r in c.execute("select name from sqlite_master where type='table' order by name")]
        out["table_rows"] = {t: c.execute(f'select count(*) from "{t}"').fetchone()[0] for t in tables}
        c.close()
        out["checkpointed_sha256"] = sha256_file(db)
        out["checkpoint_ok"] = busy == 0
    finally:
        if tmp:
            shutil.rmtree(tmp, ignore_errors=True)
    return out


if __name__ == "__main__":
    print(json.dumps(digest(sys.argv[1], in_place="--in-place" in sys.argv[2:]), ensure_ascii=False, indent=1, sort_keys=True))
