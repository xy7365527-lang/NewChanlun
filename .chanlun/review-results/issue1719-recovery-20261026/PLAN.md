# #1719 center_break 桥接实施计划

**开始时钟**：2026-10-26T11:31:40+0800  
**基线 head**：`b1c0ba1a587b4a8133c3bfa97950ef3f8a1f9b2d`  
**工作分支**：`sandcastle/issue-1719`  
**实施工蜂**：claude-opus-5-5（唯一 #1719 实施 owner）

## 一、任务确认

根据 `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1719-ENDPOINT-TRACE26.md` 考古结论：

1. **事实产生端已存在**：`rust/src/theta_v0/classifier/center_lifecycle.rs` 的 `CenterEventMachine` 产出 `Broken` 事件，携带完整 `CenterId`（`start_index + zd + zg`）
2. **对象账本已存在**：`rust/src/theta_v0/classifier/nest_lifecycle/book.rs` 的 `NestLifecycleBook`，持有泛型 `LedgerBook`
3. **活窗产出已存在**：`rust/src/theta_v0/classifier/nest_lifecycle/pan.rs` 的 `provide_active_pan_live_windows` 产出 `PanLiveWindow`，但只留 `b_center_start`（不含 zd/zg）

**缺口**：三端点尚未连通。需要：
- 新增桥接绑定，在对象注册处保留精确 `CenterId`、原 `LifecycleKey` 和具名政策版本
- 新增 center_break 破坏事实输入到账本清账的路径
- 持久恢复能力（从事实重建匹配、清账及去重集合）

## 二、设计方案

### 2.1 数据形状

```rust
/// 中枢破坏事实（第二层清账输入）
pub struct CenterBreakFact {
    /// 破坏的中枢精确身份
    pub center_id: CenterId,
    /// 破坏发生的级别
    pub level: u32,
    /// 破坏证据（三类点）的源坐标
    pub breaker_source_index: usize,
    /// 破坏侧
    pub breaker_side: Side,
    /// 首次已知该事实的 bar
    pub known_at: usize,
}

/// 活假设对象绑定（注册时保留精确身份）
pub struct HypothesisBridge {
    /// 原活假设账本键
    pub lifecycle_key: LifecycleKey,
    /// 绑定的 B 中枢精确身份
    pub center_id: CenterId,
    /// 具名政策版本
    pub policy_version: String,
    /// 注册时钟
    pub registered_at: usize,
}
```

### 2.2 三个端点

#### 端点 1：事实产生（center_lifecycle → CenterBreakFact）

- 消费 `CenterEventMachine::step_classification` 产出的 `Broken` 事件
- 提取 `CenterId`、level、breaker 信息、事件的 known_at
- **不修改 center_lifecycle.rs**（已有完整信息）
- 在消费侧（`backtest/fill.rs` 或新增消费模块）将 `Broken` 事件转为 `CenterBreakFact`

#### 端点 2：对象匹配（PanLive + Book → HypothesisBridge）

- 在 `provide_active_pan_live_windows` 产窗处，读取完整 `Center` 快照（不只是 `start_index`）
- 注册新对象时，保存 `HypothesisBridge`（`LifecycleKey` + 完整 `CenterId` + 政策版本）
- 存储位置：`NestLifecycleBook` 新增 `bridges: BTreeMap<LifecycleKey, HypothesisBridge>`
- **精确匹配原则**：level + 完整 `CenterId`（含 zd/zg）+ 原 `LifecycleKey` 三重绑定

#### 端点 3：第二层清账（CenterBreakFact + Bridge → Invalidated）

- 新增 `NestLifecycleBook::apply_center_break` 方法
- 输入：`CenterBreakFact`（事实）+ `as_of`（消费 bar）
- 查找：按 `level` + `CenterId` 精确匹配 `bridges`
- 清账：只清匹配的原对象一次，产生 `Invalidated(CenterBreak)` 修订
- 新增 `InvalidatedReason::CenterBreak`（独立原因码，不复用 `ForceOvertake`）
- 幂等：同一事实重复输入不重复清账（去重键：`(center_id, breaker_source_index)`）

### 2.3 持久恢复

- `NestLifecycleBook` 新增持久化字段：
  - `applied_breaks: BTreeSet<(CenterId, usize)>` 去重集
  - `bridges: BTreeMap<LifecycleKey, HypothesisBridge>` 绑定集
- 重放时从真实 S 事实链重建：
  - 按原 known_at 顺序喂入 `CenterBreakFact`
  - 恢复对象注册时重建 `bridges`
  - 同一政策版本、同一事实顺序 → 相同清账结果

### 2.4 channel_switch 默认保持

- 原 `feed_replay_bar` 路径保持不变（ADR-0003 明确）
- center_break 作为**第二层**，只有在桥到位后才可启用
- 不替代严格交易准入失败，不修改 `Confirmed`/`Invalidated` 的终态钟

## 三、实施步骤

### Step 1：数据类型（红）
- [ ] 新增 `CenterBreakFact` 结构体（`nest_lifecycle/types.rs` 或新文件）
- [ ] 新增 `HypothesisBridge` 结构体
- [ ] `InvalidatedReason` 新增 `CenterBreak` 变体
- [ ] `NestLifecycleBook` 新增字段：`bridges`、`applied_breaks`

### Step 2：端点 1 事实产生（绿）
- [ ] 新增 `center_break_extractor.rs` 模块
- [ ] 从 `CenterLifecycleEvent::Broken` 提取 `CenterBreakFact`
- [ ] 测试：真实 BSP 链产生 `Broken` → 提取完整 `CenterBreakFact`

### Step 3：端点 2 对象匹配（绿）
- [ ] 修改 `provide_active_pan_live_windows`，保留完整 `Center` 引用
- [ ] `NestLifecycleBook::advance` 注册时建立 `HypothesisBridge`
- [ ] 读取具名政策版本（从配置或上下文）
- [ ] 测试：产窗 → 注册 → `bridges` 含精确 `CenterId` + 原键

### Step 4：端点 3 清账（绿）
- [ ] 实现 `NestLifecycleBook::apply_center_break`
- [ ] 精确匹配：level + `CenterId` → `LifecycleKey`
- [ ] 产生 `Invalidated(CenterBreak)` 修订，更新 `applied_breaks`
- [ ] 幂等测试：重复事实不重复清账

### Step 5：持久恢复（绿）
- [ ] `NestLifecycleBook` 实现 `Clone`（如需）
- [ ] 重放测试：清空账本 → 按原序喂事实 → 对拍对象/版本/清账集合
- [ ] 负控：错级别、错 `CenterId`、同 start 不同核心、迟到事实

### Step 6：正式消费（集成）
- [ ] 在 `p123_fast_replay.rs` 或新会话适配器集成三端点
- [ ] 调用路径：`CenterEventMachine` → 提取事实 → `apply_center_break`
- [ ] 产生账目输出（`revisions`）
- [ ] 与原 `feed_replay_bar` 逐位对拍（center_break 未接时输出不变）

### Step 7：验证（全）
- [ ] 所有单测通过（nc-remote）
- [ ] 负控全覆盖：Superseded/Reset/错级/错 CenterId/重复/迟到
- [ ] 重启恢复边界验证
- [ ] 正式消费证据（实际事件 + 账目输出）

## 四、写区与测试命令

**写区**（仅限本票必要新增）：
- `rust/src/theta_v0/classifier/nest_lifecycle/` 新增/修改
  - `center_break.rs`（新）
  - `book.rs` 字段与方法
  - `types.rs` 或独立 `center_break_types.rs`
- `rust/src/theta_v0/classifier/` 可能新增消费模块
- `rust/src/bin/p123_fast_replay.rs` 集成点（最小接线）

**测试命令**（仅 nc-remote）：
```bash
# 单元测试（远端）
NC_REPO=/path/to/sandcastle-issue-1719 NC_TARGET=sandcastle-issue-1719 \
  ssh nc-remote 'cd /path && cargo test --lib theta_v0::classifier::nest_lifecycle::center_break'

# 集成测试（如需）
NC_REPO=... NC_TARGET=... \
  ssh nc-remote 'cd /path && cargo test --test center_lifecycle_integration'
```

## 五、未完成项与 BLOCK

（实施过程中遇到的共享函数冲突、正式消费定点、恢复持久边界等，记录在此）

## 六、交付物

1. 代码：本工位全部 commit 的 head
2. 测试证据：非零执行的测试输出（测试数、通过数）
3. 事件输出：真实 `CenterBreakFact` 提取与 `Invalidated(CenterBreak)` 修订
4. 恢复负控：错误输入的拒绝证据
5. 消费者路径：正式回放或 paper 使用同核的调用证明
6. 本计划文件更新：实际执行路径、头、命令、未完成项

**约束重申**：
- 不修改 #1527 admission/resource、#1717 SR、#1718 fill
- 不改 main，不关票，不 force push
- Rust 仅 nc-remote 或 CI
- 报告时间取实际 date 输出，引用冻结 head 实际路径行号
