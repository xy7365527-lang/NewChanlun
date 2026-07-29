# 影子评审 #623 —— S3 门户补齐（备战档只读 + 短差档三锁 + 失败处置通知）

> 日期：2026-07-29；评审车：claude opus，新上下文独立评审，**未参与 #623 实装**
> 全程单线程（无子代理、无后台任务）；只读 + 跑测试；**零源文件改动、零 git mutation**
> 评审基准：`573644425d`（开工时 HEAD）；隔离树 `git archive HEAD | tar -x -C /tmp/shadow623`，
> 独立 `CARGO_TARGET_DIR`（**未**用 `git worktree add`）
> 评审期间并行线推进 HEAD `573644425d` → `8db5a94a2b`（#622 落地 + #576 拆分批 3/4）；
> **#623 交付面在该区间零改动**（`git diff --stat 573644425d HEAD -- portal.rs tests/standby.rs
> tests/short_retrace.rs` = 空），故全部结论在当前 HEAD 仍成立；`book.rs`/`mod.rs` 行号引用给
> 两套（评审基准 / 当前 HEAD）。

---

## 结论先行

**VERDICT：AC 全条达成，票面可收；但短差档三锁中的「幂等消费」与「一一对应去重」两把锁在实现层
名不副实——HIGH×1 / MEDIUM×3 / LOW×4，全部落在 `portal.rs` 的 `ShortRetracePortal` 与判败派生
路径上，备战档一侧无实质发现。**

- 实施报告的**全部量化指纹经独立复现，逐项吻合**：`cargo test --lib` = 2144 passed / 0 failed /
  137 ignored；`cargo test --lib retrace_ledger` = 61 passed（48 S1 + 13 本票，standby 5 +
  short_retrace 8，逐文件计数复核）；`cargo test --doc retrace_ledger` = 1 passed（compile_fail）；
  retrace_ledger 零新增编译警告；`adapter.rs`/`book.rs`/`log.rs`/`ledger_kernel/`/`nest_lifecycle.rs`
  净 diff = 0 行。**无一处报数造假**。
- compile_fail doctest **真实且归因正确**（不是 `ignore`；探针 P1 证明去掉犯规那一行后正文全部编译
  通过，故失败成因确为缺 `TradableSignal` 实现）。
- 「降级声明候选」**事实成立**：`5ea5082830` 单独检出确不可编译（E0583 × **5**）；`73c7b34bea`
  隔离树 `cargo build --lib --tests` 零 error。定性见 §5。
- 最重一条（HIGH-1）：`ShortRetracePortal::consume()` 对**尚未登记**的身份调用一次，即把该身份永久
  写进 `consumed`；此后 sync 进来的真记录**永远取不出**，而 `len()` / `balances_with()` 全部显示
  正常——「重复消费幂等」这把锁记的是「调用过 consume」，不是「处置过一条记录」。

---

## 1. 独立复现（逐项照实报数）

| 项 | 命令 | 实施报告 | 本车实测 | 判 |
|---|---|---|---|---|
| 编译（HEAD 隔离树） | `cargo build --lib --tests` | 零 error | `Finished` 零 error | ✅ 吻合 |
| retrace_ledger 单测 | `cargo test --lib retrace_ledger` | 61 passed | **61 passed / 0 failed** | ✅ 吻合 |
| 全量 lib | `cargo test --lib` | 2144 / 0 / 137 | **2144 passed / 0 failed / 137 ignored** | ✅ 吻合 |
| doctest | `cargo test --doc retrace_ledger` | 1 passed | **1 passed**（`portal.rs - …StandbyWatch (line 51) - compile fail … ok`） | ✅ 吻合 |
| retrace_ledger 新增警告 | `cargo build --lib --tests \| grep -i retrace_ledger\|portal.rs` | 0 | **0 行命中** | ✅ 吻合 |
| 新增测试数 | 逐文件 `grep -c '#[test]'` | 13（standby 5 + short 8） | standby **5** + short_retrace **8** = **13** | ✅ 吻合 |
| S1 基线测试数 | 同上 @ `a9756715b1` | 48 | clocks 7 + log_replay 19 + portal 5 + state_machine 17 = **48** | ✅ 吻合 |
| 未触碰面 | `git diff a9756715b1 73c7b34bea -- adapter/book/log` | 零行 | **空**（`ledger_kernel/`、`nest_lifecycle.rs` 同为空） | ✅ 吻合 |
| 净 diff 形状 | `git diff a9756715b1 73c7b34bea -- retrace_ledger/` | 新增 3 文件 + mod.rs 两处追加 + tests/mod.rs 两 mod + tests/portal.rs 3 行 doc | **逐字符吻合**（mod.rs 仅 `pub mod portal` + `pub use portal::{…}`） | ✅ 吻合 |

**未撞上 #622 在制品编译失败**：全部测试在 `git archive` 隔离树 + 独立 `CARGO_TARGET_DIR` 中跑，
不受主工作目录波动影响。

### 1.1 compile_fail doctest 的真实性与归因（票面复核项 2）

- **真的是 compile_fail、不是 ignore**：`cargo test --doc` 输出行明确带 `- compile fail`
  （`portal.rs:51` 的代码块围栏即 ` ```compile_fail `），且计入 `1 passed`——`ignore` 的 doctest
  会显示为 `ignored`，不会显示 `compile fail`。
- **归因正确**（探针 P1）：把 doctest 正文原封不动搬进独立集成测试、只把 `place_order(watch)` 换成
  `place_order(pack)`（`ThirdPointPack`，确实实现了该 trait），**编译并通过**。故 import 路径、
  字段名、类型构造全部成立，那段代码块的编译失败只可能来自 `StandbyWatch: TradableSignal` 缺失。
  实施报告 §4.3 自陈的 `compile_fail` 局限（不保证失败原因）在**当下**已被 P1 补齐；该局限对**未来**
  重构仍成立（P1 是一次性证据，不是常驻守卫）。

---

## 2. 发现（按严重度）

### HIGH-1 — `consume()` 对未登记身份的调用永久毒化该身份，判败处置永不下发

**位置**：`rust/src/theta_v0/classifier/retrace_ledger/portal.rs:243-248`

```rust
pub fn consume(&mut self, identity: &RetraceKey) -> Option<ShortRetraceRecord> {
    if !self.consumed.insert(*identity) {   // ← 无条件先写 consumed
        return None;
    }
    self.records.get(identity).copied()     // ← 再查是否真有记录
}
```

`consumed.insert()` 在 `records.get()` **之前且无条件**执行。对一个尚未 `sync` 进门户的身份调用一次
`consume`（下游轮询早于账本落锤、或"先消费后同步"的调用序），该身份即被记为已消费；此后 `sync()`
把真记录登记进来，`consume()` 仍恒返回 `None`。

**探针 P3 复现**（全绿）：

```
portal.consume(&id)  → None   （尚未登记，返回 None 本身合理）
book = failed_ledger(...); portal.sync(&book);
portal.len()          → 1     （记录确已在档）
portal.balances_with(&book) → true  （账平断言照样成立）
portal.consume(&id)  → None   ← 这条判败永远取不出来了
```

**后果**：AC「重复消费幂等」的实际担保被击穿——锁记的是「调用过 `consume`」，不是「处置过一条记录」。
且**无任何可观测信号**：`len()`、`balances_with()`、`disposal_notices()` 全部显示正常，丢失静默。
裁定八「已入场者失败处置通知」在这条路径上等于漏发。

**严重度定性**：判 HIGH 是因为这是本票**唯一新增的有状态组件**上的逻辑缺陷，且缺陷落在票面明写的
三锁之一。缓解事实（不改定性）：`ShortRetracePortal` 目前**无生产消费方**（实施报告 §4.4 已登记，
消费方接线归 #575），故当下无生产面损害。

**修法**（供实施车参考，非本车裁定）：先查再标——`let record = self.records.get(identity).copied()?;
if !self.consumed.insert(*identity) { return None; } Some(record)`。

---

### MEDIUM-1 — 「一一对应去重」退化为「条数相等」：账平断言不比身份集合，`sync` 不订正偏离内容

**位置**：`portal.rs:236-240`（`sync`）、`portal.rs:259-261`（`balances_with`）；
测试 `tests/short_retrace.rs:66-83`、`:87-102`

`balances_with` 的实现是 `self.records.len() == ledger.short_retrace_records().len()`——**只比条数**。
`sync` 用 `entry(...).or_insert(record)`——已登记身份**静默保留旧值**，不订正。

**探针 P4 复现**：门户里塞 1 条账本里**根本不存在**的伪造身份，账本有 1 条真判败 →
`balances_with()` 返回 `true`，而两边身份集合**完全不相交**。

**探针 P5 复现**：先 `admit` 一条同身份但 `retest_end` 被改成 `{index: 999_999, price: -1}` 的记录，
再 `sync(&book)` → `or_insert` 不覆盖，`balances_with()` 仍为 `true`，
`disposal_notices()[0].position.index` = `999_999`（**处置通知带错位置**）。

契约裁定八的「一一对应去重」是 bijection 语义（门户记录集 ↔ 账本判败集逐条对应）；实现只做到
「键不重复 + 条数相等」，**双射从未被任何断言覆盖**。现有测试
`sync_is_one_to_one_with_ledger_failures` 只断言 `portal.len() == 2`，未断言身份集合相等，
更未断言内容相等——测试名声称的语义强于测试实际覆盖的语义（`no-patch-mentality` 禁条 5「声明膨胀」）。

**修法**：`balances_with` 改为身份集合（乃至逐条内容）相等；`sync` 的 `or_insert` 改 `insert`
（账本是真相，门户是投影），或在冲突且内容不等时显式报错。

---

### MEDIUM-2 — 判败缺回抽位置的重放态：全量不变量放行，只读派生 `panic`

**位置**：`portal.rs:177-179`

```rust
retest_end: evidence.retest_end
    .expect("判败必带回抽位置：重回快照框内已发生（适配器 missing 桶已挡）"),
```

S1 对**成立档**的同型 `expect`（评审基准 `book.rs:391` / 当前 HEAD `book.rs:546`）配有镜像不变量
兜底（基准 `book.rs:429-432` / 当前 HEAD `book.rs:612-615`：`if entry.state == Confirmed { assert!(evidence.retest_end.is_some()) }`）。
**#623 加了 `expect`，却没加判败侧的镜像不变量**——`assert_entry_invariants` 至今（含当前 HEAD，
#622 落地后）对 `Invalidated` 一侧零断言。

活路上该 `expect` 确实不可达（`adapter.rs:163-170` 的 `check_material_completeness` 挡死
`outcome.is_some() && retest_end.is_none()`）。但**重放路径不挡**：`log.rs:243-273` 的
`replay_settle` 不校验证据完整性；`log.rs:33-40` 模块头已如实声明「JSONL 落盘后被外部改写」是
在案敞口通道。

**探针 P6 复现**（两条，全绿）：手工构造一组记录，`NotConstituted{RetestReentered}` 的落锤证据
`retest_end: None` →
- `P6a`：`RetraceLedger::fold(...)` 返回 `Ok`，`assert_invariants()` **全绿放行**；
- `P6b`：`short_retrace_records()` **panic**（`should_panic(expected = "判败必带回抽位置")` 命中）。

**后果**：一个**只读派生函数**在边界输入下 panic，与 `log.rs:35` 的「边界输入即不信任输入……一律转成
`RetraceLogError` 返回，**不 panic**」声明相抵。S1 影子评审已把这类「声明的防线覆盖不到声明的范围」
定性为 MEDIUM（`formalization-validity-domain`：有效域 ≠ 定义域），本条同构。

**修法**：在 `assert_entry_invariants` 补判败侧镜像断言（一行，与成立档对称），使 `expect` 的
前提由不变量真正兜住。

---

### MEDIUM-3 — 幂等锁与通知发放路径互不相交，通知侧零幂等

**位置**：`portal.rs:243-248`（`consume`，有 `consumed` 集合）vs `portal.rs:263-266`
（`disposal_notices`）与 `portal.rs:162-164`（`RetraceLedger::failure_disposal_notices`）——
后两者**完全不读 `consumed`**。

**探针 P7 复现**：`consume` 成功一次、再次 `consume` 返回 `None`（幂等成立）之后，
`portal.disposal_notices().len()` 仍为 `1`，`book.failure_disposal_notices().len()` 仍为 `1`，
`portal.len()` 也不区分已消费/未消费；门户不提供「未消费列表」。

裁定八「已入场者失败处置通知」的自然消费入口就是**通知列表**（方法名、返回类型、票面 AC③ 的载荷
描述都指向它），而 AC②「重复消费幂等」的锁挂在**另一个方法**上。字面上 AC 两条各自成立，但
**名义与担保错位**：走通知路的下游拿不到任何幂等保证，走 `consume` 路的下游拿到的是记录而不是通知。

**修法**：或让 `disposal_notices()` 只发未消费的、`consume` 返回通知；或明确文档化两条路的分工并
在票面登记「通知路径无幂等」。当前 `portal.rs:263` 的 doc「已登记的每条记录各发一份」是**如实**的
（未声明膨胀），故定 MEDIUM 而非 HIGH。

---

### LOW-1 — 备战档禁区不是排他的：`StandbyWatch` 不是唯一合法入口（S1 LOW-4 残留）

`RetraceLedger::entries()` / `entry()`（基准 `book.rs:319-325` / 当前 HEAD `book.rs:466-472`）与
`RetraceEntry::registration()`（基准 `mod.rs:315-320` / 当前 HEAD `mod.rs:327`）在本票之后**仍全部
`pub`、仍无任何名分标记**，`RetraceEntry` 字段仍全 `pub`。任何消费方仍可 `ledger.entries()` 手搓一份
「备战档」，绕开 `TradableSignal` 闸门。

**S1 LOW-4 对 S3 的字面要求已满足**（契约 §497「备战档不得直接包装 `entries()` / `RetraceEntry`
公共字段」——`StandbyWatch` 确是结构不同、字段不同、互无 `From`/`Into` 的独立类型；本车全仓 grep
确认无 `impl From<StandbyWatch>`、无 `Deref`、`TradableSignal` 除 `ThirdPointPack` 外零实现）。
但 LOW-4 指出的**敞口本身未被关闭**，实施报告未把这一残留登记在案。

判 LOW 而非更高：LOW-4 原文明说「本条不要求 S1 改」，且票面 AC 只要求「类型面证明不可入信号通道」
（已达成），不要求封死裸读面。登记供 #575 消费方接线时一并裁。

### LOW-2 — `TradableSignal` 未封闭（unsealed）

`portal.rs:39` 是完全公开的标记 trait。crate 内任意模块（及未来任何下游）一行
`impl TradableSignal for StandbyWatch {}` 即可拆掉整道闸门。

**缓解（实质有效）**：真这么干时，`portal.rs:51` 的 `compile_fail` doctest 会因「该段代码变成能编译」
而转红——该 doctest 是有效的常驻回归守卫，不只是一次性证明。故不判 MEDIUM。建议后续用 sealed trait
（私有 supertrait）把「只有本模块能实现」写进类型，而不是靠一条 doctest 兜。

### LOW-3 — 裁定八「024:36 / 024:46 短差语义（上沿减 / 下沿补，campaign 同构）」无落位、未登记

契约裁定八的短差档条目原文含这一句；`ShortRetraceRecord`（`portal.rs:126-139`）只有 `side` +
两处位置，对「上沿减 / 下沿补」的仓位语义零表达。

票面 AC 列表**未含**此项（AC② 只列亚型签 / 键唯一 / 幂等 / 账平），故**不判本票违规**。但实施报告
§4「偏离 / 存疑」四条也未登记该缺口——裁定八与票面 AC 之间的这处差集应当在案，否则下游会以为
「裁定八短差档已全部落地」。

### LOW-4 — 实施报告 §1 的 E0583 清单漏一个模块

报告第 13 行列出 4 个缺失模块（`audit` / `rebase_audit` / `dead_center` / `two_pass_evidence`），
实测 `5ea5082830` 隔离树编译报 **5** 个 E0583：`audit`、**`rebase`**、`rebase_audit`、
`dead_center`、`two_pass_evidence`（`tests/mod.rs` 在该 commit 有 10 条 `mod` 声明）。
不影响任何结论，登记订正。

---

## 3. 备战档禁区专项复核（票面复核项 4）

| 检查 | 结果 |
|---|---|
| `TradableSignal` 实现清单 | 全仓仅 `impl TradableSignal for ThirdPointPack`（`portal.rs:41`）；`StandbyWatch` 无实现 ✅ |
| `From` / `Into` / `Deref` 泄漏 | `retrace_ledger/` 内零命中（唯一 `Into` 是 `log.rs:452` 的 `impl Into<PathBuf>` 路径参数，无关）✅ |
| 字段表禁区 | `StandbyWatch`（`portal.rs:71-81`）五字段，**结构上没有** `retest_end` / `confirmed_as_of` / `death_certificate`；`standby_watch_has_no_retest_position_field_by_construction` 用穷尽解构把这点钉成编译期事实 ✅ |
| 状态过滤 | `watch_of`（`portal.rs:95-107`）只放 `Provisional`；`confirmed_and_failed_candidates_never_enter_the_standby_portal` 已锁 ✅ |
| 枚举确定性 | `entries()` = `BTreeMap::values()`，按 `RetraceKey` 全序；`standby_portal_is_deterministic_across_centers` 断言有序 ✅ |
| 名义标注落实 | 「盯这里不是买这里」落在 `portal.rs:43-47` 类型 doc + `portal.rs:16-23` 模块头禁区节；不止注释——`TradableSignal` + 字段表两道结构性墙 ✅ |
| 排他性 | ❌ 见 LOW-1：`StandbyWatch` 是新增的窄门，不是唯一入口 |

**结论**：备战档一侧**无实质发现**。类型面隔离做得比 AC 要求的更结实（trait 闸门 + 字段表缺席 +
compile_fail 常驻守卫三重），唯一保留是裸读面未封（LOW-1，非本票义务）。

---

## 4. 三锁逐锁探针结论（票面复核项 3）

| 锁 | 实现 | 探针 | 判 |
|---|---|---|---|
| 键唯一冲突拒绝 | `admit`（`portal.rs:227-233`） | P2a：二次 `admit` 返 `Err(DuplicateIdentity)`，`len()` 不变 | ✅ **名实相符** |
| 一一对应去重 | `sync`（`portal.rs:236-240`）+ `balances_with` | P2c 正路径成立；但 P4 / P5 证明只比条数、不订正内容 | ⚠️ **退化**（MEDIUM-1） |
| 账平断言 | `balances_with`（`portal.rs:259-261`） | P2c：同步前 `false`、同步后 `true`，断言确会触发 | ⚠️ **口径过弱**（MEDIUM-1） |
| 幂等消费（AC 另列） | `consume`（`portal.rs:243-248`） | P2b 正路径成立；P3 证明可被永久毒化；P7 证明与通知路不相交 | ❌ **HIGH-1 + MEDIUM-3** |

探针源码 10 条全绿，落在隔离树 `/tmp/shadow623/rust/tests/shadow623_probe.rs`（**从未进入仓库工作树**）。

---

## 5. 「单独检出不可编译」降级声明事实核（票面复核项 5）

| 事实 | 命令 | 结果 |
|---|---|---|
| `5ea5082830` 单独检出不可编译 | `git archive 5ea5082830 \| tar -x -C /tmp/shadow623-mid` + `cargo build --lib --tests` | ✅ **成立**：`error[E0583]` × 5（`audit`/`rebase`/`rebase_audit`/`dead_center`/`two_pass_evidence`） |
| `73c7b34bea` 树编译通过 | 同法建 `/tmp/shadow623-fin` | ✅ **成立**：`Finished dev profile in 25.87s`，零 error |
| 最终态干净（零 #622 内容） | `git diff a9756715b1 73c7b34bea -- retrace_ledger/` | ✅ **成立**：净 diff 恰为本票三新文件 + 三处纯追加，`adapter/book/log` 零行 |

**定性（本车意见，最终归编排侧）**：判 **LOW**，不构成交付面降级。理由——
(a) 票面 AC 与 `git-workflow.md` 均无「每个 commit 单独可编译」条款；
(b) 交付以最终态计，最终态经独立隔离树复现为干净且全绿；
(c) 实施报告 §4.1 已**主动、完整、准确**自陈成因（`git commit -- <pathspec>` 的隐式重新 add 语义）
    并给出订正手法——这是诚实报告，不是掩盖。

**残留代价（登记在案）**：`git bisect` 或逐 commit CI 扫过 `5ea5082830` 这一点会撞硬编译失败；两条
订正 commit 不能把它从历史里移除。若该分支后续要跑逐 commit 门禁，需要 squash（须改写历史，本票禁
git mutation 之下实施车未做，判断正确）或在门禁里显式跳过该 SHA。

**方法论旁证**：本会话 memory 里已有 `feedback_commit_check_staged_area`（「commit 前查 staged 区」）
条目——本次事故是该条目的又一实例，且实施报告已把新的触发面（`git commit -- <pathspec>` 而非
`git add`）写清楚，值得并入该条记忆而不是另起。

---

## 6. 结果包六要素

1. **结论** —— 见「结论先行」：AC 全条达成，实施报告报数零造假，compile_fail doctest 真实且归因正确，
   降级声明事实成立但定性 LOW；HIGH×1 / MEDIUM×3 / LOW×4，集中在 `ShortRetracePortal`。
2. **定义依据** —— #574 契约裁定八（备战档禁区 / 短差档亚型签 + 三锁 / 通知非信号 / 总禁区不进口外部
   状态）逐条比对 `portal.rs` 实现；#623 票面 AC 四条逐条比对测试名；#621 影子评审 LOW-4 的 S3 义务
   （契约 §497）比对 `StandbyWatch` 类型形状。三锁的「一一对应」按 bijection 语义解——这是「一一对应」
   在数学与契约语境下的标准所指，也是 §102 实施报告自陈「三锁真正有意义的地方是下游消费者多次同步
   时是否会重复处理」所要求的强度。
3. **边界条件**（结论翻转条件）——
   - HIGH-1 翻转：若编排侧裁定 `consume` 的契约就是「按调用次数幂等」而非「按记录处置幂等」，则 P3
     是预期行为，HIGH-1 降为文档缺失（LOW）。本车不认为该读法与 AC「重复消费幂等**不重复处置**」
     （测试名 `consume_is_idempotent_and_does_not_double_process`）相容。
   - MEDIUM-1 翻转：若「一一对应去重」的契约口径就是「条数守恒」，则 P4/P5 不是缺陷。需编排侧对
     裁定八该词单裁。
   - MEDIUM-2 翻转：若编排侧认定「JSONL 被外部改写」通道下 panic 是可接受的 fail-loud（而非须转
     `RetraceLogError`），则降为 LOW；但「成立档有镜像不变量、判败档没有」的不对称仍需订正理由。
   - LOW-3 翻转：若「上沿减/下沿补」已由票面裁定归 #575/#624，则本条不成立（本车未见该裁定文本）。
   - 全部结论的公共前提：评审基准 `573644425d` 快照 + #623 交付面在其后零改动（已复核）。
4. **下游推论** —— (a) #575 消费方接线前须先解 HIGH-1，否则接上就是漏发处置的生产面；(b) MEDIUM-1
   未解时，任何以 `balances_with()` 作验收行的下游验收都不构成真实对账，验收行强度须重估；
   (c) MEDIUM-2 若不补不变量，S4 golden 日志对拍一旦引入被改写样本，会以 panic 而非可诊断错误出现；
   (d) LOW-1 使「备战档禁区」在架构上仍是**加法**（多一道窄门）而非**减法**（封死宽门），#575 登记
   消费方时须显式约束只走 `standby()`。
5. **谱系引用** —— `no-patch-mentality` 禁条 5「声明膨胀：在注释/文档中声明代码不具备的能力」
   （090 号严格性语法规则）：MEDIUM-1（测试名声称的语义强于实际覆盖）、MEDIUM-3（AC 名义与担保
   错位）据此定性。`formalization-validity-domain`「有效域 ≠ 定义域」（231 号）：MEDIUM-2 是其在
   「边界输入 fail-loud 声明」上的同构实例，与 #621 影子评审 MEDIUM-3 同族。`no-workaround`：本次
   **未**发现绕过概念矛盾的行为，无 `/escalate` 触发。买卖点身份账本域仍无已结算谱系条目
   （#621 影子评审已登记此事实），故不引用域内谱系。
6. **影响声明** —— 本评审**零代码改动、零 git mutation**。产出仅本文件
   `chanlun/review-results/shadow-623-s3-review-20260729.md`。探针 `shadow623_probe.rs` 全程只存在于
   隔离树 `/tmp/shadow623/rust/tests/`（`git archive` 导出的非仓库目录），**从未进入仓库工作树**；
   收口 `git status` 复核确认工作区无本车造成的任何改动（开工时的 #622/#576 在制品在评审期间由并行车
   自行提交，与本车无关）。本文不改动任何定义、模块或验收口径，仅登记事实与建议。

---

## 7. 探针源码（可复现）

隔离树建法与探针全文，供复核：

```bash
mkdir -p /tmp/shadow623 && cd <repo> && git archive 573644425d | tar -x -C /tmp/shadow623
# 探针写入 /tmp/shadow623/rust/tests/shadow623_probe.rs（见下），然后：
cd /tmp/shadow623/rust && CARGO_TARGET_DIR=/tmp/shadow623-target cargo test --test shadow623_probe
# → 10 passed; 0 failed
```

关键三条（其余为常规复核，全文见隔离树）：

```rust
// P3：consume 对未登记身份的永久毒化
#[test]
fn p3_consume_poisons_unregistered_identity_forever() {
    let center = frame(1_200);
    let id = key_of(center, 3);
    let mut portal = ShortRetracePortal::new();
    assert_eq!(portal.consume(&id), None);            // 尚未登记（合理）
    let book = failed_ledger(center, 3);
    portal.sync(&book);
    assert_eq!(portal.len(), 1);                      // 记录已登记
    assert!(portal.balances_with(&book));             // 账平断言照样成立
    assert_eq!(portal.consume(&id), None);            // ← 永远取不出来
}

// P4：账平断言只比条数
#[test]
fn p4_balances_with_is_count_only_not_identity_matched() {
    let book = failed_ledger(frame(1_200), 3);
    let mut portal = ShortRetracePortal::new();
    let bogus = ShortRetraceRecord { identity: key_of(other_frame(300), 11), /* … */ };
    portal.admit(bogus).unwrap();
    assert!(portal.balances_with(&book));             // 身份集合完全不相交，账仍"平"
}

// P6：判败缺回抽位置 —— 不变量放行、只读派生 panic
#[test]
fn p6a_tampered_failure_without_retest_passes_all_invariants() {
    let book = RetraceLedger::fold(provenance(), &tampered_records(frame(1_200), 3)).unwrap();
    book.assert_invariants();                         // 全绿放行
}
#[test]
#[should_panic(expected = "判败必带回抽位置")]
fn p6b_short_retrace_records_panics_on_that_entry() {
    let book = RetraceLedger::fold(provenance(), &tampered_records(frame(1_200), 3)).unwrap();
    let _ = book.short_retrace_records();             // panic
}
```
