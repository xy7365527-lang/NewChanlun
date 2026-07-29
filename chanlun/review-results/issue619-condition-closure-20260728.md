# #619 条件收口交付：影子评审 #603 的 H1/H2/M3/L9/L10 + 文档订正

> 角色：实装执行层（全程前台单线程，未派 Task/子代理/后台任务）
> 基线：`kimi-nest-mainline-20260717` @ `2a7971d298`（代码面同 `dc2b7dd48b`）
> 票据：#619（map #597 线）；条件来源 `chanlun/review-results/shadow-603-review-20260728.md`
> 独立 target（仓外、不提交）：`/tmp/kimi-nest-target-619fix`
> 产物（仓外、不提交）：`/tmp/f619-{base,new}-{2000,20000,100000}.{stdout,dump,p116,stderr}`、
> `/tmp/f619-mod-{full,mine}.patch`、`/tmp/f619-533-comment.md`

## 0. 结论

**六条条件全部收口，BTC 三窗三面零字节漂移。**

| 条件 | 级别 | 处置 | 落点 |
|---|---|---|---|
| **H1** + **L12** | HIGH + LOW | 合并修：迁移写入点统一 + 认领关联改走 append-only 修订链 | `nest_lifecycle.rs` `migrate_entry`/`MigrationKind`/`settlement_stats` |
| **H2** | MEDIUM | `center_upgrade_match` 改确定性优选存活前身 + 多候选语义文档化 | `nest_lifecycle.rs:center_upgrade_match` |
| **M3** | MEDIUM | #533 golden 第三次重锚补留痕（issue 评论 + 测试模块头登记表 + 本 commit body） | `gh issue comment 533` → `#issuecomment-5111642984` |
| **L9** | LOW | 计时 flaky 测试本体补注记（时间敏感性/隔离单跑绿/与 #491 无关） | `classifier/mod.rs:3721-3735` |
| **L10** | LOW | `P116_DUMP` 面纳入 #533 护栏（三窗全文对拍） | `issue533_p123_byte_guardrail.rs` + 3 份新 fixture |
| **文档订正** | — | revert 报告 §6-1(b) 加订正注记（体例同 impl 报告，锚 #618 Resolution） | `issue603-tier2-revert-20260728.md:3-19` |

**L11**（`advance()` 265 行 / `nest_lifecycle.rs` 5005 行）按派发口径**不在本票动手**，由编排侧登记 #454。

---

## 1. H1 + L12（合并修）：迁移写入点统一 + 认领关联走修订链

### 1.1 缺陷复述（评审 H1）

`force_overtake_claimed_count` 经 `entry.superseded_from` 反查被认领的前身键。该字段是**单个
`Option`、被两个码共用、每次迁移整体覆盖**。留痕认领产出的新身份是 `Provisional`；只要它多活
一个 bar、活窗右端延展一格 ⟹ `bridge_match` 命中 ⟹ `superseded_from` 被改写成「上一 bar 的
自己」⟹ 指向终态前身的唯一可审计关联丢失 ⟹ 纠误口径少计。

### 1.2 修法（严格形式，非补丁）

**(a) 关联来源换成 append-only 修订链**（`settlement_stats`，`nest_lifecycle.rs:836-850`）：

```rust
let claimed: BTreeSet<LifecycleKey> = self.entries.values()
    .flat_map(|entry| entry.revisions.iter())
    .filter_map(|revision| match revision.kind {
        LifecycleRevisionKind::CenterUpgraded { from } => Some(from),
        _ => None,
    })
    .collect();
```

`CenterUpgraded { from }` 修订自己就带 `from`，修订链**永不覆盖**。取**全部** `from`（不止评审
建议的首条）：同一 entry 可先留痕认领终态前身、再逐 bar 迁移，迁移写入的 `from` 是自己的旧键
——旧键已被 `remove`、不在 `entries` 内，`claimed.contains` 恒不命中，不污染口径。

**(b) 两处逐句同构的迁移块提成单一写入点**（L12，`nest_lifecycle.rs:1414-1441`）：

```rust
fn migrate_entry(&mut self, old_key, new_key, migration: MigrationKind, as_of) -> LifecycleRevision
```

`MigrationKind`（新私有枚举，`:361-373`）只说「哪一种迁移」、**不带 `from`**——`from` 由迁移点
自己填（就是被 `remove` 的那个键），调用方无从填错。「写 `superseded_from`」与「写哪种来源修订」
自此绑成同一个写入点，一致性**由类型保证**而非靠两处代码各自遵守纪律。`advance` 第 1 步的两个
分支各缩到 2 行。

**(c) 声明订正**（090 声明膨胀）：
- `superseded_from` 字段文档（`:418-427`）改写为「**当下来源指针**」，显式登记有效域：
  每次迁移整体覆盖、不承载历史、**不得用于任何跨 bar 的口径统计**；
- `LifecycleRevisionKind::CenterUpgraded` 文档（`:339-344`）把「统计口径按 `superseded_from`
  反查」改为「按本修订自带的 `from` 反查」，并显式禁止经 `superseded_from` 中转。
- 原「逐条对应本 entry 首个非 `Observed` 修订的 kind」这句在覆盖后不成立，已随本条删除。

### 1.3 新增测试（评审点名的缺口：多步链场景）

| 测试 | 场景 | 钉住什么 |
|---|---|---|
| `issue619_claim_association_survives_subsequent_bridge_migration` | 认领留痕后**认领方多活一个 bar 被桥迁移** | (a) `superseded_from` 确实被覆盖成「上一 bar 的自己」（成因不被掩盖）；(b) `CenterUpgraded{from}` 仍在修订链里；(c) `force_overtake_claimed_count` 仍 = 1 |
| `issue619_center_upgrade_chain_migrates_twice_keeping_clocks` | 同锚 B **连升两次**（两次都走迁移形态） | `book.len()==1`（不分裂）、`observed_at`/`first_provable_at` 一路不后移、两条 `CenterUpgraded` 的 `from.b_center_start` 逐条留痕 `[20, 30]`（后一条不覆盖前一条） |

**负控实证**（把 `claimed` 临时改回旧实现跑同一测试）：

```
issue619_claim_association_survives_subsequent_bridge_migration ... FAILED
  assertion `left == right` failed: 纠误口径不随桥迁移丢数（票 #619 H1）
  left: 0    right: 1
```

⟹ 评审 H1 的失效场景在旧实现下**实测复现**（口径从 1 掉到 0），不是理论构造；新实现下取回 1。
负控代码已全部撤回（`grep NEGATIVE-CONTROL` 零命中）。

---

## 2. H2：`center_upgrade_match` 确定性优选存活前身

### 2.1 缺陷复述（评审 H2）

旧实现 `self.entries.keys().find(|old| bridge_by_center_upgrade(old, key))` 取键序首个匹配者，
**不看死活**。文档给的正当性「同链上至多一只存活」把「存活」当成了「匹配」——留痕形态**恰恰
把终态前身留在册**（这是它的设计要点），于是第三次 B 升级时死/活两个前身同时匹配，选中谁由
`sort_tuple`（`seg_c_full` 排在 `b_center_start` **之前**）决定，而 C 右端随 `as_of` 漂移、
与死活完全无关。若盲选到终态前身 ⟹ 本该迁移的**存活**前身不被迁移 ⟹ 五钟不继承、沦为孤儿
⟹ 后续走 `IdentityVanished`。

### 2.2 修法

签名改为 `fn center_upgrade_match(&self, key, as_of) -> (Option<LifecycleKey>, bool)`，一次遍历
同时给出「选中谁」与「可否迁移」（`migratable` 判定从 `advance` 内移入，消除两处重算）：

1. 首选 `BTreeMap` 序首个**可迁移**者（`Provisional` ∧ `as_of ≥ last_as_of`）→ 立即返回；
2. 无可迁移者 → 退回 `BTreeMap` 序首个匹配者，走认领留痕。

两级都在 `BTreeMap` 序上取首个 ⟹ **确定性、无平局歧义**保持不变，只是把「优先级」从字节序
换成语义序。

### 2.3 多候选语义文档化

`center_upgrade_match` 的 doc 新增「多候选语义」节（`:1443-1462`）：显式声明「可匹配者可以有
多只」、旧论证为何不成立、两级优选规则、以及「为什么不能按 `LifecycleKey` 序盲取」的排序链
推导（`sort_tuple` 先比 `seg_c_full`，C 右端与死活无关）与后果（孤儿 + 凭空的 `IdentityVanished`）。

### 2.4 新增测试

`issue619_center_upgrade_prefers_migratable_predecessor_over_terminal_one`：刻意构造终态前身
（`c=(70,130)`）在键序上**排在**存活前身（`c=(70,131)`）之前（测试内以
`assert!(terminal_key < live_key)` 把该前提显式钉住），第三次 B 升级时断言：

- delta 里的 `CenterUpgraded{from}` == **存活**前身的键；
- 无 `IdentityVanished` 修订（不留孤儿）；
- `book.len()==2`（终态前身留档 + 迁移后的新身份）；
- 迁移后 `observed_at == 131`（五钟随迁移继承，不是重新建仓）；
- 终态前身 `assert_eq!(entry, &terminal_before)` 逐位不动。

**负控**：旧盲取实现下该测试 FAILED（`优选可迁移（存活）前身，而非键序首个（终态）前身`）。

### 2.5 行为影响（BTC 100k）

**零**。全窗只有 3 条认领、链长都是 1（评审 §9-H2 已 grep 自证），从未出现多候选 ⟹ 优选规则
在本窗不改变任何选择。三窗三面逐字节相同（§5），真值列一位不变（§4）。

---

## 3. M3：#533 golden 第三次重锚的留痕

三处同时补齐：

1. **issue 评论**：`gh issue comment 533` → `#issuecomment-5111642984`。写明 `dc2b7dd48b` 重锚了
   哪些文件、原因（诊断行补 `seg_a` 完整锚，消解归因粗键歧义）、以及「为什么是纯字段追加、
   无行为变化」的两条自证（剥字段后三窗逐字节相同 / stdout 面跨四个 commit 哈希一致）——后两条
   由影子评审 #619 §4.2/§5.2 独立复现，非交付方自证。
2. **测试模块头登记表**：`issue533_p123_byte_guardrail.rs` 的「golden 变更纪律」节新增一张
   逐 commit 的变更登记表（四行：`fa912ba991` / `0e0d011da2` / `dc2b7dd48b` / 本次），
   **每次动 golden 追加一行**——使留痕不再只依赖 commit message 与 `git blame`。
3. **本 commit body**：再次引用 #618 小修包与本次收口（见 commit message）。

---

## 4. L9：计时 flaky 的测试本体注记

`rust/src/theta_v0/classifier/mod.rs` `incremental_tower_scaling_dominates_full_synthetic` 的
doc 注释新增「⚠ 时间敏感（票 #619 L9 登记）」节：

- 判据是**墙钟时间比**（`ratio_at_max < 0.7`）⟹ 对机器负载敏感，并行跑整套 `--lib` 时偶发红；
- **隔离单跑恒绿**，复现红时的正确处置是单独重跑确认，**不是改阈值**（阈值的因果标定见其下方
  既有长注释，`no-patch-mentality` 合规）；
- **与 #491 无关**且性质不同：#491 是字节摘要不符、恒红、与负载无关；本测试是负载相关的偶发红；
- 登记此前只散落在 4 份评审报告的自然语言里（逐份给了文件:行锚），本注记补齐测试本体侧。

本轮 `--lib` 全量跑该测试**绿**（未复现）。

---

## 5. L10：`P116_DUMP` 面纳入 #533 护栏

`P116_DUMP`（`p123_fast_replay.rs:199`）是与 stdout / `P421_LIFECYCLE_DUMP` 并列的独立产物面，
原先**不在** #533 护栏覆盖内 ⟹ #603 链的回退与小修包两侧交付都漏列了它，只能靠评审逐次手工
`cmp`。本次纳入：

- `run_p123` 增设 `P116_DUMP` 环境变量并返回该面字节；
- 2000 面与 20k/100k 面**都走全文对拍**（不是哈希）：该面体积 438B / 8.8KB / 21.6KB，比 dump
  小三个数量级，没有理由退回「只能报告『变了』」的哈希断言 —— 失败时给出字段级 diff；
- 新增 fixture：`issue533_p123_{2000,20000,100000}_p116.golden.txt`。

**这是新增 golden，不是重锚**：stdout / dump 三窗 golden 本次**一个字节未改**（§6 对拍）。

---

## 6. 验收：复测 + 三窗三面对拍

### 6.1 字节对拍（本 session 自跑，改动前后同一 target）

改动**前**（HEAD `2a7971d298`）与改动**后**在 `/tmp/kimi-nest-target-619fix` 各跑一遍三窗：

| 窗 | stdout | `P421_LIFECYCLE_DUMP` | `P116_DUMP` |
|---|---|---|---|
| 2000 | `cmp=0` | `cmp=0` | `cmp=0` |
| 20000 | `cmp=0` | `cmp=0` | `cmp=0` |
| 100000 | `cmp=0` | `cmp=0` | `cmp=0` |

改动前的三窗产物同时与**仓内 golden** 对拍全绿（2000 stdout/dump 全文 `cmp=0`；20k/100k 两个
SHA-256 与 `.sha256` fixture 逐位相同），确认基线未被他工位未提交面污染。

### 6.2 档1-only 真值列（BTC 100k，`P421_LIFETIME_SUMMARY`）

```
entries=301 first_provable=207 provisional=1 confirmed=135 force_overtake=40
force_overtake_claimed=2 never_constituted=74 identity_vanished_refuted=28
identity_vanished_seam=23 flash_terminal=74 nonflash_count=226 nonflash_median=Some(88.0)
force_lifetime_median=Some(64.5)
```

与派发口径逐位相同：`entries=301` / `first_provable=207` / `refuted=28` / `flash=74` /
`nonflash 226/88.0` / `force_overtake=40` / **`claimed=2`**。

**逐项归因**：H2 修正在本窗**未改变任何选择**（全窗 3 条认领、链长均为 1，无多候选场景），
H1 修正在本窗**取回同一个数**（两例留痕认领的认领方都在认领的同一 bar 即终结，`superseded_from`
来不及被桥迁移覆盖——这正是评审所说「本窗没炸纯属巧合」）。故真值列**零变化**，且 `claimed=2`
在多步链下的稳定性由新增的三个测试（而非本窗数据）钉住。

### 6.3 护栏门

| 门 | 结果 |
|---|---|
| `cargo test --release --test issue533_p123_byte_guardrail -- --ignored` | **2 passed / 0 failed**（现含 P116 三窗全文对拍） |

### 6.4 测试门

`CARGO_TARGET_DIR=/tmp/kimi-nest-target-619fix cargo test --release --lib`：

```
test result: FAILED. 2075 passed; 1 failed; 137 ignored
唯一红：theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard  ← #491（既线）
```

**数目自洽**：2075 = 2044（评审 #619 复现的基线）+ 3（本票新增）+ 28（**他工位未提交面**
`rust/src/theta_v0/classifier/ledger_kernel/`，票 #573 T1，实测 `cargo test --lib ledger_kernel`
= 28 passed）。本票口径 = **2047**。

`nest_lifecycle` 模块单跑：**46 passed / 0 failed**。计时 flaky 本轮绿。

---

## 7. 未提交面隔离（纪律自证）

工位 `/tmp/kimi-nest-mainline` 并发活跃，`rust/src/theta_v0/classifier/mod.rs` 里**混着他工位
（#573 T1）的 3 行**（`pub mod ledger_kernel;` + 2 行文档）与本票的 15 行 L9 注记。整文件
`git add` 会吞掉他工位的模块声明，而 `ledger_kernel/` 目录本身未跟踪 ⟹ 提交声明不提交目录
= 直接打断主线编译。

处置：`git diff` 导出该文件 patch → 裁掉他工位那个 hunk → `git apply --cached --recount`
只 stage 本票 hunk。核验：`git diff --cached mod.rs` 只含 L9 注记 16 行；
`git diff mod.rs`（未 stage 残留）恰为他工位的 3 行。

其余文件（`nest_lifecycle.rs` / `issue533_p123_byte_guardrail.rs` / revert 报告 / 3 份新 fixture）
逐 hunk 核对**纯本票改动**（`git diff` grep `ledger_kernel|#573` 零命中）。
未碰他工位的其它未提交面（`agent-roster-*`、`issue571-*`、`treasury-reverify-*`、
`p100_cert_bsp_recon.rs`、11 份 `??` 评审报告）。未 push、未 merge、未改 `Cargo.toml`、
未关票、未改 map/roster。未触 `formal/` ⟹ 未触发 fixture 漂移 gate。

---

## 8. 结果包六要素

1. **结论**：#619 六条条件全部收口。H1 由「经 `superseded_from` 中转」改为「经 append-only
   `CenterUpgraded{from}` 修订链」，并与 L12 合并为单一迁移写入点 `migrate_entry`（一致性由
   类型保证）；H2 由「按键序盲取」改为「可迁移者优先的两级确定性优选」并把多候选语义完整
   文档化；M3/L9/L10 分别补齐 golden 留痕、flaky 测试本体注记、P116 护栏面。三个新测试覆盖
   评审点名的两类多步链缺口，且**两条关键断言经负控实证会在旧实现下变红**。BTC 100k 三窗
   三面零字节漂移，档1-only 真值列一位不变。

2. **定义依据**：`nest_lifecycle.rs` — `LifecycleRevisionKind::CenterUpgraded`（:331-344，
   认领事件的权威载体）、`superseded_from`（:418-427，当下来源指针 + 有效域）、`MigrationKind`
   （:361-373）、`migrate_entry`（:1414-1441，全 book 仅有的两个 `remove` 点收敛处）、
   `center_upgrade_match`（:1443-1478，两级优选 + 多候选语义）、`settlement_stats` claimed
   集合（:836-850）、`LifecycleKey::sort_tuple`（:134-143，`seg_c_full` 先于 `b_center_start`
   的排序链）、`bridge_by_center_upgrade`（:211-218，严格同锚判据不动）、`assert_invariants`
   （:1318-1327，来源链两端满足桥或认领——两码互斥，本次修改后仍成立且被 46 条测试逐条调用）；
   `issue533_p123_byte_guardrail.rs:44-58` golden 变更纪律（M3/L10 的判据来源）；
   `divergence.rs:334-349` `segments_diverge_or`（三通道**或**关系——新测试构造反超时须三通道
   同时不衰减，H2 用例的强 bar 量级由此定）；`shadow-603-review-20260728.md` §9 H1/H2/M3/L9/L10/L12。

3. **边界条件**（本结论何时翻转）：
   (a) H1 修法依赖「修订链 append-only、`push_revision` 是唯一追加点、无删除 API」——若将来
       引入修订删除/压缩，`claimed` 集合的正确性需重判；
   (b) H2 的两级优选依赖「同链上**可迁移**者至多一只」在**正常喂入序**下由构造成立（修复后
       有可迁移候选必迁移，不会新建第二只 Provisional）；**倒退喂入**（`as_of < last_as_of`）
       下可出现两只 Provisional 共存，此时优选仍确定（取键序首个可迁移者）但「至多一只」的
       构造性论证不适用——故文档只声明「确定性」，**不再声明「至多一只存活」**；
   (c) 本报告全部量化读数为 **L2 = BTC 100k 单窗真实数据**，不外推其它标的/窗口；三个新测试
       为 **L1（合成数据）**——它们验证的是代码路径可达性与口径正确性，不产生市场性结论；
   (d) 「H2 在本窗零行为影响」的判定基于「全窗 3 条认领、链长均为 1」这一本窗事实，换数据段
       后 H2 修正**会**改变选择（那正是它的目的），届时 golden 需按纪律重锚并说明原因；
   (e) 测试门「唯一红 #491」在计时 flaky（L9）复现时会变成两红——该 flaky 与本链无关，注记
       已入测试本体。

4. **下游推论**：
   (a) 引用「纠误后反超数 = `force_overtake − claimed`」的下游**不再需要**先确认认领方是否在
       认领 bar 即终结（评审 §11-4(c) 的附加条件由本次修复消除）；
   (b) `superseded_from` 自此**只可用于单 bar 的来源追溯**，任何跨 bar 口径必须走修订链——
       字段文档已把该约束写进有效域声明；
   (c) `P116_DUMP` 面进入常驻回归门 ⟹ 后续任何改动若动到该面，`cargo test --test
       issue533_p123_byte_guardrail -- --ignored` 直接给出字段级 diff，不再依赖评审手工 `cmp`；
   (d) `migrate_entry` 成为迁移语义的唯一扩展点——将来若新增第三种迁移码，加一个
       `MigrationKind` 变体即可，不会再出现「两处同构块各自演化」的漂移面（L12 关闭）；
   (e) 档1-only 真值列（§6.2）**仍是当前唯一有效真值**，impl 报告 §4/§8-4 的 304/31/229/95.0
       仍作废（评审 §11-4(a) 结论不变）。

5. **谱系引用**：#619（本票）；#603（主链，含 comment-5111127257 回退裁定）；#599（档位裁定）；
   #618（误诊订正与小修包来源——本次两处文档订正的锚）；#613/#601（L2 零回归口径，本次未触）；
   #591/#592（被 #618 推翻的误诊同族刻画）；#533（字节护栏门与 golden 变更纪律，M3/L10 的
   判据来源）；#491（既线唯一红）；#573 T1（他工位未提交面 `ledger_kernel`，本次仅隔离不触碰）；
   #454（L11 体量/嵌套，编排侧登记，本票不动）；`no-patch-mentality`（H1/H2 都按「严格形式」
   重写而非加分支；L10 选全文对拍而非哈希；L9 选注记而非改阈值）；`coding-style` 2026-07-28
   补注（`NestLifecycleBook` 属「带修订留痕的单线程状态机/账本」，`migrate_entry` 的
   `remove`/`insert` 在该例外内）；`formalization-validity-domain`（真值列标 L2 = BTC 100k
   单窗，新测试标 L1 合成）。

6. **影响声明**：
   - **改动**：`rust/src/theta_v0/classifier/nest_lifecycle.rs`（`MigrationKind` 新增、
     `migrate_entry` 新增、`center_upgrade_match` 重写 + 签名变更、`settlement_stats` claimed
     集合来源变更、两处迁移块收敛、3 处文档订正、3 个新测试）；
     `rust/src/theta_v0/classifier/mod.rs`（**仅** L9 doc 注记 15 行，逐 hunk 隔离 stage）；
     `rust/tests/issue533_p123_byte_guardrail.rs`（P116 面纳入 + 模块头两节文档）；
     `chanlun/review-results/issue603-tier2-revert-20260728.md`（文首订正注记，正文不动）。
   - **新增**：`rust/tests/fixtures/issue533_p123_{2000,20000,100000}_p116.golden.txt`、本报告。
   - **API 面**：`center_upgrade_match` 与 `migrate_entry` 均为**私有**方法，`MigrationKind` 为
     私有枚举 ⟹ crate 外可见面零变化。`force_overtake_claimed_count` 的语义不变（取值来源变），
     其唯一消费点仍是 `p123_fast_replay.rs:743/749` 的 `eprintln!`（stderr 诊断）——**未进判据
     与证书真值路径**，#449 方向不越界。
   - **外部动作**：`gh issue comment 533`（`#issuecomment-5111642984`）。未关票、未改 map、
     未碰 roster、未 push。
