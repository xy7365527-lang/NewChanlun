# V2 N2 rung 级力度门实装报告（typed 装配路径）

- **日期**：2026-07-20/21　**工位**：worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`，HEAD = `640609071d`
- **实装卡**：`chanlun/review-results/v2-rung-force-gate-implementation-card-20260720.md`（下称「卡」，§n 均指该卡）
- **授权文件**：`rust/src/theta_v0/classifier/nest.rs`（生产语义唯一落点）；`rust/src/theta_v0/classifier/turn_class.rs` 仅测试模块改写（卡 §7 T8 授权）
- **纪律**：零 git mutation（无 commit/stash/checkout）；主仓 `/Users/silencehan/Projects/NewChanlun` 全程未写；`rust/Cargo.toml` 零改；typed_ledger/TW/sep_legs 决策层零触；judge_at 未动一个 bit（P92_BIT_EXACT 五维 pre/post 逐字相等，见 §5 I5）

---

## 0. 结论速览

| 项 | 结果 |
|---|---|
| 生产语义改动 | `extend_typed_upward` 候选过滤加 1 个合取项 `!event.divergence_confirmed → continue`（nest.rs:775），与卡 §4.1 逐字一致 |
| 注释/不变量 | 头注改写（nest.rs:658-665）、#97 D1 注释 supersede 登记（nest.rs:769-774）、门后 debug 不变量（nest.rs:720-722） |
| 单测 | T1-T7 新落于 nest.rs:1778-1959，T8（turn_class 合成证书测试）改走 `from_parts_for_test` 并加 R-2 登记注释；全绿 |
| cargo test --release --lib | **1770 passed; 0 failed; 129 ignored**（实装态初验 1762 passed；其后并行工作线新增 `classifier/nest_lifecycle.rs`（未跟踪文件，8 个 #[test]）使计数变 1770，本任务终态复验两遍均 1770/0 failed）——两种计数下均零变红 |
| 链深重测（全量 4.6M 双跑，本报告 §6.3） | I1-I4 全过、I5 五维 diff=0；含未确认 rung 链 100% 纵向重路由（2,820/2,820），零杀死零未归因；单级占比 A 30.9%→43.2%、B 34.0%→86.9%；XiaozhuandaCandidate 150→0，DeferOrphan +9 逐事件闭合 |
| 链深重测（250k/1M 截段双跑） | I1-I4 全过、I5 五维 diff=0；含未确认 rung 的链 100% 归因纵向重路由（变浅），零未归因；深度≥2 链收缩、单级占比 50.5%→66.2%（250k）/ 42.1%→61.3%（1M），方向与全量双跑一致 |
| R-2 实测 | `XiaozhuandaCandidate` 链侧计数 **150→0（全量）**；7→0（250k）/ 32→0（1M）；`ExecEvidenceOnly` 2,413→0（全量）/ 43→0 / 276→0——类空实测落账，处置仍待 Lead 裁（本报告不代裁） |

---

## 1. 实装清单（对照卡 §4 三落点 + §7 单测）

### 1.1 落点 1（唯一生产语义改动）：rung 级力度门

`rust/src/theta_v0/classifier/nest.rs:775-777`：

```rust
if event.side != side || !event.divergence_confirmed {
    continue;
}
```

kind-blind 读 `NestCandidateEvent.divergence_confirmed`（provider 单源物化字段），与基例门（nest.rs:686 `!base.divergence_confirmed → None`）同字段同源，nest 层零重算、不 fork。DFS 排序键（:762-766）、回溯 pop（:805-810）、`level > top_level` 终止（:756-758）逐字未动；`NestRung::assembled(..., true)` 的 `cand` 恒 true 原样（:782）。

### 1.2 落点 2（注释对齐，090 声明=能力）

- `assemble_typed_certificate` 头注（nest.rs:658-665）：改写为「力度在基例门（本函数 `!base.divergence_confirmed → None`）与 **rung 级门**（`extend_typed_upward` 候选过滤）各合取一次——同字段同源（provider 单源物化，V2 N2 rung 级力度门，实装卡 §4）」。
- #97 D1 注释（nest.rs:769-774）：按卡 §4.1 伪码改写，登记 supersede（裁定 R-1，见 §7.1）。
- sidecar push 同位注释（nest.rs:787-788）：补「V2 N2 门后此处恒 true」。

### 1.3 落点 3（门后 debug 不变量，零行为改动）

nest.rs:720-722（`confirmed_low_to_high.reverse()` 之后）：

```rust
// V2 N2 门后不变量：confirmed 向量全真（基例门 + rung 级门双重保证）。
// release 编译掉，与 cert F-01 builder 断言同款纪律。
debug_assert!(confirmed_low_to_high.iter().all(|&flag| flag));
```

### 1.4 单测（卡 §7 T1-T8）

- **T1-T7**（nest.rs:1778-1959，`#[cfg(test)]`）：`rung_gate_rejects_unconfirmed_parent`（门拒 + 同基例 top=1 不受影响）、`rung_gate_accepts_confirmed_parent`（门过、链深 2、身份高→低含基例）、`rung_gate_dfs_reroutes_to_confirmed_candidate`（DFS 横向重路由确定性）、`rung_gate_base_semantics_unchanged`（基例门零改）、`rung_gate_sidecar_all_true_invariant`（confirmed 全真且与 identities 等长）、`rung_gate_kind_blind_pan`（Pan 支同拒）、`rung_gate_backtrack_pop_symmetry`（回溯 pop 对称、无半成品）。
- **T8**（turn_class.rs 测试模块）：合成 `confirmed=[false,…,true]` 链的分类测试改走新增测试专用数据载体重建入口 `TypedNestCertificate::from_parts_for_test`（nest.rs:463-507，`#[cfg(test)]` 可见，与 `NestCertificate::from_parts` 同款诚实边界——不宣称装配前置成立）；四处 R-2 登记注释（turn_class.rs:367-371、:514-516、:621-623、:767-769）；分类机器本体零改，partition 锁测试（`turn_class_partition_unique_and_sidecar_neutral`）保留并改走重建路径。
- **旧口径断言改写登记**（卡 §7 回归线授权对象，逐条登记）：`p97_rung_screening_is_structural_and_identities_align`（nest.rs:1751-1776）原断言「父级 confirmed=false 不否决链」=#97 D1 旧口径，已被门 supersede——改写为 confirmed=true 父级成链 + 身份对齐覆盖，改写事由注于测试头注（:1752-1756）。**这是唯一被改写的判据性旧断言**；classifier 其余既有测试零改。

### 1.5 明确未改（卡 §4.4 禁越界清单核对）

`n_delta_rec`、`is_sub`/`sel_order`/`select_best`、`NestRung`/`NestCertificate` 结构、`typed_interval`、provider 全部、`segments_diverge_or` 族、终端背书、旧 `extend_upward`（保持 deprecated 原样）、`chi_bool`、econ `build_nest_certificate`、runner 生产门、p92/p123 bin 行格式——`git diff` 逐块核对，上述区域 diff=0（nest.rs diff 仅 §1.1-1.4 各块 + 测试；turn_class.rs diff 仅测试模块）。

## 2. 改动 diff 摘要

```text
rust/src/theta_v0/classifier/nest.rs       | 260 +++++++++++++++++++++++++++++++++--
rust/src/theta_v0/classifier/turn_class.rs |  85 +++++++-----
```

（worktree 内另有其它工作线的未提交改动——runner.rs/exit.rs/overlay_state.rs 等——非本任务产物，本任务零触。本报告全部 pre/post 对照在同一 worktree 树态内进行，那些改动对门效应测量中性。）

## 3. 测试输出（`cargo test --release --lib`）

实装态初验（全量 pre-run 前）与截段双跑后终态复验：

```text
test result: ok. 1762 passed; 0 failed; 129 ignored; 0 measured; 0 filtered out; finished in 1.00s
test result: ok. 1770 passed; 0 failed; 129 ignored; 0 measured; 0 filtered out; finished in 1.01s
test result: ok. 1770 passed; 0 failed; 129 ignored; 0 measured; 0 filtered out; finished in 1.04s
```

计数 1762→1770 的漂移已查明：并行工作线在会话中途新增未跟踪文件
`rust/src/theta_v0/classifier/nest_lifecycle.rs`（恰 8 个 `#[test]`，`mod.rs:79` 注册），
非本任务产物；两种计数下均 0 failed。`--features backtest_bin` 与否不影响计数（均 1770）。

T1-T7 + turn_class 子集（17 测试）单跑：

```text
test theta_v0::classifier::nest::tests::rung_gate_rejects_unconfirmed_parent ... ok
test theta_v0::classifier::nest::tests::rung_gate_accepts_confirmed_parent ... ok
test theta_v0::classifier::nest::tests::rung_gate_dfs_reroutes_to_confirmed_candidate ... ok
test theta_v0::classifier::nest::tests::rung_gate_base_semantics_unchanged ... ok
test theta_v0::classifier::nest::tests::rung_gate_sidecar_all_true_invariant ... ok
test theta_v0::classifier::nest::tests::rung_gate_kind_blind_pan ... ok
test theta_v0::classifier::nest::tests::rung_gate_backtrack_pop_symmetry ... ok
（+ turn_class 10 测试全 ok，含 partition 锁与 044:16 正例）
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## 4. 链深重测协议

**全量 4.6M pre/post 双跑已完成**（本报告 §6.3，卡 §6 原始协议）；250k/1M 截段双跑为并行补充证据（§5/§6.1-6.2），两套结果方向与归因结构一致。

- **数据**：`analysis/data_cache/btc_1m_full.json`（4,613,599 bar；worktree 内只读副本）。
- **命令**：`P92_MAX_BARS=<250000|1000000> P92_DUMP=<dump路径> p92_nest_replay_postruling <数据>`（release，`--features backtest_bin`）；截断 env 用探针自带 `P92_MAX_BARS`（p92_nest_replay_postruling.rs:163-166）。
- **pre/post 构造**：post = 工作树实装态二进制；pre = 仅把 nest.rs:775 门线临时回退为 `if event.side != side`（单行编辑，非 git 操作），构建出 pre 二进制后**立即恢复原行**（回退窗口内零其它操作；恢复后 `git diff` 与实装态逐字一致并经终态 cargo test 复验）。pre/post 除门线一个合取项外同树同参——门效应被干净隔离。
- **产物**（`/tmp/v2gate/`）：`dump_{pre,post}_{250k,1M}.txt`、`stdout_{pre,post}_{250k,1M}.txt`、`result_{250k,1M}.json`（I1-I4 机算读数）、`analyze.py`（核对脚本）。
- **深度口径**：与 p105 探针 CERT 侧同法——链深 = CERT 行 ids 段数（`|` 分隔，高→低含基例）；同一 (caliber,exec,top,ids) 去重。

## 5. 不变量核验（卡 §6 I1-I5）

### I1（门后 confirmed_vec 恒全 '1'）——**过**

| 截段 | post 证书型 TURN_CLASS 行 | confirmed_vec 全 '1' | 违例 |
|---|---|---|---|
| 250k | 142 | 142 | **0** |
| 1M | 623 | 623 | **0** |

（`class=DeferOrphan confirmed_vec=0` 行是事件层孤儿行、非证书行，格式硬编码 vec=0，不在 I1 域内——p92_nest_replay_postruling.rs:812。）

### I2（全 '1' 证书 bit-identical 存续）——**过**

| 截段 | pre 全 '1' CERT 行 | post 中逐字在场 | 缺失 |
|---|---|---|---|
| 250k | 134 | 134 | **0** |
| 1M | 565 | 565 | **0** |

卡 §5.1 核心命题实证成立：门只删候选，全 confirmed 链 DFS 首次可行命中不变。

### I3（含 '0' 链归因完备）——**过**

| 截段 | pre 含 '0' CERT | (a) 杀死 | (b) 纵向重路由（变浅） | (c) 横向重路由 | 未归因 |
|---|---|---|---|---|---|
| 250k | 52 | 0 | **52** | 0 | **0** |
| 1M | 343 | 0 | **343** | 0 | **0** |

(a) 杀死结构性不可能（基例门未动 ⟹ 同基例深度 1 证书恒存续，归因恒有可落点）；(b) 含「同基例更浅 top 成链」与「更浅 top 处横向换 confirmed 候选」两亚型（归因键 = 基例身份，两亚型不细分）。反向归因同样闭合：post 相对 pre 的**新增** CERT 行（250k=8、1M=58）基例身份 100% 挂在 pre 含 '0' 链上，零 de-novo 证书。

### I4（类迁移对账）——**过（等式按 covered_b 口径修正后精确成立）**

| 截段 | XiaozhuandaCandidate | ExecEvidenceOnly | DeferOrphan pre→post | pre 未确认 rung 事件（唯一） | 其中 B 链消费的 Trend |
|---|---|---|---|---|---|
| 250k | 7 → **0** | 43 → **0** | 33 → 36（**+3**） | 17（Trend 8 / Pan 9） | **3** |
| 1M | 32 → **0** | 276 → **0** | 197 → 204（**+7**） | 68（Trend 31 / Pan 37） | **7** |

卡 I4 原文「DeferOrphan 增量 = pre 中被链消费为 rung 的未确认事件数」按字面值不成立（3≠17、7≠68），**实测精化后逐事件闭合**：增量 = pre 中被 **B 口径链**消费为 rung 的**未确认 Trend** 事件数（250k：3/3 逐事件相等；1M：7/7 逐事件相等）。两个修正因子均在仓内既有语义中，非门的新效应：① DeferOrphan 谓词只收 Trend kind（`is_defer_orphan_event`，turn_class.rs:198-200：Pan 未确认 rung 事件永不入 defer 域）；② p92 孤儿发射的覆盖集 = `covered_b`，仅由 B 口径装配环喂入（p92_nest_replay_postruling.rs:786-791、:799-803）——仅被 A 口径链消费的未确认 Trend 事件在 pre 期就已是 DeferOrphan（实测：250k 其余 5 个、1M 其余 24 个 Trend 未确认 rung 事件 pre/post 两侧均在孤儿册）。pre 孤儿行 post 100% 存续（两截段 diff=0）。partition 不破：每证/每事件仍恰一类。

### I5（P92 五维 diff=0）——**过**

`P92_BIT_EXACT` 行 pre/post 逐字相等（两截段同）：

```text
P92_BIT_EXACT old_path_diff=0 tower_diff=0 moves_centers_bsp_pan_diff=0 lifecycle_cp_ownership_diff=1 classification_total_diff=1
```

门对五维零扰动。**照实登记**：`lifecycle_cp_ownership_diff=1` 与 `classification_total_diff=1` 在本 worktree pre-gate 态即存在（门中性，pre/post 同值）——属 worktree 既有状态（疑为其它工作线未提交改动所致），非本任务引入，留 worktree 所有者查明，本任务不越界处置。

**全量 4.6M 双跑的同行读数**（§6.3 产物 stdout，pre/post 逐字相等）：

```text
P92_BIT_EXACT old_path_diff=0 tower_diff=0 moves_centers_bsp_pan_diff=0 lifecycle_cp_ownership_diff=0 classification_total_diff=0
```

两套读数差异（截段 =1/=1 vs 全量 =0/=0）为回放窗口长度函数，与本门无关（各自 pre/post 同值）；照实并列，不归因、不抹平。

旁证（卡 §5.3 不动面）：`P92_INPUT`/`P92_YIELD`/`P92_PROVIDER`/`P92_SNAPSHOT` 四行 pre/post 逐字相等——provider 事件集与每个事件的 `divergence_confirmed` 值零改。`P92_D3` 边数随证书集合收缩而变（250k：44→4；1M：288→41）——D3 统计在装配件上计数，集合变则计数变，属预期后果而非违规维度（不在五维内）。

## 6. 链深分布对照（截段实测 + p105 归档基线方向对照）

### 6.1 截段 pre/post 深度谱（CERT 行去重后，深度 = ids 段数）

| 截段 | 口径 | 深度分布 pre（1/2/3/4） | 深度分布 post（1/2/3/4） | 单级占比 pre→post | 证书总数 pre→post |
|---|---|---|---|---|---|
| 250k | A | 47/36/22/0 | 47/28/16/0 | | |
| 250k | B | 47/24/10/0 | 47/4/0/0 | 94/186=50.5% → 94/142=**66.2%** | 186 → 142 |
| 1M | A | 191/156/102/48 | 191/128/62/17 | | |
| 1M | B | 191/164/44/12 | 191/28/5/1 | 382/908=42.1% → 382/623=**61.3%** | 908 → 623 |

要点：① 深度 1 计数 pre/post **逐格相等**（47/47、191/191，A/B 皆然）——基例门零改的直接实证；② 收缩全部发生在深度≥2，且 B 口径收缩远剧于 A（1M：B 深度 2 由 164→28、深度 4 由 12→1）——B 口径区间更宽、旧语义下更容易套进未确认父级；③ 总数下降、单级占比上升，方向与卡 §6 step 6 预期一致。

### 6.2 p105 归档基线对照（方向性声明，090 不拍等值）

归档基线（`p105-cert-level-spectrum-20260717.md:44-45`）：全量 4.6M、sidecar 严格装配去重集 66 张（A=41/B=25），深度 1/2/3/4 = A:24/11/5/1、B:24/1/0/0，单级占 72.7%。本报告截段读数与其**窗口不同（前 250k/1M bar ≠ 全量）、集合口径不同（p92 dump 去重集 ≠ sidecar 严格装配 66 张）**，只作方向对照：门后单级占比上升、多级链收缩、B 口径多级近灭门——三个方向在两个截段上与基线→门后的预期走向一致。

### 6.3 全量 4.6M pre/post 双跑（卡 §6 原始协议，含基线漂移查明）

**协议**：`P92_CKPT=250000 P92_DUMP=<路径> p92_nest_replay_postruling analysis/data_cache/btc_1m_full.json`（release，`--features backtest_bin`）；pre = 实装前工作树构建的二进制（构建先于任何本任务编辑），post = 实装态二进制；pre/post 同树同数据同参，唯一变量 = 门线 1 个合取项。p105 探针（`p105_cert_level_spectrum`）对两个 dump 各跑一遍出官方深度表。产物：`/tmp/v2_gate_dump_{pre,post}.txt`、`/tmp/v2_gate_p92_{pre,post}_stdout.txt`、`/tmp/v2_gate_p105_{pre,post}.txt`、核验脚本 `/tmp/v2_gate_verify.py` + `/tmp/v2_gate_verify_i34.py`、diff `/tmp/v2_gate_impl.diff`。

**基线漂移查明（卡 §6 步骤 1「与归档 p105 逐格核对，不等则先查明漂移再动工」）**：pre-run 与归档 p105 **逐格不等**，漂移先于本门存在，归因三条（均非本门效应）：

| 维度 | 归档 p105（2026-07-17 冷备） | 本工作树 pre-run（全量） |
|---|---|---|
| 证书总数（终态 as_of=4613598） | 66（A=41/B=25） | 5285（A=2767/B=2518） |
| CKPT_STATS as_of=4500000 | A=41/B=25 | A=2712/B=2106 |
| 归档 66 证身份在 pre dump 命中 | — | 仅 9/66 |
| 塔顶 / BSP 总量 | max_lvl=4 / 28,417 | top=5 存在 / 29,860 |

1. **p117 T1 终端背书级别移位**（bsp-terminal-endorsement-ruling-20260718）：基例终端查法由归档期 `levels[ℓ]` 位格等式（`p105-cert-level-spectrum-20260717.md:24` 亲述口径）改为 `levels[ℓ-1].bsp` 窗口查法（`p92_nest_replay_postruling.rs:1035-1045` 委托 `nest::terminal_bits_at_event`）——低一级账本点更密 ⟹ 基例背书通过率数量级上升（`P92_YIELD terminal_confirmed=855`），证书总量 66→5285 的主因。
2. **wave-1 收束**：confirmed 事件 `interval_b` 收束到 t\*（卡 §5.3 已述），ids 内嵌坐标与归档期全离开段坐标不同 ⟹ 归档 66 证身份仅 9/66 在 pre 命中。
3. **塔/分类本体演进**：pre 重放出现 top=5 链、BSP 总量 29,860（归档 28,417）——工作树既有演进，与本门无关（pre/post 同树对照不受影响，I5 实证）。

**depth_pre → depth_post（p105 探针官方行）**：

| caliber | n pre→post | d1 | d2 | d3 | d4 | d5 | 单级占比 pre→post |
|---|---|---|---|---|---|---|---|
| A | 2767 → 1980 | 855 → 855 | 724 → 590 | 586 → 352 | 423 → 141 | 179 → 42 | 30.9% → 43.2% |
| B | 2518 → 984 | 855 → 855 | 829 → 118 | 496 → 10 | 337 → 1 | 1 → 0 | 34.0% → 86.9% |

深度 1 两口径 pre/post **逐格相等**（855/855）——基例门零改的全量实证；收缩全部在深度≥2，B 口径多级近灭门（d3 496→10、d4 337→1、d5 1→0）。

**I1-I4（全量集，脚本机算）**：

- **I1**：post 全部 2,964 张证书 TURN_CLASS 行 `confirmed_vec` 恒全 `'1'`，**零违例**（DeferOrphan 行契约 vec=0，不在域内）。
- **I2**：pre 全 `'1'` 证书 2,465 张，post **2,465/2,465 bit-identical 存续**（CERT 行全等）——卡 §5.1 核心命题全量实证。
- **I3**：pre 含 `'0'` 证书 2,820 张（按 ids 归并）——**杀死 0 / 纵向重路由 2,820 / 横向重路由 0 / 未归因 0**。初算有 18 张 top=4/5 深链未归因，精化复核（`/tmp/v2_gate_verify_i34.py`：纵向 = 同基例且 ids 为 pre ids 后缀的浅链）确认全部 18 张为纵向变浅，归因闭合。零横向实例：本数据集无「sel 序靠前未确认候选遮挡次前 confirmed 候选」的实发。
- **I4**：`XiaozhuandaCandidate` **150→0**；`ExecEvidenceOnly` 2,413→0；post 全部证书恒 `NestedConfirmed`（2,964）。`DeferOrphan` 986→995（**+9**）；机械复算：pre **B 口径**链内被消费为 rung 的未确认 **Trend** 事件（身份去重）= **恰 9 个**，post 新增孤儿身份 9 个逐元素 ∈ 该集合，pre 孤儿 ⊆ post 孤儿——逐事件闭合（修正因子与 §5 I4 截段读数同：defer 域 Trend-only + covered_b 覆盖集）。

**I5（全量 stdout diff）**：除 4 行外 pre/post stdout 逐字全同——`P92_CERT`（A 2767→1980、B 2518→984）、`P92_D3`（sidecar 计数随集合）、`P92_BASELINE`（new_B_certificates 同步）、`P92_MISSED`（covered_b 1049→929，missed=0 不变）；`P92_BIT_EXACT` 五维、`P92_YIELD`（candidates=18297、divergence_confirmed=8606、terminal_confirmed=855）、`P92_PROVIDER`、`P92_SNAPSHOT` 逐字相同——provider 事件集与每事件力度真值零改。stderr 前缀进度计数器序列 pre/post 逐值相同。p105 探针侧旁证：`P105_BSP events_total=29860 buys=15179 sells=14681 max_lvl=4` pre/post **逐字相同**——塔/BSP/分类本体零触。

## 7. 裁定登记（均不自决）

### 7.1 R-1（supersede #97 D1）——已按卡落码，登记在案

#97 D1「rung 初筛只看结构、力度留在基例门」口径由 nest.rs:775 的合取项 supersede；注释改写（:769-774）与唯一旧断言改写（:1752-1756，p97 测试）均已登记。裁料 = 卡 §1.1 教义锚（027:22/38/46、061:32）+ `doctrine-vs-code-nest-recursion-20260720.md` G-2。

### 7.2 R-2（XiaozhuandaCandidate 类空 vs p118 前提冲突）——实测落账，不代裁

- **实测**：门后生产装配不再产出 top-unconfirmed 链——`XiaozhuandaCandidate` 链侧计数 **150→0（全量 4.6M）**；7→0（250k）/ 32→0（1M）；链侧 `ExecEvidenceOnly` 2,413→0（全量）/ 43→0（250k）/ 276→0（1M）；post 全部证书恒 `NestedConfirmed`（I1）。DeferOrphan 承接质量迁移：全量 +9、1M +7、250k +3，均与 pre B 链内未确认 Trend rung 事件逐事件闭合（§5 I4 / §6.3 I4）。
- **冲突登记**：p118 关④ 设计前提「链可以穿过未确认父级成证」（`p118-xiaozhuanda-branch-design-20260718.md:42`）与本门直接冲突；教义侧 `doc-level-domain-20260717.md:140` 第 5 条「小转大样本本来就不产生多级链」与门自洽。两条在案文档的冲突本报告不裁。
- **现状处置（卡建议方向的实装事实，非裁定）**：`turn_class` 分类机器保留——对 `from_parts_for_test` 数据载体合成的 `confirmed=[false,…,true]` 输入仍逐测试正确（T8 防回归锁保留，turn_class.rs:367-371 等四处 R-2 注释在案）；生产路径自本门起不再向其喂入 top-unconfirmed 链。类空后的最终处置（保留机器+登记 vs 小转大检测迁出 nest 链载体）待 Lead 裁。

### 7.3 R-3（只落 typed 路径）——按卡执行

旧 `extend_upward` 保持 deprecated 原样（diff=0）；econ 塔外近似链 `build_nest_certificate` 零触，其 95.36% rungs 空基线不受影响（另一条链）。

## 8. 照实否定与边界声明（090）

- **不**结算 E2E-N2：塔原生 `LineageEdge` 逐边见证仍是缺口（`chanlun/plans/mainline-merged-roadmap-20260717.md:72`）；本任务交付 = typed 装配路径 rung 级力度门，仅此而已。
- **不**触生产入场路径：typed 装配消费方 = 探针 bin + runner sidecar（卡 §2.4），生产入场门不消费 typed 证书。
- **不**解决 G-1 活假设、G-3 时钟倒置、G-6 类背驰地板、G-7 skip edge（卡 §8.2 同口径）。
- 截段 ≠ 全量：§6.1 截段读数仅方向性；全量 4.6M 双跑已完成（§6.3），p105 归档 66 张口径与本工作树 pre 基线（5285 张）的漂移已查明归因（p117 终端移位 + wave-1 收束 + 塔演进），非本门效应。
- I4 卡面等式按字面不成立，已按 covered_b（B 口径）+ Trend-only defer 域修正并逐事件闭合（§5 I4 / §6.3 I4）——照实登记，不粉饰为「通过」。
- `lifecycle_cp_ownership_diff=1`/`classification_total_diff=1` 为 worktree 既有、门中性（§5 I5；全量双跑同行读数为 =0/=0，两套读数已并列登记），未查明归属，如实挂账。
- 并行工位冲突照实登记：本报告文件在会话期间被另一并行实装工位覆写过一次（其版本 = 截段双跑证据线），现行版本为两工位证据合并稿（全量 4.6M 双跑 = 本工位产物 §6.3；截段双跑 = 并行工位产物 §5/§6.1）；两工位对 nest.rs/turn_class.rs 的实装内容逐字一致（门线、T1-T7、`from_parts_for_test`、R-2 注释均同），终态 `git diff` 唯一。
- 本文一切命题为结构/集合实测（候选过滤、行级比对、归因计数），零概率推断、零回测验证策略、零 EMH 假设。

## 9. 产物清单

- 代码：`rust/src/theta_v0/classifier/nest.rs`（门 :775、注释 :658-665/:769-774/:787-788、不变量 :720-722、T1-T7 :1778-1959、`from_parts_for_test` :463-507、p97 测试改写 :1751-1776）；`rust/src/theta_v0/classifier/turn_class.rs`（仅测试模块 + R-2 注释 :367-371/:514-516/:621-623/:767-769）。
- 全量重测产物（本工位）：`/tmp/v2_gate_dump_{pre,post}.txt`、`/tmp/v2_gate_p92_{pre,post}_stdout.txt`、`/tmp/v2_gate_p105_{pre,post}.txt`、`/tmp/v2_gate_verify.py`、`/tmp/v2_gate_verify_i34.py`（+ 输出 `_out.txt`）、`/tmp/v2_gate_impl.diff`。
- 截段重测产物（并行工位，`/tmp/v2gate/`）：`dump_{pre,post}_{250k,1M}.txt`、`stdout_{pre,post}_{250k,1M}.txt`、`result_{250k,1M}.json`、`analyze.py`、二进制 `p92_pre`/`p92_post`/`p105`。
- 本报告：`chanlun/review-results/v2-rung-force-gate-impl-20260720.md`（worktree 内）。
