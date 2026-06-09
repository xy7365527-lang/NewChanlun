# fugue_version_i.py 架构审计（只读分析）

> 任务：定位 sub-level 卖点行为、出场触发源、多 FSM slice 管理、入场方式四个问题。
> 范围：`analysis/fugue_version_i.py`（1053 行）+ 依赖 `src/newchan/trading/cost_reduction_fsm.py`（601 行）。
> 认识论等级：**L0**（纯代码静态分析，不依赖回测数据；行为推导自源码控制流，不是经验验证）。

---

## 一、四问直答（先给结论，再给行号证据）

| # | 问题 | 代码实际行为 | 是否符合"正确"预期 |
|---|------|------------|------------------|
| 1 | sub-level 卖点触发时做什么？ | **降成本短差**（开短差循环，等 buy_any 回补；降成本阶段 `total_shares` 不变只降 cost_basis）。**不是清仓**。但只作用在机动仓 slice，核心全仓不参与。 | ✅ 行为正确（短差非清仓）／⚠ 但有效仓位被限制 |
| 2 | 出场由什么触发？ | **仅** `entry_ladder`（买点归属的最高涌现中枢层）的 confirmed **type1 卖点**（+ 可选 2% 硬止损）。任何级别的 `sell_any` 都**不**触发出场。 | ✅ 正确（只由归属层触发） |
| 3 | 多 FSM 独立 slice 还是叠加同一仓位？ | **独立不相交 slice**：核心全仓（`core_shares`）完全不做短差；机动仓（默认 10%）按级别数**均分**成互不相交的 slice，每级别一个独立 FSM 各算各的 cost_basis。 | ❌ **这是需要修改的地方** |
| 4 | 入场一次满仓还是逐级建仓？ | **一次满仓**：`_open` 里 `core_shares = INITIAL_CAPITAL / price` 单次建仓；type2 买点只计数（`n_addon`）不加仓。 | ✅ 正确（无 ladder 建仓） |

**一句话**：四个问题里三个（Q1/Q2/Q4）行为正确，唯一的结构性偏差在 **Q3——核心全仓被完全排除在降成本之外，降成本被关进默认 10% 的机动仓里再按级别均分**。这同时解释了历史笔记里反复出现的"降成本拖累 / 降成本仍负 alpha"（`project_version_i` / `project_complete_fugue_v2`）。

---

## 二、出场决策控制行（应只由最高级别触发）

**位置：`run_version_i` 的 LONG 分支，`fugue_version_i.py:817-824`**

```text
817  elif state == _LONG:
818      # 硬止损 A/B：核心仓跌破 entry×(1−2%) → 全仓清出
819      if _stop_on and c < entry_price * (1.0 - STOP_FRAC):
820          n_core_stops += 1
821          _close(i, c, "stop_core_2pct")
822      # 主出场：entry 层 confirmed type1 卖点（顶背驰）→ 全仓清出
823      elif sig.sell1[entry_ladder]:
824          _close(i, c, f"exit_{ladder_name(entry_ladder)}_type1sell")
```

- **出场唯一信号 = `sig.sell1[entry_ladder]`**（第 823 行）。`sell1` 是 confirmed **type1 卖点**磁带（`fugue_version_i.py:388` 字段定义；`_scan_confirmed_bsps` / 各 tracker 的 `s1` 只在 `kind=="type1"` 置位，见 263-274 / 307-313 行）。
- `entry_ladder` 在 `_open(i, c, arm_ladder)`（第 814 行）传入 `el=arm_ladder` → `entry_ladder = el`（第 736 行）。
- `arm_ladder` 是**买点归属的最高涌现中枢层**，确定逻辑：
  - FLAT→ARMED（`797-801`）：在 `[FIRST_BSP_LADDER, max_ladder]` 扫描，`hi` 取**最高**有 `buy1[k]` 的 ladder。
  - ARMED 期间（`804-806`）：若更高 ladder 出现 type1 买点，`arm_ladder` **向上升级**。
- 因此出场严格只由"归属层"的 type1 卖点触发，sub-level 的 `sell_any`（任意卖点）**永不**进入出场判定——它只在 else 分支（`825` 起）驱动声部短差。

> **细微但重要**：`entry_ladder` 是"买点归属的最高层"，不必然等于"绝对最高涌现级别 `max_ladder`"。若最高涌现级别当时没有 type1 买点，归属会落到较低层（即 `project_version_i_dynamic_level` 记录的动态级别归属）。这符合缠论"出场级别 = 进场归属级别"，**不是 bug**。

---

## 三、sub-level 卖点行为控制行（应是降成本短差，不是清仓）

**位置：`run_version_i` LONG 分支的 else 段，`fugue_version_i.py:825-849`**

```text
825  else:
826      # 多重赋格降成本：各声部（entry 层以下级别）独立 FSM，
827      # 触发 = 本级别 sell_any(高抛) / buy_any(低吸)。核心仓位不参与短差。
828      for v in voices:
829          if v.stopped:
830              continue
831          f = v.fsm
832          has_open = (f.active_short_diff is not None
833                      and f.active_short_diff.is_open)
834          ...（止损 B 略，836-843）
844          if sig.sell_any[v.ladder] and not has_open and f.state in _COST_OPEN_STATES:
845              v.fsm = transition(f, FsmEvent(
846                  FsmEventType.SUB_LEVEL_SELL_POINT, price=c, level="sub"))
847          elif sig.buy_any[v.ladder] and has_open and f.state in _COST_ACTIVE_DIFF_STATES:
848              v.fsm = transition(f, FsmEvent(
849                  FsmEventType.SUB_LEVEL_BUY_POINT, price=c, level="sub"))
```

**行为证据链（确认是"短差"非"清仓不回补"）：**

1. sell_any（高抛，`844-846`）→ FSM `SUB_LEVEL_SELL_POINT`：
   - 在 POSITION_OPEN（`cost_reduction_fsm.py:303-314`）或 COST_REDUCING 的 `_open_short_diff`（`405-419`）→ **只记录一个 `ShortDiffCycle`（sell_price, shares=total_shares×sub_ratio），`total_shares` 不变**。降成本阶段是"虚拟卖出"。
2. buy_any（低吸，`847-849`）→ FSM `SUB_LEVEL_BUY_POINT` → `_close_short_diff`（`347-402`）：买回，`profit=(sell−buy)×shares`，`cost_basis -= profit/total_shares`，**`total_shares` 仍不变**。
3. 真实减仓只在 **EARNING_SHARES**（挣股数，`cost_basis≤0` 后）才发生（`_open_earn_diff` 第 530 行 `total_shares - short_shares`）——这是缠师"成本为 0 后卖出多少买入多少赚股数"的金额守恒阶段，不是清仓。
4. `_close`（出场）前会把未闭短差强制按出场价回补（`fugue_version_i.py:762-767`），不留悬空空单。

→ **sub-level 卖点 = 降成本/挣股数短差，配对买回，永不清仓核心仓**。Q1 行为正确。

**⚠ 但有效域被限制**：第 827 行注释明说"核心仓位不参与短差"。短差只跑在 `voices` 的机动 slice 上（见下节）。所以降成本对总仓位的影响上限 ≈ `MANEUVER_RATIO`（默认 0.1）。这是 Q3 问题的直接后果。

---

## 四、FSM 独立 slice 实现行（**这是需要修改的地方**）

**位置：`_open()` 内嵌函数，`fugue_version_i.py:734-751`**

```text
734  def _open(bar_idx: int, price: float, el: int) -> None:
735      nonlocal state, entry_bar, entry_price, entry_ladder, voices, core_shares
736      entry_bar = bar_idx; entry_price = price; entry_ladder = el
737      # 全仓方向核心（不做短差，仅 entry 层 type1 卖点主出场）
738      core_shares = INITIAL_CAPITAL / price          # ← 核心全仓，整段持仓期不参与短差
739      # 降成本级别 = entry 层以下所有 ladder（≥ floor_ladder）。entry 层本身不降成本。
740      levels = sorted({k for k in range(floor_ladder, el)})
741      maneuver_total = maneuver_ratio * INITIAL_CAPITAL          # ← 机动仓 = 默认 10%
742      slice_cap = maneuver_total / len(levels) if levels else 0.0  # ← 再按级别数均分
743      voices = []
744      for k in levels:
745          f0 = CostReductionFSM.create(
746              own_capital=slice_cap, margin_amount=0.0,            # ← 每级独立 slice 本金
747              sub_ratio=_level_trade_fraction(k))
748          f = transition(f0, FsmEvent(
749              FsmEventType.BUY_POINT_CONFIRMED, price=price, level=f"L{k}"))
750          voices.append(_Voice(ladder=k, own_capital=slice_cap, fsm=f))
751      state = _LONG
```

**结算逻辑佐证（`_close`，`fugue_version_i.py:757-778`）：**

```text
757  total_value = core_shares * price                  # 核心全仓市值（与短差无关）
...
770  slice_value = snap.cumulative_recovered + snap.total_shares * price
771  slice_gain = slice_value - v.own_capital           # ← 各 slice 净增益，互不相交
772  total_value += slice_gain                          # ← 叠加的是"各自 slice 的净增益"
```

**为什么这是"独立 slice（错误）"而非"叠加同一仓位（正确）"：**

- `core_shares`（全仓）与每个 `v.fsm`（own_capital=slice_cap）是**三层互不相交的资金池**：
  1. 核心全仓 `INITIAL_CAPITAL`，整段持仓期 `total_shares` 一次设定、一次清出，**零短差参与**；
  2. 机动仓总额 = `0.1 × INITIAL_CAPITAL`；
  3. 机动仓再被 `len(levels)` 均分为不相交的 `slice_cap`，每级别一个 FSM 在**自己那一小片**上独立高抛低吸。
- 各级别短差盈亏只回到**各自 slice 的净值**（`slice_gain`，第 771 行），级别之间、以及与核心仓之间**不叠加**。
- 结果：降成本作用在"`0.1 / N` 大小的孤立小片"上，对持有的核心全仓 cost_basis **毫无影响**。这与缠论降成本的本意（对**同一份持仓筹码**做次级别高抛低吸、逐步压低这份筹码的成本）方向不一致。

**对照 docstring 自述的设计意图**（`fugue_version_i.py:6-9` 与 `692-695`）：作者**有意**把核心仓设为"不做短差的方向底仓"，机动仓 0.1 出自第 31 课"每只股票留 1/10"。所以这不是疏忽，是一个**架构选择**——但它把"降成本"降级成了一个对总收益近乎无关紧要的 10% 旁路。用户判定的"正确架构"（叠加同一仓位）要求：**任何级别的次级别卖点都对同一份核心持仓做降成本**，而不是各自切一小片。

---

## 五、入场方式控制行（应一次满仓——已正确）

**位置：`fugue_version_i.py:794-814`（FLAT/ARMED 状态机）+ `738`（建仓）**

```text
794  if state == _FLAT:
797      for k in range(FIRST_BSP_LADDER, min(sig.max_ladder + 1, MAX_LADDER)):
798          if sig.buy1[k]: hi = k           # 归属层 type1 买点 → ARM
800      if hi >= FIRST_BSP_LADDER: state = _ARMED; ...
803  elif state == _ARMED:
810      do_enter = any(sig.buy_any[k] for k in range(LADDER_BAR, arm_ladder))  # 区间套次级别确认
811      if not do_enter and (i - arm_bar) > SUB_EXPIRY: do_enter = True         # 或超时
813      if do_enter: _open(i, c, arm_ladder)   # ← 单次进入，满仓
...
850  if sig.type2_buy: n_addon += 1            # ← type2 买点仅计数，不加仓
```

- 入场是 **ARM（归属层 type1 买点）→ 区间套次级别确认/超时 → `_open` 一次满仓**（`core_shares = INITIAL_CAPITAL/price`）。
- **无逐级建仓 / 无 ladder 加仓**：type2 买点（`850-851`）只累加 `n_addon` 计数器，从不调用建仓或改变 `core_shares`。
- Q4 正确。

---

## 六、建议的最小修改方案（Q3 唯一需要改的点）

> 目标：把降成本从"独立小 slice"改成"叠加在同一份核心仓位上"。
> 严格性声明（no-patch-mentality）：下面区分**真实最小改动**与**必须先决断的概念问题**，不掩盖后者。

### 6.1 概念前置（必须先定，否则改动是补丁）

把多级别短差**叠加到同一份核心全仓**会撞上一个并发约束：单个 `CostReductionFSM` 同时只允许一个 `active_short_diff`（`cost_reduction_fsm.py:409-412` 会抛 `IllegalTransitionError`）。当前的"每级一个独立 FSM + 不相交 slice"正是为了让多个级别能**并发**各自高抛低吸而设计的。

因此"叠加同一仓位"有两种严格形式，需要先决断（属于"选择"类，建议 `/escalate` 或由编排者裁定）：

- **形式 A（共享仓位 + 串行短差）**：核心全仓即降成本仓位，全部级别共用**一个** FSM，`total_shares = core_shares`。同一时刻只有一个开放短差。多级别卖点竞争同一短差通道（高层优先 / 先到先得）。语义最贴近"同一份筹码降成本"，但牺牲级别并发。
- **形式 B（共享仓位 + 并发短差，份额上限协调）**：保留多 FSM 以维持级别并发，但每个 voice 的短差按 `core_shares × sub_ratio` 基于**全仓**计量，且所有 voice 的已开放短差份额之和受 `Σ open_shares ≤ core_shares` 约束；所有短差盈亏累积进**同一个** core cost_basis / 净值。语义是"同一份全仓被多级别协同降成本"。

> 在 A/B 未决断前直接改 6.2 的行，会是"非严格"的——所以这一步不能跳。

### 6.2 形式 A 的最小改动（若选 A——改动最小、最严格）

| 行 | 现状 | 改成 |
|----|------|------|
| `738` | `core_shares = INITIAL_CAPITAL / price` | 保留（核心全仓 = 降成本仓位本体） |
| `740-742` | 切出 `maneuver_total` 再 `slice_cap` 均分 | **删除** slice 切分 |
| `743-750` | `for k in levels:` 建 N 个独立 FSM | 改为建**单个** FSM：`own_capital=INITIAL_CAPITAL`（=核心仓本金），并以 `BUY_POINT_CONFIRMED` 把它带到 POSITION_OPEN，使其 `total_shares == core_shares` 同源 |
| `828-849` | `for v in voices:` 多声部各驱动各 FSM | 改为**单 FSM**：在 `[floor_ladder, entry_ladder)` 任一级别出现 `sell_any` 且当前无开放短差 → 该唯一 FSM `SUB_LEVEL_SELL_POINT`；任一级别 `buy_any` 且有开放短差 → `SUB_LEVEL_BUY_POINT`。短差直接降这份全仓的 `cost_basis` |
| `757-778` | `total_value = core_shares*price` + Σ`slice_gain` | 改为 `total_value = fsm.total_shares * price + fsm.cumulative_recovered`（同一份仓位的市值 + 已回收现金，不再叠加不相交 slice） |

> 形式 A 下 `_Voice` / `fsm_short_pnl` 按级别拆分的贡献度统计需相应改为"单 FSM + 触发级别打标签"（贡献度仍可按 `level=` 字段归集，FSM 已支持 `ShortDiffCycle.level`）。

### 6.3 形式 B 的改动（若选 B——保并发，改动较大）

- 保留多 voice，但 `_open` 第 746 行 `own_capital` 改为基于全仓（`core_shares × _level_trade_fraction(k)` 的本金口径），并在第 828 起的循环**前**加一个全局约束检查：`sum(已开放短差 shares) + 新短差 shares ≤ core_shares` 才允许 `SUB_LEVEL_SELL_POINT`。
- `_close` 的 `slice_gain` 叠加（771-772）改为：核心仓 `total_shares` 直接被各短差的挣股数净增/降成本盈亏修改（需让多 FSM 写回同一个 `core_shares`，即 FSM 不再持有独立 own slice，而是持有"对共享仓位的操作记录"）。
- 形式 B 实质要求把 `CostReductionFSM` 从"自带 own_capital 闭包账户"改造成"无状态短差算子作用于外部共享仓位"——这超出单文件最小改动，会触及 `cost_reduction_fsm.py` 的数据模型。

### 6.4 不需要改的（确认正确，避免误伤）

- 出场逻辑 `817-824`：保持只由 `sell1[entry_ladder]` + 止损触发。**不要**让 sub-level `sell_any` 进入出场。
- 入场逻辑 `794-814`：保持单次满仓。**不要**引入 ladder 加仓。
- FSM 短差配对（`cost_reduction_fsm.py` 短差/挣股数两阶段）：行为正确，形式 A 下复用单实例即可。

---

## 结果包六要素

**1. 结论**
`fugue_version_i.py` 的四项中 Q1（短差非清仓）、Q2（仅归属最高层 type1 卖点出场）、Q4（一次满仓）行为正确；**唯一结构性偏差是 Q3**——核心全仓（`core_shares`，738 行）被显式排除在降成本外，降成本被关进默认 10% 机动仓（741 行）再按级别均分为互不相交 slice（742 行），各级 FSM 各算各的净值（771-772 行），与"叠加在同一份持仓上降成本"的正确架构不符。

**2. 定义依据**
- 缠论降成本（第 31/33/35 课，见 `fugue_version_i.py:30-37` docstring 与 `cost_reduction_fsm.py:12-24`）：对**同一份持有筹码**用次级别高抛低吸压低其成本基础（降成本阶段股数守恒）／成本为 0 后金额守恒挣股数。当前代码把"同一份筹码"替换成"10%/N 的孤立小片"，使降成本不作用于核心持仓的 cost_basis。
- FSM 短差语义（`cost_reduction_fsm.py:303-402`）：`SUB_LEVEL_SELL_POINT`→记录短差不减仓，`SUB_LEVEL_BUY_POINT`→买回降 cost_basis，确认 Q1 为短差非清仓。

**3. 边界条件**
- 若 `MANEUVER_RATIO` 提到 1.0 且 `floor_ladder` 使 `len(levels)=1`，则机动 slice ≈ 核心仓，Q3 的"独立 slice"退化为近似"同仓位"——但仍与核心 `core_shares` 分账（结算仍走 `slice_gain` 叠加），结论不翻转。
- 若用户认可 docstring 的"核心仓=纯方向底仓不降成本"设计（第 31 课 1/10 机动资金口径），则 Q3 不算 bug 而是设计选择，结论翻转为"四项全部符合作者意图"。**翻转开关 = 对'降成本是否应作用于核心持仓本身'的定义裁决。**
- Q2 结论翻转条件：若把 `sig.sell_any` 误接进 823 行的出场判定（当前未接），出场会被任意级别触发。

**4. 下游推论**
- 若按 6.2 改为形式 A，降成本将首次真正作用于全仓 cost_basis，历史笔记 `project_version_i` / `project_complete_fugue_v2` 记录的"降成本拖累/负 alpha"需在新架构下**重新回测**——旧否定性结论建立在"降成本只影响 10% 旁路"的有效域上，不可直接外推到全仓降成本。
- 形式 A/B 的决断会反向约束 `cost_reduction_fsm.py` 是否需要从"闭包账户"重构为"共享仓位算子"。

**5. 谱系引用**
- 涉及降成本架构，相关谱系：267 号操作方法论 v1（满仓满融降成本，`cost_reduction_fsm.py:7`）、268a 号结算（own_capital 独立核算）、525 号（笔中枢真实 BSP）。
- 降成本提款机 bug 谱系（`project_costreduction_moneyprinter_bug`）：本文件 369 行已去 `max(0,…)` 截断（亏损短差照实扣 recovered），该 bug 在本文件**不存在**；但此审计**未**重新验证 slice 净值口径下是否有其他记账泄漏（超出只读静态分析范围，需 L2 回测）。
- 不确定是否存在"降成本作用域（核心仓 vs 机动仓）"的独立谱系记录——建议溯源确认 267/268a 是否已就此决断；若无，6.1 的 A/B 选择属新"语法记录"。

**6. 影响声明**
- 本产出**未改动任何代码或文件**，仅新增审计报告 `analysis/fugue_vi_architecture_audit.md`。
- 指出的待改点集中在 `fugue_version_i.py:738-751`（建仓 slice 切分）、`757-778`（结算叠加）、`828-849`（多声部驱动）；6.1 的 A/B 决断若落地会进一步影响 `src/newchan/trading/cost_reduction_fsm.py` 的数据模型。
- 出场（817-824）与入场（794-814）经审计确认正确，**标记为不可误伤区**。
