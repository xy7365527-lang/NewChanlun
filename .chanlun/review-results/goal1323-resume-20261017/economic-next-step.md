# #1406 C6a / #1394 只读续接核查

核查日期：2026-10-17。只读查询 GitHub、提交及 CI 配置；未修改仓库源码、票、分支，未触发运行，未运行本机 cargo，未访问外盘或下载 artifact。

## 结论

**可以立即推进的一个缺口：在现有 C6a 候选上恢复并闭合专用远端验收装配。** 现有代码提交仍在，但专用 CI v3 未进入该候选树，最新票面仍记录 H3 阻断，不能直接 dispatch 通用 CI 冒充 C6a 运行。

远端 `validate/1406-c6a-final` 当前精确 HEAD：
`b6e690ecae5723fbbcb773f15400c71d1010da87`。
[提交](https://github.com/xy7365527-lang/NewChanlun/commit/b6e690ecae5723fbbcb773f15400c71d1010da87)

该 SHA 的 Actions API `total_count=0`，check-runs API `total_count=0`；按分支列出的 Actions 亦为空，该分支无 PR。**没有 C6a CI 链接或终态可交付，不能编造。** 这表示当前 API 未见记录，不证明历史上绝不曾有已删除的运行。

## 可复用提交与未闭合前件

- `239e1123aa34150f551fba7f0a211f4f163493ee`：C6a core v2 / driver v3。
- `b6e690ecae5723fbbcb773f15400c71d1010da87`：driver v4 的 Q 启动拒绝重试，仅改 `s_session/tests/tb03c_query_runtime.py` 与 `test_tb03c_query_runtime_contract.py`。
- 直接父链：`b6e690ecae` → `239e1123aa` → `4d9b072e5b671fc1a84f7d75b3976471b93d2afb`。
- [最新 C6a 评论](https://github.com/xy7365527-lang/NewChanlun/issues/1406#issuecomment-6199094453)：核心/旧 driver 静审已过；driver v4 48/48 纯替身合同并非真实 HTTP/owner 证据。当时 CI v2 新 H3 为 Capture 原件/事实及 Recorded 回执 cut 未绑 after owner，两个 hash 自洽反例仍误接纳，CI v3 正在返修，`PENDING/CONTRACT_READY=False`。
- 评论提到 driver v4 当时尚未提交；现在 `b6e690ecae` 已补提交，说明票面时间落后于分支；但未发现可证明后续 CI v3 已闭合的最新评论或提交。

**配置穷举范围与结果**：对该 SHA 的整个 `.github/` 执行 `git grep -i -E 'c6a|tb03c_query_runtime|CONTRACT_READY'` 零命中；列举其全部 workflows 和 `.github/scripts`，scripts 仅 `run-c7a-remote.py`、`run-cargo-with-telemetry.sh`、`test-feature-delta.sh`、`test-run-cargo-with-telemetry.sh`。`ci.yml:4` 有通用 `workflow_dispatch`，但 `:313` C7a、`:356` HFS 都限定 `refs/heads/codex/1406-c5-ci-20261015`，不会在 C6a 分支执行。未发现专用 C6a workflow、源绑定或可检查的 `CONTRACT_READY` 门。

下一步应取回旧 CI v3 审包（若有其他存档），或在上述精确 SHA 的隔离工作树恢复它，先保留 H3 两反例、逐值 oracle/公共图像绑定、具名步骤角色请求绑定、无 TERM 回执的 -15 拒绝、startup 协议，然后独立复核。保留同一 t0 端点 1200/1830/1920/2040/2070 秒，内层 600/630/300/10/30、TERM3/KILL3，以及原资源、取消、回收、原件/清单要求；确认精确源身份与通过门后才考虑一次具名运行。当前没有合法的 C6a dispatch 命令可建议。

### 建议的最小写范围与验收

以下是重建提议，不冒称旧审包原文件名；如找回原包，应优先保留原文件名和已审内容。

1. `.github/scripts/run-c6a-remote.py`：专用监督、证据绑定及收尾。可读取已有 `s_session/tests/tb03c_query_runtime.py` 输出，但不要复制第三份金额 reducer。
2. `.github/scripts/test-run-c6a-remote.py`：该监督器的受控正反例，尤其 H3 两个 hash 自洽伪造、错身份/逐值/步骤请求、无 TERM 回执退出、期限和取消回收。
3. `.github/workflows/ci-c6a.yml`：独立手动入口，只绑定具名候选和预期源身份，保原资源/时间上限及总是归档；避免改旧 C5/C7a 已封存 workflow。

现存 `s_session/tests/tb03c_query_runtime.py` 与 `test_tb03c_query_runtime_contract.py` 属只读复用来源。若出现新实质缺陷，应先报告再缩小修改，不因 CI 接线随意改 driver、fixture、oracle、资金策略或产品代码。

最小验收：全部原具名反例拒绝与完整正例通过；每个保留原件/公共图像/oracle/回执 cut 绑定同一 after owner、请求及角色；只接受明确 TERM 回执对应退出；startup ECONNREFUSED 重试满足零发送/零响应和原绝对期限；超期成功回执不得 PASS，失败必须封存且回收；workflow 固定源身份、job/步骤期限、原件归档均独立审查。上述只能签准备完成；随后一次真实远端运行，必须核具名执行数量、真实 E/B/Q 运行、完整原件及清理，再签 C6a 声明域。

## 已有真实运行与通用失败分开

旧 C5 v6 / C7a 精确 SHA：`4d9b072e5b671fc1a84f7d75b3976471b93d2afb`。
[CI 38744887626](https://github.com/xy7365527-lang/NewChanlun/actions/runs/38744887626) 当前 API：`completed/failure`。

| 实际 job | 终态 | 链接 |
|---|---|---|
| #1406 C5 固定 HFS checkpoint 与失败清理 | success | [116455832539](https://github.com/xy7365527-lang/NewChanlun/actions/runs/38744887626/job/116455832539) |
| #1406 C7a 六轨与 Resolve 恢复 | success | [116455832744](https://github.com/xy7365527-lang/NewChanlun/actions/runs/38744887626/job/116455832744) |
| macos-session / macos-session | success | [116455832733](https://github.com/xy7365527-lang/NewChanlun/actions/runs/38744887626/job/116455832733) |
| rust-check | failure | [116455832594](https://github.com/xy7365527-lang/NewChanlun/actions/runs/38744887626/job/116455832594) |
| test / fixture-drift / s-observer | success | 同轮 run 的 jobs |

[既有封存说明](https://github.com/xy7365527-lang/NewChanlun/issues/1406#issuecomment-6196476911) 报告 C5 Linux35+macOS35+HFS1 共71实例通过；C7a TestOnly WE-5 为356原件/120oracle/20恢复/4次SIGKILL及三组双跑。它同时报告通用 default 2974/3/161、backtest 2980/3/161，三败仅指纹一致、非根因已证。**本次独立查到了 jobs 终态；未重下 artifact，数量与哈希仅作为既有报告引用。不能外推到 C6a、新 SHA 或全图。**

## #1394 前置仍开放

当前 issue OPEN，五项验收均未勾选。其当前正文要的是正式会话、真实 E Reserve、B Prepared/Commit 持久回执、同源公共观察、独立账簿 oracle、新进程重放与所改持久边界恢复；内存骨架及测试文件内的 FinancialSession 不足。

[更正评论](https://github.com/xy7365527-lang/NewChanlun/issues/1394#issuecomment-6116542024) 已撤回 PR1476 五项通过结论，恢复候选 `codex/1394-audit-index-20261008@88bf08b7c3` 是当时记录，本次未据此声称该工位仍在。

[PR1476](https://github.com/xy7365527-lang/NewChanlun/pull/1476) 本次实时状态 **CLOSED / isDraft=true**，HEAD `157d863179b4512c2798969f73fdcfb88f8c4686`；不能按旧记忆说它仍是开放 draft，也不能据 CLOSED 称已合入。

[最新 policy 回填](https://github.com/xy7365527-lang/NewChanlun/issues/1394#issuecomment-6153728414) 要 `quantity_policy_id` 引用与 `transfer_escrow` 责任类目/释放触发；数值未签前只允许 AwaitingPolicy / NotEvaluable / InsufficientEvidence，不替用户填默认。这里暂不横拆或重做实现；本轮建议集中恢复 C6a CI 装配，#1394 的完整生产接续仍需其独立验收工位处理。

无任何结论支持关闭 #1406、#1394、#1323、SPEC #1340 或 #1279。

原始本轮 issue JSON 已保存在本目录 `issue1406-economic.json`、`issue1394-economic.json`。旧临时证据目录丢失时，应以远端提交和上述原始 CI/评论为恢复入口，不能引用其不存在的本机文件作为新验收证据。
