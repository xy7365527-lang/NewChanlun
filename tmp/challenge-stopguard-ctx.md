# 异质质询上下文：stop-guard 时序 bug + definitions.yaml 腐化

## 问题描述

ceremony（创世仪式）完成后，agent 执行了 `rm .chanlun/.ceremony-in-progress`，
然后输出"请指定工作方向"并停止。stop-guard hook 检测时标志位已不存在，放行了停止。

按协议，热启动 ceremony 应该"不等待编排者确认，直接进入蜂群循环"。agent 不应该停下来问问题。

同时发现 definitions.yaml 有 2 个 implementation_status 腐化条目：
- 买卖点：标注 stub（0%代码），实际 complete（337行代码 + 43测试 + 98%覆盖率）
- 流转关系：标注 partial（顶点聚合+共振待完善），实际 complete（268行 + 22测试）

agent 基于腐化数据错误判断"买卖点是最大缺口"，浪费一轮读取已完成的代码。

## 关键文件

### stop-guard hook (.claude/hooks/ceremony-completion-guard.sh)
- 检查 1：.chanlun/.ceremony-in-progress 存在 → block
- 检查 2：蜂群任务队列非空 → block
- 检查 3：生成态谱系矛盾 → block
- 检查 4：@proof-required 标签 → block
- 全部通过 → allow

### 时序问题
```
实际发生：ceremony报告 → rm flag → 输出问题 → guard检查(flag已无) → allow
应该发生：ceremony报告 → 行动声明 → tool call(蜂群启动) → rm flag
```

### definitions.yaml 腐化
- 14个实体中有2个 implementation_status 与实际不符
- 腐化率 14.3%（2/14）
- 没有自动化机制保证 definitions.yaml 与代码同步

## 质询目标

1. 时序 bug 的根因是 hook 设计问题还是 agent 行为问题？
2. definitions.yaml 腐化是偶发还是系统性的？
3. 这两个问题是否有共同的深层原因？
