# Agent Roster 2026-07-22

| 类型 | 模型 | 任务 | 状态 |
|---|---|---|---|
| general-purpose | opus | #138 出场证书 τ_γ 语义研究：读 alpha.pdf §12-14 / 严格alpha.pdf §1 / chi-dimension 材料 / runner.rs / kimi-nest worktree，产出 chanlun/review-results/exit-certificate-semantics-20260721.md + gh issue comment/close | 完成（报告 127 行已落盘并验收；gh 评论 issuecomment-5042875161 已核实；三个未决点留票 OPEN） |
| general-purpose | opus | #138 词汇表原文锚定：blog 原文逐字引文（A组9词）+ B组4词数学出处（alpha.pdf §12-14 等）→ chanlun/review-results/chanlun-glossary-sources-20260722.md | 完成（报告 327 行落盘；61课:38 / 14课:42 关键引文抽查逐字吻合；τ_γ 记号 PDF 无逐字出现标未验证；#138 评论 issuecomment-5043029540 已核实） |

| research | opus | issue #144 周期同构猜想研究报告（chanlun/review-results/cycle-isomorphism-conjecture-20260722.md，只读+单文件产出） | 完成（94行落盘已验证） |
| codex (后台) | GPT-5.6 Sol | #144 周期同构猜想 Lean 4 形式化判定 → chanlun/review-results/cycle-isomorphism-lean-20260722.md + formal/Research/CycleIsomorphism.lean | 完成（无条件同构证伪/条件链已证；0 sorry；lake env lean EXIT 0；ADR 未动，待用户裁定回填） |
| Explore (后台) | sonnet | #139 spec 前置侦察：theta_v0 出场逻辑/声部表示/账本/短差/测试接缝 6 问地图 | 完成（ExitType 已单源；生产 close 桶未 typed；4 个端到端测试先例已定位） |
| codex (worktree) | GPT-5.6 Sol | #147 T3：P1–P8 全互斥通道解释器 | 已撤销（用户改令：T 系列不走 codex，改派 Claude agent） |
| general-purpose (worktree) | opus | #147 T3：P1–P8 全互斥通道解释器（first-match + C0 兜底）+ TDD + cargo test 全绿 + commit | 派发中 |
| general-purpose (worktree) | opus | #149 T5：短差显式机制端到端（P4/P5 谓词实装 + CloseShortDiff/OpenShortDiff + 分腿账本 + TriggerProjectionSound）+ TDD + cargo test --lib 全绿 + commit 到 t5-149-shortdiff | 完成（fcd18cb389；主控复跑 1755 passed/0 failed/128 ignored；net +19 测试；未 push） |
| general-purpose (前台) | sonnet | #155 A/B 对照实验：econ_positive.rs gate_pass L1 C3 门放开，wf7 窗 trades 取数，报告 /tmp/wf155_report.md（只产数据不下结论；跑完恢复原行） | 完成（A/B 产物逐字节一致；dump 三窗覆写仅存 wf8，逐笔按 wf8 口径并标注；1168 行恢复经主控独立 sed 验证） |
| codex-exec | GPT-5.6 Sol | issue #162 代码考古：XZD 通道在 m8_e2e 惰性根因（四问，只读，报告→/tmp/issue162_report.md） | 进行中 |
