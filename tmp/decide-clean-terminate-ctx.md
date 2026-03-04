# 决策请求：蜂群的"真阴性干净终止"是否过早

## 请求类型
选择类（多路径价值判断）——不是定理推导，需要价值判断选择方向。

## 当前系统状态

ceremony_scan.py 输出（2026-02-21，commit 8be55a9）：
```json
{
  "mode": "warm_start",
  "session": "2026-02-21-v17-session.md",
  "fallback_triggered": true,
  "workstations": [],
  "clean_terminate": true,
  "definitions": 14,
  "pending": 0,
  "settled": 92,
  "downstream_actions": {
    "total": 73,
    "unresolved": 8,
    "execution_rate": "67%"
  }
}
```

系统声明条件满足：
- roadmap.yaml 的 tasks 全部 status=completed（level_recursion、real_data_validation、buysellpoint_module、xiaozhuan_da、definition_status_sync 全部完成）
- pending 谱系为 0
- 测试通过（1435个，0失败）
- pattern-buffer 中没有 status=candidate 以上的模式（只有 status=observed 的）

## 081号谱系定义的"真阴性干净终止"条件

来自 .chanlun/dispatch-dag.yaml：
```
trigger: "roadmap 为空 AND pending 谱系为空 AND 测试全通过 AND pattern-buffer 无达标 AND 所有其他 scan_sources 为空"
```

注：roadmap "为空"的解释存在歧义——是指"没有 active 任务"还是"文件为空"？
当前 roadmap.yaml 有 5 个任务，全部 status=completed。

## 下游行动审计（ceremony_scan 的 downstream_actions 字段）

total=73，unresolved=8，execution_rate=67%

unresolved 的 8 项（来自 downstream-action-audit-v17.md 分析）：

**长期工程项（平台约束或长期研究，不是立即可做的）：**
1. Guard 结晶为统一异质审计协议（062号）——长期工程任务
2. 四种话语映射的工程化评估（064号）——理论研究，与核心代码无关
3. 卢麒元四矩阵降级（064号）——理论研究
4. 递归终止表述不一致：dispatch-dag 混用"区间套收敛"和"背驰+分型"（069号）
5. post-commit-flow "→接下来" 统计验证（070号）
6. 结构性不可加固解法矩阵（071号）——长期架构洞见
7. bypassPermissions 兼容性（072号）——平台层确认
8. 谱系结算时自动生成 task 的 hook（074号/076号）

**重要问题**：ceremony_scan.py 目前没有扫描 downstream_actions 中的 unresolved 项作为工位来源。dispatch-dag 的 scan_sources 包含"谱系下游行动未执行"，但 ceremony_scan.py 的 `clean_terminate` 判断没有包含这个维度。

## 人类编排者提出的 10 个可能工作来源

1. 代码覆盖率（是否有模块低于 80%？）
2. 代码质量（模块超过 800 行？函数超过 50 行？）
3. 集成测试缺口
4. 性能基准测试
5. 文档
6. 可视化（b_chart/b_plot 递归级别可视化）
7. 082号下游："半事件驱动"hooks 提示机制实现
8. bi.md #4："mode='new' 已可用但尚未设为默认"——真实数据验证后切换
9. 实战回测框架
10. 更多品种真实数据验证（目前只验证了 AAPL 1分钟）

## 核心矛盾

**矛盾 1：scan_sources 实现不完整**
dispatch-dag 定义 ceremony_scan 应扫描"谱系下游行动未执行"，但 ceremony_scan.py 的 `clean_terminate` 判断没有包含这个维度。当前有 8 个 unresolved 下游行动。

**矛盾 2：downstream_actions.unresolved 的分类**
这 8 个 unresolved 中，很多被标记为"长期工程任务"或"平台阻塞"——这些是 background_noise（079号：不应作为工位的噪音）还是真实工位？

**矛盾 3：roadmap"为空"的语义**
roadmap.yaml 有 5 个任务全部 completed。"roadmap 为空"是指 active=0，还是文件任务全部 completed？

## 四个选项

- **A. 干净终止合法**：所有核心定义已实现+测试全绿=阶段性完成。新工作需要人类提供新方向（如"做回测框架"或"做可视化"）
- **B. scan 不够深**：ceremony_scan.py 的 no_work_fallback 只扫描测试失败，应扩展为扫描覆盖率/代码质量/集成测试缺口
- **C. roadmap 应自动从代码库状态派生新任务**：scan 应该分析定义文件中的 TBD、代码中的 TODO、覆盖率报告等，自动写入 roadmap
- **D. 混合**：承认核心定义实现完毕是里程碑，但从可行工作中选择 2-3 个最高价值项写入 roadmap

## 关键约束

1. **081号谱系（已结算）**：终止逻辑已精确化，roadmap+pending+测试+pattern-buffer 四条件
2. **079号谱系（已结算）**：背景噪音不应作为工位——长期平台阻塞项是噪音
3. **076号谱系（已结算）**：下游推论靠主动认领，不是强制阻塞
4. **原则 10（CLAUDE.md）**：蜂群是默认工作模式，不是可选优化
5. **原则 7（CLAUDE.md）**：commit/push 后不允许等待信号

## 需要决断的问题

1. downstream_actions.unresolved=8 是否应该阻止 clean_terminate？
   - 具体说：那 8 个 unresolved 是否都是 background_noise，还是有些应该作为工位写入 roadmap？
2. 选项 A/B/C/D 中，哪个最符合系统原则？
3. 如果选 D，从 10 个可能工作来源中选哪 2-3 个写入 roadmap？

## 相关谱系引用

- 081号：roadmap 对象化 + 终止逻辑修正
- 079号：下游推论 = 建议，非阻塞
- 076号：下游推论靠主动认领
- 018号：四分法（定理/选择/语法记录/行动）
- 041号：编排者代理（Gemini decide）
