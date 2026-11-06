"""#1393 静态边界检查；不编译、不调用产品、Lean 或网络。"""
import ast
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
BASE = "3315ed1d59ba31ffbad3f0b959702eaa92f4ddbd"
PATH = "rust/src/theta_v0/parser/inclusion.rs"


def function(source, name):
    start = re.search(r"(?:pub\(crate\) )?fn " + name + r"\(", source).start()
    brace = source.index("{", start)
    depth = 1
    end = brace + 1
    while depth:
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return source[start:end]


before = subprocess.check_output(["git", "show", f"{BASE}:{PATH}"], cwd=ROOT, text=True)
after = (ROOT / PATH).read_text()
primitives = {}
for name in ("contains", "merge", "strict_dir", "fold_step", "fold_all"):
    old, new = function(before, name), function(after, name)
    assert old == new, f"已裁几何核发生修改：{name}"
    primitives[name] = hashlib.sha256(new.encode()).hexdigest()
for path in ("s_session/s_tb02_contract.py", "s_session/s_query_integrity.py", "s_session/tests/test_tb02u_contract.py"):
    ast.parse((ROOT / path).read_text(), filename=path)
lean = (ROOT / "formal/Origin/RawInclusionInitialization.lean").read_text()
assert not re.search(r"\b(sorry|admit|axiom)\b", lean)
rule_source = (ROOT / "rust/src/bin/s_structure_session.rs").read_text()
rule = re.search(r'const RULE_REVISION: &str = "([^"]+)";', rule_source)[1]
old_rule_source = subprocess.check_output(["git", "show", f"{BASE}:rust/src/bin/s_structure_session.rs"], cwd=ROOT, text=True)
old_rule = re.search(r'const RULE_REVISION: &str = "([^"]+)";', old_rule_source)[1]
assert rule == old_rule + "+raw-initialization-w/1"
print(json.dumps({"status": "PASS", "base": BASE, "unchanged_primitives_sha256": primitives,
                  "rule": rule, "python_ast": "PASS", "lean_placeholder_scan": "PASS",
                  "rust_compile": "NOT_RUN", "lean_compile": "NOT_RUN"}, ensure_ascii=False, indent=2))
