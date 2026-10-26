# #1621 / #1622 关票核验包（2026-10-25）

## 已落地

- PR #1688 经用户明确批准后 squash 合入 main：`13ed4d45c8ff85175819ca987690328ab2a247a4`。
- 最终候选 `78dc0449500d5ad82ef42b5304527beade24fc8e` 的 CI run `40118404588` 为 success。
- 非作者最终代码/合同独审为 PASS，原件 `INDEPENDENT-REVIEW.md`。BindVoice `skip_resend(21)` 明确抓域 1 Voice 缺失；C4b 仅调整 E/B 恢复顺序，生产时钟门未放宽。
- 既有实施报告已在 main：`.chanlun/review-results/issue1621-1622-clock-integration-20261024/IMPLEMENTATION.md`、`.chanlun/review-results/issue1621-main-regression-20261024/IMPLEMENTATION.md`、`.chanlun/review-results/issue1621-clock-repair-20261023/IMPLEMENTATION.md`。

## #1621 的 X 归属已明确迁移

本包附 `X-DOMAIN-RESEARCH.md`。2026-10-25 已先把验收写进既有票，再修订 #1621 指向：

- #1407：`E.ReleaseExact / X.AcceptExact`，接纳不得早于 E Release 提交；提前 1ns 负控不得产生接纳状态变化或场所调用。
- #1416：`S.ExecuteCommitted` 的实际 `execute_commit_ns` 与 `causal_refs`。
- #1422：`X.OwnAttemptAndCall`，实际调用不得早于 E Release 与 S Execute 两个提交；提前负控不得有 CAS、MayHaveCalled 或场所调用，E/S 收据与责任不变。

义务是迁移而非删除。#1621 只保留 S/E/B；B 书账转移和旧 Nautilus adapter 不冒充 X。未新增 `blocked by #1422`，避免 TB-03/TB-04 逻辑等待环；#1430 两道在途窗口不变。

## 关票前仍待完成

- main run `40135530860` 已完成且为 success（全部 job，含 extras 与 coverage）；这是 `13ed4d45c8ff85175819ca987690328ab2a247a4` 的主线验收，不再用 PR 旧 CI 代替。
- 本关票包需作为纯文档 PR 经用户另行批准合入；最终独审和 X 勘察入 git 是关票门必要条件。
- 以下旧工位含未提交文件或修改，必须先逐项核对，不得直接 reset/clean：
  - `/Volumes/AgentStorage/issue1621-main-regression-20261024`：未跟踪 roster 与报告文件。
  - `/Volumes/AgentStorage/issue1621-clock-repair-20261023/checkout`：未跟踪 `IMPLEMENTATION-HIGH.md`。
  - `/Users/silencehan/Projects/NewChanlun/NewChanlun-pr1688`：`tb03c_kill_tests.rs` 有未提交修改。
- 最终关票评论须引实际 main 提交、main CI、仓内报告与工位处置；#1406、#1323 不随本包关闭。
