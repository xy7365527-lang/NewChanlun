# 影子评审：#624 T3 验收环（对拍 + golden 锚 + 消费方登记 + 旧模块处置）

> 日期：2026-07-29
> 评审车：claude（opus），新上下文独立评审，**未参与 #624 实装**
> 工位：worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`，HEAD `16417473cb`
> 约束：只读 + 可跑测试；零源文件修改；零 git mutation；单线程（无子代理/后台任务）
> 对象 commit 链：`3180f8f446` → `148c814312` → `66cce932dc` → `8b8905def2` → `1eb76f104b` → `16417473cb`

## 结论（先行）

**PASS with findings：0 HIGH / 2 MEDIUM / 3 LOW。**

四件交付的**技术面全部实测复现通过**：指纹 2197/0/138 与报告逐位一致；全量 `--no-fail-fast` 零红；
`compile_fail` doctest 生效；对拍 9 条 fixtures 的「输入/旧输出」两列**逐条回到删除前源码核对，无一条追述失实**；
golden 三层锚（字节/指纹/11 条逐条篡改）真跑真断言；删除段无残留、迁入类型 derive 与字段逐字未改；
消费方 0/3 的 grep 实证成立且三方实体在仓内均真实存在。

两条 MEDIUM 都不是行为缺陷，是**验收声明与其证据之间的缝**：
① F7/F7′ 两条对拍没有复刻旧 fixture 的「同一中枢」输入，报告未声明这一偏离；
② 票面 AC「编排者终审记录在票」尚未兑现（票上零评论），实施车已如实上报。

## 一 · 独立复现实测（逐项照实报数）

开工 `git status` 自核：脏面 3 改 3 新，全部属并行线（`issue571-*.md` / `treasury-*.json` /
`p100_cert_bsp_recon.rs` + 三份 review-results 新文件），与 #624 面零交叉，全程未碰。

| 项 | 命令 | 实测 | 报告声明 | 判定 |
|---|---|---|---|---|
| lib 指纹 | `cargo test --lib` | **2197 passed / 0 failed / 138 ignored**（6.46s） | 2197/0/138 | ✓ 一致 |
| 全量零红 | `cargo test --no-fail-fast` | 全 target `test result: ok`，`FAILED` 计数 **0** | 零红 | ✓ 一致 |
| doctest | `cargo test --doc retrace_ledger` | `portal.rs:51 StandbyWatch - compile fail ... ok`，1 passed | compile_fail 生效 | ✓ 一致 |
| 全 target 编译 | `cargo check --all-targets` | Finished（1m13s），零错误 | 通过 | ✓ 一致 |
| golden 文件 | `wc -lc` | **12 行 / 3608 字节**，11 条 revision + 1 行 provenance 头 | 12/3608/11 | ✓ 一致 |
| 旧模块规模 | `git show 8b8905def2^:…` | **572 行 / 9 条 `#[test]`** | 572 行 / 9 条 | ✓ 一致 |
| 新增测试 | `grep -c '#[test]'` | `replay_parity.rs` 5 + `golden_log.rs` 6（含 1 `#[ignore]`） | +11 新增 / ignored +1 | ✓ 自洽（5+5 活测 + F1 迁入 1 = 11；2195+11−9=2197） |

## 二 · 对拍独立抽验（≥3 条，回删除前源码逐条核）

抽 F3 / F4 / F5 / F6 / F7 / F8 六条（超过要求的 3 条），逐条把报告 §一 基线表的「输入」「旧输出」
两列与 `git show 8b8905def2^:rust/src/theta_v0/classifier/first_retrace_replay.rs` 的测试体对照：

| # | 报告登记的旧输出 | 删除前源码实测 | 判定 |
|---|---|---|---|
| F3 | `Err(StrictPairError::NoCompletedMove(id(273)))` | 同（`lv_case2_first_pair_has_no_completed_pair_at_first_judge_time` 末断言） | ✓ 如实 |
| F4 | `Err(StrictPairError::SameMove(0))` | 同 | ✓ 如实 |
| F5 | `Err(StrictPairError::MissingSource(id(277)))` | 同 | ✓ 如实 |
| F6 | `Err(RetraceLifecycleError::FirstRetraceConsumed(identity))` | 同 | ✓ 如实 |
| F7 | `Ok([RetestReenters{old,4}, Supersede{old}, Restart{new}, Success{new,6}])` | 同（事件序逐位一致） | ✓ 如实 |
| F8 | `Ok(vec![])` | 同 | ✓ 如实 |

**语义映射表 15 项抽验**（报告 §二）：

- `StrictCompletedPair` / `RetraceOutcome` 标「迁入，一个 bit 不改」——核 `retrace_ledger/mod.rs:107-122`
  与旧 `first_retrace_replay.rs:11-14 / 96-100`：derive 均为 `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`，
  字段名/类型/顺序逐字相同（新增 doc 注释，字段本体零改）。✓
- `StrictPairError::{SameMove, NotAdjacent}` → `RetraceRejection::NotAdjacent` 标「两码合一，判据同为
  `retest == leave + 1`」——核 `adapter.rs:171-183`：新判据确为 `retest_move_index != leave_move_index + 1`，
  `leave == retest` 自然落进该码。✓
- `FirstRetraceConsumed` → 静默吸收 + `late_absorbed` + audit `LateAbsorbed`——核 `book.rs:410-418`
  （`LedgerAdmission::TerminalAbsorbed` 分支同时自增计数与 push audit）。✓
- `Supersede` 标「无事件名，退位由终态本身承载」——核 `book.rs:264-266`：前档已终态时
  `guard_single_active` 直接 `Ok(None)`，新档由 `link_restart`（`book.rs:363-374`）挂
  `Restarted{previous}`。✓ 结构性成立。
- `PairDepartureMismatch` → `ActiveCandidateNotSettled`——核 `book.rs:267-283`：判据同构
  （departure 不等且前档未终态）。✓

## 三 · golden 锚（②）

| 层 | 落点 | 独立核验 |
|---|---|---|
| 字节 | `golden_log.rs:109-124` | `externalize()` 经 `JsonlRetraceLogStore::append_all` 写临时文件再读回，与仓内 golden 全串比较——**真字节**，不是结构比较。✓ |
| 指纹 | `golden_log.rs:130-148` | `GOLDEN_DIGEST = 0x808b_849b_9ba1_d112` 钉常量；篡改测 `for index in 0..live.journal().len()` 对 **11 条各改一次 `as_of`** 断言 `assert_ne`——真跑真循环，非声明。✓ |
| 重放 | `golden_log.rs:154-183` | 从磁盘经 `JsonlRetraceLogStore::load()` → `RetraceLedger::fold()`，逐条比对三态/留档/出生钟/落锤钟/Restart 谱系/判败原因码 + `established()` + `short_retrace_records()`，末尾 `settled()` 跑全量 `assert_invariants()`。✓ |

**六场景覆盖核对**（`golden_log.rs:75-92` 剧本 vs 报告 §五表）：改口 ✓（`reconcile_window` 显式核对通道，
`as_of=800`）、判胜 ✓（`as_of=900`）、判败 ✓（`as_of=600`）、Restart ✓（sequence 5 与 9 两条
`restarted` 载荷，golden 文件可见）、迟到吸收 ✓（`as_of=1000`）、死人挂号 ✓（`as_of=1100`，
`unwrap_err()`）。日志条数手算 2+1+3+1+4 = **11** 与 `GOLDEN_RECORD_COUNT` 一致。

**「迟到 + 死人挂号走 audit 流」的裁定六口径声明——如实。** `golden_scenario_covers_the_two_audit_only_faces`
（`golden_log.rs:227-247`）从**两侧**正面钉死：警报计数（`late_absorbed==1` / `dead_center_registrations==1`）
+ audit 事件存在性（`LateAbsorbed` / `DeadCenterRejected`）。二者确实不在 golden 字节里（我逐行读了
fixture，11 条 revision 无一条对应这两拍）——这是裁定六「未被采纳的输入不进真相路径」的**可观测证据**，
报告称其「不是覆盖缺口」的说法成立。

## 四 · 删除段审计（④）

- **删除面**：`git show 8b8905def2 --stat` = `first_retrace_replay.rs` −572 / `classifier/mod.rs` −3
  （2 行 doc + 1 行 `pub mod`）+ 迁入侧 3 文件净 +38。**未删任何其他文件。** ✓ 与报告一致。
- **全仓残留**：`grep -rn first_retrace_replay --include=*.rs` → `src/` 内**零命中**；`.md` 侧命中全在
  spec 与历史 review-results（合法的历史引用）。`level_view/tests/projection_pairing.rs:41` 有一处
  注释性出处标注（F1 迁入说明），非代码引用。✓
- **`closed_loop` 同名字段豁免核实**：`closed_loop/{buy,sell}.rs` 的 `first_retrace: bool` 是 Lean
  `e.bsp.firstRetrace` 的镜像字段，与被删模块无类型/无 import 关系，未碰。✓
- **迁入逐字对照**：见 §二（derive/字段一个 bit 不改）。✓
- **F1 迁入逐字性**：`git show 3180f8f446 -- …/projection_pairing.rs` 的四条断言与旧
  `lv_case2_auto_pairing_tuple_keeps_all_three_versions_pinned` **逐字相同**（仅去 `lv_case2_` 前缀 +
  加迁入出处 doc）。✓
- **删除后全绿**：本轮 HEAD 实测 2197/0/138 + 全量零红（§一）。✓

## 五 · 不覆盖 2（F3/F5）的登记诚实性（⑤）

报告 §四 C **诚实，无粉饰**。三点逐条核实：

1. 「仓内不再有任何一处执行『seed 唯一归属 + 均 Completed』这条 fail-closed 证明」——
   我独立 grep 了 `level_view/` 与全仓 `center_indices` 消费点：`level_view` 侧确无该证明；
   `p83_yield_remeasure.rs:383-393` 有一个**部分同构的统计面**（同一 `center_indices.contains(seed_index)`
   判据），但**无 Completed 过滤、无唯一性检查、非 fail-closed**（只累加 `unassigned_projected` 计数）。
   故报告断言成立（见 LOW-2：该近邻实现未在缺口登记里提及）。
2. 「旧模块零生产调用点故删除不改现有生产行为」——删除前 4 处 `.rs` 引用全在 `retrace_ledger` 内
   （`adapter.rs` / `mod.rs` / `tests/mod.rs`），diff 可证。✓
3. 「无未测代码残留，非删测试保绿」——被测函数 `strict_completed_pair` / `unique_completed_move`
   与其测试 F3/F5 同刀删除。✓ 这一条是本次删除段最容易出事的地方，实施车没走捷径。

三个纯净随删码（`MissingSource` / `NoCompletedMove` / `AmbiguousCompletedMove`）+ 第四个部分并入的
`SameMove`——报告 §四 C 写「四个……（后者部分并入 `NotAdjacent`）」，措辞准确。
（旁注：`AmbiguousCompletedMove` 分支在旧模块 9 条 fixtures 里本就零覆盖，属旧模块的既有缺口，不是本票新增。）

## 六 · 消费方登记（③）

- **grep 实证**：`grep -rn "retrace_ledger" --include=*.rs src/` 排除模块自身目录后
  **唯一命中 `src/theta_v0/classifier/mod.rs:89`**（模块注册行）。零生产引用，与「0/3」一致。✓
  （注：报告写 `mod.rs:92`，删 3 行注册后已位移到 89 —— 见 LOW-1。）
- **三方落名实体核实**（登记表不能指向不存在的消费方）：
  `crate::trading::center_book::CenterBook` → `src/trading/center_book.rs:36` **存在** ✓；
  `signal::locate_pan_div_structure` → `src/theta_v0/classifier/signal.rs:655` **存在** ✓；
  交易层 `crate::trading` 线 **存在** ✓。
- **登记表落码**：`retrace_ledger/mod.rs:24-38` §消费方登记，三档 + 2′ 备战面共四行，每行带消费面 /
  消费方 / **未接线**标注 / 接线票或缺口，末行显式写「生产接线点计数（登记时点 2026-07-29）：0/3」。
  与报告 §六表逐行一致。✓

## 七 · Findings

### MEDIUM-1 · F7/F7′ 对拍未复刻旧 fixture 的「同一中枢」输入，且未声明该偏离

- **证据**：旧 `lv_case2_new_departure_supersedes_then_restarts_with_new_identity`
  （`first_retrace_replay.rs` 删除前测试体）中 `new = RetraceIdentity { center: old.center, departure_move_index: 5 }`
  ——新旧身份**共享同一 center**；自动机的 Restart 分支（同文件 `replay_first_retrace` 内
  `active = RetraceIdentity { center: active.center, … }`）也明写 center 不变。
  新对拍 `replay_parity.rs:88,93`（F7）与 `:129,133`（F7′）用的是 `frame(1_200)` → **`frame(1_400)`**，
  即两个 `end_index` 不同的 `CenterFrame`。
- **失效情景**：报告 §三把 F7/F7′ 判为 **PASS**，§一 基线表的「输入」列登记的是旧 fixture 的输入，
  两表之间隐含「同输入」——但落测实际换了框。当前不产生错误结论（我静态验证过同框路径同构：
  `book.rs:264-266` 前档终态即 `Ok(None)`，与 frame 无关；`link_restart`（`book.rs:369`）按
  `anchor = start_index`（`mod.rs:196-198`）路由，`frame(1_200)` 与 `frame(1_400)` 同锚），但对拍强度
  低于报告给人的印象：**同框 + 新 departure 这条恰是旧 fixture 的输入面，一条测试都没跑**。
  若将来 `guard_single_active` 的分支序改动（例如先比 frame 再比 departure），这两条仍绿而旧语义已漂移。
- **可辩护的一面（如实记入）**：新身份含「临时右边」（裁定一），departure 从 3 走到 5 期间中枢右边
  通常确实会变，用 `frame(1_400)` 比旧的「center 恒定」更贴近真实语义。报告 §四 B-5 已登记
  `CoordinateWindow → CenterFrame` 的身份加强，但**没有把这条与 F7 对拍输入的偏离连起来**。
- **建议**：报告 §三 F7/F7′ 两行补一句输入面偏离声明（「新身份含临时右边 ⟹ Restart 场景框必变，
  对拍以同锚异框替代旧的同框」），或补一条同框（`frame(1_200)` → `frame(1_200)`, dep 3→5）的对拍测试。
  二选一即可，不必都做。

### MEDIUM-2 · 票面 AC「旧模块处置：编排者终审记录在票」未兑现

- **证据**：`gh issue view 624` → `comments: 0`。编排者 2026-07-29 的「A（5 删）」终审目前只存在于
  报告 §七、commit `8b8905def2` 的 message 与 dispatch log 里，**票上零记录**。
- **失效情景**：#624 AC 第 4 条明写「旧模块处置：编排者终审记录在票」，第 5 条「两轴 + 影子评审票」。
  按当前状态，5 条 AC 中 3 条闭合（对拍报告 / golden 锚 / 消费方登记表），2 条待落。
  #225 先例设这条 AC 的用意正是「删除不得只靠执行侧自述」——报告与 commit message 都是执行侧文书，
  不构成票面授权痕迹。
- **归属**：实施车已在 `/tmp/issue624-dispatch-20260729.log` §6 显式声明「未做（不在执行车授权内）」，
  **不是隐瞒**。本条是编排者侧的落字动作 + 本影子评审票的入库，非实装缺陷。

### LOW-1 · 报告两处事实性数字陈旧

- `§六`（报告 178-179 行）：`src/theta_v0/classifier/mod.rs:92` → 实测 **`mod.rs:89`**（删 3 行注册后位移）。
  结论（唯一命中 / 0-3）不变。
- `§八`（报告 214 行）：`golden_log.rs` **269 行** → 实测 **268 行**（`1eb76f104b` 拆函数后未回填）。
  ≤800 硬杠不受影响。
- 其余 Standards 数字实测复现：最长新函数 **34 行**（`f7_new_departure_after_reentry_…`）/ 33 行
  （`golden_scenario_covers_the_four_faces_…`），`retrace_ledger/mod.rs` **521 行**——与报告一致。✓

### LOW-2 · §四 C 缺口登记未提 `p83_yield_remeasure.rs` 的近邻统计实现

- **证据**：`p83_yield_remeasure.rs:383-393` 的 `unassigned_projected` 用与旧 `unique_completed_move`
  同一判据（`view.moves.iter().any(|v| v.center_indices.contains(seed_index))`）；
  `c2-yield-remeasure-20260715.md:51` 早有在案记录「这与既有 D7 原语一致」。
- **为什么算 LOW 而非 MEDIUM**：它**不构成对报告断言的反证**——p83 那段既无 Completed 过滤、
  也无唯一性（Ambiguous）检查、更不 fail-closed，只累加统计计数。报告说「不再有任何一处执行这条
  **fail-closed 证明**」仍然成立。
- **为什么仍值得记**：将来在 provider 侧补回该证明时，p83 是现成参照，也是**潜在的双实现点**
  （统计口径与 fail-closed 口径分家漂移的风险位）。缺口登记里点一句，接线票能少走一段路。

### LOW-3 · 指纹「任一字段被改写即红」的声明强于其验证维度

- **证据**：`golden_log.rs:17` 模块头写「任一字段被改写即红」；篡改测（`:139-147`）只逐条改
  `revision.as_of` 一维。
- **结构上声明成立**：`digest_records`（`log.rs:457-466`）是 FNV-1a over `encode_record` 的完整 JSON，
  而 `WireRecord`（`log.rs`）覆盖 `sequence` / `key` / `kind` / `as_of` / `evidence` 全字段 —— 任一字段
  变确实改指纹。故这是**验证维度单一**，不是声明失实。报告 §五 措辞（「对 11 条记录各改一次 `as_of`」）
  比模块头更准确。

## 八 · 未发现的问题（正面登记，避免「没说 = 没查」）

以下面均已查、未发现问题，如实登记以界定本评审的有效域：

- 迁入类型的 derive / 字段漂移：**无**；
- 删除段越界（删了授权外的文件）：**无**（F1 迁入是**新增** 24 行到 `projection_pairing.rs`，
  实施车已在 dispatch log §6 主动声明这一「授权范围外的文件改动」——如实上报，我确认它确为新增非删除）；
- golden 为变绿而被静默重生成：**无迹象**（变更纪律写进模块头 `:22-36`，重生成入口 `#[ignore]` 隔离，
  digest 钉常量，本轮 HEAD 实测三层锚全绿）；
- 消费方登记虚构落名：**无**（三方实体全部实存，见 §六）；
- 测试计数造假：**无**（2195→2197 的 +11/−9 分解可逐条对上）；
- `ledger_kernel/` 与 `nest_lifecycle.rs` 逻辑面：**未碰**（本评审按约定不进该面，亦确认
  `8b8905def2` 的 5 文件 diff 与之零交叉）。

## 结果包六要素

1. **结论**：#624 T3 验收环 **PASS with findings（0 HIGH / 2 MEDIUM / 3 LOW）**。四件交付的技术面
   全部独立复现通过；两条 MEDIUM 分别是对拍输入面的未声明偏离（MEDIUM-1）与票面 AC 未闭合
   （MEDIUM-2），均非行为缺陷，不阻塞删除段本身的正确性。
2. **定义依据**：#624 票面 5 条 AC；spec `spec-kernel-and-retrace-ledger-20260728.md` §T3 章
   「`first_retrace_replay.rs` 处置随票落（替换/删除须编排者终审，#225 先例）；其 fixtures 转作对拍基线
   （行为等价面：严格 pair 映射、消费一次性、Supersede/Restart 纪律）」+ §测试决策「对拍
   `first_retrace_replay` 现有 fixtures 行为等价面；golden 事件日志锚」；#575 验收「无消费方不接生产」；
   #574 契约裁定一/二/三/六。判定输入 = 本轮实测的测试指纹、golden 字节、删除前源码逐条对照、
   全仓 grep 实证。
3. **边界条件**（本结论翻转条件）：
   - 若判定「对拍必须逐位复刻旧 fixture 输入」为硬要求（而非「行为等价面对拍」），MEDIUM-1 升级为
     HIGH，F7/F7′ 两行的 PASS 判定须撤回重测；
   - 若判定「编排者终审记录在票」是删除段的**前置**授权条件（而非事后落字），MEDIUM-2 升级为
     阻塞项，`8b8905def2` 须待票面落字后才算授权完备；
   - 若 §四 C 的「seed → 唯一 CompletedMove 映射归 provider 职责」被推翻（改判归账本入口），
     则不覆盖 2 从「已知代价」升级为缺口，裁定 A 须回退为替换——这一条与报告 §四 C 自述的翻转
     条件一致，我复核后认同其判断。
4. **下游推论**：
   - 三消费方接线票可直接以本票 golden（11 条修订 / digest `0x808b849b9ba1d112`）作回归锚——
     接线不得改变这些字节；本评审确认该锚的三层证据链真实可执行；
   - provider 侧接真实 `level_view` 时须补回 §四 C 登记的映射证明，补时应同时处理
     `p83_yield_remeasure.rs` 的近邻统计实现（LOW-2），避免双口径分家；
   - `retrace_ledger::{StrictCompletedPair, RetraceOutcome}` 现为仓内该语义**唯一**定义点，已实证
     （全仓 grep 零残留）。
5. **谱系引用**：#465 裁定 A（抽内核 T1 + 建账本 T3）；#574 八条裁定（本评审据裁定三/六核对
   F6/Supersede 的「有意改变」是否有裁定背书——有）；#225 先例（删除须编排者终审，MEDIUM-2 的依据）；
   孤儿盘点 `rust-orphan-five-tier-inventory-20260727.md` §第 4 件（`ext_raw_refs = []`，裁定 A 的事实前提，
   本轮 grep 复核成立）；`c2-yield-remeasure-20260715.md:51`（LOW-2 的在案记录）。
   **概念分离复核**：报告称 `Supersede` 事件名的消失是「瞬态自动机 → 持久账本」范式切换的必然推论
   而非语义丢失——我从 `book.rs:264-266 / 363-374` 独立验证该推论成立（退位由前档终态本身承载，
   新代谱系由 `Restarted{previous}` 承载，两处信息之和 ⊇ 旧 `Supersede` 事件所载信息）。
6. **影响声明**：本评审**零源文件修改、零 git mutation**，仅新增本报告一个文件
   （`chanlun/review-results/shadow-624-t3-acceptance-review-20260729.md`）。跑过的命令均为只读或
   构建/测试（`cargo test --lib` / `--no-fail-fast` / `--doc` / `cargo check --all-targets`），
   产物落 `target/`，不影响仓内受控文件。并行线未提交面（`issue571-*` / `treasury-*` /
   `p100_cert_bsp_recon.rs` 等 6 项）全程未碰。
