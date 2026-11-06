# PR1843 共享 DevSkim 清单集成（46C）

结论：PR1843 已与当前 main（含 PR1844 合入 876fab8f）合并，冲突只在 `.github/devskim-reviewed-1488.json`，按条目并集解决。新 head `55d012bf223ef2cff70abef648996b464966d433` 已普通 push 到同一分支 `codex/1818-closeout-archive45f`，GitHub 判 MERGEABLE，CI 运行中。

## 做了什么

- 工位：`/Volumes/AgentStorage/decision1818-archive45f/worktree`（PR1843 原工位，开工时干净，HEAD = b6dd6bfb）。
- `git merge --no-ff origin/main`，唯一冲突文件为清单 JSON（单行文件，文本冲突不可逐行解）。
- 按 `path` 三方结构合并：base 112 条；PR 侧新增 5 条（#1818 的 layout-delta41o/41t、layout-final41w、layout-review41i/41r 的 .json），main 侧新增 3 条（#1539 的 archaeology.json、review40y.json、review41k.json）；两侧无修改、无删除、无同路径冲突，`schema/issue` 元字段一致。
- 结果 120 条，路径唯一；以 main 版为底保持原序，PR 的 5 条追加在末尾，条目内容逐字取自 PR 侧原件。紧凑序列化与 main 原文件逐字节一致（已对 theirs 做回环校验），保留末尾换行。
- 合并提交 55d012bf，父为 b6dd6bfb 与 origin/main。

## 检查

- 清单 120 条全部核对：每个 path 在新 head 存在，且文件 SHA-256 与登记的 `sha256_bytes` 相符（0 不符）。这是 `scripts/devskim_sarif_gate.sh` 依赖的绑定前提；规则身份 `identity_bytes` 需真实 SARIF，由 CI DevSkim 判定。
- PR 相对 main 的净差异仍为 25 文件 +2010/−2，与集成前同范围。
- `docs/adr/0059-*.md` 与 `.chanlun/review-results/issue1818-decision-20261101/` 在 b6dd6bfb 与 55d012bf 之间零差异。
- 未改扫描规则、baseline、gate 脚本、workflow、隔离、资金、资源帽；无 force、无删除；未在本机跑 cargo/rustc。

## 原审仍有效 / 需补审

- 仍有效：PR1843 自身的全部文书与 ADR 0059（45I 差异审、45F 归档验收）以及 b6dd6bfb 的 12 处 SHA 误报登记（45Q 源审 PASS）——这些字节在新 head 未变。
- 需补审：仅合并提交对清单 JSON 的并集解法（PR 5 条 + main 3 条，无其他改动）。建议非作者对 `git diff b6dd6bfb 55d012bf -- .github/devskim-reviewed-1488.json` 做一次窄差异审（可用上面的结构对比复核），并以新 head 的 DevSkim 结论作为动态证据。

## CI

- CI run 41364485736（55d012bf）queued；DevSkim run 41364485771（55d012bf）in_progress。合入前须两者必要项全绿，DevSkim 零未登记 finding。
- 合入消息不得带对 #1818 的自动关票词。

## 恢复

- 集成前 head：b6dd6bfb07ac185b3af4b56c576b41cb28e479d5（远端历史中保留，新提交是其子孙，可随时回退分支指针）。
- 三方原件：本目录 base.json / ours.json / theirs.json / merged.json。
