#!/usr/bin/env python3
"""#1588 devskim 分片：排除前对拟排除目录做全量凭据形状扫描（非抽样）。

覆盖面：git 跟踪的全部文件原文；zip 成员解压后内容；JSON 内 base64(+xz/gzip) 内嵌附件解码后内容。
只输出「路径 / 成员 / 模式组 / 命中数」，绝不输出命中值。
用法：python3 scan_credentials.py <仓库根> [目录 ...]
"""
import base64
import collections
import gzip
import io
import lzma
import re
import subprocess
import sys
import zipfile

# 强特征：固定前缀的真实凭据格式，命中即需人工复核
STRONG = {
    "github_token": re.compile(rb"\b(ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{36}\b|\bgithub_pat_[A-Za-z0-9_]{60,}"),
    "anthropic_key": re.compile(rb"\bsk-ant-[A-Za-z0-9_-]{20,}"),
    "openai_key": re.compile(rb"\bsk-(proj-)?[A-Za-z0-9_-]{32,}"),
    "aws_access_key": re.compile(rb"\b(AKIA|ASIA)[0-9A-Z]{16}\b"),
    "slack_token": re.compile(rb"\bxox[abprs]-[A-Za-z0-9-]{10,}"),
    "private_key_pem": re.compile(rb"-----BEGIN [A-Z ]*PRIVATE KEY-----"),
    "google_api_key": re.compile(rb"\bAIza[0-9A-Za-z_-]{35}\b"),
    "gitlab_pat": re.compile(rb"\bglpat-[A-Za-z0-9_-]{20}"),
    "jwt": re.compile(rb"\beyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}"),
}
# 弱特征：键名=值形态，需看上下文判定（多为占位/字段名/哈希）
WEAK = {
    "kv_secret": re.compile(
        rb"(?i)\b(api[_-]?key|secret[_-]?key|client[_-]?secret|password|passwd|access[_-]?token|auth[_-]?token|bearer)"
        rb"[\"']?\s*[:=]\s*[\"']?(?!\$\{|<|\*{3}|x{6})[A-Za-z0-9_\-+/.]{16,}"
    ),
}
B64_FIELD = re.compile(rb'"data"\s*:\s*"([A-Za-z0-9+/=]{64,})"')


def decode_blob(raw: bytes) -> bytes | None:
    try:
        blob = base64.b64decode(raw, validate=True)
    except Exception:
        return None
    for opener in (lzma.decompress, gzip.decompress):
        try:
            return opener(blob)
        except Exception:
            pass
    return blob


def scan(buf: bytes, label: str, hits: collections.Counter, weak: collections.Counter) -> None:
    for name, pat in STRONG.items():
        n = len(pat.findall(buf))
        if n:
            hits[(label, name)] += n
    for name, pat in WEAK.items():
        n = len(pat.findall(buf))
        if n:
            weak[(label, name)] += n


def main() -> None:
    root = sys.argv[1]
    dirs = sys.argv[2:] or [".chanlun/review-results", "s_session/tests/fixtures"]
    files = subprocess.run(["git", "-C", root, "ls-files", "-z", "--", *dirs],
                           check=True, capture_output=True).stdout.split(b"\0")
    hits: collections.Counter = collections.Counter()
    weak: collections.Counter = collections.Counter()
    stats = collections.Counter()
    for f in filter(None, files):
        path = f.decode()
        data = open(f"{root}/{path}", "rb").read()
        stats["files"] += 1
        # 原文扫描时，把 base64 附件字段整体剔除（避免随机 base64 偶合前缀），附件另行解码扫描
        stripped = B64_FIELD.sub(b'"data":"<blob>"', data)
        scan(stripped, path, hits, weak)
        for m in B64_FIELD.finditer(data):
            dec = decode_blob(m.group(1))
            stats["b64_fields"] += 1
            if dec is not None:
                stats["b64_decoded"] += 1
                scan(dec, f"{path}#data@{m.start()}", hits, weak)
        if path.endswith(".zip"):
            with zipfile.ZipFile(io.BytesIO(data)) as z:
                for info in z.infolist():
                    stats["zip_members"] += 1
                    scan(z.read(info), f"{path}!{info.filename}", hits, weak)
    print("== 覆盖统计")
    for k, v in stats.items():
        print(f"  {k}: {v}")
    print(f"== 强特征命中（固定前缀凭据格式）: {sum(hits.values())}")
    for (label, name), n in sorted(hits.items()):
        print(f"  {n:5d} {name} {label}")
    print(f"== 弱特征命中（键名=值，需人工看上下文）: {sum(weak.values())}")
    for (label, name), n in sorted(weak.items()):
        print(f"  {n:5d} {name} {label}")


if __name__ == "__main__":
    main()
