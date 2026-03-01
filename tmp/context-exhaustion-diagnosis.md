# 上下文耗尽问题诊断与解决方案设计

**工位**：ws-context-exhaust（v127-swarm）
**日期**：2026-03-01
**模式**：Codex diagnose（异质代码层否定）
**Codex 结果文件**：`.chanlun/review-results/codex-diagnose-20260301-0655.md`

---

## 一、结论

**Codex 否定成立**：274号声明"context 耗尽是暂停不是终止"在工程上**目前尚未严格保证**。
存在三个实现层缺口，全部标注为"实现错误"（不是定义冲突）。

最脆弱点排序：
1. 条件3（原子链 compaction 边界）——.ceremony-step 文件实测不存在
2. 条件2（L1/L2 恢复信息）——precompact-save.sh 未保存工位局部执行态
3. 条件1（compaction 触发本身）——平台层，只能部分缓解

---

## 二、Codex 诊断核心发现

### §1. Context 消耗主要来源

| 来源 | 严重性 | 可控性 |
|------|--------|--------|
| 工具输出全量注入（Read/Bash/scan JSON） | 极高 | 可减少 |
| 对话历史累积（无阶段压缩） | 极高 | 可减少 |
| 谱系文件深层引用 | 高 | 可减少 |
| Skill 触发过宽 | 中高 | 可减少 |
| CLAUDE.md + Rules 强制加载 | 中（固定底噪） | 基本不可减少 |

### §2. 现有热启动机制三大缺口

#### 缺口A：工位级执行状态丢失
- **场景**：工位完成任务 60% 时 compaction
- **问题**：precompact-save.sh 仅存全局元信息，不存工位局部执行态（已完成子项、next_action、工件引用）
- **直接验证**：.chanlun/checkpoints/ 目录不存在

#### 缺口B：ceremony 相位机缺失
- **场景**：Lead 在 ceremony 步骤8（rescan）中途 compaction
- **问题**：无持久化 ceremony 状态文件（epoch/phase/rescan_result_hash）
- **直接验证**：.chanlun/.ceremony-step 文件不存在（226号下游推论4 声明要创建，但实测未执行）
- **影响**：原子链（224号/225号）在 compaction 边界的承诺在工程上是空洞的

#### 缺口C：TeamCreate 无事务化语义
- **场景**：子蜂群在 TeamCreate 后、spawn 前 compaction
- **问题**：无 operation_id + 状态日志 + resume reconciliation
- **影响**：可能出现孤儿工位或重复创建

### §3. 修复方案（Codex 评估，我判定成立）

**P0（必须）**：

1. **ceremony_state.json** — Lead 级 WAL 状态机
   ```json
   {
     "epoch": "<ceremony轮次>",
     "phase": "RESCAN_STARTED | RESCAN_DONE | EVAL_DONE",
     "last_transition_ts": "<ISO8601>",
     "rescan_hash": "<scan输出hash>"
   }
   ```
   每次相位迁移原子落盘（tmp→fsync→rename）。
   恢复规则：
   - phase=RESCAN_STARTED 且无 RESCAN_DONE → 重做步骤8（幂等）
   - RESCAN_DONE 无 EVAL_DONE → 执行步骤9
   - spawn/terminate 用 op_id 去重

2. **工位级 checkpoint schema**
   ```json
   {
     "agent_id": "<工位ID>",
     "phase": "<任务相位>",
     "done": ["<完成子项>"],
     "pending": ["<待执行子项>"],
     "next_action": "<下一步>",
     "artifacts": [{"path": "<路径>", "sha": "<hash>"}],
     "op_id": "<幂等键>",
     "updated_at": "<ISO8601>"
   }
   ```
   路径：`.chanlun/checkpoints/{agent_id}.json`
   写入时机：长输出前、重工具调用前后、spawn 前

**P1（高收益）**：

3. **ceremony_scan.py 输出瘦身**：摘要优先+hash，详情按需展开
4. **谱系引用深度策略**：默认只拉一阶依赖，二阶按需

**P2（可选）**：

5. Skill 触发条件收紧 + 工具输出预算守卫（大输出自动摘要化）

**P3（谨慎）**：

6. CLAUDE.md/rules 只做语义等价重写，不做语义删减

### §4. 与已有谱系的关系

| Codex 发现 | 已有谱系 | 关系 |
|-----------|---------|------|
| ceremony 相位外部化 | 226号下游推论4（.ceremony-step 文件） | 226号已识别但未执行 |
| 原子链 compaction 边界 | 224/225号（原子链声明） | 224/225只修复了 hook 边界，未修复 compaction 边界 |
| LLM 不维护精确状态 | 057号 | 完全一致 |
| 不引入全局截断 | 274号 | 方案符合约束 |
| 不引入串行阻塞点 | 218号 | ceremony_state.json 是 Lead 本地状态机，不影响并行 |

---

## 三、判定总结

| 问题 | 否定是否成立 | 严重性 | 处理建议 |
|------|------------|--------|---------|
| 缺口A（工位执行状态） | 成立 | 高 | P0：实现工位 checkpoint |
| 缺口B（ceremony 相位机） | 成立 | 极高 | P0：实现 ceremony_state.json |
| 缺口C（TeamCreate 事务） | 成立 | 中 | P1：operation_id 机制 |
| Prompt 瘦身策略 | 成立（可优化） | 中 | P1：scan 输出瘦身 + 谱系一阶引用 |

**无误判**：Codex 的所有否定均基于实际架构缺口，无 Codex 误读上下文的情况。

---

## 四、边界条件

1. 本诊断所有场景均为"实现错误"（非定义冲突）——274号的设计意图正确，实现未跟上
2. 类型B中断（纯文本后）仍是平台限制，Codex 方案未涉及（正确排除）
3. ceremony_state.json 方案的严格性依赖写入操作本身不被 compaction 中断——对极短窗口的 compaction（写文件中途）仍是已知 Gap，但概率极低

---

## 五、影响声明

**涉及模块**：
- `.claude/hooks/precompact-save.sh`（需扩展工位 checkpoint 写入）
- `scripts/ceremony_scan.py`（需输出瘦身）
- 待创建：`.chanlun/checkpoints/` 目录 + schema
- 待创建：`.chanlun/ceremony_state.json` WAL 机制

**谱系引用**：
- 226号（Lead 中断三类根因，下游推论4 是本工位工作的直接前置）
- 274号（废除 depth_budget，声明 compaction = 暂停不是终止）
- 224号/225号（ceremony 原子链）
- 057号（LLM 不是状态机）
- 218号（Lead 并行化）
