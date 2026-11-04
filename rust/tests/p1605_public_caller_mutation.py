"""#1605 公开入口变异负控（在 rust/ 下运行：python3 tests/p1605_public_caller_mutation.py）。

1. 生效记录改为非默认值（version=2，annualization_days=126，cvar_tail=0.25），p1605_public_* 须绿。
2. runner.rs/metrics.rs 公开入口改读旧常数记录（version=2，数值为原生效值），须编译成功且两条测试
   都在数值断言处红（两侧都是 Signed(2)，红灯只能来自指标数值）。
3. 还原全部临时修改并以原生效记录重跑，须绿。变异不提交。
"""
import json
import subprocess
import sys
from pathlib import Path

FILES = [
    Path("src/theta_v0/policy_records/research-stats.json"),
    Path("src/theta_v0/backtest/runner.rs"),
    Path("src/theta_v0/backtest/metrics.rs"),
]
CMD = ["cargo", "test", "--lib", "p1605_public_", "--", "--test-threads=1"]
originals = {p: p.read_text() for p in FILES}


def run(label):
    print(label, flush=True)
    r = subprocess.run(CMD, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print(r.stdout, flush=True)
    return r


try:
    old = json.loads(originals[FILES[0]])
    old["version"] = 2
    active = json.loads(json.dumps(old))
    active["fields"]["annualization_days"]["value"] = 126.0
    active["fields"]["cvar_tail"]["value"] = 0.25
    FILES[0].write_text(json.dumps(active))
    r = run("P1605 PUBLIC NONDEFAULT EXPECT GREEN")
    if r.returncode != 0 or "2 passed; 0 failed" not in r.stdout:
        sys.exit("nondefault green failed")

    const = '\nconst P1605_OLD_RECORD: &str = r##"' + json.dumps(old) + '"##;\n'
    for p in FILES[1:]:
        s = originals[p]
        n = s.count("Some(ACTIVE_RESEARCH_STATS_JSON)")
        if n == 0:
            sys.exit(f"{p} 无公开入口加载点")
        p.write_text(s.replace("Some(ACTIVE_RESEARCH_STATS_JSON)", "Some(P1605_OLD_RECORD)") + const)
        print(f"MUTATED {p} load_sites={n}", flush=True)
    r = run("P1605 PUBLIC OLD CONSTANT MUTANT EXPECT RED")
    if r.returncode != 101 or "0 passed; 2 failed" not in r.stdout or "assertion `left == right` failed" not in r.stdout:
        sys.exit("mutant did not fail at numeric assertions")
finally:
    for p, s in originals.items():
        p.write_text(s)
    print("RESTORED", flush=True)

r = run("P1605 PUBLIC RESTORED EXPECT GREEN")
if r.returncode != 0 or "2 passed; 0 failed" not in r.stdout:
    sys.exit("restored green failed")
print("P1605 MUTATION CHECK OK", flush=True)
