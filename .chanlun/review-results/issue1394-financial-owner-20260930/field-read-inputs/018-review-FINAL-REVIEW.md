# #1394 C 局部字段批读取：独立 Spec + Standards 评审

审查对象：`NewChanlun-1394-field-audit-20261002` 的单文件未提交差异 `rust/src/bin/s_session_v3/source_audit.rs`。固定 HEAD 为 `dc425df7b7bbe98d16fe3bb963a7b3e530e5f6f9`，其 Rust tree 为 `c60452f606202ed6b4754ce29bd17698d115ed32`；冻结文件 SHA-256 `aa5ee2c0261e7487446c9991559e028d87475a405c741fe5b331b74b0c11e3b5`、patch SHA-256 `f5d9348d991adf0b9b17635596537a9299b27ec0d91504ff599bb7936eed2f31` 均经独立复核一致。暂存区无差异。

## 代码审查报告

### 概念层质询结果

**通过（限本次 S-content 来源审计读取优化）。** 增量没有修改笔、线段、中枢、级别或买卖点判据，也没有新增教义定义。受影响的是已存在的 `CC005` 组事实、`CC006` 形态来源及见证与同 cut 原件的绑定读取；业务比较仍用原来的整数、文本、来源差集、身份和见证判据。因此 `.chanlun/definitions/`、`docs/spec/`、`缠论知识库.md` 中没有被此单文件差异改写的缠论概念条件，未发现需要转入 `.chanlun/genealogy/` 的概念矛盾。

反例重点是合法 Merkle 图上的错类型、缺字段、伪造 `dependency_refs`、改变 `high` 后重算 typed 身份，以及见证合并值错配。新 `scalar_value` 逐字保留来源审计原有 Number/String/Bool/Null 可渲染标量范围；`Bool(true)` 后续仍不能通过 `.as_i64()` 或 `.as_str()`，不会隐式当作数值 1 或文本。新批读只先取固定字段的 `ContentRef`，后续实际渲染、类型和业务判据仍按原调用点执行；上层完整 `sc::verify` 仍先于来源审计。没有严格判据失败后切换宽松判据的路径。

### Spec

**PASS，限单文件局部读取契约。** `budget=None` 的既有 `ArrayAuditSource` 开关仅在三个小局部读取固定字段：part 的 `merged_index/raw_refs/dependency_refs`，组载荷的 `high/low`，见证绑定的 `high/low/open/close`。`Fields::read_subset` 使用已有规范对象字段序列读取，并返回按键名取得的原 `ContentRef`。每个 part 或 payload 只保存本次局部字段引用，不跨 record、cut、source 或事务复用。`budget=Some` 时 `local_fields` 直接返回 `None`，三个位置继续旧逐字段 `field/number/text` 路径及业务交错顺序。

已读执行工位的定向结果：11 个相关测试通过；Some 完整夹具读取 `151351`、F 调用 `170797`，读取摘要和预算轨迹与固定基线相同；错型、预算前门与期限先于 SQL 的反例通过。局部 None 读取示例分别为 `118→50`、`227→174`、`400→175` 次；它们证明这三个小字段路径减少读取，不证明全源、整服务或 9011 的收益。本工位独立核过源代码分流及 `git diff --check`，没有再启动占内存的 Cargo 编译。

### Standards

**PASS（此增量）。** 未见超过 80% 把握的新增 CRITICAL、HIGH 或 MEDIUM 缺陷。复用现有 `Fields::read_subset` 和 S-content 路径实现，没有改公开 wire、持久 schema、业务规则或现有验证闭包。新增测试较长，但均在原文件的测试模块中，覆盖本次三处生产读取及有预算原门序；没有据行数单独判定缺陷。冻结文件和 patch 绑定、未暂存差异、`git diff --check` 已核。

标准 Clippy 原配置仍受未修改的 `state.rs:60` 既有 `clippy::absurd_extreme_comparisons` deny 阻断；不能声称全绿。执行工位报告了本文件靶向编译和测试通过，本工位未独立重复构建。

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | pass |
| LOW | 0 | note |

**结论：PASS（冻结单文件的局部代码评审）。** exact pending 四成员 release 对照仍由管理工位独立核验；本报告不把局部读取减少称作真实服务、9011、S/Q、资源或 #1323 整图验收通过，也不授权合入 main。
