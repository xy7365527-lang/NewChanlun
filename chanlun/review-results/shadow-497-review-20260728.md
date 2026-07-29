# 影子评审：#497 level_view 夹具抽 submodule + store 归位

- **评审车**：claude（opus），新上下文独立评审，未参与实装；单线程，无子代理、无后台任务
- **评审面**：`git diff e520cf7fa4..HEAD -- rust/src/theta_v0/classifier/level_view.rs level_view/ level_view_store.rs mod.rs`
- **工位**：`/tmp/kimi-nest-mainline`，HEAD = `02ef5f93b1`
- **票面**：#497（`ready-for-agent`）；实施收口 `/tmp/issue497-dispatch-20260728.log`
- **写入范围**：仅本文件。零源文件改动、零 git mutation（`git status` 全程只读）

---

## 结论（先行）

**VERDICT = PASS（纯搬移等价性成立）。零 HIGH。2 条 MEDIUM，5 条 LOW。**

票面两项范围（① 测试模块抽 submodule 并消 19 次夹具重复；② `ConfirmCursor`/`ConfirmCursorStore`/`ConfirmState`
归位 `level_view_store.rs`）均已完成，且三条硬验收全部由本车独立复现通过：

| 验收项 | 票面要求 | 本车独立读数 | 判定 |
|---|---|---|---|
| 失败集合 | 与搬移前逐字一致 | 前后同为 `{signal::tests::extract_signals_bit_exact_digest_guard}`（#491） | ✅ |
| 测试总数 | 不变 | 剔除 #621 并行车新增后，叶名集合 2220 == 2220，`diff` exit=0 | ✅ |
| 250k dump | `cmp` = 0 | `P116_DUMP` 与 `P421_LIFECYCLE_DUMP` 双 dump SHA-256 逐字节相同 | ✅ |
| 禁改生产语义 | 无 | 生产段 diff 仅 = 1 行 `pub use` + 3 块搬出 + 2 字段 `pub(super)`，零逻辑行 | ✅ |

两条 MEDIUM 都不是「实装做错了」，而是「票面结论的边界必须显式写清、否则下游会误读」：
源头 Standards FAIL 并未消解（M-1），以及封装面被扩大到 classifier 全子树后的不变量守卫缺口（M-2）。

---

## 一、逐项复现（照实报数）

### 1. 失败集与测试总数

基线树用 `git archive e520cf7fa4 | tar -x -C /tmp/shadow497-base` 导出（**未用 `git worktree add`**），
独立 target 目录；HEAD 侧另用 `git archive HEAD` 导出 `/tmp/shadow497-head`（**规避工作区并行车未提交面**：
`git status` 显示 `rust/src/bin/p123_fast_replay.rs`、`p100_cert_bsp_recon.rs` 已改、
`rust/tests/shadow621_probe.rs` 未跟踪，均属他车）。

| 档 | 树 | passed | failed | ignored |
|---|---|---|---|---|
| `cargo test --lib --no-fail-fast` | base `e520cf7fa4` | 2082 | 1 | 137 |
| `cargo test --lib --no-fail-fast` | head `02ef5f93b1` | 2130 | 1 | 137 |
| `cargo test --no-fail-fast`（全量） | base | 2146 | 1 | 146 |
| `cargo test --no-fail-fast`（全量） | head | 2193 | 2 → **1**（见下） | 146 |

- **唯一红逐字一致**：两侧同为
  `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`
  （`src/theta_v0/classifier/signal.rs:3418`，摘要 left=16618955402698307653 / right=10432481772907336594，
  两侧完全同值）——即在案 #491，与本票面无关。
- **全量档 HEAD 侧第 2 条红是负载 flake，非回归**：
  `theta_v0::classifier::tests::incremental_tower_scaling_dominates_full_synthetic`
  （`src/theta_v0/classifier/mod.rs:3839`，实测比值 0.727 vs 门线 0.7，纯墙钟标度断言）。
  单跑复验 base×3、head×3 **全绿**，判定为并发跑测时机器负载所致，两树同性质。
- **+48 差额全部来自 #621**：base→head 的新增测试 100% 落在
  `theta_v0::classifier::retrace_ledger::tests::*`（commit `9e44d98728`，在评审区间内但非 #497 产出）。
- **level_view 面自身**：`cargo test --lib theta_v0::classifier::level_view` 两侧同为
  **31 passed / 0 failed / 0 ignored**。

### 2. 测试名集合 IDENTICAL 复验

`cargo test --lib -- --list`，剥模块前缀取叶名后排序：

```
level_view 面：      28 vs 28   diff exit=0（IDENTICAL）
全库（剔 retrace_ledger）：2220 vs 2220   diff exit=0（IDENTICAL）
```

带模块路径的全限定名如实变化（`level_view::tests::confirm_state_*` →
`level_view::tests::confirm::confirm_state_*`），属子模块化必然，与仓内 #621 `retrace_ledger/tests/` 先例同口径。
28 条 level_view 测试**无一被 `#[ignore]`、无一丢失、无一重名**。

### 3. 250k dump 抽点复验（#69 二进制口径）

两树各自 `CARGO_TARGET_DIR=... cargo build --release --bin p123_fast_replay`（bin 源在
`e520cf7fa4..HEAD` 内 **零变化**，已用 `git diff --stat -- rust/src/bin/` 确认为空），同参同数据：

```
P116_MAX_BARS=250000 P116_DUMP=... P421_LIFECYCLE_DUMP=... \
  p123_fast_replay /tmp/kimi-nest-mainline/analysis/data_cache/btc_1m_full.json
```

| 产物 | pre SHA-256 | post SHA-256 | cmp |
|---|---|---|---|
| `P116_DUMP`（60,534 B） | `614b8332…62421f` | `614b8332…62421f` | exit=0 |
| `P421_LIFECYCLE_DUMP`（67,837,667 B） | `ce00782a…c0ec14a` | `ce00782a…c0ec14a` | exit=0 |

stdout 唯一差异是 `P123_SPARSE_SUMMARY` 行的 `prefix_s`（44.509 → 39.719，墙钟耗时字段），
其余 `triggers/reevals/pan_hits/pan_writes/pan_invalidations` 等全部计数字段逐字相同——
按 #69 口径（shadow-494 报告先例）判为豁免项。**收口报告声称的 `614b83…` 由本车独立复现命中。**

### 4. 纯搬移抽验（超出要求：全 36 项，非抽 3 项）

写脚本按 brace 计数从 `e520cf7fa4:level_view.rs` 的 `mod tests { … }` 抽出全部 36 个顶层 item，
与 HEAD 的 8 个子模块文件逐 item 对照：

```
base items: 36   head items: 37
only_base: []           ← 零丢失
only_head: [pan_resident]  ← 唯一新增，即票面点名的 19 处折叠封装
逐字节不同: 15 项
```

15 项差异**逐条核对，全部落在两个已声明的改动上，零断言语义变化**：

- 6 项 = 共享夹具加 `pub(super)` 前缀：`unit` / `extended_windows` / `mbar` / `trend_block` / `query`
  （`level_view/tests/fixtures.rs:3,16,88,101,115`）、`pan_provider_fixture`
  （`pan_memo_fixtures.rs:27`）；`struct PanProviderFixture`（`pan_memo_fixtures.rs:3`）7 字段同。
- 9 项 = `provide_nest_candidate_events_resident(1, …)` → `pan_resident(…)`。
  **本车独立核对基线 19 处调用点首参全部字面为 `1`**（`level_view.rs` 基线 3373..4058 段逐处 `grep -A1` 确认），
  `pan_resident`（`pan_memo_fixtures.rs:176`）是 8 行纯转发，**折叠合法**。

`ConfirmCursor` 归位段（`level_view_store.rs:17-118`）与基线逐字节对照：

- `ConfirmState` 枚举 + `as_option` —— **逐字节完全相同**（`True`）。
- `ConfirmCursor` struct + `Default` impl —— 差异仅 9 个字段的 `pub(super)` 前缀 + 3 行新增 doc；
  `Default` 的 9 个初值（`k0:0` / `NEG_INFINITY` / `INFINITY` / `Scanning`）逐字节不变。
- `ConfirmCursorStore` + impl —— 差异仅 `cursors` 字段与 3 个方法的可见性前缀（`retain_active_for_run`
  另有 rustfmt 换行）；`retain_run_starts` / `retain_active_for_run` / `cursor_mut` / `poison_for_test`
  四个方法体逐字节不变。

commit `1c1aa1d537` 的「纯反缩进搬移」声明**独立验证成立**：把父提交 `70d4460359` 的 `mod tests` 体
统一去 4 空格后，与 `1c1aa1d537:level_view/tests.rs` **逐字节相同（True）**。

### 5. `pub(super)` 面审计

提升清单（全部在 `level_view_store.rs`，`pub(super)` = `classifier` 子树可见）：

| 项 | 位置 | 消费点 | 最小性 |
|---|---|---|---|
| `ConfirmCursor` 9 字段 | `level_view_store.rs:32-40` | `level_view.rs:1834` 起确认核心；`tests/confirm.rs:255` | 无更窄 path（见下） |
| `ConfirmCursorStore.cursors` | `:81` | `tests/confirm.rs:229,231,255,271,273,429` | 仅测试消费 |
| `retain_active_for_run` | `:93` | `level_view.rs:1813` | ✅ |
| `cursor_mut` | `:106` | `level_view.rs:1834` | ✅ |
| `poison_for_test`（`#[cfg(test)]`） | `:111` | `tests/confirm.rs:411` | ✅ |
| `ConfirmKey.level` / `.run_window` | `level_view.rs:494-495` | store 侧 `retain_*` | ✅ |

- **最小性判定**：`level_view` 与 `level_view_store` 的最近公共祖先就是 `classifier`，
  故 `pub(super)` ≡ `pub(in crate::theta_v0::classifier)`——在「类型搬到兄弟模块 + 原模块直访字段」
  这个既定分解下，**不存在更窄的 path 可见性**。收口报告称其为「拆分的必然代价」属实。
- **全仓 grep 确认无 classifier 子树外访问**：`.cursors` / `cursor_mut` / `retain_active_for_run` /
  `poison_for_test` 的消费点全部在 `src/theta_v0/classifier/` 内。
  子树外唯一消费方 `src/bin/p123_fast_replay.rs:160,400,1709,2739` 只用
  `ConfirmCursorStore` 类型名与 `pub fn retain_run_starts`——**均为搬移前已 pub 的面，零扩大**。
- **`pub use` 保路径成立**：`level_view.rs:24` 的
  `pub use super::level_view_store::{ConfirmCursor, ConfirmCursorStore, ConfirmState};`
  使 `p123_fast_replay.rs` 的 import 行零改动（bin 源 diff 为空即为实证）。

### 6. 禁迁 crate 核查

- `rust/tests/` 在 `e520cf7fa4..HEAD` 的 **tracked 变更为空**（`git diff --stat` 无输出）。
  工作区那个未跟踪的 `rust/tests/shadow621_probe.rs` 属 #621 影子车，不在本票产出内。
- crate 内 `#[cfg(test)]` hook 全部原地未动：`CONFIRM_CORE_CALLS` thread_local 与其
  reset/read helper 仍在 `level_view.rs:27-38`（`#[cfg(test)]` 门控不变），
  由子模块经 `use super::*` 访问（Rust 祖先私有项对后代可见，无需提升可见性）。
- 仓内无 `event_probe` 符号（该名为派单泛指）；本票涉及的探针即上述 `CONFIRM_CORE_CALLS`
  与 `poison_for_test`，两者都留在 crate 内、都仍受 `#[cfg(test)]` 门控。

### 7. 生产语义零改动（逐 hunk）

`level_view.rs` 生产段（截至 `mod tests` 之前）base 2049 行 vs head 1962 行，
`unified_diff` 全量 124 行，逐 hunk 归类：

1. `+4` 行：`pub use` 再导出 + 3 行 doc（`level_view.rs:21-24`）。
2. `-18` 行：`ConfirmState` + impl 搬出。
3. `-30` 行：`ConfirmCursor` + `Default` impl 搬出。
4. `±2` 行：`ConfirmKey.level` / `.run_window` 加 `pub(super)`。
5. `-42` 行：`ConfirmCursorStore` + impl 搬出。

**零条件分支、零常量、零算式、零签名（对外）被改。** `level_view_store.rs` 侧对称：
仅顶部 import 补 `Tick` / `BTreeSet` / `HashMap` / `ConfirmKey` + 三块搬入，原有内容零触碰。

编译警告面：`cargo test --lib --no-run` 两侧同为 **53 warnings**，且两侧 grep `level_view`
在警告输出中**零命中**——新文件零新增警告，搬空后的 `use std::collections::{BTreeSet, HashMap}`
（`level_view.rs:19`）仍有实际消费者，无 unused import。

---

## 二、Findings

### MEDIUM-1 — 源头 Standards FAIL HIGH-2 并未消解，关票措辞若不订正即构成声明膨胀

**证据**：`rust/src/theta_v0/classifier/level_view.rs` 当前 **1965 行**（生产段 1962 行），
是 `.claude/rules/common/coding-style.md:19` 「800 max」的 **2.45 倍**。

#497 的来源是 `code-review-issue69-standards-20260727.md` 的 Standards 轴 FAIL HIGH-2
（该文件 1720 → 3926 行）。本票把 #69 引入的 ~1885 行测试增量搬走后，文件回到**与 #69 之前同量级的
超限状态**——即：**本票消解的是「#69 的增量违规」，不是「≤800 违规」本身**。
票面《范围》第 3 句已把长函数/参数列排除（「另议，不在本票」），故实装完全合规；
但生产段仍有 11 个 >50 行函数（`provide_nest_candidate_events_ext_resident` 277 行、
`assemble_level_view_impl` 200 行、`pan_block_triple` 190 行…），这些正是把文件钉在 1962 行的实体。

**为何是 MEDIUM 而非 LOW**：若 #497 以「标准债已还」关闭，下游读到的是「level_view 已合 ≤800 标准」，
而代码不具备该属性——这正是 `no-patch-mentality.md` 禁止模式 5「声明膨胀」。

**建议（不在本车执行范围）**：关票 comment 明写「仅还 #69 增量；≤800 未达成，剩余 1962 行生产段
另立标准债票」，并把长函数拆分挂成 #497 的后继 issue（先 `gh issue list --search --state all` 查重）。

### MEDIUM-2 — `ConfirmCursor` 字段可见性从 1 个模块扩到 24 个兄弟模块，游标不变量失去编译期守卫

**证据**：`level_view_store.rs:32-40` 的 9 个字段（含 `k0`、`state`）由「`level_view` 模块私有」
提升为 `pub(super)`。`classifier/mod.rs` 下有 **24 个 `pub mod` 兄弟**
（`signal` / `decompose` / `divergence` / `nest_lifecycle` / `retrace_ledger` / `ledger_kernel` …），
它们及其后代现在都能直接写 `cursor.k0 = …` / `cursor.state = ConfirmState::Confirmed(t)`。

`k0` 的 doc 声明是「下一个尚未消费的 lower-leg 下标；**只允许落在确认水线内**」——这是一条不变量。
搬移前，任何写入都必须经 `level_view.rs` 内的 `scan_confirm_cursor` 路径；搬移后，
该纪律由「模块私有」这个编译期保证降级为「约定」。同理 `ConfirmCursorStore.cursors` 直接暴露 map 本体，
`retain_run_starts` / `retain_active_for_run` 的剪枝纪律可被绕过。

**边界**：如第 5 节所述，在票面指定的分解下**不存在更窄的 path 可见性**，且实测子树外零访问、
子树内实际消费点只有 `level_view.rs:1813,1834` 与 `tests/confirm.rs` 六处。
所以这不是「实装选错了」，是**分解方式本身的代价**，本票不做属合规。

**建议**：后继票把消费这些字段的确认核心（`scan_confirm_cursor` / `trend_confirm_state_core`）
一并归位 `level_view_store.rs`，字段退回私有、对外只留 `pub(super)` 推进方法。
这才是把「只允许落在确认水线内」重新变成编译期事实的严格形式。

### LOW-1 — 收口报告的基线读数不可从 git 复现

`/tmp/issue497-dispatch-20260728.log` 记「基线 2104 passed / 26 failed / 137 ignored」。
本车在 `e520cf7fa4` **纯净隔离树**实测为 **2082 passed / 1 failed / 137 ignored**。
差额来自当时共享工作树携带了并行车 #621 的未提交态（26 红中 25 条为 `retrace_ledger`）。
**结论不受影响**——「唯一非 retrace_ledger 红 = #491，前后逐字相同」已由本车在两棵纯净树独立复现。
但证据链里的「基线」一栏取自脏树，第三方无法据 commit 复算。后续同类收口建议一律用
`git archive <base> | tar -x` 导隔离树取基线读数。

### LOW-2 — 派单口径写「两 commit」，实际 #497 为四 commit

评审面 `e520cf7fa4..HEAD` 内的 #497 提交是四个：`edb3d9c765`（store 归位）、
`5d54530c59`（`pan_resident` 折叠）、`1c1aa1d537`（tests.rs 抽出）、`02ef5f93b1`（八路拆分）。
派单只点名后两个。本报告按**四个全评**（前两个恰是唯一触碰生产段与断言调用点的提交，
若只评后两个会漏掉全部生产面改动）。

### LOW-3 — `pan_memo_a` / `pan_memo_b` 按体积切分而非按主题

`coding-style.md:47` 要求「Organize by feature/domain, not by type」。
`pan_memo_a.rs`（313 行）与 `pan_memo_b.rs`（386 行）的文件名不承载任何领域含义，
合并后 ~695 行**仍在 800 以内**，即分成两个并非 ≤800 约束所迫。
建议：或合为单个 `pan_memo.rs`，或按内容改名（a = 冷路径/增长/不完备 MACD，
b = 失效纪律/重物化/强制冷 oracle → 如 `pan_memo_growth.rs` / `pan_memo_invalidation.rs`）。
其余 6 个文件名（`confirm` / `projection_pairing` / `triple_anchor` / `feature_gating` /
`fixtures` / `pan_memo_fixtures`）均为主题命名，合规。

### LOW-4 — 三个类型现有两条对外公开路径

`classifier` / `level_view` / `level_view_store` 三级全部 `pub`，故
`ConfirmCursor` 等既可经 `classifier::level_view_store::ConfirmCursor`（原生定义处）
访问，也可经 `classifier::level_view::ConfirmCursor`（`level_view.rs:24` 的 `pub use`）访问。
兼容层是有意为之且 doc 已注明理由，无功能风险；但同物双路径长期存在会让「哪条是正路」失去唯一答案。
建议在后继票把 `p123_fast_replay.rs:160` 的 import 改指原生路径，然后删掉这行 `pub use`。

### LOW-5 — 票面《验收》里的在案红名单已过期

票面写「失败集合……（lee×3 = #446、digest guard = #491 在册）」。
本车两树实测：`lee_m3_*` / `lee_m4_*` 共 6 条**全绿**，唯一红只有 #491。
即 #446 的 lee×3 在 `e520cf7fa4` 之前某次修复中已转绿，票面读数陈旧。
不影响本票判定（「前后一致」这条更强的判据已满足），但关票时不宜照抄票面那句在案名单。

---

## 三、结果包六要素

1. **结论**：#497 实装 PASS。四个 commit 构成的改动经三条硬验收（失败集/测试名集/250k 二进制 dump）
   独立复现全部通过，生产语义零变化。无 HIGH。MEDIUM-1 要求订正关票措辞，
   MEDIUM-2 记为后继票，两者均不阻断 #497 关闭。
2. **定义依据**：
   - 「纯搬移」判据取自票面《范围》「纯搬移，零语义」+《验收》三条；
     等价性由 `P116_DUMP`/`P421_LIFECYCLE_DUMP` 双 dump SHA-256 相等（250k bar，真实 BTC 数据）实证——
     这是 L2 级证据（真实数据、可否证），非 L0/L1。
   - 「≤800」与「按 feature/domain 组织」取自 `.claude/rules/common/coding-style.md:19,47`。
   - 「声明膨胀」判据取自 `.claude/rules/no-patch-mentality.md` 禁止模式 5。
   - 可见性最小性判据：Rust path 可见性语义（`pub(super)` on `level_view_store` ≡
     `pub(in …::classifier)`，为覆盖 `level_view` + `level_view_store` 的最小 path）。
3. **边界条件（结论何时翻转）**：
   - 若在 250k 之外的窗口（如全窗 461 万 bar）出现 dump 差异 → 「零语义」翻转。本车只测 250k，
     有效域**仅覆盖 250k 前缀**，未做全窗；票面亦只要求 250k 抽点。
   - 若 `P116_DUMP` 的字段覆盖面不含某条被搬移代码影响的量（dump 是「只写不判」旁路），
     则二进制相等不足以证明全部语义等价 → 该情形下结论降为「dump 可观测面上等价」。
     缓解：`level_view` 面 31 条单测 + 全库 2220 条叶名测试同时全绿，两条独立证据同向。
   - 若 MEDIUM-2 所述的 24 个兄弟模块中，将来有任一模块直写 `ConfirmCursor` 字段 →
     「零风险」翻转为实际不变量破坏。当前实测为零，属**未来风险**而非现存缺陷。
   - 若 `incremental_tower_scaling_dominates_full_synthetic` 在空载单跑下仍红 →
     「flake」判定翻转为回归。本车 base×3 / head×3 单跑全绿。
4. **下游推论**：
   - `p123_fast_replay.rs` 及一切经 `classifier::level_view::Confirm*` 的消费方**无需任何改动**
     （bin 源在评审区间内 diff 为空，即为实证）。
   - #69 的封印口径（250k dump 二进制相等）在本次搬移后**继续成立**，
     后续以 `614b8332…62421f` 为 `P116_DUMP` 新基线指纹可直接复用。
   - 任何引用 `level_view.rs:2041..3926` 一类旧行号的文档/票面锚点全部失效（文件已 4072 → 1965 行）。
   - #491 digest guard 仍红，与本票无关，归 #610 归因线。
5. **谱系引用**：本票不涉及概念分离，无相关谱系条目。相关规则谱系：
   `no-patch-mentality.md`（090 号严格性语法规则、声明膨胀禁止）为 MEDIUM-1 的依据；
   `formalization-validity-domain.md` 的认识论等级用于第 3 项边界条件的有效域标注（本报告证据 = L2）。
6. **影响声明**：本次评审**未改动任何源文件、未执行任何 git mutation**。
   新增产物仅本文件 `chanlun/review-results/shadow-497-review-20260728.md`。
   评审期间在 `/tmp` 下建了两棵隔离树（`shadow497-base` / `shadow497-head`）、
   两个 target 目录与若干 dump/list 中间产物，全部在仓外，不影响工作树。
   工作区并行车未提交面（`p123_fast_replay.rs`、`p100_cert_bsp_recon.rs`、
   `shadow621_probe.rs`、`chanlun/review-results/` 下他车文件）**全程零触碰**。

---

## 附：复现命令

```bash
# 隔离树（禁 git worktree add）
mkdir -p /tmp/shadow497-base /tmp/shadow497-head
cd /tmp/kimi-nest-mainline
git archive e520cf7fa4 | tar -x -C /tmp/shadow497-base
git archive HEAD       | tar -x -C /tmp/shadow497-head

# 失败集 / 总数
for t in base head; do (cd /tmp/shadow497-$t/rust && cargo test --lib --no-fail-fast); done

# 测试名集合 IDENTICAL
for t in base head; do (cd /tmp/shadow497-$t/rust && \
  cargo test --lib -- --list | grep ': test$' | sed 's/: test$//' | sort) > /tmp/s497-$t.txt; done
diff <(grep -v retrace_ledger /tmp/s497-base.txt | sed 's/.*:://' | sort) \
     <(grep -v retrace_ledger /tmp/s497-head.txt | sed 's/.*:://' | sort)   # exit=0

# 250k dump
for t in base head; do (cd /tmp/shadow497-$t/rust && \
  CARGO_TARGET_DIR=/tmp/shadow497-target-$t cargo build --release --bin p123_fast_replay); done
for t in base head; do P116_MAX_BARS=250000 P116_DUMP=/tmp/s497-$t.p116 \
  P421_LIFECYCLE_DUMP=/tmp/s497-$t.lc /tmp/shadow497-target-$t/release/p123_fast_replay \
  /tmp/kimi-nest-mainline/analysis/data_cache/btc_1m_full.json > /tmp/s497-$t.stdout 2>&1; done
cmp /tmp/s497-base.p116 /tmp/s497-head.p116 && cmp /tmp/s497-base.lc /tmp/s497-head.lc

# 逐 item 纯搬移对照（脚本 /tmp/shadow497_extract.py）
python3 /tmp/shadow497_extract.py \
  /tmp/shadow497-base/rust/src/theta_v0/classifier/level_view.rs \
  /tmp/kimi-nest-mainline/rust/src/theta_v0/classifier/level_view/tests --dump
```
