# 下游推论评估报告（v220-swarm/downstream-audit）

日期：2026-03-11
评估者：downstream-audit 工位
任务不可分解因为：7条推论的评估是同一轮审计的原子操作，每条评估依赖相同的代码上下文，拆分不产生并行收益。

## 评估总表

| # | 来源 | 推论 | 评估 | 理由 |
|---|------|------|------|------|
| 1 | 415号 | 保留历史折叠提案被否定：不进入实现阶段 | **resolved** | 415号已结算（status: 已结算），提案归档为被否定的设计方向。无代码变更需求——这是一条否定性结论（"不做X"），不产生实装工位 |
| 2 | 415号 | 正确修复方向确认：ghost settlement 解法是 SUBLATED 标记释放锁区 | **需立即实装** | 当前代码中 `SUBLATED` 标记**不存在**（`grep SUBLATED topological-computation/*.py` 零匹配）。v219-swarm 实装的三方案（A: negate免检, B: fold同构, C: per-cycle residue）解决了部分锁区问题，但未实现 Gemini 提出的 SUBLATED 标记机制。396号仍为生成态，residue 机制已有代码骨架（`residue_vertices()`、`_cycle_residue_vertices()`、`boundary_edge` 类型），但被摧毁的 cycle 缺少显式的 SUBLATED 状态标记——目前 `purge_invalid_cycles` 直接删除失效 cycle，而非标记为 SUBLATED 后释放锁区 |
| 3 | 415号 | 396号谱系获得支持：settlement closure→transformation 与 Gemini SUBLATED 建议一致 | **resolved** | 这是一条认识论层面的确认（396号方向正确），不产生独立工位。其实装效力已被推论2吸收——SUBLATED 标记实装就是 396号方向的具体落地 |
| 4 | 415号 | negate 100% blocked 根因确认：是 would_destroy_settled 锁区过大问题 | **resolved** | v219-swarm 方案A已实装：`engine.py:934-937` negate 函数完全跳过 `would_destroy_settled` 检查，注释明确引用410号推论4。negate 不再被 settlement 约束阻塞。根因已修复 |
| 5 | 412号 | WhatsApp 通知集成 [deferred] | **deferred** | 原文已标注 deferred。OpenClaw 的 `--summary` 参数已实装（`ceremony_scan.py:1306`），但 BOOTSTRAP.md 尚未创建（`glob **/BOOTSTRAP.md` 零匹配），说明 OpenClaw workspace 配置未完成。WhatsApp 集成依赖 OpenClaw 基础配置完成后才有意义。不阻塞当前工作 |
| 6 | 412号 | 多 channel 扩展 [deferred] | **deferred** | 原文已标注 deferred。与推论5同理——OpenClaw 基础设施是前提，channel 扩展是后续增量。不阻塞当前工作 |
| 7 | 412号 | 逢亮 daemon 与 OpenClaw 交互 [deferred] | **deferred** | 原文已标注 deferred。daemon 产出 handoff schema 后 ceremony_scan 自然检测变化的机制已存在（ceremony_scan 读取 `handoff_schema.json`），但 OpenClaw cron job 尚未配置。交互路径的两端已各自实装，连接是 OpenClaw 部署后的自然结果。不阻塞当前工作 |

## 统计

- **resolved**: 3条（#1, #3, #4）
- **需立即实装**: 1条（#2）
- **deferred**: 3条（#5, #6, #7）

## 需要立即实装的工位

### 工位：SUBLATED 标记实装（396号/415号联合推进）

**来源推论**：415号推论2（SUBLATED 标记释放锁区）
**关联谱系**：396号（settlement 从 closure 到 transformation，生成态）

**当前状态**：
- `purge_invalid_cycles()` 在 fold/sublate 操作后直接删除失效 cycle（engine.py:671-689）
- 被删除的 cycle 信息丢失——无法区分"从未存在"和"曾经 settled 后被扬弃"
- 396号的 residue 机制（`residue_vertices()`、`boundary_edge`）已有骨架，但 residue 产出与 cycle 生命周期未打通

**实装内容**：
1. `SettledCycle` 增加 `status` 字段：`ACTIVE` | `SUBLATED`
2. `purge_invalid_cycles()` 改为将失效 cycle 标记为 `SUBLATED` 而非删除
3. `would_destroy_settled()` 仅检查 `ACTIVE` 状态的 cycle，`SUBLATED` 的 cycle 不阻塞操作
4. SUBLATED cycle 的 residue 边成为新穿越目标（与 traversal.py 对接）
5. 补偿性回溯：已被 purge 的 cycle 无法恢复，但未来新 settlement 走新路径

**边界条件**：
- 如果 SUBLATED cycle 数量无限增长，需要 GC 策略（但比 purge 好——至少保留了转化的记录）
- 与 396号的最终结算关联——396号从生成态到已结算需要 SUBLATED 机制的 L2 验证

**优先级理由**：这是 415号 Gemini 质询的核心建设性结论，也是 396号（生成态）落地的关键步骤。不实装则 settlement 的 closure→transformation 扬弃停留在定义层面。
