# #1597 独审后尾部快速轴评审

- 评审者：claude-opus-5-5（本机 claude CLI，经 CLIProxyLocal；独立会话、非作者；只读工具）
- 发起：主控合入代理 /root/merge_train，2026-10-21
- 被审：`775172dc44`、`63aef907b8` 与合并提交 `648012957a` 的冲突解法（独审原审至 `0f747b938c`）
- 结论：APPROVE

---

独审之后新增的三处尾部都没发现阻塞问题。只有两条 LOW，都在探针 bin 里，不影响生产代码。我没跑 `cargo test`，只做了读 diff 的核对，结论是静态审查得出的。

## (a) 775172dc44：留存锁计入扁平展开
- 改动很小：在 `shared_full_facts_retain_subquadratic_nodes` 里，凡是 `as_shared().is_none()`（即扁平形态）的坐标集合，都把元素数累加进 `flat_elems`，最后断言它等于 0。这正好补上独审 M-1 指出的漏洞：以前测试的 `walk` 遇到扁平集合直接跳过，所以就算完整事实路径退回扁平展开，节点数的上限检查也照样能过。
- 空集合不会误报：`c.len()=0`，不管是哪种形态都累加 0。
- 断言的位置在所有 `visit` 调用之后、Θ(n²) 和节点上限断言之前，顺序没问题。
- 没有发现问题。

## (b) 63aef907b8：探针 group-evidence 模式
- 逻辑正确：`window()` 先把 PEAK 重置到当前存活量，再逐 bar 调 `IncrFactsState::append`。每 `every` 根调一次生产入口 `parse_layer_with_group_evidence`，末根那次单独调、不进滚动摘要，所以 `i + 1 < bars.len()` 这个条件能防止末根被重复调用。每次的结果用完即丢，摘要覆盖 layer、groups、stroke_facts、segment_facts 四项。
- **LOW-1**：文件头 `//!` 的「模式」清单和 `main` 里的用法字符串都没加 `group-evidence`，`layer` 也没加（这是 0f747b938c 就留下的）。纯文档滞后。
- **LOW-2**：这个提交单独看没过 fmt（`rolling.update(...)` 一行超长）。合并提交 648012957a 已经修好，属于记录性质，不用另行处理。

## (c) 合并提交 648012957a 的冲突解法
- **stroke.rs**：main 侧（merge-base `df6e680930` → `^2`）对这个文件只改了一处，就是 #1596 在 `fn sorted` 里加的 `xs.shrink_to_fit()` 和注释，没有别的语义改动。合并结果和本分支 `^1` 的这个文件完全一样。丢掉 shrink 的理由成立：`From<Vec<usize>>` 走 `Rc::from(Vec)`，会按精确长度新分配一块再拷贝，原 Vec 的空闲容量随之释放，留存的内存里不会有多余容量。#1596 收缩容量的目的仍然达到。
- **probe**：已核实 main 侧这个文件和 `1bc3534014` 字节相同。`git diff ^1 648012957a` 在这个文件上只有两处格式改动：一是 `inclusion::IncrFactsState` 和 `inclusion::{...}` 两条 use 调换了顺序（rustfmt 排序），二是上面说的超长行折行。没有语义改动。
- **其余文件**：`git diff ^2 648012957a` 列出的 10 个文件，全都在本分支自 merge-base 以来改过的范围内。其中除 probe 的格式修整外，合并结果都和 `^1` 完全相同。两侧都改过的文件里，Cargo.toml、parser/mod.rs、backtest/incremental.rs 等在合并结果里和 main 一致；`s_structure_session.rs` 只多出本分支的 `.to_vec()` 显式展开，main 的 #1596 内容此前已经由 `e7f309be83` 并入。没有发现丢失 main 侧语义改动的情况。

VERDICT: APPROVE
