# #634 实施报告：`cand_event.rs` 按域拆 + #612 观察/事件类型分离

- **票**：GitHub #634（parent #576 段 2/3 转移；上游 #612 已关，交付随本票；票面「Blocked by #614」已核销，见 §1）
- **日期**：2026-07-29
- **工位**：worktree `/private/tmp/wt-634`，分支 `ticket-634`，基 = main HEAD `7f0bad3607`
- **模型档**：高难（opus）
- **交付状态**：改动全留 worktree **未提交面**（本票禁一切 git mutation，编排方统一提交）
- **名分**：拆分属「现役」延续，不产新名分（纯移动不产新名；#612 的 `ObservedState` 是既有
  `CandidateState` 的三态子域类型，不是新概念）

---

## 0. 一句话结论

`cand_event.rs`（1417 行）按票面三域拆成 `cand_event/{key,book,observe}.rs` 各带 `mod tests`
（最大 653 行，全部 ≤800），**零行为变化四件套全绿**；随后落地 #612 观察/事件类型分离——
新增三态 `ObservedState` 使 `∅ → Invalidated` 从「运行期 assert 拦」变为**编译期不可构造**
（负控实验实测 `E0599`），退役的运行期 panic 锁 1 处、保留深度防御 `unreachable!` 2 处
（理由见 §5）。

**指纹**：基线 `2583 passed / 0 failed / 138 ignored`；段 2 拆分后 `2583 / 0 / 138`（严格相等，
测试名叶集合逐字一致）；段 3 后 `2582 / 0 / 138`（−2 个退役 `should_panic` +1 个新编译期锁，
差额逐条对账见 §6）。全程 `0 failed` 硬杠守住；编译 warning 恒为基线 55。

---

## 1. main 当前形状重核（段 3 前提订正）

票面写于 ticket-55x 线，其两条前提在 main 上**均不成立**，开工前逐条重核：

| 票面陈述 | main @ `7f0bad3607` 实测 | 处置 |
|---|---|---|
| 输入在 `ticket-550/551/552/553` 线，与 main 分叉、对侧 35 提交未并入 | N1+N2 八票链已移植入 main（merge `a559d2edba`，map #529，在 #614 并线之后）。`cand_event.rs` 在 main | 「Blocked by #614」按此**核销**，以 main 当前形状开工 |
| `cand_event.rs` 1120 行 | **1417 行**（含移植后增量：生产 862 + `mod tests` 555） | 数字订正，域划分按 1417 行实际内容做 |
| `CandidateObservation` **不含** `Invalidated` 变体，∅→Invalidated 编译期不可构造 | **前提不成立**。main 上观察侧与事件侧**共用同一个四态枚举** `CandidateState`（`Provisional`/`Unresolved`/`Confirmed`/`Invalidated`），`CandidateObservation.state: CandidateState` ⟹ `Invalidated` 在类型层**可表达** | 见下「订正结论」 |

### 段 3 前提订正结论

`pub enum CandidateObservation` grep 不命中，是因为票面把「观察结构体不含该变体」记成了既有事实；
实测 main 上的真实形状是：

- `CandidateState`（`cand_event.rs:62`，四态，含 `Invalidated`）**同时**担任事件侧状态
  （`CandidateEvent.state`）与观察侧状态（`CandidateObservation.state`）；
- `Invalidated` 的唯一构造点是 `invalidate()`（事件簿对既有 revision 的判决，写 `invalidated_at`、
  递增 `revision`、置 `supersedes_revision`），两条失效路径（观察缺席 / 区间回缩）都要求存在 `prior`；
- 因此「∅ → Invalidated」在 E2E-D5 状态机里是禁止边，但类型层拦不住，main 上靠**两道运行期锁**守：
  1. `reject_unobservable_invalidation()`（`cand_event.rs:486`，`advance()` 入口 assert）；
  2. `event_probe::on_append()` 内两处 `unreachable!`（`:406` / `:423`），附 `should_panic` 测试
     `:1067` / `:1077`。

**故段 3 的落法不是「确认已不可构造」，而是「把该边真正搬到编译期」**：引入观察侧三态类型
`ObservedState`（`key.rs`），`CandidateObservation.state: ObservedState`。这与票面意图一致
（票面要的是编译期不可构造），只是起点比票面记载的更靠前一步。

---

## 2. 拆分域划分与理由

原文件 `cand_event.rs` → 目录 `cand_event/`，`mod.rs` 只做模块头 + `mod` 声明 + re-export：

| 文件 | 行数 | 内容 | 划分理由 |
|---|---|---|---|
| `mod.rs` | 47 | 模块头 doc + `mod` 声明 + 全部 pub 项 re-export | 保证外部 `cand_event::X` 路径**逐字不变**（四件套②的机器载体） |
| `key.rs` | 370 | 常量（FNV/规则版本）、`CandidateKind`、`ParentFingerprint`、`CandidateKey`、`CandidateState`、**`ObservedState`**（新）、`StructuralPredicates`、`CandidateEvent`、`CandidateStreams`、闭区间四谓词、`CandidateProjection` + `projection()`；`mod tests` = 5 个（4 个区间真值表 + 1 个 `ObservedState` 穷举 match 编译期锁，#634 修复轮 LOW-2 由 `book::tests` 移入，理由见下方脚注） | 票面「key/types」域。判据是**纯函数 + 纯类型**：零状态、零外部调用、零 I/O。四个区间谓词与身份键同域，因为它们都是「候选是什么」的定义层，不涉及生命史 |
| `book.rs` | 616 | `CandidateEventBook`（`advance`/`invalidate_unseen`/`advance_observation`/`latest_event`/`append`）、`event_probe` 模块、`invalidate`、`make_revision`、`same_projection`；`mod tests` = 11 个（终态原为 12，#634 修复轮 LOW-2 挪出 1 个到 `key::tests`，见下方脚注；**13 是段 2 中间态数字**，见「修复轮」节 LOW-1） | 票面「event book」域。修订协议 + 三只钟 + 终态不复活 + 幂等跳过 + 覆盖探针，是唯一持可变状态的域 |
| `observe.rs` | 400 | `CandidateObservation`、`structural_observations_for_level`、`merged`、`StructuralScan`、`structural_observation`、`trend_observation`、`pan_observations_for_level`、`observations_for_level`、`stable_id`、`PAIR_ID_SEED`；`mod tests` = 3 个 | 票面「observation 产出」域。单次塔扫描 → 观察的投影侧（结构门投影 + Pan 证书投影 + 同 episode 归约），不持状态、不判修订 |
| `fixtures.rs` | 136 | `#[cfg(test)]` 共用夹具：`center`/`segment`/`observation`/`deferred_envelope_break_scan`/`scan_observations`/`extended_episode_scan` | 原为单文件 `mod tests` 内的私有夹具，拆分后被 `book`/`observe` 两域的 tests 共用。夹具不带 `#[test]` ⟹ **不进测试名清单**（已实测，见 §4③） |

### 两条硬事实的遵守情况

1. **「迁测试治不了本」（#551 小修批实测：测试区迁出后生产部分仍 833>800）**——本票**真拆生产部分**：
   原 862 行生产代码按域分到 key 226 / book 355 / observe 275（各域自身的 `mod tests` 留在同文件）。
   任何单域的生产+测试合计都 ≤653。
2. **「`event_probe` hook 是 `#[cfg(test)]`，拆分必须留在本 crate 内」**——五个新文件全部在
   `newchan_rust` lib crate 内（`src/theta_v0/classifier/cand_event/`），**未迁 `rust/tests/`**。
   `event_probe::on_*` 的调用点仍是 `book.rs` 内的 `#[cfg(test)]` 行，探针断言全部有效
   （`growth_revision` / `invalidated_shrink` / `birth_confirmed` 三个非零锁在终跑里绿）。

### 跨域可见性处置

- `event_probe` 内 `use super::{CandidateEvent, CandidateState}`：`super` 现指 `book`，两个名字由
  `book.rs` 顶部 `use` 引入 ⟹ 原文逐字保留可解析，doc 里 `[`super::make_revision`]`、
  `[`super::CandidateEventBook::advance`]`、`[`super::reject_unobservable_invalidation`]`
  等 intra-doc 链接**全部仍在同文件内**，无需重锚。
- 三处 doc intra-link 因跨文件必须重锚（见 §4① B 类）。

---

## 3. panic 锁取舍理由（票面段 3 要求「实施定，说明理由」）

main 上有**三处**机器锁守 E2E-D5 禁止边。逐处裁定：

### ① `reject_unobservable_invalidation()` —— **移除**

- **裁定**：移除（连同 `advance()` 里的调用），原位留注释说明退役缘由与替代物。
- **理由**：它是「`observation.state == Invalidated` ⟹ assert 失败」的运行期 assert。观察侧状态改用
  `ObservedState`（无 `Invalidated` 变体）后，**该边不再是「被拦住」，而是不可表达**——携带
  `Invalidated` 的观察根本构造不出来。留一个永远无边可守的 assert 是死代码，且会误导读者以为
  这条边仍在类型层开放。
- **代偿**：`ObservedState` 的类型文档写全推导链（为什么 `Invalidated` 不是可观察状态、
  放行的后果是「终态但无失效钟」的畸形事件被 `is_terminal()` 永久钉死）；
  编译期性质由 §4④ 的负控实验 + `observed_state_domain_excludes_invalidated_and_lifts_pointwise`
  穷举 match 测试双重锁住。

### ② / ③ `event_probe::on_append()` 内两处 `unreachable!` —— **保留为深度防御**

- **裁定**：保留，改 doc 说明其性质已从「唯一机器载体」降为「深度防御」。
- **理由**（为什么不能一并移除）：`on_append` 的入参是 `next: &CandidateEvent`，
  而 `CandidateEvent.state` 是**四态**的 `CandidateState`——事件侧**必须**能表达 `Invalidated`，
  因为那正是 `invalidate()` 的产物。于是「`next.state != Invalidated`」在此处不是类型保证，
  而是**调用点纪律**：失效路径走 `invalidate()` + `on_invalidate()`，不经 `on_append`。
  把它变成类型保证需要给事件侧也分类型（「非失效事件」 vs 「失效事件」），那是重新设计事件簿的
  数据模型，远超本票「只拆不修」的范围。
- 保留使该纪律一旦被破坏就当场失败，而不是静默滑过——静默滑过会让「该边从未发生」与
  「该边发生过但没人看见」不可区分（原 doc 的这条论证仍然成立，故逐字保留）。
- **推导链第 2 条已更新**：从「靠 `reject_unobservable_invalidation` 挡下」改为
  「`observation.state: ObservedState` + `From<ObservedState> for CandidateState` 的像不含
  `Invalidated` ⟹ 编译期保证」，并注明旧锁已退役。

### 测试侧连带处置

两个 `should_panic` 测试（`observed_invalidation_at_birth_is_rejected_by_the_book_entry_lock` /
`observed_invalidation_on_live_candidate_is_rejected_by_the_same_lock`）**必须移除**：它们要构造
`observation(.., CandidateState::Invalidated)`，段 3 后编译期就构造不出来（这正是段 3 的目的，
不是回归）。接替它们的是 `observed_state_domain_excludes_invalidated_and_lifts_pointwise`：
穷举 match `ObservedState` 的三个变体（**无 `_ =>` 兜底**）+ 断言提升映射逐点正确 + 像不含
`Invalidated` + 观察可达的终态只有 `Confirmed`。若将来给 `ObservedState` 加变体（尤其把
`Invalidated` 加回观察侧），该 match **编译失败**——不是运行期红，是编译期红。

---

## 4. 零行为变化机械证据四件套（#359 口径）

四件套的判定对象是**段 2 拆分**（零行为变化）。段 3 是票面授权的**有意类型收紧**，其证据单列于
§5、§6。以下证据取自「段 2 拆分完成、段 3 未开始」的中间态。

### ① 纯移动（diff 逐 hunk 审无逻辑改）

方法：把原文件与五个新文件各自的**非空行去缩进后排序**，做多重集对比（比逐 hunk 审更强——
它对行的移动完全不敏感，只暴露真正增删的行）。

```
$ git show HEAD:rust/src/theta_v0/classifier/cand_event.rs | grep -vE '^\s*$' | sed 's/^[[:space:]]*//' | sort > old_lines.txt
$ cat cand_event/{mod,key,book,observe,fixtures}.rs | grep -vE '^\s*$' | sed 's/^[[:space:]]*//' | sort > new_lines.txt
$ comm -23 old_lines.txt new_lines.txt      # 仅在旧文件 = 16 行（默认 UTF-8 locale，见下方订正）
$ comm -13 old_lines.txt new_lines.txt      # 仅在新文件（下略，全部为下述三类）
```

> **订正（#634 修复轮，原 MEDIUM-1）**：上面的 `comm -23` 在默认 UTF-8 locale 下对中文行**静默
> 漏配对**（`sort` 的 `LC_COLLATE` 排序与 `comm` 的逐行比较不一致，不报错、不提示），实际漏了 1
> 行。独立复算：`LC_ALL=C comm -23` 与 Python `collections.Counter` 多重集差**均**给出 **17**
> 行（不是 16）；漏掉的 1 行是 A 类第 2 条 doc 重锚（`advance_observation`）因新路径变长触发的
> rustdoc **折行续行**——旧侧该处实际是 2 行，不是 1 行。**教训**：`comm` 不适合含中文的行集
> 对比，机械证据类命令今后一律加 `LC_ALL=C`，或直接改用多重集计数。下表已按 17 行订正。

**旧文件独有 17 行，逐条归类（无一条是逻辑行）**：

- **A 类 — doc intra-doc link 跨文件重锚（4 条，旧侧行数；含 1 处折行续行）**：
  | 原文 | 重锚为 | 原因 |
  |---|---|---|
  | `[`signal::FirstStructuralGates`]`（`StructuralPredicates` doc） | `[`super::super::signal::FirstStructuralGates`]` | `signal` 不在 `key.rs` 作用域（`key.rs` 无需 use 它） |
  | `[`CandidateEventBook::advance_observation`]`（`CandidateProjection` doc，旧侧 1 行 → 新侧因路径变长折成 2 行） | `[`super::book::CandidateEventBook::advance_observation`]` | 目标已移到 `book.rs`；新路径变长触发 rustdoc 折行，旧侧对应行数应计 2 不是 1 |
  | `[`StructuralPredicates::resolved_state`]`（`reject_unobservable_invalidation` doc） | `[`super::key::StructuralPredicates::resolved_state`]` | `book.rs` 只在 doc 提及该类型，use 它会产 unused import warning |
- **B 类 — 模块路径重锚 / import 重组（7 条）**：`use super::decompose::center_trend_gate` →
  `super::super::`；`use super::signal` → `super::super::signal`；`use super::divergence::{..}`
  按 rustfmt 拆多行；`use super::super::types::{Center, Direction, Segment, Side}` 按域分拆
  （`key.rs` 只需 `Side`，`observe.rs` 需 `Center/Direction/Segment`）；
  `use super::super::super::types::Tick` 合进 `fixtures.rs` 的 types import；
  `blocks: &[super::decompose::MoveBlock]` ×2 → `super::super::decompose::MoveBlock`
  （**同一类型**，只是相对路径深度 +1）。
- **C 类 — 夹具函数签名加可见性修饰（6 条）**：`fn center/segment/observation/`
  `deferred_envelope_break_scan/scan_observations/extended_episode_scan` → 前缀 `pub(super)`
  （从「同文件 `mod tests` 私有」变为「`cand_event` 及其后代可见」，因为夹具现被两个域的 tests 共用）。
  可见性放宽仅限 `#[cfg(test)]` 编译单元，生产面零影响。

A4 + B7 + C6 = **17**（原报告 16 是 `comm` 漏计 A 类第 2 条折行续行所致，已订正）。

**新文件独有行同样零逻辑行**：全部为 (a) 五个模块头 doc + `mod.rs` 的 re-export 说明注释、
(b) `mod`/`pub use`/`use` 声明及其 rustfmt 续行、(c) `pub(super) fn` 夹具签名及其 rustfmt 续行、
(d) B 类路径重锚的新形式。

> 过程如实登记：首版 `fixtures.rs` 曾把一处原为反引号的 `deferred_envelope_break_scan`
> 升级成 intra-doc 链接 `[`...`]`。这属超出纯移动的多余改动，行级对比暴露后已改回逐字原文
> （旧独有行数由 20 降至 17；原文写「降至 16」是同一处 `comm` 漏计导致，已订正）。

### ② pub 项原样导出（re-export 面不变、下游零改动）

`cand_event` 是 `pub mod`，外部全部经 `cand_event::X` 路径消费。`mod.rs` 逐项 re-export：

```rust
pub use book::{event_probe, CandidateEventBook};
pub use key::{
    interval_is_degenerate, interval_is_sub, intervals_are_disjoint, intervals_touch,
    CandidateEvent, CandidateKey, CandidateKind, CandidateProjection, CandidateState,
    CandidateStreams, ObservedState, ParentFingerprint, StructuralPredicates,
    CANDIDATE_RULE_VERSION, FNV_OFFSET_BASIS, FNV_PRIME,
};
pub use observe::CandidateObservation;
pub(crate) use observe::observations_for_level;
#[allow(unused_imports)]
pub(crate) use observe::{pan_observations_for_level, structural_observations_for_level};
```

**下游零改动实测**（段 2 拆分完成时）：

```
$ git status --short
 D rust/src/theta_v0/classifier/cand_event.rs
?? rust/src/theta_v0/classifier/cand_event/

$ git diff --stat HEAD -- rust/src/theta_v0/classifier/mod.rs \
    rust/src/theta_v0/classifier/chain_cert/ rust/src/theta_v0/classifier/cand_sub.rs rust/src/bin/
（空输出 = 下游消费点零改动）
```

`#[allow(unused_imports)]` 的理由（已写入 `mod.rs` 注释）：`structural_observations_for_level` /
`pan_observations_for_level` 是 `pub(crate)`，拆分前的 crate 内可见路径是 `cand_event::<名>`，
实测消费者只有同文件 `mod tests`（`cand_event/` 外零引用，grep 在案）。拆分后这两个测试迁到域内
`mod tests` 走域内路径 ⟹ 本层 re-export 无消费者。**仍保留**是为守住「可见面逐字不变」这条验收
条款；缩面属行为变化，不在本票范围。代价是一行 attribute，换来编译 warning 数与基线严格相等（55）。

### ③ 测试名 IDENTICAL（`--list` 前后逐字对拍）

```
$ cargo test --lib -- --list | grep ': test$' | sort | wc -l
基线: 2721        段 2 拆分后: 2721        （总数相等）

$ diff <(sed 's/.*:://' baseline_tests.txt|sort) <(sed 's/.*:://' split_tests.txt|sort)
（空输出 = 全仓 2721 条测试的叶名集合逐字零差异）

$ diff <(grep 'cand_event::tests::' baseline|sed 's/.*tests:://'|sort) \
       <(grep -E 'cand_event::(book|key|observe)::tests::' split|sed 's/.*tests:://'|sort)
（空输出 = cand_event 的 20 条叶名逐字零差异，20 = 20）
```

**口径说明（必须明说的一处偏差）**：完整测试路径中间**插入了域段**——
`cand_event::tests::X` → `cand_event::{book|key|observe}::tests::X`。这是票面「三域各带
`mod tests`」的**定义后果**，不是测试内容变化：Rust 的测试路径由模块路径决定，域内 `mod tests`
必然带域名。故「测试名 IDENTICAL」在本票按**叶名（函数名）集合逐字一致**判定，并附完整路径的
逐条映射（20 条：4 → `key::tests`、13 → `book::tests`、3 → `observe::tests`；分配依据 = 主要被测对象）。
夹具迁入 `fixtures.rs` 后**不产生任何新测试名**（总数 2721 不变，已证）。

### ④ 失败集逐字一致 + 对拍零 diff

```
基线（干净 HEAD）：  test result: ok. 2583 passed; 0 failed; 138 ignored; 0 measured
段 2 拆分后 第 1 跑：test result: FAILED. 2582 passed; 1 failed; 138 ignored
                     failures: theta_v0::classifier::tests::incremental_tower_scaling_dominates_full_synthetic
段 2 拆分后 第 2 跑：test result: ok. 2583 passed; 0 failed; 138 ignored; 0 measured
```

**failed 项性质登记（#619 L9）**：`incremental_tower_scaling_dominates_full_synthetic` 是编排方
点名的 known flaky，本票命中「两跑中一红一绿」，按登记性质处理：

- 该测试断言的是**增量塔 vs 全量塔的耗时比**（性能标度），与 `cand_event` 域无因果关系
  （拆分不改任何算法路径，见证据①）；
- 单测隔离重跑 **3 次全绿**（`0.15s` 各次）：负载敏感型断言在全量并发跑时被挤，非本票引入；
- 段 2 全量重跑绿（2583/0），段 3 后连续多次全量跑均绿（2582/0）。

**行为对拍零 diff**：本票的行为产物对拍由仓内既有金值/等价测试承担，终跑全绿：

| 测试 | 对拍内容 |
|---|---|
| `chain_certificate_book_classify_fnv1a_golden` | 候选事件链证书的 **FNV1a 金值**（字节级产出指纹）→ 候选事件产出未变 |
| `candidate_event_stream_per_segment_full_replay_equals_incremental` | 候选事件流：全量重放 ≡ 增量 |
| `chain_certificate_book_incremental_equals_full_replay` | 链证书簿：增量 ≡ 全量重放 |
| `causal_and_terminal_projection_drives_agree_on_common_chain_keys` | fresh↔causal 双通道载荷比对 |

**豁免声明**：`formal/` fixture 漂移 gate（CLAUDE.md 节拍）不触发；证据：本票改动面
`git status --short` 全在 `rust/src/theta_v0/classifier/` 与 `rust/src/bin/`，`formal/` 零改动
⟹ 豁免 `scripts/check_fixture_drift.py`。

**编译面**：`cargo test --no-run`（全 target：lib + 所有 bin + `rust/tests/` 集成 crate）
**零 error**；`cargo test --lib --no-run` 的 warning 数恒为 **55 = 基线 55**；
`rustfmt --check --edition 2021` 对全部改动文件干净。

---

## 5. 段 3（#612）证据：观察侧 `Invalidated` 编译期不可构造

### 实现

```rust
// key.rs
pub enum ObservedState { Provisional, Unresolved, Confirmed }   // 三态，无 Invalidated
impl From<ObservedState> for CandidateState { /* 唯一提升点，单射 */ }
impl StructuralPredicates { pub fn resolved_state(self) -> ObservedState { .. } }  // 返回类型收紧

// observe.rs
pub struct CandidateObservation { .., pub state: ObservedState, .. }

// book.rs — make_revision
state: observation.state.into(),                                  // 三态 → 四态的唯一提升点
confirmed_at: if observation.state == ObservedState::Confirmed { .. }
// advance() 里的 reject_unobservable_invalidation(observation) 已移除（原位留退役说明注释）
```

### 证据 ①：观察侧 `Invalidated` 引用 grep 零命中

```
$ grep -A5 "pub enum ObservedState" key.rs | grep -c Invalidated
0                                    # 类型定义不含该变体

$ grep -rn "ObservedState::Invalidated" src/ tests/ --include=*.rs | wc -l
0                                    # 全仓零引用（构造不出来）

$ grep -rn "reject_unobservable_invalidation" src/ tests/ --include=*.rs
key.rs:76    ///   只能靠运行期 assert 拦（拆分前的 `book::reject_unobservable_invalidation`）；
book.rs:44   // #612/#634：此处原有 `reject_unobservable_invalidation` 运行期 assert，守 E2E-D5
book.rs:212  ///    `Invalidated`。（拆分前这一步靠运行期 assert `reject_unobservable_invalidation` 保证，
                                     # 3 处全为注释里的历史说明，函数体已移除
```

**全仓 `CandidateObservation.state` 的赋值点清单（8 处，全部三态）**：

```
src/bin/p123_fast_replay.rs:4322              state: ObservedState::Provisional,
src/theta_v0/classifier/mod.rs:3084           state: cand_event::ObservedState::Provisional,
src/theta_v0/classifier/mod.rs:4647           state: cand_event::ObservedState::Provisional,
src/theta_v0/classifier/cand_sub.rs:423       state: ObservedState::Provisional,
src/theta_v0/classifier/chain_cert/tests.rs:47   state: ObservedState::Provisional,
src/theta_v0/classifier/chain_cert/tests.rs:60/275/511/806   .state = ObservedState::Confirmed;
src/theta_v0/classifier/cand_event/observe.rs:257  state: ObservedState::Confirmed,   （Pan 域投影）
＋ observe.rs 的 trend_observation / merged 与 fixtures.rs 走 `resolved_state()` 派生值（同为三态）
```

**未改的同名点（辨明依据，防误改）**：`cand_sub.rs:309` 与 `chain_cert/tests.rs:986` 构造的是
**`CandidateEvent`**（带 `observed_at` / `invalidated_at` 字段），事件侧必须留四态 `CandidateState`；
`chain_cert/tests.rs:350/526/811` 读的是 `ChainNode.state: Option<CandidateState>`（事件侧读数，
其中 `:350` 断言 `Some(CandidateState::Invalidated)` 是链节点证伪留痕的正当读取）。

### 证据 ②：负控实验（编译期不可构造的直接实测）

临时在 `book.rs` 的 tests 里加一行试图构造携带 `Invalidated` 的观察，`cargo test --lib --no-run`：

```
error[E0599]: no variant, associated function, or constant named `Invalidated`
              found for enum `key::ObservedState` in the current scope
   --> src/theta_v0/classifier/cand_event/book.rs:433:54
    |
433 |         let _ = observation((30, 35), ObservedState::Invalidated);
    |                                                      ^^^^^^^^^^^ variant, associated function,
    |                                                                  or constant not found in `key::ObservedState`
error: could not compile `newchan_rust` (lib test) due to 1 previous error
```

实验后已**撤回**（`grep -c negative_control_temporary_probe → 0`，撤回后 `cargo test --lib`
2582/0 复现）。这条负控把「编译期不可构造」从声明变成实测事实——正是拆分前那两个
`should_panic` 测试所测行为的编译期对偶。

### 证据 ③：常驻编译期锁

`book.rs::tests::observed_state_domain_excludes_invalidated_and_lifts_pointwise`：穷举 match
`ObservedState` 三变体（无 `_ =>` 兜底）+ 逐点验证 `From` 提升 + 断言像不含 `Invalidated`
+ 断言观察可达终态只有 `Confirmed`。加变体即编译失败。

---

## 6. 指纹对照（基线 → 终跑）

| 阶段 | passed | failed | ignored | `--list` 条数 | warnings | 各文件 ≤800 |
|---|---|---|---|---|---|---|
| 基线（干净 `7f0bad3607`） | 2583 | 0 | 138 | 2721 | 55 | — (1417 行单文件) |
| 段 2 拆分后（跑 1） | 2582 | 1 ⚠ | 138 | 2721 | 55 | ✓ |
| 段 2 拆分后（跑 2） | **2583** | **0** | 138 | 2721 | 55 | ✓ |
| 段 3 后（终跑） | **2582** | **0** | 138 | 2720 | 55 | ✓ 最大 653 |

⚠ = known flaky `incremental_tower_scaling_dominates_full_synthetic`，性质登记见 §4④。

**段 2 指纹**：`2583 = 2583` **严格相等**，测试名叶集合逐字一致 → 零行为变化成立。

**段 3 差额逐条对账（−1）**：

```
$ diff <(sed 's/.*:://' 段2清单|sort) <(sed 's/.*:://' 终态清单|sort)
< observed_invalidation_at_birth_is_rejected_by_the_book_entry_lock: test          （移除）
< observed_invalidation_on_live_candidate_is_rejected_by_the_same_lock: test       （移除）
> observed_state_domain_excludes_invalidated_and_lifts_pointwise: test             （新增）
```

2583 − 2 + 1 = 2582 ✓。三条的逐条理由见 §3「测试侧连带处置」——移除的两条测的是**运行期锁**，
其守的边已编译期不可表达；新增的一条测**编译期性质**本身。**这是段 3 的目的达成，不是覆盖回退**：
被移除的两条所测的行为，现由「§5 证据②负控实验（一次性实测）+ 证据③穷举 match（常驻锁）」共同承担，
且从运行期前移到了编译期。

**终态文件行数**：`book.rs` 653 / `observe.rs` 400 / `key.rs` 326 / `fixtures.rs` 136 /
`mod.rs` 47 = 1562（原 1417 + 145：模块头 doc、imports、re-export、`ObservedState` 类型及其文档、
新测试）。

**改动文件清单（10 个，全在 worktree 未提交面）**：

```
删除  rust/src/theta_v0/classifier/cand_event.rs                    (-1417)
新增  rust/src/theta_v0/classifier/cand_event/mod.rs                (+47)
新增  rust/src/theta_v0/classifier/cand_event/key.rs                (+326)
新增  rust/src/theta_v0/classifier/cand_event/book.rs               (+653)
新增  rust/src/theta_v0/classifier/cand_event/observe.rs            (+400)
新增  rust/src/theta_v0/classifier/cand_event/fixtures.rs           (+136)
修改  rust/src/theta_v0/classifier/mod.rs                           (段 3：2 行观察构造点)
修改  rust/src/theta_v0/classifier/cand_sub.rs                      (段 3：1 行 + import)
修改  rust/src/theta_v0/classifier/chain_cert/tests.rs              (段 3：5 行 + import)
修改  rust/src/bin/p123_fast_replay.rs                              (段 3：1 行 + import)
新增  chanlun/review-results/issue634-implementation-20260729.md    (本报告)
```

后四个文件的改动**全部属段 3**（观察构造点随类型分离落地，票面明写「构造点改动随拆分落地」）；
段 2 拆分阶段这四个文件**零改动**（§4② 实测在案）。

---

## 7. Standards 自查

按 `docs/agents/delivery-discipline.md` 与 `docs/agents/generation-constitution.md`：

- **关票门五子句**：本票**不关票**（编排方统一提交），故第 1/2 子句（resolution 引用 commit hash、
  已并入 main）当前**不适用**——改动留 worktree 未提交面是编排方指定的交付形态。第 3 子句
  （声明在此刻均为真）：本报告每条数字均由本 session 实测命令产出，无预写结论。第 4 子句
  （逐 commit 可编译）：由编排方提交时声明；本票交付面 `cargo test --no-run` 全 target 零 error，
  可编译性已备证。第 5 子句（工位清理）：worktree `/private/tmp/wt-634` 有在制品未合入，
  **不得删**，待编排方处置。
- **对照型 AC 的对照物**：票面「四件套」对照物 = 干净基线 `7f0bad3607` 的实测读数
  （2583/0/138、2721 条测试名、55 warnings），已在 §6 表格钉住，可独立复核。
- **豁免声明**：①`formal/` fixture 漂移 gate 豁免（证据见 §4④ 末）；②「测试名 IDENTICAL」按叶名
  集合判定而非完整路径，偏差原因 + 逐条映射见 §4③ —— 这是**降级说明而非豁免**：完整路径的
  差异被逐条列举并归因到票面指定的域段插入。
- **编号纪律**：本报告中 `#634`/`#612`/`#576`/`#614`/`#551`/`#550`/`#529`/`#359`/`#619`/`#246`/`#535`
  均指 GitHub Issue；`E2E-D5`/`E2E-O`/`037:20` 为 SPEC 内部标签，非票号。
- **多票同提交降级**：不触发（本票交付单一课题；#612 是本票的段 3，票面明写「交付随本票」，
  且其证据在 §5 独立成节，可脱离段 2 单独验证）。
- **名分四态**：拆分属「现役」延续，不产新名分。`ObservedState` 是既有 `CandidateState` 的三态
  子域类型（`From` 提升单射），不引入新概念、不改任何判据语义。
- **只拆不修**：段 2 严格纯移动（§4① 行级证据）。段 3 只做票面明写的类型分离，未顺手改任何
  既有逻辑。过程中发现的既有缺陷（如全仓 55 个既有 warning，含 `fill.rs`/`runner.rs` 大批
  unused import）**如实登记不动手**。
- **事故照实申报**：一处过程订正已登记（§4① 末：首版多加了一个 doc intra-doc 链接，
  超出纯移动，行级对比暴露后改回逐字）。

---

## 8. 遗留风险 / 未裁项

1. **`event_probe` 的两处 `unreachable!` 仍是运行期载体**（§3 ②/③）。要把它们也搬到编译期，需给
   **事件侧**也分类型（「非失效事件」 vs 「失效事件」），属事件簿数据模型重设计，超出本票范围。
   建议：若 #612 线还有后续，作为独立票评估；本票不动。
2. **`mod.rs` 的 `#[allow(unused_imports)]`**（§4②）。它保住了「可见面逐字不变」，代价是掩盖了
   「两个 `pub(crate)` re-export 当前零消费者」这个事实（已写在紧邻注释里）。若将来确认这两个
   函数永久无 crate 外消费者，可另开票缩面并删 allow——那是行为变化，须单独验收。
3. **`CandidateProjection` doc 链接指向私有项**（`super::book::CandidateEventBook::advance_observation`
   是私有 method）。这是拆分前就有的状态（原文即链向私有 method），本票只重锚路径未改性质；
   `cargo doc` 可能对此报 `private_intra_doc_links`。未验证（`cargo doc` 未跑，非本票验收项）。
4. **测试路径变化的下游影响**：任何按**完整路径**过滤 `cand_event::tests::` 的外部脚本
   （CI filter、`--exact` 调用、文档里的复制粘贴命令）会失配，须改为
   `cand_event::{book|key|observe}::tests::`。实测 `grep -rn "cand_event::tests" .`（全 worktree，
   除 `.git/` 与本报告）**零命中**，`.github/workflows/` 内也无 `cand_event` filter
   ⟹ 仓内无失配点；仓外（他人脚本/文档/其他 worktree）**未核**。
5. **`chain_cert/tests.rs:350` 的 `Some(CandidateState::Invalidated)` 断言**未受影响（读的是事件侧
   `ChainNode.state`），但它提示：链侧仍以四态 `CandidateState` 为读数口径。段 3 只收紧了**观察侧**
   入口，事件侧与链侧的四态语义一字未动——这是有意为之（失效留痕是链谱系的正当读数），不是漏改。

---

## 9. 修复轮（#634 影子评审 CONDITIONAL PASS 处置对账，2026-07-29）

影子评审 `chanlun/review-results/issue634-shadow-20260729.md` 判 **CONDITIONAL PASS**：代码零
缺陷，两条 MEDIUM 全在证据/报告精度，5 条 LOW 不挡关票。本节逐条对账处置结果。

### MEDIUM-1｜四件套①证据工具口径 —— **已订正（16→17）**

- **处置**：§4① 已就地订正：`comm -23` 在默认 UTF-8 locale 下静默漏计 1 行（A 类第 2 条
  `advance_observation` doc 重锚因路径变长触发的 rustdoc 折行续行），报告原写「旧文件独有 16
  行」现订正为 **17**（A4 + B7 + C6 = 17），并补登工具缺陷说明。
- **本轮独立复算**（不采信影子数字，重新对拍）：用重建的段 2 中间态面（见 MEDIUM-2）与
  `git show HEAD` 基线单文件独立跑一遍：

  ```
  默认 locale comm -23（仅旧）    16
  LC_ALL=C comm -23（仅旧）      17
  Python Counter 多重集差（仅旧） 17
  ```

  与影子评审的判定**逐字复现**（17，非 95——95 是影子评审对**终态**（含段 3）做的对比口径，
  本节延续原报告 §4① 的方法学，对比对象是**段 2 中间态**，两个口径不是同一分母，均已在报告内
  分别标注清楚）。
- **纪律订正**：以后行级证据类命令一律加 `LC_ALL=C`，或直接用 Python 多重集计数替代 `comm`——
  `comm` 不适合含中文的行集对比。

### MEDIUM-2｜段 2 中间态重建 —— **已交付**

- **交付位置**：`/private/tmp/wt-634-seg2-state/cand_event/{mod,key,book,observe,fixtures}.rs`
  ——段 2 完成态（纯移动拆分完成、段 3 未开始）的文件面，从当前终态**手工回退段 3 全部改动**
  重建：`ObservedState` 类型定义与 `impl From<ObservedState> for CandidateState` 移除；
  `StructuralPredicates::resolved_state()` 返回类型 `ObservedState → CandidateState`；
  `CandidateObservation.state` 字段类型 `ObservedState → CandidateState`（`observe.rs`、
  `fixtures.rs` 的 `observation()` 夹具同步）；`book.rs` 恢复 `reject_unobservable_invalidation()`
  函数体与 `advance()` 入口调用点、恢复 2 个 `should_panic` 测试
  （`observed_invalidation_at_birth_is_rejected_by_the_book_entry_lock` /
  `observed_invalidation_on_live_candidate_is_rejected_by_the_same_lock`）、移除
  `observed_state_domain_excludes_invalidated_and_lifts_pointwise`、`event_probe::on_append`
  与 `reject_unobservable_invalidation` 的 doc 推导链回退为基线原文（cross-file 路径按段 2
  的跨文件重锚规则调整，如 `[`StructuralPredicates::resolved_state`]` → 段 2 面同样需要
  `[`super::key::StructuralPredicates::resolved_state`]`，因为该函数本就分属不同文件）。
- **重建方法**：对照 `git show HEAD:rust/src/theta_v0/classifier/cand_event.rs`（基线单文件
  1417 行）逐段核对，凡段 3 引入的类型/函数/测试改动一律回退到基线原文，凡段 2 的跨文件路径
  重锚（A/B/C 三类，见 §4①）保留。
- **验证方法**（全程不动 git，仅用 `cp` / `git show`(只读) 做文件级临时换入换出）：
  1. 备份终态面（`cand_event/` 目录 + 4 个下游文件）到 `/tmp` 之外的临时目录；
  2. 把 scratch 面 5 个文件换入本 worktree 的 `cand_event/`，把 4 个下游文件（`classifier/mod.rs`
     `cand_sub.rs` `chain_cert/tests.rs` `bin/p123_fast_replay.rs`）用 `git show HEAD:<path>` 换回
     基线内容（这 4 个文件在段 2 阶段本就零改动，等同基线）；
  3. `cd rust && cargo test --lib --no-run`：**0 error**，55 warnings（与基线相符，cand_event
     域命中 0 条新增）；
  4. `cargo test --lib`：**2583 passed; 0 failed; 138 ignored**（与基线严格相等）；
  5. `cargo test --lib -- --list`：**2721** 条，与另行独立复现的**真基线**（临时把 `cand_event/`
     换成 `git show HEAD` 的单文件 `cand_event.rs` 重新跑一遍 `--list`）逐字对拍：总数 2721=2721，
     叶名（函数名）集合 **diff 为空** ⟹ 逐字一致；唯一差异是 20 条完整路径的域段插入
     （`cand_event::tests::X` → `cand_event::{book|key|observe}::tests::X`，域拆分的必然后果，
     非测试内容变化）；
  6. 全部换入操作用步骤 1 的备份逐文件 `diff` 校验**逐字还原**，再在终态面复跑
     `cargo test --lib` 确认仍为 **2582/0/138**（见 §10 指纹）。
- **结论**：段 2「零行为变化」这条声明现由 scratch 面的独立可执行复现支撑，不再仅是报告断言。
- **给编排方的提交顺序说明**：
  1. **commit 1（段 2 纯移动）**：把 `/private/tmp/wt-634-seg2-state/cand_event/*.rs` 换入
     worktree 的 `rust/src/theta_v0/classifier/cand_event/`，删除单文件 `cand_event.rs`
     （若尚存在于该提交前的树上），**不改动**下游 4 个文件（它们在段 2 阶段零改动，保持基线内容）。
     该 commit 的 `git diff <parent> <commit1>` 即为四件套①②③的可机器复核对照物。
  2. **commit 2（段 3 类型分离）**：在 commit 1 基础上应用本报告 §5 的全部改动——`key.rs` 加回
     `ObservedState` 类型与 `From` 实现、`resolved_state()` 收紧返回类型、`observe.rs`/`fixtures.rs`
     的 `state` 字段类型收紧、`book.rs` 移除 `reject_unobservable_invalidation` 与 2 个
     `should_panic`、新增编译期锁测试，以及 4 个下游文件的构造点换名。该 commit 即为当前 worktree
     的终态（含本轮修复轮的 LOW-2/LOW-3 调整）。
  3. 两个 commit 分别可独立 `cargo test --lib` 验证（commit 1 → 2583/0/138；commit 2 → 见 §10）。

### LOW-1｜book.rs 测试数表格口径 —— **已订正**

- §2 表格已订正：`book.rs` 行由「13」改为终态实测值（本轮修复轮应用 LOW-2 后为 **11**；
  LOW-2 应用前的终态值为 **12**；**13 是段 2 中间态数字**，三个数字已在表格与本节分别标注，
  不再混用同一单元格。

### LOW-2｜编译期锁测试域归属 —— **已挪至 `key::tests`**

- **处置**：`observed_state_domain_excludes_invalidated_and_lifts_pointwise` 已从
  `cand_event/book.rs::tests` 移至 `cand_event/key.rs::tests`（该测试只触及
  `ObservedState`/`CandidateState`/`From`/`is_terminal()`，全部定义在 `key.rs`，零 `book` 域
  依赖——不建 `CandidateEventBook`、不调 `advance`）。测试体逐字未动，仅新增一段说明落位理由
  的注释（引用 #634 影子评审 LOW-2）。
- **连带影响**：`book.rs` 616 行（原 653，扣除移出的测试 + §2 表格订正后的净变化）、11 个测试
  （原 12）；`key.rs` 370 行（原 326）、5 个测试（原 4）。完整路径随之变为
  `cand_event::key::tests::observed_state_domain_excludes_invalidated_and_lifts_pointwise`
  （原 `cand_event::book::tests::...`）——这是移动测试落位的直接后果，`--list` 总数不受影响
  （仍 2582，见 §10）。

### LOW-3｜`book.rs:48` 注释补充作用域 —— **已补**

- **处置**：`book.rs` 第 44-50 行注释追加一句：「（二者的唯一调用点
  `event_probe::on_append(..)` 带 `#[cfg(test)]`——只在 test build 下被触发，生产二进制不执行；
  #634 影子评审 LOW-3 实测在案）」，消除「生产 build 里仍有一道运行期防御」的误读。

### LOW-4｜`cargo doc` 既有 warning —— **登记（拆分前既有，非本票新增）**

- **处置**：本节确认影子评审的补跑结果——`cargo doc --no-deps --lib` 对 cand_event 域命中 6 处
  `private_intra_doc_links` warning（`book.rs:207/208/214/217`、`key.rs:104/222`），性质与拆分
  前相同（旧文件同样的私有项 doc 链接同样会报，`advance_observation`/`make_revision`/`invalidate`/
  `signal::FirstStructuralGates` 均无 `pub`）。全仓既有 542 条 doc warning。`cargo doc` 不在票面
  验收项内，本票 warning 口径（`cargo test --lib` 的 55）实测相等，见 §10。**不改动代码**——这是
  仓库既有状态的登记，不是本票范围内的缺陷。

### LOW-5｜仓外测试路径引用 —— **照实登记，无法核实**

- **处置**：仓内 `grep -rn "cand_event::tests"`（排除 `.git/` 与 `review-results/`）零命中，
  `.github/workflows/` 内无 `cand_event` filter ⟹ 仓内无失配点。仓外（他人脚本/文档/其他
  worktree 按完整路径 `cand_event::{book|key|observe}::tests::` 过滤或复制粘贴旧路径
  `cand_event::tests::` 的调用）**在只读评审范围内无法核实**，如实登记为开口风险，不挡关票。

---

## 10. 修复轮后指纹对照

**终态面**（`/private/tmp/wt-634`，含本轮 LOW-2/LOW-3 改动，git 未提交）：

```
cargo test --lib --no-run   0 error / 55 warnings（cand_event 域命中 0 条新增）
cargo test --lib            test result: ok. 2582 passed; 0 failed; 138 ignored; 0 measured
cargo test --lib -- --list  cand_event 域 19 条（book 11 / key 5 / observe 3）
rustfmt --check             cand_event/{mod,key,book,observe,fixtures}.rs 全部 exit 0
文件行数                     book 616 / observe 400 / key 370 / fixtures 136 / mod 47（全 ≤800）
```

**对账**：本轮只挪了 1 个测试的域归属（LOW-2）+ 补了 1 段注释（LOW-3），**未新增/删除任何测试**、
未改任何生产逻辑 ⟹ `2582 passed / 0 failed / 138 ignored` 与修复轮前逐字相等，`--list` 总数
2720（19 条 cand_event 测试，域内分布由 12/4/3 变为 11/5/3，总数不变）。

**scratch 面**（`/private/tmp/wt-634-seg2-state/cand_event/`，段 2 中间态重建，非 git 仓）：

```
cargo test --lib --no-run   0 error / 55 warnings
cargo test --lib            test result: ok. 2583 passed; 0 failed; 138 ignored; 0 measured
cargo test --lib -- --list  2721 条，与独立复现的真基线（HEAD 单文件）叶名集合逐字一致（diff 为空）
```

**改动文件清单（本轮修复轮，均在 worktree 未提交面，未新增文件）**：

```
修改  rust/src/theta_v0/classifier/cand_event/book.rs      （LOW-2 移出 1 测试；LOW-3 补注释）
修改  rust/src/theta_v0/classifier/cand_event/key.rs        （LOW-2 移入 1 测试 + 落位说明注释）
修改  chanlun/review-results/issue634-implementation-20260729.md （MEDIUM-1/LOW-1 数字订正 + 本节）
新增  /private/tmp/wt-634-seg2-state/cand_event/{mod,key,book,observe,fixtures}.rs （MEDIUM-2 交付，worktree 外，非 git 追踪）
```
