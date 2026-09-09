> 阅读版：仅转换链接；原始独评字节与 SHA 见验收索引。

# #1370 TB-01-A：目录候选静态增量审查

结论：**PASS_STATIC_INCREMENTAL_SCOPE，0H / 0M / 0L**。候选修复了已签目录对象分支被拒绝的问题，并使 Python 对所有 metadata 行采用与 Rust 相同的文本键、文本值约束。耐久实现和原有 8 个测试逐字未变。本结论只覆盖此候选差异。

基线：`e79c74a17a800e660b515e1f88fc9a5706826b42`。候选 Rust SHA-256：`1820c9a464df80b22fe50233905fba5fc959d50936000fd800f6e198493508be`；Python：`63203e75dbf58a3d59ed5b0fe7a15d5eb1101be9e19945b4d47ad00f0e3e95a9`。已核 `candidate.json` 的基线和两份源哈希，且 Rust 基线与此前独立保存的 e79 源一致。

## 目录内容与类型

从固定提交读取的 `s_session/catalog/signed-catalog.json` SHA-256 为 `938b0ef59282e689c114cdcb211e2709c86e64c618e4ec0573862bda43508069`。实际为 116 条、116 个唯一 ID；`branches` 有 82 个数组、34 个对象。全部 id/kind/title/domain 均为字符串。

Rust [JsonShape](../UNBUNDLED-REFERENCES.md#ref-1) 与 Python [branches 校验](../UNBUNDLED-REFERENCES.md#ref-2) 都接受数组或对象，成功后直接返回完整解析值，未筛项、展平或替换嵌套内容。Rust 初始化对每个 `branches` 克隆并存成 JSON，查询读取该值；这段初始化逐字未变。其余形状约束保持原义：scope/evidence/detail 为对象，comparisons/input_refs/raw_bars 为数组。

这里的“116 条保真”指现有目录投影字段及每条 branches 的完整嵌套 JSON 值。已签源另有 acceptance_mode、axis_ids 或 criterion 字段；现有查询投影不导出这些字段，本候选未改变这一点。

## Metadata 与耐久边界

Python [meta_dict](../UNBUNDLED-REFERENCES.md#ref-3) 对 SELECT 返回的每一行先检查 `type(key) is str` 与 `type(value) is str`，再写入结果映射。其范围包括额外未知 key，能阻断 BLOB/NULL 等非文本进入成功响应；与 Rust [db_meta](../UNBUNDLED-REFERENCES.md#ref-4) 的 `String/String` 读取相符。缺少某个键的既有兼容行为没有改变。

Rust 新 JsonShape 之前的完整源码前缀共 61083 字节逐字相同，SHA-256 `3684e48ded04b07f4df7ef7dc14489f1ddbc6667333ad070d469649ba0229fb1`。这覆盖 AcceptInput 的 epoch fence/profile 重放、Begin、取消 CAS，以及先持久完整 batch、再核对 ownership 与字节并发布可达根的调用顺序。关键位置为 [persist_batch_before_publish](../UNBUNDLED-REFERENCES.md#ref-5)、[cmd_advance](../UNBUNDLED-REFERENCES.md#ref-6) 与 [cancel_own_begin](../UNBUNDLED-REFERENCES.md#ref-7)。此前 e79 耐久报告的作用域没有被本差异改变。

## 测试归属与验证限制

原有 8 个测试逐块字节相同；每个测试的 SHA-256 收录在 [JSON 证据](../payload/inputs/TB01-A-CATALOG-FIX-REVIEW.json)。新增 [第 9 个测试](../UNBUNDLED-REFERENCES.md#ref-8) 使用真实 signed fixture，经过 cmd_init/read_catalog_in_tx，检查 116 与 82/34、逐 ID 的 branches/title/domain 同值和初始 implementation_status，并拒绝标量、null 与坏 JSON。测试没有单独断言 kind/catalog_revision/其他初始状态；对应实现未改，已作静态核对。

本次没有执行新增测试、编译或运行候选 native，没有 HTTP/浏览器/Node/SQLite 运行验证；没有修改 Q/N、仓库或服务。候选冻结后的增量运行验证由根及 Python 复核线继续。此前 [e79 耐久结论](../payload/inputs/tb01-post-repair-durability/REVIEW-e79.md) 仅在其既定范围内引用，此报告不替代正式独审或整票验收。
