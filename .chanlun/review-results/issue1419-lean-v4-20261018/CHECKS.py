"""#1419 工作草稿：只读静查与有限整数模型；不启动 Lean/Rust/产品。"""
import hashlib
import itertools
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
BASE = "545e9f75faec49be39de7799e0aaf3f6c8276de9"
DONOR = "634f62cddfaddf656a35ca9b9db20cb716b62bff"
PREP = Path("/Volumes/AgentStorage/issue1419-sandcastle-manager-20261018/lean-same-source-preparation")
INDEX = json.loads((OUT / "SOURCE-INDEX.json").read_text())
COMMANDS = []


def run(args, expected=0):
    p = subprocess.run(args, cwd=ROOT, text=True, capture_output=True)
    COMMANDS.append({"argv": args, "exit_code": p.returncode, "stdout": p.stdout, "stderr": p.stderr})
    assert p.returncode == expected, COMMANDS[-1]
    return p.stdout


def sha(data):
    return hashlib.sha256(data).hexdigest()


def write(name, value):
    (OUT / name).write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def lean_code(text):
    """剥离嵌套注释/字符串，仅用于关键字与依赖扫描，绝非 Lean parser。"""
    i, depth, string = 0, 0, False
    result = []
    while i < len(text):
        pair = text[i:i + 2]
        if depth:
            if pair == "/-":
                depth += 1
                i += 2
            elif pair == "-/":
                depth -= 1
                i += 2
            else:
                result.append("\n" if text[i] == "\n" else " ")
                i += 1
        elif string:
            if text[i] == "\\":
                i += 2
            else:
                string = text[i] != '"'
                result.append("\n" if text[i] == "\n" else " ")
                i += 1
        elif pair == "/-":
            depth = 1
            i += 2
        elif pair == "--":
            j = text.find("\n", i)
            i = len(text) if j < 0 else j
        elif text[i] == '"':
            string = True
            i += 1
        else:
            result.append(text[i])
            i += 1
    assert depth == 0 and not string, "未闭合注释/字符串"
    return "".join(result)


def source_checks():
    assert run(["git", "rev-parse", "HEAD"]).strip() == BASE
    assert run(["git", "branch", "--show-current"]).strip() == "codex/1419-lean-v4-20261018"
    run(["git", "diff", "--check", "--", "formal"])
    names = set(run(["git", "diff", "--name-only"]).splitlines())
    new = set(run(["git", "ls-files", "--others", "--exclude-standard"]).splitlines())
    allowed = {x["path"] for x in INDEX["files"]}
    report = str(OUT.relative_to(ROOT)) + "/"
    assert all(p in allowed or p.startswith(report) for p in names | new)
    assert set(p for p in names | new if p.startswith("formal/")) == allowed
    manifest = []
    for row in INDEX["files"]:
        path = row["path"]
        current = (ROOT / path).read_bytes()
        donor = subprocess.check_output(["git", "show", DONOR + ":" + path], cwd=ROOT)
        base = None if row.get("base_present") is False else subprocess.check_output(
            ["git", "show", BASE + ":" + path], cwd=ROOT)
        assert sha(donor) == row["donor_sha256"]
        assert (sha(base) if base else None) == row.get("current_base_sha256", row.get("base_sha256"))
        assert sha((PREP / (path.replace("/", "__") + ".patch")).read_bytes()) == row["patch_sha256"]
        code = lean_code(current.decode())
        prior = lean_code(base.decode()) if base else ""
        for token in ["sorry", "admit", "axiom", "unsafe", "native_decide"]:
            assert len(re.findall(r"\b" + token + r"\b", code)) <= len(re.findall(r"\b" + token + r"\b", prior)), (path, token)
        assert not re.search(r"(?m)^\s*(?:set_option\s+(?:maxRecDepth|maxHeartbeats)|opaque\b)", code)
        assert not any(line.rstrip() != line for line in current.decode().splitlines())
        manifest.append({"path": path, "base_sha256": sha(base) if base else None,
                         "donor_sha256": sha(donor), "final_sha256": sha(current),
                         "final_bytes": len(current), "donor_identical": current == donor})
    write("SOURCE-MANIFEST.json", manifest)
    # 依赖范围：全 formal/**/*.lean 的 import；只查直接/反向传递边，未作类型检查。
    sources = {str(p.relative_to(ROOT / "formal")).removesuffix(".lean").replace("/", "."): p
               for p in (ROOT / "formal").rglob("*.lean")}
    imports = {m: re.findall(r"(?m)^import\s+([\w.]+)", lean_code(p.read_text())) for m, p in sources.items()}
    touched = {p.removeprefix("formal/").removesuffix(".lean").replace("/", ".") for p in allowed}
    closure = set(touched)
    while True:
        expanded = closure | {m for m, deps in imports.items() if set(deps) & closure}
        if expanded == closure:
            break
        closure = expanded
    local = Path("/Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/src/lean")
    unresolved = {m: [d for d in imports[m] if d not in sources and not (local / (d.replace(".", "/") + ".lean")).exists()]
                  for m in touched}
    assert not any(unresolved.values()), unresolved
    write("DEPENDENCIES.json", {"method": "全 formal 文本 import 闭包，不执行编译器", "scanned_modules": len(sources),
          "direct": {m: imports[m] for m in sorted(touched)}, "reverse_closure": sorted(closure),
          "local_lean_source": str(local), "mathlib_present": (ROOT / "formal/.lake/packages/mathlib").exists(),
          "typecheck": "NOT_RUN"})
    print("SOURCE: 11 项基线/donor/patch 字节核验通过；写集、空白与新增证明逃逸关键字检查通过。")


def move_model_checks():
    # segment = ('s', lo, hi)，compose = ('c', subs, level)；外缘故意不参与取值。
    def lean(m):
        if m[0] == "s":
            return m[1:]
        def seq(xs):
            if not xs:
                return (0, 0)
            if len(xs) == 1:
                return lean(xs[0])
            a, b = lean(xs[0]), seq(xs[1:])
            return min(a[0], b[0]), max(a[1], b[1])
        return seq(m[1])

    def rust(m):
        if m[0] == "s":
            return m[1:]
        xs = [rust(x) for x in m[1]]
        return (min(x[0] for x in xs), max(x[1] for x in xs)) if xs else (0, 0)

    def cache(m):
        if m[0] == "s":
            return m[1:]
        acc = None
        for x in m[1]:
            lo, hi = cache(x)
            acc = (lo, hi) if acc is None else (min(acc[0], lo), max(acc[1], hi))
        return acc if acc is not None else (0, 0)

    def shift(m):
        return m if m[0] == "s" else ("c", [shift(x) for x in m[1]], m[2] + 7)

    atoms = [("s", lo, hi) for lo, hi in itertools.combinations_with_replacement(range(-3, 4), 2)]
    trees = atoms + [("c", [], 1)] + [("c", [a], 1) for a in atoms]
    trees += [("c", [a, b], 1) for a, b in itertools.product(atoms, repeat=2)]
    trees += [("c", [("c", [a, b], 1), ("c", [], 1), a], 2) for a, b in itertools.product(atoms, repeat=2)]
    for m in trees:
        assert lean(m) == rust(m) == cache(m) == lean(shift(m))
    assert lean(("c", [("s", 10, 40), ("s", 5, 25)], 1)) == (5, 40)
    assert lean(("c", [("s", -40, -10), ("s", -25, -5)], 1)) == (-40, -5)
    print(f"C_MODEL: {len(trees)} 个有限树匹配递归极值/缓存归约/级别平移；非 Zn 末成员及空子树边界通过。")
    return len(trees)


def turn_model_checks():
    intervals = list(itertools.combinations_with_replacement(range(-2, 3), 2))

    def contains(b, x):
        return b[0] <= x[0] and x[1] <= b[1]

    def transition(b, xs, up):
        for x in xs:
            kill = x[1] > b[1] if up else x[0] < b[0]
            confirm = x[0] < b[0] if up else x[1] > b[1]
            if kill:
                return "killed"
            if confirm:
                return "confirmed"
        return "open"

    triples = 0
    for a, b, c in itertools.product(intervals, repeat=3):
        for up in [True, False]:
            first = (a[1] < b[1] and c[1] <= b[1] and c[0] < b[0]) if up else (
                b[0] < a[0] and b[0] <= c[0] and b[1] < c[1])
            strict = (a[1] < b[1] and c[1] < b[1]) if up else (b[0] < a[0] and b[0] < c[0])
            if not contains(b, c) and (c[1] != b[1] if up else c[0] != b[0]):
                assert first == strict
            peak = a[1] < b[1] if up else b[0] < a[0]
            assert (peak and transition(b, [c], up) == "confirmed") == first
            xs = [a, c]
            outside = next((x for x in xs if not contains(b, x)), None)
            assert transition(b, xs, up) == (transition(b, [outside], up) if outside is not None else "open")
            if transition(b, xs, up) != "open":
                assert transition(b, xs + [b, (-5, 5)], up) == transition(b, xs, up)
            else:
                assert all(contains(b, x) for x in xs)
        triples += 1
    assert transition((0, 10), [(-2, 12)], True) == "killed"
    assert transition((0, 10), [(-2, 12)], False) == "killed"
    counterexamples = [
        {"a": [0, 8], "b": [2, 10], "c": [3, 9], "old_strict_top": True, "new_window": "open"},
        {"a": [0, 8], "b": [2, 10], "c": [1, 10], "old_strict_top": False, "new_window": "confirmed"},
    ]
    for x in counterexamples:
        assert (x["a"][1] < x["b"][1] and x["c"][1] < x["b"][1]) == x["old_strict_top"]
        assert transition(x["b"], [x["c"]], True) == x["new_window"]
    write("D-FIRST-COUNTEREXAMPLES.json", {"scope": "局部几何函数有限模型，非 Rust 执行；显示不能声称 /2 已同拍", "cases": counterexamples})
    print(f"D_MODEL: {triples} 组三元素×双方向，首出窗/并列域/终态追加保持通过；旧严格判据有 2 个具名分离例。")
    return triples


def scan_model_checks():
    source = (ROOT / "formal/Origin/CenterConstruct.lean").read_text()

    def endpoints(name):
        return list(map(int, re.search(r"def " + name + r" : List Tick := \[([^]]+)\]", source)[1].split(",")))

    def segs(ps):
        return [(min(a, b), max(a, b), b > a) for a, b in zip(ps, ps[1:])]

    def scan(units):
        centers, i = [], 0
        while i + 2 < len(units):
            seed = units[i:i + 3]
            zd, zg = max(u[0] for u in seed), min(u[1] for u in seed)
            if zd >= zg:
                i += 1
                continue
            j = i + 3
            while j < len(units) and units[j][0] <= zg and zd <= units[j][1]:
                j += 1
            zn = [u for u in units[i:j] if u[2] == units[i][2]]
            centers.append([i, j - 1, zd, zg, min(u[0] for u in zn), max(u[1] for u in zn), j < len(units)])
            i = j + 1 if j < len(units) else j
        return centers

    up = endpoints("e2UpEndpoints")
    groups = [
        ("witness_e2_up_zn_c_resume", segs(up)),
        ("witness_e2_down_zn_c_resume", segs([45 - p for p in up])),
        ("witness_e4_tangent_seed_two_unit_connector", segs(endpoints("e4Endpoints"))),
        ("witness_zn_direction_not_parity", [(10, 40, True), (20, 50, True), (0, 30, False), (5, 25, False), (15, 60, True)]),
    ]
    for name, units in groups:
        statement = source.split("theorem " + name + " :", 1)[1].split(":= by", 1)[0]
        expected = [[int(x.strip()) if x.strip() not in ["true", "false"] else x.strip() == "true"
                     for x in body.split(",")] for body in re.findall("⟨([^⟨⟩]+)⟩", statement)]
        assert scan(units) == expected, name
    # 独立检查四案例缺席旧 fixture；不把可预见漂移改名为 gate 通过。
    fixture = json.loads((ROOT / "rust/tests/fixtures/theta_v0_center_parity.json").read_text())
    missing = [n for n in ["e2_up_zn_c_resume", "e2_down_zn_c_resume", "e4_tangent_seed_connector", "zn_direction_not_parity"] if n not in fixture]
    assert len(missing) == 4 and "break_up_scope" not in fixture["meta"]
    write("E-FIXTURE-GAP.json", {"status": "SOURCE_SHAPE_MISMATCH", "missing_cases": missing,
          "missing_metadata": ["meta.break_up_scope", "meta.break_up_note"], "fixture_drift_gate": "NOT_RUN",
          "reason": "漂移脚本会执行 lake；当前任务禁止。需最终统一 source 远端重导出并修复 ref_v1 消费者。"})
    print("E_MODEL: 4 组 donor 字面期待值匹配；冻结核心、Zn 极值、非 Zn 末成员及 C 续扫可分辨。旧 fixture 缺 4 案例/2 元数据（已登记）。")


def pair_model_checks():
    leaf_names = ["CORE_NOT_STRICTLY_SEPARATED", "UP_NEWBIRTH", "DOWN_NEWBIRTH", "UP_EXPANSION", "DOWN_EXPANSION"]
    development = ["extension", "newBirth", "newBirth", "expansion", "expansion"]
    relation = ["coreOverlap", "upContinuation", "downContinuation", "levelExpansion", "levelExpansion"]

    def classify(a, b):
        if a[3] < b[0]:
            return 1
        if b[3] < a[0]:
            return 2
        if b[2] < a[1]:
            return 4
        if a[2] < b[1]:
            return 3
        return 0

    def independent(a, b):
        return [not (b[2] < a[1] or a[2] < b[1]), a[3] < b[0], b[3] < a[0],
                a[2] < b[1] and b[0] <= a[3], b[2] < a[1] and a[0] <= b[3]]

    centers = list(itertools.combinations_with_replacement(range(-2, 3), 4))
    count = 0
    for a, b in itertools.product(centers, repeat=2):
        holds = independent(a, b)
        k = classify(a, b)
        assert sum(holds) == 1 and holds[k]
        four = "upContinuation" if b[0] > a[3] else "downContinuation" if b[3] < a[0] else (
            "levelExpansion" if ((b[2] < a[1] and b[3] >= a[0]) or (b[1] > a[2] and b[0] <= a[3])) else "coreOverlap")
        assert relation[k] == four
        count += 1
    source = (ROOT / "formal/Origin/CenterPairFixtureExport.lean").read_text()
    fixture = json.loads((ROOT / "rust/tests/fixtures/center_pair_1424_lean.json").read_text())
    keys = ["DD", "ZD", "ZG", "GG"]
    cases, matrices = [], []
    for line in source.splitlines():
        if not line.strip().startswith('row "'):
            continue
        name = re.search(r'row "([^"]+)"', line)[1]
        a, b = [tuple(map(int, m)) for m in re.findall(r"mkCenter (-?\d+) (-?\d+) (-?\d+) (-?\d+)", line)]
        assert list(a) == sorted(a) and a[1] < a[2] and list(b) == sorted(b) and b[1] < b[2]
        k = classify(a, b)
        cases.append({"name": name, "A": dict(zip(keys, a)), "B": dict(zip(keys, b)), "leaf": leaf_names[k], "development": development[k]})
        matrices.append({"name": name, "row_next_col_prev": keys, "matrix": [[(x > y) - (x < y) for y in a] for x in b], "four_state": relation[k]})
    assert len(cases) == 8 and cases == fixture["cases"]
    assert fixture["schema"] == "tb1424-lean-pair/1" and fixture["source"] == "Origin/CenterPairFixtureExport.lean"
    write("F-FIXTURE-STATIC.json", {"scope": "解析 Lean 输入字面量 + Python 整数公式；不是 Lean 导出", "cases": len(cases),
          "fixture_sha256": sha((ROOT / "rust/tests/fixtures/center_pair_1424_lean.json").read_bytes()),
          "semantic_fields_match": True, "finite_ordered_pairs": count, "matrix_scope": "128 格由输入核算；fixture/Lean 导出没有矩阵字段，未声称运行 Rust 比较", "derived": matrices})
    print(f"F_MODEL: {count} 个有序域中心对五叶恰一/四态公式一致；8 例输入、标签、三态投影与既有 JSON 逐字段相同。")
    return count


try:
    source_checks()
    counts = {"C_trees": move_model_checks(), "D_triples": turn_model_checks()}
    scan_model_checks()
    counts["F_ordered_pairs"] = pair_model_checks()
    write("CHECK-RESULTS.json", {"static_and_finite_models": "PASS", "counts": counts,
          "lean_typecheck": "NOT_RUN", "lean_export": "NOT_RUN", "fixture_drift_gate": "NOT_RUN",
          "rust_product_execution": "NOT_RUN", "independent_review": "NOT_RUN"})
    print("边界：Lean 类型检查/求值/fixture 漂移 gate、Rust 与产品运行、独审全部 NOT_RUN。")
finally:
    write("STATIC-COMMANDS.json", COMMANDS)
