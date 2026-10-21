你是 NewChanlun 仓库的非作者独立审查者。只读审查，不改文件、不写 GitHub、不打印任何令牌或密钥。用简体中文输出。

## 审查对象

当前工作目录是 worktree，分支 `decision/pbatch-20261021`（PR #1658），相对 `origin/main` 的差异：`git diff origin/main...HEAD`。它把四张 P 类签署单的用户批复落进正本层：

- #1559 → 新 `docs/adr/0044-cost-stop-policy-signed-instance-v1.md`
- #1560 → 新 `docs/adr/0045-sr-v1-policy-signed.md`
- #1561 → `docs/adr/0028-first-capability-verification-profile.md` 追加「## P2 处置」节
- #1567 → 新 `docs/adr/0046-s-admission-anchor-issuance-and-takeover.md`；新实施票 #1657（`gh issue view 1657 --repo xy7365527-lang/NewChanlun`）
- `CONTEXT.md` 三处指针；`.chanlun/review-results/pbatch-land-20261021/acceptance.sh` 验收脚本

## 依据材料（只读）

- 已 PASS 的签署单草稿：`/Volumes/AgentStorage/decisions/<票号>/draft.md`（1559、1560、1561、1567），考古 `archaeology.md`
- 用户批复评论（gh 在 /tmp 下运行并带 `--repo xy7365527-lang/NewChanlun`）：`gh api repos/xy7365527-lang/NewChanlun/issues/comments/<id>`，id = 6310844461（#1559）、6310845841（#1560）、6310811399 与 6310848771（#1561）、6310847303（#1567）
- 批量问表 `/Volumes/AgentStorage/decisions/pclass-batch-20261021.md`
- 流程：`.sandcastle/docs/graph-engineering.md` §4–§5；AGENTS.md（收敛通则、引原文四步核、正本层名分）
- 落地要求：草稿里引「#1557 同批未签草稿」的出处须改为已签正本 ADR 0040:39/:41/:45

## 逐条判 PASS 或 BLOCK

- L1 忠实：每份正本的签署值与对应草稿裁定、用户批复逐项一致；没有新增、删减或改动用户已批的取值（特别核 θ、费率、四项状态、SR-1..4、P2 结论、锚名分与签发把关条件）。
- L2 引用回查：抽查每份 ADR 至少 5 处行号/评论引用在 `origin/main`（或注明的提交 `0ade553a`）上成立；ADR 0040 出处已订正；`qujiantao.md:467/:519`、`CONTEXT.md:169/:444` 等订正后的行号正确。引缠师原文的须过四步核（行号、逐字、正文界、行首/行内注）。
- L3 正本冲突：不与 `.chanlun/definitions/`、既有 ADR（尤其 0021、0028、0029、0030、0035、0037、0040、0041）、#799 收敛通则、#804 总缝规则冲突；不新增同一判断的宽严两档或失败兜底；ADR 0021 未被改动。
- L4 验收：在 worktree 根运行 `bash .chanlun/review-results/pbatch-land-20261021/acceptance.sh`，核对脚本忠实实现了四份草稿的验收清单（编号代入正确、没有放宽判据）；文件类项必须全部 PASS。票评论类项此时可能 FAIL（评论待独审后发出），合入后关票类项为 PENDING，均不计 BLOCK，但请说明。
- L5 边与票：#1657 正文写明 Part of #1323、引 ADR 0046、范围与验收与草稿二·1/二·2 一致；GraphQL 实查 #1657 parent = #1340、blockedBy 含 #1567、blocking 含 #1527。
- L6 越界：没有替用户做批复外的决定；没有触及 Destination 改写；没有凭据或私钥值；证据目录内无本地回环 URL。

## 输出

最后输出一个 JSON 代码块：`{"verdict":"PASS|BLOCK","items":[{"id":"L1","verdict":"PASS|BLOCK","evidence":"…","fix":"…"}…],"nonblocking":["…"]}`。BLOCK 必须给出可执行的修改点（文件:行与改法）。
