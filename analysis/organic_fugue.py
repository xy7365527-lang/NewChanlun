"""有机赋格（Organic Fugue）— 单一 LOU × N 声部的完整递归操盘框架。

实现 `analysis/organic_fugue_design.md`（2026-06-10 设计稿）。

═══════════════════════════════════════════════════════════════════════
统一原则（§0）
═══════════════════════════════════════════════════════════════════════
多重赋格 = 同一个操盘程式（第38课同级别分解程式）在每个中枢承载层上运行；
各声部差别只有两个：账本对操作的解释（master=真实仓位，voice=共享仓位上的
短差）和该层投入的资金筹码（第40课）。只有**一个** FSM 类型
（LevelOperatingUnit），实例化 N 份。

═══════════════════════════════════════════════════════════════════════
模块拓扑（§3）
═══════════════════════════════════════════════════════════════════════
  FatigueMonitor   41课守门员：上级别衰竭证据集，守卫 OPEN_REV
  SizeAllocator    40课规模：中枢振幅占比归一 frac[k]（中枢生死事件时重算）
  LevelOperatingUnit  38课程式 FSM：RIDE（含 P5 域腿子循环）↔ REV（段尺度反向）
  OrganicLedger    共享账本：_SharedFugue 语义扩展重定义（两阶段守恒律零改动
                   继承 + rev 槽 + earning 反作用钩子）。复用 ShortDiffCycle。
  run_organic      回测主循环（ARM/区间套入场/主出场逐字承自 run_version_i）

═══════════════════════════════════════════════════════════════════════
O0≡P5 等价守卫（§6，最重要的 diff 验证）
═══════════════════════════════════════════════════════════════════════
`OrganicConfig()` 默认值 = P5 配置（rev_mode=False）。此配置下 run_organic 必须
与 `run_version_i(tape, floor_ladder=LADDER_SEG, pairing=PAIRING_VARIANTS["P5"])`
逐笔对账（trades + 短差 trace 逐位等价）。若 LOU 重写连 P5 都复现不了，框架无效。
守卫脚本：organic_fugue_backtest.py 的 O0 步（不过不进入 O1+）。

═══════════════════════════════════════════════════════════════════════
不变量（§5.5）与有效域声明
═══════════════════════════════════════════════════════════════════════
- INV-1（stock 模式，真实股数域）：任意时刻真实卖出（earning 阶段开腿的
  total_shares 扣减）不超过持有。**由构造保证**：earning 开腿 shares =
  total_shares×frac（frac≤1）且即时扣减 → Σ 真实卖出永不超持有；
  shares≤0 的开腿尝试被拒绝并计数（n_open_rejects_zero，非静默）。
  ⚠ 有效域声明：cost>0 阶段的腿是**虚拟价差收集器**（股数守恒，不动
  total_shares，P5 继承的声明建模，危险性已在 E6 落盘）——"Σ开放腿≤持仓"
  对虚拟腿是范畴错误（虚拟腿不卖出真实股票），INV-1 不施加于虚拟腿。
  这同时是 O0≡P5 逐位等价的必要条件（P5 虚拟腿无 Σ 上限）。
- INV-2：earning 腿金额守恒（卖 V 买 V，total_shares 净增，cost_basis 锁 0）
  ——`close_diff` was_earning 分支，承自 _SharedFugue 零改动。
- INV-3（futures 真实空头）：**本实现不包含**（设计 F1 为主线外探索性扩展；
  market_mode 仅接受 "stock"，传 "futures" 直接抛错——诚实声明而非半成品）。

═══════════════════════════════════════════════════════════════════════
原文锚定（§7 摘要；完整表见设计文档）
═══════════════════════════════════════════════════════════════════════
REV 腿（38课"向下段……先卖后买"）/ T5 三岔（38课）/ 41课门（"刀口舔血"）/
韵律锁定（39课）/ 两阶段守恒律（31课）/ earning 出场升级（31课"历史性大顶"）/
挣股数（43课）/ 持股持币二元（45课）/ 域腿（49课，P5 原样）/ type3 强制走（33课）。

认识论等级：框架结构 L0（原文+已结算谱系推导）；O0 守卫 L1（管线等价）；
O1-O4 变体结论 L2→L3（OKLO 主验证 + QQQ/BRN 交叉，见 organic_fugue_backtest）。

谱系：E1-E10（interval_nesting_reverse_backtest 结果包）/ 525号 / 521号 /
project_shared_position_fugue / project_organic_fugue_design。
"""

from __future__ import annotations

import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

from newchan.trading.cost_reduction_fsm import ShortDiffCycle  # noqa: E402

import fugue_alpha_diagnosis as _ef  # noqa: E402
from fugue_alpha_diagnosis import INITIAL_CAPITAL, CompletedTrade  # noqa: E402
from fugue_version_i import (  # noqa: E402
    FIRST_BSP_LADDER,
    LADDER_BAR,
    LADDER_MOVE,
    LADDER_SEG,
    MAX_LADDER,
    NO_LADDER_DIVS,
    NO_LADDER_EVENTS,
    OSC_KEY_OFFSET,
    THETA_AMP,
    BarSignalI,
    ladder_name,
)

__all__ = [
    "OrganicConfig",
    "ORGANIC_VARIANTS",
    "FatigueMonitor",
    "SizeAllocator",
    "OrganicLedger",
    "LevelOperatingUnit",
    "REV_KEY_OFFSET",
    "run_organic",
    "leg_kind",
]

# rev 腿账本键偏移（§5.5 槽空间：main=ladder / osc=+100 / rev=+200）。
REV_KEY_OFFSET = 200

_FLAT, _ARMED, _LONG = 0, 1, 2
STOP_FRAC = 0.02

# LOU 状态（voice：RIDE=正向操盘/域腿子循环开放；REV=段尺度卖腿开放）。
RIDE, REV = 0, 1


def leg_kind(key: int) -> str:
    """账本槽键 → 腿类型（报告归因用）。"""
    if key >= REV_KEY_OFFSET:
        return "rev"
    if key >= OSC_KEY_OFFSET:
        return "osc"
    return "main"


# ════════════════════════════════════════════════════════════
# 配置（变体轴 §8.1）
# ════════════════════════════════════════════════════════════

@dataclass(frozen=True)
class OrganicConfig:
    """有机赋格配置。默认值 = P5 逐字（O0 等价守卫的基线锚定）。

    P5 继承轴（与 `fugue_version_i.PairingConfig` P5 字段逐字对应）：
      open_kinds / hard_type3 / pre_type3 / center_gate / theta_amp /
      same_center_close / osc_mode / osc_buy_sub —— main 腿（P4 离开段腿）与
      域腿（P5 osc）语义零改动。

    有机扩展轴（设计 §5.2/§5.3/§5.4/§5.6）：
      rev_mode   : REV 腿 + 段终结判定（38课三触发：confirmed type1 卖 ∨
                   div(up,*) 卖侧 ∨ confirmed type3 卖）。该触发集默认同时
                   作用于 master 出场（§5.2 master 表"同 T1 触发"）。
                   False 时 master 出场 = sell1（P5 逐字）。
      master_seg_end : master 出场是否采用段终结三触发（仅 rev_mode 下有意义；
                   默认 True = 设计 §5.2 原文形态）。False = master 出场保持
                   sell1（P5 逐字），仅 voice 加 REV 腿——探索性拆解轴
                   （O1v）：OKLO 首轮数据显示 rev 腿独立净现金为正而 vs_B0
                   深负、交易数 220→803，归因指向 master 触发扩展砍暴露而
                   非 voice REV 腿本身；该混淆必须可分解才能正确裁决 REV
                   假设的边界条件（formalization-validity-domain）。
      rev_gate   : 41课门。True=OPEN_REV 须 fatigue[u(k)] 非空；False=门不设
                   （O1 消融：测段尺度反向裸 alpha）。master 门恒开（§2.3：
                   master 是最高声部，其卖点本身就是全局衰竭判定）。
      rev_close  : REV 关腿消融轴 R（§5.2）：
                   "conf"   = T5 原样（confirmed type1/2 买 ∨ div(down,
                              consolidation) 买侧）
                   "cand"   = type1 买放宽为 candidate（type2/div 不变）
                   "nested" = type1 买 candidate ∧ buy_any[k−1]（27课区间套
                              定位；type2/div 不变）
      sizing     : "equal"=1/N_sub（P5 逐字）/ "structure"=中枢振幅占比
                   frac[k]（§5.4，三类腿统一用）。
      earning_reaction : 31课 earning 反作用（§5.6）。True 且 cost_basis≤0 后：
                   master 段终结不再清仓，改开 entry 级 REV 腿（43课字面执行：
                   金额守恒挣股数）；清仓权上移到 exit_ladder_earning =
                   min(entry+1, max_ladder) 的 confirmed type1 卖；该层从未
                   给出卖点 → 持有至数据尾（31课"历史性大顶"的级别相对化）。
      market_mode: 仅 "stock"（INV-3 futures 扩展不在本实现，见模块 docstring）。
    """

    # ── P5 继承轴 ──
    open_kinds: tuple = ("type1", "type2")
    hard_type3: bool = True
    pre_type3: bool = True
    center_gate: bool = True
    theta_amp: float = THETA_AMP
    same_center_close: bool = True
    osc_mode: bool = True
    osc_buy_sub: bool = False
    # ── 有机扩展轴 ──
    rev_mode: bool = False
    master_seg_end: bool = True
    rev_gate: bool = False
    rev_close: str = "conf"
    sizing: str = "equal"
    earning_reaction: bool = False
    market_mode: str = "stock"


ORGANIC_VARIANTS: dict[str, OrganicConfig] = {
    # O0：≡P5 逐位守卫（基线锚定）
    "O0": OrganicConfig(),
    # O1：加 REV 腿，无 41课门 → 段尺度反向本身是否有 alpha
    "O1": OrganicConfig(rev_mode=True, rev_gate=False),
    # O2：加 41课门 → 门的因果增量（O2−O1）
    "O2": OrganicConfig(rev_mode=True, rev_gate=True),
    # O3：结构规模 vs 均分（O3−O2）
    "O3": OrganicConfig(rev_mode=True, rev_gate=True, sizing="structure"),
    # O4：earning 反作用（前置：触发率>0）
    "O4": OrganicConfig(rev_mode=True, rev_gate=True, sizing="structure",
                        earning_reaction=True),
    # 探索性（§8.2 判据6：不进主判决）
    "O2c": OrganicConfig(rev_mode=True, rev_gate=True, rev_close="cand"),
    "O2n": OrganicConfig(rev_mode=True, rev_gate=True, rev_close="nested"),
    # O1v：voice-only REV（master 出场保持 P5 sell1）——拆解 rev_mode 捆绑轴
    # 的归因混淆（master 触发扩展 vs voice REV 腿），exploratory
    "O1v": OrganicConfig(rev_mode=True, master_seg_end=False, rev_gate=False),
}


# ════════════════════════════════════════════════════════════
# FatigueMonitor（§5.3，41课守门员）
# ════════════════════════════════════════════════════════════

class FatigueMonitor:
    """上级别衰竭证据集 fatigue[u]，守卫 voice 的 OPEN_REV。

    证据（任一即门开，§4e）：
      1. u 层背驰事件卖侧（div_events，kind ∈ {trend, consolidation}——盘整
         背驰即可算"有衰竭迹象"）；
      2. u 层 type1 卖（candidate 即可——E4/P3：左侧信号 > 右侧确认）；
      3. u 层 confirmed type3 卖（中枢向下离开 = 上级别自身转入向下段）。
    时效：证据自事件 bar 起生效，至 u 层新的向上 move settle（创新动力 =
    衰竭被市场否定）时清空。

    u(k) = k 之上最近的有结构的承载层，封顶 entry_ladder；不存在 → 门恒关。
    "有结构（曾出现 move）"的可观测形式 = 该层曾产出任何 BSP/背驰事件
    （两者都以 move 存在为前提；事件流是引擎暴露面上 move 涌现史的严格代理）。
    """

    __slots__ = ("evidence", "structure_seen")

    def __init__(self) -> None:
        self.evidence: dict[int, set] = {}
        self.structure_seen: set = set()

    def observe(self, ladder: int, bsp_evs: tuple, div_evs: tuple,
                up_settled: bool) -> None:
        """消费某 ladder 本 bar 的事件流，更新证据集。每 bar 每承载层调用。"""
        if bsp_evs or div_evs:
            self.structure_seen.add(ladder)
        # 清空先于添加：同 bar "新向上 move settle + 新卖侧证据" 时，新证据
        # 属于 settle 之后的市场状态，保留（事件序：settle 否定的是旧证据）。
        if up_settled:
            ev = self.evidence.get(ladder)
            if ev:
                ev.clear()
        ev = None
        for e in bsp_evs:
            # e = (kind, side, seg_idx, confirmed, center_seg_start, zd, zg, price)
            if e[1] != "sell":
                continue
            if e[0] == "type1" or (e[0] == "type3" and e[3]):
                if ev is None:
                    ev = self.evidence.setdefault(ladder, set())
                ev.add(("bsp", e[0], e[2], e[3]))
        for d in div_evs:
            # d = (kind, direction, side, seg_idx, force_a, force_c, price)
            if d[2] == "sell":
                if ev is None:
                    ev = self.evidence.setdefault(ladder, set())
                ev.add(("div", d[0], d[3]))

    def u_of(self, k: int, entry_ladder: int) -> int | None:
        """k 之上最近曾涌现结构的承载层（≤ entry_ladder）。无 → None。"""
        for u in range(k + 1, entry_ladder + 1):
            if u in self.structure_seen:
                return u
        return None

    def gate_open(self, k: int, entry_ladder: int) -> bool:
        u = self.u_of(k, entry_ladder)
        if u is None:
            return False
        return bool(self.evidence.get(u))


# ════════════════════════════════════════════════════════════
# SizeAllocator（§5.4，40课规模）
# ════════════════════════════════════════════════════════════

class SizeAllocator:
    """中枢振幅占比归一的声部资金分配 frac[k]。

    A_k = (ZG_k − ZD_k)/c（k 层当前存活中枢的相对振幅）；无存活中枢 → A_k=0
    （没有结构域就没有操作权，与 E2 域腿 center_alive 同构）；
    frac[k] = A_k / Σ_j A_j（Σ=0 → 全 0）。
    重算时机 = 任一层中枢生死事件（center book 版本号变化，不逐 bar 避免
    churn）；腿 open 时读快照，存续期内固定（open_diff 的 frac 入参语义）。

    认识论标注（§4d）：振幅正比公式是 L0 推导（"每一重都对应着一定的资金与
    筹码"的最简结构化解释），其优于均分是待验证假设（O3 vs O2 裁决）。
    """

    __slots__ = ("frac", "_version")

    def __init__(self) -> None:
        self.frac: dict[int, float] = {}
        self._version = -1

    def maybe_recompute(self, version: int, active_levels: list,
                        last_center: dict, dead_centers: dict, c: float) -> None:
        if version == self._version:
            return
        self._version = version
        amps: dict[int, float] = {}
        for k in active_levels:
            lc = last_center.get(k)
            if lc is None or lc[0] in dead_centers.get(k, ()):
                continue
            amp = (lc[2] - lc[1]) / c if c > 0 else 0.0
            if amp > 0:
                amps[k] = amp
        total = sum(amps.values())
        self.frac = ({k: a / total for k, a in amps.items()} if total > 0 else {})


# ════════════════════════════════════════════════════════════
# OrganicLedger（§5.5）— _SharedFugue 的语义扩展重定义
# ════════════════════════════════════════════════════════════

@dataclass
class OrganicLedger:
    """共享仓位 + 三类腿（main/osc/rev）并发短差账本。

    不修改 `_SharedFugue`（它是 P1-P7 回归基线的组成部分；no-patch：语义扩展
    用新账本完整重写）。`open_diff`/`close_diff` 的守恒律算术与 _SharedFugue
    逐字一致（O0≡P5 逐位等价的账本前提）；扩展仅有：
      - open_diff 返回 bool（LOU 需要知道腿是否实际开启才转换状态）；
      - n_open_rejects_zero 计数器（shares≤0 / 槽占用 / 冻结的开腿拒绝，
        非静默——INV-1 真实股数域的可观测面）。
    """

    entry_price: float
    total_shares: float
    cost_basis: float
    level_frac: float = 0.0
    cumulative_recovered: float = 0.0
    earning: bool = False
    # key → (ShortDiffCycle, was_earning, anchor)；key ∈ {ladder, ladder+100,
    # ladder+200}；anchor = (center_seg_start, 边界价, kind) | None（rev 腿 None
    # ——段尺度腿无中枢锚，§4b：价格已离开中枢，T7 冻结走市场级 center book）。
    active: dict = field(default_factory=dict)
    completed: list = field(default_factory=list)
    stopped: set = field(default_factory=set)
    n_open_rejects_zero: int = 0
    trace: list | None = None
    _diff_open: dict = field(default_factory=dict)

    def open_diff(self, key: int, frac: float, sell_price: float, bar: int = -1,
                  anchor: tuple | None = None) -> bool:
        """开腿（高抛/REV 先卖）。返回是否实际开启。

        守恒律算术与 _SharedFugue.open_diff 逐字一致；earning 阶段真实减仓
        （INV-1 由构造保证：shares = total×frac, frac≤1, 即时扣减）。
        """
        if key in self.active or key in self.stopped:
            return False
        shares = self.total_shares * frac
        if shares <= 0:
            self.n_open_rejects_zero += 1
            return False
        cyc = ShortDiffCycle(level=f"L{key}", shares=shares, sell_price=sell_price)
        self.active[key] = (cyc, self.earning, anchor)
        if self.earning:
            self.total_shares -= shares
        if self.trace is not None:
            self._diff_open[key] = (bar, sell_price)
        return True

    def close_diff(self, key: int, buy_price: float, bar: int = -1) -> None:
        """闭腿（低吸/REV 买回）。profit 写入共享 cost_basis（守恒律由 open
        时阶段 was_earning 决定，与 _SharedFugue 逐字一致）。"""
        rec = self.active.get(key)
        if rec is None or buy_price <= 0:
            return
        cyc, was_earning, _anchor = rec
        cb_before = self.cost_basis
        shares_before = self.total_shares
        closed = ShortDiffCycle(
            level=cyc.level, shares=cyc.shares, sell_price=cyc.sell_price,
            buy_price=buy_price, is_open=False)
        if was_earning:
            # INV-2 金额守恒：卖 V 买 V，total_shares 净增，cost_basis 锁 0
            self.total_shares += cyc.shares * cyc.sell_price / buy_price
        else:
            # 股数守恒：价差 profit 落袋 + 降共享 cost_basis
            profit = closed.profit
            self.cumulative_recovered += profit
            if self.total_shares > 0:
                self.cost_basis -= profit / self.total_shares
            if self.cost_basis <= 0:
                self.cost_basis = 0.0
                self.earning = True
        del self.active[key]
        self.completed.append((key, closed))
        if self.trace is not None:
            sell_bar, _sp = self._diff_open.pop(key, (-1, cyc.sell_price))
            self.trace.append({
                "ladder": key,
                "leg": leg_kind(key),
                "sell_bar": sell_bar,
                "sell_price": cyc.sell_price,
                "buy_bar": bar,
                "buy_price": buy_price,
                "shares": cyc.shares,
                "diff": (cyc.sell_price - buy_price),
                "profit": closed.profit,
                "was_earning": was_earning,
                "shares_delta": self.total_shares - shares_before,
                "cost_basis_before": cb_before,
                "cost_basis_after": self.cost_basis,
            })


# ════════════════════════════════════════════════════════════
# LevelOperatingUnit（§5.2，38课程式 FSM）
# ════════════════════════════════════════════════════════════

class LevelOperatingUnit:
    """单个中枢承载层的操盘单元（voice 实例；master 循环在 run_organic 主体）。

    状态：RIDE（持多语义：域腿子循环开放）/ REV（段尺度卖腿开放）。
    master 建仓时全部 voice 初始化为 RIDE。

    转换表（§5.2）：
      T1 RIDE→REV  confirmed type1卖 ∨ div(up,*)卖侧 ∨ confirmed type3卖，
                   守卫 = 门开 ∧ k∉frozen；动作 = 先 CLOSE_OSC（若开）→ OPEN_REV
      T2 RIDE→RIDE 同 T1 触发但门关 → 无动作（域腿照常）
      T3/T4        P5 域腿逐字（开 = c≥ZG ∧ sell_any[k−1] ∧ 中枢存活 ∧ 非冻结；
                   平 = c≤锚ZD（∧ osc_buy_sub 时次级别买点） ∨ 锚中枢死亡）
      T5 REV→RIDE  关腿触发按 rev_close 消融轴（conf/cand/nested）
      T6 REV→RIDE  candidate type3 买 → 预逃逸
      T7 REV→RIDE  confirmed type3 买 → 逃逸 + 冻结（冻结由市场级 center book
                   的 hard_type3 机制执行——confirmed type3 买在 center 维护中
                   已置 frozen[k]，LOU 不重复置位）
    同 bar 优先级：逃逸（T6/T7）> 正常关腿（T5）> 开腿（T1/T3）——与 P5
    pairing 分支"先平后开"事件序一致。

    rev_mode=False 时 step 仅执行 T3/T4（P5 域腿逐字）——O0≡P5 的 LOU 前提。
    """

    __slots__ = ("ladder", "state")

    def __init__(self, ladder: int) -> None:
        self.ladder = ladder
        self.state = RIDE

    # —— REV 触发判定（静态谓词，master 复用同一段终结语义 §5.2）——

    @staticmethod
    def seg_end_trigger(evs: tuple, devs: tuple) -> bool:
        """38课段终结三触发：confirmed type1 卖 ∨ div(up,*) 卖侧 ∨ confirmed type3 卖。"""
        for e in evs:
            if e[3] and e[1] == "sell" and (e[0] == "type1" or e[0] == "type3"):
                return True
        for d in devs:
            if d[2] == "sell":
                return True
        return False

    @staticmethod
    def _rev_close_trigger(cfg: OrganicConfig, evs: tuple, devs: tuple,
                           sub_buy: bool) -> bool:
        """T5：38课三岔（不跌破低点=type2 买 / 跌破+盘背=div(down,consolidation)
        / 新下跌背驰=type1 买），按 rev_close 消融轴调整 type1 分量。"""
        for e in evs:
            if e[1] != "buy":
                continue
            if e[0] == "type1":
                if cfg.rev_close == "conf":
                    if e[3]:
                        return True
                elif cfg.rev_close == "cand":
                    return True
                else:  # nested：candidate type1 买 ∧ 次级别买点（27课区间套）
                    if sub_buy:
                        return True
            elif e[0] == "type2" and e[3]:
                return True
        for d in devs:
            if d[0] == "consolidation" and d[1] == "down" and d[2] == "buy":
                return True
        return False

    def step(self, cfg: OrganicConfig, sig: BarSignalI, c: float, bar: int,
             pos: OrganicLedger, evs: tuple, devs: tuple,
             last_center: dict, dead_centers: dict, frozen: dict,
             fatigue: FatigueMonitor, entry_ladder: int, frac: float,
             counters: dict) -> None:
        """每 bar 一步（仅 _LONG 态由主循环调用）。修改 pos/counters/自身状态。"""
        k = self.ladder
        okey = k + OSC_KEY_OFFSET
        rkey = k + REV_KEY_OFFSET

        # ── REV 态：关腿检查（优先级 T6/T7 逃逸 > T5 正常关）──
        if self.state == REV:
            pre = cfg.pre_type3 and any(
                (not e[3]) and e[0] == "type3" and e[1] == "buy" for e in evs)
            hard = any(e[3] and e[0] == "type3" and e[1] == "buy" for e in evs)
            if hard:
                pos.close_diff(rkey, c, bar)
                counters["n_rev_close_t7"] += 1
                self.state = RIDE  # 冻结已由 center book hard_type3 机制置位
            elif pre:
                pos.close_diff(rkey, c, bar)
                counters["n_rev_close_t6"] += 1
                self.state = RIDE
            elif self._rev_close_trigger(
                    cfg, evs, devs, sig.buy_any[k - 1] if k >= 1 else False):
                pos.close_diff(rkey, c, bar)
                counters["n_rev_close_t5"] += 1
                self.state = RIDE

        # ── RIDE 态：T1（REV 开腿）──
        if self.state == RIDE and cfg.rev_mode and (evs or devs):
            if self.seg_end_trigger(evs, devs):
                counters["n_rev_attempts"] += 1
                gate_ok = (not cfg.rev_gate
                           or fatigue.gate_open(k, entry_ladder))
                if not gate_ok:
                    counters["n_rev_gate_rejects"] += 1
                elif k in frozen:
                    counters["n_rev_frozen_rejects"] += 1
                else:
                    if pos.active.get(okey) is not None:
                        pos.close_diff(okey, c, bar)  # 先 CLOSE_OSC（T1 动作）
                    if pos.open_diff(rkey, frac, c, bar, anchor=None):
                        counters["n_rev_open"] += 1
                        self.state = REV

        # ── 域腿子循环（T3/T4 = P5 逐字；REV 态下 osc 已关、不开新腿）──
        if cfg.osc_mode and self.state == RIDE:
            orec = pos.active.get(okey)
            lc = last_center.get(k)
            center_alive = (lc is not None and lc[0]
                            not in dead_centers.get(k, ()))
            if orec is not None:
                o_anchor = orec[2]
                o_dead = (o_anchor is not None and o_anchor[0]
                          in dead_centers.get(k, ()))
                if o_dead:
                    pos.close_diff(okey, c, bar)  # 中枢死亡 → 强制回补
                elif (o_anchor is not None and c <= o_anchor[1]
                      and (not cfg.osc_buy_sub or sig.buy_any[k - 1])):
                    pos.close_diff(okey, c, bar)
                    counters["n_osc_zd_close"] += 1
            elif (center_alive and k not in frozen
                  and k >= 1 and c >= lc[2]
                  and sig.sell_any[k - 1]):
                if pos.open_diff(okey, frac, c, bar, anchor=(lc[0], lc[1], "osc")):
                    counters["n_osc_open"] += 1


# ════════════════════════════════════════════════════════════
# 回测主循环（§3 数据流：磁带 → 账本/监视器 → LOU → 账本操作）
# ════════════════════════════════════════════════════════════

def run_organic(
    i_signals: list[BarSignalI],
    *,
    floor_ladder: int = LADDER_SEG,
    config: OrganicConfig | None = None,
    stop_mode: str = "none",
    diag: list | None = None,
) -> tuple[list[CompletedTrade], dict]:
    """有机赋格回测。进出场/ARM/区间套入场逐字承自 `run_version_i`。

    `config=None` → OrganicConfig() 默认值 = P5 配置（O0）。
    磁带要求：bsp_events 非全空（事件磁带）；rev_mode 时 div_events 亦须可用
    （organic_signals 产出；布尔磁带直接抛错——能力守卫，声明=能力）。
    """
    cfg = config or OrganicConfig()
    if cfg.market_mode != "stock":
        raise NotImplementedError(
            "market_mode='futures'（INV-3 真实空头）是设计 F1 主线外扩展，"
            "本实现不包含（见模块 docstring 不变量节）")
    if cfg.rev_close not in ("conf", "cand", "nested"):
        raise ValueError(f"rev_close 非法：{cfg.rev_close!r}")
    if floor_ladder < FIRST_BSP_LADDER:
        raise ValueError(
            f"有机赋格要求 floor_ladder ≥ {FIRST_BSP_LADDER}（中枢承载层）；"
            f"bar/bi 级无 kind/中枢概念。floor_ladder={floor_ladder}")
    if not any(s.bsp_events for s in i_signals):
        raise ValueError("有机赋格要求事件磁带（BarSignalI.bsp_events 全空）")
    if cfg.rev_mode and not any(s.div_events for s in i_signals):
        raise ValueError(
            "rev_mode 要求背驰磁带（BarSignalI.div_events 全空——该磁带"
            "不是 organic_signals 产出）")

    n = len(i_signals)
    state = _FLAT
    entry_bar = -1; entry_price = 0.0; entry_ladder = -1; arm_bar = -1
    arm_ladder = LADDER_MOVE
    pos: OrganicLedger | None = None
    active_levels: list[int] = []
    voices: dict[int, LevelOperatingUnit] = {}
    master_state = RIDE          # earning 反作用下 master 的 RIDE/REV（§5.6）
    SUB_EXPIRY = _ef.SUB_EXPIRY
    n_addon = 0
    n_core_stops = 0
    _stop_on = stop_mode in ("A", "B")

    # ── 市场级中枢生命周期账本（跨 trade 持续；与 run_version_i 逐字一致）──
    last_center: dict[int, tuple] = {}
    dead_centers: dict[int, set] = {}
    frozen: dict[int, int] = {}
    center_version = 0           # 中枢生死/冻结事件版本号（SizeAllocator 重算门控）

    fatigue = FatigueMonitor()
    allocator = SizeAllocator()

    counters: dict[str, int] = {
        "n_close_normal": 0, "n_close_pre_type3": 0, "n_close_hard_type3": 0,
        "n_open_gate_rejects": 0, "n_osc_open": 0, "n_osc_zd_close": 0,
        "n_rev_attempts": 0, "n_rev_gate_rejects": 0, "n_rev_frozen_rejects": 0,
        "n_rev_open": 0, "n_rev_close_t5": 0, "n_rev_close_t6": 0,
        "n_rev_close_t7": 0, "n_master_rev_open": 0, "n_master_rev_close": 0,
        "n_earning_reached": 0, "n_exit_upgraded": 0, "n_open_rejects_zero": 0,
    }
    rev_attempts_by_ladder: dict[int, int] = {}
    rev_opens_by_ladder: dict[int, int] = {}

    trades: list[CompletedTrade] = []
    ladder_attribution: dict[int, int] = {}
    ladder_held_bars: dict[int, int] = {}
    fsm_recovered: dict[int, float] = {}
    fsm_diffs: dict[int, int] = {}

    def _frac_for(k: int) -> float:
        if cfg.sizing == "structure":
            return allocator.frac.get(k, 0.0)
        return pos.level_frac

    def _open(bar_idx: int, price: float, el: int) -> None:
        nonlocal state, entry_bar, entry_price, entry_ladder, pos, \
            active_levels, voices, master_state
        entry_bar = bar_idx; entry_price = price; entry_ladder = el
        active_levels = list(range(floor_ladder, el))
        n_sub = len(active_levels)
        level_frac = (1.0 / n_sub) if n_sub > 0 else 0.0
        pos = OrganicLedger(
            entry_price=price, total_shares=INITIAL_CAPITAL / price,
            cost_basis=price, level_frac=level_frac,
            trace=([] if diag is not None else None))
        voices = {k: LevelOperatingUnit(k) for k in active_levels}
        master_state = RIDE
        state = _LONG

    def _close(bar_idx: int, price: float, reason: str) -> None:
        nonlocal state, entry_bar, entry_price, entry_ladder, pos, \
            active_levels, voices, master_state
        if pos is None or entry_price <= 0 or pos.total_shares <= 0:
            state = _FLAT; pos = None; active_levels = []; voices = {}
            master_state = RIDE
            return
        for key in list(pos.active.keys()):
            pos.close_diff(key, price, bar_idx)
        total_value = pos.total_shares * price + pos.cumulative_recovered
        pnl_pct = (total_value - INITIAL_CAPITAL) / INITIAL_CAPITAL * 100
        held = bar_idx - entry_bar
        trades.append(CompletedTrade(
            entry_bar=entry_bar, entry_price=entry_price, exit_bar=bar_idx,
            exit_price=price, pnl_pct=round(pnl_pct, 4), exit_reason=reason,
            n_short_diffs=len(pos.completed),
            cost_basis_at_exit=round(pos.cost_basis, 6)))
        if pos.earning:
            counters["n_earning_reached"] += 1
        counters["n_open_rejects_zero"] += pos.n_open_rejects_zero
        if diag is not None:
            diag.append({
                "entry_bar": entry_bar, "entry_price": entry_price,
                "exit_bar": bar_idx, "exit_price": price, "exit_reason": reason,
                "entry_ladder": entry_ladder,
                "entry_ladder_name": ladder_name(entry_ladder),
                "pnl_pct": round(pnl_pct, 4),
                "cost_basis_entry": entry_price,
                "cost_basis_exit": pos.cost_basis,
                "total_shares_entry": INITIAL_CAPITAL / entry_price,
                "total_shares_exit": pos.total_shares,
                "reached_earning": pos.earning,
                "n_diffs": len(pos.trace),
                "diffs": list(pos.trace),
            })
        for key, cyc in pos.completed:
            fsm_recovered[key] = fsm_recovered.get(key, 0.0) + cyc.profit
            fsm_diffs[key] = fsm_diffs.get(key, 0) + 1
        ladder_attribution[entry_ladder] = ladder_attribution.get(entry_ladder, 0) + 1
        ladder_held_bars[entry_ladder] = ladder_held_bars.get(entry_ladder, 0) + held
        state = _FLAT; entry_price = 0.0; entry_ladder = -1; pos = None
        active_levels = []; voices = {}; master_state = RIDE

    for i in range(n):
        sig = i_signals[i]
        c = sig.close
        evrows = sig.bsp_events or NO_LADDER_EVENTS
        devrows = sig.div_events or NO_LADDER_DIVS
        ums = sig.up_move_settled

        # ── 中枢生命周期账本（每 bar，市场性质）。与 run_version_i 的差异
        # 仅两处且均不改变 book 内容：(1) center_version 自增（SizeAllocator
        # 重算门控的纯观测计数）；(2) dead.add/last_center 写入前的 not-in
        # 守卫（为版本号判定服务；set.add/dict 赋值本就幂等 → book 状态
        # 逐位等价，O0 守卫 + 合成对账单测验证）──
        if evrows is not NO_LADDER_EVENTS:
            for lad in range(FIRST_BSP_LADDER, MAX_LADDER):
                for ev in evrows[lad]:
                    kind, side, confirmed, cs = ev[0], ev[1], ev[3], ev[4]
                    if cs is None:
                        continue
                    dead = dead_centers.setdefault(lad, set())
                    if confirmed and kind == "type3":
                        if cs not in dead:
                            dead.add(cs)
                            center_version += 1
                        if cfg.hard_type3 and side == "buy":
                            frozen[lad] = cs
                    elif cs not in dead:
                        if last_center.get(lad) != (cs, ev[5], ev[6]):
                            center_version += 1
                        last_center[lad] = (cs, ev[5], ev[6])
                        if lad in frozen and frozen[lad] != cs:
                            del frozen[lad]

        # ── FatigueMonitor（市场性质，每 bar；仅 rev 模式消费，但证据积累
        # 与仓位无关 → 始终维护，跨 trade 有效）──
        if cfg.rev_mode:
            for lad in range(FIRST_BSP_LADDER, MAX_LADDER):
                bsp_l = evrows[lad] if evrows is not NO_LADDER_EVENTS else ()
                div_l = devrows[lad] if devrows is not NO_LADDER_DIVS else ()
                up_l = bool(ums) and bool(ums[lad])
                if bsp_l or div_l or up_l:
                    fatigue.observe(lad, bsp_l, div_l, up_l)

        if state == _FLAT:
            hi = -1
            for k in range(FIRST_BSP_LADDER, min(sig.max_ladder + 1, MAX_LADDER)):
                if sig.buy1[k]:
                    hi = k
            if hi >= FIRST_BSP_LADDER:
                state = _ARMED; arm_bar = i; arm_ladder = hi

        elif state == _ARMED:
            for k in range(FIRST_BSP_LADDER, min(sig.max_ladder + 1, MAX_LADDER)):
                if sig.buy1[k] and k > arm_ladder:
                    arm_ladder = k
            do_enter = any(sig.buy_any[k] for k in range(LADDER_BAR, arm_ladder))
            if not do_enter and (i - arm_bar) > SUB_EXPIRY:
                do_enter = True
            if do_enter:
                _open(i, c, arm_ladder)
            elif sig.sell1[arm_ladder]:
                state = _FLAT

        elif state == _LONG:
            # 结构规模：中枢生死事件门控重算（churn 避免，§5.4）
            if cfg.sizing == "structure":
                allocator.maybe_recompute(
                    center_version, active_levels, last_center, dead_centers, c)

            ev_entry = evrows[entry_ladder]
            dev_entry = devrows[entry_ladder]

            # ── master 循环（持股↔持币，45课；§5.2 master 表）──
            if _stop_on and c < entry_price * (1.0 - STOP_FRAC):
                n_core_stops += 1
                _close(i, c, "stop_core_2pct")
            elif master_state == REV:
                # earning 反作用下的 master REV 腿（entry 级先卖后买，43课）：
                # 关腿按 T5-T7（与 voice 同一程式——单一 FSM 多声部转位）。
                mrkey = entry_ladder + REV_KEY_OFFSET
                pre = cfg.pre_type3 and any(
                    (not e[3]) and e[0] == "type3" and e[1] == "buy"
                    for e in ev_entry)
                hard = any(e[3] and e[0] == "type3" and e[1] == "buy"
                           for e in ev_entry)
                t5 = LevelOperatingUnit._rev_close_trigger(
                    cfg, ev_entry, dev_entry,
                    sig.buy_any[entry_ladder - 1] if entry_ladder >= 1 else False)
                if hard or pre or t5:
                    pos.close_diff(mrkey, c, i)
                    counters["n_master_rev_close"] += 1
                    master_state = RIDE
                # 升级出场（31课"历史性大顶"的级别相对化）在 REV 态同样有效
                exit_lad = min(entry_ladder + 1, sig.max_ladder)
                if exit_lad > entry_ladder and sig.sell1[exit_lad]:
                    counters["n_exit_upgraded"] += 1
                    _close(i, c, f"exit_{ladder_name(exit_lad)}_earning_upgrade")
            else:
                # master RIDE：段终结判定
                if cfg.rev_mode and cfg.master_seg_end:
                    master_trigger = LevelOperatingUnit.seg_end_trigger(
                        ev_entry, dev_entry) or sig.sell1[entry_ladder]
                else:
                    # P5 逐字（rev 关，或 O1v：voice-only REV 拆解轴）
                    master_trigger = sig.sell1[entry_ladder]
                if master_trigger:
                    if (cfg.earning_reaction and pos.earning):
                        # §5.6：清仓降格为 entry 级 REV 腿（金额守恒挣股数）；
                        # frac=1.0 = 全仓可卖余量（earning 开腿即时扣减 →
                        # INV-1 由构造满足）
                        mrkey = entry_ladder + REV_KEY_OFFSET
                        if pos.open_diff(mrkey, 1.0, c, i, anchor=None):
                            counters["n_master_rev_open"] += 1
                            master_state = REV
                        else:
                            _close(i, c,
                                   f"exit_{ladder_name(entry_ladder)}_type1sell")
                    else:
                        _close(i, c,
                               f"exit_{ladder_name(entry_ladder)}_type1sell")
                elif cfg.earning_reaction and pos.earning:
                    # 升级出场条件持续有效（master RIDE 且已 earning）
                    exit_lad = min(entry_ladder + 1, sig.max_ladder)
                    if exit_lad > entry_ladder and sig.sell1[exit_lad]:
                        counters["n_exit_upgraded"] += 1
                        _close(i, c,
                               f"exit_{ladder_name(exit_lad)}_earning_upgrade")

            if state != _LONG:
                continue  # master 已清仓

            # ── voice 声部（main 腿 = P4 离开段腿，LOU 外围机制；osc/rev = LOU）──
            for ladder in active_levels:
                evs = evrows[ladder]
                devs = devrows[ladder]
                # main 腿闭腿（P5 逐字：normal > pre > hard）
                rec = pos.active.get(ladder)
                if rec is not None and evs:
                    anchor = rec[2]
                    normal = any(
                        e[3] and e[0] == "type1" and e[1] == "buy"
                        and (not cfg.same_center_close or anchor is None
                             or e[4] == anchor[0])
                        for e in evs)
                    pre = cfg.pre_type3 and any(
                        (not e[3]) and e[0] == "type3" and e[1] == "buy"
                        for e in evs)
                    hard = cfg.hard_type3 and any(
                        e[3] and e[0] == "type3" and e[1] == "buy" for e in evs)
                    if normal:
                        pos.close_diff(ladder, c, i)
                        counters["n_close_normal"] += 1
                    elif pre:
                        pos.close_diff(ladder, c, i)
                        counters["n_close_pre_type3"] += 1
                    elif hard:
                        pos.close_diff(ladder, c, i)
                        counters["n_close_hard_type3"] += 1
                # main 腿开腿（P5 逐字）
                if (cfg.open_kinds and evs
                        and pos.active.get(ladder) is None
                        and ladder not in frozen):
                    for e in evs:
                        if not (e[3] and e[1] == "sell"
                                and e[0] in cfg.open_kinds):
                            continue
                        if cfg.center_gate:
                            cs, zg = e[4], e[6]
                            if (cs is None
                                    or cs in dead_centers.get(ladder, ())
                                    or c <= zg
                                    or (c - zg) / c < cfg.theta_amp):
                                counters["n_open_gate_rejects"] += 1
                                continue
                        pos.open_diff(ladder, _frac_for(ladder), c, i,
                                      anchor=(e[4], e[6], e[0]))
                        break
                # LOU（osc 域腿 + rev 段尺度腿）
                lou = voices[ladder]
                if (cfg.rev_mode and lou.state == RIDE and (evs or devs)
                        and LevelOperatingUnit.seg_end_trigger(evs, devs)):
                    # 触发尝试按 ladder 细分（门开率报告 §8.2-2）
                    rev_attempts_by_ladder[ladder] = \
                        rev_attempts_by_ladder.get(ladder, 0) + 1
                n_rev_before = counters["n_rev_open"]
                lou.step(cfg, sig, c, i, pos, evs, devs,
                         last_center, dead_centers, frozen, fatigue,
                         entry_ladder, _frac_for(ladder), counters)
                if counters["n_rev_open"] > n_rev_before:
                    rev_opens_by_ladder[ladder] = \
                        rev_opens_by_ladder.get(ladder, 0) + 1
            if sig.type2_buy:
                n_addon += 1

    if state == _LONG and pos is not None and pos.total_shares > 0:
        _close(n - 1, i_signals[-1].close, "eod_close")

    # ── 归因/报告 ──
    leg_contrib: dict[str, dict] = {}
    for key in sorted(set(fsm_recovered) | set(fsm_diffs)):
        name = f"{leg_kind(key)}:{ladder_name(key % 100)}"
        d = leg_contrib.setdefault(name, {"recovered_cash": 0.0, "n_short_diffs": 0})
        d["recovered_cash"] = round(d["recovered_cash"]
                                    + fsm_recovered.get(key, 0.0), 1)
        d["n_short_diffs"] += fsm_diffs.get(key, 0)
    avg_held = {
        k: round(ladder_held_bars[k] / ladder_attribution[k], 1)
        for k in ladder_attribution if ladder_attribution[k]
    }
    extra = {
        "config": {
            "open_kinds": list(cfg.open_kinds), "hard_type3": cfg.hard_type3,
            "pre_type3": cfg.pre_type3, "center_gate": cfg.center_gate,
            "theta_amp": cfg.theta_amp,
            "same_center_close": cfg.same_center_close,
            "osc_mode": cfg.osc_mode, "osc_buy_sub": cfg.osc_buy_sub,
            "rev_mode": cfg.rev_mode, "master_seg_end": cfg.master_seg_end,
            "rev_gate": cfg.rev_gate,
            "rev_close": cfg.rev_close, "sizing": cfg.sizing,
            "earning_reaction": cfg.earning_reaction,
        },
        "counters": dict(counters),
        "rev_attempts_by_ladder": {ladder_name(k): v for k, v
                                   in sorted(rev_attempts_by_ladder.items())},
        "rev_opens_by_ladder": {ladder_name(k): v for k, v
                                in sorted(rev_opens_by_ladder.items())},
        "ladder_attribution": {ladder_name(k): v
                               for k, v in ladder_attribution.items()},
        "ladder_avg_held_bars": {ladder_name(k): v for k, v in avg_held.items()},
        "leg_contribution": leg_contrib,
        "addon_2buy_marks": n_addon,
        "n_core_stops": n_core_stops,
        "floor_ladder": floor_ladder,
        "stop_mode": stop_mode,
    }
    return trades, extra
