# 交易扁平化机制审计：净额执行 vs 赋格声部独立执行

- 工位：深度调研文档工位（wave kimi-nest-mainline-20260717），只写本 .md；rust/src 未改一行；无 git mutation；主仓只读。
- 纪律：090（声明=能力，照实否定合法）/ v3 硬禁令（不引入概率/统计推断作决策基础；不用回测验证策略；不假设 EMH——本文全部为机制论证与结构恒等，无统计推断）。
- 前作：`chanlun/review-results/m8-opsem-vs-norders-recon-20260719.md`（口径对账：75629 净额 fill vs 518 声部 round-trip/窗，同口径比 48.7×）。本文在其之上回答**机制为什么**与**怎么修**。
- 结论先行：
  1. **fill 按净额 ΔN 执行，不按声部开合执行**。声部开仓/平仓从不产生独立 fill——它们只移动聚合目标 p̃；唯一订单出口是 `Schedule_Θ(p*−p_t)` 的单净额订单（每 bar 至多一张，`coverage.rs:2692`、`runner.rs:1856-1863`）。
  2. **佣金按净额收**：声部 A 开多 100、声部 B 开空 60，若同 bar 同向聚合则净额层只 fill +40 一次、费基 = 40 手名义（`apply_fill` 费=成交名义×fee_rate，`runner.rs:3778`）；声部层的 100/60 只活在只读旁路账本里（`overlay_state.rs:170-250`）。
  3. **churn 的机制根源是「每 bar 重定目标 + 净额单订单出口」**：sizing 基准 `base_units = equity_nav/px` 每 bar 重算（`runner.rs:1279`），每条声部目标单位 = `base_units×w_depth×w_dir`（`coverage.rs:1426-1428,1528,1552`）随之每 bar 漂移；所有声部的漂移经 p̃ 聚合后由单订单兜底，|Δ|≥0.5 手（`coverage.rs:2694`）即触发真实 fill。三窗实测 fill 密度 12.4%/11.2%/5.4%（见 §5.2），Comm+Slip 是毛价格 PnL 的 11×/133×/14×。
  4. **修复方向 = 声部独立执行 + 事件驱动 sizing**（仓位在声部开仓时冻结，只在声部开/合/结构事件时 fill）。只换账本不换 sizing 治不了 churn——机制论证见 §6.3。量级估算（非预测）：订单数降 ≥24×，佣金降 1-2 个数量级，被声部间对冲毛额化部分抵消（§7）。

---

## 1. 净额执行机制全链（行号锚）

### 1.1 决策→订单→成交 链条

每根可交易 bar（`runner.rs:1267` `if !bar.untradable && px > 0.0`）走同一条链：

```
classify_at(i)（前缀因果分类，runner.rs:1270）
  → pi_theta_step_traced(...)（runner.rs:1462-1476）
      ├─ coverage_step_from_buckets_sep → legs（每声部 units = base_units×w_depth×w_dir，
      │    coverage.rs:1528/1552）→ p̃ = net_target_units(&legs)（coverage.rs:2307）
      │    + sep_legs 只读重打包（coverage.rs:2342-2353）
      ├─ p* = pi_theta_position(p̃, p_t, base_units, ...)（coverage.rs:2642-2651，
      │    LexArgmin over 𝒦_Θ，cap = γ̄·|base_units|，coverage.rs:2667-2672）
      └─ order = schedule_order(p*, p_t, exec_index)（coverage.rs:2692-2727；
           标准路径出口 coverage.rs:3031，P1 强平 :2932，P2 :2966，P3/P4 :3000）
  → 挂单（runner.rs:1856-1863）：if order.qty > 0 ⟹ pending[exec_index].push(order)
     —— 每 bar 至多一张净额订单（order 是单变量，PanDiv 臂也只覆写同一槽，
        runner.rs:1543/1552）
  → 延迟成交（runner.rs:1191-1208）：本 bar 到达 exec_index 的挂单经 apply_order 成交，
     executed_qty>0 ⟹ n_orders_executed += 1（runner.rs:1204）
```

### 1.2 净额单一仓位账本

- `units: f64` = 净 lot（`runner.rs:1065` 注释：「p_t = 净 lot（apply_order 维护，有符号：正多/负空/0空仓）」）——**全账户只有一个有符号标量持仓**，无声部维度。
- `apply_order`（`runner.rs:3691-3723`）→ `apply_fill`（`runner.rs:3736-3800`）：先平后开两段式，费用 = 成交名义 × fee_rate（`runner.rs:3778` `fee_paid += close_qty * px * fee_rate`，开仓段同理），fee_rate = (commission_bps+slippage_bps+tax_bps)/10⁴ = **3bps**（`runner.rs:1060-1061`；默认 1+2+0，`config.rs:222-235`）。
- 计数口径：`RunResult.n_orders`（`runner.rs:70`）= `FillOutput.n_orders`（`runner.rs:2003`）= `n_orders_executed`（`runner.rs:1104` 声明、`:1204` 增量）= **净额层真实 fill 事件数**。
- `OverlayRunResult` 口径（`runner.rs:652-679`）：`n_overlay_fill_events`（:659）声明同源 `net_result.n_orders`；`n_overlay_voices`（:663）= 声部数（前作 §3 的 090 违规已修为三态一致）。

### 1.3 声部账本（只读旁路，不驱动 fill）

`OverlayState`（`overlay_state.rs:101-113`）是 hedge-mode 逐声部簿：

- 恒等三链（`overlay_state.rs:13-18`）：P^sep = Σ q_v σ_v e_v；N = Σ σ_v q_v；Order = ΔN。
- `step`（`overlay_state.rs:170-250`）：① 价格 PnL 累计（:172-182）；② rebalance 到目标 P^sep——离场记 `ClosedVoice`（:200-216）、存续 resize（:219-230）、新开 `VoiceBook`（:231-243）；③ order = ΔN（:246-249）。
- **关键**：runner 里 `ov.step(...)` 的产出 `ostep` 只用于 ΔN 守恒 debug_assert（`runner.rs:1558-1567`），**不改 cash/units/trade_pnls**（`runner.rs:1555-1557` 注释「只读旁路」）。声部账本是影子簿，真实成交与它无关。
- 窗口终点 `ov.force_flat`（`runner.rs:1903-1910`）同样只搬账本行。

声部层真正的 round-trip 账本是 G4 typed ledger：`typed_ledger: Vec<TypedTrade>`（`runner.rs:1110`），开腿登记（`runner.rs:1573` 起）+ 四类关腿结算 push（`runner.rs:1709/1747/1778/1810`）+ 窗口终点 censored（`runner.rs:1946-1969`）= OPSEM trades.jsonl 的 518 行/窗（前作 §1.2）。

---

## 2. 任务问题②：fill 按净额 ΔN 还是按声部开合？

**按净额 ΔN。声部开合从不产生独立 fill。** 机制证据：

1. 声部开/合只改变 `legs`/`sep_legs`（声部目标集），经 `p̃ = net_target_units(&legs)`（`coverage.rs:2307`）先**求和消维**，再经 p* 与 p_t 的差产唯一订单（`coverage.rs:2692-2694` `let delta = p_star - p_t`）。声部身份在订单上不留痕——`Order { action, qty, exec_index }`（`coverage.rs:2726`）三个字段没有 voice_id。
2. 成交侧 `cash/units/entry_cost` 是三个标量（`runner.rs:1064-1066`），无 per-voice 状态可挂。
3. 对照测试坐实净额消维：`overlay_state.rs:308-327` `hedged_two_voices_net_zero_but_book_nonzero`——父多 10 + 子空 10 ⟹ 净 N=0、`order=0`（净额账户不下单），但账本记两声部。

**A 开多 100 / B 开空 60 的佣金口径**（机制推演，三种时序）：

| 时序 | 净额层 fill | 费基 |
|---|---|---|
| 同 bar 同时开 | 一张 +40 | 40 手名义（100−60 被净额消除） |
| A 先开、B 后开（不同 bar） | +100、−60 两张 | 160 手名义（与声部独立执行相同） |
| 两声部存续期间每 bar 重定目标 | 每 |ΔN|≥0.5 手的 bar 各一张 | 与声部无关，随 NAV/价漂移 |

即：**净额只在「同 bar 反向事件」时省佣金；声部事件错开时净额不省任何费用，反而叠加每 bar 重定目标的 churn 费**（§5）。佣金一律按净额成交名义计（`runner.rs:3778`），不存在按声部计费的路径。

---

## 3. 设计对照：赋格声部独立持仓 vs 净额单仓

| 维度 | 赋格声部独立执行（设计目标） | 净额单仓（现状实装） |
|---|---|---|
| 持仓状态 | 每声部一条 (σ_v, q_v, entry_cost_v)——hedge-mode position book（多空对冲.pdf §10.2 保存 (Q⁺,Q⁻)） | 单标量 `units`（`runner.rs:1065`），PDF §10.1 N=Σσ_v q_v 的有损投影 |
| fill 触发 | **事件驱动**：声部 open / close / 结构角色变化时，该声部产独立 fill | **每 bar 目标差**：p*−p_t 非零且 ≥0.5 手即 fill（`coverage.rs:2694`） |
| 订单身份 | Order 携 voice_id，成交归因到声部 | Order 三字段无身份（`coverage.rs:2726`） |
| 对冲两腿 | 各自存续、各自结算（PDF §10.2 (Q⁺,Q⁻) 不互相湮灭） | 同 bar 反向量互相湮灭（`overlay_state.rs:308-327` 见证 order=0） |
| PnL 实现 | 声部平仓时一次性实现（一个 round-trip 一次结算） | 每次减仓 fill 都实现一段（`apply_fill` 段1，`runner.rs:3771-3798`；F-06 部分减仓逐 fill 产 TradeRecord，`runner.rs:3550-3563`） |
| 佣金基数 | Σ_v 声部毛周转（含对冲两腿毛额） | Σ_t |ΔN_t| 净周转 + churn 周转 |
| 现状对应物 | `OverlayState` 已有账本形状（`overlay_state.rs:59-95`），但**只读、不驱动 fill**（`runner.rs:1555-1567`）；G4 typed ledger 已有声部 round-trip 记录（`runner.rs:1110`），但**不对应真实成交** | `pi_theta_fill_loop_overlay` 全链（§1.1） |

PDF 自身的分层正是这个对照：§10.1 净额账户（N=Σσ_v q_v）vs §10.2 hedge-mode position book（保存 (Q⁺,Q⁻) 而非 Q⁺−Q⁻）；§11 线性恒等 Q⁺ΔP−Q⁻ΔP=(Q⁺−Q⁻)ΔP 保证**价格 PnL 两层相等**（`overlay_state.rs:20-27` 的对账测试 `per_voice_pnl_reconciles_with_net`，`overlay_state.rs:331-350`）——**但 §11 恒等只覆盖价格 PnL，不覆盖费用**：费用 ∝ 周转名义，两层周转不同（§5.3 恒等式），这正是"价格 PnL 对账全绿、账户却爆亏"的机制解释。m8 报告三层数字自洽（前作 §2，18/18 bit-exact），亏在执行层费用。

---

## 4. 任务问题③：75629 张净额 fill 里，多少是声部间对冲被净额消除的、多少是同一声部被切碎的？

### 4.1 分解恒等式（机制，可构造性度量）

定义三个周转量（手数，均可从 OverlayState 的逐 bar 簿构造性算出）：

- **声部毛周转** G = Σ_t Σ_v |q_{v,t} − q_{v,t−1}|
- **净周转** T = Σ_t |ΔN_t|（= 净额层真实成交手数，fee/3bps 可反推美元名义）
- **声部生命周期周转** L = Σ_v (q_open(v) + q_close(v))（每声部开+平各一次的事件驱动周转）

则由三角不等式与 ΔN 守恒（`overlay_state.rs:246-249`）：

```
G − T  = 声部间对冲被净额消除的周转（≥0；同 bar 反向变化的抵消量）
G − L  = 同一声部被净额切碎的周转（≥0；存续期内每 bar 重定目标的 wiggle 累计）
T − L  = churn 净效应 = (G−L) − (G−T)   ⟹   T = L + (G−L) − (G−T)
```

即净额层成交 = 生命周期必需成交 + **切碎增量** − **对冲节省**。churn 是否主导取决于 (G−L) 与 (G−T) 的差。

### 4.2 照实缺席：精确拆分需要当前生产代码不存在的计数器

- OverlayState 逐 bar 重算 q_v（`overlay_state.rs:219-244`），数据足够算 G 和 L，但**不累计**这两个量（无字段、无落盘）；OPSEM dump 只写声部终结算行（前作 §1.2），不含逐 bar q_v 轨迹。
- 因此 G−T 与 G−L 的精确数值本工位给不出（只读工位不跑二进制；即使跑，生产二进制也不产此二量）。这不是回避——**计数器定义已给出（§4.1），并列入修复设计 §8.4 作为验收量**。
- 可给的机制性边界：n_orders 每张订单 qty≥1 手（`runner.rs:1857` qty>0 才挂、`:1201` executed>0 才计），75629 张 fill × fee 3bps 反推净周转名义 = 三窗 Comm+Slip 合计 55,853,036 / 3e-4 ≈ **1.86×10¹¹ USD**（m8 报告 :21-23 三行 Comm+Slip 16755156+20993704+18104176）。而生命周期必需周转 L 的上界 = 2×声部数×单声部上限手数×均价，量级 10⁸-10⁹ USD（§7.2 估算）——**T 比 L 大约两个数量级 ⟹ 切碎增量 (G−L) 是 T 的主导项，对冲节省 (G−T) 至多是次要修正**。这是量级论证，不是精确拆分；精确值待 §8.4 计数器落地后照实填。

---

## 5. 扁平化导致 churn 的机制论证

### 5.1 机制链：每 bar 重定目标 → 聚合漂移 → 单订单兜底

1. **sizing 基准每 bar 重算**：`base_units = equity_nav / px`（`runner.rs:1279`）——NAV 含浮盈 MtM（`runner.rs:1228` `cash + units*px`），px 是分钟级 close。两个噪声源（价格噪声 + 持仓浮盈噪声）每 bar 注入 sizing 基准。
2. **每条声部目标随基准漂移**：`units = base_units × w_depth × w_dir`（`coverage.rs:1426-1428`，实装 :1528/:1552）。声部集合不变、无任何新信号，目标也每 bar 变。
3. **聚合后经单订单出口**：p̃ = Σ σ_v·units_v（`coverage.rs:2307`）→ p* = lot 对齐的 LexArgmin（`coverage.rs:2642-2651`）→ `qty = |p*−p_t|.round()`（`coverage.rs:2694`）。多声部并发时各声部 wiggle 在净额上**不抵消则累计**——几十条声部各自漂移 ±ε，聚合漂移 √k·ε 量级，越过 0.5 手阈（`schedule_order` qty=0 ⟹ Hold/Wait 不交易，`coverage.rs:2697-2702`）的 bar 就产一张真实订单。
4. **成交即费用**：每张 fill 按成交名义 × 3bps 收费（`runner.rs:3778`、`:1060-1061`），费用与「是否有新交易信号」无关——漂移本身在被收费。
5. **事件与漂移混在一起**：声部真实开/合（事件）与每 bar 重定目标（漂移）经同一个 Δ 出口，净额层无法区分「该付的费」（声部生命周期必需）与「不该付的费」（重定目标噪声）。

### 5.2 实测印证（m8 报告，/tmp/m8_e2e_all_systems_oos.md:21-23；bar 数 /tmp/m8_opsem_p3fold.out:283-288）

| 窗 | bars | n_orders | fill 密度 | ΣN_tΔP_t（毛价格 PnL） | Comm+Slip | 费/毛 PnL |
|---|---|---|---|---|---|---|
| p3fold | 260560 | 32195 | 12.4% | +1523360 | 16755156 | **11.0×** |
| wf7 | 260560 | 29190 | 11.2% | +158330 | 20993704 | **132.6×** |
| wf8 | 264960 | 14244 | 5.4% | +1263300 | 18104176 | **14.3×** |

每 8-19 根 bar 一张真实 fill，而声部结算仅 518 行/窗（前作 §1.2）——**每张声部 round-trip 对应 27-62 张净额 fill**（按窗：32195/518≈62、29190/518≈56、14244/518≈28；三窗合计 48.7×，前作 §1.3）。m8 报告层2判语「成本真实化后极负 = 高频费主导，非机制缺陷」（:28）——本审计的修正是：**「高频费主导」本身就是执行机制（扁平化）的缺陷**，不是外生成本。net_r 三窗全负（-15.4M/-21.0M/-17.1M）而毛价格 PnL 三窗全正，差额几乎全部由 Comm+Slip 构成。

### 5.3 为什么「每 bar 净 ΔN 微调」与「声部事件驱动开合」的费用量级必然悬殊

事件驱动下声部 v 一生成交 2 次（开、平），费用 = 2·q_v·px̄_v·r；净额层声部 v 存续 H_v 根 bar 内，其目标 wiggle 与其他声部聚合后，凡是 |ΔN|≥0.5 手的 bar 都成交——v 存续期内被「代收代付」的 fill 数与 H_v、并发声部数正相关，与 v 自身是否有事件无关。518 声部/窗 ÷ 260560 bar ⟹ 声部事件率 ≈ 0.4%/bar，而 fill 密度 5-12%/bar——**fill 密度是声部事件率的 14-31 倍**，差额即漂移驱动的 churn（机制论证，与 §4.2 的周转量级论证互证）。

---

## 6. 声部独立执行的订单数与佣金量级估算（机制论证，非统计预测）

### 6.1 前提：只换账本不换 sizing 治不了 churn

OverlayState 现状已经演示了这一点：它的 `step` 每 bar rebalance（`overlay_state.rs:186-244`），resize 分支（:219-230）每 bar 按新 `q_units` 调 q_v——若直接把这套簿接上真实 fill，每声部每 bar 一张订单，订单数只会更多（k 条声部 × 每 bar）。**修复必须双管齐下：① 声部独立持仓/独立 fill；② sizing 事件化（开仓时冻结 q_v，存续期不随 NAV/价重定目标）**。② 是把 `base_units` 从「每 bar 协变量」降级为「开仓时刻快照」——声部目标 q_v = round(base_units(entry_bar)×w_depth×w_dir / lot)·lot，开仓时定死。

### 6.2 订单数量级

事件驱动声部执行的 fill 数有构造性上界：

```
fills_voice ≤ Σ_v (1 开 + 1 平) + censored 开仓数
            ≤ 2 × 518 = 1036 /窗（censored hold 只有开无平 ⟹ 实际在 518-1036 之间）
```

对比现状 32195/29190/14244：**每窗降 14-62 倍，三窗合计 75629 → ≤3108，降 ≥24×**。若保留结构事件触发的 resize（如级别晋升/角色变化时重定目标），每声部加有限次事件 fill，上界变为 Σ_v(2 + n_event(v))，量级不变（声部结构事件率 ≪ bar 率，与 5.3 的 0.4%/bar 同源）。

### 6.3 佣金量级

佣金 = r × 周转名义。事件驱动下：

```
T_voice = Σ_v 2·q_v·px̄_v   （开平各一次，含对冲两腿毛额——净额消除的 (G−T) 在此被毛额化回来）
```

以 p3fold 为锚（均价量级 2023H1 BTC ≈ 2.8×10⁴ USD，仅作量级换算，非预测输入）：单声部 q_v = base_units×w_depth×w_dir，base_units ≈ nav0/px ≈ 10⁶/2.8×10⁴ ≈ 36 手，w_depth ∈ [0.10, 0.60]（`coverage.rs:1427` w=[0.60,0.30,0.10]）⟹ 典型 q_v ∈ [4, 22] 手（未计 w_dir 调制与 cap）。取 q_v ∈ [5, 30] 手作区间：

```
T_voice ≈ 2 × 518 × [5, 30] × 2.8×10⁴ ≈ [1.5×10⁸, 8.7×10⁸] USD
佣金_voice ≈ T_voice × 3e-4 ≈ [4.4×10⁴, 2.6×10⁵] USD
对比现状 16.76×10⁶ ⟹ 降约 64×-380×（量级：1-2 个数量级）
```

**对冲毛额化的反向修正**（诚实计入）：净额在同 bar 反向事件时省掉的 (G−T) 部分，声部执行要毛额付回。但该部分上界 = 2×Σ_对冲对 min(q_A,q_B)·px，受同时持有反向声部的对数限制，量级不超过 L 本身（即最多把上表翻倍）——**不改变 1-2 个数量级的结论**。三窗合并同量级（wf7/wf8 均价不同，结论区间平移，数量级不变）。

**口径声明**：本估算是机制量级论证（恒等式 + 显式假设区间），标注同 m8 报告 [L1机制/费率未标定]——费率 3bps 是保底未标定参数（`config.rs:233-235`），所有佣金数禁作策略择优输入；订单数上界（§6.2）是构造性的，不依赖费率标定。

---

## 7. 修复设计（只设计，不实装）

### 7.1 设计原则

- **新臂不动旧臂**：仿 overlay 臂模式（`run_theta_v0_pi_overlay`，`runner.rs:694`）新增声部执行臂，`pi_theta_fill_loop_overlay` 的 `overlay: Option<...>` 参数同款——`None` ⟹ 净额路径逐字节不变（现有全部臂回归锁）。**决策路径单源**：新臂复用同一 `pi_theta_step_traced`（`runner.rs:1462`）的 `step_trace.sep_legs` 与 opened/closed/silent_drops/risk_exits/overlay_closes 五类生命周期事件（`runner.rs:1573/1702/1735/1771/1803`）——信号、typed ledger、TW 账本序列与净额臂 bit-exact，**只有执行层（fill/cash/fee/equity/n_orders/trade_pnls）不同**（这正是要修的东西，差异是设计意图，非回归）。
- **账户语义升级**：单标量 `units` → hedge-mode 声部簿（PDF §10.2 (Q⁺,Q⁻)），净敞口 N = Σσ_v q_v 降级为**派生只读量**（仅供风控门 `k_theta_risk_gate` 与 cap 判据消费，与现状 `runner.rs:1281` 的 p_t 口径衔接）。
- **sizing 事件化**：q_v 在开仓 bar 冻结（§6.1），存续期仅允许结构事件触发 resize（级别/角色变化；可选 ±δ 手带宽的离散重定，带宽是显式参数，默认 ∞ = 不重定）。

### 7.2 前/后伪码

**前（现状，`runner.rs:1462-1476` + `:1856-1863` + `:1191-1208`）**：

```
每 bar i:
    (next_active, p*, order, step_trace) = pi_theta_step_traced(..., p_t, base_units_t, ...)
    // base_units_t = equity_nav_t / px_t   ← 每 bar 重算（runner.rs:1279）
    // order = Schedule_Θ(p* − p_t)         ← 唯一净额出口（coverage.rs:2692）
    if order.qty > 0: pending[exec_index].push(order)        // runner.rs:1857-1862
    // 声部簿 ov.step(sep_legs) 只读旁路（runner.rs:1558-1567），不产 fill
成交 bar i:
    for o in pending[i]: apply_order(o, px, fee_rate, &mut cash, &mut units, ...)  // 单标量
        executed>0 ⟹ n_orders_executed += 1                                       // runner.rs:1204
```

**后（设计，新臂 `pi_theta_fill_loop_voice`）**：

```
每 bar i:
    (next_active, _p*, _net_order_忽略, step_trace) = pi_theta_step_traced(... 同参 ...)
    // 决策轨迹与净额臂同源同参 ⟹ step_trace/typed_ledger/TW 序列 bit-exact
    voice_orders: Vec<VoiceOrder> = []
    for (c, leg) in step_trace.opened:            // 事件①开仓（runner.rs:1573 同源事件流）
        q_v = round(base_units_i × w_depth(leg) × w_dir(c) / lot) × lot   // ★开仓时刻冻结
        if q_v ≥ lot: voice_orders.push(VoiceOrder{ voice: leg.id, side: σ_v, qty: q_v, kind: Open })
    for leg in step_trace.closed + silent_drops + risk_exits + overlay_closes:  // 事件②平仓
        if let Some(book) = books.get(leg.id):
            voice_orders.push(VoiceOrder{ voice: leg.id, side: −σ_v, qty: book.q, kind: Close })
    for ev in structural_resize_events(leg):      // 事件③结构 resize（级别/角色变化；默认空集）
        Δq = new_target(ev) − books[ev.voice].q
        if |Δq| ≥ lot: voice_orders.push(...)
    for vo in voice_orders: pending_voice[exec_index].push(vo)   // 无事件 ⟹ 本 bar 零订单
成交 bar i:
    for vo in pending_voice[i]:
        book = books.entry(vo.voice)               // 每声部独立 units_v / entry_cost_v
        fill = apply_fill_vo(vo, px, fee_rate, &mut cash, &mut book)   // 复用 apply_fill 两段式
        n_voice_fills += (fill.executed_qty > 0)
        if book.q == 0: settle voice → ClosedVoice（entry/exit/px/pnl 冻结，overlay_state.rs:81-95 形状）
    N_derived = Σ σ_v·q_v   // 只读派生，喂风控门/cap；不再驱动订单
```

### 7.3 落点（行号锚，只设计不实施）

| 改动 | 落点 | 说明 |
|---|---|---|
| 新臂函数 `pi_theta_fill_loop_voice` | `runner.rs:1041` 旁（与 `pi_theta_fill_loop_overlay` 并列） | 共享 classify/风控/TW/opsem 全部上游；分叉点在 `:1462` 之后——消费同一 step_trace，忽略净额 order |
| `VoiceOrder` 类型 | `types.rs` Order 旁 | = Order + voice_id + kind；不改现有 Order（`coverage.rs:2726`） |
| 声部执行簿 | 复用/升级 `overlay_state.rs:101-113` OverlayState | 加 per-voice `cash` 影响与 fee 累计字段；现有 pnl_v 是价格 PnL（:77），费后 PnL 需新字段（声明≠能力，禁复用 pnl_v 冒充费后） |
| sizing 冻结 | `runner.rs:1279` base_units 的消费侧 | 净额臂保持每 bar 重算（bit-exact）；声部臂只在 opened 事件取快照。**不改 `coverage.rs:1528/1552`**（那是决策层目标，两臂共享）——冻结发生在执行臂把 q_units 落簿时 |
| pending/fill 循环 | `runner.rs:1076` pending、`:1191-1208` fill 段 | 新臂用独立 `pending_voice` 队列与 per-voice `apply_fill` 包装；净额臂原队列不动 |
| 验收计数器 | FillOutput 装配（`runner.rs:1997-2007` 旁） | 见 §7.4 |

### 7.4 验收量与 bit-exact 影响清单

**bit-exact 必须不变的**（新臂 vs 净额臂同窗同参对跑）：

- `typed_ledger` 全序列（声部 round-trip 518/窗 逐行一致）——决策同源的直接推论；
- TW 账本事件序列与终态 `tw_final`（`runner.rs:2005`）；
- `step_trace.sep_legs` 逐 bar 序列（声部目标集由决策层产，执行臂不反馈）；
- 净额臂自身全部产出（`overlay=None` 路径，`runner.rs:1039-1040` 注释的现有承诺）。

**设计性改变（非回归，须重出报告）**：`n_orders`（→ §6.2 量级）、`cash`/`equity_curve`（费用路径变）、`trade_pnls`（按声部结算，条数 ≈ 声部数而非 fill 数）、`r_decomp` 的 Comm+Slip 行（`runner.rs:1983-1986`；守恒断言 `runner.rs:1990-1994` 必须在新臂同样成立——per-voice 账本 Σ_v ledger_delta_v == Σ_v (price_pnl_v − fee_v)，残差容差同款）。

**新增验收计数器（§4.2 的诚实缺席靠它们补）**：

- `gross_voice_turnover` = Σ_t Σ_v |Δq_{v,t}|（G）；
- `net_turnover_filled` = Σ_t |成交手数|（T，新臂应 == Σ_v(q_open+q_close)+Σ resize）；
- `hedge_netted_away` = G − T（对照臂读数，量对冲节省）；
- `voice_shred` = G − L（对照臂读数，量切碎）；
- 断言：`n_voice_fills ≤ 2×n_voices + n_resize_events`（事件驱动上界，构造性可断）。

### 7.5 与既有裁定的衔接

- 本设计不改决策层 ⟹ 层1 signal（m8 报告 :14-16 转引结论）、typed ledger μ 训练口径（A5-A10 诸裁定）全部不受影响；
- `OverlayRunResult.n_overlay_fill_events`（`runner.rs:659`）声明的「ΔN 非零步数 = 真实下单次数」在声部臂下获得**第二读数**（声部 fill 数），两读数分列，禁互相冒充（090）；
- 费率 3bps 未标定（m8 [L1机制/费率未标定]）不变——本设计降的是**周转**，不是费率假设；费率标定是独立工位。

---

## 8. 未尽事项 / 诚实缺席

- §4.2：G−T 与 G−L 的精确数值缺席（生产无计数器，只读工位不跑二进制）——已给出恒等式与验收计数器设计（§7.4），落地后照实填数。
- 未逐行核 `w_dir` 的全部调制分支（`coverage.rs:1426` 起的 (ℓ,δ,σ_higher,role) 四参）——§6.3 的 q_v 区间因此取宽（[5,30] 手），只保量级结论，不影响机制论证。
- `feasible_net_cap(risk)` 的具体值未核（`coverage.rs:2669`）——影响 cap 上界，不影响 fill 触发机制。
- m8 报告「声部数(A/S/F)」列（:21-23，如 325/205/205）的精确口径未核（A/S/F 三字母定义不在本文范围）；本文声部数一律取 OPSEM 落盘的 518/窗（前作已核口径）。
- 新臂的保证金/风控衔接（hedge-mode 下毛敞口 Q⁺+Q⁻ 可能触发与净敞口不同的 margin 判据，`runner.rs:1281` 的 p_t 输入语义需裁定）——设计层已标注 N_derived 只读派生，具体 margin 口径（CME-simple 是否按毛腿计）留编排者裁定。
- 本工位未实装任何代码（纪律：只写 .md）。

## 9. 锚索引

**代码锚（worktree /tmp/kimi-nest-mainline）**：

- 净额执行链：`runner.rs:1041-1048`（fill loop 签名）、`:1060-1061`（fee_rate）、`:1064-1066`（cash/units 标量）、`:1076`（pending 队列）、`:1104`（n_orders_executed）、`:1191-1208`（延迟成交+计数）、`:1279`（base_units 每 bar 重算）、`:1462-1476`（pi_theta_step_traced 调用）、`:1543/1552`（PanDiv 覆写同订单槽）、`:1555-1567`（overlay 只读旁路+ΔN 断言）、`:1856-1863`（单订单挂单）、`:1903-1910`（overlay 终点强平）、`:1997-2007`（FillOutput 装配）
- 口径字段：`runner.rs:70`（n_orders）、`:652-679`（OverlayRunResult，:659 n_overlay_fill_events / :663 n_overlay_voices）
- 成交机：`runner.rs:3514-3568`（track_position_transition，F-06 于 :3550-3563）、`:3691-3723`（apply_order）、`:3736-3800`（apply_fill，费于 :3778）
- 决策层：`coverage.rs:1379-1394`（SepLeg.q_units）、`:1426-1428`（units 公式）、`:1528/:1552`（units=base_units×w）、`:2307`（p̃=net_target_units）、`:2342-2353`（sep_legs 打包）、`:2642-2651`（pi_theta_position）、`:2667-2672`（cap）、`:2692-2727`（schedule_order，:2694 round、:2697-2702 零量不交易）、`:2907-2939`（P1 强平支路）、`:3031`（标准订单出口）
- 声部账本：`overlay_state.rs:13-18`（三恒等）、`:59-95`（VoiceBook/ClosedVoice）、`:101-113`（OverlayState）、`:170-250`（step：:186-216 离场、:218-244 开仓/resize、:246-249 ΔN）、`:308-327`（对冲净零测试）、`:331-350`（Σpnl_v 对账测试）
- 配置：`config.rs:208`（default_lot=1）、`:222-235`（commission 1bps + slippage 2bps + tax 0 = 3bps）

**文档/数据锚**：

- `chanlun/review-results/m8-opsem-vs-norders-recon-20260719.md` §1.3/:57-59（75629/1554 ≈ 48.7× 同口径比）
- `/tmp/m8_e2e_all_systems_oos.md:21-23`（三窗 n_orders / ΣN_tΔP_t / Comm+Slip 表行）、`:28`（层2判语）
- `/tmp/m8_opsem_p3fold.out:283-288`（三窗 bar 数 260560/260560/264960 + 结算行）
- 设计锚（代码注释转引）：多空对冲.pdf §10.1（净额 N=Σσq）/ §10.2（hedge-mode (Q⁺,Q⁻)）/ §11（价格 PnL 线性恒等）/ p16 关卡10；TARGET_STRATEGY_MAXFULL.md §M5
