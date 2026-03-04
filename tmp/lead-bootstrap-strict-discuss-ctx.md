# Lead 最小自举形式——严格讨论上下文

## 讨论目标

严格讨论 ceremony_scan.py 扩展方案和 ceremony.md 重写方案。
不是"能不能做到"，而是"严格的形式是什么"。

## 已收敛的共识（Codex × Gemini）

1. 最小链路：scan → TeamCreate → spawn_all_parallel → consume_all → persist → re-scan → TeamDelete
2. ceremony_scan.py 从"状态扫描器"升级为"确定性调度计划生成器"
3. Lead 不可委托项：ceremony_scan.py 调用、TeamCreate、Task spawn、git commit/push、TeamDelete
4. 可移出 Lead 的项：gangju（并入 scan）、meta-observer/topology-analyst/audit（进 workstations 列表）
5. 白名单缩减：5 项 → 3 类
6. ceremony.md 目标：~40 行，形式即行为
7. F3 已解决：结构能力可作为 ephemeral skill invocation 出现在 workstations 列表中

## 编排者核心约束

1. "形式即行为——不依赖记忆，只依赖结晶"
2. "永远不要串行"
3. "我只需要严格，不需要务实"

## 当前 ceremony_scan.py 代码事实

- 952 行，已包含：roadmap 扫描、session 遗留、pending 谱系、frozen 节点过滤、topo_effect、delta_genealogy、delta_blocks、pattern_buffer、downstream_audit、async_self_reference、genealogy_anomalies
- 输出 JSON 包含 workstations[]，但只有业务工位
- gangju_analysis.py 是独立脚本（894行），Lead 在 RTAS 循环中单独调用

## 扩展方案（待严格讨论）

### 方案核心：ceremony_scan.py 吸收 gangju + 结构工位推导

1. **gangju 并入 scan**：
   - import gangju_analysis 的核心函数（extract_gang, compute_block_stats, derive_mu 等）
   - 在 scan 的 main() 末尾调用
   - audit_needed=true 时，将审计工位加入 workstations[]

2. **结构工位推导**：
   - meta-observer：swarm_cycle_end 条件（所有业务工位完成后）→ 加入 workstations[]
   - topology-analyst：.chanlun/block-topology/blocks/ 非空 → 加入 workstations[]
   - code-verifier：src/**/*.py 有变更 → 加入 workstations[]（但这需要 git diff，scan 是否应该做？）

3. **输出格式统一**：
   ```json
   {
     "workstations": [
       {"name": "risk_management_module:stop_loss_strategies", "type": "business", ...},
       {"name": "meta-observer", "type": "structural", "trigger": "swarm_cycle_end"},
       {"name": "topology-analyst", "type": "conditional", "trigger": "blocks_exist"},
       {"name": "gangju-audit", "type": "structural", "trigger": "audit_needed"}
     ]
   }
   ```

### 待讨论的严格问题

1. **结构工位的触发时机**：meta-observer 应该在 scan 阶段就加入 workstations，还是在 re-scan 阶段？
   - 如果 scan 阶段就加入：meta-observer 与业务工位并行 spawn，但此时业务工位还没完成，meta-observer 观察什么？
   - 如果 re-scan 阶段加入：符合 swarm_cycle_end 语义，但需要 scan 区分"首次扫描"和"重扫描"

2. **gangju 的执行时机**：gangju 原本在"所有工位完成后、re-scan 之前"执行。并入 scan 后：
   - 首次 scan 时 gangju 的输入（block_stats、genealogy_stats）反映的是 scan 时刻的状态
   - re-scan 时 gangju 的输入反映的是工位完成后的状态
   - 这两个时刻的 gangju 输出可能不同——scan 需要区分吗？

3. **code-verifier 的触发**：需要 git diff 检测文件变更。scan 是否应该执行 git diff？
   - 如果是：scan 变得更重（需要 git 操作）
   - 如果不是：code-verifier 不在 scan 输出中，Lead 仍需自己决策

4. **ceremony.md 重写后的 Stop hook 处理**：当前 ceremony.md 有"每次 Stop hook 拦截时，执行步骤 1（调用 TaskList）"。重写后这条规则放在哪里？

5. **增量持久化的责任转移**：当前 Lead 在工位完成时增量写 session。如果 Lead 变薄，增量持久化由谁负责？
   - 各工位自行写入？→ 并发写入冲突
   - Lead 在 consume_all 时批量写入？→ 中途断掉时丢失

## 严格讨论要求

请从代码层面严格分析：
1. ceremony_scan.py 扩展的精确代码变更（哪些函数新增/修改）
2. 结构工位触发时机的严格判定（首次 scan vs re-scan）
3. gangju 并入 scan 的精确方式（import 还是内联）
4. ceremony.md 重写的精确文本（~40行）
5. 边界条件：什么情况下这个方案会失败？
