# #743 架构落地后的存量票据漂移普查（issue #785 产出）

- 日期：2026-07-30
- 票据：#785（parent #743 结案后的存量漂移普查，归口 map #695）
- 边界：**只判不改**——本文档未 close/edit 任何 issue，未改任何图正文，未动 main 代码。
- 方法：#743 Decisions-so-far 12 条决策逐条 `git show --stat <commit>` 核实实际改动文件；对全部 61 张 open issue（含本票）做关键词命中扫描（命中集 = 12 条决策实际触及的文件路径/符号名）；命中票逐条 Read 票面 + Read/grep 仓内当前事实核对；`gh issue view` 拉取三张 open map（#695/#529/#737）正文逐段核对。

## 0. #743 十二条决策 × 实际改动文件（证据底座）

| 决策 | 票 | commit | 实际改动（`git show --stat`）|
|---|---|---|---|
| C5 | #744 | `52c4fd53b1` | `backtest/mod.rs`、`backtest/runner.rs`（8697→784行，测试段全迁出）、`backtest/runner_tests.rs`（新增，含全部 `#[test]`）|
| C6 | #745 | `6e4193bf4e` | `classifier/pipeline.rs`（**整删**416行）、`econ_positive.rs`（注释2行）、`classifier/{ref_v1,six_state,voice_eat}.rs`（名分标注）、`complete/mod.rs`、`ledger/separate.rs` |
| C7 | #753 | 无（名分表批准，纯决策，衍生 E1-E5） | — |
| C7-E1 | #760 | `f0a4a1a597` | `classifier/{cand_delta,sublevel,incremental_profile,stage_profile,tower_cache}.rs`（**整删**）、`classifier/incremental/`（5文件整删）、`classifier/tests/`（5文件整删），共 3933 行；报告明载：这批文件从未被 `mod` 声明纳入编译树，与现役 `backtest::incremental`（另一实体）不混淆 |
| C2 | #746 | `946cea9d45` | `env_registry.rs`（新增555行，46键单源）+ 98 处调用点迁移，覆盖 `backtest/{admission,capture_oos,econ_positive,fill,gamma_dump,incremental,opsem_dump,perm_test,runner_tests,segment_gn,wverify_run}.rs`、`classifier/{mod,rebase_txn,signal}.rs`、`lineage_book.rs`、`mod.rs` |
| C7-E2 | #761 | `b1032618e6` | `recursive_t/{center,divergence,mod,operator,trend,types}.rs`（GUARD-ROLE 注释标记，零逻辑）|
| C1 | #747 | `eeb8be23aa` | `classifier/bsp.rs`（**新增**242行，`bsp_at`/`bsp_bit_at` 族）、`bsp_bridge.rs`、`bsp_bridge/tests.rs`、`classifier/{mod,nest,projection}.rs`（nest.rs 为 rustfmt 重排，无语义改动）、`backtest/{admission,econ_positive,fill,signal}.rs`（econ_positive/fill 为 rustfmt/单源调用改写，非签名变更）、11 个 `bin/p*.rs` 探针 |
| C7-E3 | #762 | `451edf8722` | `fugue_v3/*`（11文件）、`recursive_t/*`（11文件，覆盖 C7-E2 已标注的同批文件）——纯注释名分标记 |
| C7-E4 | #763 | `62d6ffd119` | `trading/*`（29文件）——纯注释名分标记，零逻辑，两活物零触碰 |
| C4 | #748 | `8515416539` | `classifier/mod.rs`（6464→5389行）、`classifier/diag/`（新增5文件，1107行迁出）、`classifier/tower_cache.rs`（**新增**563行——注意：这是 mod.rs 内联逻辑的新抽取，非恢复 C7-E1 删除的旧 604 行孤儿版本，两者内容不同）|
| C3 | #749 | `b88f5f472d`（docs-only） | 无代码改动；结论：`fill.rs` 三"职责"（`drive_campaign_wiring`/`account_mirror_*`/8组探针）**不拆**——单一生产调用方 `pi_theta_fill_loop_overlay`，搬迁不消灭跨文件复杂度 |
| C7-E5 | #764 | `a0ff1483eb` | `rust/src/` 顶层17个散件（`bi_engine.rs` 等）——纯注释名分标记，14件标现役+2件（`c_segment_verify.rs`/`segment_tangency_tests.rs`）单列 |

**关键澄清**：C6/C7-E1/C7-E2/C7-E3/C7-E4/C7-E5 中除 C6（真删 `pipeline.rs`）与 C7-E1（真删 3933 行孤儿簇）外，其余 E 系列均为**纯注释名分标记，零逻辑改动**；C1/C2/C4/C5 均带 `cmp=0`/`wf8` 金标准零行为变化独立实证。**没有一条决策改变过任何票据可观察的运行时行为**——凡票据判定基于"行为变了"的推测，均不成立；漂移只可能来自**文件路径 / 函数名 / 行号 / 测试落点 / env 访问方式**的物理挪动，或该文件被判定为孤儿死代码整删。

## 1. 四分清单表

### 作废（0 条）

本次普查**未发现**票面前提被彻底消灭（指向已删文件且事情已做完 / 与新裁定正面冲突到无法执行）的票据。已知嫌疑名单中最像"作废"的 #125，经查证据不足以支撑"作废"（详见"需改写"表），改判需改写。

### 需改写（4 条）

| 票号 | 票名 | 判定 | 三段证据 |
|---|---|---|---|
| **#125** | fill.rs FillContext 提取：fee_rate 单一来源 + 共享 apply_order + 状态管理 | 需改写 | **票面**："三 bar-loop（`pi_theta_fill_loop_overlay` / `plan_and_fill_mtm` / `plan_and_fill_mtm_dual`）...fee_rate ×6 拷贝统一为单一来源"。**#743 决策**：C3（#749，`b88f5f472d`）对同一文件 `fill.rs` 的内部拆分经济性给出"不拆"裁定（三职责单一生产调用方，风险大于收益）；C1（#747）对 `fill.rs` 做了 rustfmt 重排（无签名变化）。**仓内事实**：`git show 6033c79085:rust/src/theta_v0/backtest/fill.rs` 实查——`plan_and_fill_mtm`/`plan_and_fill_mtm_dual` 两函数**不存在**（当前三条 bar-loop 是 `simulate_fills`/`pi_theta_fill_loop`/`pi_theta_fill_loop_voice`/`pi_theta_fill_loop_overlay`）；`fee_rate` 独立赋值点仅 2 处（:47、:4405），非票面所称 6 处；`apply_order`（:118）、`track_position_transition` 仍在。判定理由：函数名与重复计数双双过期，需要重新盘点当前 bar-loop 拓扑；且 C3 已就同文件拆分给出经济性否决，改写时须明确本票诉求（跨循环 FillContext）与 C3 否决范围（单循环内三职责）是否属同一问题，若同构则本票风险已进一步升高。 |
| **#404** | runner.rs 测试归位：288 个测试仍压在装配层，四个 seam 零自测 | 需改写 | **票面**："runner.rs 共 8697 行...测试段 928-8697（7770行，288个测试函数）全部压在装配层"。**#743 决策**：C5（#744，`52c4fd53b1`）已把 6720+ 行测试从 `runner.rs` 迁出到独立文件 `runner_tests.rs`（commit message 明载"mod tests 6720 行迁 runner_tests.rs"）。**仓内事实**：`runner.rs` 现 784 行、`grep -c '#\[test\]'` = 0；`runner_tests.rs` 含 118 个 `#[test]`。票面字面前提"测试仍压在装配层"已被打翻——`runner.rs` 本身已不含测试。但票据深层验收目标（按 seam 归类到 `signal.rs`/`admission.rs`/`fill.rs`/`ledger.rs`/`opsem_dump.rs` 各自文件）**未达成**——C5 只是把测试整体搬到相邻的单一文件 `runner_tests.rs`，不是按 seam 分发。需改写：起点从"runner.rs 里的 288 个测试"改写为"runner_tests.rs 里的 N 个测试（实测数字与 288 有出入，需重新点数）待按 seam 二次搬迁"。 |
| **#648** | classifier 主干切换：kimi 拆分格局扶正、main 内联退役 | 需改写 | **票面**："正房基底 = kimi 封存 tip `b242452174` 的 `classifier/` 子树（含 ... #633 incremental 目录化与 ≤50 收敛）"。**#743 决策**：C7-E1（#760，`f0a4a1a597`）"零引用证据"逐件核实后**整删** `classifier/incremental/`（5文件）、`tower_cache.rs`（604行）、`sublevel.rs`、`cand_delta.rs`、`incremental_profile.rs`、`stage_profile.rs`、`tests/`（5文件），报告明载"从未被 `mod` 声明纳入编译树"，与现役 `backtest::incremental`（另一实体）确认不混淆；C4（#748，`8515416539`）又把 main 内联版 `classifier/mod.rs` 从 6464 行经 `diag/` 拆分（5新文件）压到 5389 行，并新增独立 `tower_cache.rs`（563行，与被删的 604 行版本内容不同，是从 mod.rs 内联逻辑新抽取，非旧孤儿复活）。**仓内事实**：#648 票面要"扶正"的 `classifier/incremental 目录化` 在 main 树里此前留存的同名/同构死壳副本已被坐实为零引用孤儿并物理删除；main 内联版本身结构也已因 C4 改变（`diag/` 子目录 + 新 `tower_cache.rs`）。判定：C7-E1 的裁定不是否定 #648 的既定方向（#643 裁定 (b) 依然是"kimi 拆分格局为正房"，未被推翻），而是打翻了 #648"方法"段第1-3步描述的当前 main 分支起始状态——实施时不能再假设 main 树里有可直接 `mod` 声明激活的既存副本，必须从 kimi 冻结 tip 重新拉取干净子树，且要核对 C4 已改变的 mod.rs/diag/tower_cache.rs 落点是否与待移植子树冲突。 |
| **#399** | econ_positive.rs 截窗骨架收敛：4 处 MAX_BARS 复制抽共享 helper + 截断边界补单测 | 需改写 | **票面**："四处「载数据→MAX_BARS 环境变量截断→分桶→writeln! 报告」骨架...抽成共享 helper"，列出 4 个函数（:2674/:3035/:3193/:3593，行号均为票面原始行号）。**#743 决策**：C2（#746，`946cea9d45`）"46键单源+98处迁移"，其中 `econ_positive.rs` 在案；C6（`6e4193bf4e`）与 C1（`eeb8be23aa`）也各对该文件有 2/12 行改动（注释更新+bsp 单源调用改写，语义不变）。**仓内事实**：`git show 6033c79085:.../econ_positive.rs` 实查——`MAX_BARS`/`MAX_BARS_DEFAULT` 截窗骨架现出现于至少 **9** 处测试函数（除票面原 4 处外新增 `acc_classification_level_hole_dx`/`acc_bottomup_nest_parity_probe`/`acc_nest_trigger_quality_probe`/`h2_sample_exclusion_dx`/`l2_depth_distribution_dx`），且全部 env 变量读取已改写为 `crate::theta_v0::env_registry::ECON_L2_MAX_BARS`（C2 单源注册表调用），非票面原文暗示的裸 `std::env::var("ECON_L2_...")` 字符串字面量。判定：票面"4处"范围已过期（实为9+处，文件本身在此期间独立增长），且抽取 helper 时必须遵循 C2 已落地的 env_registry 单源纪律（helper 内部读取应走注册表键而非裸字符串），否则新 helper 会与 C2 裁定的"46键单源"冲突。需改写范围清单与实施口径。 |

### 待人判（1 条）

| 票号 | 票名 | 拿不准在哪 |
|---|---|---|
| **#485** | [SPEC] 高级别三类点产出链修复：anchor 门 + 归属链 + 终态切换 | **票面**：Testing Decisions 第1层"classifier 夹具层...沿用 `mod.rs`/`signal.rs` 既有 anchor 测试族（p117 `q7_ruling_c_*` 同款夹具）"。**#743 决策**：C7-E1（#760）报告"补注"明载删除了 `classifier/tests/level_signals.rs` 中的 `q7_ruling_c_first_class_structural_direction_third_class_provenance_kept`（旧口径 `buy3=0`，报告称"系 #576 搬走的 Q7-#1 裁定C 旧口径...现役口径由 `mod.rs:3783` 覆盖（#486 后，buy3=1）"，删除的是死孤儿非现役版本）；C4（#748）又把 `classifier/mod.rs` 从 6464 行经 `diag/` 拆分压到 5389 行（5新文件迁出 1107 行）。**拿不准的点**：#485 引用的"`q7_ruling_c_*` 同款夹具"究竟指向仍在 `mod.rs:3783` 现役的版本，还是与 C7-E1 删除的死孤儿版本存在命名/结构上的隐性依赖；且 C4 的 `diag/` 迁出是否把 #485 Testing Decisions 段落引用的具体 anchor 测试函数带出了 `mod.rs`/`signal.rs`。这需要逐一核对 #485 spec 全文列出的所有测试接缝断言（范围 1-3 共约十余条）与当前 `mod.rs`/`diag/*.rs`/`signal.rs` 的函数级现状，工作量超出本次快速核验（本票为 SPEC 大票，非本次三段证据流程可在合理篇幅内完全坐实），故单列待人判，建议下一步指派专项核对。 |

### 不受影响（约 55 条，含 60 张已核 open issue 中除上述 5 条外的全部）

判定方法：逐一核对票面正文与"证据底座"表列出的全部改动文件路径/符号名，采用两级核验——(a) 关键词命中扫描（票面提及的文件路径 vs 12 条决策实际触及的文件路径集合，零命中直接归不受影响）；(b) 对扫描命中但经详查确认无实质交集的票据单独重新核实。

**(b) 类——命中关键词但详查后确认无交集**（逐条留痕）：

- **#410** `filter_gamma` 死代码：票面目标文件 `backtest/selector.rs` **未出现在任何一条决策的改动文件列表中**；`git show 6033c79085:.../selector.rs` 实查 `filter_gamma`/`filter_gamma_with_admission` 两函数仍在（:351/:382）。不受影响。
- **#454** `nest_lifecycle.rs` 拆分：`classifier/nest_lifecycle.rs` 未出现在任何一条决策的改动文件列表中。不受影响（**独立旁注**：票面称"2212行"，仓内实测已达 6386 行，此漂移源自本票范围外的并行工作，与 #743 无关，不在本次判定范围内，仅供下一步经手人参考）。
- **#649** FeeQuoter 移植：票面目标签名 `apply_open`/`apply_close`/`settle_forced_virtual`（`fee_rate: f64 → fees: &FeeQuoter`）在 `overlay_state.rs`；C1 对 `fill.rs` 的改动经 diff 核实**仅为 cargo fmt 重排**（参数换行），未改任何函数签名；`overlay_state.rs` 本身不在 12 条决策改动列表内。不受影响。
- **#600** #571 影子评审 HIGH-1 修复：目标 `open_ledger.rs`、`fill.rs:1789` 附近语义——`open_ledger.rs` 不在改动列表内；`fill.rs` 改动仅格式化。不受影响。
- **#620** 账本内核抽取 spec：目标 `nest_lifecycle.rs` 不在改动列表内。不受影响。
- **#754** N3 平价锁重言式：目标 `classifier/chain_cert/{mod,tests}.rs`，票面自述"只动 chain_cert，模块不同"（与同期在飞的 `bsp_bridge` 修复轮已自行划清）；`chain_cert/` 不在 12 条决策改动列表内（C1 只动了 `bsp_bridge.rs`，是不同模块）。不受影响。
- **#765** `retrace_ledger/book.rs` 瘦身：该文件不在改动列表内。不受影响。
- **#485** 见"待人判"。
- **#640** seed→CompletedMove：目标 `first_retrace_replay.rs`（已在 #743 之前由 #624 裁定删除，与本次12条决策无关）、`provider`/`level_view` 侧，均不在改动列表内。不受影响。
- **#713** restore 跨 bar 旧方向复活：目标 `recursive_tower.rs:83-88`（`ElementId` 定义）——注意与 C7-E1 删除的 `classifier/incremental/tower.rs` 是完全不同的文件（前者是塔核心对象定义，现役；后者是孤儿死代码）。不受影响。
- **#723**/**#728**/**#730**/**#735**：目标分别为 map #500 雾裁定（无具体文件）、`strategy/coverage/element.rs`（`ElementView`）、`backtest/{ledger.rs:71,selector.rs:299}`——均不在 12 条决策改动列表内。不受影响。
- **#778** N6 Sel_Θ：票面引用 `nest.rs:56-90`、`nest.rs:543-547 vs 798-807`（两套 DFS 序）——C1 对 `nest.rs` 的 diff 逐行核实为**纯 rustfmt 重排**（换行/缩进），未见 `sel_order`/`select_best` 关键词改动，语义未变。**旁注**：rustfmt 重排会移动行号，票面所引精确行号（56-90/543-547/798-807）存在漂移风险，下一步经手人核对时建议按函数名而非行号定位；语义判定本身不受影响。

**(a) 类——关键词零命中，视为不受影响**（覆盖 map #695 全部 L 编号子票 #715/#716/#717/#719/#721/#722/#726/#731/#733/#737/#740/#750/#757/#759 及其余无文件/符号级重叠票据 #158/#159/#160/#161/#169/#180/#241/#252/#335/#341/#364/#368/#372/#378/#407/#411/#412/#462/#466/#627/#657/#680/#688/#756/#783 等）：票面正文均未提及"证据底座"表列出的任一改动文件路径或符号名（`fill.rs`/`runner.rs`/`runner_tests.rs`/`env_registry.rs`/`classifier/{mod,bsp,bsp_bridge,nest,projection,tower_cache,pipeline,incremental,cand_delta,sublevel,stage_profile,incremental_profile,diag/*}.rs`/`recursive_t/*`/`fugue_v3/*`/`trading/*`/`rust/src/*.rs` 顶层散件/`econ_positive.rs`/`backtest/signal.rs`）。

补充两条个别核实：
- **#407** `signal.rs` 第三处 StopInput：目标 `backtest/signal.rs:96-99`（`entry_structural_stop`）与 `strategy/mod.rs`——C2 迁移触及 `classifier/signal.rs`（不同文件，同名不同路径），C1 触及 `backtest/signal.rs` 但 diff 仅为 rustfmt/单源调用改写（7行），未涉 `entry_structural_stop`/`center: match bsp.center` 段落。不受影响。
- **#756** SPEC：本票即 #743 自身实施总单（map #743 的产出源头，非下游被打翻对象），其内容随 12 条决策逐票落地已基本执行完毕；不计入"被 #743 打翻"范畴，仅作说明性排除，不进四分主表。

## 2. 三张 open map 逐段影响

### #695（欠账总账，68笔）

- **Destination**：无冲突——总账口径本身不描述具体代码路径，#743 的12条决策均未触及。
- **Notes**：无冲突。
- **Decisions so far**：其中 `research：陈旧票交付状态核对`（#696）判定表已把 **#125** 列入"未交付5"（与 #158-161 一并归"图尾"处理）——本次普查发现 #125 因 C3（#749）裁定 + 函数名/计数过期需改写，与 #696 判定表"未交付、归图尾"的既有安排**不冲突**（仍是未交付，只是改写理由更新），但下一步经手人处置 #125 时应把本报告的三段证据一并带上，避免图尾处置时按票面原始（已过期）内容执行。
- **Not yet specified** / **Out of scope**：均未提及与12条决策重叠的文件路径，无冲突。

### #529（塔内原生化长线，N0-N7）

- **Destination/Notes**：无直接冲突——12条决策全部标注"零行为变化"（cmp=0/wf8金标准），#529 的 N0-N4 已交付面（#537/#540/#550-553/#544/#636/#641/#666/#668/#669/#671）建立在生产语义之上，未受影响。
- **文件级重叠**：C1（#747）改动了 `classifier/bsp_bridge.rs`/`bsp_bridge/tests.rs`——这正是 #529 N4（#668，已关闭子票）实装的核心文件。C1 报告"行为零变化独立实证"（17处逐点等价+wf8 cmp=0复跑），且 #754（N3平价锁重写）票面已自行申报与同期 `bsp_bridge` 在飞修复轮（`ticket-668c`）划界。综合判定：#529 图正文（Destination/Notes/Not yet specified/Out of scope）**无一句被打翻**，但 `bsp_bridge.rs` 内部实现已随 C1 改为调用 `classifier::bsp::bsp_bit_at` 单源 API，下一步凡直接引用 `bsp_bridge.rs` 内部实现细节（而非仅接口行为）的子票（如 #778 N6、#754 N3）应留意函数体已变但接口/行为不变。
- **Not yet specified** 第3条"塔内原生化 vs nest 独立对照的演化关系：全部原生后对照面存废"——与 ADR-0005 相关，未被12条决策触及。

### #737（missing_cert 成因定案）

- **Decisions so far**：空（无已决票），本图仍处于纯观测/裁定前阶段。
- **Notes**：纪律段"新口径禁止（用既有探针与跑法）"——12条决策改动了若干探针 bin 文件（`bin/p_issue668_bsp_bridge_battery.rs`/`bin/p_issue668_bsp_key_truth.rs` 等，C1 触及），均为内部调用改写（改调 `bsp_bit_at` 单源 API），报告称零行为变化；探针**输出口径**未变，`filter_gamma`/`selector.rs` 等 missing_cert 计数相关文件完全未被12条决策触及。
- **Destination/Out of scope**：均未提及与12条决策重叠的具体文件，无冲突。

## 3. 计数汇总

| 分类 | 数量 |
|---|---|
| 作废 | 0 |
| 需改写 | 4（#125、#404、#648、#399）|
| 不受影响 | 55（含全部核实过的 open issue，扣除本票 #785 与需改写/待人判/#756说明性排除）|
| 待人判 | 1（#485）|
| **open issue 总数（含 #785）** | 61 |
| 三张 open map 逐段影响 | #695：Decisions-so-far 一条注记须随 #125 更新；#529：无正文被打翻，`bsp_bridge.rs` 内部实现变化需下游子票留意；#737：无正文被打翻 |

## 附：判定纪律自查

- 每条判定均遵循"票面原句 / #743 决策+commit / 仓内当前事实"三段证据结构。
- "作废"档为空并非因为放松标准，而是12条决策中除 C6/C7-E1 外全部为零行为改动或纯注释标记，真正物理删除的文件（`pipeline.rs`、`classifier/incremental/*` 等）当前没有任何 open issue 直接以其为唯一目标（#648 的目标是"扶正 kimi 格局"而非"操作这些具体死壳文件"，故判需改写而非作废）。
- "待人判"仅 #485 一条，系因其为 SPEC 级大票、测试接缝断言条目多，超出本次三段证据核验的合理篇幅，如实标注而非勉强判定。

---

## 4. 订正节（2026-07-30 晚，#485 专项核对回报后追加）

本报告第 1 节「待人判（1 条）」已由专项核对结清。**四分计数订正：作废 0 / 需改写 4 / 不受影响 56 / 待人判 0。**

### #485 改判为「不受影响」

Testing Decisions 段点名的六处接缝逐条实查，无一失效：

| 接缝 | 状态 | 证据 |
|---|---|---|
| `q7_ruling_c_first_and_third_class_structural_direction_authorized` | 在原处现役，`classifier/mod.rs:2825` | `git diff 8515416539^ 8515416539 -- mod.rs` 对该函数名零命中——C4 未改函数体，行号由 3900 挪至 2825（上方诊断逻辑迁出所致） |
| C7-E1 删除的同名族兄弟 | 非同一份夹具 | 被删者为 `q7_ruling_c_first_class_structural_direction_third_class_provenance_kept`（`buy3=0`，与现役 `buy3=1` 断言方向相反），被 `mod.rs:3049` 内联 `mod tests {}` 遮蔽、从未进编译树；系 #486 已裁废的旧口径死壳 |
| `judge_first_cached_provenance_gate_preserved_for_direct_callers` | 在原处现役，`signal.rs:2674` | C2 迁移 diff 零命中 |
| `center_oscillation_wiring_tests` 族（24 个 `#[test]`） | 在原处现役，`fill.rs:2532` | C1 对 fill.rs 仅 rustfmt 重排，该 mod 名零命中 |
| `gate_on_chain_rebase_migrates_survivor_and_writes_off_vanished_suspension` | 在原处现役，`fill.rs:3206` | 同上 |
| `m8_e2e_all_systems_oos` | 在原处现役，`wverify_run.rs:1535` | 该文件不在十二条决策任一改动列表内 |

### 本报告第 1 节的一处引数错误

第 46 行「现役口径由 `mod.rs:3783` 覆盖」中的行号不准——实际 C4 前 3900、C4 后 2825。**结论方向不变**（现役版本存在、非 C7-E1 删除对象），仅引数错。

### 附带发现（与 #743 漂移无关，独立事实）

**#485 的「范围 1（anchor 门）」在 #743 之前已作为 #486 落地合并进 main**：`8e3cc9bd14 fix(classifier): #486 高级别三类改用结构方向锚` + `a3bd8ac008 merge: #486 anchor 门结构方向授权`；现役夹具文档注释自载「★ADR 补充十三 / #486 / Spec #485」。

范围 2（归属链框四边配对）/ 范围 3（终态切换）实现状态本次未深查，线索一条：`SuspensionTerminationSource`（`strategy/center_oscillation_trade.rs:327`）当前仅 `BrokenByThirdClassBuy` / `BrokenByThirdClassSell` / `RebaseVanished` 三变体、无独立 `Reset` 分支，提示范围 3 可能亦已部分动过。

处置（#485 是否缩为「范围 2/3 收尾票」）归 [#786](https://github.com/xy7365527-lang/NewChanlun/issues/786)。
