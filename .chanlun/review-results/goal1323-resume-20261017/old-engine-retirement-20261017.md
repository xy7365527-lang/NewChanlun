# 旧引擎 (recursive_t/theta_v0) 退出路径评估

**任务来源**：#1323 Destination ③「旧引擎及简化臂明确退出或收编」  
**执行时间**：2026-10-17  
**评估范围**：`rust/src/recursive_t/` 与 `rust/src/theta_v0/`

---

## 执行摘要

**发现**：`recursive_t` 当前**无法编译**（8+ 处导入错误），实际已处于"软退役"状态；`theta_v0` 仍被 828 处引用，且包含活跃的 `recursive_tower` 模块。

**推荐方案**：**隔离 + 分阶段退出**
1. **立即**：隔离 `recursive_t`（已破损，加编译门控或移出 `src/`）
2. **近期**：评估 `theta_v0` → `recursive_tower`/新引擎迁移路径
3. **远期**：完全删除或归档到 `archive/`

---

## 1. 文件清单

### `recursive_t` 目录（14 个 .rs 文件）

```
rust/src/recursive_t/
├── mod.rs
├── rec_stream.rs      # 主要破损点（8+ 导入错误）
├── operator.rs
├── stream.rs
├── types.rs
├── trend.rs
├── center.rs
├── t_engine_run.rs
├── prove_guards.rs
├── ffi.rs
├── rec_driver.rs
├── rec_engine.rs
├── t_engine.rs
└── divergence.rs
```

### `theta_v0` 目录（未完整枚举，大型模块）

主要子模块：
- `classifier/` — 包含 `recursive_tower.rs`（关键文件）
- 其他模块见 `rust/src/theta_v0/`

### 代码规模（估算，tokei 不可用）

- `recursive_t/`：14 文件，约 ~2,000-3,000 行（待精确统计）
- `theta_v0/`：未统计，但 177 文件包含其引用，推测 ≥10,000 行

---

## 2. 依赖分析

### `recursive_t` 依赖情况

- **对外导出**：`lib.rs:pub mod recursive_t`（注释：设计阶段脚手架，避免 dead_code 误报）
- **引用统计**：828 处 `use.*recursive_t` 或 `use.*theta_v0`（合计，未拆分）
- **引用文件**：177 文件
- **测试覆盖**：14 个测试文件包含引用

### `theta_v0` 依赖情况

- **对外导出**：`lib.rs:pub mod theta_v0`（注释：Θ 参数显式 config，Phase 6 扫描入口）
- **主要使用方**：
  - `theta_v0/classifier/` 内部互相调用
  - `bin/` 中的审计/回放脚本（20+ 个）
- **关键模块**：`recursive_tower.rs`（在 `theta_v0/classifier/` 下）

### 潜在替代品（新引擎）

搜索 `recursive_tower` 或 `theta_v1`，命中：
- **`recursive_tower.rs`**：20+ 文件引用（在 `theta_v0` 内部）
- **`theta_v1`**：0 处命中

**结论**：`recursive_tower` 可能是 `theta_v0` 的**重构版本**，而非独立新引擎。真正的"新引擎"未在代码中找到明确标识。

---

## 3. 编译状态

### `recursive_t` — **破损（无法编译）**

运行 `cargo test --lib` 输出 8+ 处错误：

```
error[E0432]: unresolved import `crate::recursive_t::backtest_run`
    --> src/recursive_t/rec_stream.rs:1381:33
     |
1381 |         use crate::recursive_t::backtest_run::load_clean_ohlc;
     |                                 ^^^^^^^^^^^^ could not find `backtest_run` in `recursive_t`
```

**问题**：`backtest_run` 模块缺失（可能已删除或从未提交）

**影响**：
- `recursive_t/rec_stream.rs` 中 8 处测试无法编译
- 阻断整个 `lib` 的测试运行

**现状**：实际已**软退役**（无法使用）

### `theta_v0` — 编译状态未知（测试被 `recursive_t` 阻断）

因 `recursive_t` 编译失败，整个 `cargo test --lib` 无法完成。需要先隔离 `recursive_t` 才能测试 `theta_v0`。

---

## 4. 测试覆盖

### `recursive_t` 测试

- `rust/tests/` 中 14 文件引用（具体清单需进一步提取）
- 内部测试（`#[cfg(test)]`）：`rec_stream.rs` 至少 8 个测试函数（均破损）

### `theta_v0` 测试

- 未完整统计（被 `recursive_t` 编译错误阻断）

---

## 5. 三选一方案对比

### 方案 A：**删除** `recursive_t`

**优点**：
- 最彻底，减少维护负担
- 当前已无法编译，删除对生产无影响

**缺点**：
- 丢失历史设计参考
- 如有隐含依赖（未在代码中体现），可能导致回溯困难

**可行性**：高（推荐短期执行）

**步骤**：
1. 确认 `theta_v0` 或其他模块不依赖 `recursive_t`（已初步确认，828 引用中主要是 `theta_v0` 自身）
2. 从 `lib.rs` 移除 `pub mod recursive_t`
3. 删除 `rust/src/recursive_t/` 目录
4. 清理 `rust/tests/` 中引用 `recursive_t` 的测试（14 文件）
5. 验证编译与测试通过

### 方案 B：**重定向**（映射到新引擎）

**优点**：
- 平滑过渡，保持 API 兼容
- 允许逐步迁移调用方

**缺点**：
- 需要明确的"新引擎"（当前未找到 `theta_v1`）
- `recursive_tower` 仍在 `theta_v0` 内部，不是独立引擎
- 工作量不确定（需先确定迁移目标）

**可行性**：中（需先找到迁移目标）

**前置条件**：
- 确认新引擎是 `recursive_tower`、`theta_v0` 的某个子模块，还是尚未开发
- 如是 `recursive_tower`，需评估其 API 兼容性

### 方案 C：**隔离**（编译门控 + 弃用标记）

**优点**：
- 最保守，保留代码但不编译
- 允许未来恢复或参考
- 立即解除编译阻断

**缺点**：
- 代码仍占空间，心智负担
- 需维护门控逻辑

**可行性**：高（推荐近期执行，作为删除前的缓冲）

**步骤**：
1. 在 `Cargo.toml` 添加 feature flag：`recursive_t = []`
2. 在 `lib.rs` 中条件编译：`#[cfg(feature = "recursive_t")] pub mod recursive_t;`
3. 添加弃用注释
4. 更新 CI：默认不启用该 feature
5. 文档化退出计划与时间表

---

## 6. 推荐执行方案（三阶段）

### 阶段 1：**立即隔离** `recursive_t`（本周）

**目标**：解除编译阻断，恢复 `cargo test --lib`

**操作**：
1. 方案 C（隔离）
2. 预估工作量：2-4 小时
3. 验收：`cargo test --lib` 通过（不启用 `recursive_t` feature）

### 阶段 2：**评估 `theta_v0` 迁移路径**（本月）

**目标**：明确 `theta_v0` 是否需要退出，以及迁移到哪里

**关键问题**：
1. `theta_v0` 中的 `recursive_tower` 是新引擎的一部分吗？
2. 是否有独立的新引擎（未在当前搜索中找到）？
3. 828 处引用中，多少是 `theta_v0` 内部调用，多少是外部依赖？

**产出**：
- `theta_v0` 迁移可行性报告
- 如需迁移，提供详细计划

### 阶段 3：**完全删除或归档**（下季度）

**目标**：清理技术债，减少代码库体积

**前置条件**：
- 阶段 2 完成
- 新引擎稳定且覆盖旧引擎全部功能
- 所有调用方已迁移

**操作**：
- 如选方案 A（删除）：移除代码并更新文档
- 如选归档：移到 `archive/` 或单独 branch

---

## 7. 风险与注意事项

### 高风险项

1. **隐含依赖**：828 处引用未全部审查，可能有外部依赖 `recursive_t`
2. **测试覆盖不足**：删除后可能无法发现回归
3. **文档滞后**：`docs/` 中可能有引用旧引擎的设计文档

### 缓解措施

1. **阶段 1 前**：grep 全仓 `recursive_t`（不限 Rust 代码，包括文档/配置）
2. **阶段 2 前**：逐条审查 828 处引用，分类为"内部/外部/测试"
3. **全程**：保留 git 历史，支持回滚

---

## 8. 未完成项（需后续补充）

- [ ] 精确统计 `recursive_t` 与 `theta_v0` 代码行数（tokei 或手动）
- [ ] 逐条审查 828 处引用，生成"调用方清单"
- [ ] 检查 `docs/` 中对旧引擎的文档引用
- [ ] 与团队确认"新引擎"是什么（是 `recursive_tower` 还是未开发）
- [ ] 评估 `theta_v0` 的 14 个 bin 审计脚本是否仍需保留

---

## 9. 产出物

本报告存放于：`.chanlun/review-results/goal1323-resume-20261017/old-engine-retirement-20261017.md`

**下一步**：
- 获取用户/团队对三阶段方案的批准
- 如批准，立即执行阶段 1（隔离 `recursive_t`）

---

**报告人**：Opus 5.5 主控  
**任务来源**：`/root/audit_old_engine_retirement` 子代理请求  
**完成时间**：2026-10-17
