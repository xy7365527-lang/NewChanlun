#!/usr/bin/env python3
"""记录本次实际读取的正本行、冻结 tracker 证据及有限 Python 静态检查。"""
import ast
import hashlib
import importlib.util
import json
import shutil
from pathlib import Path

OUT = Path(__file__).resolve().parent
AUTHOR = OUT.parent / "live-author"
R = Path("/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun")


def save(name, value):
    (OUT / name).write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


selected = {
    ".chanlun/definitions/beichi.md": [315, 317, 338, 465, 466, 563, 564, 565, 567, 568, 569],
    "docs/chanlun/text/blog/061-第61课.md": [12, 26, 28, 56],
    "docs/chanlun/text/blog/033-第33课.md": [12, 24, 26, 52],
    "docs/chanlun/text/blog/079-第79课.md": [12, 52, 58, 60, 64, 130],
    "docs/chanlun/text/blog/065-第65课.md": [12, 42, 44, 48, 50],
    "docs/chanlun/text/blog/071-第71课.md": [12, 22, 24, 26, 30, 32, 34, 40, 42],
    "rust/src/theta_v0/parser/segment.rs": list(range(789, 842)),
}
entries = []
for relative, numbers in selected.items():
    path = R / relative
    lines = path.read_text().splitlines()
    is_blog = "/blog/" in relative
    boundary = next((i + 1 for i, line in enumerate(lines) if "↑正文" in line), None) if is_blog else None
    for number in numbers:
        text = lines[number - 1]
        positions = [text.find(token) for token in ("(娇注：", "（娇注：", "(注：", "（注：", "(注:") if token in text]
        index = min(positions) if positions else len(text)
        entries.append({
            "path": str(path), "sha256": digest(path), "line": number, "text": text,
            "kind": "original-blog" if is_blog else "project-doctrine" if "beichi" in relative else "production-code-evidence",
            "body_boundary": boundary, "before_body_boundary": number < boundary if boundary else None,
            "author_header": lines[11] if is_blog else None,
            "line_starts_note": text.startswith(("（注", "(注", "（娇注", "(娇注")),
            "inline_note_start": index if index != len(text) else None,
            "used_author_text": text[:index] if is_blog else None,
        })
save("source-reading-evidence.json", entries)

tracker = []
for number in (872, 974, 975, 978, 979, 987, 989, 990):
    path = AUTHOR / f"issue-{number}.json"
    issue = json.loads(path.read_text())
    matching = []
    for comment in issue["comments"]:
        snippets = [line for line in comment["body"].splitlines() if any(token in line for token in
                    ("毕业", "#974", "#975", "#978", "#979", "#987", "#989", "#990", "首/末", "首笔", "末笔", "当前", "strokes", "残段"))]
        if snippets:
            matching.append({"url": comment["url"], "exact_lines": snippets})
    tracker.append({"issue": number, "snapshot_path": str(path), "sha256": digest(path),
                    "state_in_frozen_snapshot": issue["state"], "title": issue["title"], "url": issue["url"],
                    "selected_evidence": matching})
save("tracker-reading-evidence.json", {
    "live_query_performed": False, "scope": "审查提供的冻结 issue 快照，不声称实时 tracker 状态",
    "issues": tracker,
    "conclusion": "冻结链已裁首末笔差并记录989/990实施。给定Stroke切片的首末反查未定义本次R_W未确认残段的StructuralPenNow资格。",
})

script = AUTHOR / "check_live.py"
source = script.read_text()
ast.parse(source, filename=str(script))
compile(source, str(script), "exec", dont_inherit=True)
save("python-static-check.json", {
    "script": str(script), "script_sha256": digest(script), "ast_parse": "pass", "compile_without_write": "pass",
    "tools": {name: {"on_path": shutil.which(name), "python_module": importlib.util.find_spec(name) is not None}
              for name in ("ruff", "mypy", "pylint", "black")},
    "git_diff_py": "empty at review start in R", "lint_or_type_pass_claimed": False,
    "scope": "独立审阅冻结研究脚本；未将缺失类型标注或紧凑排版扩为生产改码任务",
})
save("causality-audit.json", {
    "read_isolation": "fail-for-frozen-author-package",
    "read_isolation_witness": {"code_line": 40, "reference_top_level": ["groups", "prefixes", "strokes"],
                                "online_seal_line": 212, "false_field": "final_pens_read:false"},
    "hashing_final_files_before_seal": "also occurs at line38 via INPUTS; hash streaming is not semantic use and must be labelled separately",
    "online_output_data_dependency": "accepted conditionally on frozen finite template and accepted per-prefix inputs",
    "argument": [
        "At step t, prices/times are appended from observation t only; every pen endpoint and anchor raw index is <=t.",
        "New raw certificates use current stable pens plus observed prices; stored certificates were first validated at earlier or current steps.",
        "Local c start is third K certificate end. Parent c start is third whole certificate end. No final_end-minus-length operation exists.",
        "L uses current selected anchor, current price, fixed first stable pen, and event time differences. Joint trigger uses all conditions at the same t.",
        "Independent function reproduces every row without reference.stdout or final object/pens/parent until its online seal.",
        "Prefix-only replay deletes groups/strokes from delivered reference text and preserves eight key output hashes. This is a finite control, not a theorem for every parser/history.",
    ],
    "general_algorithm_claim": "not accepted; bootstrap and four 5x5 groups are supplied template choices",
    "source_role_and_completed": None,
})
