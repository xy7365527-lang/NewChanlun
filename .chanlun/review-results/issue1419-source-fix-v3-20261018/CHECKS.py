"""#1419 工作草稿：变更相关静查，绝不调用编译器、产品或 Lean。"""
from pathlib import Path
from collections import Counter
import hashlib
import json
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
HEAD = "2694afcf147f3ba4114fde386470886c2428efc0"
BASE = "5cb7253d46d973669e7d10cd3331e12c9ef313b1"
DONOR = "634f62cddfaddf656a35ca9b9db20cb716b62bff"
records = []


def command(args):
    result = subprocess.run(args, cwd=ROOT, text=True, capture_output=True)
    records.append({"argv": args, "exit_code": result.returncode,
                    "stdout": result.stdout, "stderr": result.stderr})
    return result


def blob(rev, path):
    return subprocess.check_output(["git", "show", f"{rev}:{path}"], cwd=ROOT).decode()


def masked(text):
    # 保留偏移及换行；跳过字符串、raw 字符串、字符字面量与可嵌套块注释。
    out = list(text)
    raw_re = re.compile(r'(?:br|r)(#*)"')
    char_re = re.compile(r"'(?:\\.|[^'\\\n])'")
    index = 0
    while index < len(text):
        end = None
        if text.startswith("//", index):
            end = text.find("\n", index)
            if end < 0:
                end = len(text)
        elif text.startswith("/*", index):
            depth, end = 1, index + 2
            while depth and end < len(text):
                if text.startswith("/*", end):
                    depth += 1
                    end += 2
                elif text.startswith("*/", end):
                    depth -= 1
                    end += 2
                else:
                    end += 1
        else:
            raw = raw_re.match(text, index) if text[index] in "br" else None
            if raw:
                close = '"' + raw[1]
                end = text.find(close, raw.end())
                end = len(text) if end < 0 else end + len(close)
            elif text[index] == '"':
                end = index + 1
                while end < len(text):
                    if text[end] == "\\":
                        end += 2
                    elif text[end] == '"':
                        end += 1
                        break
                    else:
                        end += 1
            elif text[index] == "'":
                char = char_re.match(text, index)
                if char:
                    end = char.end()
        if end is not None:
            for at in range(index, end):
                if out[at] != "\n":
                    out[at] = " "
            index = end
        else:
            index += 1
    return "".join(out)


def literals(text, aliases):
    code = masked(text)
    tokens = list(re.finditer(r"[A-Za-z_]\w*|::|->|[^\s]", code))
    found = []
    for at, token in enumerate(tokens[:-1]):
        if token[0] not in aliases | {"Self"} or tokens[at + 1][0] != "{":
            continue
        if at and tokens[at - 1][0] in {"struct", "impl"}:
            continue
        depth, fields, end = 1, set(), at + 2
        while end < len(tokens) and depth:
            word = tokens[end][0]
            if depth == 1 and word in {"rmove", "interval_cache"}:
                if tokens[end + 1][0] in {":", ","}:
                    fields.add(word)
            if word == "{":
                depth += 1
            elif word == "}":
                depth -= 1
            end += 1
        if "rmove" in fields:
            found.append({"name": token[0], "line": text.count("\n", 0, token.start()) + 1,
                          "interval_cache": "interval_cache" in fields})
    return found


assert command(["git", "rev-parse", "HEAD"]).stdout.strip() == HEAD
changed = command(["git", "diff", "--name-only", HEAD, "--", "rust"]).stdout.splitlines()
new = command(["git", "ls-files", "--others", "--exclude-standard", "--", "rust"]).stdout.splitlines()
rust = sorted(set(f for f in changed + new if f.endswith(".rs")))
checks = {}
checks["rustfmt_changed_only"] = command([
    "rustfmt", "--edition", "2021", "--config", "skip_children=true", "--check", *rust
]).returncode == 0
checks["product_diffcheck_base"] = command([
    "git", "diff", "--check", BASE, "--", "rust", "s_session"
]).returncode == 0
checks["incremental_diffcheck_head"] = command(["git", "diff", "--check", HEAD]).returncode == 0
full = command(["git", "diff", "--check", BASE])
checks["full_tree_diffcheck"] = {"exit_code": full.returncode, "passed": full.returncode == 0,
    "note": "旧 v2 SOURCE.patch 上下文空格是 artifact 数据；产品通过不冒称全树通过。"}

files = command(["git", "ls-files", "--", "*.rs"]).stdout.splitlines()
sources = {f: (ROOT / f).read_text() for f in files}
aliases = {"LeveledMove", "LevelMove"}
alias_origins = []
for name, source in sources.items():
    code = masked(source)
    for match in re.finditer(r"\bLeveledMove\s+as\s+(\w+)|\btype\s+(\w+)\s*=\s*[\w:]*\bLeveledMove\s*;", code):
        alias = match[1] or match[2]
        aliases.add(alias)
        alias_origins.append({"file": name, "alias": alias, "line": source.count("\n", 0, match.start()) + 1})
census = []
for name, source in sources.items():
    if not re.search(r"\b(?:LeveledMove|LevelMove|LM2|LM)\b", source):
        continue
    for entry in literals(source, aliases):
        census.append({"file": name, **entry})
missing = [entry for entry in census if not entry["interval_cache"]]
counts = dict(sorted(Counter(entry["file"] for entry in census).items()))
fix_counts = {
    "rust/src/theta_v0/backtest/econ_positive/tests.rs": 22,
    "rust/src/theta_v0/backtest/runner_tests.rs": 4,
    "rust/src/theta_v0/classifier/cand_predicate.rs": 3,
    "rust/src/theta_v0/classifier/signal/cert.rs": 2,
    "rust/src/theta_v0/classifier/voice_eat.rs": 2,
}
for name, count in fix_counts.items():
    before = blob(HEAD, name)
    after = (ROOT / name).read_text()
    assert len([c for c in literals(before, aliases) if not c["interval_cache"]]) == count
    assert re.sub(r"^\s*interval_cache: None,\n", "", after, flags=re.M) == before
    assert after == blob(DONOR, name)
checks["interval_cache_census"] = not missing
checks["mechanical_33_match_donor"] = True
(OUT / "CONSTRUCTOR-CENSUS.json").write_text(json.dumps({
    "method": "git ls-files -- '*.rs' 全仓 Rust；词法屏蔽注释/字符串，枚举 LeveledMove/LevelMove、use/type 别名和含 rmove 的 Self 字面量，平衡花括号读取顶层字段；非 Rust 文本和宏动态生成不属于该静态枚举。",
    "rust_files_scanned": len(files), "aliases": sorted(aliases), "alias_origins": alias_origins,
    "per_file_literals": counts, "literal_total": len(census), "fixed_per_file": fix_counts,
    "missing": missing, "entries": census,
}, ensure_ascii=False, indent=2) + "\n")

source = (ROOT / "rust/src/bin/s_session_v3/source_contributions.rs").read_text()
request = source[source.index("    fn remove_request("):source.index("struct NativeObjects")]
checks["request_text_owner_and_str_borrow"] = (
    "touched: &mut Tree<TextValue>" in request and "Tree<String>" not in request
    and ".get(id.as_str())" in request and "touched.insert(s, owned(s," in request)
producer = (ROOT / "rust/src/bin/s_session_v2/content_producer.rs").read_text()
group = producer[producer.index("    fn group_payload_value("):producer.index("    fn step_payload_value(")]
checks["labels_borrowed_no_extra_vector"] = (
    "reasons.iter().copied().chain(" in group and "reasons.to_vec()" not in group
    and "labels.push" not in group and '"waiting/equal-extremes"' in producer)
k1 = (ROOT / "rust/src/theta_v0/parser/second_fold.rs").read_text()
restore = k1[k1.index("    pub fn from_wire("):k1.index("pub struct SecondFoldStore")]
checks["k1_verification_no_json_clone"] = (
    "wire_match::matches(&probe.wire_view(), wire)" in restore
    and "probe.to_wire()" not in restore and "wire.clone()" not in restore)
matcher = (ROOT / "rust/src/theta_v0/parser/second_fold/wire_match.rs").read_text()
checks["k1_matcher_no_heap_constructor"] = not re.search(
    r"\b(?:Vec|String|Box|Rc|Arc)::|\.to_(?:vec|string|owned)\(|\.clone\(|\.collect\(|format!", matcher)
checks["k1_owner_handoff"] = {"status": "PENDING_BUDGET_API", "registered_wire_clone_remains":
    "self.restored.insert(key, wire.clone());" in k1,
    "note": "仅核验端去树已落地；恢复登记端仍待 #1488 非复制许可链接口，不算闭环。"}
checks["protected_sources_unchanged"] = not command([
    "git", "diff", HEAD, "--", "formal", "rust/src/theta_v0/parser/inclusion.rs",
    "rust/src/s_content/capture_allocation", "rust/src/bin/s_session_v3/audit_owner.rs",
    "rust/src/bin/s_session_v3/q.rs", "rust/src/bin/s_session_v3/effects.rs",
    "rust/src/bin/s_session_v2/tb02c.rs",
    ".chanlun/review-results/issue1419-native-semantic-v2-20261018"
]).stdout
tower = (ROOT / "rust/src/theta_v0/classifier/recursive_tower.rs").read_text()
checks["c_resume_behavior_unchanged"] = "i = if j < units.len() { j + 1 } else { j };" in tower
checks["lean_donor_binding"] = all(DONOR in (ROOT / f).read_text() for f in [
    "rust/src/theta_v0/classifier/recursive_tower.rs", "rust/src/theta_v0/classifier/center.rs",
    "rust/src/theta_v0/classifier/descend.rs", "rust/src/theta_v0/classifier/tests/center_pair_1424.rs"])
checks["rust_compile_and_tests"] = "NOT_RUN"
checks["lean_product_formal_raw_s3q"] = "NOT_RUN"
checks["execution_authorized"] = False
(OUT / "CHECK-RESULTS.json").write_text(json.dumps(checks, ensure_ascii=False, indent=2) + "\n")
(OUT / "COMMANDS.json").write_text(json.dumps(records, ensure_ascii=False, indent=2) + "\n")
print(json.dumps({"checks": checks, "constructor_counts": counts,
                  "changed_rust": rust}, ensure_ascii=False, indent=2))
assert all(value for key, value in checks.items() if isinstance(value, bool) and key != "execution_authorized")
