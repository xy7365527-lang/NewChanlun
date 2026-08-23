# Agent Roster — 2026-08-19

| Agent | 类型 | 模型 | 任务 | 状态 |
|---|---|---|---|---|
| `remote-main-merge-1101` (`sub-d2a42ba8`) | RLM 子代理（实施） | `openai-codex/gpt-5.6-sol` | [远端 main 谱系并轨：旧线归档 + ours merge + 无 force 快进](https://github.com/xy7365527-lang/NewChanlun/issues/1101) | 中断：仅完成旧远端归档，未 merge/push |
| `remote-main-merge-1101-resume` (`sub-a4f49b52`) | RLM 子代理（续办实施） | `openai-codex/gpt-5.6-sol` | 同票恢复：从已验证 archive 继续 ours merge 与无 force 快进 | 完成：`9bde23bc2b` 普通快进至 `origin/main`，父会话独立验收后关票 |
| `main-baseline-switch-1103` (`sub-b5e9b3db`) | RLM 子代理（实施） | `openai-codex/gpt-5.6-sol` | [全仓基线切回 main：CI / worktree / harvest / 纪律文档去特例化](https://github.com/xy7365527-lang/NewChanlun/issues/1103) | 被 Standards 评审打回：2 项；从 GPT-5.6 Sol 升到 GPT-5.6 Sol Pro 修正 |
| `review-1103-standards` (`sub-13fe0fa3`) | RLM 子代理（评审/Standards） | `openai-codex/gpt-5.6-sol` | `6177c3e55a` 对 `9bde23bc2b` 的标准轴评审 | 完成：2 findings（祖先闸缺失；三处正本重复） |
| `review-1103-spec` (`sub-1eb85c1d`) | RLM 子代理（评审/Spec） | `openai-codex/gpt-5.6-sol` | `6177c3e55a` 对 [全仓基线切回 main](https://github.com/xy7365527-lang/NewChanlun/issues/1103) 的规格轴评审 | 完成：PASS（0 findings） |
| `fix-1103-after-review` (`sub-26d31efd`) | RLM 子代理（评审修正） | `prime-inference/openai/gpt-5.6-sol-pro` | #1103 Standards 两项阻断修正 | 失败：provider 返回空响应，未改文件 |
| `fix-1103-standards-retry` (`sub-d477cd5a`) | RLM 子代理（聚焦重试） | `openai-codex/gpt-5.6-sol` | #1103 Standards 两项阻断修正 | 完成：amend 为 `ba449739ab`，待复审 |
| `rereview-1103-standards` (`sub-c44f69ec`) | RLM 子代理（复审/Standards） | `openai-codex/gpt-5.6-sol` | `ba449739ab` 标准轴复审 | 完成：PASS（0 findings） |
| `rereview-1103-spec` (`sub-1ff61f43`) | RLM 子代理（复审/Spec） | `openai-codex/gpt-5.6-sol` | `ba449739ab` 规格轴复审 | 完成：PASS（0 findings） |
| `merge-main-1103` (`sub-fb7fa201`) | RLM 子代理（合入/CI） | `openai-codex/gpt-5.6-sol` | #1103 `ba449739ab` ff-only 合入 `main`、普通 push、CI 取证 | 完成：核心 CI success；CodeQL/DevSkim 设置缺口转 #1106；关票 |
| `research-code-scanning-1106` (`sub-931f0501`) | RLM 子代理（Research） | `openai-codex/gpt-5.6-sol` | [私有仓 CodeQL / DevSkim：code scanning 未启用的授权与收敛路径](https://github.com/xy7365527-lang/NewChanlun/issues/1106) | 完成：报告 `83ac81dcc6`，待入 main |
| `merge-research-1106` (`sub-d4f4f10f`) | RLM 子代理（报告合入） | `openai-codex/gpt-5.6-sol` | #1106 报告 ff-only 入 main、CI 取证 | 完成：`83ac81dcc6` 入 main，核心 CI success，关票 |
| `security-workflows-1108` (`sub-f7fea742`) | RLM 子代理（实施） | `openai-codex/gpt-5.6-sol` | [安全工作流按授权收敛：禁用 CodeQL + DevSkim artifact/findings gate](https://github.com/xy7365527-lang/NewChanlun/issues/1108) | 实装完成：`7f69d9c8a0`，本地实扫 11,445 findings，待双轴评审 |
| `review-1108-standards` (`sub-b25fad6a`) | RLM 子代理（评审/Standards） | `openai-codex/gpt-5.6-sol` | #1108 `7f69d9c8a0` 标准轴评审 | 完成：3 findings，打回 |
| `review-1108-spec` (`sub-4df05cbb`) | RLM 子代理（评审/Spec） | `openai-codex/gpt-5.6-sol` | #1108 `7f69d9c8a0` 规格轴评审 | 完成：PASS（0 findings） |
| `harvest-fix-1098` (`sub-7e03a3b8`) | RLM 子代理（实施） | `openai-codex/gpt-5.6-sol` | [harvest.sh 合入判据修复](https://github.com/xy7365527-lang/NewChanlun/issues/1098) | 已停止：误解用户意图；零 tracked 改动/commit/push；孤立测试备份 `/tmp/nc-1098-harvest-runtime-stopped-20260819.sh` 后工位/分支清理 |
| `sandcastle-native-1110` (`sub-d5c9ed62`) | RLM 子代理（实施/上游勘察） | `openai-codex/gpt-5.6-sol` | [Sandcastle 官方 GitHub Actions 落地：agent:implement → Draft PR → agent:review](https://github.com/xy7365527-lang/NewChanlun/issues/1110) | 完成上游/scaffold/auth 勘察；用户改选 Prime 前停于零改动 |
| `sandcastle-native-prime-1110` (`sub-797be5ba`) | RLM 子代理（Prime 实施续办） | `openai-codex/gpt-5.6-sol` | #1110 官方 GitHub Actions + Prime Agent 薄适配 | 停于认证裁定：确认用户不用 Prime Inference；零文件改动/commit |
| `fix-1108-after-review` (`sub-e1e59e05`) | RLM 子代理（评审修正） | `openai-codex/gpt-5.6-sol` | #1108 Standards 三项阻断修正 | 完成：amend `af10903417`，待复审；11,445 findings 由 [分诊票](https://github.com/xy7365527-lang/NewChanlun/issues/1111) 承接 |
| `rereview-1108-standards` (`sub-18429539`) | RLM 子代理（复审/Standards） | `openai-codex/gpt-5.6-sol` | #1108 `af10903417` 标准轴复审 | 完成：PASS（0 findings） |
| `rereview-1108-spec` (`sub-70f5c095`) | RLM 子代理（复审/Spec） | `openai-codex/gpt-5.6-sol` | #1108 `af10903417` 规格轴复审 | 完成：PASS（0 findings） |
| `sandcastle-native-claude-1110` (`sub-a36e6eb1`) | RLM 子代理（官方安装续办） | `openai-codex/gpt-5.6-sol` | #1110 GitHub-hosted Sandcastle + Claude Code subscription | 已暂停：新硬要求“实际工蜂进入旧 Prime chat family”；零 tracked/commit/push，保留未跟踪上游草稿 |
| `merge-main-1108` (`sub-10924770`) | RLM 子代理（合入/CI） | `openai-codex/gpt-5.6-sol` | #1108 `af10903417` ff-only 入 main、CI/DevSkim artifact 取证 | 完成：main 三方一致；核心 CI 全绿；artifact 11,445 results；父验收关票 |
| `research-prime-remote-child-1115` (`sub-b980329d`) | RLM 子代理（Research） | `openai-codex/gpt-5.6-sol` | [Prime remote child 可行性研究](https://github.com/xy7365527-lang/NewChanlun/issues/1115) | 完成：报告 `11fc021849`；结论必须改 Prime core，待入 main |
| `merge-research-1115` (`sub-a33c9c1d`) | RLM 子代理（报告合入） | `openai-codex/gpt-5.6-sol` | #1115 报告 ff-only 入 main、CI 取证 | 完成：`11fc021849` 入 main，核心 CI success，父验收关票 |
| `devskim-triage-1111` (`sub-76fdd4c6`) | RLM 子代理（AFK 分诊） | `openai-codex/gpt-5.6-sol` | [DevSkim 11,445 findings 基线分诊](https://github.com/xy7365527-lang/NewChanlun/issues/1111) | 实装完成：`7415005cf7`，待双轴评审 |
| `remote-main-terminal-1104` (`sub-fbdd0a86`) | RLM 子代理（终验/远端退场） | `openai-codex/gpt-5.6-sol` | [远端主线终验](https://github.com/xy7365527-lang/NewChanlun/issues/1104) | 失败：读取阶段 tool call 卡死，零变更 |
| `remote-main-terminal-1104-resume` (`sub-0e308913`) | RLM 子代理（终验续办） | `openai-codex/gpt-5.6-sol` | #1104 从零变更状态恢复终验与已批准 mirror 退场 | 失败：第二次首个执行 tool call 悬空；远端零变更 |
| `remote-main-terminal-1104-opus` (`sub-4c1a2386`) | RLM 子代理（终验第三车） | `anthropic/claude-opus-4-8` | #1104 机械终验/archive/delete | 完成只读终验；因 main 良性前进至 `11fc021849` 停于写闸，零远端变更 |
| `draft-prime-remote-child-rfc-1126` (`sub-9d9b29eb`) | RLM 子代理（RFC 草拟） | `openai-codex/gpt-5.6-sol` | [Prime 上游 RFC：remote-child/v1](https://github.com/xy7365527-lang/NewChanlun/issues/1126) | 草稿完成：`0eeb9b3754`，待双轴评审 |
| `remote-main-retire-1104-final` (`sub-3a6c3ba6`) | RLM 子代理（远端退场最终写车） | `anthropic/claude-opus-4-8` | #1104 archive+删除 main-rewritten | 完成：archive 精确、活跃 mirror 删除、父验收关票 |
| `review-1111-standards` (`sub-f06abe1c`) | RLM 子代理（评审/Standards） | `openai-codex/gpt-5.6-sol` | #1111 `7415005cf7` 标准轴评审 | 完成：1 P2（driver name 未锁） |
| `review-1111-spec` (`sub-e686984b`) | RLM 子代理（评审/Spec） | `openai-codex/gpt-5.6-sol` | #1111 `7415005cf7` 规格轴评审 | 完成：1 partial（CLI/base 未真正固定） |
| `fix-1111-after-review` (`sub-4860e2f0`) | RLM 子代理（评审修正） | `anthropic/claude-opus-4-8` | #1111 pin immutable scanner + driver identity | 完成：amend `1afc7b7014`，待复审 |
| `rereview-1111-standards` (`sub-5ceb9ef3`) | RLM 子代理（复审/Standards） | `openai-codex/gpt-5.6-sol` | #1111 `1afc7b7014` 标准轴复审 | PASS |
| `rereview-1111-spec` (`sub-ccb8e238`) | RLM 子代理（复审/Spec） | `openai-codex/gpt-5.6-sol` | #1111 `1afc7b7014` 规格轴复审 | PASS |
| `review-1126-rfc-standards` (`sub-c4ed0fdb`) | RLM 子代理（RFC评审/Standards） | `openai-codex/gpt-5.6-sol` | #1126 `0eeb9b3754` 标准轴评审 | FAIL：3硬+Sprawl smell |
| `review-1126-rfc-spec` (`sub-082e0c39`) | RLM 子代理（RFC评审/Spec） | `openai-codex/gpt-5.6-sol` | #1126 `0eeb9b3754` 规格轴评审 | FAIL：parent-only/closure/STOP三项 |
| `fix-1126-rfc-after-review` (`sub-65b4c54f`) | RLM 子代理（RFC评审修正） | `anthropic/claude-opus-4-8` | #1126 七项阻断收敛/公开自包含/缩稿 | 完成：amend `3f98c14db8`，134行，待复审 |
| `rereview-1126-rfc-standards` (`sub-1c2ac166`) | RLM 子代理（RFC复审/Standards） | `openai-codex/gpt-5.6-sol` | #1126 `3f98c14db8` 标准轴复审 | PASS |
| `rereview-1126-rfc-spec` (`sub-e907de20`) | RLM 子代理（RFC复审/Spec） | `openai-codex/gpt-5.6-sol` | #1126 `3f98c14db8` 规格轴复审 | 1 finding：过度缩掉broker/version/slices |
| `fix-1126-rfc-final-spec` (`sub-b8b66ae6`) | RLM 子代理（RFC最终小修） | `anthropic/claude-opus-4-8` | #1126 补回极简broker/version/实施切分 | 完成：amend `68c9540b9b`，147行，待最终复审 |
| `finalreview-1126-rfc-standards` (`sub-0b475733`) | RLM 子代理（RFC终审/Standards） | `openai-codex/gpt-5.6-sol` | #1126 `68c9540b9b` 标准轴终审 | PASS |
| `finalreview-1126-rfc-spec` (`sub-090cf8aa`) | RLM 子代理（RFC终审/Spec） | `openai-codex/gpt-5.6-sol` | #1126 `68c9540b9b` 规格轴终审 | PASS |
| `publish-prime-remote-child-discussion-1126` (`sub-b23e49af`) | RLM 子代理（上游 Discussion 发布） | `openai-codex/gpt-5.6-sol` | #1126 双轴 PASS RFC 发布到 Prime Feature requests | 完成：[Discussion #1571](https://github.com/PrimeIntellect-ai/prime-agent/discussions/1571)，回读hash一致 |
| `merge-rfc-1126` (`sub-50b73a98`) | RLM 子代理（RFC合入/CI） | `openai-codex/gpt-5.6-sol` | #1126 `68c9540b9b` ff-only入main、CI取证 | 完成：三方一致、核心CI全绿；票OPEN等待Discussion #1571 |
| `rebase-1111-onto-main` (`sub-5b474f37`) | RLM 子代理（重基对拍） | `openai-codex/gpt-5.6-sol` | #1111 PASS候选重基 `68c9540b9b` | 完成：`31197f7e60`，5 blobs/diff/patch-id一致 |
| `merge-main-1111` (`sub-9d9fb939`) | RLM 子代理（合入/CI） | `openai-codex/gpt-5.6-sol` | #1111 `31197f7e60` ff-only入main、真实DevSkim取证 | 完成：DevSkim 268预期红；核心CI lld Bus error转 #1141；父验收关票 |
| `fix-security-pickle-1116` (`sub-f77ea790`) | RLM 子代理（AFK实施/诊断） | `openai-codex/gpt-5.6-sol` | #1116 安全 pickle cache | 完成：`46f343ce34` rebased，clean；无显式reply |
| `fix-fengliang-tls-1117` (`sub-6aa5c701`) | RLM 子代理（AFK实施/诊断） | `openai-codex/gpt-5.6-sol` | #1117 FengLiang TLS | 完成：`0fee3b2c9a`，clean；交接前需重基复验 |
| `fix-arxiv-https-1118` (`sub-37097939`) | RLM 子代理（AFK实施/诊断） | `openai-codex/gpt-5.6-sol` | #1118 arXiv HTTPS | 完成：`88d7507641`，5006 pass/DevSkim真首跳清零 |
| `diagnose-rust-lld-1141` (`sub-9a3e725c`) | RLM 子代理（AFK实施/诊断） | `openai-codex/gpt-5.6-sol` | #1141 CI rust-lld Bus error 诊断 | 完成：取证到target cache/88MB/并发linker，未产提交 |
| `review-1118-standards` (`sub-4e40dcdd`) | RLM 子代理（独立Standards评审） | `openai-codex/gpt-5.6-sol` | #1118 `88d7507641` Standards轴 | PASS |
| `review-1118-spec` (`sub-6a2c24cc`) | RLM 子代理（独立Spec评审） | `openai-codex/gpt-5.6-sol` | #1118 `88d7507641` Spec轴 | PASS |
| `merge-main-1118` (`sub-7e0b1006`) | RLM 子代理（合入/CI） | `openai-codex/gpt-5.6-sol` | #1118 `88d7507641` ff-only入main、DevSkim 268→267取证 | 完成：三方一致；#1141承接ENOSPC；父验收关票 |
| `push-retry-1118` (`sub-dbb167b4`) | RLM 子代理（同票push恢复） | `openai-codex/gpt-5.6-sol` | #1118 ordinary push网络悬挂有界重试 | 完成：发现远端已更新，按漂移闸零push退出 |
| `review-1116-standards` (`sub-9792f244`) | RLM 子代理（独立评审/恢复） | `openai-codex/gpt-5.6-sol` | #1116 Standards轴 | FAIL：2 HIGH/3 MED/2 LOW |
| `review-1116-spec` (`sub-56bb58a0`) | RLM 子代理（独立评审/恢复） | `openai-codex/gpt-5.6-sol` | #1116 Spec轴 | FAIL：2个安全测试空锁/P2证据缺口 |
| `recover-1117-handoff` (`sub-7bf850d7`) | RLM 子代理（独立评审/恢复） | `openai-codex/gpt-5.6-sol` | #1117 重基/复验/交接恢复 | 完成：`1fc4c9fb96`，8 tests/build/DevSkim 0 |
| `recover-rust-lld-1141` (`sub-440011e3`) | RLM 子代理（诊断恢复/实施） | `openai-codex/gpt-5.6-sol` | #1141 磁盘/cache/并发 fail-loud mitigation | 完成：回收prior patch/重基，未产提交 |
| `finish-rust-lld-1141` (`sub-af46c82d`) | RLM 子代理（最终实施） | `openai-codex/gpt-5.6-sol` | #1141 ENOSPC最小修复与证据文档 | 完成：`d45c26a9dc`，clean |
| `fix-review-1116` (`sub-022a4aef`) | RLM 子代理（评审修复） | `openai-codex/gpt-5.6-sol` | #1116 修复2H/3M/2L+2P2 | 完成：`d97d937ad3`，76 tests/DevSkim目标0 |
| `review-1117-standards` (`sub-0001c12a`) | RLM 子代理（独立Standards评审） | `openai-codex/gpt-5.6-sol` | #1117 `1fc4c9fb96` Standards轴 | FAIL：1H/2M/2L |
| `review-1117-spec` (`sub-a22aa8de`) | RLM 子代理（独立Spec评审） | `openai-codex/gpt-5.6-sol` | #1117 `1fc4c9fb96` Spec轴 | REQUEST CHANGES：不可用公网IP伪默认 |
| `review-1141-standards` (`sub-ce379be3`) | RLM 子代理（独立Standards评审） | `openai-codex/gpt-5.6-sol` | #1141 `d45c26a9dc` Standards轴 | NEEDS_CHANGES：1P2/2P3 |
| `review-1141-spec` (`sub-9ec5806f`) | RLM 子代理（独立Spec评审） | `openai-codex/gpt-5.6-sol` | #1141 `d45c26a9dc` Spec轴 | FAIL：lld探针必红+sampler吞错 |
| `fix-review-1117` (`sub-9818e020`) | RLM 子代理（评审修复） | `openai-codex/gpt-5.6-sol` | #1117 修复公网死默认/URL/多实例状态 | 中断：nested Codex孤儿，保留dirty改动 |
| `fix-review-1141` (`sub-c418520a`) | RLM 子代理（评审修复） | `openai-codex/gpt-5.6-sol` | #1141 修复1P2/2P3+2Spec blocker | 完成：`5edd6637d0`，16格矩阵全绿 |
| `recover-review-fix-1117` (`sub-6c902ceb`) | RLM 子代理（修复恢复） | `openai-codex/gpt-5.6-sol` | #1117 接管dirty diff、直办验证提交 | 完成：`fe3e9cbade`，26 tests/DevSkim0 |
| `rereview-1141-standards` (`sub-0d461916`) | RLM 子代理（独立Standards复审） | `openai-codex/gpt-5.6-sol` | #1141 `5edd6637d0` Standards复审 | NEEDS_CHANGES：2P3（统计口径/helper抽取） |
| `rereview-1141-spec` (`sub-94c72607`) | RLM 子代理（独立Spec复审） | `openai-codex/gpt-5.6-sol` | #1141 `5edd6637d0` Spec复审 | PASS；远端2轮待合入后 |
| `rereview-1117-standards` (`sub-1c148671`) | RLM 子代理（独立Standards复审） | `openai-codex/gpt-5.6-sol` | #1117 `fe3e9cbade` Standards复审 | FAIL：2M/1L浏览器/WS回归 |
| `rereview-1117-spec` (`sub-2b46b50c`) | RLM 子代理（独立Spec复审） | `openai-codex/gpt-5.6-sol` | #1117 `fe3e9cbade` Spec复审 | REQUEST CHANGES：2M/1L |
| `fix-1141-standards-p3` (`sub-497e92fd`) | RLM 子代理（P3修复） | `openai-codex/gpt-5.6-sol` | #1141 统计口径+telemetry helper抽取 | 完成：`57458afd`，helper/测试全绿 |
| `finalreview-1141-standards` (`sub-218f0e08`) | RLM 子代理（独立终复审） | `openai-codex/gpt-5.6-sol` | #1141 Standards终复审 | NEEDS_CHANGES：1P3 helper测试未接CI |
| `finalreview-1141-spec` (`sub-57add0f2`) | RLM 子代理（独立终复审） | `openai-codex/gpt-5.6-sol` | #1141 Spec终复审 | PASS；远端2轮待合入后 |
| `finalreview-1116-standards` (`sub-f55c74a1`) | RLM 子代理（独立终复审） | `openai-codex/gpt-5.6-sol` | #1116 Standards终复审 | FAIL：1M hyperedge multiplicity |
| `finalreview-1116-spec` (`sub-9f39be6e`) | RLM 子代理（独立终复审） | `openai-codex/gpt-5.6-sol` | #1116 Spec终复审 | FAIL：1H lazy cooccurrence归零 |
| `fix-1141-wire-helper-test` (`sub-0fcc8bb8`) | RLM 子代理（最终P3修复） | `openai-codex/gpt-5.6-sol` | #1141 helper测试接入必跑CI | 完成：`c9fc577c38` |
| `wire-review-1141-standards` (`sub-3d6fc953`) | RLM 子代理（接线终复审） | `openai-codex/gpt-5.6-sol` | #1141 `c9fc577c38` Standards | PASS |
| `wire-review-1141-spec` (`sub-4f9fe2ed`) | RLM 子代理（接线终复审） | `openai-codex/gpt-5.6-sol` | #1141 `c9fc577c38` Spec | PASS |
| `fix-1116-hyperedge-semantics` (`sub-183f5539`) | RLM 子代理（语义修复） | `openai-codex/gpt-5.6-sol` | #1116 hyperedge multiplicity+lazy cooccurrence | 完成：`07a912e692`，81 tests；待远端重基 |
| `merge-verify-1141` (`sub-f0daaf75`) | RLM 子代理（合入/连续远端验证） | `openai-codex/gpt-5.6-sol` | #1141 `c9fc577c38` ff-only + 2×workflow_dispatch | 停止：local main含#1130且禁push |
| `fix-1117-browser-ws-compat` (`sub-25f8f2b7`) | RLM 子代理（兼容性修复） | `openai-codex/gpt-5.6-sol` | #1117 AbortSignal/WS churn/错误分类 | 中断：tracked改动保留；待远端重基/收尾 |
| `merge-verify-1141-remote-only` (`sub-607064f5`) | RLM 子代理（remote-only合入/验证） | `openai-codex/gpt-5.6-sol` | #1141 显式SHA→origin/main，保留local #1130 | 停止：OAuth缺workflow scope，remote未变 |
| `merge-verify-1141-ssh443` (`sub-92099c29`) | RLM 子代理（SSH443合入/验证） | `openai-codex/gpt-5.6-sol` | #1141 官方meta验host key后SSH push | 完成：push CI+2 dispatch全绿；父验收关票 |
| `rebase-final-1116-onto-1141` (`sub-64067e26`) | RLM 子代理（远端基线重基） | `openai-codex/gpt-5.6-sol` | #1116 最终链重基origin/main@c9fc | 完成：`dfd37860cb`，range-diff等值/81 pass |
| `finish-rebase-1117-onto-1141` (`sub-17bbcbf5`) | RLM 子代理（远端基线重基） | `openai-codex/gpt-5.6-sol` | #1117 compat收尾+重基origin/main@c9fc | 完成：`91d8f45097`，33 tests/DevSkim0 |
| `postrebase-review-1116-standards` (`sub-32cb7ac8`) | RLM 子代理（post-rebase终复审） | `openai-codex/gpt-5.6-sol` | #1116 `dfd37860cb` Standards | PASS |
| `postrebase-review-1116-spec` (`sub-b8e5d24d`) | RLM 子代理（post-rebase终复审） | `openai-codex/gpt-5.6-sol` | #1116 `dfd37860cb` Spec | PASS |
| `postrebase-review-1117-standards` (`sub-4a5562b2`) | RLM 子代理（post-rebase终复审） | `openai-codex/gpt-5.6-sol` | #1117 `91d8f45097` Standards | PASS |
| `postrebase-review-1117-spec` (`sub-93a3f193`) | RLM 子代理（post-rebase终复审） | `openai-codex/gpt-5.6-sol` | #1117 `91d8f45097` Spec | PASS |
| `merge-verify-1116-remote-only` (`sub-15f01642`) | RLM 子代理（remote-only合入/CI） | `openai-codex/gpt-5.6-sol` | #1116 `dfd37860cb`→origin/main，DevSkim267→266 | 停止：keyscan缺RSA，零push |
| `merge-verify-1116-ed25519` (`sub-0dd1e7d7`) | RLM 子代理（ED25519 remote-only合入） | `openai-codex/gpt-5.6-sol` | #1116 官方meta ED25519→SSH443 push | 完成：CI绿/DevSkim267→266；父验收关票 |
| `rebase-1117-onto-1116` (`sub-bdd07b16`) | RLM 子代理（串联重基） | `openai-codex/gpt-5.6-sol` | #1117 `91d8f45097` onto #1116 `dfd37860cb` | 完成：`28acbdc8a8`，range-diff/20 blobs全等 |
| `merge-verify-1117-remote-only` (`sub-c6aa82db`) | RLM 子代理（remote-only合入/CI） | `openai-codex/gpt-5.6-sol` | #1117 `28acbdc8a8`→origin/main，DevSkim266→260 | 停止：官方/meta首次504，零push |
| `merge-verify-1117-meta-retry` (`sub-03ba1481`) | RLM 子代理（meta退避恢复合入） | `openai-codex/gpt-5.6-sol` | #1117 官方ED25519 SSH443 push | 停止：push/CI绿，DevSkim偏差转独立裁定 |
| `triage-1117-devskim-delta` (`sub-8b91b40f`) | RLM 子代理（Actions安全差分裁定） | `openai-codex/gpt-5.6-sol` | #1117 DevSkim266→268偏差独立分诊 | REQUEST_CHANGES：scanner-aware混淆 |
| `fix-1117-scanner-honesty` (`sub-c1442616`) | RLM 子代理（扫描诚实性修复） | `openai-codex/gpt-5.6-sol` | #1117 移除scanner-aware字符串混淆 | 完成：`b434cf9801`，诚实DevSkim274 |
| `honesty-review-1117-standards` (`sub-be8b96de`) | RLM 子代理（scanner诚实终复审） | `openai-codex/gpt-5.6-sol` | #1117 `b434cf9801` Standards | PASS |
| `honesty-review-1117-spec` (`sub-49687ffc`) | RLM 子代理（scanner诚实终复审） | `openai-codex/gpt-5.6-sol` | #1117 `b434cf9801` Spec/安全 | ACCEPT_CLOSE |
| `merge-verify-1117-honesty` (`sub-9b49c550`) | RLM 子代理（honesty remote-only合入） | `openai-codex/gpt-5.6-sol` | #1117 `b434cf9801`→origin/main，DevSkim274 | 完成：CI绿/诚实notes；父验收关票 |
| `implement-prime-remote-child-core-1142` (`sub-998bdbfa`) | RLM 子代理（Prime Core MVP实施） | `deepseek/deepseek-v4-pro` | #1142 remote-child/v1 durable admission纵切片 | 失败：DeepSeek 402 Insufficient Balance（4次退避均0 token），零改动 |
| `prime-core-1142-phase1` (`sub-01c09ef1`) | RLM 子代理（Prime Core Phase 1） | `deepseek/deepseek-v4-pro` | #1142 durable admission/replay/restart纵切片 | 失败：DeepSeek 402 Insufficient Balance（4次退避均0 token），零改动 |
| `codex-1142-phase1` (`pid 73915`) | Codex CLI实施 | `gpt-5.6-sol` high | #1142 durable admission/replay/restart纵切片 | 草稿完成：check绿/9 tests；独立评审FAIL，未提交 |
| `review-prime-core-1142-phase1` (`sub-63055a5b`) | RLM 子代理（Prime Core Phase1独立评审） | `deepseek/deepseek-v4-pro` | #1142 dirty candidate安全/规格审计 | FAIL：5H+7M+2L，9 tests非空壳 |
| `codex-1142-hardening` (`pid 79541`) | Codex CLI加固 | `gpt-5.6-sol` high | #1142 状态机/重投递/签名/journal/配额负路径 | 完成：check绿/19 tests，未提交 |
| `rereview-prime-core-1142-phase1` (`sub-c307fd52`) | RLM 子代理（Phase1加固复审） | `deepseek/deepseek-v4-pro` | #1142 逐项复测前轮findings | PASS：19 tests/无阻断 |
| `phase1-local-commit` (`138cf7412b96`) | 主控验收提交 | `n/a` | #1142 Phase1 durable admission core | 完成：check绿/19 tests/DeepSeek PASS；未push |
| `codex-1142-phase2a` (`pid 95455`) | Codex CLI实施 | `gpt-5.6-sol` high | #1142 daemon原父list/message/restart/family纵切片 | 完成并提交 `6ee6ef0d124f`：check/37 TS/10 Python tests |
| `review-prime-core-1142-phase2a` (`sub-78b4b62b`) | RLM 子代理（Phase2A快速评审） | `deepseek/deepseek-v4-pro` | #1142 `6ee6ef0d124f` 原父list/message/restart/family | BLOCKING：--no-session adapter回归 |
| `codex-1142-phase2b` (`pid 52814`) | Codex CLI实施 | `gpt-5.6-sol` high | #1142 steer/followup/stop/delete/reconnect | 完成并随 `dd65ea5426a5` 提交：check/23TS/10Python |
| `codex-1142-no-session-fix` (`pid 64876`) | Codex CLI阻断修复 | `gpt-5.6-sol` high | #1142 in-memory parent兼容回归 | 完成并随 `dd65ea5426a5` 提交：check/23TS/10Python |
| `review-prime-core-1142-phase2b` (`sub-adc0ed33`) | RLM 子代理（Phase2B快速评审） | `deepseek/deepseek-v4-pro` | #1142 `dd65ea5426a5` controls+no-session | PASS；gateway/timer列Phase3接线 |
| `codex-1142-phase3-gateway` (`pid 13317`) | Codex CLI实施 | `gpt-5.6-sol` high | #1142 daemon-owned mock gateway/process smoke | 完成：gateway `bfc8fd27`+mock guard `808e8959`；40 TS/10 Py/daemon-mode全绿 |
| `codex-1142-phase4` (`pid 47261`) | Codex CLI实施 | `gpt-5.6-sol` high | #1142 生产lease/stop定时器+真实daemon restart回归 | 失败：Codex额度用尽(至Aug27)，零改动 |
| `prime-core-1142-phase4` (`sub-b1a80be5`) | RLM 子代理（Phase4实施） | `deepseek/deepseek-v4-pro` | #1142 生产调度器+daemon restart回归 | 完成并提交 `a3fa40d14704`：check/23TS/daemon-mode全绿 |
| `review-prime-core-1142-phase4` (`sub-aef792d4`) | RLM 子代理（Phase4快速评审） | `deepseek/deepseek-v4-pro` | #1142 `a3fa40d14704` scheduler/restart | PASS |
| `prime-core-1142-live-smoke` (`sub-8ec7bc9d`) | RLM 子代理（隔离live smoke） | `deepseek/deepseek-v4-pro` | #1142 独立daemon+原父+worker全链 | PASS：全链10项绿，~/.prime/installed diff空 |
| `implement-1128-model-registry` (`sub-633bc828`) | RLM 子代理（AFK实施） | `deepseek/deepseek-v4-pro` | #1128 模型registry allowlist+fixtures | 完成：`d92a585578`，30 TS+verify全绿，未push |
| `review-1128-standards` (`sub-d8bbf742`) | RLM 子代理（独立Standards评审） | `deepseek/deepseek-v4-pro` | #1128 `d92a585578` Standards | PASS；3条非阻断L/I |
| `review-1128-spec` (`sub-898636c3`) | RLM 子代理（独立Spec评审） | `deepseek/deepseek-v4-pro` | #1128 `d92a585578` Spec | REQUEST_CHANGES：1P1子串误路由+1P2 |
| `fix-1128-label-matching` (`sub-bdf6cd62`) | RLM 子代理（评审修复） | `deepseek/deepseek-v4-pro` | #1128 标签精确匹配+最小EVENT_PAYLOAD+机械锁 | 完成：`0f400070`，TDD红绿/31 tests |
| `rereview-1128-standards` (`sub-f8c65ab8`) | RLM 子代理（修复Standards复审） | `deepseek/deepseek-v4-pro` | #1128 `0f400070` Standards delta | PASS；1非阻断锁建议 |
| `rereview-1128-spec` (`sub-eb0900d2`) | RLM 子代理（修复Spec复审） | `deepseek/deepseek-v4-pro` | #1128 `0f400070` Spec delta | ACCEPT；1非阻断大小写建议 |
| `merge-1128-registry` (`sub-b861cc7b`) | RLM 子代理（remote-only合入/CI） | `deepseek/deepseek-v4-pro` | #1128 `0f400070`→origin/main | 停止：remote漂移97ce1ed→5015dd2d，零push |
| `rebase-1128-onto-5015dd2d` (`sub-af71d624`) | RLM 子代理（远端基线重基） | `deepseek/deepseek-v4-pro` | #1128 chain onto `5015dd2d` | 完成：`85e0078f`，range-diff/patch-id/26 blobs全等 |
| `merge-1128-registry-retry` (`sub-73d5a039`) | RLM 子代理（remote-only合入重试） | `deepseek/deepseek-v4-pro` | #1128 `85e0078f`→origin/main | 完成：main=85e0078f；CI红=既有#1163；票OPEN待smoke |
| `extend-1128-review-deepseek` (`sub-4a7f4f55`) | RLM 子代理（registry扩展） | `deepseek/deepseek-v4-pro` | #1128 review显式DeepSeek+fixtures | 完成：`401c3134`，verify/31 tests绿，未push |
| `fix-1185-jq-escape` (`sub-12dc5bac`) | RLM 子代理（烟测bug修复） | `deepseek/deepseek-v4-pro` | #1185 jq转义+机械锁 | 完成：`565a8f04`，18条jq逐条解析/31 tests，未push |
| `review-1185-jq-review-ds` (`sub-a0d4c806`) | RLM 子代理（增量双轴评审） | `deepseek/deepseek-v4-pro` | #1185+reviewDS `85e0078f..565a8f04` | PASS：无阻断 |
| `merge-1185-chain` (`sub-7923a979`) | RLM 子代理（remote-only合入） | `deepseek/deepseek-v4-pro` | #1185 `565a8f04`→origin/main | DRIFT-STOP：85e0078f→5af15620，零push |
| `rebase-1185-onto-5af15620` (`sub-0ab2a107`) | RLM 子代理（远端基线重基） | `deepseek/deepseek-v4-pro` | #1185 chain onto `5af15620` | 完成：`d087a055`，零漂移核验全过 |
| `merge-1185-chain-retry` (`sub-d2187c11`) | RLM 子代理（remote-only合入重试） | `deepseek/deepseek-v4-pro` | #1185 `d087a055`→origin/main | 运行中 |
| `push-reset-retrigger-1173` (`sub-2d090dcc`) | RLM 子代理（push/reset/重触发） | `deepseek/deepseek-v4-pro` | #1173 verify对齐push+PR reset+review重触发 | 运行中 |
| `push-reopen-retrigger-1173` (`sub-5aadfebb`) | RLM 子代理（push/reopen/重触发二轮） | `deepseek/deepseek-v4-pro` | #1173 载荷marker+reopen PR1200+review触发 | 运行中 |
