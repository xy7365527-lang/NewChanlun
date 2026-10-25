#!/usr/bin/env python3
"""#1488：在原始 SARIF 上精确识别经审误报，不修改原件或基线。"""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import sys


def identity(result):
    """绑定完整 result，包含全部位置、片段与规则；不按旧三元键豁免。"""
    return tuple(hashlib.sha256(json.dumps(result, sort_keys=True, ensure_ascii=False,
                                         separators=(",", ":")).encode()).digest())


def digest_bytes(value):
    if not isinstance(value, list) or len(value) != 32 or any(
        type(x) is not int or not 0 <= x <= 255 for x in value
    ):
        raise ValueError("身份必须为 32 个 SHA256 字节")
    return tuple(value)


def load_allowlist(path, root):
    data = json.loads(path.read_text())
    if set(data) != {"schema", "issue", "files"} or data["schema"] != "devskim-reviewed-false-positives/v1":
        raise ValueError("误报清单 schema 不符")
    if data["issue"] != 1488 or not isinstance(data["files"], list):
        raise ValueError("误报清单归属或文件列表不符")
    allowed = set()
    seen = set()
    root = root.resolve(strict=True)
    for entry in data["files"]:
        if set(entry) != {"path", "sha256_bytes", "results"}:
            raise ValueError("文件清单字段不符")
        name = entry["path"]
        if not isinstance(name, str) or not name or "\\" in name:
            raise ValueError("文件路径无效")
        pure = PurePosixPath(name)
        if pure.is_absolute() or ".." in pure.parts or str(pure) != name or name in seen:
            raise ValueError("文件路径越界、非规范或重复")
        seen.add(name)
        target = (root / name).resolve(strict=True)
        if not target.is_relative_to(root) or not target.is_file():
            raise ValueError("文件路径越界或不是普通文件")
        if tuple(hashlib.sha256(target.read_bytes()).digest()) != digest_bytes(entry["sha256_bytes"]):
            raise ValueError(f"误报原件身份变化：{name}")
        if not isinstance(entry["results"], list) or not entry["results"]:
            raise ValueError("误报结果列表为空或无效")
        for item in entry["results"]:
            if set(item) != {"rule", "identity_bytes"} or not isinstance(item["rule"], str):
                raise ValueError("误报结果字段不符")
            key = (name, item["rule"], digest_bytes(item["identity_bytes"]))
            if key in allowed:
                raise ValueError("重复误报结果")
            allowed.add(key)
    return allowed


def classify(sarif, allowed):
    keys = set()
    removed = 0
    remaining = 0
    for run in sarif["runs"]:
        for result in run["results"]:
            loc = result.get("locations", [{}])[0].get("physicalLocation", {})
            name = loc.get("artifactLocation", {}).get("uri", "?")
            rule = result.get("ruleId", "?")
            if (name, rule, identity(result)) in allowed:
                removed += 1
            else:
                remaining += 1
                name = name.removeprefix("file://")
                line = loc.get("region", {}).get("startLine", 0)
                keys.add(f"{rule}|{name}|{line}")
    return {"keys": sorted(keys), "excluded_results": removed, "remaining_results": remaining}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--sarif", type=Path, required=True)
    parser.add_argument("--false-positives", type=Path, required=True)
    parser.add_argument("--source-root", type=Path, required=True)
    args = parser.parse_args()
    try:
        allowed = load_allowlist(args.false_positives, args.source_root)
        print(json.dumps(classify(json.loads(args.sarif.read_text()), allowed)))
    except (OSError, ValueError, TypeError, KeyError, IndexError) as error:
        print(f"经审误报清单验证失败：{error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
