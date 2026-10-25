> 正本：本文件是级别同构对拍锁与严格判据不兜底锁协议的正本。
> 最后裁定：#1531（2026-10-22）
> 受影响代码清单：见「范式实例」节

## 问题

#1531（FU-09）要裁三件事。教义在 #799 收敛通则和 #804 总缝规则里已经定了，本票只补它们没写的**机械锁协议**，不重裁教义：

- **Q1 同输入换级别逐位相同的完整证明**：对拍锁写成什么样才算证明？「完整」的边界在哪里？
- **Q2 严格判据失败不得宽松兜底的机械锁**：#799 只给了判别式「看两套判据是不是接在同一个 if-else 上」（AGENTS.md:58），没给测试形态。
- **Q3 CI 实际跑对拍的实施路径**：#799 裁定十一要求「只编译不执行的锁按无锁计」。锁放在哪里、满足什么条件，才算被 CI 执行？

本票是纯文档裁定。锁的代码实装交给下游实施票，不作关本票条件。先例是同组 FU-08 的 #1530 resolution（评论 6281488966「不作关本票条件」）和 ADR 0039:353 的 B 段。

**考古标出的冲突（逐条写明，正文按此处置）**：

1. **正本与代码不符（Q1）**：AGENTS.md:107 仍写「恒等性的对拍测试锁尚缺（#1278 ⑧列 ❌）」。但这把锁 `center_segments_window_identity_on_l0_domain` 已在 main 上（center.rs:529-531），#1294 已 CLOSED，报告 issue1294-lock-network-batch1.md:44 自称「本锁补齐其缺失的测试锁载体（sliceA ⑧❌ 收口）」。处置：ADR 0047 写明 AGENTS.md:107 这半句已过时，以本 ADR 为准。AGENTS.md 不在本票正本落点内，本票不改它。
2. **CI 定位漂移（Q3/Q4）**：#1294 报告 :10 写锁在「CI `rust-check` job 的 `cargo test --all-targets`」里执行。现行 ci.yml 已在 #1588 拆分 job（ci.yml:143-150 注释），默认目标的 `cargo test` 现在属于 `rust-test-default` job（:239），步名「cargo test — default targets」（:256-257）。另外 AGENTS.md:80 仍把「先让 CI 跑 `cargo test`」写成待做的前置，而承接它的 #810 已 CLOSED。处置：这两处只是引用位置的订正，没有触及 #799 裁定十一的已裁原则本身。Q3 仍按考古 A 类**直接沿用**；本票只把 CI 面的引用改按 **job 名和步名**，不按行号，并注明 AGENTS.md:80 的前置已由 #810 完成。
3. **台账失踪（Q7）**：结构判据.md:28-33「明细」列指向的 issue1278-slice*.md，在 main 上 `ls .chanlun/review-results/ | grep -c 1278` 为 0。:33 写「追加 #1302 在办」，实际 #1302 已 CLOSED。处置：登记处沿用 ⑧ 列（:20），不依赖失踪的 sliceA 明细文件。
4. **ADR 编号（上一稿 R3 BLOCK）**：上一稿验收命令里的编号是字面占位，而且没写替换规则。主控重写评论 6310613016 要求「ADR 编号取 main 当前最大号 +1 写死进验收命令」。现 origin/main 已含 0044-0046（PR #1658 已合入），最大号 0046；预留表 adr-reservations.json 最大也是 46，开着的 PR 无新增 ADR 文件。处置：本票写死 **ADR 0047**，并在预留表登记 `"1531": 47`；同批 #1532 登记 48，不撞号。
5. **#1429 追加时点前后矛盾（上一稿 R3 BLOCK）**：上一稿裁定四写「落地时」，票面修改段和验收第 9 项写「关本票前」。处置：统一为一个时点，见裁定四.3。
6. **「兜底」撞词（Q9）**：本票禁的「宽松兜底」与词汇表里的合法词条「兜底通道（C0）」（CONTEXT.md:397-398）同字。处置：裁定二.4 划界。

## 权威依据

1. **#799 收敛通则正本句**（AGENTS.md:48）：「同一个判断不得存在宽严两档实现，更不得让宽松的那档接管严格那档的失败。」判别式（AGENTS.md:58）：「看两套判据**是不是接在同一个 if-else 上**」。三类正当分化（AGENTS.md:54-56）：①不同判断；②诊断模式 vs 生产模式，诊断「**不得参与准入**」；③已声明的独立对照臂（#799 裁定六）。
2. **#804 总缝规则正本句**（AGENTS.md:66）：「级别的差异只准以传进去的值出现，不准是另一套判据。」判别式（AGENTS.md:70）：「把两级的输入换成同一份数据喂进去，**两边输出必须逐位相同**」。#804 裁定原评论第 18 行措辞相同（https://github.com/xy7365527-lang/NewChanlun/issues/804#issuecomment-5138190089）。
3. **机械锁分派**：AGENTS.md:78「grep 降级为分诊线索，不是判据」；AGENTS.md:80「本条属**行为不变式**类 ⟹ 载体只能是**测试锁**（同输入同输出对拍），不能靠 grep gate」。#799 裁定十一原评论第 34 行（https://github.com/xy7365527-lang/NewChanlun/issues/799#issuecomment-5138004315）：「**验收判据必须真的被 CI 执行才算数，只编译不执行的锁按「无锁」计**」。同一评论第 28 行说明 grep 不可取的理由：`sublevel_diverges`「跟 `div_cand` 没有任何共同标识符」。
4. **#804 ② 举证门**（AGENTS.md:88-90）：例外须「指得出实质」「写得出退场条件」「不超上限」，按 #799 裁定二全仓上限 3 条。**#804 ③**（AGENTS.md:96 小标题「举证书的载体：随概念走，不另建台账」；:98 正本写进 `.chanlun/definitions/<概念>.md` 作**例外条款**段；:101「**不建总清单**」）。
5. **#812 Z-3 撤销 E1**（AGENTS.md:107）：`center_from_segments` 与 `center_from_window`「可达输入域恒等、输出恒等」，E1「不占总缝规则例外配额」，「两函数并存不构成违规」。分派点在 `recursive_tower.rs:398` 和 `:641`（`let build = if is_l0 {`）。
6. **可达输入域口径**（.chanlun/definitions/zhongshu.md:69）：「总缝规则与收敛通则的判别式，在「可达输入域」上评估。……主张伪分歧的一方必须举出可达性论证（点名到保证它的那行代码），举不出就按真分歧办。」
7. **测试锁登记列**（.chanlun/definitions/结构判据.md:20）：「⑧测试锁（同输入同输出；只编译不执行按无锁计）」。锁网行在 :33。
8. **失败即失败**（docs/adr/0027-classification-chong-operating-program-ownership.md:32）：「失败必须作为失败处理，不能切到另一套判据、旧入口或宽松路径重新判定。」
9. **#1323 Destination**：票面第 32 行「同一判断不存在第二套生产实现或失败兜底」；第 68-69 行引 #799 和 #804。**#1340 AT-02**（票面第 410 行）：「同输入换级别参数同判断逐位相同；全候选真值不救回选中失败；旁路不回流主塔」。
10. **范式锁（main 上现役）**：
    - `theta_v0::classifier::center::tests::center_segments_window_identity_on_l0_domain`（rust/src/theta_v0/classifier/center.rs:529-531）。先断言域外同向三段「两函数非无条件恒等（锁非 vacuous）」，再对 L0 方向交替域逐位对拍。登记于 issue1294 报告 :21，CI 面写在 :43。
    - `theta_v0::classifier::cand_predicate::tests::rmove_dir_does_not_fallback_after_selected_criterion_fails`（cand_predicate.rs:929-951）。断言 `rmove_dir(&rmove)` 为 `None`，同时断言 `rmove_dir_with_criterion(&rmove, DirCriterion::DualEnvelopeRiseFall)` 为 `Some(Direction::Up)`。生产入口 `rmove_dir` 只调用 `rmove_dir_with_criterion(rmove, DEFAULT_DIR_CRITERION)`（:168-169）。
    - 两个测试模块都只有 `#[cfg(test)]` 门（center.rs:339、cand_predicate.rs:565），不依赖 feature。
11. **CI 执行面（现行 .github/workflows/ci.yml）**：
    - 触发：push 与 pull_request 到 main（:3-8）。
    - job `rust-test-default`（:239），步名「cargo test — default targets」：`cargo test --all-targets --no-fail-fast --jobs 1`（:256-257）。
    - feature 专属测试单列一步、按测试名过滤执行的先例：步名「cargo test — TB-02-C member source binding」（:354-355），以及「cargo test — backtest_bin feature」（:409-414）。
    - ci.yml 里 `include-ignored` 计数为 0。
    - #1294 报告 :30 把锁的 CI 面合格线写成：「`cargo test --all-targets`（default targets）内非 `#[ignore]`，确定性无外部数据依赖。」
12. **下游承接先例**：docs/adr/0039-production-entry-inventory-and-unique-launcher.md:353「**B. 本裁定要求的机械锁（由列明的下游票验收，判据同样二值；不作关本票条件）**」。#1530 resolution 第 7 行：「下游：B10–B16 机械锁由 #1523、#1383、#1360、#1413 验收，不作关本票条件。」第 5 行记有 resolution 轮次即补边的先例（「补边 #1532→#1413、#1530→#1383」）。
13. **落地序列**（.sandcastle/docs/graph-engineering.md:77）：T 类 PASS 后「正本写进 `decision/<票号>` 分支……票上发 resolution 评论……PR 合入后关票……接新 blocking 边」。
14. **ADR 编号机制**：
    - 主控重写评论 6310613016（#1531）：「ADR 编号取 main 当前最大号 +1 写死进验收命令，统一 #1429 追加时点」。
    - `git ls-tree --name-only origin/main docs/adr/` 最后一项为 `0046-s-admission-anchor-issuance-and-takeover.md`；`/Volumes/AgentStorage/decisions/adr-reservations.json` 最大预留 46。decision-claimer.mts 的 `allocateAdr` 同样按 max(main, 有效预留)+1 取号，本票手工取号与它一致，得 0047。
15. **术语**：CONTEXT.md:397-398「**兜底通道（C0）**: 所有 P_j 均不命中时命中的通道，裁决为 Hold。」CONTEXT.md:386「运行中改值是 #799 所禁的失败兜底；同一判断的宽严两档本身不是自由度。」
16. **自由度通则边界**（docs/adr/0041-degrees-of-freedom-configurable-not-excluded.md:11）：「同一判断的松紧两个版本不是自由度，事前单选也不得落为可调字段。」:15：「可调字段不得有「主值失败时的备用值」」。

## 候选

本票各点都属 #799/#804 的已裁推论（[新缠论:推论]），不是 [新缠论:选择] 或参数类，所以不按 ADR 0041 落成可调字段（依据 0041:11 和 CONTEXT.md:386）。考古没有 C 类，下列 B 类均以「默认值，确认或否决」呈现。

**Q1 对拍锁的完整证明（B）**
默认值：对拍锁用同一判断入口，在两个级别参数下喂同一份输入，`assert_eq!` 整个输出逐位相等。锁必须含三件：
- ① 至少一组非空转臂：历史分歧或域外向量，证明锁会红；
- ② 可达输入域全枚举，或覆盖各类落点的网格；
- ③ 点名保证可达性的那行代码。

「完整」以可达输入域为界。范式取 center.rs:531。出处：AGENTS.md:70、zhongshu.md:69、issue1294 报告 :42。——确认或否决。
推理链（重过）：#804 判别式只定了「同份数据、逐位相同」。#1294 锁 3 把它拆成「分歧臂 + 可达域枚举」两段，zhongshu.md:69 又把评估域限定在可达域、要求点名可达性代码。三者合起来就是上面三件，没有引入新取舍。
排除：
- 「完整 = 全输入空间对拍」：E1 两函数在域外确有分歧（center.rs:535-546 的同向三段），全空间对拍必然判红，等于推翻已裁的 #812 Z-3（AGENTS.md:107「并存不构成违规」）。按「违已裁推论」排除。
- 「grep gate / 形状检查代替对拍」：违 AGENTS.md:78/:80 与 #799 裁定十一，排除。

**Q2 不兜底锁的测试形态（B）**
默认值：凡同一判断存在其它候选判据或历史宽档的，各配一把负控测试。构造「选中判据失败、而候选或宽档成功」的输入，并同时断言：
- ① 候选或宽档单独调用确实成功（锁不空转）；
- ② 生产入口输出仍为失败（None、Err 或拒绝）。

诊断输出不进准入断言；锁里不得出现「严格失败则改断宽松结果」。范式取 cand_predicate.rs:929。出处：AGENTS.md:48/:55/:58、ADR 0027:32、#1340 AT-02。——确认或否决。
推理链（重过）：只断言②时，无法区分「没兜底」和「宽档在这份输入上本来也失败」，锁会空转。①补上非空转性，与 Q1 的分歧臂同构。cand_predicate.rs:929 的两条 `assert_eq!` 正是这个形态。
排除：「严格失败时允许宽档结果进入诊断字段并参与准入」——违 AGENTS.md:55「不得参与准入」，属 #799 兜底，排除。

**Q3 CI 执行面总原则（A，直接沿用）**
已裁：沿用 #799 裁定十一（只编译不执行按无锁计）与 AGENTS.md:80（行为不变式只能用测试锁）。默认目标测试以 job `rust-test-default` 的步「cargo test — default targets」为执行面，按 job 名和步名引用，不按行号；AGENTS.md:80 的「先让 CI 跑 `cargo test`」已由 #810（CLOSED）完成。job 名与步名只是引用订正（「问题」冲突 2）。出处：#799 评论 5138004315 第 34 行、ci.yml:239/:256-257、#810。

**Q4 ignore、feature 专属、仓外数据的锁（B）**
默认值：锁同时满足以下三条才算被 CI 执行，缺一条按无锁计：
- (i) 不标 `#[ignore]`；
- (ii) 确定性，不依赖仓外数据；
- (iii) 落在 `cargo test --all-targets` 默认目标内；feature 专属的锁须在 ci.yml 单列一步，按测试全名过滤执行。

出处：#1294 报告 :30、ci.yml 步「cargo test — TB-02-C member source binding」:354-355；更早先例 `.chanlun/review-results/issue1147-gate-on-retest-20260821.md:142`「只编译不执行按无锁计（`#[ignore]` / 未入 CI 均计无锁）」与 :161（`#[ignore]` 探针不计入任何载体验收，2026-08-21，早于 #1294 报告）。——确认或否决。
推理链（重过）：「只编译不执行按无锁计」的理由是 CI 不跑的断言挡不住回退。三种情形各自对应：
- `#[ignore]` 测试默认不执行，而 ci.yml 里 `include-ignored` 为 0；
- feature 专属测试不在默认目标里编译；
- 仓外数据在 CI 环境不存在。

三者都等价于不执行。
排除：「CI 加 `--include-ignored` 全量跑 306 个 ignore 测试」——它们多数依赖仓外数据，会把锁面变成不确定，不满足 (ii)，所以不作默认路径。具体某把锁若确属确定性，应去掉 `#[ignore]`，而不是开全局开关。

**Q5 ADR 编号（B）**
默认值：按主控重写评论 6310613016，编号取「max(origin/main, 有效预留) + 1」写死：origin/main 最大 0046，预留表最大 46，本票取 **0047**，预留表登记 `"1531": 47`。验收命令直接写 `docs/adr/0047-level-isomorphism-and-no-fallback-locks.md` 与 `ADR 0047`。出处：主控评论 6310613016；`git ls-tree --name-only origin/main docs/adr/`；adr-reservations.json；decision-claimer.mts `allocateAdr` 同一算法（max(main, 预留)+1）。——确认或否决。
推理链：
- 上一稿顾虑「main+1 撞 #1652 或 PR #1658」，两者现均已合入 main，origin/main 最大号即 0046，+1 不再撞号。
- 同批在途的 #1532 若也新建 ADR，取 0048 并同样登记预留表，二者不并发撞号。
- 写死后验收命令无需任何换号规则即可照原文执行，正是上一稿 R3 要求的改法。

**Q6 #1429 票面追加时点（B）**
默认值：统一为「**发本票 resolution 评论的同一轮次**」：ADR 0047 已写进 `decision/1531` 分支、PR 已开出即可，在 #1429 验收第 1 条末尾追加「（按 ADR 0047）」。本票在该 PR 合入后关闭。出处：graph-engineering.md:77、#1530 resolution 第 5 行（PR 未合入即补边）。——确认或否决。

**Q7 锁的登记处（B）**
默认值：ADR 0047 只立协议，不列全仓锁清单。每把锁以测试全名（`模块路径::tests::测试名`）登记在所属判定点的 ⑧ 列：Rust 侧走结构判据.md §4 对应行，概念层走 `.chanlun/definitions/<概念>.md`。现役两把范式锁在 ADR 里点名。出处：结构判据.md:20、AGENTS.md:96/:101、issue1294 报告 :21。——确认或否决。

**Q8 关票条件与承接（B）**
默认值：关票条件只含三项：ADR 0047 落盘；两把范式锁按全名可跑且非 `#[ignore]`；#1429 票面已承接。新锁实装由 #1429（TB-02-H）和 #1360/#1361/#1369 承接，列进 ADR 的 B 段，不作关本票条件。不新增依赖边。出处：ADR 0039:353、#1530 resolution 第 7 行。——确认或否决。

**Q9 「兜底」与「兜底通道 C0」划界（B）**
默认值：本票所禁的「兜底」专指「同一判断在严格判据失败后换判据重判并放行」（AGENTS.md:48）。兜底通道 C0（CONTEXT.md:397-398）输出 Hold、不放行准入，属限定词①「不同判断」（AGENTS.md:54），不在锁的管辖内，锁不得把 C0→Hold 判红。CONTEXT.md 词条不改。——确认或否决。

## 裁定

Q3 为 A 类直接沿用；其余 B 类默认值按先例采纳（#1323 Notes N-2 由主控代裁），无 C 类待决。正本是一份新 ADR。

正本落点：docs/adr/0047-level-isomorphism-and-no-fallback-locks.md

本文件首三行按 ADR 0039 先例写正本头（正本／最后裁定 #1531／受影响代码清单指向下文「范式实例」节）。正文含以下五条裁定。

### 裁定一、级别同构对拍锁（Q1）

1. 凡受 #804 管辖的判定，载体是对拍测试锁：同一判断入口，在两个级别参数下喂同一份输入，`assert_eq!` 断言整个输出逐位相等。
2. 每把锁必含三件：
   - ① 至少一组非空转臂，用历史分歧或域外向量证明锁会红；
   - ② 可达输入域全枚举，或覆盖各类落点的网格；
   - ③ 在测试注释里点名保证可达性的那行代码（zhongshu.md:69）。
3. 「完整」以可达输入域为界，不以全输入空间为界。#812 Z-3 认定的并存（AGENTS.md:107）不因本 ADR 改判，也不占例外配额。
4. 可达域上写不出对拍的，只有两条路：按 #804 ② 举证门在概念定义文件里登记例外条款（AGENTS.md:88-90、:98，全仓上限 3），或者按真分歧办。不设第三条路。
5. 范式实例：`theta_v0::classifier::center::tests::center_segments_window_identity_on_l0_domain`。本 ADR 注明：AGENTS.md:107 的「恒等性的对拍测试锁尚缺」已由 #1294 补齐，以本 ADR 为准。

### 裁定二、严格判据不兜底负控锁（Q2、Q9）

1. 凡同一判断存在其它候选判据或历史宽档的，各配一把负控锁。输入构造成「选中判据失败、候选或宽档成功」。
2. 锁里同时断言两件：①候选或宽档单独调用成功（非空转）；②生产入口仍为失败（None、Err 或拒绝）。
3. 锁里不得出现「严格失败则改断宽松结果」的分支；诊断值不得进入准入断言（AGENTS.md:55）。
4. 本 ADR 所禁的「兜底」只指 AGENTS.md:48 意义上的失败兜底。兜底通道 C0（CONTEXT.md:397-398）属不同判断，锁不得把它判红。
5. 范式实例：`theta_v0::classifier::cand_predicate::tests::rmove_dir_does_not_fallback_after_selected_criterion_fails`。

### 裁定三、CI 执行面（Q3、Q4）

1. 沿用 #799 裁定十一：锁只有被 CI 执行才算数，只编译不执行按无锁计；grep 只作分诊线索（AGENTS.md:78/:80）。
2. 锁须同时满足三条，缺一按无锁计：(i) 不标 `#[ignore]`；(ii) 确定性，不依赖仓外数据；(iii) 在 job `rust-test-default` 的步「cargo test — default targets」所覆盖的默认目标内。
3. feature 专属锁须在 ci.yml 单列一步，按测试全名过滤执行，先例是步「cargo test — TB-02-C member source binding」。这一步扩的是执行面，不是第二套判据。
4. 引用 CI 面一律用 job 名和步名，不用行号。

### 裁定四、登记与承接（Q6、Q7、Q8）

1. 每把锁按测试全名登记在所属判定点的 ⑧ 列（结构判据.md:20），不另建台账。
2. 新锁实装由 #1429（TB-02-H，验收第 1 条已承接「每级使用同一判定，换相同输入逐位对拍；不因层级另造宽档」）和 #1360/#1361/#1369 承接，写进 ADR 的 B 段，不作关本票条件。
3. **时点唯一**：发本票 resolution 评论的同一轮次（ADR 0047 已写进 `decision/1531`、PR 已开出），在 #1429 验收第 1 条末尾追加「（按 ADR 0047）」。本票在该 PR 合入后关闭。因此验收 A9 一定先于 A10。

### 裁定五、编号与落地（Q5）

1. 本 ADR 编号写死为 0047（origin/main 最大 0046 与预留表最大 46 取大 +1），预留表登记 `"1531": 47`。
2. 下列验收命令照原文执行，不需要换号。

### 范式实例

本 ADR 只立协议，不列全仓锁清单（裁定四.1）。受影响代码限于下列两把现役范式锁，均在 main 上、仅 `#[cfg(test)]` 门、不依赖 feature：

- `theta_v0::classifier::center::tests::center_segments_window_identity_on_l0_domain`（rust/src/theta_v0/classifier/center.rs:529-531；测试模块门 center.rs:339）——裁定一范式。
- `theta_v0::classifier::cand_predicate::tests::rmove_dir_does_not_fallback_after_selected_criterion_fails`（rust/src/theta_v0/classifier/cand_predicate.rs:929-951；生产入口 `rmove_dir` 在 :168-169；测试模块门 cand_predicate.rs:565）——裁定二范式。

## 验收清单

**A. 关本票条件**（照原文执行；判据均二值）

1. 文件存在：`test -f docs/adr/0047-level-isomorphism-and-no-fallback-locks.md` 退出码 0。
2. 可执行命令：`grep -c '^> 正本：' docs/adr/0047-level-isomorphism-and-no-fallback-locks.md` 输出 `1`；`grep -c '^> 最后裁定：#1531' docs/adr/0047-level-isomorphism-and-no-fallback-locks.md` 输出 `1`。
3. 可执行命令：`grep -c 'center_segments_window_identity_on_l0_domain' docs/adr/0047-level-isomorphism-and-no-fallback-locks.md` ≥ 1，且 `grep -c 'rmove_dir_does_not_fallback_after_selected_criterion_fails' docs/adr/0047-level-isomorphism-and-no-fallback-locks.md` ≥ 1。
4. 可执行命令：`grep -c '兜底通道' docs/adr/0047-level-isomorphism-and-no-fallback-locks.md` ≥ 1，且 `grep -c 'rust-test-default' docs/adr/0047-level-isomorphism-and-no-fallback-locks.md` ≥ 1。
5. 测试名：`cd rust && cargo test --lib theta_v0::classifier::center::tests::center_segments_window_identity_on_l0_domain -- --exact 2>&1 | grep -c 'test result: ok. 1 passed'` 输出 `1`。
6. 测试名：`cd rust && cargo test --lib theta_v0::classifier::cand_predicate::tests::rmove_dir_does_not_fallback_after_selected_criterion_fails -- --exact 2>&1 | grep -c 'test result: ok. 1 passed'` 输出 `1`。
7. 可执行命令（非 ignore）：`cd rust && cargo test --lib -- --list --ignored 2>/dev/null | grep -cE '^theta_v0::classifier::(center::tests::center_segments_window_identity_on_l0_domain|cand_predicate::tests::rmove_dir_does_not_fallback_after_selected_criterion_fails): test$'` 输出 `0`。
8. 可执行命令（CI 执行面在册）：`grep -c '^  rust-test-default:$' .github/workflows/ci.yml` 输出 `1`，且 `grep -cF 'cargo test --all-targets --no-fail-fast' .github/workflows/ci.yml` ≥ 1。
8a. 可执行命令（例外未超限）：`grep -rhE '^#+ .*例外条款' .chanlun/definitions/ | wc -l` 输出 ≤ 3。
9. 票状态（先于 A10）：`gh issue view 1429 --repo xy7365527-lang/NewChanlun --json body --jq .body | grep -c 'ADR 0047'` ≥ 1。
10. 票状态：`gh issue view 1531 --repo xy7365527-lang/NewChanlun --json state --jq .state` 输出 `CLOSED`。

**B. 本裁定要求的机械锁（由下游票验收，判据同样二值；不作关本票条件）**

11. 票状态（#1429 承接各级同判定的对拍锁）：`gh issue view 1429 --repo xy7365527-lang/NewChanlun --json state --jq .state` 输出 `CLOSED`。
12. 票状态（TB-02/TB-03/TB-11 汇验）：`for n in 1360 1361 1369; do gh issue view $n --repo xy7365527-lang/NewChanlun --json state --jq .state; done | grep -c CLOSED` 输出 `3`。

## 受影响票与 blocking 边变化

| 票 | 变化 | 依据 |
|---|---|---|
| #1531（本票） | 无边增删。现 blocking 1533/1360/1361/1369（全 open），blocked_by 1530（closed）、1545（closed） | `gh api …/1531/dependencies/*` 实查 |
| #1545 | 无。已作废的裁定票，CLOSED | 实查 state |
| #1429（TB-02-H） | **只改票面**：resolution 同一轮次在验收第 1 条末尾追加「（按 ADR 0047）」。不加 #1531→#1429 边：本票验收不依赖 #1429 的产出；#1429 已经由 #1443（TB-02-T，blocked_by 含 1429）进入 TB-02 汇验 | 实查 #1429 blocking 1431/1432/1434/1443 |
| #1360（TB-02）、#1361（TB-03）、#1369（TB-11） | 无边增删。各自 blocked_by 已含 1531，对应票面列的 TB-02/03/11 | 实查 |
| #1533 | 无边增删。blocked_by 已含 1531 | 实查 |
| #1323 | PR 合入后在 map 的 Decisions so far 加一行（graph-engineering.md:77），无边变化 | — |

无需开 research 票或探针票：考古没有 C 类事实缺口。

## 授权引用

本条依 #1323 Notes N-2 由主控代裁（#1323 评论 6310866929：P 类问表中有先例的 B 类默认值确认由主控按推荐值裁定，经独审后落地）。本票分级 T，两轮独审 R7 均判 T；转 P 只是「再 BLOCK 转 P」的机械后果。另引 N-1 预授权：用户 2026-10-18 原话：「其实很多技术的东西 完全不需要我来做 我完全信任你的判断」（#1323 评论 6261281111，https://github.com/xy7365527-lang/NewChanlun/issues/1323#issuecomment-6261281111；正本 .sandcastle/docs/graph-engineering.md:90）。
