# recursive_t 已退役

**状态**：DEPRECATED（已废弃，仅保留历史参考）

**退役日期**：2026-10-17

**退役原因**：

1. **编译失败**：存在 8+ 处导入错误，无法编译通过
2. **架构演进**：已被 `theta_v0` 替代（828 处活跃引用）
3. **维护成本**：与当前代码库架构不兼容

**已移除组件**（2026-10-17）：

- `backtest.rs`（644 行）
- `backtest_run.rs`（774 行）
- 相关导入已注释（`center.rs`, `rec_stream.rs`, `t_engine_run.rs`）

**剩余文件状态**：

- 保留作为历史参考
- 不再维护或修复
- 后续清理将由 Destination ③ 的完整退役票推进

**相关决策**：

- 审计产出：`.chanlun/review-results/goal1323-resume-20261017/old-engine-retirement-20261017.md`
- 退役计划：`.chanlun/review-results/recursive-t-retirement-plan-20261017.md`
- 归属目标：#1323 Destination ③（旧引擎退出）

**迁移指引**：

所有新开发应使用 `theta_v0`（位于 `rust/src/theta_v0/`）。
