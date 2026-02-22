# 全体系声明-能力缺口审计汇总（v37-swarm）

## 审计范围
- CLAUDE.md 18条原则 + 五约束 + 递归行为规则 + 热启动
- dispatch-dag.yaml v3.1 所有事件/条件/skill映射
- 21个 hooks + 7个 skills + 19个 agents + manifest v2.0

## P0 严重缺口（3个）

### GAP-1: lead-audit.sh 未注册到 settings.json
- dispatch-dag 声明 lead_audit_registered 验证条件
- lead-audit.sh 文件存在且功能完整（088号谱系核心实现）
- 但未在 .claude/settings.json 中注册 → Claude Code 完全不执行它
- **影响**: Lead 直接执行 Write/Edit/Bash 时的拓扑异常对象化审计完全失效

### GAP-2: code-verifier 无触发机制
- dispatch-dag 声明 code-verifier 在 file_write src/**/*.py 时触发
- 无对应 hook 实现此事件检测
- code-verifier.md agent 存在但从不被事件驱动激活
- **影响**: Python 代码变更后无自动验证

### GAP-3: manifest.yaml 含3个幽灵条目
- ceremony-guard.sh（不存在，已被 agent-team-enforce.sh 替代）
- lead-permissions.sh（不存在，已被 lead-audit.sh 替代）
- team-structural-inject.sh（从未创建）
- manifest v2.0 声称"与实际严格对齐"——此声称为假
- **影响**: 声明-能力不一致的典型实例

## P1 重要缺口（4个）

### GAP-4: event_skill_map "事件驱动"实际是"hooks提示+Lead手动认领"
- dispatch-dag 声明 9个 skill 由事件自动触发（triggers 措辞暗示自动化）
- 实际：hooks 执行部分检查逻辑，但不 spawn 对应 agent
- genealogist/quality-guard/meta-observer/skill-crystallizer/topology-manager 全部是 D策略
- 这是 082号设计意图，但措辞需要对齐

### GAP-5: build_failure 事件未实现
- dispatch-dag 声明 consecutive_failures >= 3 → auto_fix_build
- 无 hook 检测连续构建失败

### GAP-6: topology-manager 无触发机制
- dispatch-dag 声明 genealogy_count_threshold 触发
- 无 hook 检测谱系数量阈值（141个谱系已远超30的阈值）

### GAP-7: 020号反转条件声明 block vs 实际 advisory
- CLAUDE.md 声明"修改 CLAUDE.md/核心定义/已结算谱系"时"必须阻断等待"
- spec-write-guard.sh 实际对这些修改输出 advisory（allow），不是 block
- 原则0（蜂群能修改一切）vs 020号反转（必须阻断）的张力

## P2 次要缺口（3个）

### GAP-8: dispatch-spec.yaml 严重过时
- 仍引用032号 divine-madness（已被088号替代）
- 仍声明 structural_stations mandatory spawn（已改为 skill）
- 仍有 claude-challenger 虚假事件声明

### GAP-9: ceremony-completion-guard.sh 名称误导
- 名称暗示 ceremony 专用，实际是通用 Stop guard
- manifest 描述"6项检查"，实际代码5项

### GAP-10: 结晶检测时机不完整
- crystallization-guard.sh 在 git commit 时检查（PostToolUse/Bash）
- 但 dispatch-dag validation 声明 crystallization_check 应在 ceremony 后检查
- ceremony_scan.py 不执行结晶检测

## 审计统计

| 层面 | 一致 | 部分实现 | 未实现 | 总计 |
|------|------|---------|--------|------|
| CLAUDE.md 原则 | 27 | 8 | 0 | 35 |
| dispatch-dag 事件 | 5 | 4 | 2 | 11 |
| dispatch-dag 条件 | 3 | 2 | 3 | 8 |
| hooks | 19 | 1 | 1(未注册) | 21 |
| skills | 7 | 0 | 0 | 7 |
| agents(skill_map) | 1 | 5 | 3 | 9 |
| agents(platform) | 10 | 0 | 0 | 10 |

## 讨论焦点

1. GAP-1 是否应立即修复（lead-audit.sh 注册）？
2. GAP-3 manifest 幽灵条目是否反映更深层的组件命名演化追踪缺口？
3. GAP-4 event_skill_map 的"自动触发"措辞是否需要修正为"D策略"？
4. GAP-7 原则0 vs 020号反转的张力应如何解决？
5. dispatch-spec.yaml 是否应该废弃还是同步更新？
