# #1394 C 完整数组线性审计：独立 Spec + Standards 评审

审查对象为 `NewChanlun-1394-linear-audit-20261002` 固定 HEAD `e4ea36fbd16c3784b846092b6c28b976ff6343a5` 上的五个生产文件及一个新测试文件。Rust 基线 tree 为 `7196a77fef56cf87cbe91dc4b135326a142e2395`，候选虚拟 tree 为 `c60452f606202ed6b4754ce29bd17698d115ed32`，差异 SHA-256 为 `8e67ee4fe48625e61e83a6bb56fe18743774784df40c908db7cfe554222034c9`。本评审核对了 `CANDIDATE-BINDING.json` 的六个文件 SHA-256，均与冻结工作树一致；暂存区无差异，工作树仍未提交。

## 代码审查报告

### 概念层质询结果

**通过（限本次 C 内容审计性能补丁）。** 变更只处理 S-content 规范数组读取及持久内容审计，没有新增笔、线段、中枢、级别或买卖点等缠论判据；核对 `.chanlun/definitions/`、`docs/spec/` 与 `缠论知识库.md` 后，未找到被这六个文件修改的教义定义。无需以生产代码反向裁定教义，也无新矛盾需要进入 `.chanlun/genealogy/`。

反例核对聚焦实际受改的内容协议：根 `ContentRef` 的 kind、extent、format 与摘要；0、1、非二幂及较大数组的规范 k 分裂；子树期望叶数；非序列叶的 Value tag/extent；重复摘要的每个逻辑位置；损坏 F 与缺失原件；`JsonInner` 0x07/0x08 与公开/模板闭包。`lookup_array_items` 逐节点调用原 `read_seq_node`、逐叶调用 `read_checked`，不以摘要复用真实读取；上层 `verify`/`verify_template` 与业务判据仍在原调用链。它返回裸 `ContentRef`，并不携带 JsonInner 上下文；原上下文路径和完整闭包仍承担对应拒绝。未见同一生产判断在失败后转向宽松判据。

### Spec

**PASS，局限于冻结候选的局部契约。** 完整数组枚举对规范序列做迭代 DFS，按逻辑顺序产出 `ContentRef`，不展开成员子图，也不保存跨 source、cut 或事务的 body/proof。`NodeSource::audit_full_array_scan` 默认关闭；历史审计仅 `read_indexed_history_for_audit` 且 `budget=None` 开启，来源审计在原 `sc::verify` 后且 `budget=None` 开启；有预算分支、普通读取、`lookup_path` 与 fragment 保留原逐 Index 路径。`object_index` 两处仅在显式开关打开时批量取完整数组，原字段和业务检查仍执行。

定向证据：新增 4 个 array-scan 测试与既有 11 个 path 测试覆盖 Ref/order/kind/extent、0/1/3/7/16/65/257 项、错误 k/count、非法叶、损坏与 missing；SQLite 65 项旧 1607 次、新 195 次节点读取，并在同事务改坏或删节点后拒绝。Some 分支定向对拍了结果、SQL、F、预算事件及期限先行门序。执行工位记录的 58 个定向测试为 58 通过、0 失败；本工位未另行重复编译。其提供的 27 cut、28 raw、1 AcceptedPending 只读副本上，基线与候选各三次完整审计均成功且结果摘要相同；节点 SELECT 每次 `8,792,023 → 8,090,429`（减少 `701,594`，约 7.98%）。该计数不是全部 SQL 或端到端延迟。

### Standards

**PASS（补丁质量），保留一个既有检查限制。** 已查 staged 与未暂存差异及周边代码；未见超过 80% 把握的新增 CRITICAL、HIGH 或 MEDIUM 缺陷。遍历使用显式栈而非递归，叶 body 只在当前读取中使用；重复摘要仍按位置重新读取。六文件 `git diff --check` 通过，冻结哈希与候选绑定一致。执行工位记录的靶向格式、编译和测试检查通过。标准 Clippy 曾被未修改的 `state.rs:60` 既有 `clippy::absurd_extreme_comparisons` deny 阻断；其单独屏蔽该旧 lint 的差异诊断通过。这不构成该补丁新缺陷，也不能宣称原配置下 Clippy 全绿。

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | pass |
| LOW | 0 | note |

**结论：PASS（冻结六文件的局部代码评审）。** 尚无 RSS/heap 峰值、真实服务、9011 完整重放、S/Q 或待恢复发布的通过证据；本结论不替代这些验收，也不表示 #1323 整图完成或允许合入 main。诊断 probe 与 release 对照不在本次生产候选六文件内。
