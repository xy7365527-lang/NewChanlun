# 影子评审：#573 T1 账本内核抽取（expand `94acacd4d2` + migrate `d40607d805`）

> 票据：#573（Parent #59，#465 裁定 A 之 T1）。角色：影子评审执行层——本 session **未参与 #573 任何实装**，
> 只读复核 + 自跑复现；未改仓内任何既有文件，只写本报告；全程前台单线程，未派 Task/子代理/后台任务；
> 零 git mutation（无 commit/stash/checkout/worktree）。
> 基线：分支 `kimi-nest-mainline-20260717` @ `d40607d805`（HEAD），对照点 `886a439c78`。
> 独立产物（仓外、不提交）：`/tmp/shadow573-recon/`（我自跑的 8 面 replay）、`/tmp/shadow573-full-r{1,2,3}.txt`
> 与 `/tmp/shadow573-full-run1.txt`（4 轮全量测试）、`/tmp/shadow573-blob/`（HEAD blob 导出，比对锚）。
> **对拍基线**沿用交付方 `/tmp/issue573-baseline2/`（那是「expand 后 / migrate 前」的纯态锚，本 session
> 无法在不做 git mutation 的前提下重建该时点；其可信度由「8 面全部与我自跑的 migrate 后产物逐字节相同」
> 反向支撑——若基线被做过手脚，migrate 后重跑不可能字节命中）。

## 0. 结论

**PASS WITH NOTES。**

**票面唯一验收尺（nest 行为 bit-exact + 不变量保持 + 零生产语义变更）四条通道全部独立复现，无一处读数造假、
无 HIGH 级缺陷：**

- **指纹**：`--lib` = **2075 / 1 / 137**、全量 `--no-fail-fast` = **2137 / 1 / 146**，与实装报数逐数相同；
  唯一红 = `extract_signals_bit_exact_digest_guard`（#491 在案恒红，与本票无关）。
- **对拍**：p123 20k/100k × {stdout, `P116_DUMP`, `P421_LIFECYCLE_DUMP`} + p92 20k/100k stdout，
  共 **8 面 `cmp` 逐字节全同，diff_count=0**（非只比 SHA）。p123 stderr 唯一差异是墙钟 `prefix_s`，
  全部计数字段逐字相同。release 二进制含内核新 panic 串（`准入要求条目已建仓`/`修订留档长度溢出`），
  证明是真编译产物而非缓存假绿。
- **公共面**：`NestLifecycleEntry` 结构体定义 41 行**逐字节相同**；p123/p409 零改动（两 commit 只碰 4 个文件，
  全在 `classifier/` 下）。
- **不变量**：`assert_invariants` 155 → 159 行，唯一变化 = 3 行注释 + 1 行 `self.ledger.assert_core_invariants()`
  + 容器名替换，**既有域断言一条未删未改**（逐行 diff 自证）。
- **域边界**：`ledger_kernel/` 的**签名与命名面零域词汇**（nest/pan/force/背驰/力度/中枢/bridge/seg_ 全零命中），
  仅模块头 doc 的「由来 / 边界」说明提及 nest —— 属溯源引用，合法。

**Notes 来自 5 条 MEDIUM + 6 条 LOW，全部集中在同一个主题：内核作为「通用件」对第二消费方的守卫与
契约缺口，以及若干超出实际能力的声明措辞。当前 nest 侧行为零影响**（每条都已逐一验证 nest 调用点有前置保护）。

| # | 级别 | 一句话 |
|---|---|---|
| M1 | **MEDIUM** | `LedgerBook::migrate` 只 assert 两端不同键，**不校验目标键空闲** ⟹ 命中即 `BTreeMap::insert` 静默覆盖并丢弃目标条目的全部留档，append-only 承诺被绕过。nest 两个调用点都在 `!contains(&key)` 内故当前零影响；第二消费方无此保护。 |
| M2 | **MEDIUM** | `LedgerEntryCore::settle` **无「已终态禁再落账」守卫**，`write_settlement` 也不走 `first_write_clock` ⟹ 双落锤时两只终态钟同时有值，而 `assert_core_invariants` 逮不住（`settled_at()` 用 `.or()` 合并，仍是 `Some`）。内核 doc 声称 `LedgerState` 承载「终态吸收、禁复活、终态钟只写一次」三条纪律，实际只有「吸收」经 `admit` 提供，另两条落在域侧。nest 由 `advance` 的 `admit` 前置 + 域断言「终态互斥」双重兜住。 |
| M3 | **MEDIUM** | 「修订计数 == 留档长度是**结构性保证**／**结构上不可能不一致**」= 声明膨胀：`revisions_mut()` 是 pub trait 方法，`get_mut` 外发 `&mut Entry`，`NestLifecycleEntry.revision`/`.revisions` 仍是 pub 字段——绕过路径公开可达，内核自己的负控测试就在演示（`entry.revision = 7`）。 |
| M4 | **MEDIUM** | 「泛化面 = 四组类型参数」这句验收尺与实装不符：第三组 **TransitionPolicy 根本不是类型参数**（`LedgerAdmission`/`LedgerSettlement` 是固定具体类型，域不可替换其形状）；同时多出**第五个结构性参数 `Entry`**（域持有存储 + 14 个访问器），后者未作为 spec 偏离登记。 |
| M5 | **MEDIUM** | #574 裁定六「唯一真相 = append-only 日志、状态 = 日志折叠、状态不单独持久化」与内核的「状态为主字段、留档为旁记」结构**方向相反**，内核零 fold/apply 钩子 ⟹ T3 无法在内核上兑现裁定六。属 T1 出界（spec 已声明 T3 接口待回填），登记为 #575 定形风险，不阻塞本票。 |

M1/M2 是**潜伏守卫缺口**（当前数据与当前调用点都不触发，故实装车无从发现）；M3/M4 是**措辞 vs 能力**；
M5 是**跨票定形风险**。均不构成 FAIL —— 票面唯一验收尺是 nest 行为 bit-exact，该尺四通道全绿。
建议：M1/M2 随本票直接补两行守卫（各 1 行 assert，零行为影响、零 diff 风险）；M3/M4 改措辞；M5 进 #575 票面。

---

## 1. 票面验收尺逐条对照

| # | 验收项 | 判定 | 我的独立证据 |
|---|---|---|---|
| 1 | 泛化面 = key/观察/转移策略/原因载荷 四组类型参数 | **PASS WITH NOTES** | §5 + **M4** |
| 2 | nest_lifecycle 全部既有语义改经内核表达 | **PASS** | §4 逐 hunk |
| 3 | nest 行为 bit-exact：既有测试全绿 | **PASS**（2075/1/137、2137/1/146 自跑） | §2 |
| 4 | p123/p92 replay 对拍零 diff | **PASS**（8 面 `cmp` diff_count=0，自跑） | §3 |
| 5 | `assert_invariants` 全保 | **PASS**（既有域断言一条未删未改，逐行 diff） | §4.5 |
| 6 | 零生产语义变更 | **PASS**（三处登记偏离外零行为性改动） | §4 |
| 7 | 指纹前后对照 + 并行线未提交面先核 | **PASS**（并核出交付方口径外的一条负载敏感偶发红，见 **L5**） | §2 + §10 |
| 8 | Standards：函数 ≤50 行 | **PASS**（内核零函数 >50 行，脚本扫描） | §9 |
| 9 | Standards：文件 ≤800 行 | **PASS**（内核 579 / 677）；宿主 5411 属存量超限，本票 +85 | §9 + **L4** |
| 10 | Standards：错误 fail-loud / 常量具名 | **PASS**（7 个 `should_panic` 负控；无魔数） | §6 |
| 11 | 11 责任点落位属实 | **PASS**（逐条核，非抽 3 条） | §6 |
| 12 | 9 域留存无越界渗进内核 | **PASS**（签名/命名面零域词汇） | §6.2 |
| 13 | 第二消费方（买卖点账本）能真实落地 | **PASS WITH NOTES** | §7 + **M1/M2/M5** |
| 14 | 禁 git mutation / worktree 纪律 | **PASS**（交付方两 commit 只碰 4 文件，11 个并行线未提交面一个未吞——我逐一核过） | §10 |

---

## 2. 指纹（独立复现）

```
cargo test --lib            → 2075 passed; 1 failed; 137 ignored     ← 与实装报数逐数相同
cargo test --no-fail-fast   → 2137 passed; 1 failed; 146 ignored     ← 与实装报数逐数相同（3/4 轮）
唯一红：theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard
        （signal.rs:3418，#491 在案恒红，摘要 0xe6a263e343e43845，与本票无关）
```

内核测试 `cargo test --lib ledger_kernel` = **28 passed / 0 failed**，与实装报数相同；
编译输出 lib 警告 **37 条**（与实装报数相同），`ledger_kernel` 在警告面**零命中**。

**偏离登记（L5）**：我跑了 4 轮全量，其中 **1 轮出现 2136 / 2 / 146**（多一条红，名未捕获——该轮输出直接管进
了聚合脚本）。随后连跑 3 轮全部回到 2137/1/146。仓内 `classifier/mod.rs:3725-3738` 已由 #619 L9 把
`incremental_tower_scaling_dominates_full_synthetic` 登记为**墙钟比判据、对机器负载敏感、隔离单跑恒绿**的
固有偶发红，并明写「与 #491 无关」。该轮异常与此登记吻合。**结论：非本票回归**；但实装报告「逐数相同」
应附「该测试负载敏感，指纹须多轮取模」的口径，否则下游复核者单跑撞红会误判。

---

## 3. 对拍（8 面，我自跑）

复现命令（`../analysis/data_cache/btc_1m_full.json`，release）：

```bash
P116_MAX_BARS={20000,100000} P116_CKPT=0 \
  P116_DUMP=$O/p123-$t.dump P421_LIFECYCLE_DUMP=$O/p123-$t.lifecycle \
  ./target/release/p123_fast_replay $D > $O/p123-$t.stdout 2> $O/p123-$t.stderr
P92_MAX_BARS={20000,100000} P92_CKPT=0 \
  ./target/release/p92_nest_replay_postruling $D > $O/p92-$t.stdout 2> $O/p92-$t.stderr
```

| 面 | `cmp` vs `/tmp/issue573-baseline2/` |
|---|---|
| p123-20k.stdout / .dump / .lifecycle（5.5 MB） | **SAME / SAME / SAME** |
| p123-100k.stdout / .dump / .lifecycle（26.7 MB） | **SAME / SAME / SAME** |
| p92-20k.stdout / p92-100k.stdout | **SAME / SAME** |
| p92-20k.stderr / p92-100k.stderr（额外面） | **SAME / SAME** |
| p123-20k/100k.stderr（额外面） | DIFF —— **唯一差异为墙钟 `prefix_s`**（0.462→0.148、3.032→3.725）；`triggers/reevals/pan_*/wm_cross_*/term_*/shadow_*` 等**全部计数字段逐字相同** |

**diff_count = 0（8 面）。**

**二进制真编译自证**（memory 在案：`cargo check` 重放缓存 = 假绿）：`target/release/p123_fast_replay`
mtime `21:49:59` 晚于 `nest_lifecycle.rs` mtime `21:47:01`；且二进制内 `grep -a` 命中内核**新增**
panic 串 `准入要求条目已建仓`（`ledger_kernel::admit`）与 `修订留档长度溢出`（`push_revision`）各 1 处
—— 这两个串在 migrate 前的代码里不存在，故二进制确由 migrate 后源码编出。

---

## 4. 零生产语义变更（逐 hunk 审）

`git diff 886a439c78...HEAD -- .../nest_lifecycle.rs` 共 12 个 hunk，逐个核：

### 4.1 三个具体类型 → 类型别名（hunk 2/3/6）

`NestEventState`→`LedgerState`、`LifecycleRevision`→`LedgerRevision<NestPolicy>`、
`RetrogradeRejection`→`LedgerRetrogradeRejection<LifecycleKey>`。字段名、字段序、`pub` 性、derive 集合逐项核：

| 类型 | 旧 derive | 新 derive | 判定 |
|---|---|---|---|
| `LifecycleRevision` | `Debug, Clone, Copy, PartialEq` | 手写 `Clone/Copy/PartialEq/Debug`（`ledger_kernel/mod.rs:479-505`） | 等价 ✓（手写是为避开 `derive` 给 `P` 加多余 bound，注释已说明） |
| `RetrogradeRejection` | `Debug, Clone, Copy, PartialEq, Eq` | 同 | ✓ |
| `NestEventState` | `Debug, Clone, Copy, PartialEq, Eq` | `+ PartialOrd, Ord` | **纯扩张**，见 **L3** |

三个变体名与序（`Provisional/Confirmed/Invalidated`）不变 ⟹ 既有 `match` / 模式全部继续编译且语义不变。

### 4.2 建仓路径（hunk 8）

旧：手工构造 `LifecycleRevision` + 手工填 15 个字段的 `NestLifecycleEntry`（`revision: 1`、
`revisions: vec![revision]`）→ `entries.insert`。
新：`P::Entry::open(key, as_of)`（`revision: 0`、`revisions: vec![]`）→ `push_revision(Observed)`
（置 `revision = len() = 1`）→ `insert`。**终态逐字段相同** ✓。
`claimed_from` 臂：旧「先在局部 entry 上写 `superseded_from` + `push_revision(CenterUpgraded)` 再 insert」
→ 新「insert 后 `get_mut` + `set_migrated_from` + `push_revision`」，**delta 入列顺序（Observed → CenterUpgraded）
与终态字段均相同** ✓。

### 4.3 第 2/3 步 → `ledger.admit`（hunk 9）

旧序：倒退判 → `entry.last_as_of = as_of` → 终态判 → `continue`。
新序（`ledger_kernel/mod.rs:379-395`）：倒退判（+注记，零改写）→ `set_last_as_of` → 终态判 → `TerminalAbsorbed`。
**逐步同序**，含「终态条目的门卫钟仍然前移」这一容易漏掉的细节 ✓。
桥迁移臂（hunk 8 上半）的「倒退拒绝用**新键**记注记」「终态桥匹配 `continue` 且**不**前移门卫钟」两条
旧行为也逐字保持 ✓。

### 4.4 钟首写 / 终态落账（hunk 10/11/12）

- 第 5 步：旧 `if entry.first_provable_at.is_none() && force == Verified(true)` → 新
  `if force == Verified(true) && first_write_clock(&mut entry.first_provable_at, as_of)`。
  **短路序被交换**，但 `first_write_clock` 有副作用 ⟹ 必须 `force` 判在前才等价——**实装正好是这个序** ✓。
- 第 7 步 `structure_end_at`：同款替换 ✓。
- `invalidate`：旧 6 条赋值 + `push_revision` → 新 `settle(LedgerSettlement{...})`；
  `write_settlement`（`nest_lifecycle.rs:542`）的 Invalidated 臂逐条重现旧的
  `invalidated_at / invalidated_reason / vanish_cause 投影 / force_evidence` ✓。
- 新增 `confirm()`（`:590`）：`state=Confirmed` + `confirmed_at` + `push_revision(Confirmed)`，
  与旧第 7 步的三行内联逐条同 ✓；Confirmed 臂**不碰任何 Invalidated 侧字段**，与旧同 ✓。

### 4.5 `assert_invariants`

155 → 159 行。`diff` 唯一 hunk = 3 行口径注释 + `self.ledger.assert_core_invariants();` +
`self.entries.values()` → `self.ledger.values()`。**既有域断言一条未删、未改、未重排**（含终态互斥、
`Confirmed 必有 confirmed_at`、「从未构成 ⟹ invalidated_at == structure_end_at」、两码互斥且穷尽等）。
与实装登记的「+4 行内核调用」完全吻合 ✓。

### 4.6 三处登记偏离的核判

| 偏离 | 我的判定 |
|---|---|
| 修订计数 fail-loud 化（`revision += 1` → `u32::try_from(len()).expect`） | **同意**。`revisions.len()` 触不到 `u32::MAX`，两 profile 一律 fail-loud 是纪律加强而非行为改动；纪律方向与 `no-patch-mentality`「错误 fail-loud」一致。 |
| 访问器样板 +85 行 | **同意其为事实**，但**不同意「访问器样板与三处别名 doc 是净增项」这一归因完整性** —— 净增的根因是把 `Entry` 做成域持有的第五个参数（见 **M4/L4**）。 |
| `assert_invariants` +4 行 | **同意**，逐行 diff 自证。 |

**除上述三处外，逐 hunk 未发现任何行为性改动。**

---

## 5. 四组泛型参数 vs spec / 契约

spec T1 章（`spec-kernel-and-retrace-ledger-20260728.md:75-76`）：
「泛化面 = 四组类型参数：Key（等值 + 序）、Observation、TransitionPolicy（含判据/拒绝/终态映射）、
ReasonPayload（原因码 + 证据载荷）」。

实装 `LedgerPolicy`（`ledger_kernel/mod.rs:80-99`）实际暴露 **6 个关联类型 + 2 个方法**：

| spec 的组 | 实装 | 判定 |
|---|---|---|
| Key（等值 + 序）| `type Key: Copy + Ord + Debug` | ✓ 兑现（`Copy` 是额外收紧，见 §7） |
| Observation | `type Observation`（无 bound） | ✓ 兑现 |
| TransitionPolicy（判据/拒绝/终态映射）| **不是类型参数**：判据留域侧；拒绝 = `LedgerAdmission`（具体 enum）；终态映射 = `LedgerSettlement<P>`（形状固定的具体结构） | **M4**：这一组**消失了**。实装报告写作「按 spec 拆成两半落地」，但拆出来的两半是**内核固定类型**，域无法替换其形状——不是参数化。 |
| ReasonPayload（原因码 + 证据载荷）| `type Reason` + `type Evidence` 两个关联类型 | ✓ 兑现（拆两半的理由——可空性独立，避免恒有一半为空的复合结构——**我同意**） |
| —（spec 无）| `type RevisionKind` | ✓ 合理：「修订词汇」是 9 域留存之一，作为参数由域供给正是正确形态 |
| —（spec 无）| `type Entry: LedgerEntryCore<Self>` + **14 个访问器方法** | **M4**：第五个**结构性**参数，spec 四组之外，**未作为偏离登记**。 |

**我对 TransitionPolicy 这一组消失的实质判断**：不违约。#465 的 11 个对象无关责任点里**本来就没有
「转移判据」** —— 内核承载的是容器/留档/钟/准入，不是状态机。把力度谓词挡在内核外的理由我完全同意
（否则 nest 域概念直接渗进内核签名，违反票面「9 域留存」）。**问题只在验收话术**：「四组类型参数」
这句尺子按字面读是不成立的，实装报告的登记也只说了「拆成两半」而没点明「这一组不再是参数」。
建议关票时把该句订正为：**「泛化面 = Key / Observation / RevisionKind / ReasonPayload(Reason+Evidence) /
Entry 五组类型参数；转移判据全留域侧，内核只提供 `LedgerAdmission`（拒绝）与 `LedgerSettlement`（终态落账）
两个固定机制」**。

**`Entry` 这一参数的代价（L4）**：域持有条目布局 ⟹ 内核只能经 14 个访问器读写 ⟹ nest 侧多出 +85 行样板，
宿主文件 5326 → **5411**（第一车 §5 曾预期「行数净降」，实际反向）。这个选择本身我**同意**——它正是
`NestLifecycleEntry` 公共字段面能做到「一个 bit 不动」（p123/p409 零改动硬约束）的原因，替代方案
（内核持有条目）会强制改公共字段面从而砸掉唯一验收尺。但代价与归因应写进报告，而不是记成「访问器样板」。

---

## 6. 11 责任点落位 + 9 域留存边界

### 6.1 11 责任点（逐条核，非抽验）

| # | 责任点 | 内核落位（`ledger_kernel/mod.rs`）| 我的核判 |
|---:|---|---|---|
| 1 | per-key 注册表 | `:290 entries: BTreeMap<P::Key, P::Entry>` + `:304-345` 读面 | ✓ nest `bridge_match` 经 `ledger.keys()` 仍是 BTreeMap 序，**枚举确定性不变** |
| 2 | 首次观察建项 | `:352 open_on_observation` | ✓ 已建仓返 `None` 且零改写；nest 建仓臂 `.expect("建仓分支的身份此前不在册")` fail-loud |
| 3 | append-only 修订追加 | `:253 push_revision` | ✓ 无删除路径；`revision.key` 取追加时当前键（迁移后为新键），与旧语义同 |
| 4 | 计数与历史一致 | `:266` 计数由 `revisions().len()` 现算 | ✓ 行为等价；「结构性保证」的措辞见 **M3** |
| 5 | per-identity 倒退拒绝 | `:379 admit` + `:370 reject_retrograde` | ✓ 零修订零改写 + 显式注记；建仓前分支也可记（nest 桥臂用到） |
| 6 | 终态吸收 | `:70 is_terminal` + `:391 TerminalAbsorbed` | ✓ 禁复活；但「禁复活」只在域主动调 `admit` 时生效，见 **M2** |
| 7 | 钟首写不后移 | `:155 first_write_clock` | ✓ 已写则既不后移也不前移（测试 `:419-427` 双向钉死）；但**终态钟不走此函数**，见 **M2** |
| 8 | 推进增量 | `:168 LedgerDelta` + 全部产修订原语返回 `LedgerRevision` | ✓ 留档仍在账本内，增量不替代留档（测试 `:444-465` 钉死）；部分 API 无生产消费方，见 **L2** |
| 9 | 身份迁移保留历史 | `:403 migrate` | ✓ 退表→改键→留痕→追加→入表，与旧 `migrate_entry` **逐步同序**；目标键校验缺口见 **M1** |
| 10 | 只读枚举/过滤门户 | `:324 keys` / `:328 entries` / `:332 values` / `:337 in_state` | ✓ `consumable_closed` = `in_state(Confirmed)`、`lineage_nodes` = `values().collect()`，**顺序同为 BTreeMap 序** |
| 11 | 不变量骨架 | `:425 assert_core_invariants` + `:453 assert_history_invariants` | ✓ 7 条骨架断言，5 条有 `should_panic` 负控 |

### 6.2 9 域留存（越界扫描）

`ledger_kernel/{mod,tests}.rs` 全文 grep
`nest|pan|force|divergence|macd|bridge|center|中枢|背驰|力度|活假设|seg_|retrace|买卖点|departure`：

- **签名与命名面：零命中。**
- 命中仅 5 处，全在 `mod.rs` 模块头 doc（`:5/:6/:25/:35/:41`）的「由来 / 双消费方 / 边界」说明，
  以及 `tests.rs:3` 的探针域自查声明。**属溯源引用，非域概念渗入** ✓。
- 探针域 `Probe*`（`tests.rs:14-177`）确为纯合成对象无关消费方：自造 key/kind/reason/evidence/entry，
  **终态钟故意拆成两只**（`fulfilled_at`/`withdrawn_at`）+ 一只额外域钟 `marked_at` —— 这个设计有效证明了
  内核不假设域的字段布局 ✓。

9 个假设域责任点（六元组 key、`bridge_identity`/`same_anchor`/`bridge_by_center_upgrade`、三态业务条件、
`InvalidatedReason`/`VanishCause`、修订词汇、五钟中的域钟、provider/feed、`ForceCheck`/`ForceEvidence`、
完成信号与消失扫描）**全部留 nest 侧** ✓。第 8 步消失扫描按登记未抽（不在 11 项内），diff 面确实只换了容器名。

---

## 7. 第二消费方（买卖点身份账本）能否真实落地

这是 #465 的命门条款（防单消费方早产抽象）。按 #574 契约逐条对内核面：

| 契约要求 | 内核承接 | 判定 |
|---|---|---|
| key =（中枢四条边快照, departure 索引），需等值 + 序 | `Key: Copy + Ord + Debug` | ✓ **可落地**：`Center.{zd,zg,dd,gg}` 是 `Tick = i64`（`types.rs:15`，非 f64）、`CoordinateWindow` 已 `Copy+Ord+Hash`（`level_view.rs:163`）、`RetraceIdentity` 已 `Copy`（`first_retrace_replay.rs:88`，补一个 `Ord` derive 即可）。**`Ord` 不构成障碍** |
| 观察 =（严格相邻 pair, outcome, 窗口, as_of）| `type Observation`（无 bound）| ✓ |
| 三态 + 判据（回判 / 改口 / 残废拒收）| 判据全域侧；`LedgerAdmission`/`LedgerSettlement` | ✓ 可落地，但**内核不提供转移骨架**，T3 要自己写 advance 循环（见 M4） |
| 原因载荷 = `NotConstituted{RetestReentered\|CenterRebased}` + 证据（新旧窗口、位置、知情时）| `Reason: Copy` + `Evidence: Copy` | ✓ 均为定长可 `Copy` 结构。**注意 `Copy` 是 nest 形状带来的收紧**——任何变长证据（Vec/String）都进不了载荷 |
| 三钟（出生 / 落锤 / 门卫），首写永不改 | `opened_at()`（`open` 内写、无改写点）/ `settled_at()` / `last_as_of` + `first_write_clock` | **部分**：门卫钟由 `admit` 单调保护 ✓；出生钟无改写点 ✓；**落锤钟首写不改无内核保证**（**M2**）|
| 终态后迟到输入静默吸收 + 警报计数 | `admit → TerminalAbsorbed` ✓；计数需域自计 | ✓（计数由域从返回枚举累加，可接受）|
| 时间倒退 = 拒收 + 警报 | `admit → RetrogradeRejected` + `retrograde_rejections()` | ✓ |
| 迁移留史 | `migrate` | ✓ 契约明写本账不用 |
| 历史身份全部留档不删 | `entries` 只增不删（除 `migrate` 换键）| **M1**：`migrate` 的目标键覆盖路径是唯一的静默丢档口 |
| 三档消费门户（备战/成立/短差）| `in_state(state)` | ✓ 机制够用 |
| **裁定六：唯一真相 = append-only 日志，状态 = 日志折叠，不单独持久化** | **无承接** | **M5**：内核 `LedgerEntryCore` 把 `state` 存为权威字段、`revisions` 为旁记，且无 `apply(revision) → state` 钩子 ⟹ 无法「重放日志重建状态」。方向与契约相反 |

**总判：第二消费方可落地，形状不是为 nest 私刻的。** 四组参数在 bsp 侧都有真实实例，
`Copy + Ord` 两条 bound 经查均可满足，迁移留史确如契约所言用不上（内核不强制）。
**但内核只覆盖到「容器 + 留档 + 钟 + 准入」，不覆盖「日志为唯一真相」这一层** —— 契约裁定六必须由 T3
在内核之外另建折叠层，或反过来要求 T1 内核追加 fold 钩子。spec 已声明「T3 章内核接口形状待 T1 落地后回填」，
故这条**不阻塞本票**，但应在 #575 票面写死，否则 T3 开工时会撞上一个结构性方向冲突。

---

## 8. 实装登记遗留项核判

| 登记项 | 我的判定 |
|---|---|
| `ledger_kernel/mod.rs:455` 的 `expect("建仓即有一条建项修订：{key:?}")` 插值瑕疵 | **同意**（`expect` 取 `&str` 不做 `format_args!`，panic 时打字面量）。**补一点**：同文件其余同类断言（`:410/:427/:431/:436/:440/:443/:446/:456/:464`）都是 `assert!`/`assert_eq!` ⟹ 确实会插值；此处是**唯一**用 `expect` 表达断言的地方，改成 `assert!(!history.is_empty(), "建仓即有一条建项修订：{key:?}")` 既修插值又与全文件风格一致。级别 **LOW**。 |
| +85 行构成 | **同意事实，不同意归因完整性**——根因是 `Entry` 由域持有（§5、**L4**），不是「样板 + doc」。 |
| 修订计数 fail-loud 化 | **同意**（§4.6）。 |
| 存疑①（内核新不变量在真实 replay 上未验）→ 关闭 | **同意，且我独立复核了它的证据链**：`assert_invariants` 在 `p123_fast_replay.rs:1528` 与 `p409_pan_live_probe.rs:441` 是**生产显式调用**，而我自跑的 p123 20k/100k 两窗 `EXIT=0` ⟹「留档时序非降」「首条修订为建项词汇」两条新约束在 BTC 真实数据全程真跑过且无一违反。存疑①处置**成立**，无需降级。 |
| 存疑②（`NestEventState` 变体 doc 归属）→ 无损 | **同意**。三个变体的缠论出处（061:26 / 024:24 / 061:28 / E2E §1:81）确已逐条转写进 `nest_lifecycle.rs:228-239` 的类型别名 doc，域词汇的文档归属仍在 nest 侧。**补一条我的异议**：doc 里「`LedgerState` 承载『终态吸收、禁复活、终态钟只写一次』」这句里的后两条内核并不保证（**M2**），该句需订正。 |
| 偏离（开工 HEAD `886a439c78` 而非票面 `6381debba9`）| **同意**。`git diff 6381debba9..886a439c78 --stat` 只动 `chanlun/` 三份文档，`rust/` 零改动，对本票代码面确无影响。 |

---

## 9. 新发现（分级 + 锚）

### M1 · **MEDIUM** · `LedgerBook::migrate` 不校验目标键空闲 ⟹ 静默覆盖并丢弃留档

`ledger_kernel/mod.rs:403-417`：

```rust
pub fn migrate(&mut self, from: P::Key, to: P::Key, kind: P::RevisionKind, as_of: usize) -> LedgerRevision<P> {
    assert!(from != to, "身份迁移两端不得同键：{from:?}");
    let mut entry = self.entries.remove(&from).expect("迁移源键存在");
    ...
    self.entries.insert(to, entry);   // ← `to` 已在册时，旧条目被静默丢弃
}
```

只 assert 了两端不同键，**没有 assert `to` 不在册**。`BTreeMap::insert` 的返回值（被顶掉的旧条目）
被直接丢弃 ⟹ 目标身份的**全部 append-only 留档、五钟、终态**一并消失，且
`assert_core_invariants` 事后**逮不住**（剩下的那条条目本身自洽）。这直接违反内核 doc `:401` 的
「一切钟与既有留档一个 bit 不动」与 append-only 承诺。

**nest 侧当前零影响**（已核实）：两个 `migrate_entry` 调用点都在 `advance` 的
`if !self.ledger.contains(&key) { ... }` 块内（`nest_lifecycle.rs` 第 1 步，桥臂与中枢升级迁移臂），
`to` 必不在册。这是**调用点的保护，不是内核的保护**——第二消费方（T3）没有这个块。

**建议**：`migrate` 内加一行
`assert!(!self.entries.contains_key(&to), "身份迁移目标键须空闲：{to:?}");`。
零行为影响（nest 恒不触发）、零 diff 风险，且与既有 7 个 `should_panic` 负控风格一致。

### M2 · **MEDIUM** · `settle` 无「已终态禁再落账」守卫，终态钟不走首写纪律

`ledger_kernel/mod.rs:272-281`：

```rust
fn settle(&mut self, settlement: LedgerSettlement<P>, as_of: usize) -> LedgerRevision<P> {
    assert!(settlement.state.is_terminal(), "终态落账要求终态：{:?}", settlement.state);
    self.set_state(settlement.state);
    self.write_settlement(settlement.state, settlement.reason, as_of, settlement.evidence);
    self.push_revision(settlement.kind, as_of, settlement.evidence)
}
```

只校验**目标**是终态，不校验**当前**是否已终态。连续两次 `settle`（Confirmed → Invalidated）后：

- `state` 被改写（禁复活被绕过）；
- `confirmed_at` **与** `invalidated_at` 同时有值（`write_settlement` 逐臂只写自己那只，不清另一只，也不走
  `first_write_clock`）；
- `assert_core_invariants` **逮不住**：`:437-441` 判的是
  `state().is_terminal() == settled_at().is_some()`，而 `settled_at()` 是
  `confirmed_at.or(invalidated_at)`（`nest_lifecycle.rs:527-529`）—— 两只都有值时仍是 `Some`，断言通过。

**nest 侧当前零影响**（已核实，双重保护）：① `advance` 恒先 `admit`，已终态返回 `TerminalAbsorbed` 并 `continue`；
第 8 步消失扫描过滤 `state == Provisional`；② nest **域**断言里有「终态互斥」（`Confirmed ⟹ invalidated_at.is_none()`、
`Invalidated ⟹ confirmed_at.is_none()`）能兜住。但这两条都在**域侧**——第二消费方两条都没有。

**这同时是一处声明缺口**：`nest_lifecycle.rs:228-239` 的 doc 写「`LedgerState` 承载『终态吸收、禁复活、
终态钟只写一次』这条**账本纪律**」；实测内核只提供「吸收」（`admit`），「禁复活」需域主动调 `admit` 才生效，
「终态钟只写一次」**内核零实现**（`first_write_clock` 是自由函数，`write_settlement` 不调它）。

**建议**：`settle` 首行加
`assert!(!self.state().is_terminal(), "终态禁再落账（禁复活）");`，并把上述 doc 句订正为可兑现的措辞。

### M3 · **MEDIUM** · 「计数 == 留档长度是结构性保证」= 声明膨胀

出现三处：`ledger_kernel/mod.rs:16`（「计数由留档长度现算 ⟹ **结构上不可能不一致**」）、
`:220-221`（「追加与落锤两条写入路径由内核默认实现**独占** ⟹ …**结构性保证**」）、
`nest_lifecycle.rs`（「不再靠本模块自觉维护，而是**结构性成立**」）。

三条绕过路径**公开可达**：
1. `LedgerEntryCore::revisions_mut()` 是 **pub trait 方法**（`:233`），任何持 `&mut Entry` 者可直接 `push`；
2. `LedgerBook::get_mut()`（`:320`）就外发 `&mut P::Entry`；
3. `NestLifecycleEntry.revision` / `.revisions` 至今是 **pub 字段**（结构体定义逐字节未改，正是本票的硬约束）。

内核**自己的负控测试**就在演示绕过（`tests.rs:619 entry.revision = 7`、`:638 entry.key = key(9,9)`）——
若真是结构性不可能，这些测试写不出来。

真实的保证是：**「凡经 `push_revision`/`settle` 走的路径不会不一致」+「不变量断言会在 `assert_invariants`
调用点逮住不一致」**。这已经是相对旧代码（手工 `self.revision += 1`）的实质改进，**措辞收敛到这个事实即可**。
`no-patch-mentality` 禁止模式 5 明写「声明代码不具备的能力」，故按 MEDIUM 登记。

### M4 · **MEDIUM** · 「四组类型参数」验收句与实装面不符（详见 §5）

- 第三组 **TransitionPolicy 不是类型参数**（`LedgerAdmission` 是无参 enum，`LedgerSettlement<P>` 形状固定）；
- 多出第五个结构性参数 `type Entry` + 14 个访问器方法，**未作为 spec 偏离登记**（另两处偏离都登记了）。

实质结论不变（内核确实只承 11 个对象无关点，判据留域侧是对的），但票面验收尺按字面读不成立，
且第二消费方需据此预期「T3 要自己写状态机 + 自己实现 14 个访问器」，这个成本必须写进 #575 票面。

### M5 · **MEDIUM** · 契约裁定六（日志为唯一真相）与内核结构方向相反

`LedgerEntryCore` 的 `state()/set_state()` 使状态成为**权威字段**，`revisions()` 是**旁记**；
内核零 `apply/fold` 钩子 ⟹ 无法从留档重建状态。#574 裁定六（`issue574-semantic-contract-20260728.md:70-73`）
要求「唯一真相 = append-only 日志；状态 = 日志折叠，重放即得，**不单独持久化**；快照仅派生缓存」。

两者不是同一个数据模型。T1 spec 的 11 个责任点里没有「持久化/恢复」，且 spec `:129` 明写
「T3 章内核接口节待 T1 落地后回填（防早产抽象）」⟹ **不阻塞本票**。
但 #575 开工前必须裁：是 T3 在内核之外另建折叠层（内核维持现状），还是 T1 内核追加 fold 钩子（回头改 T1）。
建议写进 #575 票面或 spec T3 章的回填节。

### L1 · LOW · `mod.rs:455` `expect` 插值瑕疵（实装已登记）

同意登记，处置建议见 §8（改 `assert!` 而非改 `expect` 的字符串）。

### L2 · LOW · `LedgerDelta` 四个方法零生产消费方

`record_opt`（`:185`）、`as_slice`（`:199`）、`len`（`:191`）、`is_empty`（`:195`）在 `src/` 内除
`ledger_kernel/tests.rs` 外零调用（nest `advance` 只用 `new()`/`record()`/`into_vec()`）。
属 built-but-unwired 小面 —— 泛型库件保留合理 API 可接受，但仓内对该模式有既往登记习惯，照实记一笔。

### L3 · LOW · `LedgerState` 相对旧 `NestEventState` 新增 `PartialOrd/Ord`

`ledger_kernel/mod.rs:58`。纯扩张、无行为影响、无消费方。但 `Provisional < Confirmed < Invalidated`
这个序**无域含义**，容易被下游误读为「严重度/推进度」序。若非 `BTreeMap`/排序刚需（实测不是），建议删。

### L4 · LOW · 抽取后宿主文件反增 +85 行，归因应指向 `Entry` 参数化选择

`nest_lifecycle.rs` 5326 → **5411**（HEAD blob 实测，与登记一致）。第一车收口报告 §5 曾预期「行数净降」，
实际反向。根因不是「样板 + doc」而是「域持有 Entry」这一形状选择（§5）。该选择我**同意**（它是公共字段面
bit-exact 的前提），只是报告的归因该指向根因。另：`advance` 247 → 213 行的逻辑本体净降是真实的正向收益。

### L5 · LOW · 全量指纹 4 轮中 1 轮 2136/2/146，报告应附「多轮取模」口径

详见 §2。#619 L9 已在 `classifier/mod.rs:3725-3738` 把该负载敏感偶发红登记在测试本体上，
故非本票回归；但「逐数相同」这句在单轮复核者手里会误判。

### L6 · LOW · 评审窗口内并行线污染工作树（工位卫生，非本票缺陷）

见 §10。记录以备下游复核者参照。

---

## 10. 工位卫生与事实边界

**并行线污染时间线（实测 mtime）**：

| 时刻 | 事件 |
|---|---|
| 21:47:01 | `nest_lifecycle.rs` 源改动（migrate commit 内容） |
| 21:49:58/59 | release 二进制 `p92`/`p123` 编出 |
| 22:06:14 – 22:08:00 | **我的 4 轮全量测试** |
| 22:09:16 – ~22:11 | **我的 8 面 replay** |
| **22:12:44 / 22:13:15** | **并行线把 `nest_lifecycle.rs`(+472/−42)、`classifier/mod.rs`、`p123_fast_replay.rs`、`issue533` 三份 fixture + 护栏测试改成未提交态** |

**我的全部测量都早于污染时点**，故指纹与对拍读数有效。污染发生后我把**所有源码比对改锚到 HEAD blob**
（`git show HEAD:… > /tmp/shadow573-blob/`）并重做了三项（Entry 结构体、公共 item 面、`assert_invariants`），
结论与污染前一致。`ledger_kernel/{mod,tests}.rs` 经 `diff` 确认**始终未被污染**。
（此处正是 memory 在案教训「核验前先 `git status`、对比 blob 而非工作树」的现场复用。）

**交付方未提交面纪律核查**：两 commit 只碰 4 个文件（`ledger_kernel/{mod,tests}.rs`、`classifier/mod.rs`、
`nest_lifecycle.rs`），11 个并行线未提交面（roster、7 份 review-results、`treasury-*.json`、
`p100_cert_bsp_recon.rs`）**一个都未被吞** ✓。本报告文件为新增，不在该列。

**我没做什么（照实）**：

1. 未在纯 HEAD 态重建对拍**基线**——那需要 git mutation（stash/worktree/checkout），票面禁止。
   我用的是交付方的 `/tmp/issue573-baseline2/`；其可信度由「migrate 后 8 面重跑逐字节命中」反向支撑。
2. 未跑 m8 三窗、`issue533_p123_byte_guardrail`、p409 探针的**独立复现**（交付方报告称 2/2 绿、
   `P409_INVARIANTS ok`）；我复核的是 p123/p92 八面 + 全量测试门，后者已覆盖 `issue533` 护栏测试
   （在 2137 之内）。
3. 未验证 M1/M2 的可触发性于运行时——两者的不可达性由**静态调用点分析**得出（`migrate_entry` 两处调用点
   均在 `!contains(&key)` 块内；`settle` 前恒有 `admit`），未写复现测试（票面禁改源文件）。
4. 未读取或改写 GitHub issue 在线正文/评论；未执行任何 git 写操作。
5. `warning: 37` 取自我 22:06 那轮（污染前）的编译输出，非污染后重编。

---

## 11. 结果包六要素

1. **结论**：#573 T1 **PASS WITH NOTES**。唯一验收尺（nest 行为 bit-exact + 不变量保持 + 零生产语义变更）
   四通道独立复现全绿；5 MEDIUM + 6 LOW，无 HIGH。
2. **定义依据**：票面 #573 验收四条；spec `spec-kernel-and-retrace-ledger-20260728.md` T1 章
   `:70-84`；#574 契约 `:101-112`（双消费方定形）与 `:58-76`（裁定五/六）；#465 调研 §2 的 20 责任点
   11/9 分割。判定所依据的输入特征逐条落在 §1–§8 的文件:行锚上。
3. **边界条件**（结论会翻转的条件）：① 若 `/tmp/issue573-baseline2/` 基线本身被污染（我无法在禁 git mutation
   下独立重建该时点），则 §3 的「零 diff」退化为「与交付方产物一致」而非「与 migrate 前一致」；
   ② 若第二消费方（T3）实际需要变长证据载荷（非 `Copy`）或需要内核提供日志折叠，则 §7 的
   「可落地」判定翻转为「形状需回炉」；③ 若 M1/M2 的静态不可达性因后续 nest 侧改动被打破（例如新增
   不在 `!contains(&key)` 块内的 `migrate_entry` 调用点），二者立刻从「潜伏」升为实发缺陷。
4. **下游推论**：#575（T3）可在本内核上开工，但须先裁 M5（日志折叠归属）并接受 M4 的成本
   （自写状态机 + 14 个访问器）；#621/#622/#623/#624 的解挂不受本评审阻塞。
5. **谱系引用**：本票不涉概念分离，涉及既有语法规则三条——`no-patch-mentality` 禁止模式 5（声明膨胀，
   本报告 M3/M2 据此定级）、`coding-style` 2026-07-28 编排者裁定「带修订留痕的单线程状态机不适用
   Immutability」（故内核就地推进条目不构成违规）、`.claude/rules/result-package.md`（本节）。
6. **影响声明**：本 session **未改动仓内任何既有文件**，只新增本报告
   `chanlun/review-results/shadow-573-t1-kernel-review-20260728.md`。仓外产物见抬头。
   零 git mutation。若采纳 M1/M2 建议，将各改动 `ledger_kernel/mod.rs` 一行（`migrate` / `settle` 各加一条
   assert），影响面 = 内核两个原语的 fail-loud 边界，nest 行为不变（两条断言在 nest 路径上恒不触发）。
