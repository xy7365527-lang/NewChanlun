# 双轴评审：#691（N3 硬化二轮）

- **对象**：`/tmp/wt-691` 分支 `ticket-691`，`git diff 23a1869849..HEAD`（3 commits）
- **评审时点**：2026-07-29；评审人上下文与实施零瓜葛（新上下文，禁自评）
- **背景报告**：`chanlun/review-results/code-review-issue676-20260729.md` §LOW-1/2/3/4/INFO-1
- **手段**：全程前台自做，逐项独立实测；每条负控注入后 `git checkout --` 逐字还原并 `git status --porcelain` 复验空
- **判定**：**CONDITIONAL**（1×M + 3×L；五条 Acceptance 全部实测成立，M 是同一「锁的强度」病灶在本票新增代码中的复发）

---

## 〇、实测三元组

| 门 | 基线（`23a1869849`，独立 worktree 取干净快照） | HEAD（`42a81bc9bf`） | 增量 |
|---|---|---|---|
| `cargo test --release --lib` | **2571 passed / 0 failed / 138 ignored** | **2571 / 0 / 138** | **0**（LOW-3/LOW-4 只改既有测试的消息与控制流写法，不新增用例） |
| `cargo test --release --bin p123_fast_replay` | 12 / 0 / 0 | **13 / 0 / 0** | **+1**（`chain_dump_observe_advance_produces_real_certificates_on_first_beat`） |
| `cargo test --release --bin issue550_event_battery` | 0 / 0 / 0（bin 无测试模块） | **1 / 0 / 0** | **+1**（`settle_chain_readout_digest_and_summary_precede_replay_delta`） |

**本票新测试 = +2 条**（都在 bin 侧，故 lib 计数不动）。

> 评审令给的基线口径「2581/0/138 附近」与实测不符：base 与 HEAD 两侧独立测得均为 **2571**，三条 commit message 自述的 2571 才是对的。工作区全程 clean（`git status --porcelain` 空），基线取自专用 worktree `/tmp/wt-691-base`（已 `git worktree remove`），不受 AGENTS.md「共享 worktree 计数污染」那条影响。

**辅助门**：
- rustfmt（`rustfmt --check --edition 2021`，只数落在本文件内的 hunk）：`p123_fast_replay.rs` base 20 → HEAD 20；`issue550_event_battery.rs` 0 → 0；`classifier/mod.rs` 38 → 38。**零新增漂移**，且与 commit message 自述的「20 处 / 38 处既有漂移，改动前后计数不变」逐字符合。
- 逐 commit 可编译性（`cargo test --release --lib --bins --no-run`）：`08a6c6bc3e` exit=0 / `ba7fa0d5eb` exit=0 / `42a81bc9bf` exit=0。满足 delivery-discipline 关票门第 4 子句。
- 两个 bin 的测试编译告警：**新增代码零告警**（全部 58 条告警都来自 lib 既有 dead_code / unused_import，与本 diff 无关）。

---

## 一、Spec 轴：票面五条 Acceptance 逐条

| # | Acceptance | 独立实测 | 判定 |
|---|---|---|---|
| 1 | LOW-1 病态实现「删 advance 保门」变红（反事实证据入 commit message） | **成立** — 见 §二 NC-1 | ✅ PASS |
| 2 | LOW-2「every 硬编码」注入变红（反事实证据入 commit message） | **成立** — 见 §二 NC-2，读数逐字复现 | ✅ PASS |
| 3 | LOW-3 措辞复精度；LOW-4 改 match 后加第三档编译红 | **成立** — 见 §二 NC-3 与 §三 | ✅ PASS（LOW-3 有残留，见 L-2） |
| 4 | INFO-1 机器锁落地（顺序约束违反时测试红） | **成立** — 见 §二 NC-4；但锁的参照态退化，见 **M-1** | ⚠️ PASS with M |
| 5 | lib 零新增红；p123 bin 12/0 | lib 2571/0/138 前后一致、零新增红 ✅；p123 bin 实测 **13/0** 而非票面写的 12/0 | ✅ PASS（口径偏离是**加强**，见 INFO-1） |

**Spec 轴一句话**：五条 Acceptance 全部实测成立，四条反事实负控我逐条复做且读数与 commit message 逐字一致——commit 自述没有一处夸大；唯 AC-4 落地的机器锁参照态退化（对目标失效模式有分辨力，对邻近失效模式无），AC-5 的「12/0」被新增用例改成 13/0（加强，非违约）。

---

## 二、四条反事实负控：逐条复做

全部由我在 `/tmp/wt-691` 上亲手注入并跑测，非采信 commit 自述。

### NC-1（LOW-1）删 `book.advance` 整条、保留节拍门与两条 `#[cfg(test)]` 探针语句

注入：`p123_fast_replay.rs:616` 的
`let delta = self.book.advance(&cache.candidate_streams(), as_of);`
→ `let delta: Vec<classifier::chain_cert::TowerChainCertificate> = Vec::new();`

实测：**12 passed / 1 failed**。红点落在
`tests::chain_dump_observe_advance_produces_real_certificates_on_first_beat`
（`p123_fast_replay.rs:4407` = `assert!(dump.seq > 0, ...)`）。

**同时确认**：`chain_dump_cadence_advances_on_beat_and_on_last_bar`（探针序列断言）在同一注入下**仍全绿**——即「把探针记录点从 `advance` 之前挪到之后」这一半修法**独立分辨力为零**，实质闭合完全来自新增的非空候选流用例。commit message 对此**主动自陈**（"仅挪位置不足以堵住…挪位后仍全绿漏网"），照实、无隐瞒。派生出 L-1。

### NC-2（LOW-2）`self.every` 硬编码成字面量 `3`

注入：`p123_fast_replay.rs:616` 门 `if !is_last && (as_of + 1) % self.every != 0` → `% 3`

实测：**12 passed / 1 failed**，红点 `p123_fast_replay.rs:4306`：
```
assertion `left == right` failed: every=2：节拍门必须恰好在节拍根与末根放行（硬编码 every=3 的病态实现在此变红）
  left: [2, 5, 6]
 right: [1, 3, 5, 6]
```
与 commit message 自述的 `left=[2,5,6] right=[1,3,5,6]` **逐字符相同**。`every=3` 那组仍绿——正是「单一夹具锁不住参数依赖」的反面证据，doc 里也这么写了。

**两组期望值我各自独立重算**（不采信注释）：门放行条件 = `is_last || (as_of+1) % every == 0`，`bars=0..=6`、`is_last at 6`：
- `every=3`：`(1,2,3,4,5,6,7) % 3` → 只有 `as_of=2`(3)、`as_of=5`(6) 命中；`as_of=6` 的 `7%3=1≠0` ⟹ 只能来自末根支 ⟹ **[2,5,6]** ✅
- `every=2`：`(1..7) % 2` → `as_of=1`(2)、`3`(4)、`5`(6) 命中；`as_of=6` 的 `7%2=1≠0` ⟹ 只能来自末根支 ⟹ **[1,3,5,6]** ✅

两组**各自独立算对**，且两组期望值不同（真区分参数），末根支在两组里都由 `(6+1)%every≠0` 独立锁住（不是靠节拍支蹭到的）。

### NC-3（LOW-4）临时加第三档 enum variant

注入：给 `PrefixCacheReuse` 加 `NegativeControlThirdVariant`。

实测：`cargo test --release --lib --no-run` **exit=101，恰 1 个 error**：
```
error[E0004]: non-exhaustive patterns: `PrefixCacheReuse::NegativeControlThirdVariant` not covered
    --> src/theta_v0/classifier/mod.rs:4724:19
     |
4724 |             match reuse {
     |                   ^^^^^ pattern `...NegativeControlThirdVariant` not covered
```
**编译红精确落在 `match reuse` 的 scrutinee 上**（4724 是加了 2 行 variant 后的行号，与 commit message 自述的「4724 行」一致；HEAD 干净态该 match 在 4722）。

**反向对照（我加做的，用于确认这条修法不是空转）**：在第三档仍在的前提下把 `match` 回退成旧 `if reuse == PrefixCacheReuse::FreshPerPrefix { … }` ⟹ **0 error，exit=0**，即静默落入 shared 语义、不报红。`match` 的抵抗力是真的。

### NC-4（INFO-1）把 `replay` 计算挪到 `summary`/`digest` 之前

注入：`issue550_event_battery.rs` 的 `settle_chain_readout` 内，`let replay = book.advance(...)` 从 `digest` 之后挪到 `summary` 之前。

实测：**0 passed / 1 failed**，红点 `issue550_event_battery.rs:660`（summary 断言）：
```
left:  ChainBookSummary { revisions: 1, chains: 1, open: 1, ..., edges: 1, skip_edges: 1, nodes_alive: 2, ... }
right: ChainBookSummary { revisions: 0, chains: 0, ..., edges: 0, by_path_len: {}, by_root_level: {} }
```
与 commit message 自述的「从空簿（revisions=0/chains=0）变成含 wide 追加证书的簿（revisions=1/chains=1/edges=1）」**逐字符相同**。

**「重放 Delta 非零」场景真成立**：干净态下测试内 `assert!(readout.replay > 0)` 通过；我把阈值临时改 `> 9999` 迫使 panic 打出实值 ⟹ **`replay = 1`**。夹具确实制造出了非零重放 Delta，不是空转前提。

**锁断言的是不是「顺序」本身**：是。断言形式为 `readout.summary == narrow_only_book.summarize()` / `readout.digest == narrow_only_book.digest()`，`narrow_only_book` 是测试内独立构造、只喂过 `narrow` 的对照簿；顺序一颠倒，`readout` 侧就多出 replay 追加的 wide revision，等式当场破。**对「顺序颠倒」这个目标失效模式，分辨力实测为真。** 但参照态退化——见 M-1。

---

## 三、LOW-3：措辞与断言实际口径逐字对表

| 项 | 内容 |
|---|---|
| 比对面（实际代码） | `classifier/mod.rs:5015-5016`：`causal_book.heads()` / `terminal_projection_book.heads()`；`heads()` 的 doc（`chain_cert/mod.rs:822`）= 「**每 key 最新 revision**，key 升序」 |
| 旧断言消息（#676 被点名处） | 「共有 key 的**证书**在候选全集计数以外的字段分叉」——丢了 revision 精度 |
| 新断言消息（`classifier/mod.rs:5032`） | 「共有 key 的**最新 revision** 证书在候选全集计数以外的字段分叉」 |
| 全史侧另有锁 | `chain_certificate_book_incremental_equals_full_replay`（实测在 lib 全绿集内） |

**结论**：报告 §LOW-3 点名的锚（原 `:4671`，现 `:5032`）措辞与实际比对面 `heads()` **逐字一致**，精度已复。残留见 **L-2**。

---

## 四、发现清单

### M-1（Standards / Spec AC-4，`issue550_event_battery.rs:625-673`）INFO-1 机器锁的参照态退化 ⟹ 邻近失效模式漏网，且 doc 的「真推进 / 证明」超出见证面

**锚**：`rust/src/bin/issue550_event_battery.rs:629`（doc「先把簿真推进到 `as_of=100`」）、`:632`（doc「**证明**二者取的是重放前的簿态」）、`:664-672`（两条 `assert_eq!`）

**实测事实**：`book.advance(&narrow, 100)` 的效果是**零**——NC-4 打出的 `right` 侧（即 `narrow_only_book.summarize()`）是**全零** `ChainBookSummary`（`revisions: 0, chains: 0, edges: 0, by_path_len: {}`）。单条 L0 候选形不成链（链需要跨级父子边），所以「先把簿真推进到 `as_of=100`」这句里的**簿从头到尾是空的**。锁的两条 `assert_eq!` 因此都是「== 空簿读数」。

**我自构的负控（NC-5）**：把 `settle_chain_readout` 里
`let summary = book.summarize();` → `chain_cert::ChainCertificateBook::default().summarize();`
`let digest = book.digest();` → `chain_cert::ChainCertificateBook::default().digest();`
（即**整个把真实簿丢掉**，读数改取任意空簿）
⟹ **1 passed / 0 failed，全绿漏网。**

**为什么这是 M 而不是 L**：本票的存在理由就是消除「锁在扩展或病态实现下静默放宽」这一族缺陷（#676 的 MED-1/MED-2/LOW-1/LOW-2/LOW-4 全是这个形状）。这条新落地的机器锁把同一缺陷带了回来：它只对「顺序颠倒」有分辨力（NC-4 证实），对「读数根本不来自那本簿」为零；而 doc 写的是「**证明**二者取的是重放**前**的簿态」——实测只证明了「等于某个空簿」，这正是 090 §声明-能力一致要挡的形状。「先把簿真推进到」里的「**真推进**」用词，与本票 LOW-1 判定为不合格的那个「真推进」是同一个词、同一种滑移。

**修法（一处夹具改动即闭）**：让 `narrow` 本身产出**非空**簿——例如 `narrow = streams_of(&[observation(0, 20, (20,40)), observation(1, ...)], 100)`，`wide` 再叠 L2；参照态一旦非退化，NC-5 那族（读数取自任意空簿 / 取自 default 簿）当场变红。doc 同步把「真推进」改成照实表述、「证明」降格为「见证」。

### L-1（Standards / 090，`p123_fast_replay.rs:545-547`）`advanced_at` 字段 doc 与同文件测试 doc 表述张力，探针独立分辨力实测为零

**锚**：`:545-547` 字段 doc「`self.book.advance(...)` **真正调用之后**记下 `as_of`——见证的是「推进执行」，不是「越过节拍门」」 vs `:4366-4369` 测试 doc「单纯挪动语句顺序的探针只见证「程序走到了这一行」，不见证「这一行真的调用了 `advance` 并产生效果」」。

**实测**：NC-1 下 `advanced_at` 仍为 `[2,5,6]`，cadence 测试全绿 ⟹ 探针**没有**见证「推进执行」。两处 doc 对同一探针给出相反的能力声明；字段 doc 单读会让读者高估探针。

**缓解（照实）**：commit message 与 `:4366-4369` 已明确自陈这一半修法不足、实质闭合在新用例；票面 AC-1 也已由新用例满足。故 L 不 M。

**修法**：字段 doc 改为「记录点在 `advance` 之后（控制流意义上的推进执行点）；本探针**不**见证 `advance` 产生了效果——那一面由 `chain_dump_observe_advance_produces_real_certificates_on_first_beat` 锁」。

### L-2（注释与代码一致，`classifier/mod.rs:5018`）LOW-3 的精度限定只补进 assert 消息，正上方行内注释仍缺

**锚**：`:5018-5019` 行内注释「硬锁（唯一不变量）：两驱动共有 key 的**证书**逐字段相同，唯独候选全集计数…」——与被修好的 `:5032` 相隔 14 行，缺同一个「最新 revision」限定。同一处精度损失在同一测试里残留一份。

**修法**：`:5018` 同步补「最新 revision」（一处措辞）。

### L-3（Fowler / 判据同源，`p123_fast_replay.rs:4313-4362`）新增 ~30 行夹具与 lib 侧逐字同构，「同口径」只有注释声明、无机器锁

**锚**：`cadence_probe_seg`（`:4313`）、`cadence_probe_candidate_rich_segments`（`:4325`）、新用例内的 closes 公式（`:4387-4389`）

**实测同构**：逐字对照 `classifier/mod.rs` 的 `seg`、`candidate_rich_segments`、`chain_fixture`——LCG 常数（`0x9e3779b9…` / `6364136223846793005` / `1442695040888963407`）、`swing = 20 + ((state>>32) % 90)`、方向交替、`i*4 / i*4+4` 段索引、closes 的 `100 + if i%8<4 {35} else {-35} + i/16`，**全部逐字相同**。

「跨 crate 不可见」这条理由**站得住**（lib 的 `#[cfg(test)] mod tests` 私有项在 bin 编译时不存在；仓内也没有可复用的通用 test-support 缝——`rust/src/theta_v0/strategy/coverage/test_support.rs` 是 coverage 专用），且 doc `:4323-4324` 已照实披露。但「同口径」是一条**现在时声明、零机器锁**：lib 侧改夹具，bin 侧静默漂移，两处「同口径」的注释同时变假而无一处报红。

**修法（择一）**：(a) 把夹具提到 lib 的 `pub`/`pub(crate)` test-support 面（有 `cfg(feature)` 成本）；(b) 保留双份，但加一条把两侧输出对拍的测试；(c) 至少把注释从「同…口径」降格为「照 `<commit hash>` 时点的 `candidate_rich_segments` 手抄一份，两侧无自动对拍」。

### INFO-1（Spec 口径）票面 AC-5 的「p123 bin 12/0」被新增用例改成 13/0

实测 13/0（12 既有 + 1 新增）、battery bin 0/0 → 1/0。这是 LOW-1 修法本身要求的加强，不是违约；照实登记，供关票时按 delivery-discipline「关票评论各项在此刻均已为真」改写数字，别照抄票面的 12/0。

### INFO-2（altitude 观察，不作要求）`advanced_at` 这个 `#[cfg(test)]` 探针字段的可替代性

有了非空候选流夹具后，dump 行本身携带 `as_of`（`chain_dump_line(certificate, self.seq, as_of)`），节拍序列原则上可从 sink 读出，无须在**生产结构体**上挂 `#[cfg(test)]` 字段。**但不完全可替**：某些节拍根的 Delta 可能为空 ⟹ sink 只给 advance 时点的子集，拿不到精确序列。故照实登记为观察，不建议本票动。

---

## 五、纪律族逐条对表

| 纪律 | 判定 | 依据 |
|---|---|---|
| **090 严格性**（概念清晰 / 声明-能力一致 / 无模糊地带） | ⚠️ 部分 | 三条 commit message 的每个数字我都复现且逐字符相同（无夸大，这一维很硬）；扣分在 live code doc：**M-1**（「证明」「真推进」超出见证面）、**L-1**（同一探针两处 doc 相反）、**L-3**（「同口径」无锁） |
| **判据零重写** | ✅ | 新增两条用例都走生产判据：p123 侧 `classify_with_tower_events_incremental` + `ChainCertificateBook::advance`；battery 侧 `CandidateEventBook::advance` + `ChainCertificateBook::{advance,summarize,digest}`。bin 侧零判据重写；重复的只是**夹具生成器**（L-3） |
| **注释与代码一致** | ⚠️ 部分 | L-2（`:5018` 精度限定漏补）、L-1（字段 doc）、M-1（「真推进」）。反面：LOW-4 的注释「扩展第三档时编译红」经 NC-3 实测为真 |
| **Fowler**（Duplicated Code） | ⚠️ | L-3：本票在 #676「Fowler #2 重复真消除」之后新增一份 ~30 行逐字重复夹具（有跨 crate 硬约束，已披露） |
| **Fowler**（命令查询分离 / Long Function） | ✅ | 本 diff 未回退 #676-2 的拆分；`settle_chain_readout` 仍是唯一改簿点，`print_chain_readout` 未碰簿 |
| **LOW-4 类型级守卫** | ✅ | `match` 穷尽 + NC-3 编译红 + 反向对照（旧 `==` 不红）双向证实 |
| **rustfmt 零新增漂移** | ✅ | 20/0/38 前后逐一相同（§〇） |
| **delivery-discipline 关票门第 4 子句**（逐 commit 可编译） | ✅ | 三个 commit 均 exit=0（§〇） |
| **AGENTS.md 测试指纹污染** | ✅ 已排除 | 工作区全程 clean；基线取自专用 worktree 干净快照 |

---

## 六、090 限度（本次评审自身的见证面边界）

照实登记我**没有**做的事，不冒充覆盖：

1. **未复做的负控**：#676 报告 §项1 里「永不推进（门后即 return）」与「相位错一位（`as_of % every`）」两条我没重跑（票面未列为 AC，且这两条的红点与 NC-2 同一断言）。它们在本票 doc 里被写成「实做记录，非推想」——**这半句我未验证**。
2. **未跑真实数据对拍**：battery bin 的 20k / 500k 真实窗口 `ISSUE641_CHAIN` 四行逐字节对拍未做（#676 已复做过，本 diff 只往 battery bin 追加 `#[cfg(test)] mod tests`，对生产路径零改动——`git diff` 显示四行 println 全在上下文行，无 `+`/`-`）。因此「四行逐字节不变」我**按 diff 结构判定**，不是按运行输出判定。
3. **未跑 clippy / fixture 漂移检查**：CI 无 fmt/clippy job（`grep -n "clippy\|cargo fmt\|rustfmt" .github/workflows/ci.yml` 无命中）；本 diff 不触 `formal/`，故 CLAUDE.md 的 fixture 漂移 gate 不触发。
4. **M-1 的修法未实测**：我给的「让 `narrow` 产出非空簿」是修法建议，**未实做验证**能否一次通过（补 L1 观测后 `wide`/`narrow` 是否仍能造出非零 replay，需实施侧自测）。
5. **未改任何文件的最终状态、未关 issue**：五条负控注入全部还原，`git status --porcelain` 空、`git diff 23a1869849..HEAD --stat` 仍为 3 files / 268 insertions / 26 deletions；临时基线 worktree `/tmp/wt-691-base` 已 `git worktree remove`。

---

## 七、判定

**CONDITIONAL** — 放行条件：修掉 **M-1**（让 INFO-1 机器锁的参照态非退化，并把该用例 doc 里「真推进」「证明」改成照实表述），并在同一轮顺手清掉 **L-1 / L-2**（各一处措辞）。**L-3** 可登记进下一张尾部票，不阻塞。

- **Standards 轴一句话**：四条负控我逐条复做且读数与 commit 自述逐字符相同、rustfmt 与逐 commit 可编译零瑕、LOW-4 的类型级守卫经双向对照证实有效——但本票新落地的 INFO-1 机器锁把它自己要消除的那个病灶（锁在邻近病态实现下静默放宽 + doc 声明超出见证面）原样带了回来。
- **Spec 轴一句话**：票面五条 Acceptance 全部实测成立（AC-5 的「12/0」被新增用例正当地改成 13/0），无一条靠 commit 自述过关；AC-4 是「字面 PASS、实质带 M」。
- **实测三元组**：**lib 2571 / 0 / 138（基线同为 2571/0/138，增量 0）；p123 bin 13 / 0 / 0（基线 12/0，+1）；battery bin 1 / 0 / 0（基线 0/0，+1）**。
