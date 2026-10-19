"""#1527 工作草稿：执行本轮靶向检查并保存原始 stdout/stderr、argv、rc、hash。"""

from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

REPORT = Path(__file__).resolve().parent
ROOT = REPORT.parents[2]
BASE = "5cb7253d46d973669e7d10cd3331e12c9ef313b1"
SOURCE_FILES = ["s_session/recovery_admission.py", "s_session/s_service_control.py",
                "s_session/tests/issue1527_recovery/test_python_r4.py"]


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


checks = []
environment = dict(os.environ)
environment["PYTHONDONTWRITEBYTECODE"] = "1"


def check(name, argv):
    result = subprocess.run(argv, cwd=ROOT, env=environment, capture_output=True, timeout=30)
    record = {"name": name, "argv": argv, "cwd": str(ROOT), "rc": result.returncode}
    for stream in ("stdout", "stderr"):
        raw = getattr(result, stream)
        path = REPORT / (name + "." + stream + ".log")
        path.write_bytes(raw)
        record[stream] = {"path": path.name, "sha256": sha(raw), "bytes": len(raw)}
    checks.append(record)
    return result.returncode


check("python-r4", [sys.executable, "-B", "-W", "error::ResourceWarning", "-m", "unittest", "discover",
                    "-s", "s_session/tests/issue1527_recovery", "-p", "test_python_r4.py", "-v"])
check("static", [sys.executable, "-B", str(REPORT / "check_static.py")])
check("diff-whitespace", ["git", "diff", "--check"])
patch_parts = [subprocess.run(["git", "diff", "--binary", "--", "s_session/s_service_control.py"],
                              cwd=ROOT, capture_output=True, check=True).stdout]
for filename in (SOURCE_FILES[0], SOURCE_FILES[2]):
    diff = subprocess.run(["git", "diff", "--no-index", "--binary", "--", "/dev/null", filename],
                          cwd=ROOT, capture_output=True)
    if diff.returncode != 1:
        raise RuntimeError("新增文件差异生成失败")
    patch_parts.append(diff.stdout)
patch_path = REPORT / "python-r4.patch"
patch_path.write_bytes(b"".join(patch_parts))
check("patch-reverse-check", ["git", "apply", "--reverse", "--check", str(patch_path)])
write_json(REPORT / "CHECKS.json", {
    "issue": 1527, "baseline": BASE, "at_utc": datetime.now(timezone.utc).isoformat(),
    "结论": "本轮作者定向验证通过" if all(c["rc"] == 0 for c in checks) else "存在失败",
    "checks": checks, "历史52项作者与9项独审": "历史证据，非本轮通过",
    "Rust类型检查": "NOT_RUN", "Rust生产执行": "NOT_RUN", "正式恢复": "NOT_RUN",
    "外部调用": "仅临时封闭Python stub；不等于Rust消费或真实S/Q",
    "独立评审": "未进行；本工蜂没有派发其他工蜂",
})
originals = []
for filename in SOURCE_FILES:
    result = subprocess.run(["git", "show", BASE + ":" + filename], cwd=ROOT, capture_output=True)
    originals.append({"path": filename, "baseline_sha256": sha(result.stdout) if result.returncode == 0 else None,
                      "current_sha256": sha((ROOT / filename).read_bytes()),
                      "baseline_status": "既存" if result.returncode == 0 else "本轮新增"})
inputs = []
for filename in ("ROOT-I1-IMPLEMENTATION-SCOPE.md", "I1-RESPONSIBILITY-AND-REUSE.md", "ROOT-R4-ENGINEERING-SCOPE.md",
                 "PRIOR-R4-V3-DESIGN-TRANSCRIPT.md", "PRIOR-R4-PROVENANCE.json",
                 "PRIOR-R4-EVIDENCE-2672.txt", "PRIOR-R4-EVIDENCE-2734.txt", "PRIOR-R4-EVIDENCE-3140.txt",
                 "PRIOR-R4-EVIDENCE-3630.txt", "PRIOR-R4-EVIDENCE-4048.txt"):
    path = ROOT.parent / filename
    inputs.append({"path": str(path), "sha256": sha(path.read_bytes())})
artifacts = [{"path": p.name, "sha256": sha(p.read_bytes()), "bytes": p.stat().st_size}
             for p in sorted(REPORT.iterdir()) if p.is_file() and p.name != "MANIFEST.json"]
write_json(REPORT / "MANIFEST.json", {"issue": 1527, "baseline": BASE, "来源": inputs,
                                    "源码": originals, "交付原件": artifacts,
                                    "命令与退出码": checks, "patch_sha256": sha(patch_path.read_bytes()),
                                    "名分": "#1527工作草稿；当前checkout未提交差异，未合入"})
print(json.dumps({"检查数": len(checks), "退出码": [c["rc"] for c in checks],
                  "patch_sha256": sha(patch_path.read_bytes())}, ensure_ascii=False))
raise SystemExit(0 if all(c["rc"] == 0 for c in checks) else 1)
