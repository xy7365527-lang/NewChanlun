# Agent Roster — 2026-07-18（CC 主控接管 Kimi 主线；主仓禁写期间落 worktree 镜像）

接管背景：Kimi Code k3 蜂群因 403 配额于 22:33 暂停；goal = mainline-merged-roadmap-20260717.md 三阶段 10 关；阶段 1+2 全落（cargo test 1732/0），关② 终验 S=8 重放在跑。CC 主控（Fable 5，只调度不执行）接续。

| # | 类型 | 模型/载体 | 任务 | 状态 |
|---|------|----------|------|------|
| K-1 | Kimi 遗留后台 | 本机进程 76851 | p124_shard_replay s2（S=8 关②终验重尾分片） | dead（Kimi 断额退出连带 SIGHUP，止于 3.5M/4.6M） |
| K-2 | Kimi 遗留后台 | 本机进程 9472 | p124_r2_merge_fix.sh 守护归并 | dead（同上；曾空跑归并出 SHARD_TERMINAL_SUM=220，缺 s2，作废） |
| C-1 | CC 主控裁定 | Fable 5（本体，裁决簿记） | p126 §5.2 η 口径裁定→采(i)，落 chanlun/escalate/m8-eta-caliber-ruling-20260718.md | done |
| C-2 | CC 后台监视 | Bash watcher | 守 9472 退出→输出 P124M_ 门行＋分片和 | failed→由 C-7 接替 |
| C-3 | 计划派发 | codex GPT-5.6 Sol（收口后） | 关②/③终验对账报告：P124M 产量对照硬界 [748,1213]/Pan=748/877=110/122/31/CERT 三栏 | pending（blocked：输入=C-2 归并输出） |
| C-4 | 计划派发 | codex GPT-5.6 Sol（C-3 后） | p126 §2.1 前置实装两件 additive：M7_WITNESS_A10 gate＋witness η 三行；m8_e2e η_corrected 列（C-1 裁定） | pending（blocked：cargo 禁开跑至重放收口，p121 §6-8） |
| C-5 | 计划派发 | 跑批（机械档，C-4 后） | p126 §2.2/§3.1 witness/κ网格/12窗/m8_e2e 跑批＋归档 | pending（blocked：输入=C-4 产物） |
| C-6 | codex 派发 | GPT-5.6 Sol（codex exec, workspace-write） | 端到端两文档（关⑪） | done（验收：0删/21增、M7/M8 逐字一致、E2E 编号 25 个一致、锚 8/8 抽真+54 全定位、6 处原文未决如实） |
| C-7 | CC 后台执行 | Bash（原二进制直跑，零 cargo） | 重跑 s2（P124_SHARD=2/8, DUMP_PREFIX=/tmp/p124_r2_s）→完成后自动 merge_fix 归并出 P124M_ 门行 | running（预计 ~3h） |
