# TASK

评审分支 `{{BRANCH}}` 相对 `{{TARGET_BRANCH}}` 的改动，在**保持功能语义不变**的前提下直接在该分支上修正问题。

# CONTEXT

> 本提示由宿主侧 `.sandcastle/review-context.mts` 渲染（#1588）：**只含变更摘要，不含 diff 正文**。
> 整段 diff 曾把提示撑爆（#1406：~39 万 token →「Prompt is too long」）。diff 请在工作区里按需读取，见「读 diff 的方法」。

## 改动统计（git diff --stat {{TARGET_BRANCH}}...{{BRANCH}}）

```
{{DIFF_STAT}}
```

## 本分支提交（git log --oneline {{TARGET_BRANCH}}..{{BRANCH}}）

```
{{COMMITS}}
```

## 文件清单（git diff --name-status {{TARGET_BRANCH}}...{{BRANCH}}）

```
{{FILES}}
```

## 原始 issue

```
{{ISSUE}}
```

# 读 diff 的方法

你在本分支的工作区里，git 可用。**不要一次性输出整段 `git diff {{TARGET_BRANCH}}...{{BRANCH}}`**（会撑爆上下文、截断后静默漏看）。按需分块读：

- 逐文件：`git diff {{TARGET_BRANCH}}...{{BRANCH}} -- <路径>`；大文件再分段，如 `... -- <路径> | sed -n '1,400p'`；
- 逐提交：`git show --stat <sha>`，再 `git show <sha> -- <路径>`；
- 先按上面的文件清单排优先级（核心逻辑与测试先看，生成物/夹具后看），每个文件都要过到，最终答复里列出已看文件。

# 评审流程

1. **懂意图**：对照 issue 与改动摘要，弄清要干什么；再逐文件 / 逐提交读 diff。
2. **正确性**：边界情形、错误处理、unwrap/expect 合理性、测试覆盖、安全问题（注入/密钥泄漏）。
3. **清晰度**：命名、嵌套、冗余、注释质量；遵循 @.sandcastle/CODING_STANDARDS.md。
4. **纪律面**：commit message 带票号与编号声明；改动面未越出 issue 范围；无密钥写入。
5. **修正**：发现问题**直接在本分支修复并追加 commit**，message：`review: #{{ISSUE_NUMBER}}——<修正点>`。
6. **验证**：Rust 修正**禁止在本机跑 cargo / rustc**（本机 = macOS 宿主，沙盒已用垫片拦截，调用即非零退出）。
   先 commit，再远端验证：
   `NC_REPO="$(git rev-parse --show-toplevel)" NC_TARGET=sandcastle-issue-{{ISSUE_NUMBER}} ~/.local/bin/nc-remote {{BRANCH}} rust cargo check --locked`
   （日志拷回 `/tmp/nc-remote/`；测试子集同法换成 `cargo test --locked <过滤>`）。nc-remote 不可达时如实写明未验证，交由编排层推分支走 CI，不得改用本机 cargo。

# 硬性规则

- 不改功能语义；不过度简化到伤可读性。
- 不 merge、不 push、不关 issue、不动 {{TARGET_BRANCH}}。
- 本机禁 cargo / rustc（见上）。
- 无修正也要在最终答复显式写「评审通过：无修正」。

完成后输出：<promise>COMPLETE</promise>
