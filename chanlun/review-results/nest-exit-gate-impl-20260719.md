# 进出场区间套实装报告：出场=反向 typed nest 证书消费 + 进场准入 typed 证书门（nest-exit-gate-impl）

- **实装工位**：nest-exit-gate-impl（分支 `kimi-nest-mainline-20260717`，worktree `/tmp/kimi-nest-mainline`）
- **日期**：2026-07-19（行号锚核验于本工位完工快照）
- **编排者最高口径**：出场必须走区间套背驰——买点买卖点卖，所有买卖点必须经区间套背驰确认。
  止盈 = 反向买卖点（区间套确认版），不是百分比阈值。进出场同一条线。closePred 不加百分比止盈项。
  区间套把滞后压到因果下限但不承诺零滞后——验收按可修复分量 vs 内禀分量分别落账。
- **依据**：`nest-exit-implementation-card-20260719.md` §2（出场实装卡）+ `nest-gate-elevation-design-20260719.md` §3（进场升格路径 b 实装卡）。
- **纪律执行**：v3 硬禁令（无概率/统计推断、无回测验证策略、无 EMH 假设——门判据全部来自结构：
  bits/塔段/⊆/Cand，零 f64 阈值）；090（声明=能力，v0 基例拒绝率≈0 照实标注，见 §2.3/§8）；
  无 git mutation；主仓零写（BTC 数据经 worktree 内既有符号链接只读引用）；`rust/Cargo.toml` 未触碰；
  typed_ledger/TW/sep_legs 决策层语义零改动（门关时逐字节不变由回归锁测试锚定）。
- **授权文件**：`rust/src/theta_v0/strategy/exit.rs`（改）、`rust/src/theta_v0/strategy/interp.rs`（改，
  任务书所给 `backtest/interp.rs` 实位于 `strategy/interp.rs`，设计卡行 7 已注）、
  `rust/src/theta_v0/backtest/runner.rs`（改）、`rust/src/theta_v0/backtest/econ_positive.rs`（**只读引用，零改动**）。

---

## 1. 实装总览（两臂一门一开关）

| 臂 | 实装 | 锚 | 默认路径 |
|---|---|---|---|
| ①出场 | 反向项 χ^{σ_p} 消费对象：裸 BspBits → typed nest 证书（v0 基例门） | `exit.rs:158`（`exit_decision_for_nested_cert`）/ `exit.rs:181`（`reverse_nest_cert_base`）/ `exit.rs:217`（impl 门段） | 逐字节不变（旧三入口委托 gate=false） |
| ①接线 | runner v1 退出循环 / 双账嵌套退出循环，env `THETA_NEST_CERT_GATE=1` 切换 | `runner.rs:3513`/`:3853` | env 未设 ⟹ 旧函数，bit-exact |
| ②进场 | π 开仓准入门消费 typed nest 证书（`build_gate_certificate` 单一裁决源） | `runner.rs:1717-1731`（门段）/ `runner.rs:996`（`nest_gate_admit`）/ `runner.rs:1344`（hist 供给） | env 未设 ⟹ `None => step_gamma_trade` 直通，bit-exact |
| ③closePred | **零新析取项**：四析取 + F1 锚不动；出场只改既有反向项的**消费对象**；进场门在 fold 外（Γ 过滤层，与 χ 门同语义层） | `exit.rs:196-217`；`exec.rs close_pred` 未触碰 | — |
| 开关 | env `THETA_NEST_CERT_GATE` + 线程局部测试 override（VOICE_EXEC 同惯例） | `runner.rs:739-760` | 未设 ⟹ 全关 |

**单一裁决源**：进场门 = `econ_positive::build_gate_certificate`（econ_positive.rs:1572）直调，通过读出
（Nest→`n_delta` / Xzd→`gate_pass` / None→拒）逐字镜像 econ_positive.rs:367-371，无第二裁决源——
由对拍测试 `nest_gate_readout_matches_econ_gate`（runner.rs:5913）构造性守护。
出场门 = `interp::nest_confirm`（interp.rs:382，本工位 `fn` → `pub(crate)`，唯一签名改动）经
`exit::reverse_nest_cert_base` 复用——进出场**同一台机器**（`nest::chi_bool`，nest.rs:122）、
**同一谓词**（N^δ base case Conf^δ_e）、**方向镜像**（持多消费卖侧链 χ⁻ / 持空消费买侧链 χ⁺）。

## 2. 出场实装（任务①）

### 2.1 机制

`exit_decision_impl` 增第 8 参 `nest_cert_gate: bool`（exit.rs:198）：
- `false` ⟹ 反向项 = 裸 BspBits 现行语义（`exit_decision_for` / `exit_decision_for_nested` 两旧入口
  均委托 false，**逐字节不变**）；
- `true` ⟹ depth=0 反向项 = `reverse_signal(hv.side, &d.bsp) && reverse_nest_cert_base(d)`
  （exit.rs:217）——消费对象升格为 typed nest 证书。depth>0 ShortDiff 腿的 F1 段终点锚路径
  **逐字节不动**（F1 锚是否同样证书化是卡 §4-3 登记的裁定项，本工位不代裁）。

`reverse_nest_cert_base`（exit.rs:181）：对反向开仓决策自身方向 δ′ = `voice_side(root_side, depth)`
读证书基例 Conf^{δ′}_e（复用 `nest_confirm`，不 fork 第二套证书判据）。

**触发点**：反向候选部署 bar `i`——v0 基例的 first_provable ≡ 候选确认 bar（由候选
`signal_index ≤ i` 因果承载）。v1 E2E-N3 真链接入后改读**最深处证书 first_provable**
（E2E-L 五钟，roadmap:75），经本谓词接口预留——升格只换 `reverse_nest_cert_base` 实现，
调用点与四析取结构不动。

### 2.2 接线

- v1 退出循环（`plan_and_fill_mtm`，runner.rs:3513-3535）与双账嵌套退出循环
  （`plan_and_fill_mtm_dual`，runner.rs:3853-3878）：env 开启时改调 `exit_decision_for_nested_cert`
  （实参 `parent_invalid` 原样透传），未设 ⟹ 旧函数逐字节不变。
- **授权外未接线**（照实登记）：nautilus `ThetaCore`（nautilus/strategy.rs:166，非授权文件）；
  interp.rs 规则2 close 桶（π 生产路径出场，interp.rs:1197-1209）——其证书化需穿透
  `interpret`/`coverage_step` 签名族或接受默认行为变更，超本工位最小改动面，登记为待接线项（§9）。

### 2.3 v0 诚实有效域（090 声明=能力）

基例 = 单级末端 Conf（Classification 不导出跨级塔，interp.rs:379-381 同源诚实标注）。
`conf_minus ≡ sell1∨sell2∨sell3`（types.rs:210-212）与 `reverse_signal(Long,·)`（exec.rs:257）
**同源**（买侧镜像同）⟹ 对**方向干净**的反向候选，基例门拒绝率 ≈ 0——**照实声明：v0 基例
当门不是真跨级门**（设计卡 §3.2 同口径）。真拒绝域 = 方向冲突（双触发消歧为 Flat）与零 bit
候选（`nest_confirm` 对 Flat 恒 false），由单测 `reverse_cert_gate_rejects_flat_direction` 锚定。
真跨级拒绝率只能来自 v1 E2E-N3 真链（Candidate∧⊆∧递归 + first_provable 五钟）。

## 3. 进场实装（任务②，升格路径 b）

### 3.1 机制与挂载

π fill loop（`pi_theta_fill_loop_overlay`，三个公开入口的唯一函数体）内、χ 过滤出口之后、
`pi_theta_step_traced` 消费之前（runner.rs:1717-1731）：`nest_gate_hist = Some` 时对
`step_gamma_trade` 逐候选过 `nest_gate_admit`（runner.rs:996-1058）：

- Flat 方向 ⟹ 拒（与 `nest_confirm` interp.rs:382 同语义）；
- 否则 `build_gate_certificate(&tower_i, lvl, c.source_index, δ, &c.bits, hist, i, &ls.bsp, sub_centers, sub_bsp)`
  ——`bsp_of_level`/次级视图取**前缀因果分类** `classification_i`（≤i 数据，与 econ 侧 `cls_i.levels`
  同口径），`confirm_index = i`（与 econ 侧同口径）；
- `Some(Nest(cert))` ⟹ `cert.n_delta()`；`Some(Xzd(ev))` ⟹ `ev.gate_pass()`；`None` ⟹ 拒。
  拒 ⟹ 从当 bar 候选集剔除 ⟹ interpret 不归 open ⟹ **不开仓**（与 χ 门同一语义层，runner.rs:1556-1559
  注释同款「gamma 滤掉 ⟹ 不开仓」）。

**hist 供给**（runner.rs:1344-1362）：复用 `pan_div_hist`（center_oscillation 已开则零成本），否则
门开启时无条件算一次 `compute_macd`（同 config.macd，确定性 O(n) 一次性——不构成 bit-exact 风险，
只构成成本；设计卡 §3.2 同款）。

### 3.2 R5-1 铁律不触碰

门是**准入谓词不是 z 维**：`ext_i`（runner.rs:1529-1534）三维 `cand_channel/nest_depth/origin_level`
仍 None（G3 诚实口径不动）；`MuClass`/μ 桶键/`mu_estimator.rs:126-140` 零改动；R5-1 现场
（runner.rs:2611-2614/:2686-2689/:2793-2795）零改动。拒绝不进 μ 分桶，μ 分桶 bit-exact 由
门关回归锁（§5 NG-E④）+ 既有 R5-1 测试网双层守住。

### 3.3 观测落账

门开启时逐候选落 `NestGateStats`（runner.rs:956-987，通道闭集：nest_pass / xzd_pass /
flat_dir / no_level / cert_none / nest_n_delta_false / xzd_gate_fail），fill loop 收尾经 stderr 输出
`NEST_GATE_STATS` 行（runner.rs:2461-2476，env 纯诊断惯例，门关恒零不输出）。

## 4. closePred 结构判据零变更（任务③）

- `exec::close_pred`（Lean 锚四析取）**未触碰**；`exit_decision_impl` 四析取 + F1 锚结构不动——
  出场升格只改反向析取项的**消费对象**（裸 bits → typed 证书），不加第五析取、不加百分比、
  不加任何 f64 阈值（MFE 撤回登记 exit.rs:33-37 的纪律在本实装中保持）。
- 进场门在 interpret fold **之外**（Γ → Γ^cert 过滤层），fold 规则1-4 与三桶语义零改动；
  退出谓词全部判据 = 结构（bits/塔段含段查找/⊆/Cand/方向），无一处价格幅度阈值。

## 5. 单测（任务④，全部新增 6 个，全绿）

| 测试 | 锚 | 断言 |
|---|---|---|
| `reverse_cert_gate_direction_mirror` | exit.rs:557 | 出场证书消费**方向镜像**：持多消费卖侧链平多、持空消费买侧链平空；同向决策负对照；方向干净域证书变体 ≡ 默认变体逐字段（v0 同源见证） |
| `reverse_cert_gate_rejects_flat_direction` | exit.rs:600 | v0 基例门拒绝域：Flat 双触发候选裸 bits 路径触发而证书门**拒**（行为差照实登记）；`reverse_nest_cert_base` 谓词直读 |
| `reverse_cert_gate_default_path_regression` | exit.rs:630 | **默认路径回归锁**：`exit_decision_for`/`exit_decision_for_nested`（gate=false）在冲突候选下仍按裸 bits 触发，与历史行为一致 |
| `nest_gate_admit_semantics` | runner.rs:5839 | 进场门拒绝语义：合法证书放行（lvl0 Type3 基例 nest_pass）；非法拒绝（无执行段 cert_none / 零 bit StructBreak cert_none / Flat flat_dir / 两级塔 rung cand=false ⟹ nest_n_delta_false 跨级真拒绝）；单级塔基例退化放行照实登记（内禀分量） |
| `nest_gate_readout_matches_econ_gate` | runner.rs:5913 | **同源对拍**：π 新门读出 vs econ 门读出（econ_positive.rs:367-371 同判据独立构造）逐候选一致——不一致 = 第二裁决源实锤 |
| `nest_gate_env_gate_off_bitexact_on_shrinks_only` | runner.rs:5942 | **默认路径回归锁**：门关 ⟹ n_orders/trades/strat_return/equity 与基线逐字节一致；门开 ⟹ 订单只缩不增（门的设计语义） |

运行记录：

```text
$ cargo test --release --lib nest_gate
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1881 filtered out
$ cargo test --release --lib reverse_cert_gate
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1881 filtered out
```

## 6. 全绿证据（任务⑤）

```text
$ cargo test --release --lib
test result: ok. 1755 passed; 0 failed; 129 ignored; 0 measured; 0 filtered out; finished in 1.04s
```

- 基线 1749 passed / 128 ignored（`nest-exit-implementation-card-20260719.md:43` 同快照登记）→
  本工位 **1755 passed = 1749 + 本工位 6 个新测试，0 failed，零变红**。
- ignored 128 → 129（+1）：本工位新增测试**无一** `#[ignore]`；总测试数 1884 vs 基线 1877（+7 =
  6 本工位 + 1 他工位在途未提交改动，本 worktree 开工时 `git status` 已有 runner.rs/exit.rs 等
  14 个脏文件来自并行工位）。照实登记，非本工位引入。
- `cargo check --lib`：零 error；warning 计数与开工前同量级（32，全为既有 dead_code 类，
  grep 确认无 nest_gate/exit.rs 新增）。

## 7. 真门拒绝率实测登记（任务⑥）

**实测**（BTC OOS 三窗 m8_e2e，`THETA_NEST_CERT_GATE=1`，2026-07-20 补跑，EXIT=0）：

| 窗 | total | admitted | rejected | 拒绝率 | nest_pass | xzd_pass | rej: flat_dir | no_level | cert_none | nest_n_delta_false | xzd_gate_fail |
|---|---|---|---|---|---|---|---|---|---|---|---|
| p3fold | 1633 | 1324 | 309 | 18.9% | 1183 | 141 | 0 | 0 | 45 | 30 | 234 |
| wf7 | 1724 | 1433 | 291 | 16.9% | 1157 | 276 | 0 | 0 | 23 | 14 | 254 |
| wf8 | 1518 | 1225 | 293 | 19.3% | 1136 | 89 | 0 | 0 | 30 | 23 | 240 |

**结论**：门是真的——三窗拒绝率 17-19% 非零（修复前 0%）。主拒绝通道 = xzd_gate_fail（234-254，XZD 门实装生效）；cert_none/nest_n_delta_false 为小通道。

**门关 vs 门开 execR 对照（照实否定为负）**：

| 窗 | 门关 execR | 门开 execR | Δ |
|---|---|---|---|
| p3fold | −15,383,474 | −15,515,953 | −132,479（+0.9% 更负） |
| wf7 | −20,995,480 | −21,111,836 | −116,356（+0.6% 更负） |
| wf8 | −17,134,632 | −19,526,996 | −2,392,364（+14.0% 更负） |

**照实**：当前口径下门拒绝了部分盈利候选（wf8 显著）——v0 基例门的鉴别力主要不在跨级分量（rungs 空基例放行占比高，内禀于塔现状挂路径 c）；nest_n_delta_false 与 xzd_gate_fail 是门实装生效证据（可修复分量已修）。门对 net_r 的净效应为负属如实登记，不伪造通过。

## 8. 可修复 vs 内禀分量分账

- **可修复分量（已修）**：①nest 从贴标挂件升格为真门（拒绝率 0% → 17-19%，门实装生效）；②XZD 承接门生效（xzd_gate_fail=234-254 为主拒绝通道）；③cert_none/nest_n_delta_false 通道可观测。
- **内禀分量（不可修，如实落账）**：①rungs 空基例放行占比高 = 跨级链稀薄（内禀于塔现状，完整 N^δ 真链挂路径 c/横切⑪长线）；②T1 内禀延迟（背书点晚 19-590 bar）是「不预测」的必然代价，不可消除；③门对 net_r 的净效应为负（当前口径下拒绝含盈利候选）——v0 基例门鉴别力边界，v1 E2E-N3 真链当门才是跨级构造性拒绝。

## 9. 待接线项与边界（照实登记）

1. **interp.rs 规则2 close 桶证书化**（π 生产路径出场，interp.rs:1197-1209）：需穿透
   `interpret`/`coverage_step` 签名族或接受默认行为变更；本工位授权内只做了 `nest_confirm`
   的 `pub(crate)` 可见性提升（唯一签名改动），规则2 本体未动。
2. **mod.rs close 桶路径**（recognize_nested 常规反向关闭，mod.rs:699-708）：卡 §4-4 已登记，
   本工位未触碰。
3. **nautilus `ThetaCore`**（nautilus/strategy.rs:166）：非授权文件，未接线；接线时复用
   `exit_decision_for_nested_cert` 同一入口，零新逻辑。
4. **v1 E2E-N3 真链**：跨级 Candidate∧⊆∧递归 + first_provable 五钟（E2E-N2→N3→N5→N7 依赖序，
   roadmap:79）；接口已预留（出场 `reverse_nest_cert_base` / 进场 `nest_gate_admit` 的 tower 链
   消费点），真链喂入后两门拒绝率自动含跨级分量。
5. **F1 段终点锚证书化**（depth>0 ShortDiff 腿）：卡 §4-3 裁定项，本工位不代裁、未触碰。
6. **trig-bsp-class-mapping-recon 缺席**：卡 §2.3 已照实登记；本工位类别映射未新增消费点，
   无需对账。

## 10. 改动清单（diff 摘要）

| 文件 | 改动 | 净增 |
|---|---|---|
| `rust/src/theta_v0/strategy/exit.rs` | `exit_decision_for_nested_cert`（:158）+ `reverse_nest_cert_base`（:181）+ `exit_decision_impl` 第 8 参（:198，两旧入口委托 false）+ 反向项门段（:211-220）+ 模块头接线状态更新 + 3 个 NG 单测 | 约 +190 |
| `rust/src/theta_v0/strategy/interp.rs` | `nest_confirm` `fn` → `pub(crate) fn`（:382，唯一签名改动）+ doc 增出场复用注记 | +6/−1 |
| `rust/src/theta_v0/backtest/runner.rs` | `nest_cert_gate_enabled` + override（:739-760）+ `NestGateStats`（:956）+ `nest_gate_admit`（:996）+ hist 供给（:1344）+ 门段（:1717-1731）+ 统计输出（:2461）+ v1/双账退出循环接线（:3513/:3853）+ 3 个 NG-E 单测 | 约 +330 |
| `rust/src/theta_v0/backtest/econ_positive.rs` | **零改动（只读引用）** | 0 |
| `rust/Cargo.toml` | **零改动（纪律）** | 0 |

注：worktree 开工快照中上述文件已含并行工位未提交改动；`git diff --stat` 总数（runner +1012/
exit +350）含他工位在途内容，本表只计本工位净增。
