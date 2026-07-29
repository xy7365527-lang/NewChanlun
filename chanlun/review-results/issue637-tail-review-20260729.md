# #637 尾部补评审：修复轮收口（关票门第 7 子句「评审面 == 关票面」）

评审人：尾部补评审（opus，新上下文，未参与 #637 实施与影子评审）　日期：2026-07-29
对象：worktree `/private/tmp/wt-637`（分支 `ticket-637`，HEAD = main `039cf86974`）的**未提交工作区全量 diff**
　　　`rust/src/trading/center_book.rs` +295、`rust/src/theta_v0/classifier/retrace_ledger/mod.rs` +8/-4
评审重点：**修复轮引入的新语义**（首轮面已被影子评审覆盖并判 FAIL；修复轮此前未进过任何评审范围）
被复核文档：`issue637-implementation-20260729.md` §九（其声明全部独立复核，不采信）、`issue637-shadow-20260729.md`
在案依据：编排者 2026-07-29 四项裁定 1A/2A/3A/4A；票面 #637；上浮票 #664；ADR-0001 补充十一/十三/十四

---

## 结论：**CONDITIONAL**

发现计数：**HIGH 0 / MEDIUM 2 / LOW 5**

**判 CONDITIONAL 而非 PASS 的理由**：四项裁定全部忠实落地、影子 5 条 HIGH 全部消解、指纹独立复跑与报告完全一致
（2497/0/138），代码层无新引入的 HIGH 级缺陷。剩余两条 MEDIUM 都是**遗留风险登记不完整**，不是代码缺陷——
但它们各自会让下游（#664 的实施方、#575 剩余 2/3 的实施方）在只读本票产物时得到错误的心智模型，
而修复代价为零（改报告 §九.6 / 补 #664 一句话，不动代码）。

**转 PASS 的条件（三条，均不改代码）**：

1. 在实施报告 §九.6 补登记 MEDIUM-1 的机制：cert 杀**抢先关闭** `ingest` 的 kill 分支
   （`center_book.rs:147` 的 `if !dead.contains(&cs)` 守卫），故 `dead_down`/`CenterEvent::Terminated`/`version`
   三项不是「本次不置」而是「此后由 `ingest` 通道**永不补**」——并把这句话同步进 #664 的 Question 节，
   否则 #664 若选择「cert 路径不动、让 ingest 补方向」的修法会静默失败。
2. 在 §九.6 第 4 条把跨引擎锚映射的风险方向写实：首轮是「恒不命中」，修复轮删掉价格边后是
   **「可因两引擎段索引数值巧合而误命中 ⟹ 误杀在场中枢」**（后果：`alive()` 转 None、`SizeAllocator`
   当场剔除该层）。当前措辞「需要重新核实这套锚映射是否成立」未指出方向已翻转。
3. 在 §九.6 或 doc 收窄模块头首段的「唯一 diff 点」声明（`center_book.rs:6-8`）：
   `consume_death_certificate` 是第二条会改 `dead` 但**不经**该 diff 点的状态变更路径，
   `CenterEvent` 流自本票起不再是 `dead` 的忠实投影。

**PASS 的部分（如实登记）**：裁定 1A/2A/3A/4A 逐条落位无走样、无打折、无超界；影子 13 条处置声明逐条属实
（含订正 LOW-4 的诚实自证）；8 条测试全部真实存在、全部通过、命名与所证一致（唯测试 8 见 LOW-2）；
`known` 在 kill 分支记录**不会**造成误判同成功（演算见任务 3）；`already_dead` 分支不前进 `version`
与 `SizeAllocator` 门控**语义协调**（演算见任务 3）；窗口清除后 `negate_pending_departure` 早退路径**完整**；
指纹 2497/0/138 与 19 条 `center_book` 用例独立复跑完全一致；全程未见 git mutation。

---

## 发现列表

### MEDIUM-1　cert 杀抢先吞掉 `ingest` 的 kill 分支——方向盲区不是「本次不置」而是「此后永不置」

**证据**：`center_book.rs:146-164`，`ingest` 的三项收尾动作全部在 `!dead.contains(&cs)` 守卫**之内**：

```rust
if ev.confirmed && ev.class.kind() == BspKind::Type3 {
    if !dead.contains(&cs) {          // ← :147
        dead.insert(cs);
        if ev.class.side() == Side::Sell { self.dead_down[ladder]…insert(cs); }   // :149-154
        self.version += 1;                                                        // :155
        out.push(CenterEvent::Terminated { seg_start: cs, direction });           // :162
    }
    if hard_type3 && ev.class.side() == Side::Buy { self.frozen[ladder] = Some(cs); }  // :165 守卫之外
    …
}
```

而 `consume_death_certificate:416` 无条件 `dead.insert(anchor)`。

**失败场景（时序，修复轮新可达——首轮判同恒不命中，此路走不到）**：

1. `consume_death_certificate(2, cert{start_index:10,…})` → `dead={10}`、`version+=1`（`Ok(Broken)`）
2. 稍后 `ingest(2, &[Sell3 confirmed, cs=10], …)` → `known` 记入；`dead.contains(10)` 为真
   ⟹ **整个 `:147` 块被跳过**：`dead_down` 不插、`Terminated` 不发、`version` 不加
3. `is_dead_down(2,10)` 恒 `false` ⟹ `level_operating_unit.rs:1193`、`unified_osc.rs:304`、
   `axiom_voice.rs:301/:422` 一律读成「三买向上终结」⟹ 按 ADR-0001 补充十一「三买 → 回补」处置；
   真实是三卖，「三卖不能回补」（49 课）被读反 ⟹ 错误回补
4. `CenterEvent::Terminated` 消费者（模块头 `:117-118` 点名的 T4b 加码 / FatigueGate 清空路径(2) /
   域腿解冻观测）对这次死亡**永久全盲**——不是「本次没发」，是「后续 `ingest` 观测到同一 confirmed
   Type3 时也不会补发」

**与裁定 4A 的关系**：4A 把方向缺口整体上浮 #664，本条**不是要求本票修**。问题在**登记面**：
`center_book.rs:18-22` 与 `:393-396` 的措辞是「`consume_death_certificate` 杀中枢时不进 `dead_down`…」，
#664 Question 节的措辞是「对 cert 杀恒读 false」——两处都只描述**结果**，都没写出**机制**
（`ingest` 通道被 `dead` 守卫抢先关闭）。#664 范围节写的是「`consume_death_certificate` 按 `side` 登记方向」，
若实施方照此做则问题自愈；但若实施方（或后续任何人）判断「cert 路径不动、由 `ingest` 后续补方向」，
该修法会静默失败且无任何测试拦截。

**测试缺口**：8 条新测试无一覆盖「cert 杀 → 后续 `ingest` confirmed Type3」这一时序。

---

### MEDIUM-2　跨引擎锚映射的风险方向已翻转（恒不命中 → 可误杀在场中枢），§九.6 未登记该方向

**证据**：`consume_death_certificate:402-414` 修复轮后的判同全部内容是

```rust
let anchor = cert.center.start_index as i64;          // theta_v0 CompletedMove/段索引
…
if !self.known[ladder].as_ref().is_some_and(|k| k.contains(&anchor)) {  // legacy 引擎透传的 cs
    return Err(DeathCertificateError::NoMatchingCenter { ladder, anchor });
}
```

即：两套独立引擎各自切分的段序列，其索引的**整数相等**就是全部判据（价格边按裁定 2A 已删）。
`known`（`:144`）收录本层历史上见过的**每一个** `cs`，只增不减。

**失败场景**：一张锚定 theta_v0 第 10 号 CompletedMove 的死亡证明，撞上 legacy 引擎本层历史上
曾出现过的 `cs == 10`（且该中枢恰为当前 `last` 且未死）⟹ `already_dead=false` ⟹
`dead.insert(10)` + `version+=1` + `Ok(Broken)`，**在场活中枢被误杀**。后果链：
`alive(2)` → `None`（`:335-342`）⟹ `SizeAllocator::maybe_recompute`（`allocator.rs:44-46`）
`is_dead` 命中 ⟹ 该层 `amp` 被跳过 ⟹ `frac` 重算、该层仓位配额归零。

**与裁定 2A 的关系**：2A 明确要求「只做锚判同（名义映射）」并「如实声明」，doc `:362-368` 确实声明了
「名义映射…非数值同源保证」——**设计本身不是本条发现的对象**。对象是遗留风险的量级与方向：
影子对首轮的定性是「会恒不命中，不会误命中」（影子 §4 结论），修复轮删掉价格边后这一保护性副作用消失，
风险从「静默失效」翻转为「静默误杀」。而 §九.6 第 4 条的措辞——「若未来 #575 剩余部分把两引擎接同一
数据管线，需要重新核实这套锚映射在真实数据下是否成立」——读起来像「待验证的正确性」，没有指出
**误杀方向**，也没有指出即使不接同一管线、只要有任一生产调用方喂 cert 就已可触发。

**缓解现状（如实登记）**：`consume_death_certificate` 目前零生产调用点（grep 实证，见任务 1 · 3A），
`RetraceLedger::new` 亦无生产实例，故本条今日**不可触发**，是纯潜伏风险。这是它记 MEDIUM 而非 HIGH 的原因。

---

### LOW-1　`DeathCertificateOutcome::AlreadyBroken` 的 doc 说「零动作」，但 `already_dead` 分支有真实状态变更

`:437-439` 的变体 doc：「幂等确认：该锚死亡登记已存在（本证明重复消费，**或同一三类点事件已被 `ingest`
通道先观测**），**零动作**，`version` 不前进。」

实际（`:415-424`）：`ingest` 先杀那一支走的是**步骤 3**（不是步骤 1 的早退），会执行
`broken_by_certificate.insert(anchor, cert.center)`（真实写入）与 `pending_departure` 清除（可能真实写入），
只是不加 `version`。本票自己的测试 `ingest_kill_then_certificate_is_idempotent_and_archives_frame:766-770`
正是断言「框已存档」——与同一枚举变体 doc 的「零动作」互相打脸。

派生影响：该变体把两种语义不同的情形折叠成一个值——(a) 我此前消费过这张 cert（真零动作）、
(b) 这是本 cert 首次消费但 `ingest` 抢先杀了（存档发生、窗口可能被清）。调用方若按 doc 写
`if AlreadyBroken { /* 什么都没发生，跳过记账 */ }`，会漏掉 (b) 的首次登记。票面幂等条款
（「同一证明重复消费零动作」）本身**满足**（步骤 1 早退确为零动作），故仅记 LOW。

### LOW-2　测试 8 是恒真断言，`issued_as_of` 仍零引用

`late_issued_as_of_does_not_block_broken_when_anchor_still_present:818-825` 的全部内容 = ingest 建中枢 →
`cert(10,1,2,1)` → 断言 `Broken`。与测试 5（`repeated_consumption_is_idempotent_zero_action:771-783`）
的前半完全同构；`issued_as_of` 在 `consume_death_certificate` 全函数体零引用（复核确认），
故把 `1` 换成 `0`/`99`/`usize::MAX` 该测试同样恒绿。

它作为「不问迟到」的**行为宣示**是诚实的（报告 §九.2 MEDIUM-3 行如实写了「`issued_as_of` 本轮仍不参与判定」），
但它不构成独立证据——真正证明「不问迟到」的是测试 3（`superseded_anchor_certificate_still_registers_broken:735`）。
影子 MEDIUM-3 指出的「测试命名会让读者以为迟到语义已被覆盖」这一问题，形态改变但未消除。

### LOW-3　测试 1 的「真端到端」只端到一半——证明对象经真实账本产出，`CenterFrame` 仍是手搓字面量

`real_death_certificate:679-705` 确实经 `RetraceLedger::new` → `observe(Success)` → `death_certificate(anchor)`
真实开具证明（较首轮的直接构造字面量是**实质改进**，影子 MEDIUM-2 的 (a) 半已修）；
但喂给 `observe` 的 `CenterFrame { zd, zg, start_index, end_index }`（`:687`）仍由测试手写，
非来自 theta_v0 塔的真实中枢构造。报告 §九.4 第 1 行称「**真端到端**」略强于事实。

影子 MEDIUM-2 的 (b) 半（手搓值掩盖量纲错配）已随裁定 2A 删除价格边比较而**结构性消失**，不再是风险。

### LOW-4　`known` 单调只增、无界，且无测试覆盖「仅由 kill 分支记录的锚」

`known`（`:63`、写入点 `:144`）与既有 `dead` 同为 per-ladder 只增 `HashSet<i64>`，长跑磁带上随段数线性增长，
无裁剪。与 `dead`/`dead_down` 同惯例，非本票新引入的模式，故 LOW。

测试面：8 条测试中，进入 `known` 的锚全部来自「中枢形成」路径；无测试覆盖「该锚只在 `ingest` 的
confirmed Type3 kill 分支出现过、从未成为 `last`」这一形态（该形态的正确性演算见任务 3，结论无缺陷）。

### LOW-5　`ladder >= MAX_LADDER` 裸索引 panic（影子 LOW-3，未修，如实声明）

`:403 :412 :416 :417 :420` 五处裸索引。与 `ingest` 等既有代码同惯例，非本票新引入；报告 §九.6 第 5 条
已如实登记「未修」。新入口的调用方将来自 theta_v0 侧（级别口径与 `ladder` 无映射，影子 MEDIUM-4 亦未修
且已登记），越界面高于既有调用面——维持影子的 LOW 定级，无升级也无消解。

---

## 任务逐项结论

### 任务 1　裁定保真度（1A/2A/3A/4A）

| 裁定 | 要求 | 实现落位 | 判定 |
|---|---|---|---|
| **1A** ① 迟到证明照登记（054:60 / ADR-0001 补充十一 / #583） | 被更替的锚仍可判同并登记 Broken | `known` 集（`:63`/`:144`）与 `last` 分离；判定步骤 2 查 `known` 不查 `last`（`:412`）；测试 3（`:735-748`）断言被更替锚 ⟹ `Broken` 且新中枢不受影响 | **忠实** |
| **1A** ② 两条三类点点杀通道撞车 = 幂等确认，删 `KilledByOtherCause` | 该错误变体消失；撞车返回成功 | `DeathCertificateError`（`:444-451`）只剩两个变体，`KilledByOtherCause` 全仓零命中（grep 确认）；`already_dead ⟹ Ok(AlreadyBroken)`（`:423-424`）；测试 4（`:750-770`）固化 | **忠实** |
| **1A** ③ 「无此中枢」限缩为「从未见过的锚」 | `NoMatchingCenter` 只在 `known` 不含时抛 | `:412-414`；变体 doc `:445-447` 逐字写明「不包含"被新中枢取代"——那种情形按裁定 1A 应判同成功」 | **忠实** |
| **2A** ① 只做锚判同（`start_index as i64 ↔ seg_start` 名义映射） | — | `:402` + `:412`；doc `:362-368` 声明「名义映射…非数值同源保证」 | **忠实** |
| **2A** ② 禁跨量纲数值比较（删 `Tick as f64` vs `LiveCenter` 价格边） | — | 全函数体不再读 `self.last[ladder]`（复核确认：`consume_death_certificate` 内 `last` 零引用） | **忠实** |
| **2A** ③ 证明框全四边存档（`anchor → CenterFrame`），同锚不同框 fail-loud | — | `broken_by_certificate: [Option<HashMap<i64, CenterFrame>>; MAX_LADDER]`（`:69`）；`:406-410` 全框 `==` 比较（`CenterFrame` derive `PartialEq`，四边全参与）；`ConflictingCertificate`（`:448-450`）；测试 6（`:785-797`）固化 | **忠实** |
| **3A** 「生产调用点 ≥1」按票面字面结；登记表措辞；1/3 = 入口计数口径 | — | `retrace_ledger/mod.rs:32` = 「**已接线（#637）：消费入口已立**」+ 缺口列「驱动入口的上游生产链…归 #575 后续票」；`:37` = 「**1/3（入口计数口径）**」+ 「消费入口已立，非驱动链全通——`RetraceLedger` 自身仍无生产实例，见 #575 后续票」 | **忠实**（措辞与裁定原文对应，且比裁定更保守——把「非驱动链全通」写进了计数行） |
| **4A** 方向缺口本票不修、上浮 #664、代码 doc 大字声明并指向 #664 | — | 模块头 `:18-22` 独立「**⚠ 方向盲区（本票不修，已上浮 #664）**」段；方法 doc `:393-396` 同款声明；#664 实际存在（`state: OPEN`、`blocked_by #637`、范围节覆盖 `side` 字段 + `dead_down` + `frozen` + `CenterEvent` 三项） | **忠实**，唯登记的是**结果**不是**机制**（见 MEDIUM-1） |

**打折**：无。**走样**：无。**超界**（做了裁定没让做的）：`consume_death_certificate:420-422` 的
`pending_departure` 清除不在四项裁定字面内——但它对应影子 HIGH-5 中裁定 4A **未**上浮的那一项
（4A 只上浮了方向三项：`dead_down`/`frozen`/`CenterEvent`），属修复轮应做未做完则留缺口的部分，**判为在界**。
除此之外，diff 内无与本票无关的重构、无顺手改的既有逻辑（`ingest` 唯一新增行是 `:144` 的 `known` 写入）。

### 任务 2　影子 13 条处置对账（报告 §九.2 声明 vs 实测）

| # | 报告声明的处置 | 独立复核 | 判定 |
|---|---|---|---|
| HIGH-1 迟到被拒 | 已修（`known` 判同，测试 3 推翻旧断言） | 旧测试 `late_certificate_after_center_superseded_fails_loud` 已从文件消失（grep 零命中）；新测试 3 断言 `Ok(Broken)` | **属实** |
| HIGH-2 `KilledByOtherCause` 伪概念 | 已修（变体删除，撞车 ⟹ `AlreadyBroken`） | 变体全仓零命中；旧测试 `killed_by_other_cause_fails_loud_not_silent` 已消失；新测试 4 断言 `AlreadyBroken` + `version` 不变 | **属实** |
| HIGH-3 价格边量纲错配 | 已修（删 `zd`/`zg` 比较） | `consume_death_certificate` 内 `self.last` / `zd` / `zg` 零引用 | **属实** |
| HIGH-4 生产调用点 ≥1 未满足 | 按裁定 3A 字面结，不修代码 | grep 全仓：`consume_death_certificate` 的非测试引用仅 doc 与定义本身，调用点 8 处全在 `#[cfg(test)]` 内（`:715 :729 :743 :760 :776 :779 :789 :793 :809 :824`）；`CenterDeathCertificate` 确实出现在生产代码（`:29` `use`、`:400` 签名、`:69` 字段类型） | **属实**（字面达标，实质仍零驱动，报告与登记表两处均未粉饰） |
| HIGH-5 cert 杀不守恒 | 部分修（`pending_departure` 补齐）+ 方向三项上浮 #664 | `:420-422` 清除存在；测试 7（`:799-817`）覆盖「窗口置位 → cert 杀 → `has_pending_departure` 转 false → `negate` 不推 `up_strength`」 | **属实**，但方向三项的**机制**登记不完整（MEDIUM-1） |
| MEDIUM-1 幂等键是锚 | 已修（存整框，同锚不同框 fail-loud） | `HashMap<i64, CenterFrame>` + `:406` 全框比较 + 测试 6 | **属实** |
| MEDIUM-2 端到端不端到端 | 已修（`real_death_certificate` 经 `RetraceLedger::observe` 真产证明） | 辅助函数 `:679-705` 真实存在且被测试 1 使用；证明对象确经 `ledger.death_certificate(center.anchor())` 取出 | **基本属实**，「真端到端」略强（`CenterFrame` 仍手搓，LOW-3） |
| MEDIUM-3 迟到负控空壳 | 如实声明不修 + 新测试正面钉死 | `issued_as_of` 仍零引用（复核确认）；报告逐字承认「本轮仍不参与判定」 | **声明属实**；新测试恒真（LOW-2） |
| MEDIUM-4 无 level↔ladder 映射 | 如实声明不修 | `ladder` 参数仍无任何校验；§九.6 第 1 条已登记 | **属实** |
| LOW-1 已死分支缺边校验 | 随 HIGH-2 消解 | 「已死」不再是独立判定分支（`already_dead` 只决定返回值与 `version`，不决定成败），该形状确已不存在 | **属实** |
| LOW-2 cert 杀不可观测 | 未修，维持声明 | 无 getter、不写 `events_out`（复核确认）；§九.6 第 5 条登记 | **属实**（但见 MEDIUM-1 的加剧面） |
| LOW-3 裸索引 panic | 未修，维持声明 | 五处裸索引仍在；§九.6 第 5 条登记 | **属实** |
| LOW-4 rustfmt 声明不实 | 已订正 | 独立复跑 `rustfmt --check --edition 2021 src/trading/center_book.rs`：首个 hunk 即落在本票新增行 `:141` 的 `self.known[ladder].get_or_insert_with(HashSet::new).insert(cs);`——与 §九.4 举的例子逐字一致 | **订正属实** |

**总计**：13 条声明**逐条属实**，无虚报。两处措辞略强（HIGH-5 的「部分修」未点明剩余部分的机制、
MEDIUM-2 的「真端到端」），已分别记为 MEDIUM-1 / LOW-3。

### 任务 3　新代码的新问题（四项指定演算）

**演算 ①　`known` 在 kill 分支也记录，会不会让「从未形成、只见过死亡事件」的锚误判同成功？**

**不会。** 逐路推演 `known` 的全部写入面（唯一写入点 `:144`，位于 `let Some(cs) = ev.cs else { continue }` 之后、
`dead` 守卫之前）：

- 路径 A（confirmed Type3 kill 分支，`:146`）：`known.insert(cs)` 的**同一次迭代**内，`dead.insert(cs)` 必然发生
  （`:148`；若已在 `dead` 则本就在）。⟹ 该锚进 `known` 时必已在 `dead`。
- 路径 B（`else if !dead.contains(&cs)`，`:173`）：必然设 `last = Some(LiveCenter{seg_start: cs,…})`（`:204`）
  ⟹ 该锚确曾形成。
- 路径 C（两分支都不进：confirmed Type3 之外、且 `dead` 已含 `cs`）：`known` 记入但状态无变化——
  该锚此前必已由路径 A 或 B 记入，非新增。

故 `known ⊆ (曾形成 ∪ 已死)`，且「只见过死亡事件」的锚落在路径 A ⟹ 进 `consume_death_certificate` 时
`already_dead == true` ⟹ 返回 `Ok(AlreadyBroken)`、`version` 不动、`dead` 不变。
**不会产生虚假的 `Ok(Broken)`**，不会误增 `version`，不会误触发 `SizeAllocator` 重算。裁定 1A 的
「从未见过的锚」限缩语义在实现上是准确的。

**演算 ②　`already_dead` 分支存框但不前进 `version`，与 `SizeAllocator` 门控语义是否协调？**

**协调。** `version` 的唯一消费方是 `SizeAllocator::maybe_recompute`（`allocator.rs:35-39`：`if self.version == Some(version) { return }`），
其重算只读 `book.last(k)`（`:43`）与 `book.is_dead(k, lc.seg_start)`（`:44`）两项。逐分支核：

- `already_dead == true` ⟹ `dead` 此前已含 `anchor` ⟹ `is_dead` 读数不变、`last` 不变
  ⟹ 重算输出与上次**逐位相同** ⟹ 不重算不产生陈旧读数。**正确**。
- `already_dead == false` ⟹ `version += 1`（`:426`），且这是 `dead` 唯一可能改变 allocator 输出的分支。**正确**。
- 同分支内还有两处**不前进 `version` 的真实写入**：`broken_by_certificate` 存档（外部无 getter，
  allocator 不可见 ⟹ 无门控需求）、`pending_departure` 清除（其消费方 `level_operating_unit.rs:1346`、
  `unified_osc.rs:198` 逐 bar 直读 `has_pending_departure`，**不经 version 门控** ⟹ 无需版本推进）。
  且 `ingest` 的既有 kill 路径（`:170-172`）同样在守卫之外无版本地清窗口——**惯例一致**。

唯一残留是 `AlreadyBroken` 变体 doc 自称「零动作」与该分支的真实写入不符（LOW-1，语义标注问题，非门控问题）。

**演算 ③　窗口清除后 `negate_pending_departure` 的早退路径是否完整？**

**完整。** `negate_pending_departure:243-244` 首行即 `let Some((cs, side)) = self.pending_departure[ladder] else { return };`
——`pending_departure` 被置 `None` 后函数在**读 `last`、跑 `debug_assert`、算 excursion、调 `push_up_strength`
之前**全部早退。逐项确认无旁路：`pending_excursion[ladder]` 保留旧值但仅在下次窗口置位时被重置为
`NEG_INFINITY`（`:223`），且只在 `pending_departure` 为 `Some` 时才被读（`:254`）⟹ 无悬空读；
`cf_negations`/`sg_records` 不动 ⟹ 影子 HIGH-5 点名的「H2 力度历史被非法样本污染」通道确已封死。
测试 7（`:799-817`）以 `sg_records`/`cf_negations`/`up_strength_verdict` 三项断言固化了这条路径。

补充核：cert 杀的锚 ≠ `pending_departure` 的锚时（例如锚 10 被更替、窗口挂在锚 20 上），`:420` 的
`is_some_and(|(p,_)| p == anchor)` 不命中 ⟹ 窗口保留 ⟹ `negate` 正常走 `last=20` 路径，
`debug_assert_eq!(lc.seg_start, cs)`（`:246-249`）仍成立（`last` 未被 cert 路径改动）。**无新增 assert 风险。**

**演算 ④　8 条测试是否各自证明了其声称的东西？**

| # | 测试（行号） | 声称 | 实证 | 判定 |
|---|---|---|---|---|
| 1 | `death_certificate_registers_broken_end_to_end` `:707` | 真端到端 → Broken | 经 `RetraceLedger::observe(Success)` 真开证明；断言 `Ok(Broken)` + `is_dead` + `alive()==None` + `version` 前进 | **证成**，「真端到端」略强（LOW-3） |
| 2 | `no_matching_center_never_seen_fails_loud` `:724` | 从未见过 ⟹ `NoMatchingCenter` | 空 book（`known[2] == None`）⟹ `:412` 命中 | **证成** |
| 3 | `superseded_anchor_certificate_still_registers_broken` `:735` | 被更替锚 ⟹ Broken，新中枢不受影响 | ingest cs=10 → ingest cs=20（Sell1 非 Type3 ⟹ 走 `:173` 分支、`last=20`）→ cert(10) ⟹ `Broken` + `is_dead(2,10)` + `alive()==20` | **证成**（1A 的核心断言，最有分量的一条） |
| 4 | `ingest_kill_then_certificate_is_idempotent_and_archives_frame` `:750` | ingest 先杀 ⟹ `AlreadyBroken` + 框存档 + version 不变 | Buy3 confirmed 走 kill 分支 → cert ⟹ `AlreadyBroken`、`version` 相等、`broken_by_certificate[2][&10] == c.center` | **证成** |
| 5 | `repeated_consumption_is_idempotent_zero_action` `:771` | 同证明重复 ⟹ 零动作 | 两次同 cert：第一次 `Broken`、第二次 `AlreadyBroken`（走步骤 1 早退）、`version` 不变 | **证成**（票面幂等条款的直接对应） |
| 6 | `conflicting_certificate_same_anchor_different_frame_fails_loud` `:785` | 同锚不同框 ⟹ fail-loud | `cert(10,1,2,5)` → `cert(10,999,999,0)` ⟹ `ConflictingCertificate` | **证成** |
| 7 | `cert_kill_clears_pending_departure_window_and_negate_is_noop` `:799` | 窗口解除 + negate 不推力度 | candidate Buy3 置窗 → cert 杀 → `!has_pending_departure` → `negate(2,1.9)` 后 `sg_records`/`cf_negations` 不变、`up_strength_verdict == Newborn` | **证成**（唯 `Newborn` 一项在无修复时也为 `Newborn`↛，实际区分靠 `sg_records`，该断言有效） |
| 8 | `late_issued_as_of_does_not_block_broken_when_anchor_still_present` `:818` | 迟到不阻断 Broken | 与测试 5 前半同构；`issued_as_of` 零引用 ⟹ 任意值恒绿 | **恒真断言**，不构成独立证据（LOW-2） |

**新增的时序缺口（无测试）**：「cert 杀 → 后续 `ingest` confirmed Type3」（MEDIUM-1）；
「`known` 仅由 kill 分支记录的锚」（LOW-4，演算 ① 已证无缺陷）。

### 任务 4　指纹独立复跑

见下节「指纹实测」。**2497 / 0 / 138，与实施方声称完全一致。**

### 任务 5　报告 §九准确性

| 声明 | 复核 |
|---|---|
| §九.1 四裁定逐字登记 + 落位表 | **准确**：四行的「内容摘要」与编排者裁定原文语义对应无失真；「本轮落位」栏四行逐条与代码对得上（见任务 1） |
| §九.2 13 条处置对照表 | **逐条属实**，无虚报（见任务 2） |
| §九.3 修正后语义（状态 / 判定次序 / 错误枚举 / `issued_as_of`） | **准确**：三步判定次序的表述与 `:402-428` 逐行对应；「`KilledByOtherCause` 已删除」属实；「`issued_as_of` 仍不消费」属实 |
| §九.4 测试清单 8 条 | **8 条测试名全部存在、全部通过**（`cargo test --lib trading::center_book` 19 passed）；「覆盖点」栏 7 条准确、第 8 条（测试 8）声称「正面钉死」略强（LOW-2） |
| §九.4 rustfmt 订正 | **属实**：独立复跑首个 hunk 即落在新增行 `:141`，与报告举例逐字一致；「本仓不以 stock rustfmt 为 gate」结论亦成立（无 `rustfmt.toml`，既有代码大面积报差） |
| §九.5 指纹表（首轮 2495 → 本轮 2497，+2） | **属实**：本次独立复跑 2497/0/138；`trading::center_book` 19 passed = 11 既有（`:470`~`:647`）+ 8 新增（`:707`~`:825`），逐一对应；影子报告记录首轮 2495 + 净增 2 = 2497，链条自洽 |
| §九.5 「19 passed（首轮 11 既有 + 本轮 8 新增）」 | **属实**（实测 19；首轮为 11+6=17）。名单层面：首轮 6 条中仅 2 条名字留存（`death_certificate_registers_broken_end_to_end` 体已重写、`repeated_consumption_is_idempotent_zero_action` 不变），4 条被推翻/替换，另新增 6 条 ⟹ 净 +2 |
| §九.6 遗留风险 5 条 | **方向正确但两处不完整**：第 3 条（方向盲区 → #664）只写结果不写机制（MEDIUM-1）；第 4 条（跨引擎锚映射）未写出风险方向已从「恒不命中」翻转为「可误命中 ⟹ 误杀在场中枢」（MEDIUM-2）。第 1/2/5 条准确 |
| 「全程未提交（未 commit/push/reset）」 | **属实**：`git status --short` = 2 个 ` M` + 2 个 `??`（两份评审报告），`git log -1` 仍为 `039cf86974` |
| §一~§八（首轮内容） | 报告开头已声明「以 §九 为准」，其中与裁定冲突的表述（§二判同映射、§三 `KilledByOtherCause`、§四迟到折算、§五 6 条测试清单、§六指纹、§七 rustfmt）确已被 §九 对应节覆盖。**首轮正文未被删改**，作为历史记录保留——符合「逐字保留」惯例，非遗留错误 |

---

## 指纹实测

命令：`cd /private/tmp/wt-637/rust && cargo test --lib`（前台，默认 target dir，2026-07-29 本次尾部评审实跑）

```
test result: ok. 2497 passed; 0 failed; 138 ignored; 0 measured; 0 filtered out; finished in 10.55s
```

| | passed | failed | ignored | 来源 |
|---|---|---|---|---|
| 首轮终跑（报告 §六 / 影子独立复跑） | 2495 | 0 | 138 | 影子报告已独立复核 |
| 修复轮终跑（报告 §九.5 声称） | 2497 | 0 | 138 | 报告转述 |
| **本次尾部评审独立复跑** | **2497** | **0** | **138** | 上方原始输出 |

**与报告一致。**

**增量交叉验证**：`cargo test --lib trading::center_book` → `19 passed; 0 failed`。
`center_book.rs` 测试模块内共 19 个 `#[test]`（既有 11 条 `:470`~`:647`，本票新增 8 条 `:707`~`:825`），
逐一对应。2497 − 2495 = +2 = 8（本轮）− 6（首轮）✓。

**flaky 观测**：`theta_v0::classifier::tests::incremental_tower_scaling_dominates_full_synthetic`
（影子报告点名的墙钟计时类 flaky）**本次复跑通过**，与本票改动无关。

**编译警告**：`cargo build --lib` 的 54 条 warning 中，grep `center_book|retrace_ledger` **零命中**
——本票新增代码无编译警告，与报告 §七声明一致。

**git 状态**：`M rust/src/theta_v0/classifier/retrace_ledger/mod.rs`、`M rust/src/trading/center_book.rs`、
`?? chanlun/review-results/issue637-{implementation,shadow}-20260729.md`。零 commit / 零 push / 零 reset。
本评审全程只读 + 写本报告一份，未触碰主仓与 `/private/tmp/kimi-nest-mainline`。

---

## 附：与影子评审的关系（供关票门参考）

影子判 FAIL 的三条硬阻断，逐条状态：

1. **「有没有接上」（HIGH-4）** —— 代码事实未变（仍零生产调用点），但**裁定 3A 已把票面口径裁为字面结**，
   且登记表措辞已收紧到不会误导（「消费入口已立；驱动链归 #575 后续票」+「1/3（入口计数口径）」）。
   影子的事实认定成立、结论被裁定取代。**不再是阻断。**
2. **「接上后语义对不对」（HIGH-1/HIGH-2）** —— 已按裁定 1A 反转：迟到照登记、撞车判幂等、
   `KilledByOtherCause` 删除，两条固化错误行为的旧测试已推翻。**已消解。**
3. **「接上后判据命不命中」（HIGH-3）** —— 已按裁定 2A 删除跨量纲比较。副作用是风险方向翻转
   （恒不命中 → 可误命中），本评审记为 MEDIUM-2 要求补登记，非要求改代码。**已消解，留登记条件。**

本评审新增的两条 MEDIUM 都不在影子的 13 条内，也不在裁定的四项内——是修复轮新语义带出的登记面缺口。
