# 影子评审报告 · 票 #70 LEE M1（commit 447fad8f23）

- 评审人：独立影子评审（Opus，新上下文，禁自评资格成立）
- 评审对象：`447fad8f23` 5 文件 +619/−3
- 评审轴：Spec（issue #70 / 设计文档 §C/§D M1 行 / 集成点 (b)(c)）、Standards（090 / 单源 / 确定序 / 防真空 / doc=测试）
- 复跑：`cargo test --release --lib` → **1834 passed / 1 failed**（唯一失败 `signal.rs:3382 extract_signals_bit_exact_digest_guard` = #110 在案，未碰未归因）；`--release --lib level_ledger` **9/9**；debug 模式同 9 项 **9/9**（in-loop `debug_assert` 生效路径无违例）。

## 一、Spec 轴

| # | 条目 | 判定 | 证据 |
|---|---|---|---|
| S1 | M1 守界：不产订单、不动 Consume_ℓ 归因 | PASS | `LevelLedgerStep` 仅 `total_net`/`n_active_levels`（level_ledger.rs:59-66）；`step()` 无订单返回 |
| S2 | 未越界 clock_ℓ（M3）：不拆 `newly_confirmed_step` | PASS | fill.rs 全部新增落在 `ostep` 之后的旁路块（fill.rs:1057-1066），决策路径零改动 |
| S3 | 未越界 sizing w_ℓ（M4）：不改 `leg_target` | PASS | diff 无 `leg_target`/权重公式触及；lot 沿用 `config.risk.default_lot.max(1)`，与 overlay 同一表达式（fill.rs:1045 vs 1058） |
| S4 | formation_level ≡ `id.level`，不引新字段名 | PASS | 分桶键 `leg.id.level`（level_ledger.rs:167-171），零 schema 改动 |
| S5 | 账本行复用 VoiceBook/ClosedVoice | PASS | `use super::overlay_state::{side_sign, ClosedVoice, VoiceBook}` |
| S6 | bit-exact 旁路证据链 | PASS | 净额臂/W1 臂传 `None`（fill.rs:354/378）；回归锁逐字段对拍 `n_orders`/`typed_ledger`/`tw_final`/`equity_curve`（runner.rs 测试① ~2166-2172） |
| S7 | LEE-Net 恒等 release 证据分层 | **PARTIAL（MED-1）** | 见下 |
| S8 | 风险点①②（restore 祖先腿 / 幽灵腿）落测 | PASS | `level_closure_cross_level_parent_buckets_by_own_level`、`sub_lot_leg_excluded_identically_to_overlay` |

## 二、Standards 轴

| # | 条目 | 判定 | 证据 |
|---|---|---|---|
| T1 | 单源复用 `side_sign`（禁复制） | PASS | overlay_state.rs:51 提 `pub(crate)`，mirror 直接引用，零重复实现 |
| T2 | 确定序 | PASS | 双层 `BTreeMap<level, BTreeMap<ordinal,…>>`；净额/PnL 累计逐声部独立 ⟹ 序无关，bit-equal 论证成立 |
| T3 | rebalance 语义与 overlay 逐行同构 | PASS | 复合键 `(level, ordinal)` ≡ overlay 的 `ElementId` 键（overlay_state.rs:190-245 vs level_ledger.rs:165-232）：取整、离场判定、entry_px 手数加权、`side` 防御覆盖、`force_flat` 不再累计 ΔP，五处逐条同式 |
| T4 | 测试防真空 | **PARTIAL（MED-2）** | 测试①有非空前置；测试②无 |
| T5 | doc 声明 = 测试锁定 | PASS（1 处措辞落差 LOW-2） | 头注三不变量各有对应测试 |
| T6 | 文件规模/可读性 | PASS | 466 行（含 218 行测试），< 800 上限 |

## 三、问题分级

**无 HIGH（不回票）。**

### MED-1 · release 下「生产管线 × 非零敞口时刻」的 LEE-Net 恒等无硬证据
`[profile.release]`（Cargo.toml:95-101）未开 `debug-assertions` ⟹ fill.rs:1059-1065 的逐 bar `debug_assert_eq!` 在项目基线口径（`cargo test --release`）下**零执行**。而两个 integration 测试的恒等断言均取在 `force_flat` 之后，此时两侧恒为 0 ⟹ `0 == 0` 平凡成立。唯一非平凡的逐步恒等在 `lee_net_identity_holds_per_step_multi_level`，其 `sep_legs` 为手工构造，不经生产 `step_trace` 路径。结果：**「生产 sep_legs × 逐步恒等」这一格在 release 基线下是空的**。issue 的「照实分层」声明覆盖了 debug_assert 不执行这一事实，但未覆盖 integration 恒等断言的平凡性。
建议（不阻塞合入）：在 fill loop 内累计 `max|Σ_ℓ net_ℓ − N|` 与 `max|N|` 两个读数并落 `OverlayRunResult`，测试断言「残差 == 0 且 max|N| > 0」——一行前置即把该格填实，且 release 可执行。

### MED-2 · `run_theta_v0_pi_overlay_level_ledger_wired` 缺非空前置（防真空）
该测试全部断言形如 `ll.n_closed() == ov.closed_voices().len()`、`total_net() == net()`、`for lvl in levels()` 循环体——若该 60-bar 锯齿数据集在 overlay 上产生零声部，全部断言在空账下平凡通过、循环零次执行。同 commit 的测试①有 `assert_eq!(ov.closed_voices().len(), 1, "前置：…")`，二者不对称即证明该前置是本 commit 自认的规范。补一行 `assert!(ov.overlay.closed_voices().len() > 0)` 即闭合。（注：本评审未实测该数据集是否真空；缺前置断言本身即缺陷。）

### LOW-1 · fill.rs 窗口终点 `last_i`/`last_px` 计算重复两份
fill.rs:1454-1459（overlay）与 1461-1466（mirror）逐字重复同一 `(0..n).rev().find(…)` + tick 换算。单源复用轻违背，未来改判定条件时漏改一处即静默失锚。合并为一层 `if let Some(last_i)` 外层即可。

### LOW-2 · 注释声明「精确为 0 残差」但断言用 1e-9 容差
`mirror_rows_bit_equal_overlay_rows` 注释写「逐行 bit-equal ⟹ 精确为 0 残差」，紧随的 Σ 对账用 `abs() < 1e-9`。逐行 bit-equal 不蕴含 Σ bit-equal（两侧 `closed` 求和序不同：mirror 为 ordinal 升序，overlay 为 HashMap 序，f64 结合律不保）。此处该改注释（承认求和序差）或改断言（排序后求和再 `to_bits`）——090 声明/测试落差，就低不就高。

### LOW-3 · `(overlay=None, level_ledger=Some)` 组合下恒等断言静默跳过
fill.rs:1060 的 `if let Some(ov) = overlay.as_deref()` 使该组合下 LEE-Net 恒等不作任何检查。当前唯一调用点（runner.rs:690-692）恒传 Some/Some，故无现实风险；作为 API 面松散登记。

## 四、结论

**通过（不回票）**。Spec 轴 8 项 7 PASS / 1 PARTIAL，Standards 轴 6 项 5 PASS / 1 PARTIAL，无 HIGH。M1 守界与 bit-exact 旁路证据链成立且可复现；rebalance 语义与 overlay 逐行同构经独立比对确认。两项 MED 均属**证据强度**而非行为正确性——建议在 M2 票内一并闭合，不作为 #70 的阻塞项。

---

### 结果包六要素
1. **结论**：如上，通过（2 MED / 3 LOW，无 HIGH）。
2. **定义依据**：设计文档 §C.1 支柱1（账本行可复用）/§C.2（LEE-Net 恒等、级别封闭、只读旁路 bit-exact）/§D 迁移表 M1 行；集成点文档 (b)（不引 `formation_level` 新字段名）/(c)（最小实装面①与明确排除项）。实装的分桶键、恒等式、守界范围逐条对应。
3. **边界条件**：若 CI 或基线口径改为 `cargo test`（debug）或 release 开启 `debug-assertions`，MED-1 自动失效（逐 bar 恒等即成为可执行硬证据）；若实测证明 `ZZ60OV` 60-bar 数据集在 overlay 上产生 ≥1 声部，MED-2 降为 LOW（仍缺显式前置，但非真空）。若 `ElementId` 将来增字段，level_ledger.rs:214 的字面量重建与 T3 的键等价论证同时失效，届时 bit-equal 结论翻转。
4. **下游推论**：M2（Consume_ℓ 独立目标 + 订单归因）可在本镜像上直接扩展，无需回改 M1 结构；但 M2 一旦让镜像产订单，「只读旁路 ⟹ bit-exact」这条锁失效，需新的对拍协议（不能沿用本票的四字段回归锁）。
5. **谱系引用**：涉 090（严格性/声明膨胀，LOW-2）、formalization-validity-domain 231号（本实装自标 L1 管线正确性，未声明 alpha——该标注准确）。未发现与既有概念分离谱系冲突。
6. **影响声明**：本报告为只读评审产出，除本文件外未改动任何文件、未提交、未触碰工作区 ~370 个 ` D` 与 #312 的 `parser/` 未提交改动。
