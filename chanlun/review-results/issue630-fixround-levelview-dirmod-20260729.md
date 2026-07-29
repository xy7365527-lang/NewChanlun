# #630 修复轮收口：level_view 目录化 + 43 项可见性真收紧（影子 MEDIUM-1/2/3）

- **执行车**：claude（sonnet，常规档），单线程，禁子代理/禁后台任务；工位 `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`
- **依据**：`chanlun/review-results/shadow-630-review-20260729.md` MEDIUM-1/MEDIUM-2/MEDIUM-3/LOW-3；#633 批7 `incremental/` 目录模块先例
- **commit**：`e26e46a51b`（单提交，四项一次性落地——四文件迁移 + 可见性收窄 + doc 订正深度交织于同一批文件，拆分提交不增可核验性）

---

## 一、目录化映射（旧路径 → 新路径）

| 旧路径（classifier 兄弟文件） | 新路径（level_view 子模块） |
|---|---|
| `classifier/level_view.rs` | `classifier/level_view/mod.rs` |
| `classifier/level_view_projection.rs` | `classifier/level_view/projection.rs` |
| `classifier/level_view_confirm.rs` | `classifier/level_view/confirm.rs` |
| `classifier/level_view_pan.rs` | `classifier/level_view/pan.rs` |
| `classifier/level_view_pan_provider.rs` | `classifier/level_view/pan_provider.rs` |

选型：`level_view.rs` 主体改 `level_view/mod.rs`（不采用"文件+目录并存"，与 #633 批7 `incremental/` 口径一致——同分支已有先例，避免同一 crate 内两种目录模块惯例并存）。`level_view_store.rs`/`level_view/tests/` **不在本次迁移范围**（前者票面未列且与 4 拆分文件无同源关系；后者 #497 冻面不动）。

`classifier/mod.rs` 的 4 条 `pub mod level_view_projection/confirm/pan/pan_provider;` 声明整体删除，折叠进 `level_view/mod.rs` 内部的私有 `mod projection/confirm/pan/pan_provider;`（比 MEDIUM-3 建议的 `pub(super) mod` 更彻底——不留 classifier 层路径，因为这 4 个子模块的路径除 `level_view::X` 再导出外没有任何消费者）。

4 个子模块内部的 `use super::X`（原指向 classifier 兄弟模块）全部改写为 `use super::super::X`（super 由 classifier 变 level_view，多一层）；指向 `level_view.rs` 自身条目的 `use super::level_view::Y` 改写为 `use super::Y`；相互引用（如 pan.rs → confirm.rs）由 `use super::level_view_confirm::Z` 改写为 `use super::confirm::Z`。`level_view_store`（未迁移）的引用路径不变（`use super::super::level_view_store::...` 从 confirm.rs 视角，因其 super 已变）。

## 二、43 项可见性收窄对照表

以现状（本票修复前，即 #630 原实装 `ceb28e34d4` 之后）为基线核验，逐项判定：

### A. 16 个函数——全部保留 `pub(super)`，`super` 由 classifier 自动变为 level_view

| 函数 | 所在文件 | 跨文件消费点 |
|---|---|---|
| `leg_as_segment` | projection.rs | mod.rs、pan.rs、pan_provider.rs |
| `reset_confirm_core_calls` / `confirm_core_calls` | confirm.rs（cfg test） | mod.rs cfg(test) 再导入 → tests/ glob |
| `ConfirmKey::for_pair` | confirm.rs | mod.rs（`assemble_level_view_impl`） |
| `trend_confirm_state` / `trend_confirm_state_core` / `trend_confirm_time` | confirm.rs | mod.rs；tests/confirm.rs |
| `structural_block_span` / `structural_pair_span` / `pan_owner_block_index` / `pan_block_triple` | pan.rs | pan_provider.rs |
| `PanMemo::{prepare,lookup,insert,poison_for_test}` | pan.rs | pan_provider.rs；`poison_for_test` 另被 tests/pan_memo_b.rs 消费 |
| `resolve_triple_anchor` | pan_provider.rs | 本文件内 `materialize_pan_event`；mod.rs cfg(test) 再导入 |

判定：均为 level_view 子树内跨文件消费，无一能降回 private，也无一需要越出 level_view（不被 `level_view_store.rs` 等 classifier 层代码直接调用）。目录化后 `pub(super)` 的可见域从「classifier 24 个兄弟模块」自动收窄为「level_view 子树」，零关键字改动即达成 MEDIUM-2 的收紧目标。

### B. 5 个 struct + 1 个 enum——同上，全部保留 `pub(super)`

`PanSegmentIdentity`／`PanCenterIdentity`／`PanBlockIdentity`／`PanMemoKey`／`PanEventCore`（struct）、`PanMemoValue`（enum），均由 `pan.rs` 定义、`pan_provider.rs` 以字段字面量或模式匹配跨文件消费（逐项 grep 核验：`pan_provider.rs` 直接构造 `PanMemoKey { .. }`/`PanEventCore { .. }` 全字段字面量），故其**内部字段**本身缺省 private（未加任何可见性修饰——`PanSegmentIdentity`/`PanCenterIdentity`/`PanBlockIdentity` 三个"身份"类型只经 `From` impl 构造、派生 trait 比较，从未跨文件裸访问字段，字段私有性一直成立，不受本次迁移影响）；`PanMemoKey`/`PanEventCore` 的字段则确需 `pub(super)`（pan_provider.rs 以结构字面量构造/解构），随迁移同样自动收窄到 level_view 子树。

### C. ConfirmKey 的 9 个字段——本票唯一发生关键字变更的一组

| 字段 | 消费点 | 处置 |
|---|---|---|
| `level` | `level_view_store.rs:100,110`（`retain_run_starts`/`retain_active_for_run` 直读 `key.level`） | **`pub(in super::super)`**（钉 classifier，不随迁移收紧——`level_view_store.rs` 不在本次目录化范围，仍是 classifier 直接子模块，`pub(super)`=level_view 不够用，会破坏其编译） |
| `run_window` | 同上（`key.run_window.start`） | 同上，**`pub(in super::super)`** |
| `structure_generation` | `level_view/tests/confirm.rs:221/242/266/275`（`ConfirmKey`/`store.cursors.keys()` 结构字面量构造与字段读取，用于结构代次剪枝断言） | 保留 **`pub(super)`**（level_view 子树——tests/ 是 level_view 的子模块，够用；首次尝试降为 private 后 `cargo test --lib` 报 E0616，回退确认） |
| `version` / `pair_id` / `move_start` / `seg_a` / `c_start` / `b_fingerprint` | 逐项 grep 全 repo：仅 `confirm.rs` 内 `ConfirmKey::for_pair` 构造点使用，无任何跨文件/跨测试消费 | **降回 private**（7 项，本票唯一的真实收紧关键字改动） |

`ConfirmKey` 结构体本身（非字段）维持既有 `pub`（未改动——供 `level_view_store.rs::ConfirmCursorStore.cursors: HashMap<ConfirmKey, ConfirmCursor>` 使用类型名，且已是 #497 既定状态，非本票范围）。

### 小结

- 43 项中 41 项（16 fn + 5 struct + 1 enum + 9 ConfirmKey 字段中的 7 个私有化 + PanMemoKey/PanEventCore 的 13 个 pub(super) 字段——按 §B 计入 pub(super) 而非本节重复列）保持 `pub(super)` 关键字不变，靠目录移动自动收窄可见域；
- 2 项（`level`/`run_window`）因 `level_view_store.rs` 跨域直读，钉 `pub(in super::super)` 不收紧；
- 7 项（ConfirmKey 的其余字段）降回 private，是本票唯一的显式关键字收紧。

## 三、MEDIUM-1（PanMemoStats）

`level_view/mod.rs` 补 `pub use pan::{provide_divergence_pairs, PanMemo, PanMemoStats, PanResidence};`（原只有 3 个名字，遗漏 `PanMemoStats`）。`classifier::level_view::PanMemoStats` 路径恢复可达（本仓无消费者，修复前不破编译，修复后仍不破编译，纯路径补全）。

## 四、MEDIUM-3（`pub mod` → 内部私有 `mod`）

不采用影子建议的 `pub(super) mod`（那仍在 classifier 层留一条路径），而是随目录化把 4 条声明整体移入 `level_view/mod.rs` 内部，声明为无修饰符的私有 `mod`——因为这 4 个子模块除经 `level_view::X` re-export 外没有任何直接路径消费者（sed 实验+全 repo grep 双重核验），私有已是可行最小可见性。

## 五、LOW（store.rs doc 订正）

`level_view_store.rs:37-45` 的 `ConfirmCursor` 字段可见性 doc 整段重写：删除已过期的「三条收紧路径" vs "两条枚举」不一致表述（原文枚举漏列"访问器化"）与「两个 sibling 文件」的拓扑描述（迁移后 `confirm.rs` 已非 `level_view_store.rs` 的同级兄弟，而是 `level_view` 的子模块，最近公共祖先仍是 classifier 但层级关系变了），改为准确描述本票迁移后的现状与理由。

## 六、验证链（四件套 + 附加项）

| 项目 | 结果 |
|---|---|
| 测试名集合 IDENTICAL | `theta_v0::classifier::level_view` 前缀，base（影子 base 树 `6c77bbab19^`）32 条 vs 本票后 32 条，`diff` exit=0 |
| 250k dump cmp | `P116_DUMP` SHA-256 `614b83323271752a0b569d11f3efb4ea2e472524b584f1570c323b03ca62421f`；`P421_LIFECYCLE_DUMP` SHA-256 `ce00782a6a4c5267d96a04a97367586d23ea05c854de575eabf0c587fc0ec14a`——与影子评审记录逐字符相同 |
| `cargo build --lib` | 零 error；37 warnings（与影子基线逐字相同） |
| `cargo build --release --bins` | 零 error |
| `cargo test --lib` | 2205 passed / 0 failed / 138 ignored（与影子基线逐字相同） |
| 6 处存量超限函数行数 | `provide_nest_candidate_events_ext_resident` 274、`assemble_level_view_impl` 199、`trend_confirm_state_core` 106、`scan_confirm_cursor` 86、`project_extended_windows_impl` 79、`provide_divergence_pairs` 73——逐一与影子记录相同，零收敛（沿用票面"单列登记"出口，未新增超限函数） |
| 文件行数（全部 ≤800） | `mod.rs` 585、`projection.rs` 231、`confirm.rs` 395、`pan.rs` 373、`pan_provider.rs` 487、`level_view_store.rs` 711（doc 增量，未拆，票面已知） |
| fixture 漂移检查 | `scripts/check_fixture_drift.py` exit=0，全绿（本票未动 `formal/`，例行核验） |

## 七、结果包六要素

1. **结论**：MEDIUM-1/2/3 三项影子发现全部落地；43 项 pub(super) 中 41 项靠目录移动自动收窄到 level_view 子树，2 项（ConfirmKey.level/run_window）因 `level_view_store.rs` 跨域消费钉 `pub(in super::super)`（=classifier，不收紧，且是唯一必要的例外），7 项（ConfirmKey 其余字段）显式降回 private。四件套 + 行数/警告/fixture 附加验证全绿，零行为变化。
2. **定义依据**：影子评审 `shadow-630-review-20260729.md` MEDIUM-2 指名的"第四条收紧路径"（目录模块，`super` 从 classifier 变 level_view）；#633 批7 `incremental/` 目录模块先例（同一手法、同一 crate）；Rust 2018 `mod.rs`/子目录模块惯例（[The Rust Reference — Modules]，无需额外引用外部权威，属语言机制而非缠论概念）。
3. **边界条件**（结论翻转条件）：
   - 若未来 `level_view_store.rs` 也被迁入 `level_view/` 目录，则 ConfirmKey.level/run_window 的 `pub(in super::super)` 应随之收紧为 `pub(super)`（=level_view）——当前不做是因为该文件明确不在本票范围。
   - 若外部（本仓之外）存在消费 `classifier::level_view_pan_provider::X` 等旧路径的代码，则删除 `pub mod` 声明会破坏其编译；本仓内已核实零此类消费者，crate 外消费不可知但 `ConfirmCursor`/`ConfirmState` 等唯一对外契约点已通过 `level_view::` 单一路径不变维持。
   - 若 `level_view/tests/` 未来新增对 ConfirmKey 除 `structure_generation` 外字段的直接构造，会重新触发 E0616，需要把该字段从 private 升回 `pub(super)`（本票判定基于当前测试代码的静态快照，非永久不变的架构保证）。
4. **下游推论**：(a) `classifier::level_view::X` 对外路径集合不变（36 个引用文件的 `cargo build --bins` 零错误已验证）；(b) 后继若要继续收紧 ConfirmCursor/ConfirmKey 的 classifier 层可见性，需要先把 `level_view_store.rs` 一并纳入 `level_view/` 子树（本票已为此路径铺垫：目录已存在、迁移手法已验证）；(c) `#633` 批次的目录模块惯例（`incremental/`）现有第二个同构先例（`level_view/`），可视为 classifier 内的通用重构模式确认。
5. **谱系引用**：本票是纯 Rust 模块拓扑重构，不涉及缠论概念定义分离，无缠论谱系条目适用；工程侧参照 `no-patch-mentality.md`（严格性：不满足于"字面自洽"的 pub(super) 声明，追问到"是否存在更严格路径"并落地）与影子评审自身的 `chanlun/review-results/shadow-630-review-20260729.md`。
6. **影响声明**：改动 7 个文件（5 个 rename+内容修改、`level_view_store.rs`/`mod.rs` 内容修改），0 个文件新增/删除；`classifier::level_view_projection/confirm/pan/pan_provider` 4 条 classifier 层直接模块路径消失（本仓内零消费者，已验证）；`classifier::level_view::PanMemoStats` 路径恢复；`level_view` 子树内 7 个私有字段的可访问范围收窄；`level_view_store.rs` 的 doc 注释更新，无代码行为改动。不影响 `level_view/tests/`（#497 冻面）与 `incremental/`（#633 已验收面）。

## 八、偏离说明

- 票面草稿列出的文件名之一「`level_view_env.rs`」在仓库中不存在（`ls` 核实），按票面"以实际为准"条款理解为笔误，实际迁移 4 个真实存在的拆分文件（`projection`/`confirm`/`pan`/`pan_provider`），与影子评审 MEDIUM-2 指名的"4 个拆出文件"一致；`level_view_store.rs` 未计入迁移范围（该文件源自 #497，非 #630 拆分产物）。
- LOW-4（`nest_lifecycle.rs:2262` 引用 `leg_as_segment` 的行号/可见性双重过期注释）与 LOW-5（收口报告未入库）未在本票范围内一并修——票面 4 项只点名 store.rs 的 doc；LOW-5 已通过本文件入库解决；LOW-4 保留为已知既存缺口（非本票引入，本票的路径变化——`leg_as_segment` 迁到 `level_view/projection.rs:219`（行号不变，文件名变）——使该注释的失效程度不变，未恶化）。
