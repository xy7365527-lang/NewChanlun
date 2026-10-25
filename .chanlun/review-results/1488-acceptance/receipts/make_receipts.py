#!/usr/bin/env python3
"""0077 P30：把关票证据引用的远端日志摘成仓内收据（关键行 + sha256），并改写引用。

用法: receipts.py <日志清单> <引用文档...>
- 每个日志 → receipts/<run-id>.md：sha256、字节数、头行（ref/sha/cmd，截断）、关键行（test result、
  FAILED、panicked 及其后 2 行、R14_/R12_/P50_ 前缀行、EXIT），并列出引用它的文档。
- INDEX.md：run-id → sha256 → 引用文档 清单。
- 引用文档里 `/tmp/nc-remote/<id>.log` 改为仓内相对路径，并在文末追加映射说明（只追加，不删其他内容）。
"""
import hashlib, os, re, sys

REPO = "/Volumes/AgentStorage/issue1488-ticket-manager-20261018/budget-handoff-fix-v2-checkout"
REL = ".chanlun/review-results/1488-acceptance/receipts"
OUT = os.path.join(REPO, REL)
KEY = re.compile(r"^(test result|EXIT=|R1[24]_|P50_|LOCKS|WIDE|FAILED_N|MISSING_DEP|NOT_OK|ONE rc|CHECK|BUILD)|^test .* \.\.\. (ok|FAILED|ignored)$|panicked at|^error(\[|:)")
PAT = re.compile(r"/tmp/nc-remote/(\d{8}-\d{6}-[0-9a-f]{10})\.log")
CAP = 400


def receipt(path, refs):
    data = open(path, "rb").read()
    rid = os.path.basename(path)[:-4]
    lines = data.decode("utf-8", "replace").splitlines()
    head = lines[0][:400] if lines else ""
    keep, i = [], 0
    while i < len(lines):
        l = lines[i]
        if KEY.search(l):
            keep.append(f"{i + 1}: {l[:400]}")
            if "panicked at" in l:
                for j in (1, 2):
                    if i + j < len(lines):
                        keep.append(f"{i + 1 + j}: {lines[i + j][:400]}")
        i += 1
    dropped = max(0, len(keep) - CAP)
    if dropped:
        keep = keep[: CAP // 2] + [f"…（中间省略 {dropped} 行关键行，原日志按 sha256 核对）…"] + keep[-CAP // 2 :]
    sha = hashlib.sha256(data).hexdigest()
    body = [
        f"# 收据 {rid}",
        "",
        f"- 原日志：`/tmp/nc-remote/{rid}.log`（nc-remote 拷回本机；远端原件 WSL `~/nc-remote/logs/{rid}.log`）",
        f"- sha256：`{sha}`",
        f"- 字节数：{len(data)}；行数：{len(lines)}",
        f"- 引用文档：{', '.join(sorted(refs)) or '（无）'}",
        f"- 头行：`{head.replace('`', "'")}`",
        "",
        "## 关键行（行号: 原文）",
        "",
        "```",
        *keep,
        "```",
        "",
    ]
    open(os.path.join(OUT, rid + ".md"), "w").write("\n".join(body))
    return rid, sha, len(data)


def main():
    logs = [l.strip() for l in open(sys.argv[1]) if l.strip()]
    docs = sys.argv[2:]
    refs = {}
    for d in docs:
        for rid in PAT.findall(open(d).read()):
            refs.setdefault(rid, set()).add(os.path.relpath(d, os.path.dirname(REPO)))
    os.makedirs(OUT, exist_ok=True)
    rows = []
    for path in logs:
        rid = os.path.basename(path)[:-4]
        rows.append(receipt(path, refs.get(rid, set())) + (sorted(refs.get(rid, set())),))
    idx = ["# #1488 关票证据收据索引（0077 P30）", "",
           "原远端日志只在本机 /tmp 与 WSL 上；这里按 run-id 入仓关键行与整文件 sha256，供复核时比对。", "",
           "| run-id | sha256 | 字节 | 引用文档（相对 issue1488-ticket-manager-20261018/） |", "|---|---|---|---|"]
    for rid, sha, size, r in rows:
        idx.append(f"| [{rid}]({rid}.md) | `{sha}` | {size} | {'<br>'.join(r)} |")
    open(os.path.join(OUT, "INDEX.md"), "w").write("\n".join(idx) + "\n")
    have = {r[0] for r in rows}
    for d in docs:
        s = open(d).read()
        hit = sorted(set(PAT.findall(s)))
        missing = [h for h in hit if h not in have]
        if missing:
            sys.exit(f"{d}: 缺收据 {missing}")
        if not hit:
            continue
        s = PAT.sub(lambda m: f"{REL}/{m.group(1)}.md", s)
        s += ("\n\n## 证据引用迁移（0077 P30，2026-10-21 追加）\n\n"
              f"本文原引用的 {len(hit)} 份 `/tmp/nc-remote/<run-id>.log` 已改为仓内收据 `{REL}/<run-id>.md`"
              "（相对工位 checkout 根；关键行 + 整文件 sha256，原日志名与 run-id 一一对应）。索引见 "
              f"`{REL}/INDEX.md`。\n")
        open(d, "w").write(s)
        print(f"{d}: {len(hit)} 处 run-id 已改写")
    print(f"receipts={len(rows)} out={OUT}")


main()
