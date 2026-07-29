# 票 #605：死路径清除定档 + p123 两长函数拆分 + 措辞订正——交付报告

- 日期：2026-07-28（工位跨 2026-07-29 零点收口）
- 票：#605（sonnet 常规档实施票）
- 工位：`/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`；主仓只读，全程无 git mutation 越权
- 基线 HEAD（开工核面）：`9e3cd25fd6` 系列 → 收口时 HEAD 演进至 `02ef5f93b1`（并行工位 #621/#497 在同一工作树上各自 commit 推进，`git status` 全程核实未与本票目标文件交叉）

---

## 0. 结论先行

1. **死路径三件**（`provide_replay_live_windows` / `PanLiveRun<'a>` / `leg_as_segment_copy`）核实为全库仅测试可达，本轮**只做定档建议 + 措辞对齐**，未删一行代码——建议表见 §1，删除动作留编排者终审。
2. **p123 两长函数已拆分**：`main()` 378→52 行（12 个新助手函数），`run_targeted_prefix_pass()` 606→约 60 行（拆为 `TargetedPassState` 结构体 + 27 个助手函数/方法），全部 ≤50 行；p123 20k/100k stdout/stderr（除 `prefix_s` 计时字段外）逐字节零 diff，`issue533_p123_byte_guardrail` 两测试全绿。
3. **措辞订正三处**：`nest_lifecycle.rs` 死路径三件的模块头/函数doc 已对齐当前实装（本轮落地）；`IdentityVanished=0` 验收句失效订正**核实已在 2026-07-28（#559 裁定）随 commit 落地于 `nest_lifecycle.rs:71-80`，本轮未发现残留的活口径声明**（历史评审报告/roster 日志按谱系保留原则不改写）；`#421`/`#427` 票体正文的「时钟精度=触发点粒度」措辞**仍是 GitHub issue 活文本的未订正残留**，本轮**未直接编辑 GitHub issue**（发帖属外部可见/半不可逆动作，超出本票已授权的本地 git commit 范围），已在 §3 给出可直接采用的订正评论草稿，供编排者授权后过账。
4. **⚠️ 重大偏离（非本票产生，需编排者关注）**：开工核面时发现工作树内 **47 个 `chanlun/review-results/`/`chanlun/escalate/` 下已提交（HEAD 内存在）的追踪文件在磁盘上物理缺失**（`git status` 显示为未暂存的 `D`），成因不明——非本票任何操作所致（本票全程未touch该目录，且这些文件在会话开始时的 `git status` 快照中并未出现）。已如实记录清单于 `/tmp/issue605-baseline/deleted-files-list.txt`，本次**未做任何恢复/暂存操作**（`git checkout --`/`git restore` 属禁止的 git mutation），留编排者裁定处置（大概率是同工作树上并行工位操作引入，需与 #621/#497 工位核对）。

---

## 1. 死路径三件定档建议表

判据来源：#225 五档树（1 留 / 2 接 / 3 前瞻 / 4 归档 / 5 删）。

| 件 | 位置 | 建议档位 | 证据 |
|---|---|---|---|
| `provide_replay_live_windows` | `nest_lifecycle.rs:2284`（pub fn） | **5 删**（终审待编排者） | 全库唯一调用点 `nest_lifecycle.rs:3875`，位于 `#[cfg(test)] mod tests`（模块起于 2548 行区间，本函数定义在其外但调用点在其内——`pub` 可见性使编译器不报 `dead_code`，这正是同类死路径长期藏身的既知机制，见 `rust-orphan-five-tier-inventory-20260727.md` §1.3）。生产侧 `p123_fast_replay.rs` 导入清单（170-176 行）**不含**本函数名——p123 实走单 run 通道 `provide_active_pan_live_windows`（`nest_lifecycle.rs:1803`，`PanLiveOutcome` typed 返回），已确认production wiring。函数自身 doc 注释原文即承认「生产逐 bar sidecar…独立发现结构窗」——这句话本身就是「本函数不是生产接缝」的自白，只是未挑明。已被 `shadow-421-whole-20260728.md` S-H2、`shadow-527-review-20260728.md` T-3 两份独立评审登记为「生产零调用方却保留，与同 diff 删除标准不一致」。 |
| `PanLiveRun<'a>`（struct） | `nest_lifecycle.rs:2270` | **5 删**（随上一件同批次） | 唯一非测试引用是作为 `provide_replay_live_windows` 的参数类型（其本身即测试可达）；另 11 处构造点（`nest_lifecycle.rs:3939` 起）全部位于同一 `#[cfg(test)] mod tests` 内。与 `provide_replay_live_windows` 同生共死——单独保留此 struct 而删函数没有意义（零其他消费者）。 |
| `leg_as_segment_copy`（私有 fn） | `nest_lifecycle.rs:2254` | **5 删**（随上两件同批次，需订正既往「授权保留」判定） | 唯一调用点是 `provide_replay_live_windows` 内部（`nest_lifecycle.rs:2290`）。既往三份评审（`issue421-acceptance-selfcheck-20260727.md:50/203`、`shadow-430-rereview-20260728.md:286/298`、`shadow-429-rereview-20260728.md:187/285`）均将其登记为「#421/#426 明示授权的接线侧复制，LOW severity，保留不动」——**该判定的前提是这份复制服务于 #426 原定的生产接线（`provide_replay_live_windows` 曾被设计为回放主接缝）**。但生产侧实际腿转段消费方是 `p123_fast_replay.rs:1735` 的独立函数 `lifecycle_leg_as_segment`（同构逻辑的另一份独立复制，非本函数），二者互不调用。既然服务对象（`provide_replay_live_windows`）本身测试可达，「保留此复制不提升可见性」这条授权理由已经悬空——授权保护的是「不要为它去提升 `level_view.rs:436` 私有函数的可见性」，不是「保留一个已无生产受益者的复制体」。建议**随上两件一并订正为删档**，既往「保留」判定视为在缺失「其服务对象已死」这一事实前提下做出，需订正。 |

**连带说明**：三件互为依赖闭包（`PanLiveRun`/`leg_as_segment_copy` 均只服务 `provide_replay_live_windows`），删除必须同批次进行，不可只删其一。孪生的 `ReplayPrefixFeed`（旧回放喂数结构）已在此前提交中删除，三件未同批次跟删是 `shadow-421-whole-20260728.md` S-H2 指出的「新旧权威未收敛」现场——本票定档建议正是该 S-H2 的收口路径。

**本轮已做（非删除、文档对齐）**：三件的模块头/doc 注释已订正为明确标注「仅测试可达」「生产走 `provide_active_pan_live_windows`」，见 `nest_lifecycle.rs:2248-2256`（模块头段落）、`:2268-2269`（`PanLiveRun` doc）、`:2280-2284`（`provide_replay_live_windows` doc）。原「回放喂数出口（票 #426 **主接缝**）」措辞（暗示这是生产主路径）订正为「早期适配层」并附全量调用点核实结论。

---

## 2. p123 两长函数拆分映射

### 2.1 `main()`（原 651-1028 行，378 行 → 现 52 行）

原函数是纯顺序的「计算→打印」流水线，按打印分组边界切出 12 个助手函数（均为纯搬迁，零逻辑改动）：

| 原行段（HEAD `9e3cd25fd6`） | 新函数 |
|---|---|
| 652-667（参数解析+加载） | `parse_cli_and_load` |
| 669-684（启动横幅四行打印） | `print_startup_banners` |
| 686-701（terminal pass + 候选快照） | `collect_terminal_snapshot` + `build_target_map` |
| 702-738（targeted prefix pass + sparse/lifecycle 摘要打印） | `run_prefix_pass_and_report`（内部再调 `print_sparse_summary`/`print_lifecycle_summary`/`print_lifetime_summary`） |
| 800-822（judge_at 回填 + observe_snapshot） | `finalize_judge_at` |
| 823-868（全量分类对拍计数 + 逐级 diff 打印） | `compute_bit_diff`（`BitDiffCounts` 结构体）+ `print_bit_diff_levels` |
| 869-912（旧路径语义候选/终端/证书计数） | `compute_old_semantic_counts`（`OldSemanticCounts`）+ `compute_old_certificates` |
| 914-965（P123_YIELD/CERT/D3 三行打印） | `print_yield_cert_d3` |
| 958-981（BASELINE/PROVIDER/SNAPSHOT 三行打印） | `print_baseline_and_provider` |
| 982-995（BIT_EXACT/R7 两行打印） | `print_bit_exact_and_r7` |
| 996-1025（遗漏对账枚举打印） | `report_missed_reconciliation` |

### 2.2 `run_targeted_prefix_pass()`（原 1230-1835 行，606 行 → 现约 60 行）

原函数是单个巨型逐 bar 循环，~20 个局部可变变量贯穿全程。拆分方案：把全部跨 bar 状态收进 `TargetedPassState<'c>` 结构体（字段与原变量一一对应，纯状态载体迁移，判据/口径零改动），循环体按阶段切成独立函数：

| 阶段 | 新函数 |
|---|---|
| 状态初始化 | `TargetedPassState::new` |
| 逐 bar 编排入口 | `process_targeted_bar`（被 `run_targeted_prefix_pass` 主循环调用） |
| L1/L2/L3 重算判据键 | `compute_lifecycle_due_keys`（`DueKeys`，不携带借用面，避免跨调用借用冲突） |
| 活窗结构重算 + 落盘 | `recompute_stems_if_due` → `recompute_and_merge_stems` → `merge_lifecycle_stems` |
| 段账本完整性观测 | `record_seg_ledger_observation`（`SegLedgerObservation`） |
| 诊断行落盘（PAN_LIVE_HIT/MISS、PAN_LIVE_RECOMPUTE） | `write_pan_live_diag_rows` / `write_pan_live_recompute_line` |
| targeted trigger 分组/账本值比对/逐级求值 | `evaluate_and_apply_targeted_trigger` → `group_pending_runs_by_level` / `refresh_bsp_snapshots` / `evaluate_level_runs` → `evaluate_single_run` → `compute_watermark_crossed` / `compute_dirty` / `reevaluate_run`（→ `get_or_insert_run_entry` / `call_evaluate_run` / `record_reevaluation_result`） / `apply_run_events_and_shadow_check`（→ `shadow_check_run`） |
| 事件应用（DIV/TERM 落账） | `apply_targeted_events` → `record_term_signal` |
| lifecycle sidecar 喂数 + checkpoint + 进度打印 | `feed_lifecycle_and_checkpoint` → `refresh_lifecycle_if_due` / `feed_lifecycle_bar_and_dump` / `run_periodic_checkpoint` / `print_prefix_progress` |
| 循环后收口 | `finalize_targeted_pass` → `write_lifetime_dump_line` |

**关键工程决策（须知，防未来重构误判为 bug）**：`TowerCache` 的四个访问器（`causal_series`/`macd_dif`/`level_scan_units`/`level_scan_cursor`/`tower_confirmed_len`/`freeze_boundary`）在多个助手函数中被**重复调用**而非一次计算后跨函数传递——这是 Rust 借用检查器的结构性要求（`&mut TargetedPassState` 作为整体传参时，不能与源自 `state.cache` 字段的借用同时存活），不是性能疏漏：这些访问器全部是 `&self` 纯读取（无副作用、无重算），`cache` 在同一 bar 内两次调用之间不会被任何路径改写，故重复调用与保留一份传递值行为完全等价。

---

## 3. 措辞订正三处

### 3.1 死路径三件文档对齐（本轮已落地，见 §1 尾段）

### 3.2 `#421` 票体正文 + `#427` 条款一（**GitHub issue 活文本未订正，待编排者授权过账**）

**发现**：`shadow-421-whole-20260728.md` P-M1 已精确指出——票体正文「账本只在回放引擎的重估触发点喂」「时钟精度 = 触发点粒度」与 `#427` 条款一原文，均与 2026-07-28 逃生门裁定（`gh api .../issues/comments/5103071677`）授权的「独立逐 bar 喂数循环」实装口径**不一致**；代码侧（`nest_lifecycle.rs:81-88` 模块头「feed 契约（#421 逃生门）」段落）已在 commit `58672e2563`/`77d3554332` 订正为「逐 bar」表述，**唯独两张 GitHub issue 自身的正文/验收条款未同步**。

**为何本轮未直接编辑**：编辑/评论 GitHub issue 是外部共享系统上的可见动作（区别于本地 git commit），本票的 git mutation 授权只覆盖本地仓库 commit，不含 GitHub API 写操作；按会话安全默认（可见于他人/半不可逆动作先确认），本轮不擅自过账，改为交付可直接采用的订正稿：

> **建议评论稿（可贴至 #421 与 #427）**：
> 「本票（#421/#427）正文「账本只在回放引擎的重估触发点喂」「时钟精度 = 触发点粒度」的表述，已被 2026-07-28 逃生门裁定（comment-5103071677）授权的『独立逐 bar 喂数循环』方案取代——sidecar 实际逐 bar 喂数，触发点粒度仅适用于结构重算，非喂数节拍本身。代码侧模块头（`nest_lifecycle.rs:81-88`）已在 `58672e2563`/`77d3554332` 同步订正；本评论用于对齐票体正文，不改变已交付行为（生产订单流逐字节不变，见 #429/#430 三审字节护栏）。」

### 3.3 `IdentityVanished = 0` 验收句失效订正

**核实结论：已完成，落地时间早于本票开工**。`nest_lifecycle.rs:71-80`（模块头「身份消失不变量」段落，标注「票 #559 编排者裁定 2026-07-28」）已明确撤销旧「`IdentityVanished = 0`」不变量断言，替换为拆两类原因码的新不变量。全库检索确认：

- 仅有的其他「IdentityVanished」提及均在**带日期的历史评审报告**（`issue421-acceptance-selfcheck-20260727.md`、`shadow-*-20260728.md` 等）与 **roster 时间线日志**（`agent-roster-2026-07-21.md`）——这些是生成史快照（记录「当时观测到 IdentityVanished=N」的事实），roster 自身第 28 行已记录后续「撤 =0 不变量」的裁定条目，属正常谱系推进，**不应改写**（改写历史记录 = 压扁谱系，违反 `git-workflow.md`「谱系优先于汇总」）。
- 未发现任何仍在断言「IdentityVanished=0 为当前有效不变量」的活口径文档（spec/ADR/README 类）。

本条判定为**已达标，无需本轮再动**。

---

## 4. 验证证据

### 4.1 编译

```
cargo build --release --bin p123_fast_replay   # 干净，零错误、零新增警告
```

### 4.2 全量测试指纹（开工态 = 收口态，逐字一致）

`cargo test --release --lib`：**2130 passed; 1 failed; 137 ignored**——唯一失败 `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`，对应 issue #491（开放中，票面既知基线红，非本票引入；核实其失败原因是 `BspPoint` 字段随其他并行工位改动而 Debug 摘要漂移，与本票改动的两个文件（`p123_fast_replay.rs`/`nest_lifecycle.rs`）无关）。

> 注：开工首次尝试 `cargo test --lib` 时因并行工位 #621 的 `retrace_ledger/` 模块尚在提交中（`book.rs`/`log.rs` 的 `PartialEq`-on-`Result` 编译错误），暂时无法编译——这是**环境态**而非本票代码问题；待 #621 落地提交（`9e44d98728`）后重跑即恢复干净指纹，与本票改动无关。

### 4.3 #533 字节护栏（票内强制门）

```
cargo test --release --test issue533_p123_byte_guardrail -- --ignored --nocapture
# p123_replay_2000bars_byte_exact ... ok
# p123_replay_large_windows_hash_guardrail ... ok
```

### 4.4 p123 20k/100k 手工对拍（开工基线 vs 收口态）

- stdout：20k/100k 均逐字节零 diff（`diff` 无输出）。
- stderr：逐字节零 diff，**唯一例外**是 `P123_SPARSE_SUMMARY` 行的 `prefix_s=` 计时字段（wall-clock 耗时，非确定性，两次运行本就不同，不计入行为漂移）。
- 基线文件留存 `/tmp/issue605-baseline/`（`p123_{20k,100k}_std{out,err}.{before,final}.txt`）。

### 4.5 长函数标准核对

`main()`：52 行；`run_targeted_prefix_pass`：约 60 行（含 `TargetedPassState` 定义与 `impl` 块，纯函数体循环编排部分约 20 行）；拆出的全部 27 个新助手函数/关联函数逐一核对 ≤50 行（脚本核验，见工作记录）。仓内既有的其他超长函数（`recompute_lifecycle_window_stems` 277 行、`evaluate_run` 54 行、`collect_snapshot_candidates` 81 行、`observe_snapshot` 97 行、`observe_certificates` 72 行、两个 `l2/l3_..._watermark` 测试函数）均为**票 #532 覆盖面**，本票不动。

---

## 5. 收口输出

1. **commit SHA 列表**：见提交历史（本报告随代码同批或紧随提交，commit message 前缀 `refactor(theta): #605 …`）。
2. **死路径三件定档建议表**：见 §1（呈编排者终审，本轮零删除）。
3. **验证证据**：见 §4（指纹/失败集对照、#533 门、20k/100k 对拍）。
4. **长函数拆分映射**：见 §2。
5. **措辞订正三处落位**：见 §3（死路径文档已落地；#421/#427 GitHub 正文待编排者授权后过账；`IdentityVanished=0` 核实已提前完成，无需本轮动作）。
6. **偏离/存疑**：
   - **⚠️ 高优先级**：47 个 `chanlun/review-results/`/`chanlun/escalate/` 追踪文件在共享工作树上物理缺失（非本票所为），已记录清单，未做任何恢复操作，留编排者裁定（详见 §0 第 4 条）。
   - #421/#427 的 GitHub issue 正文订正未直接过账，已备好可直接采用的评论稿（§3.2），需编排者一句话授权即可执行。
