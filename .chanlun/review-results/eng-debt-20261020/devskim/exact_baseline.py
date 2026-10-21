#!/usr/bin/env python3
"""#1588 devskim 分片第三轮：按用户裁定（2026-10-21）把两个目录现有文件的现有告警按精确键入基线。

键格式与 scripts/devskim_sarif_gate.sh 一致：ruleId|uri（去 file:// 前缀）|startLine。
只输出计数与分布，不输出匹配文本。

用法：
  python3 exact_baseline.py report <sarif> <baseline>
  python3 exact_baseline.py write  <sarif> <baseline> --head SHA --run ID [--self-only]
  python3 exact_baseline.py check  <sarif> <baseline>

write：把「基线外且落在两个目录内」的键排序后追加到 findings 末尾，并追加一条 updates 记录。
基线文件自身的键（键串里的路径文本被规则命中）是唯一允许的目录外例外，同样只追加；
--self-only 只追加这一类（第二遍收敛用）。
check：断言基线外剩余为 0；本轮追加的键全部落在两个目录内或是基线文件自身。
"""
import collections
import json
import re
import sys

BASELINE_PATH = ".github/devskim-baseline.json"
ARCHIVE = re.compile(r"^\.chanlun/review-results/")
FIXTURE = re.compile(r"^s_session/tests/fixtures/.+\.jsonl?$")
ISSUE = 1588


def in_scope(path: str) -> bool:
    return bool(ARCHIVE.match(path) or FIXTURE.match(path))


def sarif_keys(sarif: str) -> set[str]:
    data = json.load(open(sarif, encoding="utf-8"))
    keys = set()
    for run in data.get("runs", []):
        for r in run.get("results", []):
            loc = (r.get("locations") or [{}])[0].get("physicalLocation", {})
            uri = loc.get("artifactLocation", {}).get("uri", "?")
            uri = uri[len("file://"):] if uri.startswith("file://") else uri
            line = loc.get("region", {}).get("startLine", 0)
            keys.add(f"{r.get('ruleId', '?')}|{uri}|{line}")
    return keys


def split(keys):
    groups = collections.defaultdict(list)
    for k in sorted(keys):
        path = k.split("|")[1]
        if in_scope(path):
            groups["scope"].append(k)
        elif path == BASELINE_PATH:
            groups["self"].append(k)
        else:
            groups["outside"].append(k)
    return groups


def show(label, keys):
    rules = collections.Counter(k.split("|")[0] for k in keys)
    tops = collections.Counter(
        "fixtures" if k.split("|")[1].startswith("s_session/") else "review-results"
        for k in keys)
    print(f"{label}: {len(keys)}")
    for r, n in sorted(rules.items()):
        print(f"  rule {r}: {n}")
    for t, n in sorted(tops.items()):
        print(f"  dir {t}: {n}")


def main() -> int:
    mode, sarif, baseline_file = sys.argv[1:4]
    opts = sys.argv[4:]
    baseline = json.load(open(baseline_file, encoding="utf-8"))
    known = set(baseline["findings"])
    found = sarif_keys(sarif)
    new = found - known
    groups = split(new)
    print(f"sarif unique keys: {len(found)}; baseline keys: {len(known)}; outside baseline: {len(new)}")
    show("outside baseline, in the two directories", groups["scope"])
    print(f"outside baseline, the baseline file itself: {len(groups['self'])}")
    print(f"outside baseline, elsewhere: {len(groups['outside'])}")
    for k in groups["outside"]:
        print(f"  elsewhere {k}")
    if mode == "report":
        return 0
    if mode == "check":
        last = baseline["updates"][-1]
        added = baseline["findings"][-last["added_findings"]:]
        bad = [k for k in added if not (in_scope(k.split("|")[1]) or k.split("|")[1] == BASELINE_PATH)]
        print(f"last update added: {len(added)}; keys outside the two directories "
              f"(excluding the baseline file itself): {len(bad)}")
        ok = not new and not bad
        print("CHECK " + ("PASS" if ok else "FAIL"))
        return 0 if ok else 1
    assert mode == "write"
    if groups["outside"]:
        print("refusing to write: findings outside the two directories need separate handling")
        return 1
    head = opts[opts.index("--head") + 1]
    run = int(opts[opts.index("--run") + 1])
    self_only = "--self-only" in opts
    add = groups["self"] if self_only else groups["scope"] + groups["self"]
    if not add:
        print("nothing to add")
        return 0
    baseline["findings"].extend(add)
    rule_counts = dict(sorted(collections.Counter(k.split("|")[0] for k in add).items()))
    baseline["updates"].append({
        "issue": ISSUE,
        "pull_request": 1593,
        "source_head": head,
        "actions_run": run,
        "added_findings": len(add),
        "rule_counts": rule_counts,
        "scope": [".chanlun/review-results/", "s_session/tests/fixtures/ (*.json, *.jsonl)"]
                 + ([BASELINE_PATH] if groups["self"] else []),
        "reason": (
            "基线文件自身因新键路径文本被规则命中的条目，收敛追加。" if self_only else
            "用户 2026-10-21 裁定：不做整目录排除；两个目录现有文件的现有告警按精确键追加，"
            "新写入文件仍被扫描，与 #1474 不凭目录放行一致。键取自 PR 上 DevSkim 的真实扫描产物；"
            "基线文件自身的条目是新键路径文本被规则命中所致。入基线前全量凭据核实 0 命中，证据："
            ".chanlun/review-results/eng-debt-20261020/devskim/DEVSKIM-TRIAGE.md、"
            "classify-excluded-output.txt、credential-scan-output.txt。原有清单顺序不变，新键仅追加。"),
    })
    with open(baseline_file, "w", encoding="utf-8") as fh:
        json.dump(baseline, fh, ensure_ascii=False, indent=2)
        fh.write("\n")
    print(f"appended {len(add)} keys")
    return 0


if __name__ == "__main__":
    sys.exit(main())
