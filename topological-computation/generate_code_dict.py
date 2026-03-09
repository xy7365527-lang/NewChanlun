"""generate_code_dict.py — 从 AST 自动生成代码辞典 JSONL。

扫描 topological-computation/ 下的 Python 源文件，提取：
  - 模块：模块名 + docstring
  - 类：类名 + 基类 + docstring + 方法列表
  - 函数/方法：函数名 + 签名 + docstring
  - import 关系、调用关系

输出格式与现有 dict_*.jsonl 兼容：
  {"term": str, "domain": "code_project", "definition": str,
   "synonyms": [str], "contrasts": [str],
   "connective_patterns": [str],
   "concept_ids": [str]}

concept_ids 字段是代码辞典特有的——存放 K_active 中对应顶点的 ID，
供 ingest_code_dict() 直接建立 concept_ref 映射。

认识论等级：L0（AST 是确定性解析，无经验假设）

Pure Python, stdlib only (ast, pathlib, json).
"""

from __future__ import annotations

import ast
import json
import sys
from pathlib import Path

_EXCLUDED_DIRS = {"__pycache__", ".venv", "node_modules", ".git", "venv", "env"}
_EXCLUDED_FILES = {"__init__.py"}


# ---------------------------------------------------------------------------
# AST 分析
# ---------------------------------------------------------------------------

def _annotation_to_str(node: ast.expr | None) -> str:
    if node is None:
        return ""
    try:
        return ast.unparse(node)
    except Exception:
        return "?"


def _extract_name(node: ast.expr) -> str | None:
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        base = _extract_name(node.value)
        if base:
            return f"{base}.{node.attr}"
        return node.attr
    return None


class _CodeAnalyzer(ast.NodeVisitor):
    """分析单个 Python 文件，提取结构化代码概念。"""

    def __init__(self, module_name: str, source_lines: list[str]) -> None:
        self.module_name = module_name
        self.source_lines = source_lines
        self.modules: list[dict] = []
        self.classes: list[dict] = []
        self.functions: list[dict] = []
        self._scope_stack: list[str] = []
        self._imports: list[str] = []
        self._calls: dict[str, list[str]] = {}  # qualified_name -> [callee_names]

    def _qualified_name(self, name: str) -> str:
        if self._scope_stack:
            return f"{self._scope_stack[-1]}.{name}"
        return name

    def _make_vertex_id(self, qualified: str) -> str:
        return f"{self.module_name}.{qualified}"

    def _extract_signature(self, node: ast.FunctionDef | ast.AsyncFunctionDef) -> str:
        args = []
        for arg in node.args.args:
            name = arg.arg
            if arg.annotation:
                ann = _annotation_to_str(arg.annotation)
                name = f"{name}: {ann}"
            args.append(name)
        sig = f"{node.name}({', '.join(args)})"
        if node.returns:
            ret = _annotation_to_str(node.returns)
            sig += f" -> {ret}"
        return sig

    def analyze(self, tree: ast.Module) -> None:
        # Module-level info
        docstring = ast.get_docstring(tree) or ""
        self.modules.append({
            "name": self.module_name,
            "docstring": docstring.split("\n")[0].strip() if docstring else self.module_name,
            "vertex_id": f"{self.module_name}.<module>",
        })
        self.visit(tree)

    def visit_ClassDef(self, node: ast.ClassDef) -> None:
        qualified = self._qualified_name(node.name)
        bases = [_extract_name(b) or "?" for b in node.bases]
        docstring = ast.get_docstring(node) or ""
        first_line = docstring.split("\n")[0].strip() if docstring else ""

        # Collect method names
        methods = []
        for item in node.body:
            if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
                methods.append(item.name)

        self.classes.append({
            "name": node.name,
            "qualified": qualified,
            "bases": bases,
            "docstring": first_line,
            "methods": methods,
            "vertex_id": self._make_vertex_id(qualified),
            "module": self.module_name,
        })

        self._scope_stack.append(qualified)
        self.generic_visit(node)
        self._scope_stack.pop()

    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
        self._handle_funcdef(node)

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:
        self._handle_funcdef(node)

    def _handle_funcdef(self, node: ast.FunctionDef | ast.AsyncFunctionDef) -> None:
        qualified = self._qualified_name(node.name)
        sig = self._extract_signature(node)
        docstring = ast.get_docstring(node) or ""
        first_line = docstring.split("\n")[0].strip() if docstring else ""

        # Determine if this is a method or standalone function
        is_method = len(self._scope_stack) > 0
        parent_class = self._scope_stack[-1] if is_method else None

        # Collect callees
        callees: list[str] = []
        for child in ast.walk(node):
            if isinstance(child, ast.Call):
                callee_name = _extract_name(child.func)
                if callee_name:
                    callees.append(callee_name)

        self.functions.append({
            "name": node.name,
            "qualified": qualified,
            "signature": sig,
            "docstring": first_line,
            "is_method": is_method,
            "parent_class": parent_class,
            "callees": callees,
            "vertex_id": self._make_vertex_id(qualified),
            "module": self.module_name,
        })

        self._scope_stack.append(qualified)
        # Visit nested definitions but don't re-process Call nodes
        for child in node.body:
            if isinstance(child, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                self.visit(child)
        self._scope_stack.pop()

    def visit_Import(self, node: ast.Import) -> None:
        for alias in node.names:
            self._imports.append(alias.name)

    def visit_ImportFrom(self, node: ast.ImportFrom) -> None:
        module = node.module or ""
        for alias in (node.names or []):
            self._imports.append(f"{module}.{alias.name}")


# ---------------------------------------------------------------------------
# JSONL 生成
# ---------------------------------------------------------------------------

def _generate_module_entry(mod: dict) -> dict:
    """生成模块的辞典条目。"""
    return {
        "term": mod["name"],
        "domain": "code_project",
        "definition": f"Python module: {mod['docstring'][:300]}",
        "synonyms": [],
        "contrasts": [],
        "connective_patterns": [f"import {mod['name']}"],
        "concept_ids": [mod["vertex_id"]],
    }


def _generate_class_entry(cls: dict, all_classes: dict[str, dict]) -> dict:
    """生成类的辞典条目。"""
    # Synonyms: base classes (if they exist in our codebase)
    synonyms = [b for b in cls["bases"] if b in all_classes]

    # Contrasts: sibling classes (same base)
    contrasts = []
    for other_name, other_cls in all_classes.items():
        if other_name == cls["name"]:
            continue
        if set(cls["bases"]) & set(other_cls["bases"]):
            contrasts.append(other_name)

    definition = f"class {cls['name']}"
    if cls["bases"]:
        definition += f"({', '.join(cls['bases'])})"
    if cls["docstring"]:
        definition += f": {cls['docstring'][:200]}"
    if cls["methods"]:
        public_methods = [m for m in cls["methods"] if not m.startswith("_")]
        if public_methods:
            definition += f". Methods: {', '.join(public_methods[:10])}"

    connective_patterns = []
    for m in cls["methods"][:5]:
        connective_patterns.append(f"{cls['name']}.{m}")

    return {
        "term": cls["name"],
        "domain": "code_project",
        "definition": definition,
        "synonyms": synonyms[:5],
        "contrasts": contrasts[:5],
        "connective_patterns": connective_patterns,
        "concept_ids": [cls["vertex_id"]],
    }


def _generate_function_entry(
    func: dict,
    all_functions: dict[str, dict],
    all_classes: dict[str, dict],
) -> dict:
    """生成函数/方法的辞典条目。"""
    # Skip private/dunder methods (they clutter the dictionary)
    if func["name"].startswith("_") and func["name"] != "__init__":
        return None

    definition = func["signature"]
    if func["docstring"]:
        definition += f": {func['docstring'][:200]}"

    # Synonyms: functions with same name in different modules
    synonyms = []
    for other_q, other_f in all_functions.items():
        if other_q == func["qualified"]:
            continue
        if other_f["name"] == func["name"] and other_f["module"] != func["module"]:
            synonyms.append(f"{other_f['module']}.{other_f['name']}")

    # Contrasts: sibling methods in same class
    contrasts = []
    if func["parent_class"] and func["parent_class"] in all_classes:
        cls = all_classes[func["parent_class"]]
        for m in cls["methods"]:
            if m != func["name"] and not m.startswith("_"):
                contrasts.append(m)

    # Connective patterns: callees
    connective_patterns = []
    for callee in func["callees"][:5]:
        connective_patterns.append(f"{func['name']} calls {callee}")
    if func["parent_class"]:
        connective_patterns.append(f"defined in {func['parent_class']}")

    term = func["name"]
    if func["is_method"] and func["parent_class"]:
        # Use Class.method as term for methods
        parent_short = func["parent_class"].split(".")[-1]
        term = f"{parent_short}.{func['name']}"

    return {
        "term": term,
        "domain": "code_project",
        "definition": definition,
        "synonyms": synonyms[:5],
        "contrasts": contrasts[:5],
        "connective_patterns": connective_patterns,
        "concept_ids": [func["vertex_id"]],
    }


def generate_code_dictionary(
    src_dir: str | Path,
    output_path: str | Path | None = None,
) -> list[dict]:
    """扫描目录下的 Python 文件，生成代码辞典条目。

    Args:
        src_dir: 源代码目录
        output_path: 可选的输出 JSONL 文件路径

    Returns:
        辞典条目列表
    """
    root = Path(src_dir)
    all_modules: list[dict] = []
    all_classes: dict[str, dict] = {}  # qualified_name -> class_info
    all_functions: dict[str, dict] = {}  # qualified_name -> func_info

    # Phase 1: 扫描所有文件
    py_files = sorted(root.glob("*.py"))
    for py_file in py_files:
        if py_file.name in _EXCLUDED_FILES:
            continue
        # Skip test files
        if py_file.name.startswith("test_"):
            continue
        # Skip experiment files (they are domain-specific runs, not core code)
        if py_file.name.startswith("experiment_"):
            continue

        try:
            source = py_file.read_text(encoding="utf-8")
        except Exception:
            continue

        source_lines = source.split("\n")
        module_name = py_file.stem
        tree = ast.parse(source, filename=str(py_file))

        analyzer = _CodeAnalyzer(module_name, source_lines)
        analyzer.analyze(tree)

        all_modules.extend(analyzer.modules)
        for cls in analyzer.classes:
            all_classes[cls["qualified"]] = cls
        for func in analyzer.functions:
            all_functions[func["qualified"]] = func

    # Phase 2: 生成辞典条目
    entries: list[dict] = []

    # Module entries
    for mod in all_modules:
        entries.append(_generate_module_entry(mod))

    # Class entries
    for cls in all_classes.values():
        entries.append(_generate_class_entry(cls, all_classes))

    # Function entries (skip private, filter None)
    for func in all_functions.values():
        entry = _generate_function_entry(func, all_functions, all_classes)
        if entry is not None:
            entries.append(entry)

    # Phase 3: 输出
    if output_path is not None:
        out = Path(output_path)
        with open(out, "w", encoding="utf-8") as f:
            for entry in entries:
                f.write(json.dumps(entry, ensure_ascii=False) + "\n")
        print(
            f"代码辞典生成完成: {len(entries)} 条目 "
            f"({len(all_modules)} 模块, {len(all_classes)} 类, "
            f"{sum(1 for e in entries if e.get('domain') == 'code_project' and '(' in e.get('definition', ''))} 函数/方法)",
            file=sys.stderr,
        )

    return entries


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        description="从 Python 源码自动生成代码辞典 JSONL"
    )
    parser.add_argument(
        "--src", default=".",
        help="源代码目录 (default: .)",
    )
    parser.add_argument(
        "--output",
        default=None,
        help="输出 JSONL 路径 (default: signifier_net/dictionaries/code_dict_project.jsonl)",
    )
    args = parser.parse_args()

    src_dir = Path(args.src)
    if args.output:
        output = Path(args.output)
    else:
        output = src_dir / "signifier_net" / "dictionaries" / "code_dict_project.jsonl"

    entries = generate_code_dictionary(src_dir, output)
    print(f"生成 {len(entries)} 条代码辞典条目 → {output}")
