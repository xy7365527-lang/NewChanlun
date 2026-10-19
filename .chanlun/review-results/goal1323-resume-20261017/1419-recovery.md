# #1419 只读恢复核查（2026-10-17）

范围：`.sandcastle/worktrees/resume-1419`，HEAD 018c6542e5；未运行 cargo，未读取外盘，未修改工作树。已读 CLAUDE.md 与 Serena initial_instructions。图索引无对应 source_audit 节点，按规则回退精确文件读取。

## 1. 真实夹具可直接恢复

Git 全 refs 的指定路径历史命中 `ecc9b196d3`：

`git show ecc9b196d3:s_session/tests/fixtures/tb02e-real/btc-tail50k-first600.json`

Git blob：`f06b63cc8e83db7c785b963f0bf10ed566471ec8`。
SHA256：`0df2091a8723b040988527bcd05612c9643530f9724be68edb52acbcf77c108a`。

包含原 schema、ticket、ruling、provenance；provenance 明示 BTC 尾50000前600、start_offset=5、1419 9cd64da5cd + 1405 199b820c9d 组合来源。原件说明此夹具锁 S 输入合同对 CC-013 的读法，不锁 parser。可以原字节恢复；不必读取外盘，更不能换成合成夹具。

## 2. tb02e 的编译兼容缺口

`rust/src/bin/s_session_v2/tb02e.rs:1227` 仍调用 `parse_layer_with_inclusion_facts`；`:1236` 的 `tb02c::build` 缺第10参数并把 Result 当 tuple。

正确接口已在 `rust/src/theta_v0/parser/mod.rs:279`：`parse_layer_with_inclusion_facts_store(bars, config, SecondFoldStore) -> (layer, facts, store)`。
`SecondFoldStore::new(round_budget)` 在 `parser/second_fold.rs:590`；`tb02c::round_budget()` 返回 u32，仅是预算而不是 store。

该测试应使用 `SecondFoldStore::new(tb02c::round_budget())` 进入同次 parser，取回该次返回的 store，作为 `tb02c::build(..., &store)` 的第10参数；显式 unwrap/expect 处理测试 Result。不能单在 build 调用处塞一个未参与 parser 的空 store。每个 frozen prefix 是独立全量输入，因此每个 prefix 自建 store 合理。

## 3. 两负例与 extreme_roots 的关系：不能据静态读数认定根因

目标测试当前行号：
- source_audit.rs:3825 `valid_merkle_shape_dependency_forgery_is_rejected_after_none_subset`
- source_audit.rs:3870 `valid_merkle_reidentified_group_high_still_breaks_shape_identity`

两负例及其直接生产验算不引用 extreme_roots。CC006 refs 在生产与 frozen oracle 都严格为3字段：merged_index/raw_refs/dependency_refs（source_audit.rs:1878附近；source_audit_oracle.rs:342附近）。因此测试替换字段列表也是3字段，并未静态发现“漏保留新增 extreme_roots 字段”。撤回可能把该字段直接添加进 shape refs 的猜测。

`extreme_roots` 属于笔 endpoint 内容（content_bi.rs:1772，tb02b.rs:296等），不是该 CC006 shape 身份字段。shape 身份配方在 content_producer.rs:1130，消费 branch、方向、窗口、highs/lows、comparisons、input_refs、rule、profile。

第一个负例故意令 dependency_refs = raw_refs，预期在 source_audit.rs:1932 拒绝。第二个重算修改后 CC005 typed ID，预期在 source_audit.rs:1995 的 shape 身份校验拒绝。公共辅助函数 source_audit.rs:3782先验证 Merkle，再比较普通路径、frozen oracle 与 metered 路径的首错；既可能失败于首错不一致，也可能是此前步骤提前拒绝。缺实际失败 stdout，不能将其中任一可能写成已定位根因。

当前 source_audit.rs:293 使用正确 `V::Bool(*x)`，没有摘要中遗失补丁的 `{"__bool": ...}`。将 Bool 改成对象会改变规范字节与身份，是非法语义近似；不可恢复该做法。

后续需要在兼容构建修复后准确执行这两个具名测试，保留实际选中数及首错 stdout，再决定修改生产还是负例。此报告不冒充运行验证。
