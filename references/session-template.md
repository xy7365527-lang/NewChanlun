# Session 模板

session 是指针，不是叙事。内容活在文件系统里（definitions/, genealogy/, git history），session 只存引用。

50行封顶。超过50行说明你在重复文件系统里已经有的信息。

```markdown
# Session: [简短标题]

**时间**: [YYYY-MM-DD-HHmm]
**分支**: [branch name]
**最新提交**: [hash] [message]
**工作树**: [clean / dirty summary]

## 定义基底
| 名称 | 版本 | 状态 |
|------|------|------|
| [name] | [version] | [已结算/生成态] |
→ 来源: .chanlun/definitions/*.md

## 谱系状态
- 生成态: [N] 个
  - [id]: [一句话]
- 已结算: [M] 个
→ 来源: .chanlun/genealogy/{pending,settled}/

## 中断点

### 阻塞项（需外部输入）
- [描述]: 等待 [什么]

### 可继续工位
- [工位名]: [一句话说明下一步]

### 最高优先缺口
- [一句话]

### 编排者议题
- [编排者提出但还没处理的方向]

## 恢复指引
1. 读取此文件获取状态指针
2. 按中断点评估可并行工位
3. 直接进入蜂群循环
```

## 规则

- **不重复文件系统**：定义内容在 definitions/ 里，谱系内容在 genealogy/ 里，代码变更在 git 里。session 不复述这些。
- **中断点是 session 的唯一独有价值**：这些信息散落在对话上下文中，compact 后会丢失，所以需要 session 保存。
- **叙事禁入 session**：审计过程、哲学讨论、代码修复细节、已否定方案——全部走 git commit messages 或 genealogy entries。
- **50行封顶**：超过50行就在重复文件系统。如果你觉得需要超过50行，问自己"这段内容是否已经活在别的文件里"。
