# #796 区间套证书门开一次：那四条线的东西通电之后活不活

**日期**：2026-07-30 ｜ **票**：#796（map #787）｜ **性质**：测量，不是修复。零生产码修改。

## 0. 复现口径

- 快照：`git archive main` → `/tmp/nc-main-796`（HEAD = `main`）。数据经 symlink 接
  `analysis/data_cache`（314MB，未复制）。**主仓与 worktree 零构建、零改动**（`git status --porcelain` 空）。
- 二进制：`cargo build --release --features backtest_bin --bin theta_backtest`（exit=0，39.5s，
  依赖预热经 `cargo check` exit=0）。
- 命令：`./target/release/theta_backtest BTC 2024-01-01 2024-01-07`
- 双臂唯一差别 = 环境变量 `THETA_NEST_CERT_GATE=1`。
- **两臂均 exit=0**。臂 B 重跑一次读数逐位相同（确定性已验）。
- **插桩非扰动已证**：臂 A（门关）插桩后读数 = #792 基线逐位相同
  （3136 / 2482 / −0.1025 / 2754 / 177）。

---

## 1. 双臂读数对比表

### ⑤ 段 `theta_v0::run_theta_v0_pi`（in-crate 路径）

| 读数 | 臂 A 门关 | 臂 B 门开 | 变化 |
|---|---|---|---|
| bar 数 | 10080 | 10080 | — |
| 不可交易占比 | 0.00% | 0.00% | — |
| **订单数** | **3136** | **1794** | **−42.8%** |
| **成交交易笔数** | **2482** | **1263** | **−49.1%** |
| **strat_return** | **−0.1025** | **−0.0322** | 亏损收窄 |
| buy&hold_return | 0.0385 | 0.0385 | —（同窗同数据） |
| CAGR | −0.9965 | −0.8185 | — |
| Sharpe | −0.3749 | −0.1193 | — |
| MaxDrawdown | 0.1120 | 0.0537 | −52% |
| win_rate | 0.2752 | 0.3943 | +0.119 |
| 认识论等级 | L2 | L2 | — |

### ⑥ 段真实 Nautilus BacktestEngine

| 读数 | 臂 A 门关 | 臂 B 门开 | 变化 |
|---|---|---|---|
| 引擎迭代次数 | 10080 | 10080 | — |
| Total events | 6_655 | 6_655 | **零变化** |
| **Total orders** | **2754** | **2754** | **零变化** |
| **Total positions** | **177** | **177** | **零变化** |
| Expectancy | −4772.11 | −4772.11 | 零变化 |
| **PnL% (total)** | **−23.06** | **−23.06** | **零变化** |
| Win Rate | 0.01 | 0.01 | 零变化 |
| Profit Factor | 3.49 | 3.49 | 零变化 |

**⑥ 段逐项逐位相同。**插桩计数给出机制解释：`GATE_ON` 在 `after-5` 与 `after-6` 两个转储点
均为 2（无增量）⟹ **⑥ 段一次都没有查询过这个门**。门不在 Nautilus 路径上。

### 关键行为读数（插桩计数，两个转储点）

| 计数器 | 臂 A（after-5 / after-6） | 臂 B（after-5 / after-6） |
|---|---|---|
| `GATE_ON` / `GATE_OFF` | 0 / 2 ｜ 0 / 2 | 2 / 0 ｜ 2 / 0 |
| **`DEC_B1`（一类买点决策）** | 0 ｜ **0** | 0 ｜ **0** |
| **`DEC_S1`（一类卖点决策）** | 0 ｜ **0** | 0 ｜ **0** |
| `DEC_B2` / `DEC_B3` | 0 ｜ 9498 / 59890 | 0 ｜ 9498 / 59890 |
| `DEC_S2` / `DEC_S3` | 0 ｜ 11531 / 36775 | 0 ｜ 11531 / 36775 |
| `DEC_BUILD`（决策构造总数） | 0 ｜ 117694 | 0 ｜ 117694 |
| **`RECOG_CHILD_OK`（depth>0 子声部产出）** | 0 ｜ **0** | 0 ｜ **0** |
| `RECOG_CHILD_FALLBACK`（落回 depth=0 根） | 0 ｜ 117694 | 0 ｜ 117694 |
| `RECOG_NESTED` | 0 ｜ 10080 | 0 ｜ 10080 |
| `PI_STEP` | 10080 ｜ 10080 | 10080 ｜ 10080 |

**一类买卖点决策：门开后仍恒 0。depth>0 嵌套子声部产出：门开后仍恒 0。**

⚠ 必须照实说明的口径限制：`DEC_*` 与 `RECOG_*` 全部只在 `after-6` 才非零
（`after-5` 全 0）⟹ 这两组读数活在 **⑥ 路径**上，而 ⑥ 路径**根本不查这个门**。
所以「门开后仍为 0」在此处是**结构性必然**，不是「开了门也压不出来」的实验结论——
门与这两个读数之间根本没有因果通路。

### 门开臂独有的生产诊断行（臂 A 无此输出）

```
NEST_GATE_STATS total=52 admitted=14 rejected=38 | nest_pass=1 xzd_pass=13
                | rej: flat_dir=0 no_level=0 cert_none=2 nest_n_delta_false=0 xzd_gate_fail=36
NEST_GATE_CHAIN xzd_fallback=37 | cross agree=14 old_pass_new_rej=26 old_rej_new_pass=0 reuse=12
NEST_GATE_T3    chain_pass=1 chain_reject=0(missing=0 broken=0) chain_none=51 | top_dist={0: 34, 1: 14, 2: 2}
NEST_GATE_INDEX events=15 events_seen=797 base_events=13 assembled=0 indexed=0
                rungs_0=0 rungs_1=0 rungs_2p=0 single_level_share=na
                | derivations=70 index_builds=5 provider_errors=0
NEST_GATE_LEVEL base_lk=[[0,0],[0,12],[0,1]] assembled_lk=[[0,0],[0,0],[0,0]] indexed_lk=[[0,0],[0,0],[0,0]]
                | base_trend=0 base_pan=13
NEST_GATE_EXIT_CAND total=12 admitted=2 rejected=10
```

---

## 2. 票面三件事逐个回答

### ① 活不活 —— **跑通了**

门开 `exit=0`，全窗 10080 bar 跑完，⑤⑥ 两段读数齐全，无 panic、无 abort。

特别地：**`fill.rs:4912` 的 `expect("THETA_NEST_CERT_GATE=1 ⟹ 真链门状态已构建")` 没有触发**。
插桩证实前提成立——`CHAINGATE_NEW=1`（`NestChainGate::new` 被调用一次，门状态确已构建），
`ADMIT_CALL=52` 说明该 `expect` 之后的 `gate.admit` 路径被真实走了 52 次。**这个已知高危点本轮实测安全。**

但「跑通」≠「口径可解释」。第 ② 条给出的是**第三种结果**：跑通了，但数没法按票面预期解释
（见下方「最锋利的一条」）。

### ② 谁被通电了 —— 三个落点，两通一不通

| 落点 | 归属 | 插桩计数（臂 B） | 判定 |
|---|---|---|---|
| `backtest/admission.rs` 受门控分支 | #106 / #126 | `GATE_ON=2`、`LVLPROJ_DERIVED=1`、`CHAINGATE_NEW=1`、`CHAINGATE_SYNC_INDEX=5`、`NESTIDX_BUILD=5`、`LOOKUP_CALL=52`、`ADMIT_CALL=52`（TRUE 14 / FALSE 38）、`OLDARM_CALL=52` | **真被执行** |
| `nest.rs` `N^δ` 相关路径 | 区间套递归核 | `NDELTA_CALL=39`、`NDELTA_TRUE=39` | **真被执行**（但调用者存疑，见下） |
| `nest_lifecycle.rs` | #597 | `LIFECYCLE_ADVANCE=0`、`LIFECYCLE_PROVIDE_PAN=0`、`LIFECYCLE_PROVIDE_ACTIVE=0` | **零触达。门开也没通电。** |

`chain_driven_level_projection` 的派生臂 `LVLPROJ_DERIVED=1`（臂 A 为 `LVLPROJ_PASSTHRU=1`）
⟹ 层投影配置确因门开而被派生启用，#168 裁定 3 那条「链活 ⟹ 投影层必载」在生产上真的生效了。

#### ★最锋利的一条：门开之后真正在滤单的**不是区间套真链，是 L2 旧臂 Xzd**

52 个候选里：

- **真链（`N^δ` 跨级递归，#106/#126 的东西）只对 1 个候选给出 Pass，对 0 个给出 Reject**
  （`chain_pass=1`、`chain_reject=0`、`nest_n_delta_false=0`）。
- **其余 51 个全是 `NoChain`** ⟹ 全部落到 L2 旧臂 Xzd 回退通道
  （`chain_none=51` = `xzd_fallback=37` + `reuse=12` + `cert_none=2`）。
- 被剔除的 38 个里，**36 个是 `xzd_gate_fail`**（旧臂判的），2 个是 `cert_none`。

**⟹ ⑤ 段订单从 3136 掉到 1794 这 43% 的削减，几乎全部由 L2 旧臂 Xzd 造成，
不是由区间套真链造成的。区间套真链对准入的净贡献 = 1 个 admit / 52。**

配套证据：typed 真链证书索引**造出来是空的**——`assembled=0 indexed=0`、
`rungs_0/1/2p` 全 0、`indexed_lk` 三级全 `[0,0]`，而 `index_builds=5`、`derivations=70`
（索引确实反复重建了，只是每次都建出空的）。`NDELTA_CALL=39` 与 `xzd_fallback=37`
数量级吻合 ⟹ **39 次 `N^δ` 调用高度疑似全部来自 L2 旧臂
`nest_gate_admit`→`build_gate_certificate`，而非 #74/#75 的 typed 真链索引。**
（未做调用方分桶插桩，此条为**推断**，见未测项。）

这正是票面预置的**第三种结果**：跑通了，但数没法按「区间套通电」解释——
坐实了「必须先收敛口径再合闸」这个次序。

### ③ 卡口②有没有被顺带解开 —— **没有。预期证实。**

`CHAINCERT_ADVANCE=0`，**双臂均为 0**。`chain_cert::ChainCertificateBook::advance`
（`chain_cert` 的生产推进入口）在门开门关两种情形下都一次未被调用。

静态面同向：`classifier/mod.rs` 里全部 `chain_cert::` 使用点（行 4000+）位于
`#[cfg(test)] mod tests`（该 `mod tests` 起于 `classifier/mod.rs:2012`）之内；
crate 内其余 `chain_cert::` 引用全在 `src/bin/p127_skip_impact.rs`、
`p129_44ke_strict_skip.rs`、`p123_fast_replay.rs`、`issue550_event_battery.rs`
四个诊断 bin 中——都不在 `theta_backtest` 链上。

**两个卡口相互独立，实测确认。#529 仍堵在卡口②。**

---

## 3. 一句话总判

> **通电了，但通的不是那四条线——门开之后 `admission.rs` 的门控分支和 `nest.rs` 的 `N^δ`
> 确实活了（跑通、无 panic、订单腰斩一半），可真正在做裁决的是 L2 旧臂 Xzd（52 个候选里 51 个
> 走 NoChain 回退、typed 真链索引造出来是空的、真链净贡献 1 个 admit），
> `nest_lifecycle.rs`（#597）和 `chain_cert`（#529）则零触达；
> 而 ⑥ 段 Nautilus 生产引擎根本不查这个门，读数逐位不变——
> 所以「一类买卖点恒 0」「depth>0 恒 0」这两条门开后依然为 0，是结构性没有通路，不是被压住了。**

---

## 4. 未测项（090 照实：以下一律写「没测出来」，不写成「不受影响」）

1. **`NDELTA_CALL=39` 的调用方归属** —— **没测出来**。只按 `xzd_fallback=37` 的数量吻合做了推断，
   未在 `build_gate_certificate` 与 typed 索引装配处分桶插桩反证。
2. **`chain_pass=1` 与 `indexed=0` 的表面张力** —— **没测出来**。`NEST_GATE_INDEX` 是否为末次重建
   的快照统计（而非累计值）未确认；若为快照，则「链过 1 例但索引空」不构成矛盾，但本轮无实测判据。
3. **⑤ 段亏损收窄（−0.1025 → −0.0322）是不是 alpha** —— **没测出来**。单标的单窗，
   且 buy&hold=+0.0385 仍跑赢两臂；订单数减半带来的方差收缩与真实边际不可分离，须 L3。
4. **⑥ 段为什么不查门** —— 已实测「不查」（`GATE_ON` 无增量），但**没测出来**其成因是设计有意
   （nautilus 独立决策层）还是接线遗漏；未读 `nautilus::backtest_engine` 的准入路径。
5. **门开后的逐笔决策是否与门关 bit-exact 之外还有 Classification 层差异** —— **没测出来**。
   `LVLPROJ_DERIVED=1` 说明分类配置被改了（层投影启用），但未对拍两臂的 `Classification` 逐字段。
6. **`NEST_GATE_EXIT_CAND total=12 admitted=2`（出场侧 χ^{σ_p} 消费）** —— 实测非零，
   但**没测出来**这 10 次出场拒绝对最终 PnL 的具体贡献。
7. **其余七品种 / 其余窗口** —— **没测**（⑥ 段硬门 BTC only；本票口径为单窗对照）。
8. **`NEST_CERT_GATE_OVERRIDE` 备用入口** —— **没用上**（env 直接生效，无需备用）。
9. **`cargo test` / Lean↔Rust 断言** —— **没跑**（越界，本票只做双臂回测对照）。

---

## 5. 临时插桩清单（**全部只存在于 `/tmp/nc-main-796`，未提交；主仓与 worktree 零改动**）

新增 `pub mod probe796`（新文件 `rust/src/theta_v0/probe796.rs`，在 `theta_v0/mod.rs` 声明）——
**28 个 `AtomicU64` + 一个 `dump(tag)` 打印函数**。以下 **16 处**各插入一行计数，**纯计数、零逻辑改动**：

| # | 文件 | 位置 | 计数器 |
|---|---|---|---|
| 1 | `backtest/admission.rs` | `nest_cert_gate_enabled()` 返回值处 | `GATE_ON` / `GATE_OFF` |
| 2 | `backtest/admission.rs` | `chain_driven_level_projection` 两臂 | `LVLPROJ_DERIVED` / `LVLPROJ_PASSTHRU` |
| 3 | `backtest/admission.rs` | `NestChainGate::new` 首行 | `CHAINGATE_NEW` |
| 4 | `backtest/admission.rs` | `NestChainGate::sync_index` 首行 | `CHAINGATE_SYNC_INDEX` |
| 5 | `backtest/admission.rs` | `NestChainGate::chain_lookup` 首行 | `LOOKUP_CALL` |
| 6 | `backtest/admission.rs` | `admit` 内 `nest_gate_admit(...)` 调用前 | `OLDARM_CALL` |
| 7 | `backtest/fill.rs` | `gate.admit(...)` 调用后（:4912 expect 下游） | `ADMIT_CALL` / `ADMIT_TRUE` / `ADMIT_FALSE` |
| 8 | `classifier/nest.rs` | `n_delta()` 返回值处 | `NDELTA_CALL` / `NDELTA_TRUE` |
| 9 | `classifier/nest_index.rs` | `build_nest_certificate_index` 首行 | `NESTIDX_BUILD` |
| 10 | `classifier/nest_lifecycle.rs` | `advance` 首行 | `LIFECYCLE_ADVANCE` |
| 11 | `classifier/nest_lifecycle.rs` | `provide_pan_live_windows` 首行 | `LIFECYCLE_PROVIDE_PAN` |
| 12 | `classifier/nest_lifecycle.rs` | `provide_active_pan_live_windows` 首行 | `LIFECYCLE_PROVIDE_ACTIVE` |
| 13 | `classifier/chain_cert/mod.rs` | `ChainCertificateBook::advance` 首行 | `CHAINCERT_ADVANCE` |
| 14 | `strategy/mod.rs` | `build_decision` 内 `stop_in` 之后（按 `point.bits` 六位分型） | `DEC_BUILD` / `DEC_B1..B3` / `DEC_S1..S3` |
| 15 | `strategy/mod.rs` | `recognize_nested` 首行 + `match child` 两臂 | `RECOG_NESTED` / `RECOG_CHILD_OK` / `RECOG_CHILD_FALLBACK` |
| 16 | `strategy/coverage/compose.rs` | `pi_theta_step_traced_with_risk_seeds` 首行 | `PI_STEP` |
| — | `bin/theta_backtest.rs` | ⑤ 段之后、⑥ 段之后各一次 `probe796::dump(tag)` | 读数落点 |

**扰动验证**：臂 A（门关）插桩后 `strat_return=−0.1025`、订单 3136、成交 2482、
⑥ 段 orders 2754 / positions 177 / `PnL%=−23.06` —— 与 #792 基线逐位相同。

**处置**：`/tmp/nc-main-796` 为一次性快照，用完即弃；主仓 `git status` 与本 worktree
除本报告外零改动。

---

*本报告基于 `/tmp/nc-main-796`（`git archive main` 解包快照）的一次性临时插桩双臂实测。
只测不改，跑崩不修——本轮**没跑崩**。*
