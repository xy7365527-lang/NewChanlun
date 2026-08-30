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

## bar 对齐（#1314）

门列与驱动 closes 出自同一份 1min json 但**清洗口径不同**（dump 另剔 OHLC ≤0，驱动
只剔 nan 且 `zip` 截到最短数组）。bar 数不等时按 `_align_start` 的首/末 close 锚定
规则取门列窗口（截头或截尾），两端都不锚即 fail-fast。多出的 bar 落在头/尾/内部的
定位用 `analysis/_diag_gate_bar_alignment.py`（逐 bar 时间戳比对，给出处置建议）。

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

# 对齐诊断命令（#1314：门列/驱动 bar 数不一致时先定位差在头/尾/内部）。
DIAG_CMD = (
    "PYTHONPATH=src:analysis uv run python analysis/_diag_gate_bar_alignment.py <SYM>"
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


def _align_start(
    sym: str,
    n: int,
    first_c: float,
    last_c: float,
    closes: list[float],
) -> int:
    """门列 ↔ 驱动 closes 的 bar 位移：返回门列内的对齐起点 `start`。

    锚哪端由 dump 侧首/末 close 与驱动侧的逐位相等决定（列文件头只存这两个 close）：

    - **只有末 close 锚定** ⇒ 多出的在**头部**（dump 覆盖到 prereg 窗外更早数据）
      ⇒ `start = n - len(closes)`（#1312 已落，本次仅修列切片，见调用点）；
    - **只有首 close 锚定** ⇒ 多出的在**尾部**（dump 磁带比驱动 closes 多覆盖到更晚
      的 bar）⇒ `start = 0`（#1314 DX：门列 n=2,058,424 vs 驱动 2,058,418，末 close
      不锚 ⇒ 多出的 6 根含尾部）；
    - **两端都锚定但 bar 数不等** ⇒ 多出的在**内部**（首末两根都在，中间多剔/多留）
      ⇒ 下标 join 不可救（截哪端都错位），fail-fast；
    - **门列少于驱动**（`n < len(closes)`）⇒ 截门列取不满 `len(closes)` 根，任一端
      锚定都救不了，fail-fast；
    - **两端都不锚** ⇒ 不同源/不同窗，fail-fast。

    两条截取臂的前提都是"被保留的那 `len(closes)` 根内部无剔除"——首末锚定核不到内部。
    内部是否另有剔除由对齐诊断（`analysis/_diag_gate_bar_alignment.py`，逐 bar 时间戳
    比对）判定，本函数不越权推断，只把不可判的两种情形拒掉并点名诊断命令。
    """
    n_drive = len(closes)
    if n == n_drive:
        return 0
    skip = n - n_drive
    if not n_drive:  # 驱动侧空序列：无可锚的 close，直接判不同源
        raise ValueError(
            f"[{sym}] 门列 bar 数 {n} ≠ 驱动侧 closes 0——驱动侧无 bar，无法锚定对齐。"
            f"重跑 dump：{REGEN_CMD.replace('<SYM>', sym)}"
        )
    head_anchors = first_c == closes[0]
    tail_anchors = last_c == closes[-1]
    if skip > 0 and tail_anchors and not head_anchors:
        print(
            f"[{sym}] 门列 {n} 根 > 驱动 {n_drive}：尾部对齐截取，舍弃头部 {skip} 根"
            f"（门列末 close {last_c} == 驱动末 close，对齐成立）"
        )
        return skip
    if skip > 0 and head_anchors and not tail_anchors:
        print(
            f"[{sym}] 门列 {n} 根 > 驱动 {n_drive}：头部对齐截取，舍弃尾部 {skip} 根"
            f"（门列首 close {first_c} == 驱动首 close，对齐成立）"
        )
        return 0
    if skip < 0:
        # 门列比驱动**少** bar：截门列取不出 len(closes) 根，任一端锚定都救不了。
        why = "门列 bar 数少于驱动 ⇒ 门列窗口不足，无可截取的对齐窗（门列过期/窗更短）"
    elif head_anchors and tail_anchors:
        why = "首末两端都锚定但 bar 数不等 ⇒ 多出的 bar 在内部，截哪端都错位"
    else:
        why = "首末 close 都不锚定 ⇒ 磁带与回测输入不同源/不同窗"
    raise ValueError(
        f"[{sym}] 门列 bar 数 {n} ≠ 驱动侧 closes {n_drive}（差 {skip:+d} 根）：{why}"
        f"（门列首/末 = {first_c}/{last_c}，驱动首/末 = {closes[0]}/{closes[-1]}）"
        f"——逐 bar join 会错位，拒绝对齐。先跑对齐诊断定位差在头/尾/内部："
        f"{DIAG_CMD.replace('<SYM>', sym)}；或重跑 dump："
        f"{REGEN_CMD.replace('<SYM>', sym)}"
    )


def load_state_gate(
    sym: str,
    closes: list[float],
    *,
    use_fatigue: bool = False,
) -> StateGate:
    """读 v3 门状态列并与驱动侧 closes 做接缝校验（bar 数 + 锚定端 close 逐位）。

    接缝校验是必需的：门列来自 `_dump_tape_rust.py` 的清洗后 bar 序列，驱动侧
    closes 来自 `load_ohlc` 的清洗（**两者口径不同**：dump 另剔 OHLC ≤0 的 bar，
    驱动只剔 nan——#1314 核对结论，见 `_diag_gate_bar_alignment.py`）——两者对齐是
    "逐 bar 列按 bar 下标 join"的前提，不对齐即错位读数，故 fail-fast 而非静默截断。

    bar 数不等时按 `_align_start` 的锚定规则取门列的对齐窗口（三列各自按同一位移
    切片），锚不上即 fail-fast。
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
    start = _align_start(sym, n, first_c, last_c, closes)
    n_keep = len(closes)
    # 锚定端才可核 close：截了头 ⇒ 首 close 不可比，截了尾 ⇒ 末 close 不可比
    # （列文件头只存原始首末两根的 close）。bar 数相等时两端都核，与 #1312 同。
    head_ok = start > 0 or not n_keep or first_c == closes[0]
    tail_ok = start + n_keep < n or not n_keep or last_c == closes[-1]
    if not (head_ok and tail_ok):
        raise ValueError(
            f"[{sym}] 门列首尾 close ({first_c}, {last_c}) ≠ 驱动侧 "
            f"({closes[0]}, {closes[-1]})——磁带与回测输入不同源，拒绝错位 join"
        )
    # 三列各自按同一位移切片（列在文件里是三段连续区，整体平移会跨列错位）。
    off = HEADER_SIZE + start
    up = raw[off:off + n_keep]
    dn = raw[off + n:off + n + n_keep]
    fat = raw[off + 2 * n:off + 2 * n + n_keep]
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
