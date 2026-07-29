# 影子评审：#622 S2 窗口纪律与引擎改口处死

- **评审车**：claude（opus，新上下文独立评审，未参与实装；单线程，无子代理）
- **日期**：2026-07-29
- **工位**：`/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`
- **评审面**：`git diff 73c7b34bea..d27f3bb80b -- rust/src/theta_v0/classifier/retrace_ledger/`
  （三 commit：`41b93bad86` + `ba0e055f81` + `d27f3bb80b`；#576 拆分区间外不评）
- **权限自律**：只读 + 跑测试；未改任何源文件；未执行任何 git mutation；自造探针
  `rust/tests/zz_shadow622_probe.rs` 已于评审结束时移出仓库（`git status` 复核无残留）

---

## 结论

**PASS WITH FINDINGS —— 1 HIGH / 3 MEDIUM / 5 LOW。**

票面三条 AC 的**行为**均已落地并被测试钉死：改口处死（窗口变 / 中枢消失 / 同窗口零动作 / 真新
departure 不误判）、死人挂号拒收（判据统一为 `death_certificate(anchor)`，与同身份迟到吸收两路
不混）、四类警报 audit 流（落流 / append-only / 结构上不进折叠）。移交五条中四条落位，第五条
（MEDIUM-1 门卫钟护栏）按编排者裁定未动、`log.rs` 在本次 diff 内零改动，声明与实际一致。

阻塞级问题一条：**改口处死是本票新增的、唯一绕开门卫钟的终态写入路径，且对知情时无任何下界
校验**——一个早于旧档出生钟的 `as_of` 会静默写出违反内核不变量「出生钟 ≤ 终态钟」的账本条目，
写入当场不报错，只在事后调 `assert_invariants()`（仅测试调用）才炸。两条通道（`observe` 碰撞、
`reconcile_window`）都中。已用独立探针实证。

---

## 独立复现（逐项照实报数）

| # | 项 | 实测 |
|---|---|---|
| 1 | `cargo test --lib` | **2175 passed / 0 failed / 137 ignored**，与对照一致 |
| 1 | `cargo test --lib retrace_ledger` | **92 passed / 0 failed**，与对照一致 |
| 2 | 改口处死探针（窗口变 / 中枢消失 / 同窗口零动作 / 真新 departure 不误判） | 全通过。窗口变 → `Invalidated` + `CenterRebased{registered_window, observed_window: Some(新窗口)}` + `center_rebased=1`；消失 → `observed_window: None`；同窗口 → `reconcile_window` 返回 `None` 且日志一个 bit 不动；旧档已终态时新 departure 正常注册、`center_rebased=0` |
| 3 | 死人挂号两路探针 | 全通过。Confirmed 后同锚新 departure → `DeadCenterReentry{anchor, attempted, death_certificate}`、零建仓、`dead_center_registrations` 与拒收次数恒等；同身份迟到 → `absorbed_late()`、`late_absorbed=1`、`dead_center_registrations` 不动。隔代序列（MEDIUM-4 场景）在活路上确已不可构造 |
| 4 | audit 流探针 | 四类事件落 JSONL 通过。**篡改 audit 文件**（就地把某条记录的 `as_of` 500 改成 123456）后 `fold(journal)` 逐条复现账本、`assert_invariants()` 通过 —— 裁定六成立。但同一次篡改**在 `load()` 侧被静默接受**（返回 `Ok(3)`，见 LOW-1） |
| 5 | 两拍守卫探针 | 通过。注册拍 side=Buy、落锤拍 side=Sell → `TerminalEvidenceContradictsRegistration`，条目一个 bit 不动 |
| 6 | MEDIUM-1 未动核 | `log.rs` 在评审 diff 内**零改动**（`git diff --stat` 未列出该文件），门卫钟语义声明段与 S1 一致 ✅。补注：`kill_as_rebased` 新增了一条 `admit` 之外的门卫钟前移写点（`book.rs:282-284`），方向单调向前、不违反非降，但「门卫钟只由 `admit` 推」这条 S1 口径已不再成立，doc 未提 |
| 7 | S1 回归 | `clocks.rs` / `log_replay.rs` / `portal.rs` / `standby.rs` / `short_retrace.rs` **零改动** ✅；但 `state_machine.rs` **不是只见追加**——既有测试 `confirmed_center_new_departure_is_counted_for_s2_hook` 被改写为 `confirmed_center_new_departure_is_rejected_as_dead_center_reentry`。这是裁定四落地的必然结果（S1 自己把该处标为 S2 接手点），属**正确改动**，仅订正「diff 只见追加」这一表述 |

> 工位并发注记：评审期间 #623 车在同一目录活动（`book.rs` / `portal.rs` / `tests/short_retrace.rs`
> 出现未提交改动）。以上指纹与探针均在同一工作树取得；#623 的改动落在
> `assert_entry_invariants` 的 `RetestReentered` 分支（`book.rs:617+`），与本报告全部结论无交集。

---

## HIGH

### HIGH-1 改口处死对知情时无下界校验，可静默写出违反内核不变量的账本

**位置**：`rust/src/theta_v0/classifier/retrace_ledger/book.rs:263-288`（`kill_as_rebased`）、
`:244-249`（`observe` 碰撞分支）、`:294-308`（`reconcile_window`）

`kill_as_rebased` 是本票新增的唯一**不经 `LedgerBook::admit`** 的终态落账路径。它拿到的 `as_of`
直接：① 单向前移旧档门卫钟（`:282-284`）；② 作为 `settle(...)` 的落锤时刻写进**首写永不改**的
`terminal_as_of`（`:285`）。全程没有任何「`as_of` 不得早于该档出生钟 / 门卫钟」的检查。

两条通道都暴露：

- `reconcile_window(anchor, window, as_of)` 是新增 **public API**，`as_of` 是调用方自由参数；
- `observe` 碰撞分支把**新档观察的 `as_of`** 用到**旧档**上——新档自己的 `open_on_observation`
  只保证新档自洽，对旧档零约束。

**实测失效场景**（探针 `p9b`）：

```
observe(frame(1_200), departure=3, as_of=500)   // 旧档注册，registered_as_of=500
observe(frame(1_400), departure=3, as_of=100)   // 同 departure、新窗口、陈旧知情时
→ 旧档 registered_as_of=500，terminal_as_of=Some(100)
→ 留档知情时序列 = [500, 500, 100]
→ 随后 assert_invariants() panic：
   ledger_kernel/mod.rs:446 「出生钟 ≤ 终态钟」
```

`reconcile_window` 同样（探针 `p9`：`registered_as_of=500`、`reconcile(..., as_of=100)` →
`terminal_as_of=Some(100)`，同一处 panic）。较弱的变体（`as_of` 只早于门卫钟、不早于出生钟）
不触发 panic 但同样把落锤钟写倒退：探针 `p2` / `p2b` 实测 `terminal_as_of=Some(600)` 而
`last_as_of=900`。

**为何算 HIGH**：

1. 破坏的是**内核级**不变量，不是域约定；且破坏发生在写入当场、**无任何报错**——
   `assert_invariants()` 只在测试里被 `settled()` 调用，生产路径不跑，损坏会一直潜伏；
2. 它同时违反 `assert_history_invariants`「留档知情时非降（append-only 时间纪律）」；
3. 与本模块自己的纪律相反：其余每一条终态写入都过 `admit` 的倒退门（`observe` 的判胜/判败、
   迟到吸收），provider 时间错乱一律 fail-loud（裁定二）。唯独改口处死静默吃下并写坏账；
4. `book.rs:245-246` 的注释「**不算倒退**（裁定五），无论 as_of 相对旧候选如何」只论证了
   「不该报 `RetrogradeRejected`」，没有论证「可以往回写落锤钟」——这两件事被合并处理了。

**可达性如实声明**：`RetraceLedger` 目前**零生产调用方**（全仓仅 `classifier/mod.rs:92` 一行
`pub mod retrace_ledger;`），故当前不可实发。但 `reconcile_window` 的 `as_of` 是外部自由入参，
接线当日即成活口。

**建议形状**（不代实施）：`kill_as_rebased` 入口取
`effective_as_of = max(as_of, entry.registered_as_of)`，或直接对 `as_of < entry.registered_as_of`
fail-loud 返回一枚新拒收码——与裁定二同族，而不是静默接受。

---

## MEDIUM

### MEDIUM-1 两拍守卫置于 `admit` 之前，把裁定三的「终态后迟到静默吸收」改成了 fail-loud

**位置**：`book.rs:331-334`（守卫在 `admit` 之前无条件施加）、`:363-386`

`advance_existing` 对任何带 `outcome` 的输入先跑两拍守卫，再 `admit`。后果：**已终态**身份收到
的迟到输入，只要证据与注册拍不一致，就返回 `Err` 而不是走裁定三的静默吸收，且
`late_absorbed` 不计数、`registration_rejected` 反而 +1。

**实测**（探针 `p8`）：Confirmed 后同身份迟到、`leave_end` 与注册拍矛盾 →
`Err`，`late_absorbed=0`，`registration_rejected=1`。

这是对裁定三既有语义的一次行为改动：迟到吸收的设计意图是「终态已固化，迟到输入改不动任何
东西，静默吞掉并计数供查 provider」。矛盾的迟到输入同样改不动任何东西，却变成了调用方必须
处理的错误；`dead_center.rs:89-108` 的「同身份迟到 ⟹ 静默吸收」测试只覆盖了一致证据的路径，
矛盾路径零覆盖、doc 零声明。

判据不在本报告——两种处置都能自洽（fail-loud 更严 / 吸收更贴裁定三）。缺的是：**选了哪一种、
为什么，以及对应的测试**。当前是「顺手的副作用」，不是「被声明的选择」。

### MEDIUM-2 `center_rebased` 是可由日志现算却不现算的计数，`fold`/`restore` 后归零

**位置**：`book.rs:75-78`（字段与 doc）、`:287`（自增）、`:489-497`（`alarms()`）；
`log.rs:147-161`（`fold` 不重建）

doc 声明改口处死「**不进四类 audit 流**——这是真实终态事件，走 `log` 的修订日志本体」。日志里
确实有 `NotConstituted{CenterRebased}` 修订，但 `alarms().center_rebased` 是独立可变字段，
`fold` 不重建它。

**实测**（探针 `p5`）：live `center_rebased=1`、journal 内 `CenterRebased` 修订数 `=1`、
`fold(journal).alarms().center_rebased == 0`。

与另外三个计数不同：那三个记录的是**未被采纳的输入**，日志里本就没有、恢复后归零是诚实的
（S1 已如实声明）。`center_rebased` 记录的是**已进日志的真实终态事件**，把它做成不重建的字段，
正是裁定六反对的「日志 + 状态双真相」形状，也违背本模块「任何可由留档现算的投影都不设字段」
（`mod.rs:301-303`）的自陈纪律。`assert_log_is_truth` 只比对 `book` 与 `active_by_anchor`，
不比对计数，所以现有测试无一条能发现它。

### MEDIUM-3 改口处死事件对消费方近乎不可见，且零测试覆盖

**位置**：`book.rs:191`（`guard_single_active` 把处死修订写进新档的 `delta`）、`:206`

`observe` 碰撞路径下，一次调用同时产生两件事：旧档被处死、新档注册（可能还判决）。但返回的
`RetraceStep` 只有一个 `key` / 一个 `state`，两者都描述**新档**。旧档之死只以一条 revision 的
形式混在 `step.delta` 里——消费方必须逐条比对 `revision.key` 才能发现「刚才有一个候选被处死」。

`tests/rebase.rs` 的 9 条测试全部通过**事后查 `book.entry(&旧键)`** 验证处死，**无一条断言
`step.delta` 的内容**。也就是说：改口处死作为「事件」的产出面（票面原话「处死记档…进工程警报
桶」的记档半边）在测试上是空的；如果 delta 里漏掉了那条 revision，现有测试集不会红。

`reconcile_window` 路径没有这个问题（返回的 `RetraceStep.key` 就是被处死的档）。

---

## LOW

### LOW-1 audit JSONL 无完整性指纹，就地改写记录载荷时 `load()` 静默通过

**位置**：`audit.rs:171-208`（`load`）、`:220-243`（`absorb`）

`absorb` 只查序号纪律（跳号拒绝 / 同序号异内容冲突）。把某一行**原地**改掉载荷（序号不变、
不产生重复行）时，读回完全无告警。探针 `p4`：把首条 `ResidualRejected` 的 `as_of` 由 500 改成
123456，`load()` 返回 `Ok(3 条)`。`log.rs` 至少有 `digest_records` 这一层指纹可用（虽只用于快照
溯源）；audit 侧没有对应物。裁定六（篡改不影响 `fold`）不受影响——这条只是说 audit 流本身的
取证价值在「外部改写」这一在案敞口下是空的。

### LOW-2 audit 持久化无游标、无 flush 接口，恢复后仍归零

**位置**：`audit.rs:129-169`（`JsonlRetraceAuditStore`）、`book.rs:499-502`（`audit_log()`）

账本只暴露只读切片，本身不落盘也不提供「已落到第几条」的游标（对照 `RetraceSnapshot::folded_through`）。
调用方要持久化只能每轮 `append_all(prov, ledger.audit_log())` 全量重投，靠 `absorb` 的幂等兜底，
写放大 O(n²)。且 `fold` / `restore` 后 `audit_log()` 为空、四类计数为 0，与 S1 同态——移交项 4
所指的「恢复后归零」在本票落地的是**容器与 wire 格式**，不是「恢复后不归零」。（裁定六禁止
audit 进真相恢复路径，所以不重建是对的；此处只订正移交项的闭合程度表述。）

### LOW-3 两枚新拒收变体挂在 adapter 的枚举上，逼出两处 `unreachable!()` 与反向依赖

**位置**：`adapter.rs:22`（`use super::book::CenterDeathCertificate`）、`:79-99`（两枚新变体）、
`book.rs:640-657`（`adapter_rejection_code` 两处 `unreachable!`）

`DeadCenterReentry` / `TerminalEvidenceContradictsRegistration` 由 `book` 产出，`admit_input`
永远产不出，却塞进了适配器的 `RetraceRejection`；结果是 adapter 反过来依赖 book 的类型，映射
函数不得不用 `unreachable!()` 填两个分支。严格形状是账本级另立一枚拒收枚举（或让映射函数只吃
适配器能产出的子集）。当前不可达，纯结构问题。

### LOW-4 两拍守卫只覆盖落锤拍，未决拍的证据矛盾被静默接受

**位置**：`book.rs:332-334`（`if admitted.observation.outcome.is_some()`）

未决拍（`outcome = None`）报出与注册拍矛盾的 `side` / `leave_end` 时，守卫不查、`judge` 直接
返回，输入被 `admit` 正常接受并**推动门卫钟**，不计警报、不落 audit。探针 `p6`：矛盾未决拍
`admission = Accepted`、`registration_rejected = 0`。最终落锤仍会与**注册拍**比对，故状态不会被
污染；但「provider 两拍改口」这个异常信号在未决拍上是丢失的，且被污染的门卫钟可能让后续
合法输入被判倒退。doc 未声明守卫的这一有效域。

### LOW-5 `reconcile_window` 无生产调用方，两类改口在实际管线里仍不可感知

**位置**：`book.rs:290-308`；全仓引用面 `classifier/mod.rs:92`

`observe` 碰撞路径只能感知「同 departure、同锚、窗口变」。裁定一的另两类偏离——**中枢从
provider 消失**、**无新 departure 时窗口变**——只有 `reconcile_window` 这一条通道，而它零调用方。
S1 同样无接线，接线归 S4，故不算本票欠账；如实记入以免「改口纪律已落地」被读成「管线已能
感知改口」。

---

## 未发现问题（正面确认）

- **裁定六结构性成立**：`fold` 的签名只吃 `&[RetraceRecord]`，`audit_log` 不可能进入折叠路径；
  篡改 audit 文件后 `fold(journal)` 逐条复现账本并通过全量不变量（探针 `p4`）。
- **死人挂号判据统一（MEDIUM-4）确已修复**：`death_certificate(anchor)` 前置于
  `guard_single_active`（`book.rs:219-232`），S1 的「前一注册是否 Confirmed」分支已从
  `link_restart` 删除，两查法并存消失；隔代序列在活路上不可构造（探针 `p3`）。
- **LOW-2 联动（同锚多 Confirmed）已补不变量**：`assert_at_most_one_confirmed_per_anchor`
  （`book.rs:584-592`）+ `assert_dead_center_invariant`（`:569-580`）。两条我都推了一遍可达性，
  在死人挂号拒收落地后确实不可达。
- **`fold` 与新路径兼容**：改口处死后 `active_by_anchor` 的重放值与活值一致（`replay_open` 在
  新档的 `Registered` 记录上覆盖锚指针，顺序正确），`assert_log_is_truth` 在全部新剧本下通过。
- **Standards**：`book.rs` 657 行、`audit.rs` 343 行，均在 800 行内；最长函数 50 行（压线）；
  `cargo test --lib` 零新增警告。（`cargo fmt --check` 在本模块有差异，但全仓 4895 处同类差异，
  rustfmt 显非本仓强制门，不计为 finding。）

---

## 结果包六要素

1. **结论**：PASS WITH FINDINGS，1 HIGH / 3 MEDIUM / 5 LOW；三条 AC 行为面均落地并有测试，
   HIGH-1 是本票新增路径引入的内核不变量敞口。
2. **定义依据**：#574 契约裁定一（改口 → 处死记档 + 新旧窗口对照 + 同窗口零动作）、裁定二
   （单活跃候选 + provider 有病 fail-loud）、裁定三（终态吸收 / 迟到静默吸收 + 警报）、裁定四
   （Success 后同中枢永禁新轮，账本自查死亡证明）、裁定五（三钟纪律：落锤钟首写不改、门卫钟
   查倒退）、裁定六（唯一真相 = append-only 日志；拒收/警报另记 audit 流、不进折叠）；#622 票面
   三条 AC；#621 移交五条 + MEDIUM-1 护栏裁定（方案②，本票不扩面）。
3. **边界条件（结论翻转的条件）**：
   - HIGH-1 若被裁定为「`as_of` 全局单调是 provider 的前置契约、账本不复查」，则降为 doc 缺陷
     （需在 `reconcile_window` 与 `kill_as_rebased` 显式声明该前置条件并补负例测试），HIGH 撤销；
   - MEDIUM-1 若裁定「矛盾的迟到输入本就该 fail-loud」，则降为「补一条测试 + 一句 doc」的 LOW；
   - MEDIUM-2 若裁定「`alarms()` 是 in-process 工程面，恢复后归零是全体计数的统一口径」，则降为
     doc 订正（把 `center_rebased` 的「走日志本体」措辞改成不承诺可观测恢复）；
   - LOW-5 在 S4 接线票落地后自动消解。
4. **下游推论**：#632 / #624 若消费 `RetraceStep`，需按 MEDIUM-3 自行扫 `delta.key` 才能拿到改口
   事件；任何接线 `reconcile_window` 的实施票必须先处置 HIGH-1，否则把自由 `as_of` 参数直接接到
   引擎即成活口。MEDIUM-1 护栏的 follow-up 票（编排者 2026-07-29 裁定另开）与 HIGH-1 属同一
   「落锤 as_of 下界」域，建议合并收口。
5. **谱系引用**：`no-patch-mentality`（声明膨胀禁止——MEDIUM-2 的 doc 与实际可观测性不符属此类）；
   `formalization-validity-domain`（LOW-4 / LOW-5：守卫与通道的有效域小于其声明域）；
   `no-workaround`（HIGH-1 的建议形状是补下界校验或 fail-loud，不是在下游加断言绕过）。
   本报告不涉及新的概念分离，无新增谱系条目。
6. **影响声明**：本次评审**未改动任何源文件**、未执行任何 git 操作；新增文件仅本报告
   `chanlun/review-results/shadow-622-s2-review-20260729.md`。评审探针
   `rust/tests/zz_shadow622_probe.rs` 为临时文件，已删除并经 `git status` 复核无残留。
   `ledger_kernel/`、`nest_lifecycle.rs`、#576 拆分面、#623 在跑面均未触碰。
