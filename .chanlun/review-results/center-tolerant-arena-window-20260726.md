# #337 容读法落地：放行判据含链尾前一格 + 取代=在场终结 —— 实装 + wf8 实测

**票面**：#337（parent #336 两裁定 / #274 / #278，阻塞 #292）
**日期**：2026-07-26
**裁定来源**：#336 末条评论（用户裁定 2026-07-26）——①「场」的口径 = 容读法；②「取代」= 在场终结
**基线**：`902cc3ff47`（#336 R3 落地）；实测时 HEAD 已被**并发工位**推进到 `36092eab8e`（#322，见 §7 纪律）

---

## 0. 一句话结论

Δidx=−1 的死亡通知**由 stale 改判合法放行**（容读格），三类死亡请求的破坏放行率
**15.4% → 49.9%（事件级）/ 99.2%（去重按实例身份）**；残余 413 条 stale **逐条归因坐实**：
410 条是「该实例此前已收过教义死亡」的**二次死亡请求**（容读格一次性，按判据拒），3 条是
Δidx=−2（容读窗外，按判据拒）——**零条无法归因**。真错杀（miskill）保持 **0**（显式失败通道未动）。
生产轨迹 `trades.jsonl`/`tower_events.jsonl` 与三份基线**逐字节一致**。

---

## 1. 放行判据改动（符号锚）

| 位置 | 符号 | 改动 |
|---|---|---|
| `rust/src/theta_v0/classifier/center_lifecycle.rs` 模块头 | ——「★#337 容读法（裁定①）」整节 | 判据表 + 三条边界如实登记 |
| `center_lifecycle.rs` | `CenterEventMachine::prev_slot` | **新字段**：容读格 =(链尾前一格出生快照, 链下标)，**仅当**退场方式 = 被取代时非空 |
| `center_lifecycle.rs` | `KillResolution::Tolerated(Center, usize)` | **新分支**：载体命中容读格 ⟹ 放行，载荷 = 该实例快照+链下标 |
| `center_lifecycle.rs` | `CenterEventMachine::resolve_kill_target` | 主格比对后**先查容读格**，再落 `rposition` 陈旧查找 |
| `center_lifecycle.rs` | `CenterEventMachine::push_point` | 一类/三类两分支：`Tolerated` ⟹ 杀载体所指实例（`prev_slot=None`，`tolerated_total+=1`），**链尾不动** |
| `center_lifecycle.rs` | `CenterEventMachine::consume_chain`（推进循环） | 取代时把旧链尾落进 `prev_slot`；旧链尾已收教义死亡（`alive=None`）⟹ 容读格置空 |
| `center_lifecycle.rs` | `CenterEventMachine::adopt` | 采纳/重基后按当前链重置容读格 = 倒数第二条；返回 `revived` |
| `center_lifecycle.rs` | `CenterEventMachine::take_alive` | **新私有**：主格教义死亡时记 `dead_tail`（重基复活判据的唯一输入） |
| `center_lifecycle.rs` | `CenterEventMachine::tolerated_kills()` / `revivals()` | **新读数**：容读格放行累计 / 重基复活累计 |

### 1.1 判据（模块头同文）

| 载体落点 | 判据 | 处置 |
|---|---|---|
| 链尾（**主格**，未收教义死亡） | `target == alive` | 放行，杀在场实例（**口径不变**） |
| 链尾**前一格**且退场方式 = 被取代 | `target == prev_slot` | ★**放行**，杀载体所指的那一个实例（链尾**不动**） |
| Δidx ≤ −2（容读窗外） | `target ∈ 已消费前缀` | 陈旧请求（不杀，不计误杀） |
| 载体不在本级链上（含 `None`） | `target ∉ 链` | **误杀拒绝**（显式失败，状态一动不动） |

### 1.2 三条边界（本机的口径选择，不是裁定原文——写进模块头不藏报告）

1. **只杀载体所指的那一个**：容读放行**不连坐链尾**。连坐 = 在没有载体证据的情况下杀第二个中枢，
   违「拒杀优先于错杀」。
2. **容读格一次性**：实例一旦收教义死亡，容读格置空 ⟹ 再指它一律 stale（已死实例不得二次死亡）。
   同理，链尾若是**被教义死亡杀掉后**才被链推进换下的，**不进**容读格（退场方式不是「被取代」）。
3. **主格不在场 ⟹ 容读窗关闭**：链尾已被杀（场为空）时校验整体不介入（口径不变）。开第二格
   等于场为空还开火。

> 边界 2 是残余 410 条 stale 的**全部**成因（§3.3 逐条归因），不是漏接。

---

## 2. 取代 = 在场终结（裁定②）：两形态与「终结」出口

| 形态 | 触发 | 事件 | dump 字段 |
|---|---|---|---|
| **教义死亡** `DeathForm::Doctrinal` | 本级三类点破坏 / 一类点同死 | `Broken` / `Reset`(died 非空) | `death_form:"doctrinal"` + `slot:"tail"\|"tail_prev"` |
| **在场终结** `DeathForm::ArenaTermination` | 被链推进**取代** | ★新 `CenterLifecycleEvent::Superseded{center,chain_index,by_chain_index}` | `death_form:"arena_termination"` |

- `CenterLifecycleEvent::death_form()` / `terminates_suspension()`：两形态**同走「终结」出口**，
  在场终结**不新增出口**（挂起仍只有「回补 / 终结」两个）。
- 不变量（单测固化）：`death_form().is_some() ⟺ killed_center_id().is_some()`。
- 同一实例**可两登记都收到**（先被取代、死亡通知晚一格到 ⟹ 容读放行成教义死亡）——不是重复
  计数，是「它什么时候不在场了」与「它是怎么死的」两个问题的两个答案。
- **本票只立判据与事件形态，不接 #292 动作**：`terminates_suspension()` 当前**无生产调用者**。

---

## 3. wf8 实测读数

命令：`M8_WIN_FILTER=wf8 VOICE_EXEC=1 OPSEM_DUMP_DIR=/tmp/r337_dump cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture`
窗口：BTC anchored i=8，test 2023-08-17..2024-02-16，264,960 bar，全 5 级。

### 3.1 总表（vs #336 R3 交付态）

| 态 | born | broken | reset | miskill | stale | superseded | chain_sync | resync |
|---|---|---|---|---|---|---|---|---|
| #336 R3（`/tmp/r3_dump`） | 717 | **120** | 36 | 0 | **683** | 586（502 聚合行） | 81 | 0 |
| **#337 容读法（`/tmp/r337_dump`）** | 717 | **389** | **37** | **0** | **413** | 586（**586 事件行**） | 81 | 0 |

`center_lifecycle.jsonl` 2139 → **2223 行**（460,177 B）。born / superseded / chain_sync / miskill / resync
**全部不变**（容读法只动死亡请求的裁决面，不动出生与链消费面）——这是改动定域的直接证据。

### 3.2 ★两形态分桶 + 放行格位分桶

| 桶 | 计数 |
|---|---|
| 教义死亡（doctrinal） | **400**（broken 389 + reset died 非空 11） |
| ├─ 落**链尾主格** `slot:"tail"` | 130 |
| └─ ★落**容读格** `slot:"tail_prev"` | **270** |
| 在场终结（arena_termination） | **586** |

逐级放行格位：L0 tail 117 / tail_prev 264；L1 tail 9 / tail_prev 4；L2 tail 2 / tail_prev 1；
L3 tail 2 / tail_prev 1。

按 kind × slot 拆开：`broken×tail` **120**、`broken×tail_prev` **269**、`reset×tail` **10**、
`reset×tail_prev` **1**。其中 `broken×tail=120` 与 `reset×tail=10` **逐数等于** #336 R3 的
broken 120 与 reset(died 非空) 10——旧口径下能放行的那批一条不多一条不少，新增的全部落在
容读格（269 三类 + 1 一类）。**这是「容读法只加不减」的直接证据**。

### 3.3 ★放行率与残余归因（照实，不做措辞美化）

| 口径 | #336 R3 | #337 |
|---|---|---|
| 三类死亡请求破坏放行率（**事件级**） | 120/780 = **15.4%** | 389/(389+391) = **49.9%** |
| 三类死亡请求破坏放行率（**去重按实例身份**） | —（未算） | 379/382 = **99.2%** |

事件级只到 49.9%，是因为分母里含大量**对同一实例的重复死亡通知**。按「被点名的不同中枢实例
里有多少最终拿到教义死亡登记」算，是 **379/382 = 99.2%**（3 个未拿到 = Δidx=−2 那 3 条）。

**残余 413 条 stale 的逐条归因（零未归因）**：

| 成因 | 条数 | 判据 |
|---|---|---|
| Δidx=−1，该身份**此前已收过教义死亡** ⟹ 二次死亡请求 | **410** | 边界 2（容读格一次性） |
| Δidx=−2（容读窗外） | **3** | 容读窗只有一格 |

Δidx 分布坍缩为 `{−1: 410, −2: 3}`。**这条归因已固化为机检断言**（`center_lifecycle_wf8_events_replay`：
逐行累积教义死亡身份集，对每条 Δidx=−1 的 stale 反查，查不到即判回归）。

### 3.4 ★miskill 保持显式失败

`miskill = 0`，通道**未动**：载体不在本级链上（跨级错取 / 载体表被重切）或 `target=None` ⟹
`Err(CenterMisKill)`，状态一动不动。容读法只把「载体在链上但落在上一格」从 stale 提为放行，
**没有**放宽「载体不在链上」这一面。单测 `mis_kill_rejected_when_target_absent_from_chain`
（4 个面：三类/一类 × 链外身份/载体缺席）原样保留且绿。

### 3.5 ★重基复活实证（#336 未能判定项 4 → 本窗关闭）

`chain_sync` 81 条（adopt 5 / rebase 76），其中 **`revived=true` 0 条**。⟹「重基把已被杀的链尾
实例重新扶上场」在本窗**未发生**（跨窗普适性仍未判，§6）。

**但重基的另一面被本票查出并如实登记**：重基同样把**容读格**按当前链重置 ⟹ 已收过教义死亡的
实例可能重新进容读格。wf8 实测 400 条教义死亡覆盖 **388 个不同实例**，多出的 **12 条全部**夹在
同级 `chain_sync` 之后（逐条归因，零未归因）。已写进 `adopt` 与模块头。**这是给 #292 的硬约束**：
终结动作必须幂等（§6 对账 C）。

---

## 4. 双轨与锁读数

| 项 | 结果 |
|---|---|
| `wf8 trades.jsonl` vs `r3_dump` / `fix331_dump2` / `h1_after_dump` | **三份全部逐字节一致**（948,340 B，`cmp` 无差异） |
| `wf8 tower_events.jsonl` vs 同三份 | **三份全部逐字节一致**（354,946 B） |
| wf8 报告行 | `execR=+4040483 MaxDD=0.0917 stage=I R=+4417092 LCB(R)=-1921382 → INCONCLUSIVE`（与 #336/#331/#329 基线逐字相同） |
| `center_lifecycle.jsonl` | **变化**（预期）：2139 → 2223 行，schema 三处变更已 GOLDEN 登记（§5） |
| `cargo test --lib` | 工作区（含并发工位未提交块）**1922 passed / 0 failed / 141 ignored**；**入库版本**（只含本票改动）**1922 passed / 0 failed / 139 ignored**——二者差额恰为 #327 块的 2 条 `#[ignore]`，实测复核过 |
| 四把锁（`--ignored`，BTC 数据） | **4/4 绿**：`btc_type2_open_short_channel_witness` / `btc_prune_leg_exit_type_matches_account_identity` / `btc_type2_residual_correction_witness` / `typed_ledger_btc_smoke` |
| #291 wf8 自证（`center_lifecycle_wf8_events_replay`，#337 新断言） | **绿**，读数与独立分析脚本逐项一致 |

**测试计数归属（并发工位污染，照实拆）**：会话起点基线 = **1910 passed / 138 ignored**（实测，
与 #336 报告一致）。终态 1922/141 拆为：
- **本票 +9 passed**（本模块 15 → 24 条：新增 10 条，删 1 条口径消失的旧测试，§5）；
- 并发工位提交 `36092eab8e`（#322 中枢延伸谓词）**+3 passed / +1 ignored**；
- 并发工位**未提交**的 #327 见证锁块（就在本票也改的 `wverify_run.rs` 里）**+2 ignored**
  ——该项已由「入库版本 139 vs 工作区 141」实测坐实，不是推断。

**四把锁「零翻动」的诚实标注（沿用 #323 Critical-2 / #331 / #336 先例）**：本票全部改动位于
`feed_center_lifecycle` 及其调用的 `CenterEventMachine`，二者由 `OPSEM_DUMP_DIR` env 门控。
四把锁**不设该 env** ⟹ 对本票改动路径的覆盖为 **0**。**零翻动是零覆盖的结果，不构成安全证据。**
真正覆盖本次改动的硬证据是：(1) 开着 dump 跑 wf8 后 `trades.jsonl`/`tower_events.jsonl` 与三份
独立基线逐字节不变；(2) 10 条新单测 + wf8 自证测试的新机检锚（§3.3）。

**慢锁不跑标注**：本票未跑重型多窗回归（`m8_e2e_all_systems_oos` 全窗臂、`l3_*` 多窗族、`p1xx`
校准 bin）。理由 = 生产轨迹逐字节不变 + 改动全在 env-gated 旁路内。**留重型窗口**。

**090 照实**：`cargo test --lib` 与四把锁跑的是当前工作区，其中含**并发工位**的未提交改动
（`wverify_run.rs` 的 #327 块）与既有脏文件 `rust/src/bin/p107_level_calib.rs`（`[[bin]]`，不进
`--lib` 编译单元）。#327 块只含 2 条 `#[ignore]` 测试 ⟹ 对本票读数无影响，但计数归属已如上拆开。

---

## 5. 变更清单

### 5.1 dump schema 变更（GOLDEN 先例登记，不静默）

`center_lifecycle.jsonl` 三处（`trades.jsonl`/`tower_events.jsonl` **不受影响**，已实证）：

1. `kind:"superseded"` 由「每 bar 每级**一行带 `count`**」→ **逐实例事件行**
   `{si,zd,zg,dd,gg,ei,chain_idx,by_chain_idx,death_form}`。理由 = 裁定②把取代升为在场终结
   （死亡登记形态之一），#292 要按**实例**终结挂起，聚合 count 不带身份接不上。行数 502 → 586。
2. `broken`/`reset` 增 `death_form`（`doctrinal`）与 `slot`（`tail`/`tail_prev`）。died 为空的
   `reset` 二者写 `null`（没死人，不编造）。
3. `chain_sync` 增 `tail_si/tail_zd/tail_zg`、`prev_si/prev_zd/prev_zg`、`revived`
   （评审 MAJOR-B：采纳/重基把「场」落到谁身上此前无产物级证据）。

### 5.2 单测（seam = `CenterEventMachine` 公开面，不变）

**新增 10 条**：
- 片五（容读法判据）6 条：`tolerated_kill_when_target_is_chain_tail_predecessor`（三类）/
  `tolerated_kill_first_class_on_chain_tail_predecessor`（一类）/
  `stale_kept_when_target_is_two_or_more_slots_upstream`（Δidx≤−2 仍拒）/
  `tolerated_slot_consumed_after_first_doctrinal_death`（容读格一次性）/
  `tolerated_slot_closed_for_already_killed_predecessor`（退场方式=教义死亡 ⟹ 不进容读格）/
  `tolerated_window_closed_when_arena_empty`（边界 3）；
- 片六（两形态）2 条：`death_forms_bucket_doctrinal_versus_arena_termination` /
  `superseded_instance_can_still_receive_doctrinal_death`；
- 片七（重基复活）2 条：`rebase_flags_revival_of_doctrinally_killed_tail` /
  `rebase_not_flagged_when_reseated_tail_is_a_different_instance`。

**删除 1 条（口径消失，非回归；tombstone 原位）**：
`stale_request_when_target_is_superseded_chain_instance` 固化的正是「链[c0,c1]、场=c1、点声明 c0
⟹ **stale**」——那正是裁定①改判的口径本身。其位置由「放行面 + 拒绝面」两条新测试接管，
tombstone 写在原位。

**TDD 红→绿三切片（如实登记，含偏差）**：
- 切片①（容读法判据）：先写 6 条测试 → **取得独立红**（3 条 assertion 失败，实得 `Stale` 期待
  `Broken`/`Reset`；另 3 条为口径不变面，一写即绿，如实登记无独立红）→ 实装 `prev_slot` +
  `Tolerated` 分支 → 绿；
- 切片②（两形态 / `Superseded` 事件）：**未取得独立红**——`Superseded` 变体与 `prev_slot` 维护
  在切片①实装时一并写入（前者是后者的同一处状态转移），其测试后补 ⟹ 一写即绿。**登记为 TDD 偏差**；
- 切片③（重基复活 `revived`）：**未取得独立红**——同上，随 `adopt` 改造一并写入。**登记为 TDD 偏差**。
- wf8 自证测试的新断言取得了**真红**：首版按「Δidx≤−2」写死，被真实产物否掉（残余 410 条
  Δidx=−1 合法存在，成因是二次死亡请求）⟹ 断言改写为「Δidx=−1 者须已收过教义死亡」的反查式
  机检锚。**这条是产物纠正了我的口径猜测，如实登记。**

### 5.3 评审随带三条的处置

| 项 | 处置 |
|---|---|
| **MAJOR-A**（举证换 550 实例覆盖） | **已落**。模块头新增「举证锚」节：`miskill 888→0` 是口径收窄后的**不可比读数**，不得当作「病治好了」的举证；锁死解除的直接反证是**出生身份多样性**（#331 复锁形态 = 51 次 born 只 2 个身份）。已固化为机检断言：wf8 自证测试 assert「L0 born 身份数 ≥ 100」，实测 **L0 553 / L1 133 / L2 27 / L3 4** 个不同身份。 |
| **MAJOR-B**（盲区入码 + `chain_sync` 身份字段） | **已落**。(a) `consume_chain` 守卫处新增「★已知限登记」块：盲区 =「链长不变/增长、末条身份未变、但更深前缀被改写」，明写后果、明写「无检出即无观测，本窗无证据≠无盲区」、明写两条闭合路径及其被否的成本理由。(b) `chain_sync` 行补 `tail_*`/`prev_*` 身份与 `revived`。 |
| **MINOR**（tombstone 措辞 / 基线数字 / rebase 复活实证补查） | **部分落**：rebase 复活实证**已闭合**（`revived` 字段 + wf8 实测 0 次，#336 未能判定项 4 本窗关闭），并顺带查出并登记「重基重开容读格 ⟹ 12 条二次教义死亡」；基线数字**已核**（会话起点 1910/138 实测复核通过，终态归属逐项拆开，§4）。**tombstone 措辞未处置**——见 §6 未能判定项 1。 |

---

## 6. #292 spec 对账（不擅自改 #292 票面，只列不一致处）

逐条对 `gh issue view 292` 的验收条款：

| # | #292 条款 | 与 #337 口径 | 结论 |
|---|---|---|---|
| A | 「触发 = 本级中枢**在场**（T1 事件机）+ 次级别买卖点」 | #292 把「在场」当**单点**；#337 后「在场」是**两格窗** | ⚠️ **不一致（需 #292 明确）**：触发用哪一格？本票判断（仅建议）：触发用**主格**（链尾）——容读格是给**死亡通知迟到**开的容差，不是「还能继续在这个中枢上做短差」。`alive_center()` 语义未变 ⟹ 若 #292 直接用它，行为即为「主格」，与建议一致。 |
| B | 「挂起只有回补/终结两出口；**全平 / 三类卖点 / 中枢破坏** ⇒ 终结」 | 裁定②新增第三条**终结触发源**：被取代（在场终结） | ⚠️ **不一致（票面枚举缺项）**：出口仍是二分（裁定明说「不新增出口」），但终结的**触发源**多一个。建议 #292 验收清单补一条单测「被取代 ⟹ 终结」。wf8 量级：在场终结 **586** vs 教义死亡 **400**。 |
| C | 「禁无本级买点证书复活」 | #337 实测：`revived=0`，但**12 条二次教义死亡登记**（重基重开容读格所致） | ⚠️ **不一致（硬约束，本票给 #292）**：同一实例可能收到**两次**终结信号 ⟹ #292 的终结动作必须**幂等**（对已终结的挂起再收终结 = no-op），否则重复减补。 |
| D | 「中枢破坏 ⇒ 终结」 | #337 的 `Broken` 可落在**容读格**（`slot:"tail_prev"`，wf8 270 条），破坏的是**已不在主格**的实例 | ⚠️ **不一致（匹配口径）**：#292 必须按**中枢身份**匹配挂起（`killed_center_id()` 已提供），不能按「当前在场中枢」匹配——否则容读放行会误终结主格上的挂起。 |
| E | 「触发 = **次级别**买卖点」 | 本机 broken/reset 由**本级**一类/三类点触发 | ✅ **一致**（不同输入流；本机只提供「谁在场 / 谁终结了」的结构地基）。 |
| F | 「盘背非必要条件」/「无互斥门」/「默认关」/「`config.rs:302` 注释」/「幽灵回补=0」 | 与本票无交集 | ➖ 不对账。 |

---

## 7. 未能判定项

1. **★评审随带 MINOR 的「tombstone 措辞」一项未处置**——#336 的影子评审**原文不在仓库里**
   （`.chanlun/review-results/` 无对应文件，全仓 grep `MAJOR-A` 零命中），本票只据 #336 末条
   评论与派工描述里的三条摘要施工。MAJOR-A / MAJOR-B / MINOR 的「基线数字」「rebase 复活实证」
   有足够具体的指向可施工并已落；**「tombstone 措辞」指的是哪一处措辞，无从确知**，不猜、不
   编造处置。⟹ 若需闭合，请提供影子评审原文或指明具体措辞。
2. **跨窗普适性**：全部读数只来自 wf8 单窗。放行率、Δidx 分布、两形态比例、`revived=0` 均为
   单窗观测。判据本身（容读窗 = 一格）是结构性的，但「一格够不够」是单窗归因。
3. **前缀分叉守卫盲区**（#336 未能判定项 3 仍开）：已入码登记（§5.3 MAJOR-B），但**未闭合**。
   本窗无检出 ≠ 无盲区（检不出的东西无从计数）。
4. **容读窗宽度 = 1 是裁定，不是实测最优**：本窗 3 条 Δidx=−2 被拒。若跨窗出现 Δidx=−2 的
   规模化残余，窗宽是否该放到 2 是新的教义裁决，本票不裁。
5. **边界 1（不连坐链尾）对一类点是否成立，未经教义复核**：一类点的教义含义是「新走势类型开始
   ⟹ 在场中枢同死」。本票让容读放行的一类点只杀载体所指的容读格实例、不动链尾（与三类点对称）。
   若缠论口径要求一类点必须清空整个场，此处需再裁。wf8 量级：**一类点落容读格的放行仅 1 条**
   （`reset×tail_prev`），爆炸半径极小；三类点落容读格 269 条。
6. **`terminates_suspension()` 无生产调用者**：本票只立形态，动作在 #292。该方法当前只被单测
   覆盖，接线正确性未在生产路径见证。
7. **L4 面仍空**（#336 未能判定项 7 沿用）：L4 只有 1 条 adopt，零 born/broken/stale。
8. **#336 未能判定项 5（一类点「禁横跨走势类型生死边界拼中枢」约束移交塔后是否仍被执行）未动**
   ——本票未查，仍属 `recursive_tower` 那一票。

---

## 8. 纪律

- **只 `git add` 本票自己改的文件**：`rust/src/theta_v0/classifier/center_lifecycle.rs`、
  `rust/src/theta_v0/backtest/opsem_dump.rs`、`rust/src/theta_v0/backtest/wverify_run.rs`（**仅本票
  hunk**，见下）、本报告。
- **★并发工位混批事故（如实登记，未自行修史）**：本会话进行中，另有工位在**同一工作区/同一
  git 索引**上工作——(a) 提交了 `36092eab8e`（#322，`recursive_t/center.rs`），(b) 在**本票也要改的**
  `wverify_run.rs` 里写了 #327 见证锁块（2 条 `#[ignore]` 测试）。本票原计划「只把自己的 hunk 入库、
  #327 块留工作区」，做法是：把 #327 块临时摘出 → `git add` 本票 4 个文件 → 还原工作区。
  **该做法失败**：`git add` 与 `git commit` 之间，并发工位把**它自己的** `wverify_run.rs` 与
  `docs/canonical-coverage-rust-impl.md` 重新 `git add` 进了**共享索引**，而 `git commit`（无 pathspec）
  提交的是**整个索引**。结果：
  - 本票首个 commit `fa916fe70e` **吞进了并发工位的 #327 块（+547 行）与
    `docs/canonical-coverage-rust-impl.md`（+19 行）**，同时**丢掉了本票自己的 `wverify_run.rs` 改动**；
  - 并发工位随后的 `af17c4d484`（#327 见证锁）因内容已被吞，成了**空 diff commit**。
  处置：本票**不自行修史**（并发工位正在这棵树上活跃工作，rebase/reset 会破坏其在制品）。
  **随后事态又变**：并发工位为收拾自己的空 commit，执行了 `git reset 36092eab8e`
  （reflog：「撤回 #327 空提交（tree 未变，重做）」）——该 reset **连带把本票的 `fa916fe70e`
  一起从分支上丢掉了**（工作区改动幸存，未丢代码）。最终本票以两个 **pathspec 限定**的 commit
  重新入库：
  - `e971040794`：`wverify_run.rs`（本票口径改动，叠在并发工位已入库的 #327 块之上，不动其一行）
    + 本报告；
  - `0e7dc46dd1`：`center_lifecycle.rs` + `opsem_dump.rs`（`fa916fe70e` 的重做）。
  **历史清理（`fa916fe70e` 的悬挂对象、两工位 commit 的时序）交用户裁决**，本票不动。
  教训（与 `feedback_commit_check_staged_area` 同型，本次复发且升级）：共享工作区/共享索引里
  **`git add` 与 `git commit` 之间必须重查暂存区**；更稳的做法是全程用 `git commit -- <paths>`
  绕开共享索引（本票后两个 commit 即改用此法，未再出事）。另：**commit 之后也要复查 `git log`**
  ——别的工位的 `reset` 会让你「已经提交」的东西再次消失。
- 既有脏文件（`CLAUDE.md`、`AGENTS.md`、`.claude/`、`.agents/`、
  `rust/src/bin/p107_level_calib.rs`、`skills-lock.json`、`tmp/fold-r3-experiment/*`、既有
  `.chanlun/**`）**未动不 add**。
- 未回关 issue、未发 GitHub 评论。
- 分析脚本为一次性 `python3 -c`（未落盘）；产物 `/tmp/r337_dump`。
- 末尾 code-review 按编排者指示**未做**（改由独立影子评审）。
