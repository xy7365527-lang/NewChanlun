#!/usr/bin/env python3
"""
check_fixture_drift.py — Lean fixture 漂移本地 gate（GitHub issue #263，#245 裁定）

工位定位：rust 四个 parity 测试（theta_v0_buy_parity / theta_v0_classifier_parity /
theta_v0_lean_parity / theta_v0_center_parity）include_str! 读 rust/tests/fixtures/ 下两个
机器导出 fixture。两个 fixture **都有** Lean #eval 机器导出器（#263「查其是否有机器导出器」
已查清：有），非手编。本脚本 regen 导出器输出到**临时文件**（绝不覆盖仓内 fixture），与仓内
fixture 做字段级 diff——Lean 改了值但 fixture 未重落盘 = 漂移，本 gate 变红。

覆盖的两个 fixture：
  1. formal/Origin/ParityFixtureExport.lean（#eval 输出整个 fixture JSON，单行 compress）
       → rust/tests/fixtures/theta_v0_parity.json
  2. formal/Origin/CenterConstruct.lean（#eval IO.println refV1FixtureJson.compress，:634）
       → rust/tests/fixtures/theta_v0_center_parity.json

跑法：
  python3 scripts/check_fixture_drift.py                    # 查两个 fixture（缺省）
  python3 scripts/check_fixture_drift.py --fixture parity   # 只查 theta_v0_parity.json
  python3 scripts/check_fixture_drift.py --fixture center   # 只查 theta_v0_center_parity.json

退出码（真漂移与环境失败严格区分，#263 要求可辨识）：
  0  无漂移（绿）
  1  真漂移（红；打出字段级差异：key path + 仓内值/regen 值）
  3  lake/lean 工具链缺失（非漂移——需先装 Lean 工具链，lake 不在 PATH）
  4  formal 构建失败（非漂移——.lake/build 冷时本脚本先 lake build，build 红走这条路）
  5  导出器运行失败（非漂移——lake env lean 跑导出器非零退出）
  2  脚本内部/用法错误

环境要求与实测耗时（090：声明=能力；以下为 2026-07-26 本仓 worktree 实测，macOS arm64，
Lake 5.0.0 / Lean 4.31.0；formal 无外部依赖——lakefile.toml 不依赖 Mathlib，lake-manifest
packages=[]，lake build 自包含、无需网络）：
  - 热路径（formal/.lake/build 已烤热，2026-07-26 多次实测）：
      lake env lean Origin/ParityFixtureExport.lean ≈ 0.3–1.9 s
      lake env lean Origin/CenterConstruct.lean     ≈ 0.4–5.9 s
      本脚本两 fixture 全查合计                      ≈ 3–7 s
  - 冷启动（formal/.lake/build 缺失或为空）：本脚本先跑 `cd formal && lake build` 全量构建
    （144 jobs，2026-07-26 实测 **12.2 s** 通过，之后走热路径）。

比较口径：单行压缩 JSON 先解析为结构再递归比对（canonical），纯格式差异不误报；
字节级一致性只做信息报告（regen 临时文件与仓内 fixture 逐字节一致 = 无漂移的另一表述）。
临时文件留在 $TMPDIR（脚本结束打印路径，不自动删），绝不写 rust/tests/fixtures/。
"""

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

EXIT_DRIFT = 1
EXIT_INTERNAL = 2
EXIT_NO_TOOLCHAIN = 3
EXIT_BUILD_FAILED = 4
EXIT_EXPORTER_FAILED = 5

ROOT = Path(__file__).resolve().parent.parent
FORMAL = ROOT / "formal"

FIXTURES = {
    "parity": {
        "exporter": "Origin/ParityFixtureExport.lean",
        "fixture": "rust/tests/fixtures/theta_v0_parity.json",
    },
    "center": {
        "exporter": "Origin/CenterConstruct.lean",
        "fixture": "rust/tests/fixtures/theta_v0_center_parity.json",
    },
}


def fail(code: int, msg: str) -> "SystemExit":
    print(f"✗ {msg}", file=sys.stderr)
    return SystemExit(code)


def ensure_toolchain() -> None:
    if shutil.which("lake") is None:
        raise fail(
            EXIT_NO_TOOLCHAIN,
            "FIXTURE-GATE ENVIRONMENT（非漂移）：lake 不在 PATH——需要 Lean/Lake 工具链"
            "（本仓 formal/ 用 lean-toolchain 固定 leanprover/lean4:v4.31.0，"
            "推荐 elan 安装）。装好工具链后重跑本脚本。",
        )


def ensure_built() -> None:
    """formal/.lake/build 缺失或为空 = 冷启动：先 lake build 全量构建（自包含，无需网络）。"""
    build_dir = FORMAL / ".lake" / "build"
    cold = not build_dir.is_dir() or not any(build_dir.iterdir())
    if not cold:
        return
    print("formal/.lake/build 缺失或为空（冷启动）——先 `lake build` 全量构建（无外部依赖）…")
    t0 = time.monotonic()
    proc = subprocess.run(["lake", "build"], cwd=FORMAL)
    dt = time.monotonic() - t0
    if proc.returncode != 0:
        raise fail(
            EXIT_BUILD_FAILED,
            f"FIXTURE-GATE ENVIRONMENT（非漂移）：`lake build` 失败（exit={proc.returncode}，"
            f"耗时 {dt:.1f}s）——formal 构建先修绿，再跑漂移检查。",
        )
    print(f"lake build 完成，耗时 {dt:.1f}s")


def regen(exporter: str, out_path: Path) -> float:
    """跑 Lean 导出器，stdout 原样写入临时文件 out_path。返回耗时（秒）。"""
    t0 = time.monotonic()
    proc = subprocess.run(
        ["lake", "env", "lean", exporter], cwd=FORMAL, capture_output=True
    )
    dt = time.monotonic() - t0
    if proc.returncode != 0:
        tail = proc.stderr.decode("utf-8", "replace")[-2000:]
        raise fail(
            EXIT_EXPORTER_FAILED,
            f"FIXTURE-GATE ENVIRONMENT（非漂移）：`lake env lean {exporter}` 失败"
            f"（exit={proc.returncode}）。stderr 尾部：\n{tail}",
        )
    out_path.write_bytes(proc.stdout)
    return dt


def diff_json(path: str, a, b, out: list) -> None:
    """递归比对两个已解析 JSON，把字段级差异追加到 out（字符串列表）。"""
    if isinstance(a, bool) or isinstance(b, bool):
        if type(a) is not type(b) or a != b:
            out.append(f"漂移 {path}: 仓内={a!r} regen={b!r}")
        return
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        if a != b:
            out.append(f"漂移 {path}: 仓内={a!r} regen={b!r}")
        return
    if isinstance(a, dict) and isinstance(b, dict):
        for k in a.keys() | b.keys():
            p = f"{path}.{k}" if path else k
            if k not in a:
                out.append(f"key 缺失 {p}（仅存在于 regen，值={b[k]!r}）")
            elif k not in b:
                out.append(f"key 缺失 {p}（仅存在于仓内 fixture，值={a[k]!r}）")
            else:
                diff_json(p, a[k], b[k], out)
        return
    if isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            out.append(f"漂移 {path}: 数组长度 仓内={len(a)} regen={len(b)}")
        for i, (x, y) in enumerate(zip(a, b)):
            diff_json(f"{path}[{i}]", x, y, out)
        return
    if type(a) is not type(b):
        out.append(f"漂移 {path}: 类型不同 仓内={type(a).__name__}({a!r}) regen={type(b).__name__}({b!r})")
        return
    if a != b:
        out.append(f"漂移 {path}: 仓内={a!r} regen={b!r}")


def check_one(name: str, tmp_dir: Path) -> bool:
    """检查单个 fixture。返回 True=无漂移。"""
    spec = FIXTURES[name]
    fixture_rel = spec["fixture"]
    fixture_path = ROOT / fixture_rel
    regen_path = tmp_dir / Path(fixture_rel).name
    print(f"[{name}] {fixture_rel}  ←  {spec['exporter']}")
    dt = regen(spec["exporter"], regen_path)
    print(f"  regen {dt:.1f}s → {regen_path}")

    if not fixture_path.is_file():
        print(f"  ✗ 漂移：仓内 fixture 不存在（{fixture_rel}），regen 已产出")
        return False
    repo_bytes = fixture_path.read_bytes()
    regen_bytes = regen_path.read_bytes()

    try:
        repo_json = json.loads(repo_bytes.decode("utf-8"))
    except Exception as e:
        print(f"  ✗ 漂移：仓内 fixture 不是合法 JSON（{e}）")
        return False
    try:
        regen_json = json.loads(regen_bytes.decode("utf-8"))
    except Exception as e:
        print(f"  ✗ 漂移：regen 输出不是合法 JSON（{e}）——导出器契约被破坏")
        return False

    diffs: list = []
    diff_json("", repo_json, regen_json, diffs)
    byte_note = "一致" if repo_bytes == regen_bytes else "不一致（语义比对为准）"
    if diffs:
        print(f"  ✗ 漂移：{len(diffs)} 处字段差异（字节级：{byte_note}）")
        for d in diffs:
            print(f"    {d}")
        return False
    print(f"  ✓ 无漂移：语义一致（0 字段差异）；字节级：{byte_note}")
    return True


def main() -> int:
    ap = argparse.ArgumentParser(description="Lean fixture 漂移本地 gate（issue #263）")
    ap.add_argument(
        "--fixture",
        choices=sorted(FIXTURES),
        action="append",
        help="只查指定 fixture（可重复）；缺省查全部",
    )
    args = ap.parse_args()
    names = args.fixture or sorted(FIXTURES)

    ensure_toolchain()
    ensure_built()

    tmp_dir = Path(tempfile.mkdtemp(prefix="fixture_drift_"))
    print(f"regen 临时目录：{tmp_dir}（保留不删，绝不写 rust/tests/fixtures/）")
    ok = True
    for name in names:
        if not check_one(name, tmp_dir):
            ok = False
    if ok:
        print("✓ fixture 漂移检查：全部无漂移（绿）")
        return 0
    print("✗ fixture 漂移检查：发现漂移（红）——regen 重落 fixture 或回退 Lean 改动")
    return EXIT_DRIFT


if __name__ == "__main__":
    sys.exit(main())
