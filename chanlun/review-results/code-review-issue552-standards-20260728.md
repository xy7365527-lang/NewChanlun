# #552 实装收尾 · Standards 轴评审

- 工作面：`/tmp/wt-552`（`ticket-552`），fixed point `8096600a35`，3 commits，+822 行
- 标准源：`.claude/rules/common/*` 全部 + Fowler 基线臭味（判断题；工具已强制项跳过；体量类只评增量方向）
- 结论：**PASS**（0 HIGH / 3 MED / 3 LOW）

## (a) 违反文档标准处

**MED-1 · 无守卫的 usize 减法** — `rust/src/theta_v0/classifier/cand_sub.rs:82`
`self.blocked - self.reflexive_blocked`。`SameLevelBlock` 全字段 `pub`、无构造函数守住 `blocked >= reflexive_blocked` 不变量，外部可自行构造违例值 → debug 崩、release 回绕成天文数字并被 bin 直接打印。
`coding-style.md`「Input Validation … Fail fast with clear error messages」「Error Handling: Handle errors explicitly at every level」。改 `saturating_sub`，或字段私有化 + 构造器守不变量。

**MED-2 · 声明与实现不一致（单一来源被绕过）** — 模块文档 `cand_sub.rs:11-12` vs 实现 `:169`、`:175`
文档写「本模块的区间不等式**唯一**委托上游单一来源 `interval_is_sub`，不在此重写第二套不等式」，但 `touching`（`child.interval.0 == parent.interval.0 || …`）与 `disjoint`（`child.interval.1 < parent.interval.0 || …`）两条区间不等式就地手写，且不带 `interval_is_sub`（`cand_event.rs:146`）的退化守卫 `child.0 <= child.1`。
后果：退化区间 `(20,10)` 计入 `pairs`，却既不进 `contained` 也不进 `disjoint` —— 计数桶不划分，静默丢格。属 `coding-style.md` 注释失真 + Fowler **Duplicated Code**。修法：把 `touches` / `disjoint` 上提到 `cand_event`，与 `interval_is_sub` 同源。

**MED-3 · Primitive Obsession（增量方向恶化）** — `cand_sub.rs:40-60`、`:151-186`
C 段区间全程为裸 `(usize, usize)`，本 diff 新增约 10 处 `.0/.1` 端点直取。这正是 MED-2 的成因。建议 newtype `CSpan { start, end }` 携 `contains/touches/disjoint`，退化守卫收进构造器。

## (b) 基线臭味

- **LOW-1 · 无谓深拷贝** `cand_sub.rs:139` `event.clone()` 逐事件深拷入 `BTreeMap`，与 `CandidateStreams` 的 Rc 设计意图（`cand_event.rs:141`「不深拷贝全簿」）相悖。
- **LOW-2 · 二次复杂度无上界声明** `count_pairs`/`count_same_level` 为 O(n²)（500k 窗实测 `same_level_pairs=1,593,865`）。仅诊断 bin，但代码与文档均未声明该扫描随窗口平方增长。
- **LOW-3 · Data Class** `AdjacentLevelContainment`（`:40`）全 pub 字段零行为；`ContainmentScan` 已有 `total_*` 可作归置位。

## 判为非问题

零消费接线 = SPEC US12/US13 强制，不判 Speculative Generality；`let mut` 局部累加器不触 `coding-style.md` Immutability（局部量非共享对象）；455 行 < 800、各函数 < 50 行、嵌套 ≤ 4 层；7 条单测覆盖双轴真值表 / 退化 / 扫描 / revision 去重，符 `testing.md`；无硬编码密钥、无外部输入面，`security.md` 不适用。
