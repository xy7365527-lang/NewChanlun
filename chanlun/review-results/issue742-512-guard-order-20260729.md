# Issue #742 — `#183` vs `#512` 防线次序评估（#714 附带发现收口）

工位：`/private/tmp/wt-742`（分支 `ticket-742`）。背景面：`chanlun/review-results/
issue714-642-fixbatch-20260729.md` MED-1 节（附带发现，本票不修）+ `chanlun/review-results/
shadow-642-review-20260729.md` MED-1 节（原始质询）。

代码锚：`rust/src/theta_v0/strategy/coverage/step.rs`——`#183` 硬门 461-471 行
（`debug_assert!`，检查 `raw` 内 `ElementId` 唯一）；`#512` 硬门 505-535 行
（`panic!`，检查 `next_idx` 内 `ElementId` 唯一，本票已改）。

## 结论摘要（三问）

1. **两道防线不等价、非纯冗余**：检查对象不同（`raw` 全量 vs `next_idx` 选后子集），交集非空、差集均非空，见下方展开。
2. **`#512` panic message 需要订正，已随本票改**：原文「idx {first_idx} 与 idx {second_idx}」暗示两个可能不同的值，但在 main 架构下 `first_idx` 恒等于 `second_idx`——这是结构性事实（见下方证明），不只是「release 恰好折叠成同一 idx」的偶然观测。已改为单值表述 + `debug_assert_eq!` 自证。
3. **release 下 `#183` 不覆盖，现状合理，已订正注释**：`#183` 是纯 `debug_assert!`，`rust/Cargo.toml` 的 `[profile.release]`（113 行起）未设 `debug-assertions = true`，故 release 构建按 Rust 默认行为编译期整体消除 `#183`。`#512` 是 release 下唯一在场的活动集重复 ID 防线，不是冗余——已在 `#512` 上方补注释挑明这层关系，不改代码逻辑。

---

## 问 1 — 两道防线语义关系

### `#183`（`step.rs:461-471`，`debug_assert!`，仅 debug 生效）

检查对象：**`raw`**（held 循环产出的 A_t 段 `raw[..a_t_end]` ∪ open 循环产出的 B_x 段
`raw[a_t_end..]`，即本 bar 因果树全量物化元素集合）。断言：`raw` 内 `ElementId` 两两不同。

拦截的病态构造：`#216` 三处注册路径（held 对位/Stale 重注册/LiveDetached registry 恢复/
open 候选注入）中任意一处闭合失效，导致同一 `ElementId` 在 `raw` 内产生两个不同 `work` 索引
——不论这两个索引之后是否存活进 `next_legs`。

### `#512`（`step.rs:505-535` 本票改动后，`panic!`，release/debug 均生效）

检查对象：**`next_idx`**（`step_active_set_with_subtree_close` 产出的 `next_legs`，经
`raw_id_idx`——`raw` 直建的 `id→idx` `HashMap`——按 id 逐一查表映射回 `work` 索引后的结果）。
断言：`next_idx` 内 `ElementId` 两两不同。

拦截的病态构造：`next_legs` 中出现同 `ElementId` 的两个条目**且两者都活过**
`step_active_set_with_subtree_close` 内部的子树清仓过滤与 AncOK 链检查。

### 交集/差集（读 `exit.rs:320-358::step_active_set_with_subtree_close` 实现坐实）

`step_active_set_with_subtree_close` 内部对 `(A_t∖𝒟_x^†)∪B_x` 的合并**已经用
`raw_ids: HashSet<ElementId>` 对 `opened`（B_x 段）做了 id 去重**（`exit.rs:330-335`，
`raw_ids.insert(b.id)` 为 false 即跳过）——即 B_x 段内部的重复、或 B_x 与 A_t 存量的重复，
在这一步会被**静默折叠**，不会产生 panic，也不会传导进 `next_legs`。反观 `active`（A_t 段）
本身只做 `closed_ids` 过滤（`exit.rs:329`），**不去重**——若 `raw[..a_t_end]` 本身含重复 id
且两份拷贝都未被子树清仓命中、都通过 AncOK 链检查，两份会原样进入 `next_legs`。

由此：

- **交集**：`raw` 的 **A_t 段**内部重复、且两份拷贝都存活过滤——debug 下 `#183` 先炸（检查
  `raw` 整体，不看是否存活）；release 下 `#183` 不在场，`#512` 接手（检查 `next_idx`，此时
  两份拷贝已存活到这一步）。这是 issue714 报告 MED-1 节实测构造的那类场景。
- **`#183` 独有（差集）**：`raw` 的 **B_x 段**重复，或 A_t 段重复但至少一份被子树清仓/AncOK
  过滤掉——`#183` 检查 `raw` 全量，命中不问是否存活；`#512` 检查 `next_idx`，看不到已被
  `step_active_set_with_subtree_close` 自身去重或过滤掉的那一份，永远不会为这类构造触发。
  即：**release 构建下，B_x 段的 raw 级重复不会被任何硬门捕获**——会被 `exit.rs:330-335`
  的 `raw_ids` 静默折叠，不 panic（这类重复目前由 open 循环自身的 `#216` 判重逻辑
  `step.rs:320-350` 在更上游预防，不依赖 `#183`/`#512`；但若该判重逻辑本身有漏洞，release
  下确实无本票讨论的两道防线兜底，这是本次评估发现的一个残余差集，如实登记，本票不改
  ——不在票面三问的修复范围内，建议后续另评估是否需要给 open 循环的 `#216` 判重逻辑单独
  补覆盖测试）。
- **`#512` 独有（差集）**：无。`next_legs` 的重复只能来自 A_t 段的原始重复（见上），B_x 段
  的重复已被 `exit.rs` 自身逻辑消灭在到达 `next_legs` 之前——`#512` 的检测域是 `#183` 检测域
  （A_t 子集）的严格子集，不存在「`#183` 测不到但 `#512` 能测到」的独立病态构造。

**结论**：两道防线不是同一构造的两次检查，`#512` 也不是 `#183` 的简单 release 版重复——
`#183` 检测域更宽（全量 `raw`，含 B_x 段，不问存活），`#512` 检测域更窄（仅 A_t 段重复且
存活进 `next_legs` 的部分），二者是部分重叠、非包含、非互斥的关系。

## 问 2 — `#512` panic message 訂正

原文 `"idx {first_idx}({}) 与 idx {second_idx}({})"` 隐含「两个可能不同的 idx 值」。结构性
证明其恒等：`next_idx` 由 `next_legs.iter().map(|l| *raw_id_idx.get(&l.id)...)`（`step.rs:480-487`）
逐条查表构造，`raw_id_idx: HashMap<ElementId, usize>` 对同一个 key（`ElementId`）的 `get()`
结果对任意调用位置恒相同——因此凡是两个 `next_idx` 条目的 `ElementId` 相等，二者映射出的
`idx` 值**必然相等**，这一性质与 build profile 无关（不是「release 恰好如此」，是 `raw_id_idx`
这一查表结构本身决定的，恒成立）。

已改（`step.rs:511-535`）：
1. 注释内「双 idx」订正为「idx」（单值）；
2. panic 前补 `debug_assert_eq!(first_idx, second_idx, ...)`，把本次订正的断言变为可执行的
   自证——若此断言未来因代码变动而失败，即说明 `raw_id_idx` 单值查表前提已被破坏，是比
   panic message 措辞更早、更精确的信号；
3. panic message 改为「next_legs 两次选入同一 idx {second_idx}({})…非「两个不同 idx」」，
   同时保留测试断言依赖的两个子串（`"重复 ElementId {id:?}"` 前缀、`source` 返回值文本）
   不变——`step_tests.rs::duplicate_active_id_panics_with_id_indices_and_sources_in_release`
   零改动通过（debug/release 双跑复核，见下方测试记录）。

## 问 3 — release 下 `#183` 是否同样覆盖

`#183`（`step.rs:464-471`）是纯 `debug_assert!`，Rust 默认行为：release profile 除非显式
`debug-assertions = true` 否则整体编译期消除断言体。核对 `rust/Cargo.toml:113` 起的
`[profile.release]`：仅设 `opt-level = 3` / `lto = false`，**未设 `debug-assertions`**，即
release 构建按默认值（`false`）编译，`#183` 不生效。

已用两次独立跑批坐实（同一构造，`duplicate_active_id_panics_with_id_indices_and_sources_in_release`）：
- `cargo test --lib`（debug）：panic 命中 `step.rs:464`（`#183`），message 为「raw 内
  ElementId 唯一性破裂…」；
- `cargo test --release --lib`（release）：panic 命中 `step.rs:533`（`#512`，本票改动后行号），
  message 为订正后的「next_active 重复 ElementId…两次选入同一 idx…」。

**故按票面预设的判定分支**：`#183` 仅 debug、release 下 `#512` 是唯一防线——现状合理，
只需订正注释使其准确反映这层关系（已在 `#512` 上方补注释，指向本报告），不需要结构调整。

---

## 测试记录

| | passed | failed | ignored |
|---|---|---|---|
| 基线（本票改动前，`git stash` 复核） | 2587 | 0 | 138 |
| 终态（本票改动后） | 2587 | 0 | 138 |

零新增红（本票只改注释与 panic message 文案 + 新增一行 `debug_assert_eq!`，无新增 `#[test]`）。
另单跑 `duplicate_active_id_panics_with_id_indices_and_sources_in_release`：debug 命中 `#183`、
release 命中订正后的 `#512`，两分支均 `ok`。

## 停手项

问 1 差集小节登记的「open 循环 `#216` 判重逻辑若失效、release 下 B_x 段 raw 级重复无兜底」
残余风险面，不在本票三问范围内，本票不改代码，如实上报，建议另开票评估是否需要补覆盖测试。
