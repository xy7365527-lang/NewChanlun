"""IPFS HTTP API client for block upload/download/pin.

Works with both public and private IPFS networks — transparent to client.
Uses IPFS HTTP API (default: localhost:5001).
Pure Python, stdlib only (urllib).
"""
from __future__ import annotations

import io
import json
import urllib.request
import urllib.error


class IPFSClient:
    """Minimal IPFS HTTP API client using urllib."""

    def __init__(self, api_url: str = "http://localhost:5001"):
        self.api_url = api_url.rstrip("/")

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
