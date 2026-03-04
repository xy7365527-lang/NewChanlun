"""区块拓扑 Content-Addressed Storage 操作 API.

block_id = SHA256(full_text)。区块一旦创建不可变。修改 = 新区块。

功能:
  - create_block: 从谱系 .md 文件创建 content-addressed 区块
  - get_block_content: 从区块读取完整文本
  - verify_working_copy: 对比 .md 文件与区块内容
  - rebuild_working_copy: 从区块重建 .md 文件
  - create_revision: 创建修订区块（合法修改），replaces 指向旧区块

谱系位置: 347号——content-addressed block topology
"""
from __future__ import annotations

import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path


def compute_content_hash(text: str) -> str:
    """Compute SHA256 of text content (UTF-8 encoded)."""
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def create_block(
    genealogy_path: Path,
    blocks_dir: Path,
    *,
    metadata: dict | None = None,
) -> str:
    """Read a .md file, create a content-addressed block, return block_id.

    The block_id = SHA256(full_text). The block JSON is saved as
    {blocks_dir}/{block_id}.json.

    Args:
        genealogy_path: Path to the .md file to embed.
        blocks_dir: Path to the blocks/ directory.
        metadata: Optional dict of extra fields to store alongside content
                  (e.g. genealogy_id, title, type, depends_on, etc.).

    Returns:
        The block_id (hex SHA256 of full_text).

    Raises:
        FileNotFoundError: If genealogy_path does not exist.
        ValueError: If full_text is empty.
    """
    if not genealogy_path.is_file():
        raise FileNotFoundError(f"Genealogy file not found: {genealogy_path}")

    full_text = genealogy_path.read_text(encoding="utf-8")
    if not full_text.strip():
        raise ValueError(f"Empty content in {genealogy_path}")

    block_id = compute_content_hash(full_text)
    block_path = blocks_dir / f"{block_id}.json"

    # Idempotent: if block already exists, return immediately
    if block_path.is_file():
        return block_id

    block = {
        "id": block_id,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "content": {
            "full_text": full_text,
        },
    }

    if metadata:
        # Merge metadata into block (but never overwrite id or content.full_text)
        for k, v in metadata.items():
            if k == "id":
                continue
            if k == "content" and isinstance(v, dict):
                for ck, cv in v.items():
                    if ck != "full_text":
                        block["content"][ck] = cv
            else:
                block[k] = v

    blocks_dir.mkdir(parents=True, exist_ok=True)
    block_path.write_text(
        json.dumps(block, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    return block_id


def get_block_content(block_id: str, blocks_dir: Path) -> str:
    """Read full_text from a content-addressed block.

    Args:
        block_id: The SHA256 hex block ID.
        blocks_dir: Path to the blocks/ directory.

    Returns:
        The full_text stored in the block.

    Raises:
        FileNotFoundError: If block does not exist.
        KeyError: If block has no content.full_text field.
    """
    block_path = blocks_dir / f"{block_id}.json"
    if not block_path.is_file():
        raise FileNotFoundError(f"Block not found: {block_id}")

    block = json.loads(block_path.read_text(encoding="utf-8"))
    content = block.get("content", {})
    if not isinstance(content, dict) or "full_text" not in content:
        raise KeyError(f"Block {block_id} has no content.full_text field")

    return content["full_text"]


def verify_working_copy(genealogy_path: Path, blocks_dir: Path) -> bool:
    """Compare a .md file against its content-addressed block.

    Looks up the block whose ID = SHA256(current file content), and
    verifies that block exists and its full_text matches.

    Args:
        genealogy_path: Path to the .md working copy.
        blocks_dir: Path to the blocks/ directory.

    Returns:
        True if the block exists and content matches exactly.
        False if file doesn't exist, block doesn't exist, or content differs.
    """
    if not genealogy_path.is_file():
        return False

    file_text = genealogy_path.read_text(encoding="utf-8")
    expected_id = compute_content_hash(file_text)
    block_path = blocks_dir / f"{expected_id}.json"

    if not block_path.is_file():
        return False

    block = json.loads(block_path.read_text(encoding="utf-8"))
    stored_text = block.get("content", {}).get("full_text", "")
    return stored_text == file_text


def rebuild_working_copy(block_id: str, output_path: Path, blocks_dir: Path) -> None:
    """Reconstruct a .md file from a content-addressed block.

    Args:
        block_id: The SHA256 hex block ID.
        output_path: Where to write the reconstructed .md file.
        blocks_dir: Path to the blocks/ directory.

    Raises:
        FileNotFoundError: If block does not exist.
        KeyError: If block has no content.full_text.
    """
    text = get_block_content(block_id, blocks_dir)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(text, encoding="utf-8")


def create_revision(
    old_block_id: str,
    new_genealogy_path: Path,
    blocks_dir: Path,
    *,
    metadata: dict | None = None,
) -> str:
    """Create a revision block for a legitimate content modification.

    The new block's `replaces` field points to old_block_id.
    The new block_id = SHA256(new full_text).

    Args:
        old_block_id: The block_id being superseded.
        new_genealogy_path: Path to the modified .md file.
        blocks_dir: Path to the blocks/ directory.
        metadata: Optional extra fields.

    Returns:
        The new block_id.

    Raises:
        FileNotFoundError: If old block or new file doesn't exist.
        ValueError: If new content is empty or hashes to same id (no change).
    """
    old_path = blocks_dir / f"{old_block_id}.json"
    if not old_path.is_file():
        raise FileNotFoundError(f"Old block not found: {old_block_id}")

    if not new_genealogy_path.is_file():
        raise FileNotFoundError(
            f"New genealogy file not found: {new_genealogy_path}"
        )

    new_text = new_genealogy_path.read_text(encoding="utf-8")
    if not new_text.strip():
        raise ValueError(f"Empty content in {new_genealogy_path}")

    new_id = compute_content_hash(new_text)
    if new_id == old_block_id:
        raise ValueError(
            f"Content unchanged — new hash equals old block_id: {old_block_id}"
        )

    new_block_path = blocks_dir / f"{new_id}.json"
    if new_block_path.is_file():
        return new_id  # Idempotent

    block = {
        "id": new_id,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "replaces": old_block_id,
        "content": {
            "full_text": new_text,
        },
    }

    if metadata:
        for k, v in metadata.items():
            if k in ("id", "replaces"):
                continue
            if k == "content" and isinstance(v, dict):
                for ck, cv in v.items():
                    if ck != "full_text":
                        block["content"][ck] = cv
            else:
                block[k] = v

    new_block_path.write_text(
        json.dumps(block, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    return new_id
