"""#1419 工作草稿：只读静查和独立 Python toy；绝不调用 Rust 编译器或产品。"""
from pathlib import Path
import hashlib
import itertools
import json
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
BASE = '5cb7253d46d973669e7d10cd3331e12c9ef313b1'
records = []

def command(args):
    result = subprocess.run(args, cwd=ROOT, text=True, capture_output=True)
    records.append({'argv': args, 'exit_code': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr})
    return result

assert command(['git', 'rev-parse', 'HEAD']).stdout.strip() == BASE
changed = command(['git', 'diff', '--name-only', BASE]).stdout.splitlines()
new = command(['git', 'ls-files', '--others', '--exclude-standard']).stdout.splitlines()
files = sorted({f for f in changed + new if f.startswith(('rust/', 's_session/tests/fixtures/'))})
rust = [f for f in files if f.endswith('.rs')]
formatted = command(['rustfmt', '--edition', '2021', '--config', 'skip_children=true', '--check', *rust])
diff = command(['git', 'diff', '--check', BASE])
# 报告保留失败原输出；上游无 Rust 编译/测试的许可。
checks = {'rustfmt_parse_and_format': formatted.returncode == 0, 'git_diff_check': diff.returncode == 0}
checks['protected_inclusion_unchanged'] = not command(['git', 'diff', BASE, '--', 'rust/src/theta_v0/parser/inclusion.rs']).stdout
missing = []
for f in rust:
    p = ROOT / f
    text = p.read_text()
    assert not re.search(r'^(<<<<<<<|=======|>>>>>>>)', text, re.M), f
    for m in re.finditer(r'include_(?:str|bytes)!\(\s*"([^"]+)"\s*\)', text):
        if not (p.parent / m[1]).exists(): missing.append(f + ':' + m[1])
    for m in re.finditer(r'include_(?:str|bytes)!\(concat!\(\s*env!\("CARGO_MANIFEST_DIR"\),\s*"([^"]+)"', text):
        if not (ROOT / 'rust' / m[1].lstrip('/')).exists(): missing.append(f + ':' + m[1])
checks['fixture_include_closure'] = not missing
checks['missing_fixtures'] = missing
# 借用 K1 视图必须保留每个 wire 字段；只检查映射，不冒称运行恢复核。
k1 = (ROOT / 'rust/src/theta_v0/parser/second_fold.rs').read_text()
view = k1[k1.index('impl serde::Serialize for SecondFoldWire'):k1.index('impl SecondFoldCursor {\n    pub fn wire_view')]
old = k1[k1.index('pub fn to_wire'):k1.index('/// 本轮预算已耗尽')]
view_keys = set(re.findall(r'serialize_field\(\s*"([^"]+)"', view))
old_keys = set(re.findall(r'^            "([^"]+)":', old, re.M))
checks['k1_top_level_field_parity'] = view_keys == old_keys and len(view_keys) == 16
checks['k1_wire_fields'] = sorted(view_keys)
structure = (ROOT / 'rust/src/bin/s_session_v3/content_structure.rs').read_text().split('#[cfg(test)]\nmod native_wire_tests')[0]
checks['projection_uses_borrowed_k1'] = '.to_wire()' not in structure and '.checkpoint()' not in structure and 'checkpoint_cursors()' in structure
checks['native_container_insert_signature'] = 'try_insert_native(at, value, work)' in (ROOT / 'rust/src/bin/s_session_v3/structure_value.rs').read_text()
checks['native_knowledge_insert'] = 'try_insert_native(knowledge_position, knowledge_object, work)' in (ROOT / 'rust/src/bin/s_session_v3/generation.rs').read_text()
# 独立 toy：old endpoint 单点间距 vs /2 极值区间，验证退化一致、同价多根例会真正改变准入。
toys = 0
for a_first in range(6):
    for a_last in range(a_first, 6):
        for b_first in range(a_last + 1, 10):
            for b_last in range(b_first, 10):
                gap = max(0, b_first - (a_last + 1))
                between = list(range(a_last + 1, b_first))
                assert gap == len(between)
                if a_first == a_last and b_first == b_last:
                    assert gap == b_first - a_first - 1
                toys += 1
assert (7 - 1 - 1 >= 3) and not (7 - 5 - 1 >= 3)
# 独立 toy：成员范围必须到终点极值区末根；事件代表仍为首根。
coords = [2, 7, 12, 17, 22, 27]
assert [c for c in coords if 7 <= c <= 27] == [7, 12, 17, 22, 27]
assert [c for c in coords if 7 <= c <= 22] == [7, 12, 17, 22]
checks['python_toy_extreme_regions'] = {'cases': toys, 'passed': True, 'scope': '独立模型，未执行Rust'}
checks['rust_compiled'] = False
checks['rust_tests_executed'] = False
checks['execution_authorized'] = False
(OUT / 'CHECK-RESULTS.json').write_text(json.dumps(checks, ensure_ascii=False, indent=2) + '\n')
(OUT / 'COMMANDS.json').write_text(json.dumps(records, ensure_ascii=False, indent=2) + '\n')
print(json.dumps(checks, ensure_ascii=False, indent=2))
assert all(v is True for k, v in checks.items() if isinstance(v, bool) and k not in {'rust_compiled','rust_tests_executed','execution_authorized'})
