# 影子评审：#604 活窗右端判据单一权威化（claude opus 影子车，独立上下文）

> 日期：2026-07-28；性质：影子评审（评审车**未参与** #604 实装）；对象 commit `e520cf7fa4`；定点 `e47039bdcb..e520cf7fa4`
> 工位 `/tmp/kimi-nest-mainline`；全程单线程主上下文（无子代理、无后台任务）；只读复现，**零 git mutation**、零源文件改动

## 0. 结论（先行）

**verdict = PASS-WITH-NOTES**

票面 #604 的两条硬验收——「判据收敛为账本侧单一权威定义，四处改为消费同一权威」与「行为零变化」——**均独立复现成立**。实施报告 §1–§6 的每一项数字（判据点定位、指纹 2081→2082、唯一红 #491、replay 零 diff、零新增警告）我在隔离纯态树上重跑后逐项吻合，未发现夸大或未跑而报的项。

findings 共 **6** 条：**HIGH 0 / MEDIUM 2 / LOW 4**。无一条推翻「行为零变化」结论，无一条要求回滚。最重一条为 MEDIUM-1（新增文档对钳位分支下了未经探针实测的可达性定位断言，与同文件 `nest_lifecycle.rs:1820-1834` 的 #578 判例同型）。

## 1. 复现方法（与实施车的取证路径**不同源**）

实施车取基线用 `git worktree add --detach`（写入共享 `.git/worktrees/`）。本车改用 **`git archive <sha> | tar -x`** 导出纯态树，全程对 `.git` 零写入，且不与工位共享 `CARGO_TARGET_DIR`（避免污染并行车的 19G target）：

| 树 | 来源 | target dir |
|---|---|---|
| `/tmp/shadow604-base` | `git archive e47039bdcb` | `/tmp/shadow604-base-target` |
| `/tmp/shadow604-tree-a` | `git archive e520cf7fa4` | `/tmp/shadow604-target` |
| `/tmp/shadow604-probe` | `git archive e520cf7fa4` + 钳位探针 | `/tmp/shadow604-probe-target` |

**必要性**：工位工作区当时为脏（`?? rust/src/theta_v0/classifier/retrace_ledger/` 与 `M classifier/mod.rs` 属并行 #621 车），直接在工位跑 `--lib` 会把 #621 的测试计入指纹，无法与 2082 对照。

## 2. 逐项复现结果

### 2.1 判据点残余（票面「bin 侧禁再自带判据」）— PASS

```
$ grep -rn "as_of\.max(" rust/src/          # 收口树
nest_lifecycle.rs:637   （权威函数的文档行，非代码）
nest_lifecycle.rs:644   （权威函数体本身）
```
权威定义外 **零命中**。基线树同一命令恰四处命中，与实施报告 §1 表格逐行一致（`p123:630` / `p409:130` / `nest_lifecycle:1637` / `:1863`；票面所写 1348/1547/574 确已漂移）。

**换皮宽搜**：两 bin + 账本内全部 `.max(` 逐条看过，无一为右端判据——
`p123_fast_replay.rs:831,851`（level 计数）、`p123:2952` / `p409:584`（OHLC 价格）、`p409:316`（stem 计数上界）、`nest_lifecycle.rs:2153,3856`（价格包络）。另查 `cmp::max` / `if as_of < …` 型换皮，零命中。

### 2.2 权威单点性 — PASS

唯一定义 `rust/src/theta_v0/classifier/nest_lifecycle.rs:643`，紧邻其唯一消费的字段 `PanLiveWindow.seg_c_live`（`:627`）之后，位于账本侧模块。四个引用点（`nest_lifecycle.rs:1649`、`:1878`、`p123_fast_replay.rs:631`、`p409_pan_live_probe.rs:130`）全部指向它。

### 2.3 构造点完备性（实施报告未做的一项）— PASS

我另查了 `PanLiveWindow` 的**全部**字面构造点，防止「四处之外还有第五处漏网」：生产侧共 5 处，其中 4 处即上述判据点；第 5 处 `nest_lifecycle.rs:2434-2440`（完成事件反推的闪现观察）取 `seg_c_live: event.interval_b`，**不经 as_of/c_start 计算**，正确地不属于本判据的适用域，豁免成立。其余构造点全在 `#[cfg(test)]` 之后（测试模块起于 `:2490`），为测试自造窗口。

### 2.4 语义逐点比对 — PASS

签名 `active_window_right_edge(c_start: usize, as_of: usize) -> as_of.max(c_start)`。四处替换后实参：

| 消费点 | 替换前 | 替换后 | 等价 |
|---|---|---|---|
| `nest_lifecycle.rs:1647` | `as_of.max(structure.seg_c.0)` | `active_window_right_edge(structure.seg_c.0, as_of)` | ✅ 逐字 |
| `nest_lifecycle.rs:1876` | 同上 | 同上 | ✅ 逐字 |
| `p123_fast_replay.rs:631` | `as_of.max(self.c_start)` | `active_window_right_edge(self.c_start, as_of)` | ✅ 逐字 |
| `p409_pan_live_probe.rs:130` | 同上 | 同上 | ✅ 逐字 |

两 bin 的 `self.c_start` 来源已核：`p123:616` 与 `p409:112` 均为 `c_start: window.seg_c_live.0`，产窗时刻 `structure.seg_c.0` 的原样搬运，无二次加工——实施报告 §2 此项属实。附注：`usize::max` 满足交换律，即便实参顺序写反亦无行为差，故参数序不构成风险面。

### 2.5 指纹 — PASS（基线独立复现，非采信报告）

| | passed | failed | ignored |
|---|---|---|---|
| 基线 `e47039bdcb`（本车实测） | **2081** | 1 | 137 |
| 收口 `e520cf7fa4`（本车实测） | **2082** | 1 | 137 |

+1 恰为新增单测。唯一红两侧同一：`theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`（`signal.rs:3418`，#491 在案恒红，左右摘要两侧逐字相同）。

`cargo test --release --no-fail-fast` 全量：**40 个测试目标 `ok` + 1 个 FAILED（即 lib 的 #491）**，无隐藏红——实施报告 §5.2 属实。

### 2.6 replay 对拍 — PASS（三路交叉）

本车**不采信**实施车留下的 `/tmp/issue604-baseline/`，而是自编译双侧 p123 各跑一遍：

| 对拍面 | 结果 |
|---|---|
| 自跑 base vs 自跑 fix，p123 20k **stdout** | **SAME** |
| 自跑 base vs 自跑 fix，p123 20k **dump** | **SAME** |
| 自跑 fix vs 实施车基线 `p123-20k.stdout` | **SAME** |
| 自跑 fix vs 实施车基线 `p123-20k.dump` | **SAME** |

stderr 唯一差异为 `prefix_s` 墙钟；`P123_SPARSE_SUMMARY` 全部计数字段（triggers=565 / reevals=161 / pan_hits=3815 / pan_invalidations=981 …）逐字相同。

**性能补测（实施报告未做，本车追加）**：报告记 20k `0.118→0.134`、100k `3.508→3.982`（两规模同向 +13%），本车首测亦见 `0.124→0.328`——形态上像回退，故交替重复测量 100k 三轮：

```
round1 base=3.543 fix=3.564 / round2 base=3.502 fix=3.462 / round3 base=3.499 fix=3.480
```

base 最小 3.499 vs fix 最小 3.462 —— **无性能回退**，报告与本车首测的偏慢均为并行车竞争造成的墙钟噪声。（`Cargo.toml` `[profile.release]` 为 `lto=false, codegen-units=16`，跨 crate 调用确不内联，但该函数每 bar 每 stem 仅调用一次、量级数千次，理论开销远低于测量分辨率，与实测一致。）

### 2.7 diff 逐 hunk — PASS

`e47039bdcb..e520cf7fa4` 仅 4 文件。代码三文件全部只含：import 追加（rustfmt 重排两 bin 的 use 块）、四处一行替换、权威函数定义新增、单测新增。**无夹带、无顺带修改、无行为改动**。并行面（`p100_cert_bsp_recon.rs`、`classifier/mod.rs`、`retrace_ledger/`、`level_view*.rs`）确未被该 commit 触碰。

## 3. Findings

### MEDIUM-1｜新增文档对钳位分支下了未经实测的可达性定位断言

**位置**：`rust/src/theta_v0/classifier/nest_lifecycle.rs:641`

> 钳位分支（`as_of < c_start`，**仅见于缓存重放路径按陈旧 `as_of` 重建窗口时**）退化为单点区间 …

这条括注是 #604 **新引入**的（收敛前四处均无任何关于该分支来源的文字），它把钳位的可达位置定位到「缓存重放路径」，即 `p123_fast_replay.rs:622 LifecycleWindowStem::window_at` / `p409:125 WindowStem::window_at`。实施报告与 commit 均未给出该定位的任何证据。

**本车实证**：在隔离探针树的权威函数内注入 `eprintln!("SHADOW604_CLAMP_HIT …")`，编译后跑 p123 —— 20k 命中 **0** 次、100k 命中 **0** 次（探针树 stdout 与纯态收口 `cmp` SAME，确认探针不改行为）。而 p123 的 `window_at` 正是该括注点名的那条缓存重放路径。

**静态佐证（钳位在四处均不可达）**：`provide_active_pan_live_windows` 在构造点前有守卫 `if active.end_index > as_of { return FrontierAheadOfClock }`（`:1839`），故 `seg_c.0 ≤ active.end_index ≤ as_of`；`provide_pan_live_windows` 的段取自 `end_index ≤ as_of` 的已完成段；两 bin 的 `window_at(index)` 中 `index` 为单调前进的回放钟，而 `stem.c_start` 来自更早 bar 的产窗。

**公允界定**：「仅见于 X」在逻辑上是排他性断言而非存在性断言，0 命中**不等于**证伪它。本 finding 的实质是：该括注为一条**未验证的可达性定位**，而同一文件 `:1820-1834` 刚刚以完全相同的理由判过同型断言——

> 该数字**从未由任何真做 `!=`/`>` 判别的探针实际测过** …… C2 的「恒可达」断言是未经验证的声明膨胀（090 号语法禁令）。

#604 在该判例的正下方 180 行处又落了一条同型断言。按 `no-patch-mentality.md` 第 5 条（声明膨胀）与 `formalization-validity-domain.md`（认识论等级强制标注），该括注应改为标注等级的表述：钳位分支当前**在 BTC 100k 实测下不可达（L2 否定性结果）**，其存在理由是防御性下界钳位，而非「见于某路径」。

**建议**：删去「仅见于缓存重放路径…」的定位，改写为「四处消费点当前均有前置守卫/单调钟保证 `as_of ≥ c_start`；本钳位为防御性下界，BTC 100k 实测零命中（L2）」。**不建议**删除 `max` 本身——#604 是零行为变化票，改判据超 scope。

### MEDIUM-2｜「单一权威」无回归防线，新增单测为 L0 同义反复

**位置**：`rust/src/theta_v0/classifier/nest_lifecycle.rs:2575-2586`

新增单测 `issue604_active_window_right_edge_boundaries` 的 5 条 assert 全部只作用于函数**自身**（`(69,95)→95`、`(69,69)→69`、`(69,40)→69`、`(0,0)→0`、`(0,MAX)→MAX`）。由于函数体即 `as_of.max(c_start)`，这 5 条等价于验证 `usize::max` 的语义——按 `formalization-validity-domain.md` 的等级表属 **L0（纯代数/定义，信息增量为零）**，无否证能力：它在任何 `max` 实现下都恒绿。

而 #604 真正要守护的不变量是**「PanLiveWindow 的活窗右端只能来自该权威函数」**，这一条**没有任何测试**。具体后果：将来任何人新增第五个 `PanLiveWindow` 构造点并自带 `as_of.max(...)`，`--lib` 2082 全绿、replay 零 diff、CI 全过——判据静默再度分叉，回到 #448 S-H1 的原点。票面验收第 1 条「bin 侧**禁**再自带判据」目前只有人工约定支撑，无机制。

**建议**：仿本仓库已有的守卫型测试（`rust/tests/issue533_p123_byte_guardrail.rs`）追加一条源码守卫：断言 `rust/src/` 下 `as_of.max(` 的命中数恒为 1（权威函数体），命中即红并打出泄漏位置。这才是把「单一权威」从一次性重构变成常驻不变量的那道 gate。

### LOW-1｜函数名 `active_` 与本模块 active/confirmed 术语二分冲突

**位置**：`nest_lifecycle.rs:643`

本模块 `active` 是专有术语，与 `confirmed` 对立：`provide_active_pan_live_windows`（active C frontier 通道）vs `provide_pan_live_windows`（confirmed 侧）；另有 `ActiveWindowFrontier` / `active_l1_window_frontier` 等。而 `active_window_right_edge` 同时服务**两个**通道，其中 `p409_pan_live_probe.rs:25` 的文件头注释明写「本探针只用 `provide_pan_live_windows`（confirmed 侧，**非 active frontier**）」。字段名为 `seg_c_live`、类型名为 `PanLiveWindow`、中文口径为「活窗」，故 `pan_live_right_edge` / `live_window_right_edge` 与既有术语更一致，`active_` 读起来像是把该判据限定在 active 通道。

顺带（**pre-existing，非 #604 引入**）：`p409_pan_live_probe.rs:25` 称「活窗右端 `seg_c_live.1 = as_of`」，与权威 doc 的 `max(c_start, as_of)` 口径不完全一致；判据集中后该注释宜一并对齐到权威函数。

### LOW-2｜实施报告 §6 的行数与 diff 不符

报告 §6 称 `nest_lifecycle.rs（+34/-2）`，`git diff --numstat` 实测为 **`32  2`**。另两项 `p123（+6/-5）`、`p409（+4/-4）` 与 commit 总计 `+133/-11` 均相符。纯笔误，不影响结论，但报告数字应可逐条核对。

### LOW-3｜p409 消费点实际零覆盖（报告已诚实声明，此处仅登记覆盖账）

票面验收要求「四处消费点回归」。实测覆盖账：`nest_lifecycle` 两处由 `--lib` 2082 覆盖，`p123` 一处由 20k/100k dump+stdout 对拍覆盖，**`p409` 一处两头落空**——该 bin 的单测数为 0（本车实测 `p409_pan_live_probe` 目标 `0 passed`），而对拍面因 O(n²) 超时被放弃（报告 §5.3/§7 已如实说明）。该处替换与 p123 逐字同形、参数序本车已静态核对无误，实际风险很低，但严格讲验收覆盖是 3/4 而非 4/4。若要补齐，最小代价是给 `WindowStem::window_at` 加一条与 `p123:2996` 同型的 stem 身份单测。

### LOW-4｜「禁 git mutation」的执行口径（流程，无实际残留）

报告 §7 称 `git worktree add --detach` 属「非本仓库内 git mutation，未违反」。严格讲它写入共享 `.git/worktrees/`。本车实测 `git worktree list` 中已无 `/tmp/issue604-baseline-src`，**无残留、无实际损害**。登记此条只为记先例：`git archive <sha> | tar -x -C <tmpdir>` 可取到同等纯态基线且对 `.git` 零写入（本次评审全程即用此法），建议后续取基线优先用它。

## 4. 结果包六要素

1. **结论**：#604 = PASS-WITH-NOTES。「账本侧单一权威 + 四处消费」与「行为零变化」两条硬验收独立复现成立；6 条 findings 均为文档严格性、回归防线与覆盖账层面，无一影响已落地行为。
2. **定义依据**：票面 #604 验收四条；`PanLiveWindow.seg_c_live` 契约（`nest_lifecycle.rs:617-627`，卡 §3「右端随 as_of 前进」）；`.claude/rules/no-patch-mentality.md` 第 5 条（声明膨胀）；`.claude/rules/formalization-validity-domain.md`（L0–L3 等级强制标注）。
3. **边界条件（结论翻转条件）**：(a) 若存在本车 grep 口径之外的第五个活窗右端计算点（例如经宏生成、或以 `if/else` 而非 `max`/`cmp::max` 书写且不含 `as_of` 标识符），则「单点权威」不成立；本车已按 `as_of.max(` / `cmp::max` / `if as_of </>` / `PanLiveWindow {` 四种口径交叉搜过，未见。(b) 若 p409 探针的活窗路径与 p123 存在本车静态比对未覆盖的语义差，则 2.6 的零 diff 不足以外推到 p409；该风险由 LOW-3 登记。(c) 若钳位分支在 BTC 100k 之外的品种/时段可达，MEDIUM-1 的 L2 否定性结果不外推——本车只测 BTC。
4. **下游推论**：`PanLiveWindow` 活窗右端此后为单点可改语义面——任何要改右端判据（例如 #578 遗留 6 所指的「有洞 frontier 是否应拒绝」裁决）的票，改一处即全通道生效，不必再做四点同步；反之亦然，误改一处即全通道受影响，故 MEDIUM-2 的 gate 价值随之升高。
5. **谱系引用**：本判据的可达性论断谱系直接续接 `nest_lifecycle.rs:1820-1834` 记录的 **#578 判例**（推翻 #559 C2「`>` 分支生产不可达」的未验证断言，援引 090 号声明膨胀禁令）；MEDIUM-1 即该判例在 #604 的同型复发。未见与本重构相关的其他生成态谱系。
6. **影响声明**：本评审**未改动任何源文件、未做任何 git 操作**（含 stash/checkout/worktree/commit），产出仅本文件一份。临时物落 `/tmp/shadow604-*`（三棵 archive 树 + 三个独立 target + 对拍产物），与工位 `rust/target` 完全隔离，未参与并行车的锁竞争面。并行面 `agent-roster-2026-07-21.md`、`p100_cert_bsp_recon.rs`、`classifier/retrace_ledger/`、`level_view*.rs` 及他人草稿全程未触碰。

## 5. 证据索引

| 项 | 命令 / 位置 |
|---|---|
| 残余判据点 | `grep -rn "as_of\.max(" rust/src/` → 2 命中（均在 `nest_lifecycle.rs:637,644`，doc + 函数体） |
| 基线四处 | 同命令于 `/tmp/shadow604-base` → `p123:630` `p409:130` `nest_lifecycle:1637,1863` |
| 构造点完备 | `grep -rn "PanLiveWindow *{" rust/src/`；生产 5 处，第 5 处 `:2434` 取 `event.interval_b` 豁免 |
| 指纹基线 | `/tmp/shadow604-base/rust` + `CARGO_TARGET_DIR=/tmp/shadow604-base-target cargo test --release --lib` → 2081/1/137 |
| 指纹收口 | 同上于 `/tmp/shadow604-tree-a` → 2082/1/137 |
| 全量 | `cargo test --release --no-fail-fast` → 40 目标 ok + 1 FAILED（lib #491） |
| 对拍 | `/tmp/shadow604-{base,fix}-p123-20k.{stdout,dump}` 四路 `cmp` 全 SAME |
| 钳位探针 | `/tmp/shadow604-probe` 注入 `SHADOW604_CLAMP_HIT`；20k/100k 各 `grep -c` = **0** |
| 性能 | 交替 3 轮 100k：base 3.543/3.502/3.499，fix 3.564/3.462/3.480 |
| 行数 | `git diff --numstat e47039bdcb..e520cf7fa4 -- rust/` → `6/5`、`4/4`、`32/2` |
