#!/usr/bin/env python3
"""#1588 ci-time：把一个 cargo test 目标按测试名切成 N 片并行跑，且机械保证「各片并集 == 全集」。

用法（在 rust/ 下）：
  test-shard.py run --index I --count N --weights W --out DIR [--skip PAT ...] -- <cargo test 参数>
  test-shard.py verify --count N DIR...

run：
  1. `cargo test <参数> -- --list [--skip PAT ...]` 动态列出全集（不维护静态清单，新增测试自动入片）；
  2. 按权重做 LPT（最长处理时间优先）分片：权重降序逐个放进当前总权重最小的片，平局取编号小者；
     权重表外的测试取表内中位数。分片是全集的纯函数，各片独立重算结果一致；
  3. 当场断言分片两两不交、并集 == 全集，否则不跑直接失败；
  4. `cargo test <参数> -- --exact <本片测试名...>` 执行本片；
  5. 从 libtest 输出逐行收「test X ... ok|ignored|FAILED」，断言实际执行集 == 本片分配集。
  DIR 下落 all.txt（全集）、assigned.txt（本片）、executed.tsv（名\t结果），供 verify 汇总。

verify：读 N 片的产物，断言全集各片一致、分配两两不交且并集 == 全集、各片执行集 == 分配集，
  并汇总 ok/ignored/FAILED 计数（与拆分前单步的 "test result:" 对拍用）。

权重只影响各片耗时是否均衡，不影响覆盖：任何权重表（含空表、过期表）下并集 == 全集都成立。
"""

from __future__ import annotations

import argparse
import re
import statistics
import subprocess
import sys
from pathlib import Path

RESULT_LINE = re.compile(r"^test (\S+) \.\.\. (ok|FAILED|ignored)\b")
LIST_SUFFIX = ": test"


def fail(title: str, message: str) -> None:
    print(f"::error title={title}::{message}", file=sys.stderr)
    sys.exit(1)


def load_weights(path: Path) -> dict[str, float]:
    weights: dict[str, float] = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        seconds, name = line.split("\t", 1)
        weights[name] = float(seconds)
    return weights


def partition(names: list[str], weights: dict[str, float], count: int) -> list[list[str]]:
    if count < 1:
        raise ValueError(f"shard count must be >= 1, got {count}")
    known = [weights[n] for n in names if n in weights]
    default = statistics.median(known) if known else 1.0

    def weight(name: str) -> float:
        return weights.get(name, default)

    shards: list[list[str]] = [[] for _ in range(count)]
    loads = [0.0] * count
    for name in sorted(set(names), key=lambda n: (-weight(n), n)):
        target = min(range(count), key=lambda i: (loads[i], i))
        shards[target].append(name)
        loads[target] += weight(name)
    return [sorted(s) for s in shards]


def check_partition(names: list[str], shards: list[list[str]]) -> None:
    seen: dict[str, int] = {}
    for index, shard in enumerate(shards):
        for name in shard:
            if name in seen:
                fail("Shard overlap", f"{name} assigned to shards {seen[name]} and {index}")
            seen[name] = index
    full = set(names)
    missing = sorted(full - seen.keys())
    extra = sorted(seen.keys() - full)
    if missing or extra:
        fail(
            "Shard union != full set",
            f"missing={len(missing)} {missing[:5]} extra={len(extra)} {extra[:5]}",
        )


def list_tests(cargo_args: list[str], skips: list[str]) -> list[str]:
    libtest = ["--list"]
    for pattern in skips:
        libtest += ["--skip", pattern]
    out = subprocess.run(
        ["cargo", "test", *cargo_args, "--", *libtest],
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    ).stdout
    names = sorted({line[: -len(LIST_SUFFIX)] for line in out.splitlines() if line.endswith(LIST_SUFFIX)})
    if not names:
        fail("Empty test list", "cargo test --list returned no tests; nothing would be executed")
    return names


def write_lines(path: Path, lines: list[str]) -> None:
    path.write_text("".join(f"{line}\n" for line in lines), encoding="utf-8")


def cmd_run(args: argparse.Namespace) -> int:
    if not 0 <= args.index < args.count:
        fail("Invalid shard index", f"index={args.index} count={args.count}")
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    names = list_tests(args.cargo_args, args.skip)
    shards = partition(names, load_weights(Path(args.weights)), args.count)
    check_partition(names, shards)
    mine = shards[args.index]
    write_lines(out / "all.txt", names)
    write_lines(out / "assigned.txt", mine)
    sizes = " ".join(str(len(s)) for s in shards)
    print(f"shard_plan index={args.index} count={args.count} total={len(names)} sizes={sizes}", flush=True)
    if not mine:
        fail("Empty shard", f"shard {args.index}/{args.count} has no tests; lower the shard count")

    proc = subprocess.Popen(
        ["cargo", "test", *args.cargo_args, "--", "--exact", *mine],
        stdout=subprocess.PIPE,
        text=True,
        bufsize=1,
    )
    executed: dict[str, str] = {}
    assert proc.stdout is not None
    for line in proc.stdout:
        # 逐行刷出：日志时间戳是事后定位慢测试的唯一依据，缓冲会把它们挤到一起。
        sys.stdout.write(line)
        sys.stdout.flush()
        match = RESULT_LINE.match(line)
        if match:
            executed[match.group(1)] = match.group(2)
    status = proc.wait()
    write_lines(out / "executed.tsv", [f"{n}\t{r}" for n, r in sorted(executed.items())])

    assigned = set(mine)
    missing = sorted(assigned - executed.keys())
    extra = sorted(executed.keys() - assigned)
    print(
        f"shard_result index={args.index} assigned={len(assigned)} executed={len(executed)} "
        f"missing={len(missing)} extra={len(extra)} cargo_status={status}"
    )
    if status != 0:
        return status
    if missing or extra:
        fail("Shard executed != assigned", f"missing={missing[:5]} extra={extra[:5]}")
    return 0


def cmd_verify(args: argparse.Namespace) -> int:
    dirs = [Path(d) for d in args.dirs]
    if len(dirs) != args.count:
        fail("Shard artifacts missing", f"expected {args.count} shard dirs, got {len(dirs)}")
    full_lists = [(d / "all.txt").read_text(encoding="utf-8").splitlines() for d in dirs]
    if any(lst != full_lists[0] for lst in full_lists):
        fail("Shard full lists differ", "shards listed different test sets; plans are not comparable")
    names = full_lists[0]
    shards = [(d / "assigned.txt").read_text(encoding="utf-8").splitlines() for d in dirs]
    check_partition(names, shards)
    totals: dict[str, int] = {"ok": 0, "ignored": 0, "FAILED": 0}
    for d, shard in zip(dirs, shards):
        executed_path = d / "executed.tsv"
        # run 在 cargo 结束后才落 executed.tsv；缺它说明该片中途被杀（超时、OOM 等），没有执行记录。
        if not executed_path.is_file():
            fail("Shard executed.tsv missing", f"{d}: no execution record; the shard did not finish")
        rows = [line.split("\t") for line in executed_path.read_text(encoding="utf-8").splitlines()]
        executed = {name: result for name, result in rows}
        if set(executed) != set(shard):
            fail("Shard executed != assigned", f"{d}: assigned={len(shard)} executed={len(executed)}")
        for result in executed.values():
            totals[result] += 1
        print(f"shard_dir={d} assigned={len(shard)} executed={len(executed)}")
    print(
        f"shard_union total={len(names)} ok={totals['ok']} ignored={totals['ignored']} "
        f"failed={totals['FAILED']}"
    )
    return 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run")
    run.add_argument("--index", type=int, required=True)
    run.add_argument("--count", type=int, required=True)
    run.add_argument("--weights", required=True)
    run.add_argument("--out", required=True)
    run.add_argument("--skip", action="append", default=[])
    run.add_argument("cargo_args", nargs=argparse.REMAINDER)
    verify = sub.add_parser("verify")
    verify.add_argument("--count", type=int, required=True)
    verify.add_argument("dirs", nargs="+")
    args = parser.parse_args(argv)
    if args.command == "run":
        if args.cargo_args[:1] == ["--"]:
            args.cargo_args = args.cargo_args[1:]
        return cmd_run(args)
    return cmd_verify(args)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
