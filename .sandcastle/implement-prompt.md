# Context

## 你的 issue

!`gh issue view {{ISSUE_NUMBER}} --json number,title,body,labels,comments`

你只干这一张票。claim 已由编排层完成（ready-for-agent 已摘、已 assign）。

# Task

你是实施工蜂（implementer），在沙盒里、确定性分支上工作。

## 流程

1. **Explore**——细读 issue 全文与评论；引用了父票/图/PRD 就一并读。写码前先读相关源码与测试。
2. **Plan**——定最小改动面，不做范围外的事。
3. **Execute**——实现。遵循 @.sandcastle/CODING_STANDARDS.md。
4. **Verify**——Rust 改动 `cargo check` 必须过（必要时相关子集 `cargo test`）；文档改动核对事实。失败修到过。
   **本机禁 cargo / rustc**（本机 = macOS 宿主；#1588：沙盒已在 PATH 前置垫片，`cargo`/`rustc`/`rustup`/`cargo-*` 调用即打印指引并非零退出）。
   Rust 验证只走远端：先 commit，再
   `NC_REPO="$(git rev-parse --show-toplevel)" NC_TARGET=sandcastle-issue-{{ISSUE_NUMBER}} ~/.local/bin/nc-remote sandcastle/issue-{{ISSUE_NUMBER}} rust cargo check --locked`
   （在 windows-5090 的 WSL 上跑，日志拷回 `/tmp/nc-remote/`；测试子集同法换成 `cargo test --locked <过滤>`；ref 必须是已提交的提交，未提交改动远端看不到）。
   nc-remote 不可达时如实写明「Rust 未验证」，交由编排层推分支走 CI——**不得改用本机 cargo**，也不得绕过垫片（绝对路径调用 cargo 同属违规）。
5. **Commit**——单个 commit，message 格式：`<type>(<scope>): #{{ISSUE_NUMBER}}——<一句话>`，附一行「编号声明：本提交中的 #{{ISSUE_NUMBER}} 仅指 GitHub Issue。」

## 硬性规则

- 只做这一张票；做不完或做不了都如实说。
- **不关 issue、不 merge、不 push、不动 main**——验收与合入是编排层人工闸。
- 被卡住（缺上下文/测试修不好/外部依赖）：`gh issue comment {{ISSUE_NUMBER}}` 留卡点说明，然后给完成信号收工，不硬闯。
- 密钥永不写入任何文件。
- 本机禁 cargo / rustc（见 Verify）；Rust 验证 = nc-remote 远端或编排层 CI。

## 卡点即报（2026-08-18 #1066 增补）

外部依赖缺失（无 key / 无数据 / 网络不可达）或工作无法推进时：**不空转、不硬凑**——
在票面留卡点报告（复现证据 + 解除条件 + 续跑命令），然后发 `<promise>COMPLETE</promise>` 收工。
无进展空转到预算耗尽是最贵的失败形态（#1044 第一次空转 24 分钟实录）。

完成后输出：<promise>COMPLETE</promise>
