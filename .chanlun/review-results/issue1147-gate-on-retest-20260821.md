# #1147 卡口① gate-on 重测：锚修复后 `THETA_NEST_CERT_GATE=1` 净贡献 + 句 2 三档载体清单

- **issue**：[#1147](https://github.com/xy7365527-lang/NewChanlun/issues/1147)（正本 §十五第 3/4 条：门默认关 + 卡口①证书门接线的验收前重测）
- **性质**：测量 + 载体清单预置，**零生产码修改**。不开门、不裁「开门/维持关」。
- **基线 commit**：`main` 顶（本分支 `sandcastle/issue-1147` 基线 `355b839c29`；未 merge 后续 #1145 两 commit，不影响本票面）。
- **锚修复在案**：点锚迁移 #1052（`2ca72e2f6a`）+ 区间包含回退 #1076（`30b91ffb4d`）+ #1028 终验订正（`61a65172b3`：Type1 首步 C 92.7%→27.5%、方向桶 159→13）均已在 `main`。
- **日期**：2026-08-21（会话起于 08-20）

---

## 0. 一句话结论

**读数不再过期，也不再是「1/52」。** 锚修复后的现役判定链在 BTC 全年 2024 窗上门开：**净贡献为正且幅度大**——`strat_return −0.2078 → +0.4674`（+0.675）、`MaxDrawdown 0.4731 → 0.2637`（−44%）、`win_rate 0.4918 → 0.5061`；但**订单数不降反升 +7.2%**（222189 → 238245），成因 = 门的**出场侧反向项 χ^{σ_p}**（1101 个被拒候选里 **68.7% 是 would_close 出场候选**），不是 #796 当年测的开仓准入门。方向桶（历史「误拒」源）已从 #796 的 C 桶 62% 降到 **33.2%**；残余「误拒」主体换成 **227 例（20.6%）「旧臂 base gate 本会放行、被 typed 链 / Xzd 回退拒掉」**。typed 真链直接裁决仍只占 **3.2%**（chain_pass 27 + chain_reject 44 / 2212），其余 96.8% 走 NoChain → Xzd 回退。

**三档载体清单（句 2）见 §6**：接线声明 10 条——2 条结构性质（1 有 Lean 定理、1 缺独立定理）、3 条行为不变式（2 有 CI 测试锁、1 措辞须订正）、5 条禁令（grep gate 全可查或已裁）。本票探针本身 `#[ignore]` 按无锁计，不计入载体验收。

---

## 1. 复现口径

- **数据**：`analysis/data_cache/btc_1m_full.json`（337MB，worktree 符号链接）。
- **窗口**：`2024-01-01 .. 2025-01-01`（闭区间，528480 bar，`bar_seconds=60`）。全年单窗前台跑（照编排层二跑指引：不 `nohup &`，前台 `| tee`，工具调用等返回）。
- **双臂唯一差别** = 环境变量 `THETA_NEST_CERT_GATE`：
  - 臂 A 门关：`./target/release/theta_backtest BTC 2024-01-01 2025-01-01`
  - 臂 B 门开：`THETA_NEST_CERT_GATE=1 ./target/release/theta_backtest BTC 2024-01-01 2025-01-01`
- **StepFail 逐候选 dump**（只臂 B 有）：探针 `gate_on_stepfail_composition_dx`（`#[ignore]`）经线程局部 `NEST_CERT_GATE_OVERRIDE` 注入门开，`P1147_STEPFAIL_DUMP_PATH` 落 JSONL。
- **交叉一致性**：臂 B 的 ⑤ 段读数在 `theta_backtest` bin（env 注入）与探针（override 注入）两条路径逐位相同（orders 238245 / strat_return 0.4674 vs 0.467363）；1 个月冒烟窗（2024-01-01..02-01）亦逐位相同（15760 / −0.0116）。
- **两臂均 exit=0**；`fill.rs` 的 `expect("THETA_NEST_CERT_GATE=1 ⟹ 真链门状态已构建")` 未触发。
- **⑥ 段（真实 Nautilus BacktestEngine）**：不查此门（#796 已实测定性；1 个月冒烟窗两臂 ⑥ 逐位相同：总订单 3223 / 持仓 1209）。全年窗 ⑥ 受 30 分钟 timeout 截断未跑完——⑥ 不在本票交付面，读数不进本报告。
- **插桩非扰动**：本票新增代码全部 `#[cfg(test)]`（探针模块 + `#[ignore]` 驱动 + fill.rs 一行 `#[cfg(test)]` 调用）；臂 A（门关）走的是未改动的生产路径。

---

## 2. 双臂读数对比表（⑤ 段 `theta_v0::run_theta_v0_pi`，全年 2024）

| 读数 | 臂 A 门关 | 臂 B 门开 | 变化 |
|---|---|---|---|
| bar 数 | 528480 | 528480 | — |
| 不可交易占比 | 0.00% | 0.00% | — |
| **订单数** | **222189** | **238245** | **+7.23%** |
| **成交交易笔数** | **111635** | **119306** | **+6.87%** |
| **strat_return** | **−0.2078** | **+0.4674** | **+0.6752** |
| buy&hold_return | 1.2363 | 1.2363 | —（同窗同数据） |
| CAGR | −0.2070 | +0.4647 | +0.6717 |
| Sharpe | −0.0077 | +0.0230 | +0.0307 |
| MaxDrawdown | 0.4731 | 0.2637 | **−44.3%** |
| win_rate | 0.4918 | 0.5061 | +0.0143 |

**读法（照实）**：

1. **净贡献不再是「1/52」**：门开把全年 strat_return 从 −20.8% 拉到 +46.7%，MaxDrawdown 砍掉 44%。单标的单窗、且 buy&hold=+123.6% 仍跑赢双臂——**这读数只证「门开改变行为且方向为正」，不是 alpha 结论**（L3 门槛不变，见 §7）。
2. **订单数不降反升 +7.2%**，与既有测试 `nest_gate_env_gate_off_bitexact_on_shrinks_only` 里的「门开 ⟹ 订单只缩不增」**方向相反**。成因不是回归，是**测试夹具 vs 真数据的域差**：该测试用 60-bar 零信号合成夹具（无出场侧），真数据上门的出场侧反向项 χ^{σ_p}（§3.3）会拦掉平仓反向信号、改变持仓生命周期，订单总数可升。**「只缩不增」只对开仓准入门（准入语义）成立，不是门的总订单单调性**——本报告照实点名，归编排者裁是否须改该测试注释/断言口径。
3. 门开改善**主要由出场侧驱动**，不是开仓侧（§3.3）。

---

## 3. 拦住的信号构成（1101 个被拒候选）

臂 B 全年共 **2212** 个候选过门：**admitted=1111（nest_pass 27 + xzd_pass 1084）、rejected=1101（cert_none 189 + nest_n_delta_false 44 + xzd_gate_fail 868）**。

```
NEST_GATE_STATS total=2212 admitted=1111 rejected=1101 | nest_pass=27 xzd_pass=1084
                | rej: cert_none=189 nest_n_delta_false=44 xzd_gate_fail=868
NEST_GATE_CHAIN xzd_fallback=1016 | cross agree=1005 old_pass_new_rej=237 old_rej_new_pass=2 reuse=968
NEST_GATE_T3    chain_pass=27 chain_reject=44(missing=31 broken=13) chain_none=2141
NEST_GATE_EXIT_CAND total=886 admitted=130 rejected=756
```

### 3.1 通道构成（实际门裁决）

| 通道 | 计数 | 占比 | 含义 |
|---|---|---|---|
| xzd_gate_fail | 868 | 78.8% | typed 链 NoChain ⟹ Xzd 回退/复用判拒 |
| cert_none | 189 | 17.2% | 无证书（struct_break 157 + Type1 32：pass 16 / div_fail 16） |
| nest_n_delta_false | 44 | 4.0% | typed 真链判拒（missing 31 / broken 13） |

**★ typed 真链（T3 #172）直接裁决仅 71/2212 = 3.2%**（pass 27 + reject 44），其余 **96.8% 走 NoChain → Xzd 回退**。这与 #796「真链净贡献 1/52」同构——**但结论已反转**：锚修复后 Xzd 回退通道（旧臂，方向判据已修）把门做成了「净贡献为正」，真链本身仍近乎不直接裁决。

### 3.2 StepFail 桶构成（#852/#846 失败分类口径，旧臂 descend 归因）

| 桶 | 计数 | 占比 | 归因（div_cand_fail 条件号 / A/B/C） |
|---|---|---|---|
| cond1 方向 | 365 | 33.2% | `dir(s)≠−δ`——历史「方向误拒」源，锚修复后为诚实方向不符（#1028） |
| cond2 D-3 取段 | 226 | 20.5% | 无父中枢语境 / 未跨界 / 前序无同向段（#883 S4-b 统一取段） |
| l0_exempt | 183 | 16.6% | lvl0 Type2/3 旧臂免门，被 Xzd 回退拒 |
| struct_break | 157 | 14.3% | 零 bit 破中枢未背驰候选（codex 终局裁决A，无确认语义） |
| cond3 Extreme | 71 | 6.4% | 价格极值未更进一步 |
| cond4 Weak | 53 | 4.8% | 力度未衰减（#990 统一判据原语） |
| anchor_baseL0(d=1) | 28 | 2.5% | 下钻锚定成功（旧臂 base gate 过），被 Xzd 回退拒 |
| type1_pass | 16 | 1.5% | Type1 全 rung 过（旧臂会放行），cert_none 拒 |
| condNone | 2 | 0.2% | 首级过、更深级 div 假 |

**方向桶从 #796 的「C 桶 62%」降到 33.2%**（365/1101），与 #1028 终验「方向桶 159→13」同向但**不可直接对表**：本读数在门的 Type2/3 descend 面，#1028 在 Type1 首步 C 面，两套分母。

### 3.3 出场侧 vs 开仓侧（would_close = 反向项 χ^{σ_p} 消费线重叠面）

| 面 | 被拒 | 占比 | 通道/桶主体 |
|---|---|---|---|
| **出场侧 would_close=True** | **756** | **68.7%** | cond1 308 / cond2 175 / struct_break 112 / cond3 59 / cond4 39 |
| 开仓侧 would_close=False | 345 | 31.3% | l0_exempt 155 / cond1 57 / struct_break 45 / cond2 40 |

`NEST_GATE_EXIT_CAND total=886 admitted=130 rejected=756` —— 出场候选 886 个里 **85.3% 被拒**；开仓候选 1326 个里 345 被拒（26.0%）。

**★ 门的净贡献主要由出场侧贡献，不是 #796 当年测的开仓准入门。** 756 个被拒出场候选 = 反向平仓信号被门拦下 ⟹ 持仓不按反向信号平、生命周期重排 ⟹ 订单升 + 回撤收窄 + return 转正。这是 T5b (#208)「出场不独立设计、反向项升格为同点递归链确认」在真数据上的实测面。

**「N7 消费线重叠面」的口径边界（照实）**：本读数用 `would_close`（门自身的出场侧反向项 χ^{σ_p} 消费，T5b #208）作「消费线重叠面」的可测代理。票面点名的 N7 = #1097（`consume_at`/`consume_router` 谱系授权形成）接在**生产 π 回路 `ThetaPiStream`**，不在 `run_theta_v0_pi` 回测路径上（#1097 报告 §四明写「本接线不触碰回测路径任何一行」）⟹ **门的回测读数与 #1097 的 N7 消费线没有同一进程内的直接重叠面可测**。重叠面只有「门自己的出场消费」这一层在案；#1097 那条线与门的重叠面须在生产 π 回路另测（本票未做，见 §7）。

---

## 4. 三向区分（正确拦截 / 误拒 / N7 重叠面）

**操作化口径（本报告自定，供编排者拍板）**：以「旧臂 nest base gate 是否放行」为正确性参照——base gate 判拒 = 候选确实无有效区间套证书 ⟹ 正确拦截；base gate 会放行却被最终门拒 ⟹ 误拒。

| 类 | 计数 | 占比 | 组成 |
|---|---|---|---|
| **正确拦截** | 874 | 79.4% | cond1 方向 365（锚修复后诚实不符）+ cond2/3/4 350 + struct_break 157 + condNone 2 |
| **误拒** | 227 | 20.6% | l0_exempt 183 + anchor_baseL0 28 + type1_pass 16（旧臂 base gate 本会放行） |
| （正交轴）**N7 消费线重叠面** | 756 | 68.7% | would_close=True 的出场侧反向项 χ^{σ_p}（跨切上面两类） |

**读法**：

1. **「误拒」主体换了**：#796 的误拒主体是方向（C 桶 62%）；锚修复后方向桶只剩 33.2% 且为诚实方向不符 ⟹ 归入正确拦截。新误拒主体 = **227 例「旧臂 base gate 会放行、被 typed 链 / Xzd 回退拒掉」**（`NEST_GATE_CHAIN old_pass_new_rej=237` 与其同源，计数口径略差：237 含旧臂 Xzd-pass 但 base gate 判拒的候选，227 是 base gate 本会放行的子集）。
2. **227 例误拒是「维持关 / 开门」裁定的关键面**：它们不是「无证」，是「证在旧臂口径下成立、新口径（typed 链 + Xzd 回退）不认」。其中 183 例是 lvl0 Type2/3（旧臂明文「Type2@level0 不误拒」的免门群）。**这是新门的宽严分叉，不是锚问题**——归编排者裁，本票只读数。
3. **重叠面 756 例**：门把 85.3% 的出场反向候选拦下。出场侧的「正确/误拒」没有开仓侧那样干净的 base-gate 参照（出场不独立设计，T5b），故三向里出场侧只报数量、不硬分正确/误拒。

---

## 5. 未测项 / 局限（090 照实：以下一律「没测出来」，不写「不受影响」）

1. **单标的单窗**：只有 BTC 全年 2024。别的品种 / 别的年份没跑。strat_return 正负可能随窗翻转，**不是 alpha**。
2. **出场侧「正确/误拒」无法切分**：出场不独立设计（T5b #208），没有 base-gate 参照面；756 例出场拒里哪些该拦、哪些不该拦**测不出来**。
3. **#1097 N7 消费线与门的真实重叠面** —— **没测**。N7 在生产 π `ThetaPiStream`，不在回测路径；重叠面须在生产 π 回路另立探针（本票越界未做）。
4. **227 例误拒里有多少是「typed 链正确更严」vs「Xzd 回退误伤」** —— **没测**。需要逐例对 typed 链谱系（`chain_genealogy`）与 Xzd 证据再下钻。
5. **订单数上升的机制分解**（拦出场 ⟹ 生命周期重排 ⟹ 多出哪些订单）—— **没测**。只测了净订单 +7.2%，没逐腿归因。
6. **门开改善的过拟合风险**：2024 是 BTC 强趋势年（buy&hold +123.6%）；门开 return 转正可能与「拦出场 + 趋势延续」共振，跨年是否成立未验。
7. **`cargo test` 全量回归** —— **没跑**（越界；只跑了 `cargo check --tests` 绿 + 本探针 + 双臂回测）。

---

## 6. 句 2 三档载体清单（正本 §二第 2 句 / #799 裁定十一）

口径：**结构性质→Lean 定理；行为不变式→测试锁；禁令→grep gate。只编译不执行按无锁计（`#[ignore]` / 未入 CI 均计无锁）。**「接线」= 卡口① `THETA_NEST_CERT_GATE` 门（`backtest/admission.rs` + `fill.rs` 消费点）。

| # | 声明 | 档 | 载体 | 状态 |
|---|---|---|---|---|
| 1 | 门关闭 ⟹ π 全路径逐字节不变（bit-exact） | 行为不变式 | 测试锁 `nest_gate_env_gate_off_bitexact_on_shrinks_only`（`runner_tests.rs:5451`，CI 跑） | ✅ 有锁 |
| 2 | 门开启 ⟹ 严格链为唯一 nest 判定源，旧臂只双读落账 + Xzd 回退 | 结构性质 | Lean `formal/Strict/Nest.lean`（`NestCertificate`/`nest_certificate_unique`）+ `formal/Origin/IntervalNestCertificate.lean`（`selectΘ`/`chiBool`）；Rust `n_delta` docstring 明写 mirror Lean `(ℓ-e):Nat` 结构递归 | ✅ 有 Lean 定理；⚠️ **无 Rust↔Lean 对拍锁**（4 个 parity 测试只盖 buy/classifier/center/gap-overlap，不含 nest） |
| 3 | 门读出门 ≡ econ 门读出（禁第二裁决源） | 行为不变式 | 测试锁 `nest_gate_readout_matches_econ_gate`（`runner_tests.rs:5401`，CI 跑） | ✅ 有锁 |
| 4 | 同一判定每级同一函数（div_cand / descend 复用生产本体，禁宽严两档） | 禁令（宽严两档禁止，#799/#804） | #814 已裁收敛（`.chanlun/definitions/beichi.md`「次级别背驰的 Extreme 条件」节：div_cand 与旧 sublevel_diverges 收成一份保留 Extreme）；行为不变式对拍锁 = `nest_gate_readout_matches_econ_gate`（π/econ 门同函数同判据） | ✅ 已裁（正本）+ 对拍锁 |
| 5 | 层载由链路径单一驱动（链活 ⟹ 投影层必载，链死 ⟹ 不载） | 结构性质 | Lean 侧 `chain_driven_level_projection` 无对应定理；现靠 `backtest/admission.rs` 单元测试间接覆盖 | ⚠️ 无独立定理（见备注 A） |
| 6 | Xzd 回退单一来源（`build_xzd_fallback`，禁第二查法） | 禁令 | grep gate：`build_xzd_fallback` 调用点应仅 `admission.rs` + `econ_positive.rs` 本体 | ✅ grep 可查 |
| 7 | 门不在生产 π（`ThetaPiStream`）路径 | 禁令 | grep gate：`THETA_NEST_CERT_GATE` / `nest_cert_gate_enabled` 不得出现于 `stream.rs`（生产 π 回路） | ✅ grep 可查（本轮实测 `stream.rs` 零命中） |
| 8 | 门默认关（运行层 env 未设 = 关） | 禁令 | grep gate：`THETA_NEST_CERT_GATE` 显式置 1 仅 `scripts/check_armR_trades_digest.py`（臂 R 口径，已登记 #817）；生产默认不置 1 | ✅ grep 可查 |
| 9 | 拒绝不经 μ 桶键（R5-1 铁律，ext_i/entry_z 三维不动） | 禁令 | grep gate：`nest_gate_admit` 结果不得流入 `entry_z`/MuClass 桶键 | ✅ grep 可查 |
| 10 | 门开 ⟹ 候选集只缩不增（准入语义） | 行为不变式 | 测试锁 `nest_gate_env_gate_off_bitexact_on_shrinks_only` 第二断言（60-bar 夹具）；**真数据上订单总数可升（§2）** ⟹ 该断言的「订单数 ≤ 基线」措辞只对零信号夹具成立 | ⚠️ 措辞须订正（见备注 B） |

**备注（照实）**：

- **A（#5 无独立 Lean 定理）**：`chain_driven_level_projection`（`admission.rs:78-92`）是「层载由链路径单一驱动」的生产唯一派生点，但其结构性质（链活 ⟹ 投影层必载 / 链死 ⟹ 不载）**无 Lean 对应定理**，现只靠 Rust 单测间接覆盖 ⟹ 按句 2 计**缺载体**。补法：Lean `formal/Origin/` 增一条 `chain_driven_projection` 谓词定理（或降级声明为行为不变式改配测试锁）。
- **B（#10 措辞订正）**：既有测试注释「门开 ⟹ 订单数 ≤ 基线」在**真数据上被本票否证**（订单 +7.2%）。该断言锁的是「准入候选集只缩不增」，不是「总订单单调不增」。措辞与断言面不一致 ⟹ 应把测试注释/断言口径改为「门开 ⟹ 准入候选集 ⊆ 基线候选集」（对拍 `step_gamma_trade` 集合），**这是本票产出的一个须修面**（不在本票改，只登记）。
- **本票探针 `gate_on_stepfail_composition_dx`（`runner_tests.rs:5519`）本身 `#[ignore]`** ⟹ 按句 2「只编译不执行按无锁计」，**不计入任何载体验收**——它是诊断读数资产（照 #846 `type1_descend_continuity_dx` 先例），不是锁。

---

## 7. 复现

```bash
cd rust
export PYDIR=/home/agent/.local/share/uv/python/cpython-3.11.16-linux-aarch64-gnu
export LD_LIBRARY_PATH=$PYDIR/lib:$LD_LIBRARY_PATH LIBRARY_PATH=$PYDIR/lib

# 臂 A 门关（全年）
./target/release/theta_backtest BTC 2024-01-01 2025-01-01
# 臂 B 门开（全年）
THETA_NEST_CERT_GATE=1 ./target/release/theta_backtest BTC 2024-01-01 2025-01-01
# 臂 B StepFail 逐候选 dump（全年；≈12.8 min）
P1147_STEPFAIL_DUMP_PATH=/tmp/p1147_stepfail_2024.jsonl \
P1147_WINDOW_START=2024-01-01 P1147_WINDOW_END=2025-01-01 \
cargo test --release --lib gate_on_stepfail_composition_dx -- --ignored --nocapture
```

- 探针源：`rust/src/theta_v0/backtest/econ_positive.rs:2049`（`stepfail_probe` 模块，`#[cfg(test)]`）
  + `rust/src/theta_v0/backtest/fill.rs:5383`（逐候选调用点）+ `rust/src/theta_v0/backtest/runner_tests.rs:5519`（驱动）。
- 机器产原始数据：`.chanlun/review-results/issue1147-stepfail-raw-2024.jsonl`（2212 行，每行 = 候选 × stepfail 桶 × admit × channel × would_close）。
- 收尾：`python3 scripts/check_fixture_drift.py` → `FIXTURE-GATE ENVIRONMENT（非漂移）：lake 不在 PATH`（本 diff 零触碰 `formal/`，非漂移）。

---

*本报告只产读数 + 载体清单，不裁「开门 / 维持关」。门默认态 = 编排者按本票读数裁定（正本 §十五第 3 条）。*
