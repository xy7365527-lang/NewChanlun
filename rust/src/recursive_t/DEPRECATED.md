# recursive_t 退役标记（Phase 1）

- 退役范围：本目录的统一递归算子与旧递归流测试面。
- 退役依据：#1323 / SPEC #1340 Destination ③；相关裁定见 #799、#804、#812。
- 当前阶段：Phase 1（隔离标记与测试盘点）。本阶段不删除实现，也不改变运行语义。

## 替代路径

新代码应使用 `theta_v0::classifier` 的生产分类路径；交易编排和持久化使用现役的 `fugue_v3` / trading owner。需要历史回放时使用 D4 replay owner，而不是新增 `recursive_t` 调用点。

## 测试处置

`recursive_t` 内嵌测试仍覆盖现役 PyO3/流式公开接口，其中重型 backtest 诊断依赖本目录的数据加载器。当前没有经过同输入对拍证明的等价替代 API，因此本阶段保留这些测试并记录为“待迁移”，不伪造迁移结果。独立对拍 harness 位于 `rust/tests/parity_recursive_flat.rs`，继续作为隔离边界证据。

## 后续条件

Phase 2 先为每组诊断确定替代输入、输出摘要和对拍证据，再迁移或废止测试；Phase 3 才能删除实现。任何新增依赖必须先开票并通过 Wayfinder 评审。
