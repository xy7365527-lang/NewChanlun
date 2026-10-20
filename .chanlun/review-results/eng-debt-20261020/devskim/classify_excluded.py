#!/usr/bin/env python3
"""#1588 devskim 分片：对拟 ignore-glob 排除目录内的全部 DevSkim 结果做逐条形状分类。

只输出计数与 rule|path|line，不输出任何匹配值。
用法：python3 classify_excluded.py <SARIF> <仓库根>
"""
import collections
import json
import re
import sys

EXCLUDED = re.compile(r"^(\.chanlun/review-results/|s_session/tests/fixtures/)")
# 已知凭据前缀：命中即人工复核（GitHub/Anthropic/OpenAI/AWS/Slack/私钥头等）
CRED = re.compile(
    r"(ghp_|gho_|ghu_|ghs_|ghr_|github_pat_|sk-ant-|sk-proj-|\bsk-[A-Za-z0-9]{20}|AKIA[0-9A-Z]{16}|"
    r"ASIA[0-9A-Z]{16}|xox[abprs]-|-----BEGIN [A-Z ]*PRIVATE KEY|AIza[0-9A-Za-z_-]{35}|glpat-)"
)


def shape(tok: str) -> str:
    core = tok.strip().strip("\"'`,:;=()[]{}<> ")
    if re.fullmatch(r"[0-9a-f]+", core):
        return f"hex{len(core)}"
    if re.fullmatch(r"sha256:[0-9a-f]{64}", core):
        return "sha256-prefixed"
    return "other"


def main() -> None:
    sarif, root = sys.argv[1], sys.argv[2]
    data = json.load(open(sarif))
    cache: dict[str, str] = {}
    by_rule = collections.Counter()
    by_shape = collections.Counter()
    other = []
    cred_hits = []
    for run in data["runs"]:
        for r in run.get("results", []):
            loc = r["locations"][0]["physicalLocation"]
            uri = loc["artifactLocation"]["uri"].removeprefix("file://")
            if not EXCLUDED.match(uri):
                continue
            rule = r.get("ruleId", "?")
            by_rule[rule] += 1
            reg = loc["region"]
            if uri not in cache:
                try:
                    cache[uri] = open(f"{root}/{uri}", encoding="utf-8", errors="replace").read()
                except OSError:
                    cache[uri] = ""
            text = cache[uri]
            off, ln = reg.get("charOffset"), reg.get("charLength")
            tok = text[off:off + ln] if off is not None and ln else reg.get("snippet", {}).get("text", "")
            key = f"{rule}|{uri}|{reg.get('startLine', 0)}"
            if CRED.search(tok):
                cred_hits.append(key)
            if rule == "DS173237":
                s = shape(tok)
                by_shape[s] += 1
                if s == "other":
                    other.append(key)
    # 文件级：被排除目录内全部文件扫已知凭据前缀（不限 DevSkim 命中）
    print("== 按规则计数（排除目录内）")
    for k, v in by_rule.most_common():
        print(f"{v:7d} {k}")
    print("== DS173237 匹配文本形状")
    for k, v in by_shape.most_common():
        print(f"{v:7d} {k}")
    print(f"== DS173237 非十六进制形状（仅键）: {len(other)}")
    for k in other:
        print("  " + k)
    print(f"== 已知凭据前缀命中（DevSkim 匹配文本内）: {len(cred_hits)}")
    for k in cred_hits:
        print("  " + k)


if __name__ == "__main__":
    main()
