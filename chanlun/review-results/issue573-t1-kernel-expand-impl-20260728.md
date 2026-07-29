# #573 T1 expand 车收口报告（claude opus 实施车一）

> 日期：2026-07-28；性质：实施车一 stdout 收口报告原文（阶段 1 完成/阶段 2 因 #619 未提交阻，后按选项 1 解锁）
> 原始日志：/tmp/issue573-t1-dispatch-20260728.log

阶段 1 的代码本体已完成并全绿，但**两次 commit 都没做**——工作区在我作业期间被并行线改变，任何提交都会吞掉别人的未提交改动。以下逐项照实。

---

# #573 T1 收口报告（阶段 1 完成 / 阶段 2 阻塞）

## 0. 先说结论

**内核写完了，28 个测试全绿，Standards 全过。但我没有 commit，也没有做阶段 2。**

原因是一句话：我开工时干净的 `nest_lifecycle.rs`，在我做基线的这段时间里，被并行线（票 #619）改了 **411 行**并留在未提交状态；同时 `classifier/mod.rs` 被并行线 `git add` 进了暂存区。票面铁律写死「并行线未提交面永不碰」——而那个面现在**就是**我阶段 2 的唯一目标文件；`mod.rs` 则让阶段 1 的 commit 也无法只装我自己的东西。

---

## 1. 两阶段 commit SHA

| 阶段 | 计划 commit | 实际 |
|---|---|---|
| 1 expand | `refactor(theta): #573 T1 expand——…` | **未提交** |
| 2 migrate | `refactor(theta): #573 T1 migrate——…` | **未执行** |

**commit 1 未提交的具体理由**（不是我怕麻烦，是三条路都撞铁律）：

内核要能编译进 crate，必须在 `classifier/mod.rs` 注册。而该文件此刻状态是 `MM`：

- 暂存区里已经有并行线 #619 加的 **15 行**（一段测试文档注释，`incremental_tower_scaling_dominates_full_synthetic` 的时间敏感登记）；
- 工作区里再叠了我的 **3 行** `pub mod ledger_kernel;`。

于是：

1. `git add rust/src/theta_v0/classifier/mod.rs` → 把并行线那 15 行一起提交，等于替别人交作业，且他们再提交时会发现内容已被吞（正是 memory 里「并发预 stage 混批」那条教训的现场重演）；
2. `git commit --only` 只提交 `ledger_kernel/` 两个新文件 → 模块未注册，不进编译树，是个不编译的死文件 commit，`no-patch-mentality`「不留半成品」直接否掉；
3. 不提交 → 就是现在这样，工件留在工作区。

三选一里没有一条是严格的，所以我选了不制造既成事实的那条，把裁定交回去。

**工件已备份**到 `/tmp/issue573-kernel-artifact/`（`ledger_kernel/{mod,tests}.rs` + `mod.rs` 快照），防止并行线清理工作区时丢失。

---

## 2. 测试指纹

| 时点 | lib | 全量（`--no-fail-fast`） |
|---|---|---|
| 票面在案基线 | 2040 / 1 / 137 | — |
| **我实测基线**（开工时） | **2044 / 1 / 137** | **2106 / 1 / 146** |
| 阶段 1 收口 | 2075 / 1 / 137 | 2137 / 1 / 146 |

- 与票面在案基线差 **+4 passed**：不是我引入的，开工快照就是 2044。票面记录的 2040 应是更早时点的读数。
- 阶段 1 净增 **+31**，其中**我新增 28 个**（`ledger_kernel` 全部），另外 **3 个是并行线**新加的 `tests/issue533_p123_byte_guardrail.rs` 字节护栏测试（在我跑基线之后落进工作区的）。
- 唯一的红 = `extract_signals_bit_exact_digest_guard`（#491，在案已知恒红，与本票无关）。

---

## 3. p123/p92 对拍

**基线已取全**（存 `/tmp/issue573-baseline/`），阶段 2 未执行故**无对拍可做**。基线可信度有交叉证据：

| 面 | SHA-256（前 16） | 与 #421 自查档 §8.6 |
|---|---|---|
| p123 20k stdout | `bd9ac1d655f9d615` | **逐字相同** ✓ |
| p123 20k P116 dump | `fcc8016a9a01a109` | **逐字相同** ✓ |
| p123 100k stdout | `d8b69c180c23c5e3` | **逐字相同** ✓ |
| p123 100k P116 dump | `8a7327feb3b9ba29` | **逐字相同** ✓ |
| p123 20k/100k lifecycle dump | `8ee16b36d5f22467` / `d46f78e236f4dbf9` | 本票新锚 |
| p92 20k/100k stdout | `91c937aa423f14b5` / `7dd3b35f149c566b` | 本票新锚 |

⚠ **这套基线已经过期**：它取自 `nest_lifecycle.rs` 尚干净的快照，而并行线那 411 行落地后 nest 行为面已变。阶段 2 开工前必须重取。

---

## 4. 内核公共面 + 11 责任点落位

新模块 `rust/src/theta_v0/classifier/ledger_kernel/`（`mod.rs` 579 行 + `tests.rs` 677 行）。

### 四组类型参数的实际签名

```rust
pub trait LedgerPolicy: Sized {
    type Key:          Copy + Ord + fmt::Debug;              // 等值 + 序
    type Observation;                                         // 建项入口输入
    type RevisionKind: Copy + PartialEq + fmt::Debug;         // 修订词汇
    type Reason:       Copy + PartialEq + fmt::Debug;         // ┐ ReasonPayload
    type Evidence:     Copy + PartialEq + fmt::Debug;         // ┘
    type Entry:        LedgerEntryCore<Self>;                 // 域自选字段布局
    fn observation_key(observation: &Self::Observation) -> Self::Key;
    fn opened_revision_kind() -> Self::RevisionKind;
}
```

「TransitionPolicy」这一组按 spec 拆成两半落地：**判据留域侧**（内核不认识桥规则/业务条件），**拒绝与终态映射进内核**——`LedgerAdmission`（倒退拒绝 + 终态吸收）与 `LedgerSettlement{state, kind, reason, evidence}`（终态落账）。这是刻意的：判据一旦进内核签名，nest 的力度谓词就会渗进来，违反「禁把 nest 域概念渗进内核」。

`ReasonPayload` 我实现为 `Reason` + `Evidence` 两个关联类型而非一个复合类型——因为两者的可空性独立（正向终态无原因码、材料缺时证据诚实为 `None`），捆成一个会逼出一个恒有一半是空的复合结构。

### 11 责任点逐条落位

| # | 责任点 | 文件:行 |
|---:|---|---|
| 1 | per-key 注册表 | `ledger_kernel/mod.rs:289` `LedgerBook.entries` + `:300-334` 读面 |
| 2 | 首次观察建项 | `:352` `open_on_observation` |
| 3 | append-only 修订追加 | `:253` `LedgerEntryCore::push_revision`（全内核唯一追加点） |
| 4 | 修订计数与历史一致 | `:253` 同上——计数由留档长度**现算**，结构上不可能不一致 |
| 5 | per-identity 倒退拒绝 | `:379` `admit` + `:370` `reject_retrograde`（后者供建仓前的判同分支用） |
| 6 | 终态吸收 | `:70` `LedgerState::is_terminal` + `:379` `admit` → `TerminalAbsorbed` |
| 7 | 钟首次写入不后移 | `:155` `first_write_clock`（已写则既不后移也不前移） |
| 8 | 每次推进返回增量 | `:168` `LedgerDelta` + 全部产修订原语返回 `LedgerRevision` |
| 9 | 身份迁移保留历史 | `:403` `migrate`（旧键退表→改键→留痕→追加修订，钟一个 bit 不动） |
| 10 | 只读枚举/过滤门户 | `:324` `keys` / `:328` `entries` / `:332` `values` / `:337` `in_state` |
| 11 | 不变量骨架 | `:425` `assert_core_invariants` + `:449` `assert_history_invariants` |

### 测试覆盖（28 个，全绿）

11 责任点各有正面测试；另有 **7 个 fail-loud 负控**（`should_panic`）：非终态落账、迁移同键、迁移源不存在、计数被篡改、有终态无终态钟、注册表键失同步、留档时序倒退。测试域 `Probe*` 是纯合成的对象无关消费方——文件里出现任何 nest 词汇都说明内核漏了域概念，这是自查手段。

### Standards 硬杠

| 项 | 结果 |
|---|---|
| 函数 ≤50 行 | ✓ 全过（脚本扫描零命中） |
| 文件 ≤800 行 | ✓ 579 / 677（预判会超才拆的目录形式） |
| 错误 fail-loud | ✓ 无静默吞错，7 个负控钉死 |
| 常量具名 | ✓ 无魔数 |
| 零新增编译警告 | ✓ `cargo check --lib` 中 `ledger_kernel` 零命中 |

---

## 5. nest 侧留存责任

阶段 2 未执行，这是**规划**不是既成事实。9 个假设域责任点全部留 nest：六元组 `LifecycleKey`、`bridge_identity`/`same_anchor`/`bridge_by_center_upgrade` 三个判同判据、三态业务条件、`InvalidatedReason`/`VanishCause`、`LifecycleRevisionKind` 修订词汇、五钟中的 `first_provable_at`/`structure_end_at`、`PanLiveWindow`/`ReplayBarFeed` provider 与 feed、`ForceCheck`/`ForceEvidence` 力度判定、完成信号与消失扫描结算。

**重基方案已就绪**（读完全部材料后定形，解锁即可执行，无需重新设计）：

- `NestEventState` → `pub type NestEventState = LedgerState`（三变体同名，测试与 p409 零改动）；
- `LifecycleRevision` → `pub type = LedgerRevision<NestPolicy>`（四字段逐位同构）；
- `RetrogradeRejection` → `pub type = LedgerRetrogradeRejection<LifecycleKey>`；
- `NestLifecycleEntry` **公共字段面一个 bit 不动**（p123/p409 零改动是硬约束），改为 `impl LedgerEntryCore<NestPolicy>`；
- `NestLifecycleBook` 内部换成 `ledger: LedgerBook<NestPolicy>`，完成信号/力度不可验审计两个 Vec 留 nest 侧；
- `advance` 第 1 步桥迁移 → `ledger.migrate(...)`；第 2/3 步 → 一次 `ledger.admit(...)`（调研报告说的「可原样抽出」）；第 5/7 步钟首写 → `first_write_clock`；`invalidate` 与 Confirmed 分支 → `entry.settle(LedgerSettlement{...})`；`delta` → `LedgerDelta`；
- `consumable_closed` → `in_state(Confirmed)`；`lineage_nodes` → `values().collect()`；`assert_invariants` 首行调 `assert_core_invariants()` 后接既有域断言（既有断言语义一条不改）。

行数预期净降（`push_revision`/`invalidate`/容器样板搬走）。第 8 步消失扫描**不抽**——它不在票面 11 项内，留 nest 可让 diff 面更小。

---

## 6. 遗留 / 偏离 / 存疑

### 阻塞项（需编排者裁定，这是本报告的核心）

**阶段 2 不可执行。** 按局部依赖原则自查：我的工作输入**确实**是 `nest_lifecycle.rs` 的当前内容，而它确实是并行线未提交面的输出——是真阻塞，不是把局部矛盾放大成全局。

并行线 #619 落在 `nest_lifecycle.rs` 的 411 行改动，改的正是我要重基的区域：修订词汇新变体、`NestLifecycleEntry` 新字段、`settlement_stats`、`advance` 第 1 步的认领/迁移分支、新私有方法 `migrate_entry`、3 个新测试。就算铁律允许我碰，在别人未提交的半成品上重基也做不出可验证的 bit-exact——基线锚不住。

**三个解锁选项，请裁：**

1. **等 #619 提交后重发本票**（我倾向这条）。HEAD 推进后工作区干净，基线重取，阶段 1 工件从 `/tmp/issue573-kernel-artifact/` 恢复即可，只需重跑测试；阶段 2 一次做完。代价 = 一轮等待。
2. **只放行阶段 1**：授权我把 `mod.rs` 那 15 行 #619 注释一并带进 commit 1，并通知 #619 线该段已被提交。代价 = 一次可控的混批 + 一条需人工传达的信息。
3. **由 #619 线代为注册模块**：他们在自己的 commit 里加 `pub mod ledger_kernel;`，我只提交两个新文件。代价 = 跨票耦合。

### 偏离登记

- **票面在案基线 2040/1/137 与实测 2044/1/137 不符**（+4 passed）。已按票面「先报再继续」处理；差额来自 HEAD 时点差，非本票引入，也非并行线未提交面（那 4 个在 lib 内，而未提交的 `p100_cert_bsp_recon.rs` 是 bin）。
- **replay 基线已过期**，原因见 §3。

### 存疑（留给影子评审）

- 内核不变量「留档时序非降」和「首条修订为建项词汇」是我加的**新**约束，nest 现状是否全程满足**尚未在真实 replay 上验证**——只在探针域验过。阶段 2 一跑 p123 就会暴露；若 nest 某条路径违反，正确处置是把该条从内核不变量降级（而非改 nest 行为），并在报告登记。这是阶段 2 的第一个风险点。
- `NestEventState` 走类型别名会让它的三个变体 doc 从 nest 侧消失（移到内核）。语义无损，但「域词汇的文档归属」是个口径问题，影子评审可能有异议。

### 未做（照实）

影子评审不归我（票面明示）；`gh` 上没有回写任何评论（票面未授权，且 blocked 状态下不该由我单方面登记）。
