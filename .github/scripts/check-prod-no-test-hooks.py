#!/usr/bin/env python3
"""#1640 §2.3 第 2 层：s_incremental 生产构建不含测试钩子（源码机检，不编译）。

扫描 S 写者 bin（`rust/src/bin/s_session_v3`、`rust/src/bin/s_session_v2`）里名字像测试钩子的
`fn` / `static` / `thread_local!`，要求其本身或外层 `mod`/`impl`/`trait` 的 cfg 门含 `test`，或只在
`feature = "s_capture_native_testonly"` 下编译（且门里不含 `s_incremental`）。
已知生产例外列在 ALLOWLIST，新增例外须评审。

用法：python3 .github/scripts/check-prod-no-test-hooks.py [仓根]
exit 0 = 无违规；exit 1 = 有违规（逐条打出 文件:行）。
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOTS = ("rust/src/bin/s_session_v3", "rust/src/bin/s_session_v2")
HOOK_NAME = re.compile(r"(_for_test|testonly|_testonly|inject|_probe_for|trace_event|TEST_HOOK)", re.I)
ITEM = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+|unsafe\s+|async\s+)*"
    r"(?:fn|static(?:\s+mut)?)\s+([A-Za-z_][A-Za-z0-9_]*)"
)
THREAD_LOCAL_ITEM = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?static\s+([A-Za-z_][A-Za-z0-9_]*)\s*:")
BLOCK = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+)?(?:mod|impl|trait)\b")

# 文件相对路径 → 允许的名字（门外既有生产诊断）。
ALLOWLIST: dict[str, set[str]] = {
    # sqlite_pager_probe：门外既有生产诊断（ADR 0042 D8 改名后进生产）。
    "rust/src/bin/s_session_v3/sqlite_pager_probe.rs": set(),
    # #1527 受限故障注入入口：dead_code、无调用方，产物不含；真实 I4/I5 验证票收口时再门控。
    "rust/src/bin/s_session_v3/admission_private/fd_wal.rs": {"inject"},
}


def gate_is_test(cfg_text: str) -> bool:
    """cfg 文本含 test，或只在 testonly feature 下编译。`not(test)` 不算。"""
    text = re.sub(r"not\(\s*test\s*\)", "", cfg_text)
    if re.search(r"\btest\b", text):
        return True
    return "s_capture_native_testonly" in text and "s_incremental" not in text and "not(" not in text


def scan(path: Path, rel: str) -> list[str]:
    if rel in ALLOWLIST and not ALLOWLIST[rel]:
        return []
    allowed = ALLOWLIST.get(rel, set())
    lines = path.read_text(encoding="utf-8").splitlines()
    violations: list[str] = []
    pending: list[str] = []  # 当前条目之前累积的 #[cfg(...)] 文本（cfg_attr 不门控编译，不计）
    attr: list[str] = []  # 跨行属性的累积
    stack: list[list] = []  # [块起始深度, 该 mod/impl 是否测试门控, 是否已见到 `{`]
    depth = 0
    for number, line in enumerate(lines, 1):
        stripped = line.strip()
        if stripped.startswith("//"):
            continue
        if attr or stripped.startswith("#["):
            attr.append(stripped)
            text = " ".join(attr)
            if text.count("[") <= text.count("]"):
                if text.startswith("#[cfg("):
                    pending.append(text)
                attr = []
            continue
        in_test_scope = any(flag for _, flag, _ in stack)
        gated = in_test_scope or any(gate_is_test(c) for c in pending)
        if BLOCK.match(line):
            stack.append([depth, gated, False])
        else:
            item = ITEM.match(line) or THREAD_LOCAL_ITEM.match(line)
            if item and HOOK_NAME.search(item.group(1)) and item.group(1) not in allowed and not gated:
                violations.append(f"{rel}:{number}: {item.group(1)} 未受 cfg(test)/testonly 门控")
        if stripped:
            pending = []
        depth += line.count("{") - line.count("}")
        if stack and depth > stack[-1][0]:
            stack[-1][2] = True
        while stack and (depth < stack[-1][0] or (stack[-1][2] and depth <= stack[-1][0]) or stripped.endswith(";") and not stack[-1][2]):
            stack.pop()
    return violations


def main() -> int:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
    violations: list[str] = []
    for base in ROOTS:
        for path in sorted((root / base).rglob("*.rs")):
            violations.extend(scan(path, path.relative_to(root).as_posix()))
    for violation in violations:
        print(f"::error::{violation}")
    print(f"check-prod-no-test-hooks violations={len(violations)}")
    return 1 if violations else 0


if __name__ == "__main__":
    sys.exit(main())
