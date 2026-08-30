"""tape v3 门状态列读取 + 状态闸（#1312 · #1282 第二轮预注册执行件）。

## 职责边界（防④列判据分叉——#1312 票面设计要点）

门的计算**全部在 Rust 生产函数本体**（`rust/src/trading/gate_state_dump.rs` 驱动
`trend_exhaustion.rs` 的 `up_unexhausted`/`down_unexhausted` 与 `fatigue_gate.rs`），
落盘为逐 bar 列文件。本模块**只读列、只做取反的进场闸**，不实现任何判据——
Python 侧一行判据都不写，是"同一判断不得有宽严两档实现"（AGENTS.md 收敛通则）
在跨语言面的执行。

## 状态闸语义（#1312 要做 ②）

- 空腿（逆势做空）进场拒于 `l2_up_unexhausted = true`——41课"大级别走势没有任何
  衰竭迹象时参与反向小级别买卖点是刀口舔血"：上涨走势未衰竭 ⇒ 不做反向空腿。
- 多腿（逆势做多）镜像拒于 `l2_down_unexhausted = true`。
- fatigue 为**可选配置臂**（默认关）：`FatigueGate` 守的是本级别 **Up 走势**衰竭
  （卖侧证据），生产侧无下跌侧镜像门 ⇒ 该臂**只作用于空腿**（给下跌侧另造一个
  fatigue 镜像 = 新判据，本票禁止）。列不可用时开该臂 ⇒ fail-fast，不代理。

## 列文件格式（小端，与 `gate_state_dump.rs::write_gate_columns` 逐字段对齐）

```text
  header: magic u64 = 0x4E43545056334700 ("NCTPV3G\\0")
          n_bars u64, ladder u64, fatigue_available u64（0/1）
          first_close f64, last_close f64  ← 与驱动侧 closes 的接缝校验
  cols:   up_unexhausted n*u8 / down_unexhausted n*u8 / fatigue_state n*u8
```

生成：
```bash
PYTHONPATH=src uv run python analysis/_dump_tape_rust.py ES GC CL ZN 6E BRN DX
cd rust && cargo test --release gate_state_dump_prereg7 -- --ignored --nocapture
```

认识论等级：读数搬运 L0（判据零实现，列由生产函数产出）。
"""

from __future__ import annotations

import struct
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA_DIR = ROOT / "analysis" / "data_cache"

MAGIC_V3G = 0x4E43545056334700
HEADER_FMT = "<QQQQdd"
HEADER_SIZE = struct.calcsize(HEADER_FMT)

# fatigue 列值（与 gate_state_dump.rs 常量逐值对齐）。
FATIGUE_FRESH = 0
FATIGUE_FATIGUED = 1
FATIGUE_UNAVAILABLE = 255

# 门列生成命令（卡点报告/异常信息共用，避免两处口径漂移）。
REGEN_CMD = (
    "PYTHONPATH=src uv run python analysis/_dump_tape_rust.py <SYM> && "
    "cd rust && cargo test --release gate_state_dump_prereg7 -- --ignored --nocapture"
)


def gate_path(sym: str) -> Path:
    return DATA_DIR / f"_tape_v3_gates_{sym}.bin"


@dataclass(frozen=True)
class StateGate:
    """逐 bar 状态闸（列 = 生产函数读数；本类只做取反的进场准入）。"""

    symbol: str
    ladder: int
    up_unexhausted: tuple[bool, ...]
    down_unexhausted: tuple[bool, ...]
    fatigue: tuple[int, ...]
    fatigue_available: bool
    #: 可选配置臂（#1312 要做 ②）：True ⇒ 空腿进场额外要求 fatigue 处于衰竭段内。
    use_fatigue: bool = False

    def __post_init__(self) -> None:
        if self.use_fatigue and not self.fatigue_available:
            raise RuntimeError(
                f"[{self.symbol}] fatigue 臂不可用：磁带无 run_high 行 ⇒ FatigueGate "
                "清空路径(1) 无数据基础（fatigue_gate.rs 头部：不提供 close 代理 "
                "run_high 的降级路径，代理 = 双真相源）。该臂须待信号层产出 run_high "
                "行后方可开启。"
            )

    def __len__(self) -> int:
        return len(self.up_unexhausted)

    def long_entry_allowed(self, i: int) -> bool:
        """多腿（逆势做多）进场准入：下跌走势未衰竭 ⇒ 拒。"""
        return not self.down_unexhausted[i]

    def short_entry_allowed(self, i: int) -> bool:
        """空腿（逆势做空）进场准入：上涨走势未衰竭 ⇒ 拒；fatigue 臂开时另需衰竭段内。"""
        if self.up_unexhausted[i]:
            return False
        if self.use_fatigue:
            return self.fatigue[i] == FATIGUE_FATIGUED
        return True

    def with_fatigue(self, on: bool = True) -> "StateGate":
        """返回切换 fatigue 臂的新闸（配置臂对照用；列不可用时开臂即 fail-fast）。"""
        return StateGate(
            symbol=self.symbol,
            ladder=self.ladder,
            up_unexhausted=self.up_unexhausted,
            down_unexhausted=self.down_unexhausted,
            fatigue=self.fatigue,
            fatigue_available=self.fatigue_available,
            use_fatigue=on,
        )


def load_state_gate(
    sym: str,
    closes: list[float],
    *,
    use_fatigue: bool = False,
) -> StateGate:
    """读 v3 门状态列并与驱动侧 closes 做接缝校验（bar 数 + 首尾 close 逐位）。

    接缝校验是必需的：门列来自 `_dump_tape_rust.py` 的清洗后 bar 序列，驱动侧
    closes 来自 `load_ohlc` 的同款清洗——两者对齐是"逐 bar 列按 bar 下标 join"
    的前提，不对齐即错位读数，故 fail-fast 而非静默截断。
    """
    path = gate_path(sym)
    if not path.exists():
        raise FileNotFoundError(
            f"[{sym}] 缺 v3 门状态列 {path}；生成命令：{REGEN_CMD.replace('<SYM>', sym)}"
        )
    raw = path.read_bytes()
    if len(raw) < HEADER_SIZE:
        raise ValueError(f"[{sym}] 门状态列文件过短（{len(raw)}B）——落盘不完整")
    magic, n, ladder, fat_avail, first_c, last_c = struct.unpack(
        HEADER_FMT, raw[:HEADER_SIZE]
    )
    if magic != MAGIC_V3G:
        raise ValueError(f"[{sym}] 门状态列 magic 不匹配——dump 格式代次错位")
    if len(raw) != HEADER_SIZE + 3 * n:
        raise ValueError(
            f"[{sym}] 门状态列长度 {len(raw)}B ≠ 头部 + 3×{n} 列字节——格式错位"
        )
    if n != len(closes):
        # 尾对齐：门列末 bar close == 驱动 closes[-1] 时，多出的 bar 在头部（dump 覆盖
        # 到 prereg 窗外更早数据）——截取尾部 len(closes) 根即与驱动逐 bar 对齐。
        if n > len(closes) and last_c == closes[-1]:
            skip = n - len(closes)
            print(
                f"[{sym}] 门列 {n} 根 > 驱动 {len(closes)}：尾部对齐截取，"
                f"舍弃头部 {skip} 根（门列末 close {last_c} == 驱动末 close，对齐成立）"
            )
            raw = raw[HEADER_SIZE + skip * 3:]
            n = len(closes)
        else:
            raise ValueError(
                f"[{sym}] 门列 bar 数 {n} ≠ 驱动侧 closes {len(closes)}——磁带与回测输入"
                f"不同源/不同窗，逐 bar join 会错位。重跑 dump：{REGEN_CMD.replace('<SYM>', sym)}"
            )
    if n and (first_c != closes[0] or last_c != closes[-1]):
        raise ValueError(
            f"[{sym}] 门列首尾 close ({first_c}, {last_c}) ≠ 驱动侧 "
            f"({closes[0]}, {closes[-1]})——磁带与回测输入不同源，拒绝错位 join"
        )
    off = HEADER_SIZE
    up = raw[off:off + n]
    dn = raw[off + n:off + 2 * n]
    fat = raw[off + 2 * n:off + 3 * n]
    return StateGate(
        symbol=sym,
        ladder=ladder,
        up_unexhausted=tuple(v != 0 for v in up),
        down_unexhausted=tuple(v != 0 for v in dn),
        fatigue=tuple(fat),
        fatigue_available=fat_avail != 0,
        use_fatigue=use_fatigue,
    )


def gate_open_rates(gate: StateGate) -> dict:
    """门开率读数（C7 步骤5-iv 门开率可观测义务的驱动侧对应）。"""
    n = len(gate)
    if n == 0:
        return {"n_bars": 0}
    return {
        "n_bars": n,
        "up_unexhausted_bars": sum(gate.up_unexhausted),
        "down_unexhausted_bars": sum(gate.down_unexhausted),
        "up_unexhausted_pct": sum(gate.up_unexhausted) / n * 100,
        "down_unexhausted_pct": sum(gate.down_unexhausted) / n * 100,
        "fatigue_available": gate.fatigue_available,
        "fatigued_bars": (
            sum(1 for v in gate.fatigue if v == FATIGUE_FATIGUED)
            if gate.fatigue_available else None
        ),
    }


def longest_unexhausted_window(flags: tuple[bool, ...]) -> tuple[int, int]:
    """最长连续 unexhausted 区间 `(start, end_exclusive)`（门 sanity 抽验用）。"""
    best = (0, 0)
    start = None
    for i, v in enumerate(flags):
        if v and start is None:
            start = i
        elif not v and start is not None:
            if i - start > best[1] - best[0]:
                best = (start, i)
            start = None
    if start is not None and len(flags) - start > best[1] - best[0]:
        best = (start, len(flags))
    return best
