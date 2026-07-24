# L1 字段名诚实化 + L2 观测补全 实装报告（P1–P6 落地 + 三重闸门验证）

- **工位**：实装工位（分支 `kimi-nest-mainline-20260717`，worktree `/tmp/kimi-nest-mainline`）
- **日期**：2026-07-19
- **授权范围**：`rust/src/theta_v0/backtest/runner.rs`（唯一改动代码文件）、`chanlun/review-results/` 下本 `.md`。
  `overlay_state.rs` 与 `strategy/mod.rs` 在授权清单内但**无需改动**（见 §一.3 说明）。
- **设计依据**：`e2e-fix-roadmap-l0-l5-20260719.md` §四 卡 C1–C6 + `m8-fix-plan-20260719.md` P1–P6。
- **纪律**：零 git mutation；主仓 `/Users/silencehan/Projects/NewChanlun` 全程未触碰；`rust/Cargo.toml` 未改。

## 090 状态声明（先于一切）

本报告声明的能力**已全部实装并通过三重闸门**（cargo test 全绿 + OPSEM 518 笔生产数值 bit-exact + 字段迁移对照），
声明=能力，无设计稿冒充。两项**照实否定**如实保留：
① D-V-2「sizing 缺入口」前提被新观测字段**证伪**（§三.4）——不存在缺失入口，units=0.0 是 spec 正确的零 sizing；
② `via_structural_prune` 的 **Rust 字段名未改**（授权外消费方 `l3_delta_r_alpha.rs:2008`），诚实化落在 dump JSON 键名 + 字段注释（§一.2）。

---

## 一 六项改动落地明细（全部在 `rust/src/theta_v0/backtest/runner.rs`，+93/−22 行）

diff 全文存档 `/tmp/l1l2_runner.diff`（206 行）。行号为**改动后**行号。

### 1. ①L1-P1（D-χ-1）`gamma_count_chi_filtered` 谎报消除

- **字段注释诚实化**（runner.rs:2324-2327）：删去「χ 过滤后」谎报，改为「χ=None（π overlay 生产路径，runner.rs:707）时是**未经 χ 过滤的原始候选数**」。
- **新增 `chi_filter_active: bool`**（结构体 runner.rs:2344-2347；赋值 runner.rs:1663 `chi.is_some()`）。
- **序列化键名改 `gamma_count_at_entry` + 新增 `chi_filter_active` 键**（runner.rs:2498-2500）。
- **与设计卡的偏离说明**：任务括号给出「`gamma_count_unfiltered` 或 `_note=chi_disabled`」两选，本实装采用卡 C1/FIX §2.3 的 `gamma_count_at_entry + chi_filter_active` 方案——语义中性、χ 未来接入（L0）后字段名仍正确（FIX §2.6「无需二次改名」的设计优势），且 `chi_filter_active=false` 与任务要求的 `chi_disabled` 标注等价。谎报消除目标达成。
- **验证**：`gamma_count_at_entry` 值 == 旧 `gamma_count_chi_filtered` 值 518/518（分布 `1:335, 2:147, 3:29, 4:2, 5:5`，与 FIX §2.5 预期一致）；`chi_filter_active` 全 `false`（518/518，χ=None 路径如实）。

### 2. ②L1-P2（D-S-1）`via_structural_prune` 来源诚实化

- **dump JSON 键名改 `via_anc_ok_prune`**（runner.rs:2478），锚定 §13 AncOK 机制（`coverage.rs:2290 ancestor_close_by_id`）。
- **字段注释补 090 来源标注**（runner.rs:2200-2210）：明文「True **全部**来自 §13 AncOK silent_drops 分支，与 econ Nest/Xzd 准入门**正交**（π 路径门未接入）——**不是** econ 结构门剪枝」。
- **Rust 字段名保留**（照实否定项）：`l3_delta_r_alpha.rs:2008` 以 `t.via_structural_prune` 消费该字段（`#[ignore]` 测试，仍在编译单元内），该文件**不在本工位授权清单**——Rust 侧整体改名会破坏未授权文件编译，属另一工位。任务允许「改名**或**注释标明来源」，本实装 = dump 键名改名 + 声明处注释标明来源。
- **验证**：`via_anc_ok_prune` 值 == 旧 `via_structural_prune` 值 518/518（分布 `True:187, False:331`，与 FIX §5.5 预期一致；True 全来自 silent_drops 的事实不变）。

### 3. ③L1-P3（D-Ovr-1）`n_overlay_orders` → `n_overlay_fill_events` 三态一致

- **字段改名 + 赋值修正**（定义 runner.rs:656-663；赋值 runner.rs:765 `fill.n_orders`；装配 runner.rs:771）：
  旧赋值 `closed_voices().len() + active_voices().count()`（声部数冒充订单数）→ 新赋值 `fill.n_orders`
  （真 ΔN 非零 fill 事件数；计数点 runner.rs:1191-1195 `executed_qty>0` 才计数，与 ΔN 守恒 debug_assert 同口径）。
- **新增 `n_overlay_voices: usize`**（runner.rs:660-663/766/772）承载原声部数读数，不丢失诊断信息（FIX §4.3 双字段方案）。
- **`overlay_state.rs`/`strategy/mod.rs` 零改动说明**：`fill.n_orders` 是既有已计算值（同源 `net_result.n_orders`），
  ΔN 守恒断言保证 overlay 臂订单 == 净额订单 ⟹ 无需在 overlay_state 侧新设计数器。
- **已知越界影响（必须后续工位处理）**：`rust/src/bin/theta_overlay.rs:80` 仍引用旧字段名 `r.n_overlay_orders`，
  `cargo check --features backtest_bin --bin theta_overlay` 编译失败（E0609）。该 bin 是 feature-gated
  （`backtest_bin`），**不进 `cargo test --lib` 编译面**（本工位闸门不受影响），但下次全特性构建会破。
  该文件未授权，修复 = 一行改名 + 补打印 `n_overlay_voices`（FIX §4.3 后伪码已给出），需授权工位承接。
- **验证**：`n_overlay_fill_events == net_result.n_orders` 同源构造性成立；OPSEM trades.jsonl 零影响（本字段不进 dump）。

### 4. ④L2-P4（D-V-2）`units=0.0` —— **照实否定「sizing 缺入口」前提**（观测字段落地 + 根因定论）

- **新增 `b1_sizing_available: bool`**（结构体 runner.rs:2348-2352；赋值 runner.rs:1665）：
  `b1_sep` 查找前移至 snap 构造前（runner.rs:1599-1606），`b1_sizing_available = b1_sep.is_some()`；
  `LedgerOpen.units` 的 `unwrap_or(0.0)` 防御性兜底**不动**（FIX §6.3 边界）。
- **根因定论（新观测字段实证，推翻 D-V-2/FIX P5 的预期）**：
  - `b1_sizing_available` = **true 518/518**——`filter_map(work.get)` 跳过情形在本样本**零发生**，不存在缺失入口。
  - `units==0.0` 共 **64 笔**，与 `depth ≥ 3` **严格双蕴含**：units==0 的 64 笔全部 depth≥3；depth≥3 的 64 笔全部 units==0。
  - trade_id=223：`b1_sizing_available=true, depth=4, w_depth=0.0`——**真实零 sizing**，非兜底。
  - 机制：`depth_weights=[0.60,0.30,0.10]` 仅 3 项（深度 0/1/2），`depth_weight(depth≥3)=0.0`
    （voice.rs:188-194「超出权重表长度返回 0.0——未用部分保留现金不重分配」，spec:42；`max_depth=3`）。
  - **结论**：units=0.0 是 spec 正确的零 sizing（深度越界不获资金），**不需要也不应该「补齐入口」**——
    强行补 sizing 属伪造资金分配（违 090/v3）。本项的诚实化产出 = 观测字段把「真实零 sizing」与「兜底缺失」在字段层分开。
  - **遗留设计问题（非本工位）**：depth≥3 腿以零 sizing 开仓仍入 typed_ledger（64/518=12.4%，
    exit_type 分布 CloseShortDiff:47/ReduceCore:11/CloseRoot:6，pnl_raw_unlevered 非零但 units 加权 PnL=0）——
    「零目标腿是否应开仓」属 sizing/voice 层设计裁定，需独立工位（path-audit M-1 相关）。

### 5. ⑤L2-P5（D-V-1）`voice_tree_at_entry.depth` 字段

- **新增 `voice_tree_depth: u8`**（结构体 runner.rs:2353-2356；计算 runner.rs:1619-1637；序列化 runner.rs:2506-2509 `"depth":{}`）。
- **口径**：根=0，沿 `parent_id` 链计数——与 `coverage.rs:1559-1567 element_depth` 同口径（铁律：来自 parent 链，非级别差）。
  链源 = 本步 `step_trace.sep_legs` 的 id→parent_id 映射（A_{t+1} 全集，含 registry restore 注入祖先）；
  防死循环上限 16（常态 ≤ max_depth=3）。dump-only，不进 μ 桶键（R5-1 边界）。
- **验证**：depth 分布 `{0:207, 1:146, 2:101, 3:44, 4:20}`（∈[0,4]，超出 max_depth=3 的 depth=3/4 共 64 笔即 §4 的零 sizing 腿）；
  `depth=0` 笔数 207 == `parent_id=None` 笔数 207（严格相等）。
- **附带实证发现**：`voice_tree.depth == certificate.nest_depth` **518/518 全等**——本样本上真嵌套深度与
  `structural_nest_depth`（econ_positive.rs:989 连续包含段计数）逐笔一致，为两计数口径等价性提供了 518 笔证据（非一般性证明）。

### 6. ⑥L2-P6 60/30/10 depth_weights 生效观测

- **新增 `w_depth_at_entry: f64`**（结构体 runner.rs:2357-2360；赋值 runner.rs:1668-1671
  `voice::depth_weight(depth, &config.voice)`；序列化同块 `"w_depth":{}`）——把 sizing 实际消费的深度权重直接外化。
- **生效核验（518 笔）**：
  - `w_depth == depth_weights[depth]`（越界 0.0）518/518 一致，零违例。
  - `w_depth==0 ⟹ units==0` 且 `w_depth>0 ⟹ units>0`（b1_sizing_available=true 下）双向零违例——
    60/30/10 权重**逐笔生效**，字段层可证（裁定 B 的实证闭环：60-30-10 是深度权重数组，非并发阈值）。
  - units 按 depth 分层（b1 可用笔）：depth=0 mean 285.82（n=207）/ depth=1 mean 132.80（n=146）/
    depth=2 mean 41.54（n=101）/ depth≥3 全 0（n=64）——比值非严格 60:30:10 因 `w_dir` 与 base_units 逐 bar 异，
    但单调递减 + 越界归零的形态与权重表设计一致（观测陈述，非统计推断）。
  - `prev_active_count` 分布（对照用，max=22）：`{0:24, 1:18, 2:28, 3:28, 4:31, 5:37, 6:40, 7:37, 8:53, 9:58, 10:42, 11:33, 12:14, 13:16, 14:18, 15:6, 16:9, 17:15, 18:4, 19:4, 20:1, 21:1, 22:1}`——
    与深度权重正交（裁定 B：不存在 30/60 并发阈值，max=22 不说明任何问题）。

---

## 二 闸门 1：`cargo test --release --lib` 全绿零变红

| 运行 | runner.rs 版本 | 结果 |
|---|---|---|
| 基线 ×6 | HEAD（改动前） | 1737 passed; 0 failed（6/6） |
| 改动后 run#1 | 本工位改动 | 1736 passed; **1 failed**（`theta_v0::classifier::tests::tower_water_levels_frontier_rewrite`，classifier/mod.rs:3804） |
| 改动后 ×6 | 本工位改动 | 1737 passed; 0 failed（6/6） |
| 单测隔离 ×3 | 本工位改动 | 该 classifier 测试 3/3 通过 |

**flake 归因**：唯一一次失败为 classifier 模块测试的并行执行 flake——①本工位 diff 只触 `backtest/runner.rs`，
与 classifier 无调用边；②该测试所属 `classifier/mod.rs` 携带**另一工位未提交的 +312 行在飞改动**
（`git diff` 实证，level_view.rs/divergence.rs 等 6 文件同改）；③隔离运行 3/3 通过；④基线与改动后全套件
各 6 次连跑全绿。**结论：非本改动引入；测试总数 1737 基线=改动后，零新增失败、零变红。**

## 三 闸门 2+3：OPSEM 重跑 bit-exact 对照 + 字段迁移对照

- 重跑命令：`OPSEM_DUMP_DIR=/tmp/m8_opsem_l1l2 cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture`（EXIT=0，32.78s）。
- 基线 `/tmp/m8_opsem_p3fold/trades.jsonl` vs 改后 `/tmp/m8_opsem_l1l2/trades.jsonl`：
  - **行数 518==518**；生产数值字段（trade_id/voice_id/position_node_id/entry_bar/exit_bar/entry_px/exit_px/
    pnl_raw_unlevered/exit_type/units/certificate 全块/divergence_input/tw_at_entry/entry_stop_dist/
    trigger_bsp_class_at_exit/exit_z_t_stage）**逐笔 mismatch=0（bit-exact）**。
  - 改名迁移无损：`via_anc_ok_prune`==旧值 518/518；`gamma_count_at_entry`==旧值 518/518；旧键零残留。
  - 新增字段：`chi_filter_active` 全 false；`b1_sizing_available` 全 true；depth/w_depth 分布与一致性见 §一.5/§一.6。
- 对照脚本存档 `/tmp/compare_l1l2.py`（输出即本文 §一/§三数字来源）。
- 副产物说明：重跑覆写 `/tmp/m8_e2e_all_systems_oos.md`（/tmp 临时产物，与基线跑批同行为，非仓内文件）。

## 四 越界与遗留清单

| 项 | 状态 | 承接方 |
|---|---|---|
| `bin/theta_overlay.rs:80` 引用旧字段名，`backtest_bin` 特性下编译失败（E0609） | **已知破坏，未修**（未授权） | 需授权工位：一行改名 `n_overlay_fill_events` + 补 `n_overlay_voices` 打印 |
| `via_structural_prune` Rust 字段名（`l3_delta_r_alpha.rs:2008` 消费） | 未改（dump 键名 + 注释已诚实化） | 需授权工位做 Rust 侧整体改名 |
| depth≥3 零 sizing 腿仍开仓入 typed_ledger（64/518） | 观测到外化，未改设计 | sizing/voice 层设计裁定，独立工位 |
| `gamma_count_at_entry` 采用卡 C1 方案（非任务括号的 `gamma_count_unfiltered`） | 已落地，偏离理由见 §一.1 | 如编排者坚持用 `gamma_count_unfiltered`，一键改名即可 |
| classifier flake `tower_water_levels_frontier_rewrite` | 归因另一工位在飞改动，未处理 | classifier 在飞改动所属工位 |

## 五 v3 + 090 合规声明

| 硬禁令 | 合规性 | 证据 |
|---|---|---|
| 不引入概率/统计推断作决策基础 | ✅ depth 分布/units 分层均为**观测陈述**（计数与逐笔对照），不作决策依据；D-V-2 定论是逐笔双蕴含（64=64），非统计推断 | §一.4/§一.6 |
| 不用回测验证策略 | ✅ OPSEM 重跑仅作字段级 bit-exact 验证，未据 trades.jsonl 数值做任何策略择优 | §三 |
| 不假设 EMH | ✅ 全文无 EMH/随机游走假设 | 全文 |
| 090 禁简化/禁补丁/声明=能力 | ✅ 两项照实否定保留（D-V-2 前提证伪、Rust 字段名未改）；theta_overlay.rs 破坏如实上报不掩盖；χ=None 路径 `chi_filter_active=false` 如实 | §一.1/§一.2/§一.3/§一.4 |

**本报告到此结束。**
