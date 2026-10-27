# #1419：共享 typed 单元输入合同

状态：单文件抽取完成，等待主控整合与独审；未编译、未运行 Rust 测试，不是 #1419 或全图验收。

工作树：`/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1419`。
唯一修改源码：`rust/src/bin/s_session_v2/tb02e.rs`；主控此前同次 parser store 的测试修复保留（现约第 1443 行）。
源码 SHA256：`9701f93d114eb62a8321cb558b6117a50b6653a0c81d7d7666fe5aedef369b60`。
补丁：同目录 `1419-units-extraction.patch`（相对恢复树 HEAD，包括先存的主控测试修复，不应重复 cherry-pick）。

## 实际接口

全部 `pub(super)`，位于 `tb02e`：

- `SegmentStrokePrices`（98 行）：`stroke_index:i64, start_price:i64, end_price:i64`。
- `CompletedSegmentUnit`（106 行）：`end_stroke:i64, start_price:i64, end_price:i64, member_stroke_indices:Vec<i64>, stroke_refs:Vec<SegmentStrokePrices>`。
- `SegmentUnitRow`（117 行）：`unit_id:String, object_id:String, start_stroke:i64, direction:String, sources:Vec<usize>, completed:Option<CompletedSegmentUnit>`。
- `units_from_segment_rows(Vec<SegmentUnitRow>, &[Segment]) -> Result<UnitInputs,String>`（227 行）。
- 原 `units_from_segments(&[Value], &[Segment])`（213 行）保留，只适配 JSON 字段，然后调用同一个 typed 核。

桥工位已确认从同次 `SegmentCandidateFact`、`StrokeFacts` 及本次 `TypedObject` 交付；本补丁不改桥文件、不生成任何生产实体 ID 或 object ID。

## 保留的合同

完成段极值仍按成员索引逐一查 `stroke_refs`，缺笔或空成员拒绝；非成员笔不进入极值。几何起止价只用于方向比较，不能代替成员极值。

候选按起笔排序，除末个外不得进行中；完成段以及末尾进行中段的连续性原检查保留。完成数量、lo、hi、方向继续与 `l0_units(layer_segments)` 逐位对比，返回同一份塔 UnitRange（含其 start/end 索引）。mismatch 的 check/position/expected/actual 收据结构保留。

真实单位身份、对象身份和来源数组在 typed 核原样传递；本层没有独立查询来源真实性的能力。旧 JSON adapter 原有非数字来源字符串 `filter_map` 行为未改，不把本次抽取冒称来源认证修复。新的直接事实适配必须从真实笔端源坐标填行。

JSON 成员索引现在须能解析为 i64 文本以进入 typed 行；生产 wire 本来就是十进制文本。不复用旧 Value 的任意类型等价性。JSON 成员引用仍先按原 wire 值精确匹配，避免不同文本归一为同一数字后错误填补缺失引用。

## 新增测试源码与验证

新增 2 个测试（未运行）：

1. `tb02e_typed_units_preserve_member_extremes_identity_sources_and_progress`（1346 行）：乱序输入恢复排序，成员极值与几何端点不同且非成员极值极端，检查输出整份 UnitRange、真实身份/source 原样传递及进行中单元。
2. `tb02e_typed_units_reject_missing_members_and_preserve_contract_guards`（1373 行）：缺成员、空成员、方向、连续性、内部进行中、塔区间和计数负例。

原真实 BTC 夹具合同测试与冻结 raw 前缀非真空测试保留；未伪造测试运行或新真实数据。

已执行：单文件 `rustfmt --edition 2021 --check` exit 0；`git diff --check -- rust/src/bin/s_session_v2/tb02e.rs` exit 0。按授权边界未执行 cargo、服务、外盘访问、commit 或 push。

下一步由桥工位使用上述类型；主控安排独审与具名远端源码验证。fmt 仅证明语法可解析和格式，不证明类型检查或行为通过。
