"""Shared event layer: append-only blocks + appendable relations — IPFS backend.

IPFS 就是 SharedLayer。区块直接写到 IPFS、从 IPFS 读。
本地文件系统不再是存储位置。CID 替代 SHA256 hash 作为 block 标识。

索引机制：IPFS MFS (Mutable File System)
- /shared/blocks/  — 每个 block 的 CID 作为文件名，内容为空（CID 本身就是索引键）
- /shared/relations/ — 每条 relation 的 CID 作为文件名

MFS 提供"列目录"能力，解决 IPFS 内容寻址无法枚举的问题。
"""

from __future__ import annotations

import json
import time

from chain.ipfs_client import IPFSClient


# MFS 根目录
_MFS_ROOT = "/shared"
_MFS_BLOCKS = f"{_MFS_ROOT}/blocks"
_MFS_RELATIONS = f"{_MFS_ROOT}/relations"


class SharedLayer:
    """Content-addressed block store + relation log — IPFS backend.

    构造函数接受 IPFSClient 实例。IPFS 不可用时拒绝创建（不降级到本地）。
    """

    def __init__(self, ipfs: IPFSClient):
        if not ipfs.is_available():
            raise RuntimeError(
                "IPFS daemon 不可用。SharedLayer 要求 IPFS 在线运行。"
                "区块拓扑在链上执行是存在方式，不是可选优化。"
            )
        self._ipfs = ipfs
        # 初始化 MFS 目录结构
        self._ipfs.files_mkdir(_MFS_BLOCKS)
        self._ipfs.files_mkdir(_MFS_RELATIONS)

    def write_block(self, content: dict) -> str:
        """写入内容寻址区块到 IPFS。返回 CID（替代原来的 SHA256 hash）。

        CID 由 IPFS 根据内容自动生成，天然去重。
        写入后在 MFS 索引中注册 CID，并 pin 防止 GC。
        """
        data = json.dumps(content, sort_keys=True, ensure_ascii=False)
        cid = self._upload_pinned(data)
        # 在 MFS 中注册 CID（文件名 = CID，内容为空占位）
        mfs_path = f"{_MFS_BLOCKS}/{cid}"
        self._ipfs.files_write(mfs_path, b"", create=True, truncate=True)
        return cid

    def _upload_pinned(self, data: str) -> str:
        """Upload and require pin before exposing the CID through MFS indexes."""
        cid = self._ipfs.upload(data)
        if not self._ipfs.pin(cid):
            raise RuntimeError(f"IPFS pin failed for CID {cid}")
        return cid

    def read_block(self, cid: str) -> dict | None:
        """根据 CID 从 IPFS 读取区块。"""
        try:
            raw = self._ipfs.download(cid)
            return json.loads(raw.decode("utf-8"))
        except Exception:
            return None

    def write_relation(
        self,
        from_hash: str,
        to_hash: str,
        relation: str,
        instance_id: str,
    ) -> None:
        """写入 relation 记录到 IPFS。

        每条 relation 作为独立 IPFS block 存储，CID 注册到 MFS /shared/relations/。
        """
        record = {
            "from": from_hash,
            "to": to_hash,
            "relation": relation,
            "instance": instance_id,
            "timestamp": time.time(),
        }
        data = json.dumps(record, sort_keys=True, ensure_ascii=False)
        cid = self._upload_pinned(data)
        # 注册到 MFS relations 索引
        mfs_path = f"{_MFS_RELATIONS}/{cid}"
        self._ipfs.files_write(mfs_path, b"", create=True, truncate=True)

    def read_new_blocks(self, known_cids: set[str]) -> list[dict]:
        """读取不在 known_cids 中的新区块。返回带 'hash' 字段（CID）的 block list。"""
        all_cids = self.all_block_hashes()
        new_cids = all_cids - known_cids
        new_blocks: list[dict] = []
        for cid in new_cids:
            content = self.read_block(cid)
            if content is not None:
                content["hash"] = cid
                new_blocks.append(content)
        return new_blocks

    def all_block_hashes(self) -> set[str]:
        """返回所有已知 block 的 CID 集合（通过 MFS 目录枚举）。"""
        entries = self._ipfs.files_ls(_MFS_BLOCKS)
        return {entry["Name"] for entry in entries}

    def read_relations(self) -> list[dict]:
        """读取所有 relation 记录。"""
        entries = self._ipfs.files_ls(_MFS_RELATIONS)
        records: list[dict] = []
        for entry in entries:
            cid = entry["Name"]
            try:
                raw = self._ipfs.download(cid)
                records.append(json.loads(raw.decode("utf-8")))
            except Exception:
                continue
        return records
