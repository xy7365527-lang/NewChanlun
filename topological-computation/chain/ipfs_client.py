"""IPFS HTTP API client for block upload/download/pin + MFS operations.

Works with both public and private IPFS networks — transparent to client.
Uses IPFS HTTP API (default: localhost:5001).
Pure Python, stdlib only (urllib).

MFS (Mutable File System) 方法用于 SharedLayer 的索引机制：
IPFS 是内容寻址的（没有"列目录"），MFS 提供类文件系统操作来跟踪已知 CID。
"""
from __future__ import annotations

import json
import urllib.request
import urllib.error
import urllib.parse


class IPFSClient:
    """IPFS HTTP API client: content-addressed storage + MFS index."""

    def __init__(self, api_url: str = "http://localhost:5001"):
        self.api_url = api_url.rstrip("/")

    # ------------------------------------------------------------------
    # Core: upload / download / pin
    # ------------------------------------------------------------------

    def upload(self, data: bytes | str) -> str:
        """Upload data to IPFS, return CID.

        Uses /api/v0/add with multipart form encoding.
        """
        if isinstance(data, str):
            data = data.encode("utf-8")

        boundary = b"----IPFSClientBoundary7d44e178"
        body = (
            b"--" + boundary + b"\r\n"
            b'Content-Disposition: form-data; name="file"; filename="data"\r\n'
            b"Content-Type: application/octet-stream\r\n\r\n"
            + data + b"\r\n"
            b"--" + boundary + b"--\r\n"
        )

        req = urllib.request.Request(
            f"{self.api_url}/api/v0/add",
            data=body,
            headers={
                "Content-Type": f"multipart/form-data; boundary={boundary.decode()}",
            },
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=30) as resp:
            result = json.loads(resp.read().decode("utf-8"))
        return result["Hash"]

    def upload_file(self, filepath: str) -> str:
        """Upload file to IPFS, return CID."""
        with open(filepath, "rb") as f:
            return self.upload(f.read())

    def download(self, cid: str) -> bytes:
        """Download data from IPFS by CID."""
        req = urllib.request.Request(
            f"{self.api_url}/api/v0/cat?arg={cid}",
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=60) as resp:
            return resp.read()

    def pin(self, cid: str) -> bool:
        """Pin CID to prevent garbage collection."""
        req = urllib.request.Request(
            f"{self.api_url}/api/v0/pin/add?arg={cid}",
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                result = json.loads(resp.read().decode("utf-8"))
            return cid in result.get("Pins", [])
        except (urllib.error.URLError, urllib.error.HTTPError):
            return False

    def is_available(self) -> bool:
        """Check if IPFS daemon is running."""
        try:
            req = urllib.request.Request(
                f"{self.api_url}/api/v0/id",
                method="POST",
            )
            with urllib.request.urlopen(req, timeout=5) as resp:
                resp.read()
            return True
        except (urllib.error.URLError, OSError):
            return False

    # ------------------------------------------------------------------
    # MFS (Mutable File System): SharedLayer 的索引层
    # ------------------------------------------------------------------
    # IPFS 是内容寻址的，没有"目录"概念。MFS 提供 /files/* API
    # 实现类似文件系统的 mkdir / ls / read / write，用于跟踪 block CID 索引。

    def files_mkdir(self, path: str) -> None:
        """MFS mkdir -p: 创建目录（含中间目录）。"""
        encoded = urllib.parse.quote(path, safe="")
        req = urllib.request.Request(
            f"{self.api_url}/api/v0/files/mkdir?arg={encoded}&parents=true",
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=10) as resp:
            resp.read()

    def files_ls(self, path: str) -> list[dict]:
        """MFS ls: 列出目录中的条目。返回 [{"Name": ..., "Hash": ..., "Size": ...}, ...]。"""
        encoded = urllib.parse.quote(path, safe="")
        req = urllib.request.Request(
            f"{self.api_url}/api/v0/files/ls?arg={encoded}&long=true",
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                result = json.loads(resp.read().decode("utf-8"))
            return result.get("Entries", None) or []
        except urllib.error.HTTPError:
            return []

    def files_write(self, path: str, data: bytes | str, *, create: bool = True, truncate: bool = True) -> None:
        """MFS write: 写入文件内容。默认 create + truncate（覆盖写）。"""
        if isinstance(data, str):
            data = data.encode("utf-8")

        encoded = urllib.parse.quote(path, safe="")
        params = f"arg={encoded}&create={'true' if create else 'false'}&truncate={'true' if truncate else 'false'}"

        boundary = b"----IPFSMFSWriteBoundary9a3c"
        body = (
            b"--" + boundary + b"\r\n"
            b'Content-Disposition: form-data; name="file"; filename="data"\r\n'
            b"Content-Type: application/octet-stream\r\n\r\n"
            + data + b"\r\n"
            b"--" + boundary + b"--\r\n"
        )

        req = urllib.request.Request(
            f"{self.api_url}/api/v0/files/write?{params}",
            data=body,
            headers={
                "Content-Type": f"multipart/form-data; boundary={boundary.decode()}",
            },
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=30) as resp:
            resp.read()

    def files_read(self, path: str) -> bytes:
        """MFS read: 读取文件全部内容。"""
        encoded = urllib.parse.quote(path, safe="")
        req = urllib.request.Request(
            f"{self.api_url}/api/v0/files/read?arg={encoded}",
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=60) as resp:
            return resp.read()

    def files_stat(self, path: str) -> dict | None:
        """MFS stat: 获取文件/目录元信息。不存在时返回 None。"""
        encoded = urllib.parse.quote(path, safe="")
        req = urllib.request.Request(
            f"{self.api_url}/api/v0/files/stat?arg={encoded}",
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=10) as resp:
                return json.loads(resp.read().decode("utf-8"))
        except urllib.error.HTTPError:
            return None

    def files_rm(self, path: str, *, recursive: bool = False) -> None:
        """MFS rm: 删除文件或目录。"""
        encoded = urllib.parse.quote(path, safe="")
        req = urllib.request.Request(
            f"{self.api_url}/api/v0/files/rm?arg={encoded}&recursive={'true' if recursive else 'false'}",
            method="POST",
        )
        with urllib.request.urlopen(req, timeout=10) as resp:
            resp.read()
