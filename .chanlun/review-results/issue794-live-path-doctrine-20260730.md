# #794 回测链口径归属普查：那个 −23% 是用哪套教义算出来的

- 日期：2026-07-30；票据：[#794](https://github.com/xy7365527-lang/NewChanlun/issues/794)（parent map [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787)）
- 性质：只读勘察 + **临时插桩实测**（插桩清单见 §6，全部只存在于 `/tmp` 快照，未提交、未进主仓）。
- **勘察基准树**：`git archive main` 解包快照 `/tmp/nc-main-794`。本 worktree 的 HEAD 是
  `19b4015927`（2026-04-20），彼时 `rust/`、`formal/`、`analysis/` 尚未入仓，故全程在 main 快照上勘察。
  数据经 symlink 接入（`analysis/data_cache/btc_1m_full.json` → 主仓同名文件，不复制 314MB）。
  **主仓与本 worktree 均零改动**（构建、插桩、运行全部在 `/tmp/nc-main-794`）。
- 对照表（直接引用不重做）：[#789](https://github.com/xy7365527-lang/NewChanlun/issues/789) 报告
  `.chanlun/review-results/issue789-multi-impl-census-20260730.md`（556 行 / 40 套口径）。
  本报告的「#789 编号」列一律指该报告 §2 的「概念 → 表内行号」，如「2.3-#2」= §2.3 中枢表第 2 行。

---

## 0. 复现基线（先证读数是真的）

```
cd /tmp/nc-main-794/rust
cargo build --release --features backtest_bin --bin theta_backtest
./target/release/theta_backtest BTC 2024-01-01 2024-01-07
```

实测（本机 2.6 秒跑完，与 [#792](https://github.com/xy7365527-lang/NewChanlun/issues/792) 读数逐位一致）：

| 段 | 引擎 | 订单 | 持仓 | 收益 | 胜率 |
|---|---|---|---|---|---|
| ⑤ in-crate | `run_theta_v0_pi` + 内部 fill 模拟 | 3136 | — | `strat_return −0.1025` | 0.2752 |
| ⑥ 真 Nautilus | `BacktestEngine` + `ThetaStrategy` | 2754 | 177 | **`PnL% −23.06`** | 0.01 |

**加插桩后重跑，两个数字逐位不变**（−0.1025 / −23.06）——证明插桩是纯计数、未扰动判据。

---

## 1. 轴一：教义口径归属（七概念 × 五列）

**先说这条链的骨架**（实测确认，非读注释）：

```
bin/theta_backtest.rs
 ├─⑤ backtest::runner::run_theta_v0_pi
 │    ├─ backtest::incremental::IncrementalClassifier::classify_at(i)   ← 逐 bar 增量
 │    │    └─ classifier::classify_with_tower_incremental               ← 分类层（七概念全在这里）
 │    │         └─ classifier::recursive_tower::compose_level_resume    ← ★中枢分叉点
 │    └─ backtest::fill::pi_theta_fill_loop
 │         └─ strategy::coverage::pi_theta_step_traced_with_risk_seeds  ← ⑤ 的决策层（实测 10080 次）
 └─⑥ nautilus::backtest_engine::run_theta_backtest
      └─ nautilus::strategy::ThetaCore::plan_for_bar
           ├─ classifier::streaming::OwnedIncrementalClassifier::append_bar ← 同一分类层
           ├─ strategy::recognize_nested                                    ← ⑥ 的决策层（实测 10080 次）
           └─ strategy::plan_orders
```

**关键实测（插桩计数，⑥ 段列为「⑥后累计 − ⑤后累计」的增量）**：

```
[⑤ 后] center_L0_complete=9700  center_Lge1_geometric=8956  max_level_idx=2
        sig_L0=75 sig_levelN=57 sig_second=132
        nest_gate_on=0 nest_gate_off=1
        div_confirm=31 div_true=24  second_struct=548
        pi_theta_step=10080  recognize_nested=0
[⑥ 增量] center_L0_complete=9700  center_Lge1_geometric=8956  max_level_idx=2
        sig_L0=75 sig_levelN=57 sig_second=132
        nest_gate_on=0 nest_gate_off=0
        div_confirm=31 div_true=24  second_struct=548
        pi_theta_step=0      recognize_nested=10080
        build_decision 分型：buy1=0 buy2=9498 buy3=59890 / sell1=0 sell2=11531 sell3=36775
```

### 轴一表

| 概念 | 实际执行到的实现（文件:行 + 函数） | 对应 #789 口径 | 有没有被裁定过 | 同一概念的其余口径在哪 |
|---|---|---|---|---|
| **笔** | `theta_v0/parser/{inclusion,fractal,stroke}.rs`，经 `parser/mod.rs:193 ParseLayerIncr::append` 增量驱动；间隔判据 `parser/stroke.rs:60-63` `b.source_index − a.source_index > 3` | **2.1-#4**（只有新笔、旧笔禁用；不回写、`collapse_consecutive` 预坍缩） | **未经裁定**。#789 S-4 记「至少 3 根」在仓内被算成三种量，无裁定票；且 theta_v0 这套与族 I **零对拍**（#789 6.1-8） | 族 I（`src/newchan/a_stroke.py` ≡ `rust/src/stroke.rs`）在**别的区，本链跑不到**（实测：`theta_v0/` 对 `crate::stroke/fractal/bi_engine` 零 import）；Lean `IsNewStroke` 全仓零引用 |
| **线段** | `theta_v0/parser/segment.rs:391 divide_segments_with_tail` + `feature_seq.rs` + `second_kind.rs`（增量态 `IncrSegments`，`segment.rs:480`） | **2.2-#4** | **部分已裁**：相切边界 `<=` 经 #246/#277/#288 一次裁定、四处同批统一（#789 §5 正面样板）。第二特征序列扫描窗口取 ∞（显式拒绝 Python 的 50）**未经裁定** | **同文件的静态臂 `segment.rs:281 analyze_termination`＝编译进树、生产零调用**（实测 grep：调用点只在 `#[cfg(test)]` 与 `second_kind.rs` 的注释引用）——本票要区分的「编译了但跑不到」在线段这里有一个现成实例；`a_segment_v0/v1.py`、`rust/src/segment.rs`、Lean `segmentsOf` 在别的区 |
| **中枢** ★ | **同一条链上两套，都跑到**：<br>① L0 ⟶ `classifier/center.rs:～ center_from_segments`（**含方向交替**）<br>② L≥1 ⟶ `classifier/center.rs:252 center_from_window`（**不看方向，纯几何**）<br>分叉点 = `classifier/recursive_tower.rs:861 compose_level_resume` 的 `let build = if is_l0 {…} else {…}` | **2.3-#2** 那一行内部的两半（= #789 教义分歧 **T-3**） | **只裁了一半**：`ZD<ZG` 严格化经 **#321**（2026-07-26 用户裁决）；**L0 与 L≥1 用不同方向判据这件事，全仓无任何裁定票**（#789 T-3 明记为纯定义分歧） | `rust/src/{zhongshu,level}.rs`、`a_center_v0.py`（首尾两段口径）、`a_ph_zhongshu.py`、Lean `CenterConstruction`（`≤` 弱口径）全在**别的区**；`theta_v0/classifier/ref_v1.rs` 在编译树内、**生产零调用**（源码自陈） |
| **走势类型** | `classifier/decompose.rs`（走势 = 中枢关系链的 maximal 等标签 run；同向 = **外缘分离** `next.dd > prev.gg`），经 `classifier/mod.rs classify_level → decompose` | **2.4-#2** | **未经裁定**（#789 M-1/M-2 两条分歧均无裁定票） | `rust/src/moves.rs` ≡ `a_move_v1.py`（趋势 ⟺ ≥2 中枢、**核心分离**）、`recursive_t/trend.rs`、Lean `TrendCompleteClassification` 在别的区；`a_xiaozhuan_da.py`（小转大）**本链全无对应概念** |
| **买卖点** | 判据出自 `classifier/signal.rs`（`extract_first_third_resume`，L0 与 L≥1 走**同一函数**）+ `classifier/rmove_compose.rs find_second_type_structure`（二类，实测 548 次/段） | **2.5-#3**（signal.rs 真生产者）+ **2.5-#4**（二类下钻） | **裁了两项**：三类点回抽用 ZG/ZD 严格 `>`（#789 认定全仓一致、可直接当既定）；L≥1 一/三类生成经 **codex-t1 裁 A**（`.chanlun/genealogy/settled/683-…-codex-t1-decision-a.md`，实装 #123 仍 in_progress）。**一类是否含背驰合取项（#789 B-1）未经裁定** | **`classifier/bsp.rs`（2.5-#2，bit-vector 口径）不在本链的判据位上**——它只提供 `BspPoint`/`OwnerRef` 类型与 `bsp_at` 查询，判据由 `signal.rs` 出；`rust/src/buysellpoint.rs`、`a_buysellpoint_v1.py`、Lean `BspClassification`、`recursive_t/divergence.rs` 在别的区 |
| **区间套** ★ | **本链一次都没有执行。** `classifier/nest.rs` 的 N^δ 证书门由 env `THETA_NEST_CERT_GATE`（`backtest/admission.rs:62 nest_cert_gate_enabled`）门控，CLI 不设该变量 ⟹ 实测 `nest_gate_on=0 / nest_gate_off=1`，`backtest/fill.rs:4489 NestChainGate` **从不构造**，整段跳过 | （本链无归属） | ADR-0005 管的是 nest 的 import 依赖方向，不裁判据；`Cand^δ_ℓ` 判据式本身 #789 记为**教义空洞**（源码自标 [需人工确认]） | 全部在旁支或别的区：`strategy/nest.rs chi_bool` 只在 `run_theta_v0_pi_chi` 入口下游（CLI 走的是 `run_theta_v0_pi`）；`econ_positive::build_nest_certificate` 只在 wverify/econ 探针里；`cand_sub.rs`+`chain_cert/`、`interval_necessity.rs` 编译进树零消费；`a_nested_divergence.py`、`recursive_t`、两套 Lean 骨架在别的区 |
| **背驰** | `classifier/divergence.rs`，调用点 `classifier/signal.rs:461` `divergence::confirm_divergence(gauge, macd_c_lt_a, force)`；**gauge = `ThetaConfig::default().divergence_gauge` = `DivergenceGauge::MacdArea`（G1：`Area(C) < Area(A)`，Σ\|hist\| 段面积）**。实测 31 次调用 / 24 次判真（每段） | **2.7-#1** 的第一档（四档 gauge 的冻结默认档） | **未经裁定**：四档 gauge 是「三套预注册 Θ 分别 OOS、不许先看结果再选」的设计，MacdArea 只是冻结默认；且同文件 `divergence.rs:256-269` **源码自陈「与缠师第 17 课原文相悖」** | 另四套力度（三维 OR / 振幅×时长 / 中枢嵌套深度 / persistence Wasserstein）全在别的区；同一 gauge 的另三档（ThetaDom/Conjunction/ThetaLex）**编译进树、本次配置下跑不到** |

---

## 2. ★ 本票的核心答案：中枢的口径分叉，两侧都跑到了

票面「特别要查」的那条，实测结论是**肯定的，且量级相当**：

| 分支 | 判据 | ⑤ 段调用次数 | ⑥ 段调用次数 |
|---|---|---|---|
| `is_l0 == true` → `center_from_segments` | 全三段 + **方向交替**（完整判据） | 9700 | 9700 |
| `is_l0 == false` → `center_from_window` | 全三段，**方向维度完全不看**（几何判据） | 8956 | 8956 |

`max_level_idx = 2` ⟹ 这一周 BTC 数据上塔真的长到了 **L0 / L1 / L2 三层**（`l_max=6` 未触顶，
自然终止）。也就是说：**L0 的中枢和 L1/L2 的中枢，在同一座塔里、同一次运行里，是按两个不同定义
造出来的，而上层的中枢又是拿下层的中枢当输入造的。** 这不是「两个模块各用各的」，是同一条链
内部的口径分叉，直接进入 −23.06% 那个数。

而这处分叉：**没有任何裁定票**。#321 裁的是 `ZD<ZG` 严格化（两个分支都遵守），不是方向判据。

---

## 3. 轴二：多处实现里，哪一处在链上

四区代号同 #789：**Py** = `src/newchan/`；**顶层** = `rust/src/*.rs` 散件；**Θ** = `rust/src/theta_v0/`；**Lean** = `formal/Origin/`。

| 概念 | Py `src/newchan` | 顶层 `rust/src/*.rs` | **Θ `theta_v0/`** | Lean `formal/` | 对拍状态 |
|---|---|---|---|---|---|
| 笔 | 旁 | 旁 | **★在链上**（`parser/stroke.rs`） | 旁 | Py↔顶层 CI 常跑；**Θ↔任何一方零对拍**（#789 6.1-8） |
| 线段 | 旁 | 旁 | **★在链上**（`parser/segment.rs::divide_segments_with_tail`） | 旁 | Py↔顶层 CI 常跑；Θ↔Lean 仅原语级（`gap_overlap` fixture），整体划分未对拍 |
| 中枢 | 旁 | 旁 | **★在链上（两处：L0 完整 + L≥1 几何）** | 旁 | Py↔顶层 CI 常跑；Θ↔Lean 有 `theta_v0_center_parity.rs`，**CI 从不执行 `cargo test`**（#789 3.1） |
| 走势类型 | 旁 | 旁 | **★在链上**（`classifier/decompose.rs`） | 旁 | Py↔顶层 CI 常跑；Θ↔Lean **未对拍**（fixture 无走势类型字段） |
| 买卖点 | 旁 | 旁 | **★在链上**（`classifier/signal.rs` + `rmove_compose.rs`） | 旁 | Py↔顶层 CI 常跑；Θ↔Lean 29 个 parity test **CI 不执行**；卖侧无对拍对象 |
| 区间套 | 旁 | （无） | **四处全在旁支，链上零执行**（env 门关） | 旁 | **全仓零对拍**（#789 6.1-1）。本链既不吃也测不出 |
| 背驰 | 旁 | 旁 | **★在链上**（`classifier/divergence.rs`，gauge=MacdArea） | 旁 | Py↔顶层 CI 常跑（fallback 口径）；Θ 的 MacdArea 与另四套力度**两两零对拍** |

**轴二最重的一条**（实测，不是推测）：`theta_v0/` 对 `rust/src/*.rs` 顶层散件与 `recursive_t/spiral/fugue_v3`
**零 crate 内依赖**——全目录 grep `crate::{stroke,fractal,segment,zhongshu,level,moves,buysellpoint,divergence}::`
零命中，只有 `mod.rs` 的文档注释提到它们。

⟹ **CI 里唯一持续跑的那套对拍（九组 `test_rust_*_equivalence.py`，Py↔顶层散件 bit-exact），
守的全部是这条链之外的代码。** 这条链上的七个概念，一个都不在那套对拍的覆盖范围里。

---

## 4. 一句话总判

> **那个 −23.06% 是用「theta_v0 单一区、逐 bar 增量」的一套口径算出来的——七个概念全部只吃
> `theta_v0/`，其余三区（Python / Rust 顶层散件 / Lean）一处都没进这条链；而它内部自带
> 一处口径分叉：同一座递归塔上，L0 的中枢按「含方向交替」的完整判据造（实测 9700 次），
> L1/L2 的中枢按「完全不看方向」的几何判据造（实测 8956 次），上层又拿下层的产物当输入，
> 两套定义混在一个数里，且这处分叉没有任何裁定票。**
>
> 同时还要说清它**不是**用什么算的：五词里分歧最深的「区间套」被一个 env 开关
> （`THETA_NEST_CERT_GATE`）关在门外，本链**一次都没跑**；「背驰」跑了（31 次调用、24 次判真），
> 但一类买卖点决策数**恒为 0**（buy1=0 / sell1=0），背驰的判定结果没有转化成任何一笔交易——
> 真正驱动这 −23% 的只有二类与三类买卖点。

### 4b. 顺手解掉 map #787 的一个待办：⑤/⑥ 数值分叉不是「两套 fill 口径」

#787「Not yet specified」里登记的「同一次运行同一份数据，in-crate −10.25%/胜率 .2752 vs
nautilus −23.06%/胜率 .01，仓内零解释」——实测归因如下，**不是 fill 差异，是两套决策层**：

| | ⑤ in-crate | ⑥ 真 Nautilus |
|---|---|---|
| 分类层 | `IncrementalClassifier` | `OwnedIncrementalClassifier` — **同一套分类判据**（计数逐项相同） |
| **决策层** | `coverage::pi_theta_step_traced_with_risk_seeds`（π_Θ 七链 / LexArgmin 𝒦_Θ 单一决策出口）**实测 10080 次** | `strategy::recognize_nested` + `plan_orders`（声部决策）**实测 10080 次** |
| 交叉调用 | `recognize_nested = 0` | `pi_theta_step` 增量 `= 0` |
| 撮合 | in-crate fill 模拟 | Nautilus venue |

两条链在决策层**互相零调用**。所以那 13 个百分点的差，首要嫌疑是决策层不同，
而非撮合口径不同。另一条实测线索：⑥ 段 `recog_child_depth_gt0 = 0`——
`recognize_nested` 的 depth>0 嵌套子声部（买卖点定律一「次级别第一类」真下钻，#789 2.5-#4）
**一次都没产出决策**，2754 笔订单全部来自 depth=0 根决策，且 Nautilus 报 `Long Ratio: 1`（全多）。

---

## 5. 未测项（090 照实：以下一律写「没测出来」，不写成「一致」/「不受影响」）

1. **⑤ 与 ⑥ 的 `Classification` 是不是真 bit-exact——没测。**
   我只测到两段的插桩计数逐项相同（9700 / 8956 / 75 / 57 / 132 / 31 / 24 / 548 各自等值）。
   **计数相同不等于逐字段相同。** 仓内有 crate 内单测（`recognize_current_integration_bit_exact_via_plan_for_bar`、
   `owned_bit_exact_*`）断言这件事，但它们跑在合成数据上，且 CI 不跑 `cargo test`。跨 ⑤/⑥ 的
   真实行情逐字段对拍，本次没做。
2. **L≥1 若改用完整判据，−23% 会变成多少——没测。** 需要改生产代码，越出「只测不改」边界。
3. **`THETA_NEST_CERT_GATE=1` 打开后的读数——没测。** 本票问的是「这条链跑到的是哪一套」，
   默认配置下答案是「没跑到」；开门后的行为是另一个问题。
4. **Lean↔Rust 的 34 个 parity 断言——没跑。** 需 `cargo test`，CI 本来也不跑（#789 3.1）。
5. **决策计数的绝对值不可当信号条数读。** `recognize_nested` 每 bar 对全量 γ 重扫，
   `build_decision` 的 buy2=9498 / buy3=59890 等是 10080 个 bar 的**累计重复计数**。
   可靠的只有两件事：**哪些恒为 0**（buy1/sell1/depth>0 子声部），以及**相对量级**。
6. **其余七个品种——没测。** ⑥ 段有 `require_btc_symbol` 硬门（instrument 硬编码 BTCUSDT.BINANCE），
   非 BTC 直接 fail-fast。
7. **`analyze_termination` 静态臂「生产零调用」是 grep 结论，未经插桩反证。** 与 #789 独立结论一致，
   但本次没在它里面埋计数器。
8. **第 5 段的 `strict_nest_sidecar` 观测面——env `THETA_STRICT_NEST_SIDECAR` 未设，本次未启用。**

---

## 6. 临时插桩清单（**全部只存在于 `/tmp/nc-main-794`，未提交、主仓与 worktree 零改动**）

新增一个 `pub mod probe794`（`rust/src/theta_v0/classifier/mod.rs`）——12 个 `AtomicU64` 计数器
+ 一个 `dump(tag)` 打印函数。以下 8 处各插入一行 `probe794::bump(...)`，**纯计数、零逻辑改动**：

| # | 文件 | 位置 | 插的是什么 |
|---|---|---|---|
| 1 | `classifier/recursive_tower.rs:861` | `compose_level_resume` 的 `let build = if is_l0` 两臂 | `L0_COMPLETE` / `LGE1_GEOM` |
| 2 | `classifier/recursive_tower.rs:304` | `compose_level`（全量臂）同一分叉 | 同上（用于反证全量臂未被调用） |
| 3 | `classifier/mod.rs` | 增量级别循环 `let is_l0 = level_idx == 0;` 后 | `MAX_LEVEL`（fetch_max） |
| 4 | `classifier/mod.rs` | `07a_extract_signals_l0` / `07a_extract_first_third_ln` / `07b_extract_second` 三个 stage 标签处 | `SIG_L0` / `SIG_LN` / `SIG_2ND` |
| 5 | `classifier/signal.rs:461` | `confirm_divergence` 调用后 | `DIV_CONFIRM` / `DIV_TRUE` |
| 6 | `classifier/rmove_compose.rs` | `find_second_type_structure` 函数体首行 | `SECOND_STRUCT` |
| 7 | `backtest/fill.rs:4473` | `nest_cert_gate_enabled()` 判定处 | `NEST_GATE_ON` / `NEST_GATE_OFF` |
| 8 | `strategy/mod.rs` | `recognize_nested` 首行、`build_decision` 首行（按 `point.bits` 六位分型） | `RECOG_NESTED` / `RECOG_CHILD` / `DEC_B1..S3` |
| 9 | `strategy/coverage/compose.rs` | `pi_theta_step_traced` / `pi_theta_step_traced_with_risk_seeds` 函数体首行 | `PI_STEP` |
| 10 | `bin/theta_backtest.rs` | ⑤ 段之后、⑥ 段之后各一次 `probe794::dump(tag)` | 读数落点 |

**扰动验证**：插桩前后两次运行的 `strat_return = −0.1025` 与 `PnL% = −23.06` 逐位相同。

**处置**：`/tmp/nc-main-794` 为一次性快照，用完即弃；主仓 `git status` 与本 worktree 均无这些改动。

---

*本报告基于 `git archive main` 解包快照 `/tmp/nc-main-794` 的只读勘察 + 一次性临时插桩实测。
worktree 内除本文件外零改动。*
