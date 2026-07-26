"""real_relations_usable 的稳定分支测试。

OSError 分支因构造不可读文件在 CI/本机上的行为不稳定而不覆盖。
"""

import pytest

from tests.conftest import real_relations_usable


@pytest.mark.unit
def test_real_relations_usable_missing(tmp_path):
    path = tmp_path / "missing.jsonl"

    usable, reason = real_relations_usable(path)

    assert usable is False
    assert "不存在" in reason


@pytest.mark.unit
def test_real_relations_usable_empty(tmp_path):
    path = tmp_path / "empty.jsonl"
    path.write_text("", encoding="utf-8")

    usable, reason = real_relations_usable(path)

    assert usable is False
    assert "为空" in reason


@pytest.mark.unit
def test_real_relations_usable_lfs_pointer(tmp_path):
    path = tmp_path / "lfs-pointer.jsonl"
    lfs_header = "version https://git-lfs.github.com/spec/v1"
    path.write_text(f"{lfs_header}\noid sha256:deadbeef\n", encoding="utf-8")

    usable, reason = real_relations_usable(path)

    assert usable is False
    assert "LFS" in reason
    assert lfs_header in reason


@pytest.mark.unit
def test_real_relations_usable_valid_json_object(tmp_path):
    path = tmp_path / "relations.jsonl"
    path.write_text('{"source": "test"}\n', encoding="utf-8")

    assert real_relations_usable(path) == (True, "")
