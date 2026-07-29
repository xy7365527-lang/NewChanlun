# 尾部影子评审：#641 N3 收口批增量（新上下文禁自评）

票：#667（阻塞 #641 关票门最后一格）　日期：2026-07-29　评审工位：Opus 影子评审（前台单线程，无子代理）
对象：`/tmp/wt-667` @ `ticket-667`，HEAD = `62eaad1fa6`（= main 现役面）
增量面：`077d04b82e` / `1713ef3cb0` / `1ce8a9cd8b` / `cba7bba8e5` / `0ff9720887` / `9c9959bb69` + 并车 `62eaad1fa6`
基线影子：`shadow-641-review-20260729.md`（覆盖至 `ticket-641b @ d5e55925a6`）
性质：**只读评审**——未改工位任何文件（评审前后 `git status --porcelain` 均空），只写本报告

**审查对象**: `rust/src/theta_v0/classifier/chain_cert/mod.rs`（`evaluate` 判死臂 / `ChainInvalidationCause` / `ChainBookSummary` / 三 deprecated 注记 / `chain_probe`）、`rust/src/theta_v0/classifier/chain_cert/tests.rs`（两条新机器锁）、`rust/src/theta_v0/classifier/mod.rs:4426-4612`（双路径两测试 + 两 helper）、`rust/src/bin/issue550_event_battery.rs`（`print_chain_summary` / 参数解析 / 模块头用法 doc）、`chanlun/review-results/issue641-chain-dualpath-divergence-20260729.md`
**统计口径**: 不适用——本报告无统计结论。全部读数为测试计数与机器断言结果，口径 = `cargo test --release --lib`（本评审自跑）+ 两个反事实负控（archive 副本，见 §一.3 / §七 C-4）

---

## 结论：**PASS WITH CONDITIONS**

票面四项对象逐项核过：判死收窄严格落在裁定②字面授权内、机器锁经反事实负控证明精准锁得住、双路径测试的对照物选择与非真空三锁均成立且夹具落簿声明独立复现逐字相同、口径归一自洽且取舍理由成立、S-5 doc 与参数解析一致、测试门零新增红。

两条 MEDIUM 属 090「声明 ≠ 实际」族：**均不改判定路径、不改任何已交付读数**，故列收编条件而非阻断。

| 轴 | 判定 | 依据 |
|---|---|---|
| 1 Absent 收窄（判死恰两支 / 三面不判死 / 机器锁真锁） | **符合** | §一，含两个反事实负控 |
| 2 双路径真锁（唯一变量 / 非真空三锁 / superset 三断言） | **符合（声明须收窄）** | §二 + 条件 C-2 |
| 3 口径归一（summarize/digest/replay 取态） | **符合** | §三，注释与代码逐字一致，取舍理由成立 |
| 4 S-2 / S-5 / S-6 三 doc | **S-5 符合；S-2 符合但连带缺口；S-6 部分不符** | §四 + 条件 C-1 / LOW-1 |
| 5 测试门（2522/0/138 零新增红） | **通过** | §五，二进制新鲜度另证 |
| 6 并车归一冲突解法（floor 探针取本线形） | **同义成立** | §六 |

**收编条件（两条，均纯文档 / 一行级，不必重跑读数与护栏）**：

- **C-1**：代码里唯一的链读数指针（`chain_cert/mod.rs:72`）指向的报告是**未订正**版本——#653 影子 MED-2 的整改落在了另存名下，其病在 main 现役面复活。§七 C-1。
- **C-2**：`chain_certificate_book_incremental_equals_full_replay` 的 doc 声称锁住「链簿无跨步隐藏状态」，**超出该测试的实际分辨力**（两侧推进序列逐字相同）；且 #641 Acceptance 1 字面（全量/增量双路径逐字节一致）在链侧已被替换为「重放/增量」，替换论证充分但未在票面/Resolution 登记为口径变更。§七 C-2。

---

## 一、Absent 收窄（票面对象 1）

### 1.1 判死是否严格恰两支

`chain_cert/mod.rs:598-612`，判死臂现为：

```rust
if nodes[0].status == ChainNodeStatus::Falsified { Invalidated(HeadInvalidated) }
else if !all_segments                            { Invalidated(PredicateFailed) }
else if head_confirmed && !extendable && has_segment { Closed }
else { Open }
```

判死出口**恰两处**，成因枚举恰两档（`mod.rs:118-127`），与裁定②字面（`Invalidated` = 谓词判不过 ∨ 链头 `Invalidated`）逐字对应。连坐支（中间节点失效即判死全链）不存在：`invalidation_cause_has_only_the_two_ruled_branches` 是其反事实锁，本评审复跑通过。

**收窄前后的差**只有一处：原 `!head_alive` 把 `Absent` 与 `Falsified` 并入同一判死支，收窄后 `Absent` 不再判死。裁定②授权面里没有「链头查无」这一支，故收窄是**向授权面回收**，不是新增语义。

**旁证（内部一致性反而提高）**：`ChainCertificateBook::advance` 的 doc（`mod.rs:874-880`，本轮**未改**）早已写明「链……**没有『缺席即失效』这条路径**——链的死活只由裁定②的两个条件决定，而这两个条件都是在重新评估里读出来的，不是从『本轮没看见』推出来的」。收窄前实装与这段自述矛盾（`Absent` 正是「本轮没看见」），077 消除了该矛盾。

### 1.2 Absent 链头是否三面不判死

- **不 `Invalidated`（因链头）**：`Absent ≠ Falsified`，第一臂不取 ✓
- **不 `Closed`**：`head_confirmed = nodes[0].state == Some(Confirmed)`；而 `mod.rs:561-571` 里 `status` 与 `state` **同源于同一个 `event = index.latest.get(node)`** —— `Absent ⟺ event.is_none() ⟺ state == None`，故 `head_confirmed` 在 `Absent` 时**结构上恒假**，无法绕过 ✓
- **自然落 `Open`**：落到 `else` 臂 ✓

**须照实指出的边界**（不构成缺口）：「Absent 链头不判死」只否掉了「因链头查无而判死」这一因。链头 `Absent` 时若剩余存活端点之间仍有事实边，链照旧走 `PredicateFailed` 判死——该支是裁定②授权的第二支，与链头状态无关，正确。

### 1.3 机器锁是否真锁得住（反事实负控，本评审自做）

在 `/tmp/wt667neg2`（`git archive HEAD` 副本，独立 `CARGO_TARGET_DIR`，**工位零改动**）上做两个负控：

| 负控 | 注入的病态 | 期望 | 实得 |
|---|---|---|---|
| 负控 1 | 判死条件改回 `nodes[0].status != ChainNodeStatus::Alive`（`Absent` 也判死） | `absent_head_stays_open_and_is_not_invalidated` 变红 | **25 passed / 1 failed，恰好该条红**，其余 25 条零误伤 |
| 负控 2 | floor 探针去掉 `&& all_segments`（回到 #653 影子 MED-1 的病态） | `all_fact_edges_do_not_increment_floor_blocked_probe` 变红 | **该条红**，panic 消息 = `谓词判不过的事实边链不属于「地板拦下」`（`tests.rs:285`） |

两条锁**精准且非真空**：`absent_head_...` 除 `status/cause/nodes[0].status` 三断言外还带 `probe.absent_node > 0`（真经过 `Absent` 分支）；`all_fact_edges_...` 断言 `segment_count()==0`、`fact_edge_count()==edges.len()`、`floor_blocked==0` 三面同时成立。`chain_probe` 用 `thread_local!`（`mod.rs:1092`），libtest 每测试独立线程 ⟹ 全局计数断言无跨测试污染。

**尝试构造的绕过路径**（脑内 + 代码核）：
1. 改回 `!head_alive` → 负控 1 已证当场红；
2. 让 `Absent` 节点继承簿内上一轮的 `state`（则 `head_confirmed` 可真）→ 结构上不可能：`status` 与 `state` 同源于同一 `event.map()`，除非同时改两处。**覆盖缺口（INFO 级）**：现有测试构造里链头首轮不是 `Confirmed`，故若有人真去做这个改动，`absent_head_...` 仍会绿。见 §七 INFO-1。
3. floor 探针条件的实际语义：`!has_segment && all_segments ⟺ edges.is_empty()`，加上 `nodes[0].status == Alive` ⟹ `alive_positions == [0]`，即**精确对应地板条款的「链头独活」**。修正不宽不窄 ✓

---

## 二、双路径真锁（票面对象 2）

### 2.1 两侧是否只有「簿推进方式」一个变量

是。`classifier/mod.rs:4470-4483`：`inputs` 由**一个**共享 `TowerCache` 生成一次（`for n in 1..=segments.len()` 逐前缀 append，push `(streams, end)`），两侧共用同一份 `&inputs`。侧 A = 一个簿持续 `advance` 并每步 `clone()` 快照；侧 B = 每个 step 用新簿从 `inputs[0]` 重放到 `inputs[step-1]`。**输入流确实同一份** ✓ 旧版「两侧调同一函数同一参数」的自比恒真（Spec 评审 C1）已消除。

### 2.2 逐 prefix 比对 + 非真空三锁是否可能真空绿

比对是三层：`certificates().len()` → 逐条 `zip` 比对（失配打 `step` + 首分叉 `index` + 两侧 `{:#?}`）→ `assert_eq!(snapshot_a, &replay_b)`（含簿内部 `latest` 映射）。

三条非真空锁：`certificates` 非空、`distinct_as_of.len() > 1`、`with_edges_count > 0`。**票面点名的真空路径（夹具链全在末步落簿）恰由第二条堵住**。

**独立复现夹具落簿声明**：在负控副本把该断言临时改成 `> 99` 强制打印实际值，实得

```text
【负控读数】distinct_as_of={124, 232, 280, 320, 396, 420, 444, 464} len=8 certificates=38
```

与 `1713ef3cb0` doc 及分叉归因报告声明的「8 个 as_of、38 条 revision」**逐字相同** ✓ 夹具由 40 改 120 是加大覆盖（40 段上 `distinct_as_of == {124}` 会使新断言直接红），非缩小避分叉。

### 2.3 但测试实锁的对象小于其声明（→ 条件 C-2）

侧 A 与侧 B 施加的是**同一个函数序列、同一初值、同一顺序、同一次数**（`advance(inputs[0..step])`）。因此任何「跨步累积的隐藏状态」在两侧会**以同样方式累积**，不产生分叉。该测试能捕获的是：`Clone` 保真、`advance` 对同一事件序列的重建确定性、无进程级/迭代序不确定性（`BTreeMap` 而非 `HashMap`）—— 这在 event-sourcing 簿上是真实且有价值的不变量，但**不是** doc 写的「若链簿将来引入依赖调用次数的隐藏缓存/去重状态，逐步快照与全历史重放会当场分叉」。真能锁住后者的对照物需要两侧**推进节拍不同**，而链簿的生命史（三只钟 / revision）按定义依赖节拍，故那种对照物在本模块**不可构造**——这正是分叉归因报告论证的同一件事。处置见 §七 C-2（改声明，不改测试）。

### 2.4 superset 三断言是否实测成立

`causal_book_drive_is_a_superset_of_terminal_projection_drive` 本评审定向复跑通过（1 passed）。三断言：`terminal_only.is_empty()`（子集）、`!causal_only.is_empty()`（差集非空，防「两驱动其实相等」的真空）、共有 key 的 `heads()` 逐字段 `==`。方向正确，且第二条把「差异存在」固化成机器约束——若将来有人静默把两驱动合一，测试当场红，符合「照实固化、不把语义差当 bug」。

**分辨力照实**：该测试用 `chain_fixture(40)`，其上链全落单一 `as_of`（见 2.2），共有链仅 3 条，故「共有 key 逐字段相同」这条的覆盖弱于同族的 120 段测试。见 §七 INFO-2。

---

## 三、口径归一（票面对象 3）

`issue550_event_battery.rs:162-177` 现状（merge 后）取态顺序：

```text
summarize()  →  digest()  →  [by_status 组装]  →  advance(last_streams, last_as_of)  = replay
```

`summary` 与 `digest` **同取于幂等重放之前**，`replay` 在其后 —— 与票面所述「#653 LOW-1 口径」一致，本线 S-3 原实装（重放先行、`summarize` 在后）已让位。注释（`:165-168`）与代码**逐字一致**，且注明了双线独立发现的出处（#653 影子 LOW-1 + #641 S-3）。

**三者自洽**：`by_status` 全部取自同一个 `summary` 值；`replay` 之后的簿态无任何读者（函数末即 drop）；`ISSUE641_CHAIN` 行同时印 `summary.*` 与 `idempotent_replay_delta={replay}`，两者语义分离。

**「同取于前」相对「同取于后」的取舍成立**：`digest` 是 golden 锚值。若取于重放之后，一旦幂等破（`replay != 0`），锚值会**静默**吸收重放追加的 revision —— 表现为 golden 漂移，而漂移原因（幂等破）与漂移本身在同一行里无法分辨。取于前则：锚值恒定描述「生产推进序列的终态簿」，破幂等由 `idempotent_replay_delta` 这一格单独报红。这个取向与「不吞、照实印出」的既有纪律一致，也与前轮影子 LOW-1 的修法建议（「把 `digest` 与 `summarize` 一并在 `advance` 之前取」）逐字吻合 ✓

---

## 四、S-2 / S-5 / S-6 三 doc（票面对象 3 / 4）

### 4.1 S-2（`ChainBookSummary` 同源声明收窄）— 符合，但有连带缺口

收窄后的声明（`mod.rs:747-751`）称：只有 `issue550_event_battery::print_chain_summary` 真迁到 `summarize()` + `digest()`；p123 的 `chain_dump_line` 是**逐证书**行格式化，粒度与全簿汇总不同，**未迁**。

独立核对 `p123_fast_replay.rs:631-687`：确为 per-certificate 格式化，且自带 `count(ChainNodeStatus::*)`、`adjacent`、`crossed` 三组就地计数 —— **「真未迁」与「粒度不同」两点均照实** ✓ 措辞比原声明（「两个 bin 同源」）准确。

连带缺口（LOW）：S-2 只登记「未迁」，未登记「何时迁 / 是否要迁」；而 Standards 评审 MED-3 点名的正是这处重复（两 bin 各自重算计数 + `ChainStatus → &str` 两份 match）。把整改条目改写成照实登记是可辩护的（Fowler 臭味非阻断），但去向应有一句话。见 §七 LOW-2。

### 4.2 S-5（用法 doc 补 `chain_every`）— 符合

模块头（`:4-8`）写「第三位参 `chain_every`，默认 5000，`.max(1)` 兜底，随读数一并印出（`advance_every`），不是静默采样」。参数解析（`:123-130`）：第三次 `args.next()`、`unwrap_or(5_000)`、`.max(1)`；报错串（`:114`）已补第三位参。`ISSUE641_CHAIN` 行确实印 `advance_every={every}`（`:180`）。**doc / 解析 / 报错串 / 印出面四处一致** ✓

### 4.3 S-6（三 pub 导出登记 deprecated）— 部分不符世代宪法 §1

宪法 §1 的机械判据：**deprecated 待退役** = 带 `#[deprecated]`/deprecated note ∨ **生产零可达但有测试调用者**；**现役** = 有非测试调用者 ∧ 无 deprecated 标记 ∧ 在 main 上。逐个核实际调用点：

| 符号 | 实际非测试调用点 | 按宪法应判 | S-6 登记 | 核验 |
|---|---|---|---|---|
| `chain_paths` (`:501`) | **零**（`advance` 走 `paths_from_index`，模块内无真实调用，仅 doc 引用） | deprecated 待退役 | deprecated | ✓ 正确 |
| `CHAIN_SEGMENT_PREDICATE` (`:184`) | `build_edge` (`:678`) 写入每条事实边的 `breach.predicate` —— **生产路径** | 现役 | deprecated | ✗ |
| `latest_of` (`:809`) | `apply` (`:887`) `let prior = self.latest_of(...)` —— **生产路径** | 现役 | deprecated | ✗ |

三条注记的措辞是「零非测试**外部**调用者」。宪法判据里没有「外部」这个限定词 —— 加上它等于把判据从「无非测试调用者」放宽成「无跨模块调用者」，于是两个生产路径正在使用的实体被登记成「待退役」。后果不是行为层的（doc-only），而是名分层的：宪法 §1 的首要功能是「让精力不再流向死代码」，而这两个不是死代码；且宪法 §3 开票门规定「实施票若触及 deprecated 名分的代码，票体必须先引 §1 判据核名分」—— 此后任何触及 `apply()` / `build_edge` 的票都被这条登记拖进核名分流程。

作者的真实意图（从「零非测试外部调用者」可读出）应是**「pub 导出面待收窄为 `pub(crate)`」**，那是 API 面问题，不是名分问题。见 §七 LOW-1（改写措辞即可，一段）。

---

## 五、测试门（票面对象 5）

本评审自跑（`/tmp/wt-667/rust`，工位自带 `target`）：

```text
cargo test --release --lib
  → test result: ok. 2522 passed; 0 failed; 138 ignored; 0 measured; 0 filtered out
```

**与票面基线 2522/0/138 逐字相同，零新增红，零红。**

定向复跑（确认本轮三条新测试真执行、真通过，非被 filter 掉）：

```text
cargo test --release --lib chain_cert   → 26 passed / 0 failed（含 absent_head_stays_open_and_is_not_invalidated、
                                          all_fact_edges_do_not_increment_floor_blocked_probe、
                                          chain_certificate_book_incremental_equals_full_replay）
cargo test --release --lib causal_book_drive → 1 passed / 0 failed
```

**二进制新鲜度另证**（防「缓存重放假绿」）：`rust/src` 下全部 `.rs` 最新 mtime = `2026-07-29 16:33:01`；lib 测试二进制 `target/release/deps/newchan_rust-88165b9b2876c1ce` mtime = `16:35:33`，晚于全部源文件；工作区 `git status --porcelain` 空。故本次三元组来自覆盖当前 HEAD 全部源文件的构建，非过期产物。

---

## 六、并车归一的冲突解法（票面对象 4）

floor 探针两线同义性核对：

- 641b 侧 `b2f7834386`：`if head_alive && head_confirmed && !extendable && !has_segment && all_segments`
- 本线 `077d04b82e`（合并取此形）：`if nodes[0].status == ChainNodeStatus::Alive && head_confirmed && all_segments && !extendable && !has_segment`

641b 线上 `head_alive` 的定义就是 `nodes[0].status == ChainNodeStatus::Alive`；本线因删除了该局部变量（判死臂改用 `Falsified`）而就地展开。**合取项集合相同、语义相同、顺序无关** ⟹ 「同义修复取本线形」成立，且取本线形不丢 641b 的任何内容，同时叠加了本线的 Absent 收窄 ✓ merge log（`issue641-ticketchain-merge-log-20260729.md:43`）对该处的「零生产行为变更」判断亦成立（探针是 `#[cfg(test)]`）。

---

## 七、新发现（分级 + 锚）

### C-1（MED，收编条件）　链读数指针指向未订正报告 —— #653 MED-2 在 main 现役面复活

**锚**：`rust/src/theta_v0/classifier/chain_cert/mod.rs:72`（唯一读数指针）→
`chanlun/review-results/issue641-n3-chain-impl-20260729.md:9-11`（头部登记）、`:230-234`（§五 5.4 表）。

代码里唯一的真实窗口读数指针写的是「真实窗口读数见 `issue641-n3-chain-impl-20260729.md`」。该文件在 HEAD 上的内容是**未订正版**：头部仍写「§五 的 BTC 读数**未改**」，§五 5.4 的 `extends` 非空列裸写 `5 / 12 / 33` —— 这三个数是现行代码跑不出来的（`extends` 改查簿命中后三窗归零，同报告 §九 9.3 与前轮影子独立实测均为 `0/0/0`），且该列**无任何就地 supersede 标注**。

前轮影子 MED-2 的整改条件正是针对这两处。#653 收口 `b2f7834386` 确实做了（头部收窄为「除 `extends` 列外未改」+ 表内删除线 `~~5~~→0` + §9.6 登记），但合流 `ac96d4056f` 时该文件走 add/add 冲突，处置为「两份并存」：**订正版被改名到 `issue641-n3-chain-impl-ticket641b-20260729.md`，未订正版占住原名**，而原名正是代码指针指向的路径。

merge log 给的理由是「同一票两工位视角，均有史料价值」。核对内容：两份 429 行中**仅 53 行不同，且差异全部是 #653 收口订正**（头部 3 行 + 5.4 表 4 行 + 尾部 §9.6 收口节 38 行），其余逐字相同 —— 两份实为同源副本的订正前/后两版，不是两种视角。故该处置的后果与前轮影子 MED-2 的整改意图相反：按指针取数的人（前轮影子原话：「N7 立案时按 §五 取容量口径」）仍会取到已撤下的数。

**影响**：零代码行为影响；文档/记账层的取数误导。**修法（任一，纯文档）**：① 在原名份 §五 5.4 补同样的删除线 + supersede 标注、头部同步收窄；或 ② 把 `mod.rs:72` 的指针改指 `-ticket641b-` 那份，并在原名份头部标「已由 `-ticket641b-` 份 supersede」。

### C-2（MED，收编条件）　双路径测试的「无跨步隐藏状态」声明超出其分辨力；Acceptance 1 口径替换未登记

**锚**：`rust/src/theta_v0/classifier/mod.rs:4459-4462`（测试 doc）+
`chanlun/review-results/issue641-chain-dualpath-divergence-20260729.md:56`（处置 1 末句「锁的是『链簿无跨步隐藏状态』」）。

推导见 §二.3：两侧施加的是同一函数序列、同一初值、同一顺序与次数，任何跨步累积状态在两侧同样累积，故该类缺陷**不会**使比对分叉。测试实锁的是「簿对同一事件序列的重建确定性 + `Clone` 保真 + 无进程级/迭代序不确定性」—— 有价值，但不是所声称的那件事。

第二面：#641 Acceptance 1 的字面是「全量/增量双路径产出逐字节一致」，Spec 评审 C1 的整改建议是「把全量路径真正接上再宣称 Acceptance 1」。收口批的实际处置是**不接**，改为「重放/增量」对照 + 另立 superset 测试固化两驱动语义差。该选择有充分论证（分叉归因报告的实测：终态投影驱动的候选身份是因果簿的真子集，共有部分状态零差异；与 #551 裁定(i) 甲口径「fork_causal_only 是终态语义的定义后果」一致），本评审**同意不接**。但后果是 Acceptance 1 在链侧不再有对应验收物，而同名测试（`..._incremental_equals_full_replay`）仍在，票面/Resolution 也未登记这次口径替换 —— 按 `delivery-discipline.md`「开票时先把对照物钉住」的同族口径，不可满足的对照型 AC 应显式降级登记，而不是让同名测试看起来仍在满足它。

**影响**：零代码行为影响；「测试锁住了 X」这一声明的可信度。**修法（纯文档）**：测试 doc 与分叉归因报告 §处置 1 把锁定对象照实改写为上述三件，并加一句「『跨步隐藏状态』不在本测试分辨力内 —— 链簿生命史按定义依赖推进节拍，不同节拍的对照物不可构造」；同时在 #641 关票评论或本票补一条「Acceptance 1 链侧口径替换」的降级登记。

### LOW-1　S-6 deprecated 登记对两个生产路径在用的实体不成立

见 §四.3。修法：把 `CHAIN_SEGMENT_PREDICATE` 与 `latest_of` 两处注记改写为「**pub 导出面待收窄**（零跨模块调用者，可降 `pub(crate)`）；模块内生产路径在用（`build_edge:678` / `apply:887`），名分=现役」，`chain_paths` 一处保留 deprecated 不动。

### LOW-2　收口批未闭合的两条 Standards 条目，未登记去向

**锚**：`rust/src/bin/p123_fast_replay.rs:4245-4257`（Standards MED-2）、`rust/src/bin/issue550_event_battery.rs:154`（Standards LOW-3）。

- **MED-2 未闭**：`chain_dump_cadence_advances_on_beat_and_on_last_bar` 仍是 `TowerCache::new()` 空流 ⟹ `advance` 恒返空 Delta ⟹ 两条断言（sink 空、`seq == 0`）在「节拍正确」「永不推进」「每根都推进」三种实现下全过；测试名承诺的节拍语义无一被夹住。同 diff 自设「非真空绿等于没锁」六处，此处自破。（该测试注释承认「行数为 0 是空流的后果」，但没有任何断言证明「节拍分支被真走过」。）
- **LOW-3 未闭**：`print_chain_summary` 仍是 `print_*` 名却在其中 `advance` 改簿态；S-3/归一进一步在该函数内新增了取态顺序约束，函数长度与职责混用未减。

两条都非阻断（Standards 轴结论为 PASS），但收口批清单里没有它们，Resolution 遗留里也没有。**修法**：本票或后续票登记去向即可（MED-2 的实修需要给 `observe` 加一个节拍探针或用非空 cache 夹具，成本高于一行，建议单列）。

### LOW-3　077 修了 floor 探针代码，未同步探针 doc

**锚**：`chain_cert/mod.rs:1077-1082`（探针字段 doc）vs `:617-624`（触发条件）。

doc 仍写「链头存活且 `Confirmed`、不可再扩展，但**链段集合为空** ⟹ 判 `Open` 而非 `Closed`」。实际条件已收窄为 `edges` **全空**（`!has_segment && all_segments ⟺ edges.is_empty()`）。「链段集合为空」在有事实边时也成立，而那种场景判的是 `Invalidated` —— 即前轮影子 MED-1 指出的「声明 ≠ 实际」在代码侧已修、doc 侧未同步（方向反转：现在 doc 的触发面宽于实际）。**修法**：doc 补一句「且全边皆链段（等价于 `edges` 为空 ⟹ 链头独活）」。

### LOW-4　本轮新引入的重复代码（Fowler #2）

**锚**：`classifier/mod.rs:4406-4423` `chain_book_over_prefixes` vs `:4426-4444` `chain_book_over_prefixes_fresh_cache`。

两函数 18 行几乎逐字相同，唯一差别是 `TowerCache` 在循环外还是循环内。测试 helper 的重复容忍度较高，但这是本轮**新增**的，且两者的差异正是 superset 测试的被测语义 —— 用一个「cache 策略」参数（闭包或两值 enum）合并，可让「唯一变量是驱动」这件事在代码结构上自证。非阻断。

### INFO-1　`Absent` 链头 + 曾 `Confirmed` 的组合无测试覆盖

`absent_head_stays_open_and_is_not_invalidated` 的构造里链头首轮为默认（非 `Confirmed`）状态，故「Absent 链头即便上一轮曾 `Confirmed` 也不得 `Closed`」这条没有机器锁。当前实装结构上不可能违反（`status` / `state` 同源，见 §一.2），故只是覆盖缺口，不是缺陷。加一行 `head_confirmed` 首轮构造即可闭。

### INFO-2　superset 测试的夹具规模弱于同族

见 §二.4：`chain_fixture(40)` 上共有链 3 条且全落单一 `as_of`。若改 120 段（与双路径 / golden 同规模），「共有 key 逐字段相同」这条的覆盖会明显加强，且 `causal_only` 非空更稳。非阻断。

### INFO-3　`Absent` 落 `Open` 的连带后果：resident 集合不再封口（仅终态投影驱动可达）

`advance`（`mod.rs:882-897`）每轮重评「本次极大路径 ∪ 簿内**非终态**链路径」。收窄前，链头 `Absent` ⟹ `Invalidated`（终态）⟹ 该链退出 resident；收窄后落 `Open` ⟹ **永久驻留 resident**，且链头在 `Absent ↔ Alive` 之间抖动会反复刷 payload revision。

**实测面零影响**：两个消费 bin 都是因果簿驱动 —— `issue550_event_battery.rs:464-471` 喂的是与 `classify_with_tower_events_incremental` 逐 bar 对拍相等的 `owned.append_bar_events` 流；`p123_fast_replay.rs:604` 喂 `cache.candidate_streams()`（持续 `TowerCache`）。因果簿 append-only ⟹ key 一旦出现即永存 ⟹ `Absent` 不可达（模块 doc `:146-152` 亦如此声明），故 `nodes_absent` 在这两条读数面上恒 0，本次收窄在真实数据读数上零变化。建议在 `ChainNodeStatus::Absent` 或 `advance` doc 里补一句该驱动下的驻留/增长面，供 N7 选驱动时读。（077 的 commit message 只登记了 lib 计数变化，未声明数据面零变化 —— 结论可由不可达性推出，但没有写出来。）

### INFO-4　`S-n` 编号无仓内定义表

票面与收口批用 `S-2 / S-3 / S-5 / S-6` 指代整改项，但仓内三份来源报告各用自己的编号体系（Standards：MED/LOW；Spec：A/C；前轮影子：MED/LOW/INFO），无任何文件定义 `S-1..S-6` 的映射。追溯「某条 S-n 对应哪条源条目」目前只能靠推断。建议收口批的整改清单落一张三行映射表（或直接沿用源编号）。

---

## 八、090 照实登记（本评审自身的限度）

1. **未复跑真实数据读数**：BTC 三窗（20k/100k/300k）与 500k 未跑；本轮增量中唯一可能影响真实读数的改动是 Absent 收窄，其零影响由「因果簿驱动下 `Absent` 不可达」推出（§七 INFO-3），**属推理而非实测**。护栏面（stdout/P116 ×4、m8 ×6、p123 dump）亦未复跑 —— 增量面对 `p123_fast_replay.rs` 零改动（并车时整文件取 main 侧），对 `issue550_event_battery.rs` 只改取态顺序与 doc，故字节门风险低，但本评审未独立验证。
2. **负控副本**：`/tmp/wt667neg2`（`git archive HEAD` 导出）+ `/tmp/wt667neg2-target`。工位 `/tmp/wt-667` 评审前后 `git status --porcelain` 均空；侧 ref、`/tmp/wt-641*` 零接触。
3. **测试门为 release lib 单面**：未跑 `--bins` / `--tests`（集成测试）/ `cargo build --all-targets`；lib 编译期 55 条 warning 未逐条核（CI 无 clippy/fmt gate，与前轮影子同口径）。
4. **未核面**：`digest()` 的 `format!("{:?}")` 在更大簿上的内存、`chain_paths` DFS 在病态输入下的规模上界（前轮影子已登记「无上限、无采样、无截断」）、fixture 漂移 gate（本轮未改 `formal/`，未触发 CLAUDE.md 的收尾节拍）。
5. **不评审的裁定面**：floor 完整口径（`CloseFloor_at` 三型）按取舍裁定第 5 条留 fog，本评审确认增量面**未越权自补**级别分量。`CONTEXT.md` 词表收编（Spec A3 / 前轮影子 INFO-3）仍未落，已在 #641 Resolution 遗留登记，非本轮增量缺口。

---

## 九、收编建议（供编排者裁）

1. **C-1 / C-2 收编后即可判 PASS**：一处纯文档标注（或改指针）+ 一段 doc 改写 + 一条降级登记，均不改代码、不必重跑读数与护栏。若编排者认为「取数指针指向未订正副本」与「测试声明宽于分辨力」不构成阻断，可直接判 PASS 并把两条转为跟进项 —— 但 C-1 建议**不要**跟进，因为它正是前轮影子已经收编过一次的同一条病，第二次逃逸会让「条件已闭」这类结论整体贬值。
2. **LOW-1 建议一并收**（一段措辞，成本极低，且涉及世代宪法判据的正确适用）；LOW-2 的 MED-2 实修建议单列后续票（成本高于一行）；LOW-3 / LOW-4 / INFO-1 / INFO-2 可挂后续。
3. **INFO-3 建议落 doc**：N7 若采用 fresh-book（终态投影）驱动，`Absent` 可达 + resident 不封口这两件事需要在选驱动时就知道，写在模块 doc 里比留在评审报告里可达性高。
4. **本票判定不阻塞 #641 关票门**：增量面的硬轴（裁定符合性、机器锁有效性、测试门）全部通过；两条条件均为记账层，可与关票并行处置。
