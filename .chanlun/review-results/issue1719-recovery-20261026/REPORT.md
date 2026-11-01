# #1719 精确 B 绑定与第二层清账核交付报告

本文件是绑定 #1719 的活期工作草稿。本轮源码组件已整理，供宿主受管封包和远端验证。未运行 Rust 编译或测试，未取得非作者独审 PASS，不能据此宣称整票或正式三端点已验收。

## 工位与恢复

- 当前分支 `sandcastle/issue-1719`，HEAD 保持 `7b5cde506be906fd2cef5f3c0de28e0752e45e18`。
- 第一项操作按用户命令执行 `git apply /Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1719-gpt-checkpoint26/wip.patch`，未加 `--index`，退出 0。随后将旧 REPORT 复制到本路径作为起点。
- 已读 42R 全文、42W 接续 brief、ADR 0003 第 5–11 行、工位 PLAN、CODING_STANDARDS、AGENTS，以及 `gh issue view 1719 --json title,body,comments`。未评论或修改票面。
- 所有源码与文书只写当前工位。未运行 git add/commit/push，未改分支、Git metadata 或 main；提交与远端验证交宿主受管步骤。

## 改了什么

`pan.rs` 的旧 active provider 保留原签名，增加 `provide_active_pan_live_windows_with_policy` 伴随入口。两个入口共用原 D-3/Extreme 核，在真正选中 `centers[center_index]` 的同一位置产生绑定。绑定包含完整 CenterId、level 所在的原 LifecycleKey、原注册 bar、具名政策版本和完成锚。PanLiveWindow、旧 feed 和 channel_switch 调用合同不变。

`center_break.rs` 的事实构造口改为接收实际 `CenterLifecycleEvent`，只有 Broken 返回事实。完整事件原件保留级别、Center 快照、chain_index、三类点确认源坐标和方向。物理 bar 映射与 known-at 分列，不假设源坐标等于物理 bar，也不改成消费 bar。测试的 Broken 经真实 CenterEventMachine 产生，输入明确为合成结构夹具，不是正式 S 已发布输入。

`feed.rs` 删除按 start 找 B 和写死版本的旧入口。新伴随 feed 先调用原两相 feed，再核对本 bar 的 Live 身份与伴随绑定；返回每份绑定的接纳结果。清账入口只接已提取事实，不再读取当前 centers。政策由调用方显式提供，组件不代填签署版本，不因 strict 失败切换锚。

`book.rs` 在原 `migrate_entry` 点同步移动绑定。Bridge 迁移不改原对象、B、政策或注册钟。CenterUpgrade 保留原对象与原政策，在新精确 B 尚未提供时暂停清账；提供后更新当前 B，同时保留原 B。终态前身的独立认领继续独立建仓、独立绑定，不继承前身政策。

## 旧缺陷逐项修复

| 旧缺陷 | 本轮处理 |
|---|---|
| 按 start 猜配、丢失核心 | active provider 同步携带真正选中的完整 B；清账无 centers 反查。多 B 同 start 测试明确选末个核心，冲突重绑被拒。 |
| 写死 `"v0"` | 显式 `CompletionPolicy { version, anchor }`，拒绝空版本；默认旧 channel_switch 入口不变。 |
| 注册覆盖旧政策 | 首次绑定冻结原对象、原 B、原注册钟、原政策。重发或新版本不能覆盖；两个锚方向均有负控。 |
| 只清第一个目标 | 遍历全部精确匹配目标；同 start 异核心、同核心异级均隔离。 |
| 去重键缺级别/原对象/版本 | 应用键包含 level、完整 CenterId、chain_index、三类点源坐标、方向、原 LifecycleKey 和原政策版本。迁移后仍使用原对象。 |
| 同一事实改 known-at 后重试 | 首次收到的有效目标事实在 book 内独立留档，包括迟到拒绝者；同一身份的时钟或完整原件冲突拒收，不允许改钟放行。 |
| 桥键缺账本项时 expect | 先检查账本存在，缺账返回空；测试直接注入孤桥模拟缺账，无生产测试写口。 |
| 缺独立清账审计 | `center_break_audits` 只追加实际清账凭证，保留完整事实、原绑定、结算键和消费钟。待处理事实另有 `center_break_facts` 读面。 |
| 缺时钟守卫 | 要求原注册 bar ≤ 物理 bar ≤ known-at ≤ 消费 bar，且对象 last_as_of ≤ known-at。合法清账的 invalidated/revision 钟取原 known-at；已推进越过的迟到事实拒绝，原账本钟不动。 |
| 把曾可证者也清掉 | 仅原政策 CenterBreak、精确 B/级别、仍 Provisional、first_provable_at 为空者可清账。Confirmed/Invalidated 和曾可证 Provisional 逐位保留。 |
| Reset/Superseded/strict 失败被混为 Broken | 非 Broken 构造失败；真实 Reset、Superseded、载体校验失败及真实力度判负均有负控。无 Broken 不产生第二层清账。 |

新增原因仍只表示第二层结构清账。不会伪造 ForceOvertake、StructureCompleted、结构卖点或成交；`consumable_closed` 仍只开放 Confirmed。没有交易消费者接线。

## 新增测试源码

以下 14 个测试均未在本轮执行。测试文件头和缺账测试注释均声明为合成结构夹具。列表是源码清单，不是通过计数。

- `i4_bridge_multiple_targets_exact_core_level_and_duplicate_fact`
- `i4_bridge_original_policy_is_frozen_in_both_directions`
- `i4_bridge_provider_conflicting_core_does_not_replace_binding`
- `i4_bridge_center_upgrade_requires_exact_new_b_and_preserves_origin`
- `i4_bridge_clock_guards_reject_future_retrograde_and_before_registration`
- `i4_bridge_provable_and_terminal_entries_keep_all_bits`
- `i4_bridge_reset_superseded_and_strict_failures_do_not_clear`
- `i4_bridge_channel_switch_default_equivalence`
- `i4_bridge_source_physical_known_and_consumed_clocks_remain_distinct`
- `i4_bridge_completion_migration_preserves_binding_and_late_fact_absorbs`
- `i4_bridge_policy_validation_and_late_registration_are_rejected`
- `i4_bridge_active_provider_keeps_legacy_outcome_and_selected_core`
- `i4_bridge_terminal_center_upgrade_opens_independent_policy_binding`
- `i4_bridge_missing_ledger_item_is_noop_without_panic`

覆盖多合法目标、同 start 异核心、同核心异级、重复事实与改钟重送、原政策双向变更、普通桥延展、完成迁移、CenterUpgrade 暂停与精确接续、终态前身独立认领、缺账、未来钟/倒退/注册前事实、三类时钟和确认源坐标分离、终态与曾可证保护、Reset/Superseded/strict 失败，以及原 channel_switch 的增量、统计、全部 entry 字段、完成信号、不可验审计和拒绝注记逐位对拍。

## 实际验证结果

- `git diff --check` 退出 0。
- 使用仓外 `/private/tmp/issue1719-parser` 中的 tree-sitter 解析 11 个 nest_lifecycle 模块文件，无语法错误或 missing 节点。使用预编译 Python wheels，未运行 cargo/rustc/rustfmt。语法解析不能证明 Rust 类型、名称解析或行为正确。
- 通过语法节点提取并与 HEAD 比较，旧 `feed_replay_bar`、`provide_replay_live_windows`、`provide_pan_live_windows` 函数文本逐字节相同。
- 源码枚举得到 14 个互不重名的 `i4_bridge_` 测试。Rust 实际执行数为 0，含义为未执行，不是通过 0 项。
- 本机未运行 cargo/rustc/rustfmt，也未对旧 HEAD 运行 nc-remote。不存在可以证明当前 WIP 的 Rust 通过日志。

## 宿主封包后必须执行

先由宿主受管提交形成新 head，再在本工位确认源码已由该 head 完整封包，固定完整 SHA。不能用当前继承的 7b5cde50 或移动的 main 替代。下面的变量在宿主提交完成后取值，所有命令绑定同一 SHA：

```sh
ISSUE1719_VERIFIED_SHA="$(git rev-parse HEAD)"
NC_REPO=/Users/silencehan/Projects/NewChanlun-sandcastle-host/.sandcastle/worktrees/sandcastle-issue-1719 NC_TARGET=sandcastle-issue-1719 ~/.local/bin/nc-remote "$ISSUE1719_VERIFIED_SHA" rust cargo check --locked --lib
NC_REPO=/Users/silencehan/Projects/NewChanlun-sandcastle-host/.sandcastle/worktrees/sandcastle-issue-1719 NC_TARGET=sandcastle-issue-1719 ~/.local/bin/nc-remote "$ISSUE1719_VERIFIED_SHA" rust cargo test --locked --lib i4_bridge_
NC_REPO=/Users/silencehan/Projects/NewChanlun-sandcastle-host/.sandcastle/worktrees/sandcastle-issue-1719 NC_TARGET=sandcastle-issue-1719 ~/.local/bin/nc-remote "$ISSUE1719_VERIFIED_SHA" rust cargo test --locked --lib i4_bridge_channel_switch_default_equivalence
NC_REPO=/Users/silencehan/Projects/NewChanlun-sandcastle-host/.sandcastle/worktrees/sandcastle-issue-1719 NC_TARGET=sandcastle-issue-1719 ~/.local/bin/nc-remote "$ISSUE1719_VERIFIED_SHA" rust cargo test --locked --lib theta_v0::classifier::nest_lifecycle::tests::f
```

`i4_bridge_` 应执行 14 项，单独逐位回归应执行 1 项；旧 `tests::f` 过滤器覆盖既有 feed/channel_switch 夹具，不接受 0 项。核日志精确 SHA、命令、退出码及非零测试数。不跑全量回放或重型验证。

格式检查及 clippy 也待宿主远端运行。只针对本票 Rust 文件运行 rustfmt，禁止全仓格式改动；若有格式差异，宿主取回补丁后重新封包并重新固定 SHA。当前未宣称格式检查或 clippy 通过。远端 clippy 命令为 `cargo clippy --locked --lib --tests`，需按新增差异判断警告，不把仓内历史警告归入本票通过声明。

远端测试后还需由宿主对最终精确 head 派非作者独审。该独审与 CI 均未交付。

## 未交付项和边界

1. 正式 S 的 Broken producer、具名 kind/解码、发布对象引用闭包和真实 S 已发布输入。此处只接既有事件机组件，未改 S producer 或公开 event/checkpoint schema。
2. B 原子持久事务、正式 checkpoint、崩溃窗口及新进程恢复。book 内存事实留档、应用去重和 Clone 均不能证明持久恢复。
3. 正式 launcher 的回放/paper 同核消费、finance 业务接线和真实账目输出。未改 launcher/finance，也没有以旧 p123 sidecar 或合成 fixture 代替它们。
4. 当前 WIP 的 Rust 编译、非零测试、最终精确 head 的非作者独审和 CI 结果。按本轮安排由宿主封包后补齐，源码交棒不等于上述验收已通过。

本轮没有发现需要额外授权才能继续完成源码组件的卡点。`ready` 仅表示本轮限定组件源码可交宿主封包验证，整票 #1719 仍未完成且不得关票。

## 返修 1

2026-11-01，基于分支 `sandcastle/issue-1719`、HEAD `8e51ce9f54cd25a174dfd472311527db422ed269` 修复 CI 编译错误。已回查 GitHub Actions run `41268966878` 的原始日志 `/Volumes/AgentStorage/i4-managed-resume42z/ci-8e51ce9f/job-124737588633.log`，日志第 549–554 行报：

```text
error[E0614]: type `nest_lifecycle::key::LifecycleKey` cannot be dereferenced
   --> src/theta_v0/classifier/nest_lifecycle/book.rs:719:54
719 | audit.settled_key == *key && audit.fact.known_at() == invalidated
```

修改 `rust/src/theta_v0/classifier/nest_lifecycle/book.rs:719`，将 `audit.settled_key == *key` 改为 `audit.settled_key == key`。该循环遍历 `self.ledger.values()`，随后以 `let key = entry.key` 取得实现了 `Copy` 的 `LifecycleKey` 值。两侧现在直接比较同类型值，仍要求审计结算键与当前条目键相等，且事实 known-at 等于失效时点，断言语义不变。

人工逐行复核范围为 `git diff 7b5cde50 HEAD -- rust/` 的全部新增或修改代码，共 7 个文件；同时核对本次单行修复及相关被调用类型和 API。生产代码与测试源码另经只读子代理分别交叉复核：

| 文件 | 复核内容与发现 |
|---|---|
| `rust/src/theta_v0/classifier/nest_lifecycle.rs` | 重导出、嵌套测试模块与 include 路径、父模块导入及私有项可见性，未发现其他确定编译问题。 |
| `rust/src/theta_v0/classifier/nest_lifecycle/book.rs` | 绑定注册、事实留档、目标收集、清账、迁移、不变量及内嵌缺账测试。修正上述值的误解引用；其余未发现确定编译问题。目标先收集为拥有键和桥副本的 Vec，再修改账本；注册时 ledger 与 bridges 的借用分属字段。 |
| `rust/src/theta_v0/classifier/nest_lifecycle/center_break.rs` | 事件匹配、值复制、排序键、derive 与字段可见性。核对 CenterLifecycleEvent 的 Copy/PartialEq/Eq、CenterId 和 LifecycleKey 的 Ord，以及各载荷的 Debug/Clone/PartialEq；未发现缺失 trait。新增容器使用 BTreeMap/BTreeSet，不要求键实现 Hash。 |
| `rust/src/theta_v0/classifier/nest_lifecycle/entry.rs` | 新增原因码分支、枚举匹配及 invalidate 调用签名，未发现确定编译问题。 |
| `rust/src/theta_v0/classifier/nest_lifecycle/feed.rs` | Live 匹配所得引用、From 转换、闭包对 book 的可变借用、迭代器返回值及集合推断，未发现确定编译问题。 |
| `rust/src/theta_v0/classifier/nest_lifecycle/pan.rs` | provider 包装、FnMut 捕获与借用结束、PanLiveWindow/Segment 的 Copy、选中窗口传递后再使用，未发现确定编译问题。 |
| `rust/src/theta_v0/classifier/nest_lifecycle/i4_bridge_tests.rs` | 全部 440 行，含 13 个测试与辅助函数。逐项核对方法、字段、父模块夹具、嵌套可见性、值与引用比较、部分移动后的字段访问、不可变借用末次使用后再修改 book、断言所需 trait，未发现确定编译问题。另一个新增测试位于 book.rs，合计仍为 14 个。 |

同时检索 `rust/src`、`rust/tests` 中被更换接口的调用与结构体构造，未发现遗留旧签名调用；核查导入使用及相关 lint 声明，未发现新增的确定未使用导入问题。除已定位的 E0614 外，本轮人工复核没有发现其他确定编译问题，因此未扩大代码修改范围。

返修验证：`git diff --check` 退出 0。本机未运行 cargo/rustc/rustfmt，Rust 测试实际执行数为 0，类型检查、借用检查、测试编译及执行仍待宿主封包后由 CI 验证。人工复核不能替代编译通过证据。本轮未执行 git add/commit/push，未修改 Git 元数据。
