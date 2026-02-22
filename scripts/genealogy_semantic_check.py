#!/usr/bin/env python3
"""genealogy_semantic_check.py — 谱系语义一致性检查

输入：新谱系文件路径
输出：JSON 格式的兼容性报告

检查项：
  a) negates/negated_by 一致性
  b) 状态转换合法性
  c) 前置谱系结论冲突（简单正则）
  d) 下游推论引用检查
"""

from __future__ import annotations

import json
import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Optional


# ── 数据结构 ───────────────────────────────────────────────

@dataclass(frozen=True)
class CheckResult:
    check: str
    status: str  # "pass" | "warn" | "fail"
    detail: str


@dataclass(frozen=True)
class SemanticReport:
    file: str
    checks: tuple[CheckResult, ...]
    overall: str

    def to_dict(self) -> dict:
        return {
            "file": self.file,
            "checks": [
                {"check": c.check, "status": c.status, "detail": c.detail}
                for c in self.checks
            ],
            "overall": self.overall,
        }


# ── 解析工具 ───────────────────────────────────────────────

def parse_yaml_frontmatter(content: str) -> dict:
    """从 markdown 文件中提取 YAML frontmatter（--- 分隔）。

    支持两种 YAML 数组格式：
    - 行内格式：key: ["a", "b"]
    - 多行格式：
        key:
          - 'a'
          - 'b'

    使用简单正则解析，避免引入 PyYAML 依赖。
    """
    m = re.match(r"^---\s*\n(.*?)\n---", content, re.DOTALL)
    if not m:
        return {}
    raw = m.group(1)
    lines = raw.splitlines()
    result: dict = {}
    current_key: Optional[str] = None
    current_list: Optional[list[str]] = None

    def _flush() -> None:
        nonlocal current_key, current_list
        if current_key is not None and current_list is not None:
            result[current_key] = current_list
        current_key = None
        current_list = None

    for line in lines:
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue

        # 多行列表项：  - 'value' 或  - "value" 或  - value
        list_item = re.match(r'^\s+-\s+(.*)', line)
        if list_item and current_key is not None:
            val = list_item.group(1).strip().strip('"').strip("'")
            # 去掉行尾注释
            val = re.sub(r'\s+#.*$', '', val).strip()
            if current_list is None:
                current_list = []
            current_list.append(val)
            continue

        # 新的 key: value 行
        kv = re.match(r'^(\w[\w_]*)\s*:\s*(.*)', stripped)
        if not kv:
            continue

        _flush()

        key = kv.group(1)
        val = kv.group(2).strip()

        if not val:
            # 值为空——可能是多行列表的开始
            current_key = key
            current_list = []
            continue

        # 行内数组 ["a", "b"]
        arr_match = re.match(r'^\[(.*)?\]', val)
        if arr_match:
            inner = arr_match.group(1) or ""
            items = [
                s.strip().strip('"').strip("'")
                for s in inner.split(",")
                if s.strip() and s.strip() not in ('""', "''")
            ]
            result[key] = items
        else:
            # 标量值，去掉行尾注释
            val_no_comment = re.sub(r'\s+#.*$', '', val)
            result[key] = val_no_comment.strip('"').strip("'")

    _flush()
    return result


def find_genealogy_root(file_path: str) -> Optional[Path]:
    """从文件路径向上查找 .chanlun/genealogy/ 根目录。"""
    p = Path(file_path).resolve()
    # 遍历所有父目录
    for parent in p.parents:
        candidate = parent / ".chanlun" / "genealogy"
        if candidate.is_dir():
            return candidate
    return None


def resolve_genealogy_file(gen_root: Path, ref_id: str) -> Optional[Path]:
    """根据谱系编号查找对应文件（在 settled/ 和 pending/ 中搜索）。"""
    for subdir in ("settled", "pending"):
        d = gen_root / subdir
        if not d.is_dir():
            continue
        # 文件名以 ref_id 开头（如 "086" 匹配 "086-island-repair-strategy-decisions.md"）
        for f in d.iterdir():
            if f.is_file() and f.name.startswith(ref_id) and f.suffix == ".md":
                return f
    return None


def load_genealogy(gen_root: Path, ref_id: str) -> Optional[tuple[dict, str]]:
    """加载谱系文件，返回 (frontmatter, content) 或 None。"""
    f = resolve_genealogy_file(gen_root, ref_id)
    if f is None:
        return None
    content = f.read_text(encoding="utf-8")
    fm = parse_yaml_frontmatter(content)
    return fm, content


# ── 检查逻辑 ───────────────────────────────────────────────

def check_negates_consistency(
    fm: dict, gen_root: Path
) -> CheckResult:
    """a) negates/negated_by 一致性检查。

    如果新谱系声明 negates X，检查 X 是否存在且 X 的 negated_by
    是否指回新谱系（或为空待填）。
    """
    new_id = fm.get("id", "")
    negates_list = fm.get("negates", [])
    if not isinstance(negates_list, list):
        negates_list = [negates_list] if negates_list else []

    if not negates_list:
        return CheckResult(
            check="negates_consistency",
            status="pass",
            detail="无 negates 声明，跳过",
        )

    issues: list[str] = []
    for target_id in negates_list:
        if not target_id:
            continue
        loaded = load_genealogy(gen_root, target_id)
        if loaded is None:
            issues.append(f"{target_id}: 谱系文件不存在")
            continue
        target_fm, _ = loaded
        target_negated_by = target_fm.get("negated_by", [])
        if not isinstance(target_negated_by, list):
            target_negated_by = [target_negated_by] if target_negated_by else []

        if new_id in target_negated_by:
            # 一致
            pass
        elif not target_negated_by:
            issues.append(
                f"{target_id}: negated_by 为空，应包含 {new_id}"
            )
        else:
            issues.append(
                f"{target_id}: negated_by={target_negated_by}，不包含 {new_id}"
            )

    if not issues:
        return CheckResult(
            check="negates_consistency",
            status="pass",
            detail="所有 negates 引用的谱系 negated_by 一致",
        )

    return CheckResult(
        check="negates_consistency",
        status="warn",
        detail="; ".join(issues),
    )


def check_state_transition(
    fm: dict, gen_root: Path
) -> CheckResult:
    """b) 状态转换合法性检查。

    如果新谱系声明的前置（depends_on）中有谱系的状态为 pending/生成态，
    而新谱系自身状态为 settled/已结算，这可能有问题。
    从 settled → pending 需要标注原因。
    """
    new_status = fm.get("status", "")
    depends = fm.get("depends_on", [])
    if not isinstance(depends, list):
        depends = [depends] if depends else []

    issues: list[str] = []
    for dep_id in depends:
        if not dep_id:
            continue
        loaded = load_genealogy(gen_root, dep_id)
        if loaded is None:
            # 前置不存在由字段验证处理，这里不重复
            continue
        dep_fm, _ = loaded
        dep_status = dep_fm.get("status", "")

        # 新谱系已结算，但前置还是生成态/pending
        settled_values = {"已结算", "settled"}
        pending_values = {"生成态", "pending", "深度张力待审"}
        if new_status in settled_values and dep_status in pending_values:
            issues.append(
                f"前置 {dep_id} 状态为 '{dep_status}'（未结算），"
                f"但新谱系状态为 '{new_status}'（已结算）"
            )

    if not issues:
        return CheckResult(
            check="state_transition",
            status="pass",
            detail="状态转换合法",
        )

    return CheckResult(
        check="state_transition",
        status="warn",
        detail="; ".join(issues),
    )


def check_conclusion_conflict(
    fm: dict, content: str, gen_root: Path
) -> CheckResult:
    """c) 前置谱系结论冲突检查。

    在前置谱系的命题/结论中搜索 "X是Y" 模式，
    然后在新谱系中搜索 "X不是Y" 或 "X非Y" 模式（简单正则匹配）。
    """
    depends = fm.get("depends_on", [])
    if not isinstance(depends, list):
        depends = [depends] if depends else []

    if not depends:
        return CheckResult(
            check="conclusion_conflict",
            status="pass",
            detail="无前置谱系，跳过",
        )

    # 从新谱系中提取否定断言模式：X不是Y / X非Y
    negation_patterns = re.findall(
        r'[「"*]*(\S{2,10})[」"*]*\s*(?:不是|并非|非|不再是)\s*[「"*]*(\S{2,10})',
        content,
    )

    if not negation_patterns:
        return CheckResult(
            check="conclusion_conflict",
            status="pass",
            detail="新谱系无否定断言模式",
        )

    conflicts: list[str] = []
    for dep_id in depends:
        if not dep_id:
            continue
        loaded = load_genealogy(gen_root, dep_id)
        if loaded is None:
            continue
        _, dep_content = loaded

        for subject, obj in negation_patterns:
            # 在前置谱系中查找肯定断言 "X是Y"
            affirm_pattern = re.escape(subject) + r'\s*(?:是|为|=)\s*' + re.escape(obj)
            if re.search(affirm_pattern, dep_content):
                conflicts.append(
                    f"前置 {dep_id} 断言 '{subject}是{obj}'，"
                    f"新谱系断言 '{subject}不是{obj}'"
                )

    if not conflicts:
        return CheckResult(
            check="conclusion_conflict",
            status="pass",
            detail="无结论冲突",
        )

    return CheckResult(
        check="conclusion_conflict",
        status="warn",
        detail="; ".join(conflicts),
    )


def check_downstream_reference(
    fm: dict, content: str, gen_root: Path
) -> CheckResult:
    """d) 下游推论引用检查。

    如果前置谱系声明了下游推论（## 下游推论 / ## 影响 段落），
    检查新谱系是否引用了这些推论（通过谱系编号引用关系）。
    """
    depends = fm.get("depends_on", [])
    if not isinstance(depends, list):
        depends = [depends] if depends else []

    if not depends:
        return CheckResult(
            check="downstream_reference",
            status="pass",
            detail="无前置谱系，跳过",
        )

    # 从新谱系内容中提取所有谱系编号引用
    new_refs = set(re.findall(r'\b(\d{3})\b(?:号|a|b)?', content))

    unreferenced: list[str] = []
    for dep_id in depends:
        if not dep_id:
            continue
        loaded = load_genealogy(gen_root, dep_id)
        if loaded is None:
            continue
        _, dep_content = loaded

        # 提取前置谱系的下游推论段落
        downstream_section = re.search(
            r'##\s*(?:下游推论|影响|downstream)\s*\n(.*?)(?=\n##|\Z)',
            dep_content,
            re.DOTALL | re.IGNORECASE,
        )
        if not downstream_section:
            continue

        downstream_text = downstream_section.group(1)
        # 从下游推论中提取提到的谱系编号
        downstream_refs = set(re.findall(r'\b(\d{3})\b(?:号|a|b)?', downstream_text))
        # 获取新谱系的 id
        new_id = fm.get("id", "")
        # 如果前置谱系的下游推论提到了新谱系的 id，检查新谱系是否也引用了前置
        if new_id in downstream_refs and dep_id not in new_refs:
            unreferenced.append(
                f"前置 {dep_id} 的下游推论提到 {new_id}号，"
                f"但新谱系未引用 {dep_id}"
            )

    if not unreferenced:
        return CheckResult(
            check="downstream_reference",
            status="pass",
            detail="下游推论引用一致",
        )

    return CheckResult(
        check="downstream_reference",
        status="warn",
        detail="; ".join(unreferenced),
    )


# ── 主流程 ─────────────────────────────────────────────────

def run_semantic_check(
    file_path: str, content: Optional[str] = None
) -> SemanticReport:
    """对指定谱系文件执行全部语义检查，返回报告。

    Args:
        file_path: 谱系文件路径（用于定位 genealogy 根目录和文件名）
        content: 文件内容。为 None 时从磁盘读取。
    """
    path = Path(file_path).resolve()
    if content is None:
        content = path.read_text(encoding="utf-8")
    fm = parse_yaml_frontmatter(content)

    # 定位 genealogy 根目录：先从文件路径向上查找，
    # 再从 cwd 向上查找（文件可能尚未写入磁盘）
    gen_root = find_genealogy_root(str(path))
    if gen_root is None:
        gen_root = find_genealogy_root(os.getcwd())
    if gen_root is None:
        return SemanticReport(
            file=path.name,
            checks=(
                CheckResult(
                    check="setup",
                    status="fail",
                    detail="无法定位 .chanlun/genealogy/ 目录",
                ),
            ),
            overall="fail",
        )

    results = (
        check_negates_consistency(fm, gen_root),
        check_state_transition(fm, gen_root),
        check_conclusion_conflict(fm, content, gen_root),
        check_downstream_reference(fm, content, gen_root),
    )

    statuses = [r.status for r in results]
    if "fail" in statuses:
        overall = "fail"
    elif "warn" in statuses:
        overall = "warn"
    else:
        overall = "pass"

    return SemanticReport(file=path.name, checks=results, overall=overall)


def main() -> None:
    if len(sys.argv) < 2:
        print(
            json.dumps(
                {"error": "用法: genealogy_semantic_check.py <谱系文件路径> [--content-stdin]"},
                ensure_ascii=False,
            )
        )
        sys.exit(1)

    file_path = sys.argv[1]
    use_stdin = "--content-stdin" in sys.argv

    content: Optional[str] = None
    if use_stdin:
        content = sys.stdin.read()
    elif not os.path.isfile(file_path):
        print(
            json.dumps(
                {"error": f"文件不存在: {file_path}"},
                ensure_ascii=False,
            )
        )
        sys.exit(1)

    report = run_semantic_check(file_path, content)
    print(json.dumps(report.to_dict(), ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
