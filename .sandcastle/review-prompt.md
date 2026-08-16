# TASK

评审分支 `{{BRANCH}}` 相对 `{{TARGET_BRANCH}}` 的改动，在**保持功能语义不变**的前提下直接在该分支上修正问题。

# CONTEXT

## Branch diff

!`git diff {{TARGET_BRANCH}}...{{BRANCH}}`

## Commits on this branch

!`git log {{TARGET_BRANCH}}..{{BRANCH}} --oneline`

## 原始 issue

!`gh issue view {{ISSUE_NUMBER}} --json title,body`

# 评审流程

1. **懂意图**：对照 issue 与 diff，弄清要干什么。
2. **正确性**：边界情形、错误处理、unwrap/expect 合理性、测试覆盖、安全问题（注入/密钥泄漏）。
3. **清晰度**：命名、嵌套、冗余、注释质量；遵循 @.sandcastle/CODING_STANDARDS.md。
4. **纪律面**：commit message 带票号与编号声明；改动面未越出 issue 范围；无密钥写入。
5. **修正**：发现问题**直接在本分支修复并追加 commit**，message：`review: #{{ISSUE_NUMBER}}——<修正点>`。
6. **验证**：修正后 `cargo check`（或等价校验）必须过。

# 硬性规则

- 不改功能语义；不过度简化到伤可读性。
- 不 merge、不 push、不关 issue、不动 {{TARGET_BRANCH}}。
- 无修正也要在最终答复显式写「评审通过：无修正」。

完成后输出：<promise>COMPLETE</promise>
