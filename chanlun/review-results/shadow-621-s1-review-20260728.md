# 影子评审 #621 S1：买卖点账本核心生命周期最小环

> 日期：2026-07-28（跑批跨 07-29 零点）
> 评审车：claude / opus，新上下文独立评审，**未参与 #621 实装**
> 工位：`/tmp/kimi-nest-mainline`；全程单线程（无子代理、无后台任务）
> 权限自律：只读 + 跑测试；**零源文件改动、零 git mutation**（自造探针为新建临时文件，跑完已移出仓库，见 §7）
> 评审锚：`70d4460359`（代码面，message 属 #626，混入事故在案，编排已裁方案 A）+ `9e44d98728`（空 diff 归属登记）

---

## 0. 结论先行

**verdict：PASS WITH FINDINGS —— 不阻塞关票。**

票面五项 AC 全部达成，四项复现逐位对上实施报告；内核冻面零触碰；混批完整性实测零差异；Standards 硬杠逐项复核通过。发现 **4 条 MEDIUM + 5 条 LOW，零 HIGH**。

四条 MEDIUM 有一个共同形状：**声明面比代码面宽**（`no-patch-mentality` 禁条 5「声明膨胀」）。代码行为本身没有一处是错的，错的是三处 doc 把「S1 只做到这里」写成了「这里就是全部」。这不是补丁思维，但也不是本仓要求的严格性——注释里那三句话按现状是不实的，须订正或降级为「S1 口径」。

最重一条（MEDIUM-1）：**跨进程恢复后，未决候选可被「陈旧知情时」落锤，并把这个陈旧知情时写进首写永不改的落锤钟与成立档 `confirmed_as_of`**。作者已声明门卫钟退回下界（偏离④），但把可观察后果写成了「输入不再被判倒退」——真实后果比这句重：它能改判，且改出来的是不可撤销的身份事实。

零生产调用点是事实（§6.3），但按 spec「消费方登记归 #575」，不判本票违规。

---

## 1. 独立复现（逐项照实报数）

### 1.1 指纹

| 项 | 实施报告 | 本车实测 | 判 |
|---|---|---|---|
| `cargo test --lib` | 2130 / 1 / 137 | **2130 passed / 1 failed / 137 ignored** | ✅ 逐位一致 |
| 唯一红 | `extract_signals_bit_exact_digest_guard`（#491） | 同（`signal.rs:3418`，摘要 `0xe6a2…` ≠ oracle） | ✅ |
| 本票新增 | 48 | **48 passed / 0 failed**（`cargo test --lib retrace_ledger`） | ✅ |
| 并行线净增 `#[test]` | +1 | `git diff e47039bdcb..HEAD -- rust/src`（排除 retrace_ledger）：**+29 / −28 = +1** | ✅ |
| `--no-fail-fast` 全量 | 唯一红 = #491 | **41 个 target：40 绿 + lib 一红；8 个集成 target 全绿、doc-tests 全绿** | ✅ |

红与本票无关的独立证据：`git log e47039bdcb..HEAD -- rust/src/theta_v0/classifier/signal.rs` **零 commit**；#491 在 tracker 上 OPEN（"GOLDEN 未随语义漂移重算，疑似 bbbd8f89fa 起"）。

> 跑批环境注记：并行线在评审期间推进了 HEAD（`9e44d98728` → `02ef5f93b1`，含 #497/#604/#610）。`--lib` 在开工态与 `02ef5f93b1` 两次测得**同为 2130/1/137**，故该指纹对并行线漂移稳健。

### 1.2 内核冻面（评审项 2）

```
git diff --stat e47039bdcb..HEAD -- ledger_kernel/ first_retrace_replay.rs   →  零行
git show --stat 70d4460359  -- ledger_kernel/ first_retrace_replay.rs nest_lifecycle.rs  →  零行
git log --oneline e47039bdcb..HEAD -- nest_lifecycle.rs
    9f2f519c39  #604 影子评审 MEDIUM-1 订正
    e520cf7fa4  #604 活窗右端判据单一权威化
```

✅ **通过**。`ledger_kernel/` 与 `first_retrace_replay.rs` 全区间零行；`nest_lifecycle.rs` 的两个 commit 全部归并行线 #604，**#621 一处未触**。

> 订正实施报告一处不完整：报告只把 `nest_lifecycle.rs` 归到 `e520cf7fa4`，实际区间内还有 `9f2f519c39`（同属 #604）。归因方向无误，枚举不全，不影响结论。

### 1.3 混批完整性（评审项 7⑤）

`git diff 70d4460359 -- rust/src/theta_v0/classifier/retrace_ledger/` → **0 行**（开工态与当前 HEAD 两次复测均为 0）。
`70d4460359` 共 11 文件：`issue626-clearing-line-ruling-20260728.md`（35 行，属 #626）+ retrace_ledger 九文件 2968 行 + `classifier/mod.rs` 三行 `pub mod` 登记（全属 #621）。✅ 与报告一致。

### 1.4 Standards 硬杠（独立测量，非采信报告）

| 项 | 实测 | 判 |
|---|---|---|
| 文件 ≤800 行 | 456 / 170 / 442 / 663 / 135 / 496 / 137 / 359 / 110 | ✅ 最长 663 |
| 函数 ≤50 行 | 最长 36（`tests/clocks.rs:69`）；生产侧最长 35（`log.rs:480 load`） | ✅ |
| 常量具名 | 生产四文件裸数字扫描：命中项全为 doc 注释中的课号/票号 + 两枚具名常量（`log.rs:57/59`） | ✅ 零魔数 |
| 错误 fail-loud | 边界输入一律 `Result`；`assert!/expect/unreachable!` 仅用于内部不变量且带定位 | ✅（但见 MEDIUM-3：fail-loud 覆盖面不足） |

---

## 2. 自造重放探针（评审项 3）

六个探针，全部通过（`cargo test --test shadow621_probe` → **6 passed / 0 failed**）。探针只走公共 API，不引用被审模块的测试 fixture。源码见 §7。

| 探针 | 要证的事 | 结果 |
|---|---|---|
| 1 `probe_fold_equals_book_under_split_invariant` | `fold(journal)==book` 的拆分不变量真成立：身份事实（三态/计数/留档/出生钟/落锤钟/注册快照/谱系/终态证据）**逐位等同**；门卫钟 = 留档下界 ≤ 活值。剧本额外断言「下界 < 活值」的缺口**真实存在**（否则该不变量是同义反复） | ✅ 成立 |
| 2 `probe_dead_center_counter_undercounts…` | `dead_center_registrations` 漏计 | ✅ 复现（→ MEDIUM-4） |
| 3 `probe_fold_accepts_tampered_log_violating_single_active_discipline` | 篡改日志折出「同锚两个未决候选」，`fold` 返回 `Ok`，其后 `assert_invariants` panic | ✅ 复现（→ MEDIUM-3） |
| 4 `probe_fold_accepts_dangling_restart_lineage` | 篡改日志折出悬空 `Restarted{previous}`，无任何不变量逮住 | ✅ 复现（→ MEDIUM-3） |
| 5 `probe_terminal_evidence_may_contradict_registration_snapshot` | 注册拍与落锤拍证据可相反，成立档静默取落锤拍 | ✅ 复现（→ MEDIUM-2） |
| 6 `probe_restore_lets_stale_knowledge_settle_a_provisional_identity` | 恢复后陈旧知情时可落锤，并写进落锤钟 | ✅ 复现（→ MEDIUM-1） |

探针 4 的**第一版**（`SnapshotPinned` 证据置 `None`）跑出 `book.rs:412` panic —— 那是 MEDIUM-3 的第三个实例（无证据的注册快照同样折得进、同样被自己的不变量判死），已并入该条。

---

## 3. 发现（HIGH / MEDIUM / LOW）

### HIGH：无。

---

### MEDIUM-1 — 恢复后未决候选可被「陈旧知情时」落锤，且写进首写永不改的落锤钟

**证据**：`log.rs:18-27`（门卫钟可恢复性声明）、`log.rs:189-197`（`recover_gate_clock`）、`ledger_kernel/mod.rs:381-397`（`admit` 的 `as_of < last_as_of` 判据）、`mod.rs:442-455`（`write_settlement` 首写不改）；探针 6。

**复现**（探针 6 逐行断言）：

```
注册 as_of=500（进日志）→ 未决观察 as_of=4000（不进日志，只推门卫钟）
活账：observe(Success, as_of=600)  →  RetrogradeRejected，状态仍 Provisional   ← 裁定五纪律生效
崩溃/重启：fold(journal)          →  门卫钟退回 500（留档下界）
恢复账：observe(Success, as_of=600) →  Accepted → Confirmed
                                      terminal_as_of = Some(600)              ← 首写永不改的身份事实
                                      established_pack().confirmed_as_of = 600 ← 对外声称的知情时
```

**问题**：作者在偏离④中已诚实声明门卫钟退回下界，但把可观察后果写成了

> 「恢复后，落在『最后一条修订知情时』与『重启前最后一次未决观察知情时』之间的输入**不再被判倒退**。」（`log.rs:25-27`）

这句对**终态**身份是准确且无害的（只影响 `late_absorbed` 计数）。对**未决**身份它漏掉了后半句：这些输入不但不再被拒，**还能落锤**——产出一条终态修订，把 600 写进 `terminal_as_of`（首写永不改）与成立档 `confirmed_as_of`。裁定五「per-identity as_of 非降，违反 = 拒收 + 警报」的保护在恢复后对未决候选静默失效，且失效的产物是不可撤销的。

**不是未来函数**（600 < 4000，用的是更旧而非更新的信息），所以不构成回测偷看；但它使**同一条输入流在「有无重启」两种情形下产出不同的账本**，破坏了裁定六「跨进程恢复且无双真相漂移」的意图边界。

**建议**（不指定方案，供裁）：三选一——① 把门卫钟下界改用 `max(留档下界, 恢复时刻)`；② 恢复后对未决身份的落锤额外要求 `as_of ≥ registered_as_of` 之外的显式护栏；③ 至少把 `log.rs:24-27` 的声明补全为「未决身份可因此被陈旧知情时落锤」，并进 S2/S4 的待裁清单。第③项是最低限度——**当前的声明按现状不实**。

---

### MEDIUM-2 — 注册拍证据与落锤拍证据无一致性守卫，成立档静默取落锤拍

**证据**：`book.rs:231-258`（`judge` 用 `admitted.evidence` = 当拍输入）、`book.rs:375-387`（`pack_of`）、`mod.rs:307-315`（`registration()` 取 `SnapshotPinned` 载荷）；探针 5。

**复现**：同一身份（同 `frame`、同 `departure`），注册拍报 `side=Buy / leave_end=(1210, ZG+50)`，落锤拍报 `side=Sell / leave_end=(7777, ZD−50)` —— 全程零拒收、零警报，`assert_invariants()` 一声不吭，`established_pack()` 给出 `side=Sell / leave_end.index=7777`。

**问题两层**：

1. **口径混用**：`ThirdPointPack` 的 `frame` 来自 `entry.key.frame`（**注册拍**，因为 frame 是身份的一部分），而 `side` / `leave_end` / `retest_end` 来自 `terminal_evidence()`（**落锤拍**）。同一个证据包内部横跨两拍，且这一点在 `ThirdPointPack` 的 doc（`book.rs:87-104`）里没有一个字。
2. **与裁定二的态度不一致**：契约对 provider 改口的一贯态度是 fail-loud（裁定二「未判完而来新 departure = provider 有病 → 报错拒收」）。位置与侧同样是既成事实（leave 笔已走完才有 departure），provider 在两拍报不同值同样是「有病」，但这里静默照单全收。

**边界条件**：真实引擎大概率两拍同值，故实盘影响概率低；但账本自称「注册拍四条边快照为身份」（`mod.rs:72-86`），而对外交付的位置证据实际取自落锤拍——这是声明与实际不一致，不是概率问题。

---

### MEDIUM-3 — `fold` 的 fail-loud 只覆盖结构违规，不覆盖域语义违规；模块头声明与实际不符

**证据**：`log.rs:29-32`（声明）、`log.rs:156-233`（`apply_record` / `replay_open` / `replay_push`）、`book.rs:401-441`（域不变量）；探针 3 / 4 及探针 4 第一版。

`log.rs:29-32` 声明：

> 「JSONL 是外部边界：损坏的行、跳号、序列冲突、**语义不可折叠**一律转成 `RetraceLogError` 返回，**不 panic**。」

实测 `fold` 的语义校验只有三条：跳号、终态再落锤/再追加（禁复活）、修订落在未建仓身份。以下三类损坏日志**全部返回 `Ok`**：

| 损坏形态 | `fold` | 其后 `assert_invariants()` |
|---|---|---|
| 同一中枢锚两条 `Registered`（违反裁定二） | `Ok` | **panic**（`book.rs:439`「同一中枢至多一个未决候选」） |
| `Restarted{previous}` 指向从未建仓的身份 | `Ok` | 静默通过（内核只查「不自环」，域侧不查「前任在账」） |
| `SnapshotPinned` 证据为 `None` | `Ok` | **panic**（`book.rs:412`「无注册快照者只可能是前缀态」） |

第一、三行直接证伪了「不 panic」这句：账本自己的不变量判死的状态，`fold` 放行了。第二行是另一种病——损坏可以完全隐形，谱系指针悬空而无人发觉（谱系学保留原则的漏洞）。

**边界条件**：三类形态**活路上均不可达**（`guard_single_active` 挡第一类，`register_new` 的原子对挡第三类，`link_restart` 只写在账身份挡第二类），只有外部篡改/文件损坏才触发。故风险敞口 = 「JSONL 落盘后被改写」这一条通道。但那正是 `log.rs:29-32` 声称已封的通道。

---

### MEDIUM-4 — `dead_center_registrations` 与其自述的 S2 交接契约不符

**证据**：`book.rs:67-71`（声明）、`book.rs:200-215`（`link_restart` 只看**前一个注册**的状态）；探针 2。

`book.rs:69-71` 声明：

> 「**S1 只计数不拒收**：检测与 fail-loud 拒收归 S2（#622）。计数即 S2 的接手点——**改判后本计数应恒等于 S2 的拒收数**。」

**反例**（探针 2，全绿）：

```
key1 @ frame(1200) Success        → Confirmed，中枢死亡证明开出
key2 @ frame(1400) 注册            → previous=key1(Confirmed) → 计数 +1   ✓
key2 判败 → Invalidated
key3 @ frame(1600) 注册            → previous=key2(Invalidated) → 计数 +0  ✗
                                     但 death_certificate(anchor) 仍 Some ⟹ S2 必须拒收本次
S1 计数 = 1，S2 应拒收数 = 2
```

根因：`link_restart` 判死人挂号的依据是 `active_by_anchor` 里**前一个注册**的状态，而不是「该锚下是否存在任何 `Confirmed`」——后者才是裁定四「Success 后同一中枢永禁新轮」的判据，也正是 `death_certificate(anchor)`（`book.rs:360-366`）已经实现的查法。两个查法在同一文件里并存且不等价。

**影响**：S2 接手时若拿本计数做回归基线（"改判前后计数应相等"），基线本身是错的。修法极小（把 `link_restart` 的判据换成 `self.death_certificate(key.anchor()).is_some()`），但**属 S2 的票面**，本条只登记事实与订正声明。

---

### LOW-1 — `fold(journal) == book` 的字面写法未在 `book.rs` 标注拆分口径

`book.rs:113` 与 `book.rs:391` 两处 doc 写「`fold(journal) == book`」，而实际比对是拆分式的（身份事实逐位等同 + 门卫钟只对下界，见 `log.rs:285-308`）。拆分口径只在 `log.rs` 模块头说明。读 `book.rs` 的人会得到一个字面不成立的等式。建议两处补「（拆分口径见 `log` 模块头）」。

### LOW-2 — 同锚多个 `Confirmed` 时死亡证明按键序取首个，无唯一性守卫

`book.rs:360-366`：`death_certificate(anchor)` 对 `in_state(Confirmed)` 做 `find`，取键序首个。S1 允许同锚出现多个 `Confirmed`（裁定四只计数不拒收），此时"哪张是真的死亡证明"由 `RetraceKey` 的字典序决定，与时间无关。S2 落地拒收后该态不可达，但 S1 期间它是可达且无声的。

### LOW-3 — 模块零生产调用点

全仓对 `retrace_ledger` 的引用只有 `classifier/mod.rs:102` 的 `pub mod` 一行。spec 用户故事 22 的命门条款「无消费方不接生产、防 built-but-unwired」把消费方登记归 #575，#621 的 AC 未含此项，故**不判本票违规**；登记在案，#575 关票前必须闭。

### LOW-4 — 备战档素材已全开，无「盯这里 ≠ 买这里」的结构性防线

`RetraceEntry` 全部字段 `pub`，`entries()` / `entry()` / `active_candidate()`（`book.rs:310-329`）无差别放行 `Provisional` 条目，`registration()` 直接给出位置 + 快照 —— 裁定八「备战档禁被消费成买入信号」所需的全部素材在 S1 就已对外开放，且无任何名分标记。S3 建备战档时若直接包这层，禁区就只剩注释在守。本条不要求 S1 改（门户归 S3），但 S3 不得把它当成"已有的备战档"。

**成立档本身复核结论**：`established()` / `established_pack()` 严格按 `Confirmed` 过滤（`pending_and_failed_candidates_never_enter_the_established_portal` 已锁）；证据包同时带 `retest_end.index`（位置）与 `confirmed_as_of`（知情时），交易层自理迟到过滤的材料齐（裁定八②）；账本不进口任何外部状态（总禁区未破）。死亡证明载荷 = 中枢四条边（含转正的临时右边）+ 落锤知情时，两个时间不混（`death_certificate_carries_center_identity_and_issue_time` 逐条断言）。**✅ 无发现。**

### LOW-5 — `ThirdPointPack` 不带 provenance

证据包无 `level` / `window` / `data_basis`，消费方须自行与 `ledger.provenance()` 配对。裁定六「不承诺跨窗口身份连续」的语境下，脱离账本单独传递的 pack 无法自证属于哪条 run。

---

## 4. 判败四行语义的如实性（评审项 6）

| 契约四行 | 代码落位 | 判 |
|---|---|---|
| ① 候选**从未成立** | `NotConstitutedReason::RetestReentered`（`mod.rs:192-203`）+ 词汇 `RetraceRevisionKind::NotConstituted`。三态本体复用内核 `LedgerState::Invalidated`，内核对该态的定义是「失效：负向终态族，**具体名分由域的原因码承担**」（`ledger_kernel/mod.rs:64-65`）——不绑定"曾构成后被推翻"，故复用无语义冲突，且 `mod.rs:183-187` 已显式说明「契约措辞 `NotConstituted{reason}` 即内核 `Invalidated` + 本原因码」 | ✅ 如实 |
| ② 中枢未破坏 | `death_certificate()` 只扫 `Confirmed`；`retest_reentry_settles_not_constituted_without_center_event` 逐条断言判败后无死亡证明 | ✅ |
| ③ 不派生任何中枢生命周期事件 | 模块对外唯一的中枢事件面就是 `CenterDeathCertificate`，判败路径不产出；`RetraceStep.delta` 只含本身份修订 | ✅ |
| ④ 判败事件是**盘背观测源**，非终结非信号 | S1 只留素材面（`not_constituted_reason()` / `terminal_evidence()`），未把判败包装成任何"信号/终结"形状；短差档归 S3 | ✅ 无越界 |

四行**在代码与词汇中均如实**，无夸大、无缺失。`CenterRebased` 只占词汇位、零产出路径，`mod.rs:33-34` 与 `mod.rs:199-202` 两处均标明属 S2 —— 诚实。

---

## 5. 「经 T1 内核表达、无自造账本机制」（AC④）复核

逐条核对 `book.rs` 的域侧代码，**未发现自造账本机制**：

| 内核责任点 | 域侧调用点 | 有无绕过 |
|---|---|---|
| per-key 注册表 / 只读枚举 | `book.contains/get/values/in_state/len`（`book.rs:147,310-357`） | 无 |
| 首建 | `open_on_observation`（`book.rs:163-166`） | 无 |
| append-only 唯一追加点 | `push_revision`（`book.rs:262-273`），全模块仅此一处 + 内核 `settle` 内部一处 | 无 |
| 倒退拒绝 + 终态吸收 | `admit` → `LedgerAdmission`（`book.rs:221-227`） | 无 |
| 终态落账（禁复活） | `settle` + `LedgerSettlement`（`book.rs:252-257`） | 无 |
| 钟首写 | 出生钟在 `open`（`mod.rs:367-377`）、落锤钟经 `first_write_clock`（`mod.rs:449-455`） | 无 |
| 增量返回 | `LedgerDelta`（`book.rs:162,276-284`） | 无 |
| 不变量骨架 | `assert_core_invariants`（`book.rs:393`），域不变量叠加不替代 | 无 |
| 迁移留史 | 未用；`set_key` / `set_migrated_from` 双 `unreachable!` 封死（`mod.rs:383-385,433-435`），与契约「第二消费方不使用迁移留史」一致 | 无 |

六关联类型实形与 spec T3 章「内核接口」节回填的形状**逐项对上**（`mod.rs:262-277`）。

**结构性亮点（照实记）**：`RetraceEntry` 只留「三钟 + 三态 + 计数 + 留档」，注册快照 / 侧 / 位置 / 原因码 / 终态证据 / Restart 前任**一律现算自留档**（`mod.rs:306-363`）——投影没有第二份存放处，裁定六「日志+状态双真相必漂移」在结构上被消掉而非靠纪律维持。这是本票最扎实的一处设计。

---

## 6. 五条已核认偏离的独立复核（评审项 7）

| # | 偏离 | 本车判 | 理由 |
|---|---|---|---|
| ① | commit 归属被并行线吞 | **同意** | `git diff 70d4460359 -- retrace_ledger/` 实测 0 行，两次复测；内容完整无损。编排已裁方案 A 不改写历史，本车不复议 |
| ② | 中枢路由锚 = 起时间边 | **同意** | 论证成立且可加固：`RetraceProvenance` 携带 `level`（`log.rs:69-76`），一本账 = 一个级别，**同级别内 `start_index` 唯一确定一个中枢**，故锚无碰撞；延伸只右扩、扩张只动 ZG/ZD，起点均不动。`mod.rs:100-102` 已写明唯一失效情形（上游重基改写起点）归 S2 |
| ③ | 观察 `outcome` 为 `Option` | **同意** | 若 outcome 恒有值，`Provisional` 只在单次调用内瞬时存在，裁定七「沉默 = 未决、永不超时处死」与 S3 备战档双双落空。且适配器 missing 桶正是用 `outcome.is_some()` 判材料齐（`adapter.rs:163-169`），逻辑自洽 |
| ④ | 门卫钟不进日志 | **同意其取舍，不同意其后果声明** | 取舍正确：把守卫状态塞进日志＝为每个未决 bar 伪造一条无载荷身份事实，那才是真的编造。探针 1 确认拆分不变量成立且缺口真实。但后果声明不全 → **MEDIUM-1** |
| ⑤ | 单 commit 而非多 commit | **同意** | `mod/adapter/book/log` 相互引用，任何中间切分都要伪造一个从未存在过的编译态；「每个 commit 可编译」优先于「commit 粒度细」是本仓一贯口径 |

---

## 7. 探针源码（可复现；已从仓库移出，此处为唯一留存）

跑法：存为 `rust/tests/shadow621_probe.rs` → `cargo test --test shadow621_probe` → **删除该文件**。
本车已按此流程执行并清理，`git status -- rust/` 复核确认工作区无本车造成的任何改动。

```rust
//! 影子评审 #621 独立探针（评审车自造，跑完即删；不属交付面）。
//! 只走公共 API，不引用被审模块的测试 fixture。

use newchan_rust::theta_v0::classifier::first_retrace_replay::{RetraceOutcome, StrictCompletedPair};
use newchan_rust::theta_v0::classifier::ledger_kernel::LedgerRevision;
use newchan_rust::theta_v0::classifier::level_view::CoordinateWindow;
use newchan_rust::theta_v0::classifier::retrace_ledger::{
    CenterFrame, RetraceEvidence, RetraceInput, RetraceKey, RetraceLedger, RetracePoint,
    RetraceProvenance, RetraceRecord, RetraceRevisionKind, RetraceSide, RetraceState,
};
use newchan_rust::theta_v0::types::{Direction, Tick};

const ZD: Tick = 100;
const ZG: Tick = 200;
const ANCHOR: usize = 1_000;

fn frame(end_index: usize) -> CenterFrame {
    CenterFrame { zd: ZD, zg: ZG, start_index: ANCHOR, end_index }
}

fn prov() -> RetraceProvenance {
    RetraceProvenance {
        level: 3,
        window: CoordinateWindow { start: 0, end: 9_999 },
        data_basis: "shadow621-probe".to_owned(),
    }
}

fn up(center: CenterFrame, departure: usize, outcome: Option<RetraceOutcome>, as_of: usize) -> RetraceInput {
    RetraceInput {
        center,
        pair: StrictCompletedPair { leave_move_index: departure, retest_move_index: departure + 1 },
        leave_direction: Direction::Up,
        retest_direction: Direction::Down,
        leave_end: RetracePoint { index: center.end_index + 10, price: ZG + 50 },
        retest_end: outcome.map(|_| RetracePoint { index: center.end_index + 20, price: ZG + 10 }),
        outcome,
        as_of,
    }
}

fn key(center: CenterFrame, departure: usize) -> RetraceKey {
    RetraceKey { frame: center, departure_move_index: departure }
}

// ── 探针 1：fold(journal) == book 的拆分不变量（身份事实逐位 + 门卫钟下界） ──
#[test]
fn probe_fold_equals_book_under_split_invariant() {
    let mut live = RetraceLedger::new(prov());
    let a = frame(1_200);
    live.observe(&up(a, 3, None, 500)).unwrap();
    live.observe(&up(a, 3, None, 4_000)).unwrap();                                  // 未决观察：推门卫钟，不产修订
    live.observe(&up(a, 3, Some(RetraceOutcome::RetestReenters), 4_100)).unwrap();
    let b = frame(1_500);
    live.observe(&up(b, 9, None, 4_200)).unwrap();
    live.observe(&up(b, 9, Some(RetraceOutcome::Success), 4_300)).unwrap();
    live.observe(&up(b, 9, Some(RetraceOutcome::Success), 5_000)).unwrap();          // 迟到吸收
    live.observe(&up(b, 9, None, 10)).unwrap();                                      // 倒退拒收

    let folded = RetraceLedger::fold(prov(), live.journal()).unwrap();
    assert_eq!(folded.len(), live.len());
    assert_eq!(folded.journal(), live.journal());

    for entry in live.entries() {
        let r = folded.entry(&entry.key).unwrap();
        assert_eq!(r.state, entry.state);
        assert_eq!(r.revision, entry.revision);
        assert_eq!(r.revisions, entry.revisions);
        assert_eq!(r.registered_as_of, entry.registered_as_of);
        assert_eq!(r.terminal_as_of, entry.terminal_as_of);
        assert_eq!(r.registration(), entry.registration());
        assert_eq!(r.restarted_from(), entry.restarted_from());
        assert_eq!(r.terminal_evidence(), entry.terminal_evidence());
        assert_eq!(r.last_as_of, entry.log_supported_gate());
        assert!(r.last_as_of <= entry.last_as_of);
    }
    let gap = live.entries().any(|e| e.log_supported_gate() < e.last_as_of);
    assert!(gap, "剧本须真的制造出门卫钟缺口");
    assert_eq!(live.entries().filter(|e| e.state == RetraceState::Confirmed).count(), 1);
    assert_eq!(live.alarms().late_absorbed, 1);
    assert_eq!(live.alarms().retrograde_rejected, 1);
    folded.assert_invariants();
    live.assert_invariants();
}

// ── 探针 2：dead_center_registrations 计数 ≠ S2 应拒收数 ──
#[test]
fn probe_dead_center_counter_undercounts_after_an_intervening_failure() {
    let mut book = RetraceLedger::new(prov());
    let f1 = frame(1_200);
    book.observe(&up(f1, 3, Some(RetraceOutcome::Success), 500)).unwrap();
    assert!(book.death_certificate(f1.anchor()).is_some(), "中枢已死");

    let f2 = frame(1_400);
    book.observe(&up(f2, 5, None, 600)).unwrap();
    assert_eq!(book.alarms().dead_center_registrations, 1);
    book.observe(&up(f2, 5, Some(RetraceOutcome::RetestReenters), 700)).unwrap();

    let f3 = frame(1_600);
    book.observe(&up(f3, 7, None, 800)).unwrap();
    assert!(book.death_certificate(f3.anchor()).is_some(), "同锚死亡证明仍可查 ⟹ S2 必须拒收本次注册");
    assert_eq!(book.alarms().dead_center_registrations, 1, "两次死人挂号只计到 1 —— 计数 ≠ S2 应拒收数");
    assert_eq!(book.entry(&key(f3, 7)).unwrap().restarted_from(), Some(key(f2, 5)));
    book.assert_invariants();
}

// ── 探针 3：损坏日志可折出违反裁定二的账本，fold 不 fail-loud ──
#[test]
fn probe_fold_accepts_tampered_log_violating_single_active_discipline() {
    let a = key(frame(1_200), 3);
    let b = key(frame(1_400), 9);   // 同 start_index ⟹ 同锚，不同身份
    let ev = |k: RetraceKey| RetraceEvidence {
        frame: k.frame,
        side: RetraceSide::Buy,
        leave_end: RetracePoint { index: k.frame.end_index + 10, price: ZG + 50 },
        retest_end: None,
    };
    let rec = |seq: u64, k: RetraceKey, kind, as_of, evidence| RetraceRecord {
        sequence: seq,
        revision: LedgerRevision { key: k, kind, as_of, evidence },
    };
    let journal = vec![
        rec(0, a, RetraceRevisionKind::Registered, 500, None),
        rec(1, a, RetraceRevisionKind::SnapshotPinned, 500, Some(ev(a))),
        rec(2, b, RetraceRevisionKind::Registered, 600, None),
        rec(3, b, RetraceRevisionKind::SnapshotPinned, 600, Some(ev(b))),
    ];

    let folded = RetraceLedger::fold(prov(), &journal).expect("篡改日志被 fold 接受（本探针要证的就是这一点）");
    assert_eq!(folded.len(), 2);
    assert_eq!(
        folded.entries()
            .filter(|e| e.state == RetraceState::Provisional && e.key.anchor() == a.anchor())
            .count(),
        2,
        "同一中枢锚出现两个未决候选 —— 活路上不可达（裁定二）"
    );
    let blown = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| { folded.assert_invariants(); }));
    assert!(blown.is_err(), "fold 放行了一个自己的 assert_invariants 判死的状态");
}

// ── 探针 5：注册拍证据 vs 落锤拍证据无一致性守卫 ──
#[test]
fn probe_terminal_evidence_may_contradict_registration_snapshot() {
    let mut book = RetraceLedger::new(prov());
    let c = frame(1_200);
    book.observe(&up(c, 3, None, 500)).unwrap();
    let registered = book.entry(&key(c, 3)).unwrap().registration().unwrap();
    assert_eq!(registered.leave_end.index, 1_210);
    assert_eq!(registered.side, RetraceSide::Buy);

    let mut flipped = up(c, 3, Some(RetraceOutcome::Success), 600);
    flipped.leave_direction = Direction::Down;
    flipped.retest_direction = Direction::Up;
    flipped.leave_end = RetracePoint { index: 7_777, price: ZD - 50 };
    flipped.retest_end = Some(RetracePoint { index: 8_888, price: ZD - 10 });
    let step = book.observe(&flipped).unwrap();
    assert_eq!(step.state, RetraceState::Confirmed, "被照单全收");

    let pack = book.established_pack(&key(c, 3)).unwrap();
    assert_eq!(pack.side, RetraceSide::Sell, "成立档的侧来自落锤拍");
    assert_eq!(pack.leave_end.index, 7_777, "成立档的位置来自落锤拍");
    assert_ne!(pack.side, registered.side, "同一身份的注册拍与落锤拍侧相反，无任何守卫拦截");
    assert!(book.death_certificate(c.anchor()).is_some());
    book.assert_invariants();
}

// ── 探针 4：损坏日志可折出悬空 Restart 谱系指针 ──
#[test]
fn probe_fold_accepts_dangling_restart_lineage() {
    let a = key(frame(1_200), 3);
    let ghost = key(frame(9_999), 42);   // 从未建仓的身份
    let ev = RetraceEvidence {
        frame: a.frame,
        side: RetraceSide::Buy,
        leave_end: RetracePoint { index: a.frame.end_index + 10, price: ZG + 50 },
        retest_end: None,
    };
    let rec = |seq: u64, kind, evidence| RetraceRecord {
        sequence: seq,
        revision: LedgerRevision { key: a, kind, as_of: 500, evidence },
    };
    let journal = vec![
        rec(0, RetraceRevisionKind::Registered, None),
        rec(1, RetraceRevisionKind::SnapshotPinned, Some(ev)),
        rec(2, RetraceRevisionKind::Restarted { previous: ghost }, None),
    ];
    let folded = RetraceLedger::fold(prov(), &journal).expect("悬空谱系被 fold 接受");
    assert_eq!(folded.entry(&a).unwrap().restarted_from(), Some(ghost));
    assert!(folded.entry(&ghost).is_none(), "前任身份根本不在账");
    folded.assert_invariants();   // 悬空前任指针不被任何不变量逮住
}

// ── 探针 6：恢复后未决候选可被「陈旧知情时」落锤 ──
#[test]
fn probe_restore_lets_stale_knowledge_settle_a_provisional_identity() {
    let mut live = RetraceLedger::new(prov());
    let c = frame(1_200);
    live.observe(&up(c, 3, None, 500)).unwrap();       // 注册（进日志）
    live.observe(&up(c, 3, None, 4_000)).unwrap();     // 未决观察（不进日志，推门卫钟）
    let k = key(c, 3);
    assert_eq!(live.entry(&k).unwrap().last_as_of, 4_000);

    let mut alive = live.clone();
    let step = alive.observe(&up(c, 3, Some(RetraceOutcome::Success), 600)).unwrap();
    assert!(step.retrograde_rejected(), "重启前：陈旧知情时被拒");
    assert_eq!(alive.entry(&k).unwrap().state, RetraceState::Provisional);

    let mut restored = RetraceLedger::fold(prov(), live.journal()).unwrap();
    assert_eq!(restored.entry(&k).unwrap().last_as_of, 500, "门卫钟退回留档下界");
    let step = restored.observe(&up(c, 3, Some(RetraceOutcome::Success), 600)).unwrap();
    assert!(!step.retrograde_rejected(), "重启后：同一条输入不再被判倒退");
    assert_eq!(step.state, RetraceState::Confirmed);
    assert_eq!(restored.entry(&k).unwrap().terminal_as_of, Some(600), "落锤钟记的是陈旧知情时");
    assert_eq!(restored.established_pack(&k).unwrap().confirmed_as_of, 600,
        "成立档对外声称的知情时 = 600，而系统重启前已知到 4000");
    restored.assert_invariants();
}
```

---

## 8. 结果包六要素

**1. 结论** —— #621 S1 **PASS WITH FINDINGS**，不阻塞关票。五项 AC 全达成；4 MEDIUM（全部形状为「声明面宽于代码面」）+ 5 LOW + 0 HIGH。建议：MEDIUM-1/-3/-4 的三处 doc 声明**本票内订正**（订正注释不改行为、不改指纹）；MEDIUM-1 的行为面处置与 MEDIUM-2 的守卫上浮至 S2/S4 裁决。

**2. 定义依据** —— #574 语义契约八条裁定逐条比对（§4/§5/§6，八条落位全部核过）；spec `spec-kernel-and-retrace-ledger-20260728.md` T3 章内核接口节（六关联类型实形对照 `mod.rs:262-277`）；#621 票面五项 AC 逐条复现（§1）；`ledger_kernel/mod.rs:1-42` 的 11 责任点表（§5 逐点核无绕过）。

**3. 边界条件（结论翻转的条件）** ——
- MEDIUM-1 若恢复路径被接入生产消费（S4 或生产化）而声明未订正、行为未加护栏 → 本条升 HIGH，verdict 转 CONDITIONAL；
- MEDIUM-3 若 JSONL 落盘路径接入不可信存储（跨机/跨用户/网络卷）→ 升 HIGH；当前只在本地 temp 与自产日志间流转，故 MEDIUM；
- MEDIUM-2 若真实 provider 被证实存在两拍改口（例如引擎重算 leave 笔终点）→ 升 HIGH；
- 全部 MEDIUM 均**不因 S2/S3 未落地而自动降级**——它们是 S1 已落盘代码与已落盘注释之间的不一致，不是"S1 没做完"；
- 若我对 `RetraceProvenance.level` 保证「一本账一个级别」的读解有误（即一本账可跨级别），则偏离②的锚唯一性论证失效 → LOW 升 MEDIUM。

**4. 下游推论** ——
- **S2（#622）**：`link_restart` 的死人挂号判据须换成 `death_certificate(anchor).is_some()`（MEDIUM-4），且**不得**用 S1 的 `dead_center_registrations` 做回归基线；`CenterRebased` 检测落地后，当前被误分类为 `ActiveCandidateNotSettled`（`registration_rejected` 桶）的引擎改口须改走警报桶。
- **S3（#623）**：备战档不得直接包装 `entries()` / `RetraceEntry` 公共字段（LOW-4）；短差档的三锁素材面（`not_constituted_reason()` + `terminal_evidence()`）已就位可用。
- **S4**：`journal()` / `JsonlRetraceLogStore` 作为 golden 事件日志锚可用，但对拍基线须固定在**未经恢复**的活账上——恢复后账本可能与活账在门卫钟与（MEDIUM-1 触发时）落锤钟上不同。
- **中枢账**：`CenterDeathCertificate` 载荷齐、可直接消费登记 Broken；但同锚多 `Confirmed` 期间（S2 之前）须自行去重（LOW-2）。

**5. 谱系引用** —— 本评审依据的已结算原则：`no-patch-mentality` 禁条 5「声明膨胀：在注释/文档中声明代码不具备的能力」（090 号严格性语法规则）—— 四条 MEDIUM 全部据此定性；`formalization-validity-domain`「有效域 ≠ 定义域」（231 号）—— MEDIUM-3 是其在"边界输入 fail-loud 声明"上的同构实例；`no-workaround`—— 本次未发现绕过概念矛盾的行为，无 `/escalate` 触发。买卖点身份账本域**尚无已结算谱系条目**（#574 契约是首份语义规约，本票是首次实装），故本评审不引用域内谱系；若 MEDIUM-1/-2 上浮成裁定，建议作为该域第一条谱系记录落盘。

**6. 影响声明** —— 本评审**零代码改动、零 git mutation**。产出仅一个文件：`chanlun/review-results/shadow-621-s1-review-20260728.md`（本文）。评审期间在 `rust/tests/` 临时建过 `shadow621_probe.rs` 并已删除（源码全文见 §7，可复现）；`git status -- rust/` 复核确认工作区无本车造成的任何改动。评审期间并行线推进 HEAD `9e44d98728` → `02ef5f93b1`（#497/#604/#610），指纹在两点复测一致，未受影响。本文不改动任何定义、模块或验收口径，仅登记事实与建议。
