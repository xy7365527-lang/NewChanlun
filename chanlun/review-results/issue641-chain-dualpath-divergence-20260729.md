# #641 链簿「双路径」分叉：诊断与归因（2026-07-29）

## 起因

Standards 评审 S-1 / Spec 评审 FAIL-1 指出
`chain_certificate_book_incremental_equals_full_replay`（`rust/src/theta_v0/classifier/mod.rs`）
是自比恒真：两侧调的是**完全相同**的 `chain_book_over_prefixes(&segments[..n], …)`、相同入参、
各自 fresh `TowerCache`，断言恒成立，锁不住任何东西。

收口时的第一版重写把对照物选成：

- 侧 A：一个共享 `TowerCache` 逐前缀增量推进；
- 侧 B：**每步新建** `TowerCache`，把 `segments[..n]` 整段喂进去全量重算。

侧 B 被当成了「全历史重放」。两侧最终 `ChainCertificateBook` 在 `index = 3` 分叉：
增量侧 4 条 revision，fresh 侧 3 条；增量侧多出一条 `L1 Pan Short → L0 Pan Short` 的
`Adjacent` 边、`as_of = 124` 的 `Closed` 链，fresh 侧为 `None`。

## 诊断（实测，`chain_fixture(40)`，n = 40 步）

在两侧最终的 `CandidateStreams` 上逐项比对：

| 读数 | 侧 A（共享 cache） | 侧 B（每步 fresh cache） |
|---|---|---|
| 事件条数 | 21 | 17 |
| 候选身份 key 数 | 21 | 17 |
| 仅本侧有的 key | 4 | **0** |
| 共有 key 的最新 `state` 差异数 | — | **0** |

分叉那条链的两个端点：

- `level=1 c_start=76`：两侧都在，两侧 `state` 均为 `Confirmed`；
- `level=0 c_start=80`：侧 A 在（`Confirmed`），侧 B **根本不存在**。

侧 B 的候选身份集合是侧 A 的**真子集**，共有部分状态零差异。

## 归因

分叉出在**事件流输入层，不在 `chain_cert`**。两侧不是同一件事的两条路径，而是
`chain_cert` 模块文档（`chain_cert/mod.rs` 的 `ChainNodeStatus::Absent` 条）早已写明的
**两种驱动**：

- **因果簿**（共享 `TowerCache`）：候选事件流 append-only，key 一旦出现即永存；
- **终态窗口投影**（每步 fresh `TowerCache`）：无记忆，只投影当步终态几何。

中间前缀步观察到、其后在最终几何里不再成立的候选身份，因果簿保留、终态投影丢弃——
差的那 4 个 key 就是它。这两条驱动**本来就不该相等**，把它们当等价对照是对照物选错，
不是产品分叉。

## 处置

1. `chain_certificate_book_incremental_equals_full_replay` 改成真双路径：输入序列由
   **一个**共享 `TowerCache` 生成一次、两侧共用；唯一变量是 `ChainCertificateBook`
   持续推进（侧 A，每步存快照）还是每个前缀都用新簿从第一步重放（侧 B）。逐 n 比
   `certificates()` 条数、逐条 zip 比对（失配打 `step` + 首分叉 `index` + 两侧 `{:#?}`）、
   整体 `assert_eq!`。锁定对象照实（#667 C-2 收窄）：簿对同一事件序列的**重建确定性** +
   `Clone` 保真 + 无进程级/迭代序不确定性。「跨步隐藏状态」**不在其分辨力内**——两侧推进
   节拍逐字相同，依赖调用次数的隐藏状态两侧同样累积、不产生分叉；而链簿生命史按定义依赖
   推进节拍，节拍不同的对照物在本模块不可构造。**实测两侧逐字段一致。**
2. 两种驱动的语义差固化成新测试
   `causal_book_drive_is_a_superset_of_terminal_projection_drive`：终态投影侧的链身份 ⊆
   因果簿侧、差集非空（证明该语义差实测存在）、共有 key 的最新 revision 逐字段相同
   （**实测成立**）。
3. 夹具规模由 40 改 120（与 golden 同规模）：40 段上四条链**全部在末步一次落簿**
   （`distinct_as_of == {124}`），逐步重放侧退化成前 39 步空簿比空簿；120 段上落簿跨
   8 个 `as_of`（`{124, 232, 280, 320, 396, 420, 444, 464}`、38 条 revision）。
   规模是**加大**取覆盖，不是缩小避分叉——40 段上双路径比对本来就全过。
4. （#667 C-2 补记）#641 Acceptance 1 字面「全量/增量双路径逐字节一致」在链侧不可满足
   （终态投影驱动的候选身份是因果簿的真子集，两驱动本就不该逐字节相等——即本报告实测的
   语义差），验收物降级替换为处置 1 的「重放/增量」比对 + 处置 2 的 superset 固化；
   降级登记见 #641 关票评论。

未改任何判据、未改 `chain_cert/`、未放宽任何断言。

## 订正（#676-5，同日追记）：处置 2 的子集断言被 120 段实测证伪

处置 2 落地的 `causal_book_drive_is_a_superset_of_terminal_projection_drive` 在
`chain_fixture(40)` 上「terminal_projection ⊆ causal」为真，测试当时按此写死断言。把夹具升到
`chain_fixture(120)`（与 golden 同规模）后子集关系**当场破裂**：

| 读数 | 因果簿 | 终态投影 | 共有 | causal_only | terminal_only |
|---|---|---|---|---|---|
| 120 段 | 38 | 31 | 20 | 18 | **11**（非空 ⟹ 子集不成立） |

40 段上 `terminal_only=0` 是规模偏差造成的巧合，不是规律；两条驱动的候选身份集合**互有
对方没有的 key**，谁都不是谁的子集。`terminal_only` 非空这一侧（因果簿反而漏了终态投影
独有的 key）此前未被诊断覆盖，根因追查另开 #681。

测试已改名 `causal_and_terminal_projection_drives_agree_on_common_chain_keys`，硬锁收窄为
「共有 key 的证书逐字段相同」，且这条硬锁本身在 120 段上也先被击穿过一轮：20 个共有 key 里
4 个证书分叉，定位到分叉全部集中在 `edges[].skipped_levels[].alive_at_level` /
`.inside_parent`（候选全集里该级别当场存活/落入父端点区间的候选计数——候选全集本就由
驱动决定，两驱动在这两个数字上天然不同）。剔除这两个字段后 20 个共有 key 逐字段完全相同，
这才是两驱动唯一站得住的不变量。双向差集计数（`causal_only=18` / `terminal_only=11`）固化
为 golden 锚，不升格为规律。
