# #604 活窗右端判据单一权威化（claude sonnet 实施车）

> 日期：2026-07-28；性质：实装收口报告；票面 = #448 S-H1（活窗右端判据 `as_of.max(c_start)` 四处独立复刻收敛为账本侧单一权威）

## 1. 四处判据点定位（票面行号已漂，本轮实测重定位）

`grep -rn "as_of\.max(" rust/src/`（收敛前）命中恰好四处，形状逐字相同 `as_of.max(c_start_变量)`：

| # | 文件:行（收敛前） | 所在函数 | c_start 来源 |
|---|---|---|---|
| 1 | `rust/src/theta_v0/classifier/nest_lifecycle.rs:1637` | `provide_pan_live_windows`（confirmed 侧产窗；生产参照实装/p409 探针口径） | `structure.seg_c.0`（新算结构） |
| 2 | `rust/src/theta_v0/classifier/nest_lifecycle.rs:1863` | `provide_active_pan_live_windows`（L1 active C frontier；生产 #527 活窗来源） | `structure.seg_c.0`（新算结构） |
| 3 | `rust/src/bin/p123_fast_replay.rs:630` | `LifecycleWindowStem::window_at`（生产回放循环缓存 stem 按当前 as_of 重建窗口） | `self.c_start`（缓存字段，跨 bar 复用） |
| 4 | `rust/src/bin/p409_pan_live_probe.rs:130` | `WindowStem::window_at`（p409 探针复用 `provide_pan_live_windows` 结构，缓存 stem 重建窗口） | `self.c_start`（缓存字段） |

四处均落在 `PanLiveWindow.seg_c_live: (usize, usize)` 元组构造点上——`seg_c_live` 是该结构体唯一携带活窗右端的字段。

## 2. 语义比对结论：**全同，无分歧**

逐点核对四要素：

- **表达式**：全部逐字 `as_of.max(c_start)`（两处直接用刚算出的 `structure.seg_c.0`，两处用缓存字段 `self.c_start`——但两个缓存字段的值本身就是产窗时刻 `structure.seg_c.0` 的原样搬运，`LifecycleWindowStem::of`/`WindowStem::of` 构造函数逐字 `c_start: window.seg_c_live.0`，无二次加工）。
- **类型/单位**：四处 `c_start` 与 `as_of` 全部是裸 `usize` 源坐标（bar index），无浮点/tick 混入；`PanLiveWindow.seg_c_live: (usize, usize)` 全仓库唯一定义（`nest_lifecycle.rs:627`），四处消费同一类型契约。
- **相等时取舍**：`as_of == c_start` 时 `.max` 两支返回值相同，无歧义分支。
- **空窗/缺失处理**：四处均为直接元组构造，不涉及 `Option`/空窗分支——右端判据本身不做存在性判断，产窗与否由各自调用方的前置守卫（`NoConfirmedCenterBefore`/`FrontierAheadOfClock` 等）决定，与右端算式正交。
- **边界条件（`as_of < c_start`）**：四处对此退化情形的处理完全一致——钳位到 `c_start`（单点区间 `[c_start, c_start]`），非四处中任一处对此有特殊分支或提前 panic。

**结论**：四处是同一判据在四个消费点的独立复刻，无语义分歧，可直接收敛，无需呈裁。

## 3. 权威位置与理由

新函数 `pub fn active_window_right_edge(c_start: usize, as_of: usize) -> usize` 落在 **`rust/src/theta_v0/classifier/nest_lifecycle.rs`**，紧邻 `PanLiveWindow` 结构体定义之后（原 635 行后）。

理由：
- `PanLiveWindow.seg_c_live` 字段本身定义在 nest_lifecycle.rs——判据与其唯一消费的字段类型同一模块，不产生跨模块耦合。
- nest_lifecycle.rs 是账本侧模块（`NestLifecycleBook` 及其观察契约的所在地），两个生产级产窗函数 `provide_pan_live_windows`/`provide_active_pan_live_windows` 本身也在此文件——四处消费点中两处（生产）已经在本模块内，另两处（p123 回放循环、p409 探针）本就 `use ... nest_lifecycle::{...}` 导入 `PanLiveWindow` 等类型，追加一个函数导入零新增耦合面。
- 不选 `types.rs` 等更底层位置：判据语义（"活窗右端随 as_of 前进，卡 §3 设计内行为"）是账本活窗契约的一部分，不是通用数值工具，放在类型层会丢失语境（该函数的文档需要引用 `PanLiveWindow` 的字段语义）。

## 4. TDD 落地

- 先写单测 `issue604_active_window_right_edge_boundaries`（`nest_lifecycle.rs` 测试模块，紧邻 `identity_close_src` 辅助函数后）：三支边界（`as_of > c_start` 前进 / `as_of == c_start` 相等 / `as_of < c_start` 钳位）+ 两端探底（`0,0` / `0,usize::MAX`），共 5 条 assert。
- 再落权威函数定义，四处调用点改为 `active_window_right_edge(c_start, as_of)`。
- 四处消费点改动：
  - `nest_lifecycle.rs:1637`（`provide_pan_live_windows`）、`nest_lifecycle.rs:1863`（`provide_active_pan_live_windows`）：直接调用（同模块内，无需 import）。
  - `p123_fast_replay.rs:172`：import 追加 `active_window_right_edge`；`:630`→`631` 调用点替换。
  - `p409_pan_live_probe.rs:56`：import 追加 `active_window_right_edge`；`:130` 调用点替换。

## 5. 验证证据

### 5.1 指纹基线 vs 收口（`cargo test --release --lib`）

- **基线**（HEAD `e47039bdcb` 原始态，独立 detached worktree `/tmp/issue604-baseline-src` 重编译）= **2081/1/137**（票面写的 2040/1/137 已过期，以本轮实测为准）。
- **收口** = **2082/1/137** = 基线 **+1**（恰为新增的 `issue604_active_window_right_edge_boundaries`）。
- 唯一红：`theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`（#491，在案恒红，基线/收口两侧逐字同一失败）。

### 5.2 `--no-fail-fast` 全量

`cargo test --release --no-fail-fast`：`--lib` 目标 2082/1/137（同上唯一红）；其余全部 integration test 二进制目标 0 failed（逐条 `test result: ok`）。无其他隐藏红。

### 5.3 replay 对拍零 diff

参照 `/tmp/issue573-baseline3/` 取法：基线用独立 detached worktree（`git worktree add --detach /tmp/issue604-baseline-src HEAD`，非 stash/checkout 本仓库，票面禁 git mutation 未违反）重编译 release 三个 bin，取 `/tmp/issue604-baseline/`；收口版在本 worktree 重编译，取 `/tmp/issue604-afterfix/`。

| 面 | 命令要素 | 结果 |
|---|---|---|
| p123 20k dump | `P116_MAX_BARS=20001 P116_DUMP=... p123_fast_replay <btc_1m_full.json>` | **SAME** |
| p123 20k stdout | 同上 | **SAME** |
| p123 100k dump | `P116_MAX_BARS=100001 ...` | **SAME** |
| p123 100k stdout | 同上 | **SAME** |
| p92 20k stdout | `P92_MAX_BARS=20001 p92_nest_replay_postruling <btc_1m_full.json>` | **SAME** |
| p92 100k stdout | `P92_MAX_BARS=100001 ...` | **SAME** |

6 面 `cmp` 全部 **SAME**。stderr 唯一差异 = `prefix_s`（墙钟：20k 0.118→0.134，100k 3.508→3.982），全部计数字段（triggers/reevals/reuses/syncs_*/wm_cross_*/term_*/shadow_*/pan_*）逐字相同——与 #573 先例一致的验收口径（stderr 进度/墙钟不在验收面）。

p409 探针（本票未强制对拍面）因全量 O(n²) 复制重算在 20k bar 规模超时（2 分钟），放弃该项额外验证——票面未要求，不影响核心验证结论（四处消费点中 p409 与 p123 共用同一权威函数与同一算式，p123 侧已零 diff 实证）。

### 5.4 编译警告

基线/收口两侧 `cargo build --release`（三 bin）均产出同一组 37 条 lib 警告 + 1 条 `p409_pan_live_probe.rs` 未用 import 警告（`std::io::Write`，预先存在，与本票无关）。**零新增警告**。

## 6. commit

`refactor(theta): #604 活窗右端判据单一权威化——账本侧单点 + 四处消费（行为零变化）`

改动文件：`rust/src/theta_v0/classifier/nest_lifecycle.rs`（+34/-2）、`rust/src/bin/p123_fast_replay.rs`（+6/-5）、`rust/src/bin/p409_pan_live_probe.rs`（+4/-4）。

## 7. 偏离/存疑

- 无代码逻辑偏离——四处判据比对结论为"全同"，未触发"分歧则呈裁"分支，直接收敛。
- 唯一流程性偏离：p409 探针的额外对拍面（非票面强制）因超时未完成，已在 §5.3 说明放弃理由与影响面评估（不影响核心验证）。
- 基线获取路径与 #573 先例一致：不信任现存 `target/release` 陈旧二进制（mtime 早于当前 HEAD commit 时间），改用独立 detached worktree 重编译，确保基线对应的 HEAD 可追溯（`git worktree add --detach`，非本仓库内 git mutation，未违反票面禁令）。
