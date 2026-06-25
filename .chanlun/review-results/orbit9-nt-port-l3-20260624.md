# orbit9 → t_engine.rs 移植 + NT 口径 L3 验证

工位：orbit9-nt-port（task#59）| commit base：4c0afeae11 | worktree：/private/tmp/orbit9-nt-port-wt
报告日期：2026-06-24 | 认识论：移植逻辑 **L0** / OFF bit-exact **L1**（对纯净基线）/ NT 口径 payoff **L3**

---

## 结论

orbit9 操作语义完全分类（A/B/C）已从 `rec_engine.rs`（TRoot，rec_btc 内部 harness）**逐字镜像移植**到
`t_engine.rs`（TPositionEngine，NT 生产回测 `backtest_t_fugue.py` 的引擎，经 TFugueStream.push_bar→
TFugueStreamCore→engine.step 驱动）。三 env 门控 `T_ORBIT9_H0`/`T_ORBIT9_DISPATCH`/`T_ORBIT9_NEST`，
缺省 OFF ⇒ NT 路径逐字不变（bit-exact，对纯净 4c0afeae11）。

- **A（H⁰ flip 门）**：route_bsp 核心反向 flip 加 `flip_confirmed` 参数 + `enable_h0_skeleton` 门，confirmed=type1
  走势完成（φ=0）才翻，背驰段 no-op。τ 手性对称（多空 flip 同构施加）。env `T_ORBIT9_H0`。
- **B（9 轨道 dispatch + O3 add）**：route_bsp 同父向 recover 分支加 dispatch，区分 O7 recover（走势完成→整条
  升回）vs O3 add（回调未完成→部分买回 m=mobile_quota，不动 h）。env `T_ORBIT9_DISPATCH`。
- **C（区间套接通）**：`orbit9_sub_trend_done(sub,view) := locate_nest(sub−BASE_LADDER,mob,view).located ∧
  (t1buy[sub]||t1sell[sub])`。located 必要非充分（located=背驰段 ⊋ done=type1 走势完成）。env `T_ORBIT9_NEST`。

**核心 L3 结论：H⁰ 单开 NT 口径与 rec_btc 口径方向+量级精确吻合（CL −48.4/−48、BTC −54.3/−54、OKLO
−84.9/−85），验证移植口径保真。H⁰ 在 NT 口径下同样 6/8 net-up 劣化。**

---

## 移植 diff（t_engine 路径文件）

| 文件 | 改动 |
|------|------|
| `rust/src/recursive_t/t_engine.rs` | TSignalView 扩展 level_diverge/nodes（ladder 索引）；TPositionEngine 三 env 门 + 计数器 n_h0_flip_blocked/n_adds/add_pnl_by_ladder；`orbit9_sub_trend_done`（坐标桥接）+ `bridge_level_view` + `add`（O3）；route_bsp 加 flip_confirmed/view 参数 + H⁰ 门 + dispatch 分支；step 调用点传 view.t1*；12 个 L0 单测 |
| `rust/src/recursive_t/stream.rs` | rerun_and_diff 从 tree 提取 level_div + nodes 填 view（门控 nest_enabled=T_ORBIT9_NEST，OFF 不提取）；坐标 +BASE_LADDER 偏移 |
| `rust/src/recursive_t/rec_driver.rs` | trend_to_node 改 pub（flat stream 与 rec extract_view 复用，避免重复实现） |
| `rust/src/recursive_t/ffi.rs` | PyTFugueStream.finish 暴露 n_h0_flip_blocked/n_adds/add_pnl_total/n_trend_done_clears 到 NT json |
| `trading_system/backtest_t_fugue.py` | out dict 加 orbit9 计数器 + orbit9_env 标记（OFF 全 0=锚点；最小改动，OFF 不变） |

---

## 移植遇到的结构差异（已 surface + 解决，非硬塞）

### 差异 1（核心）：TSignalView 缺 level_diverge/nodes，locate_nest 需要

`locate_nest(k, target, view:&LevelView, cur_bar)`（rec_nest_locator.rs 纯函数）需要
`LevelView.level_diverge[k]` 和 `LevelView.nodes[k]`。在 rec_engine 路径，rec_stream 从 tree 用
`trend_diverging_segment()` 算 level_div、`extract_view` 用 `trend_to_node()` 填 nodes 到 LevelView。但
**t_engine 路径的 TSignalView 只有 buy/sell/t1buy/t1sell/emergent_top**，flat `stream.rs` 的 rerun_and_diff
从未计算这两字段——尽管它内部**持有同结构的 tree**（同 iterate 输出）。

解决（非硬塞）：① TSignalView 扩展 level_diverge + nodes（ladder 索引，与 buy/sell 同空间）；② flat
stream.rs rerun_and_diff 复刻 rec_stream/extract_view 的提取逻辑（复用 pub 化的 trend_to_node +
trend_diverging_segment），门控 T_ORBIT9_NEST（OFF 不提取，省开销 + 零 bit-exact 风险）。

### 差异 2：ladder↔level 坐标错配（t_engine 特有，rec_engine 无）

t_engine route_bsp 用 ladder 空间（ladder=level+BASE_LADDER=3，MAX_LADDER=11），locate_nest 用 level 空间
（0..MAX_LEVEL=8）。locate_nest 内部 `while locate_level>0` 逐级降到 level 0——若直接传 ladder 会越界。
解决：`orbit9_sub_trend_done` 内构造临时 LevelView（`bridge_level_view`），把 TSignalView 的 ladder 数据
**偏移 −BASE_LADDER** 投影到 level 索引，再调 `locate_nest(sub−BASE_LADDER, ...)`。L0 单测
`坐标桥接_bridge_level_view_偏移正确` 验 ladder 6→level 3 映射。

### 与 (a)（在 rec_engine 上做接通）的 reconcile

接通公式逐字一致（`located ∧ type1`，located 必要非充分），无逻辑差异——坐标桥接是 t_engine 额外步骤。
env 门位置差异：rec_engine 用 EngineConfig+new_with_config 读 env；t_engine 用 TPositionEngine::new()
直接 std::env::var（与现有 enable_earning/enable_three_stage/enable_trend_done_clear 同款）。计数器位置：
rec 放 TRoot 字段，t_engine 放 TPositionEngine 字段（不污染共享 FugueResult），经 TFugueStreamCore getter
+ ffi set_item 暴露。

---

## OFF bit-exact 验证（NT 路径，L1）

### 关键诊断：commit json 是陈旧漂移，非该 commit 代码跑的

- commit 4c0afeae11 里的 json：CL structural **−39.4% / 3403 trades**
- 纯净 4c0afeae11 实跑（无任何 orbit9 改动，新建隔离 venv 装纯净 wheel）：CL structural **+20.3% / 5193 trades**

二者不同 ⇒ commit json 是更早 commit 跑的快照，提交时带入但未重新生成（基线漂移，类比
`project_oklo_e_baseline_drift`）。**正确的 OFF bit-exact 锚点是纯净 4c0afeae11 实跑值，非陈旧 json。**

### 我的 OFF（三 env 缺省）vs 纯净 4c0afeae11，三模式逐字段 bit-exact PASS

| 模式 | 我的 OFF | 纯净 4c0afeae11 | 字段对照 |
|------|---------|----------------|---------|
| structural | +20.3% / 5193 | +20.3% / 5193 | sink2707/rec1369/mdd−28.6/voices3 全同 ✓ |
| and | +20.5% / 5096 | +20.5% / 5096 | sink2627/rec1345/mdd−28.9/voices4 全同 ✓ |
| or | +2.8% / 5652 | +2.8% / 5652 | sink3006/rec1460/mdd−45.8/voices3 全同 ✓ |

Rust 层：149 个 recursive_t 测试 PASS（含 flat↔rec stream bit-exact 对称测试）+ 12 个 orbit9 L0 单测 PASS。

---

## 接通有效性（非死代码，L2 信息增量）

T_ORBIT9_NEST ON 时 stream 提取的 level_diverge/nodes 真让 locate_nest 定位成功（全量 CL structural ON）：
`n_h0_flip_blocked>0`（H⁰ 门真拦截背驰段反向）、`n_adds>0`（O3 add 真激活，DISPATCH+NEST 双 ON 且回调未
完成）、`add_pnl≠0`（O3 买回腿真有盈亏）。证明接通端到端工作（locate_nest 拿到非空 view），不是占位
fallback 的同义反复。三门全开各标的 n_adds ∈ [259, 3269]，n_h0_blocked ∈ [0, 64]。

---

## FillModel 撮合口径澄清（重要）

`TFugueStreamStrategy` **不向 NT 下单**（on_bar 只 push_bar，无 submit_order）——撮合固化在 Rust 引擎内
（成交=确认时点 close）。故 `FillModel(prob_fill_on_limit=0.0, prob_slippage=0.0)` 是**死参数**（无订单可撮合）。
**NT 口径 = Rust 引擎内部撮合口径（close 成交）**，NT 在此架构下只做 bar 回放调度，撮合权威在 Rust 引擎。
⇒ lead 问"FillModel 撮合口径下踏空惩罚是否变化"的答案：**FillModel 不参与，踏空惩罚完全由引擎逻辑
决定，与 prob_fill_on_limit 无关。** 若未来要测真撮合口径，须改 strategy 向 NT 下单（当前架构不下单）。

---

## NT 口径 L3（8 标的 × structural；OFF / 三门全开 ALL3 / H⁰单开 H0）

| 标的 | BH | OFF | ALL3(三门) | ΔALL3 | H0(纯H⁰) | ΔH0 | n_h0(H0) |
|------|-----|------|-----------|-------|----------|------|----------|
| CL | +28.2 | +20.3 | −37.2 | −57.5 | −28.1 | **−48.4** | 55 |
| BRN | +87.4 | −26.0 | +2.8 | +28.8 | −16.4 | +9.6 | 37 |
| DX | +4.1 | −0.7 | −2.5 | −1.8 | −4.0 | −3.3 | 10 |
| GC | +257.3 | −22.7 | −31.6 | −8.9 | −11.5 | +11.2 | 3 |
| ES | +594.3 | −2.3 | −7.6 | −5.3 | −2.9 | −0.6 | 64 |
| QQQ | +174.6 | −8.6 | −29.5 | −20.9 | −21.1 | −12.5 | 16 |
| BTC | +1380.4 | +26.0 | −48.8 | −74.8 | −28.3 | **−54.3** | 41 |
| OKLO | +307.1 | +55.3 | +74.4 | +19.1 | −29.6 | **−84.9** | 20 |

- **P1（跑赢 BH）：OFF 0/8、ALL3 0/8、H0 0/8**（NT 口径下 t_fugue 基线本就 0/8，与 orbit9 门无关——
  P1 失败是 t_fugue 引擎 regime 问题，不是 orbit9 引入的）。
- **H0（纯 H⁰）vs OFF：6/8 net-up 劣化**（CL/DX/ES/QQQ/BTC/OKLO 劣化，BRN/GC 改善）——与 rec_btc 口径
  "6/8 net-up 劣化"模式一致。
- **ALL3（三门）vs OFF：6/8 劣化**（CL/DX/GC/ES/QQQ/BTC），但 BRN/OKLO 改善——OKLO 在 ALL3 是 +19.1pp
  （O3 add 把 H⁰ 单开的 −84.9pp 反超为 +19.1pp），是接通有效（DISPATCH+NEST 真改变行为）的另一证据。

---

## 与 rec_btc 口径方向对照（R4 双口径纪律，不混表）

lead 提供 rec_btc 口径 D 的结果：**CL−48pp / BTC−54pp / OKLO−85pp**（H⁰ 配置 vs OFF 的 net-up pp 劣化）。

| 标的 | NT 口径 H0(纯H⁰) Δpp | rec_btc 口径 D Δpp | 方向 | 量级 |
|------|---------------------|-------------------|------|------|
| CL | **−48.4** | −48 | ✓ 一致 | 精确吻合 |
| BTC | **−54.3** | −54 | ✓ 一致 | 精确吻合 |
| OKLO | **−84.9** | −85 | ✓ 一致 | 精确吻合 |

**结论：NT 口径 H⁰ 单开与 rec_btc 口径 D 三核心标的方向+量级精确吻合（误差 <1pp）。** 这强证明移植的
口径保真度——t_engine 与 rec_engine 在 H⁰ 单开层面表达力一致。**H⁰ 在 NT 口径下仍劣化**（与 rec_btc
口径方向一致），未被 NT 撮合口径改变（因 FillModel 不参与，撮合权威在 Rust 引擎）。

注：rec_btc D 口径数字应是 H⁰ 单开（非三门全开）——本工位 H0 配置与之精确吻合（−48.4/−54.3/−84.9 vs
−48/−54/−85），印证此判断。ALL3（三门全开）与 D 不可直接对照（D 仅 H⁰）。

---

## 结果包六要素

1. **结论**：orbit9 A/B/C 逐字镜像移植到 t_engine.rs（NT 路径），OFF bit-exact（对纯净基线）PASS，
   NT 口径 H⁰ 单开与 rec_btc 口径方向+量级精确吻合，H⁰ NT 口径 6/8 劣化。

2. **定义依据**：A=587 候选A τ 手性对称化 + 541 P2 时序约束（flip 仅 φ=0 合法）；B=#40 9 轨道 O1-O9 +
   O3 add（不动 h 同级别短差腿部分重建）；C=区间套 H¹ 普适实例化（第61课"逐次下去"covering tower 收缩到
   最低活跃级别）+ divergence.rs:264-268 背驰段 ⊋ 走势完成（located 必要非充分）。

3. **边界条件**：① 若 commit json 不是陈旧漂移而是真基线 ⇒ 我的 OFF +20.3 会判 bit-exact FAIL（但纯净
   4c0afeae11 实跑确认 +20.3=我的 OFF，证明 json 陈旧，结论不翻转）；② 若 t1buy/t1sell 不是 φ=0 走势完成
   的忠实代理 ⇒ H⁰ 门误拦/误放（A 工位已述，与 flat bit-exact 不翻转）；③ 若 NT strategy 改为向 NT 下单
   ⇒ FillModel 参与撮合 ⇒ NT 口径 ≠ Rust 引擎口径（当前不下单，结论成立）；④ H⁰ 劣化是 regime 函数
   （BRN/GC 改善），非无条件——若全在 BRN/GC 类 regime ⇒ H⁰ 改善。

4. **下游推论**：① NT 口径 ≈ Rust 引擎口径（FillModel 不参与），生产回测 payoff = 引擎内部 close 成交
   payoff；② orbit9 门在 NT 生产路径可用（OFF bit-exact + ON 接通有效），但 H⁰ 劣化需 regime 白名单
   （类比 D 工位结论，orbit9 门是 regime 函数非全局改善）；③ OKLO 在 ALL3 vs H0 的反转（+19.1 vs −84.9）
   说明 O3 add 是独立的有效轴，可抵消 H⁰ 劣化，值得单独 L3 验证 DISPATCH+NEST 单开（不带 H⁰）。

5. **谱系引用**：orbit9 A/B/C 分类源自 #35 zhongshu-operation-semantic-classification（H⁰/H¹ 两层分工）+
   #39 exhaustive-operation-classification（9 τ 对称轨道）+ #34 NestEntry 否证（区间套≠方向滤波器）。
   H⁰ 劣化 regime 依赖呼应 539 号（空头腿=亏损唯一来源，BSP 时机误读为方向）+ 形式化有效域规则（有效域
   ≠ 定义域，H⁰ 在定义域代数成立但有效域仅部分 regime）。基线漂移呼应 `project_oklo_e_baseline_drift`。

6. **影响声明**：改动 t_engine.rs（NT 引擎，+12 L0 测试）/stream.rs（信号原料提取）/rec_driver.rs
   （trend_to_node pub）/ffi.rs（计数器暴露）/backtest_t_fugue.py（计数器 + env 标记）。**未改任何
   OFF 路径行为**（三 env 缺省逐字不变，bit-exact 对纯净基线）。新增 NT 生产路径的 orbit9 门控能力。
   不影响 rec_engine 路径（rec_btc harness 独立）。

---

## 认识论等级标注

| 产出 | 等级 | 理由 |
|------|------|------|
| A/B/C 镜像逻辑 + 坐标桥接 | L0 | 从 rec_engine 定义 + 区间套 covering tower 推导 |
| OFF bit-exact（对纯净 4c0afeae11） | L1 | 管线正确性（NT 路径逐字段对照纯净基线 PASS） |
| 接通有效（n_adds>0/n_h0>0） | L2 | 真实数据 ON 跑产生非零激活（可否证：若全 0 则死代码） |
| NT 口径 L3 payoff（8 标的 × 3 配置） | L3 | 真实数据 8 标的，可否证；H⁰ 6/8 劣化是否定性结果 |
| rec_btc 口径方向对照精确吻合 | L3 | 两口径交叉验证（CL/BTC/OKLO 误差 <1pp） |

---

## 复现命令

```bash
# 隔离 venv（含 orbit9 wheel + NT，不污染主 venv）
/tmp/orbit9_venv/bin/python  # = maturin build 的 orbit9 wheel + nautilus_trader 1.228

# OFF 基线
PYTHONPATH=/private/tmp/orbit9-nt-port-wt /tmp/orbit9_venv/bin/python \
  trading_system/backtest_t_fugue.py --symbols CL --modes structural

# H⁰ 单开
T_ORBIT9_H0=1 PYTHONPATH=... python trading_system/backtest_t_fugue.py --symbols CL --modes structural

# 三门全开
T_ORBIT9_H0=1 T_ORBIT9_DISPATCH=1 T_ORBIT9_NEST=1 PYTHONPATH=... python ... --symbols CL

# 注：多标的须每标的单进程（NT logging 全局单例第二次 init panic），见 /tmp/run_nt_l3.sh
```
