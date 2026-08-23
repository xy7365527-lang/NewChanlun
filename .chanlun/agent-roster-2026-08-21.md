# Agent Roster — 2026-08-21

| Agent | 类型 | 模型 | 任务 | 关联票 | 状态 |
|---|---|---|---|---|---|
| `decision-1145-context-codex` | Codex CLI 决策落文工蜂 | `gpt-5.6-sol` | 将 LAV post-implement/pre-review shadow ranker 接缝写入 CONTEXT.md | #1145 | 已完成（`97b1b77ab5`） |
| `review-1145-context-codex` | Codex CLI 独立决策评审 | `gpt-5.6-sol` | 核验 LAV 唯一职责/接缝/禁止覆盖与 CONTEXT 格式 | #1145 | 首轮 FAIL：LAV 与旁路排序器未显式绑定 |
| `sandcastle-158-implementer` | Sandcastle 实装工蜂 | `openai-codex/gpt-5.6-sol` | 验证顶层子树清仓后声部栈空性并固化端到端断言 | #158 | 已完成（`14cd19db5d`）；优先级误判，分支停放、不合入 |
| `sandcastle-158-reviewer` | Sandcastle 独立评审工蜂 | `openai-codex/gpt-5.6-sol` | 对 #158 分支做独立评审并直接修正 | #158 | 已完成（无追加 commit）；分支停放、不合入 |
| `wayfinder-history-afk-auditor` | RLM 只读历史审计工蜂 | `gpt-5.6-sol` | 恢复历史会话中缠论量化 AFK 承诺、票号与未回写约束 | 待建 map | 已取消（改为直接走现有图） |
| `wayfinder-tracker-afk-auditor` | RLM 只读 tracker 审计工蜂 | `gpt-5.6-sol` | 全量对账旧票、依赖、重复吸收、AFK/HITL 与当前 frontier | 待建 map | 已取消（改为直接走现有图） |
| `wayfinder-lineage-afk-auditor` | RLM 只读路线审计工蜂 | `gpt-5.6-sol` | 对账 ROADMAP、旧 maps、ADR、教义正本与历史执行日志，恢复工作线谱系 | 待建 map | 已取消（改为直接走现有图） |
| `fix-1145-context-codex` | Codex CLI 决策落文修正 | `gpt-5.6-sol` | 显式绑定 LAV 与旁路排序器唯一名分 | #1145 | 已完成（`bebf6f636c`） |
| `review2-1145-context-codex` | Codex CLI 决策修正复审 | `gpt-5.6-sol` | 核验 LAV 唯一名分阻断闭合 | #1145 | FAIL：仍允许非权威 correctness |
| `codex-1087-resume` | Codex CLI AFK 实装工蜂 | `gpt-5.6-sol` | 续接 Lean 镜像第一切片，完成语义覆盖、Rust↔Lean 对拍与无 sorry 定理签收 | #1087 / map #1055 | 首轮失败：模型缓存刷新超时后楔住；改动已备份，精简配置重试 |
| `fix2-1145-context-codex` | Codex CLI 决策落文修正 | `gpt-5.6-sol` | 禁止 LAV 产生任何权威或非权威 correctness/pass-fail | #1145 | 已完成（`334b629f84`） |
| `review3-1145-context-codex` | Codex CLI 最终决策复审 | `gpt-5.6-sol` | 核验 LAV 输出仅相对排序+不确定性元数据 | #1145 | 已完成：PASS |
| `codex-1087-retry` | Codex CLI AFK 实装工蜂（精简配置） | `gpt-5.6-sol` | 从已备份未提交面续跑 #1087；完成真实窗对拍、定理、验证并提交 | #1087 / map #1055 | 已结束：提交 `309458b188`；仅最小切片，真实三窗与独立 Lean 重算未签收 |
| `prime-1087-acceptance` | Prime Agent 验收监理 | `claude-sonnet-5` | 独立核对 #1087 票面、Codex 最终产物和新鲜验证证据，并显式回报父会话 | #1087 / map #1055 | 失败：初始任务与追发均返回空消息，未形成独立验收 |
| `decision-1148-adr-codex` | Codex CLI 决策落文工蜂 | `gpt-5.6-sol` | 记录 LAV 候选证据包、失败/安全/重放契约 | #1148 | 已完成（`9160712131`，ADR 0025） |
| `prime-1087-redo-lead` | Prime Agent 重做主控 | `claude-fable-5` | 统筹 #1087 全量重做：数据取证、独立 Lean 重算、Codex 实装、异质验收 | #1087 / map #1055 | 失败：anthropic provider 空回复结束，未执行 |
| `prime-1087-pi-handshake` | Prime Agent 通道握手 | `prime-inference/claude-fable-5` | 验证父子消息链，成功后接管 #1087 重做 | #1087 | 失败：HTTP 402 Insufficient balance，0 token |
| `prime-1087-inherited-handshake` | Prime Agent 通道握手 | `openai-codex/gpt-5.6-sol`（继承） | 验证继承当前模型/认证的父子消息链 | #1087 | 完成：显式 READY 回报成功 |
| `prime-1087-redo-impl` | Prime Agent 实施工蜂 | `openai-codex/gpt-5.6-sol`（继承） | 重做 #1087：真实三窗 + 独立 Lean 重算 + 定理与签收证据 | #1087 / map #1055 | 实装完成 `b4920e6c33`；自报 PASS，待双轴独立验收 |
| `review-1148-adr-codex` | Codex CLI 独立决策评审 | `gpt-5.6-sol` | 核验 ADR 0025 覆盖 #1148 全部已确认契约 | #1148 | 首轮 FAIL：Standards 1 + Spec 7 阻断 |
| `prime-1087-review-standards` | Prime Agent 独立评审 | `openai-codex/gpt-5.6-sol`（继承） | 固定面 `355b839c29..b4920e6c33` 规范/代码风险评审 | #1087 | FAIL：CI 未执行、缺 checkpoint/frontier、事件/c_p 字段漏验（3 HIGH） |
| `prime-1087-review-spec` | Prime Agent 独立验收 | `openai-codex/gpt-5.6-sol`（继承） | 固定面逐项复验 #1087 / SPEC S3 / ADR 0025，含真实三窗 | #1087 | FAIL：Trend ID 伪独立、CandDelta 19→10 字段裁剪/循环输入、c_p 附着缺失 |
| `prime-1087-repair1` | Prime Agent 修复工蜂 | `openai-codex/gpt-5.6-sol`（继承） | 修复双轴评审 6 项阻断：CI/checkpoint/frontier/ID/CandDelta/c_p | #1087 | 修复提交 `7e40f81f5a`；自报 PASS，待复审 |
| `prime-1087-review2-standards` | Prime Agent 独立复审 | `openai-codex/gpt-5.6-sol`（继承） | 最终面 `355b839c29..7e40f81f5a` 规范与假绿复审 | #1087 | FAIL：零窗/L2 未锁、CandDelta 非双射、价格 tick 错 10^6（3 HIGH） |
| `prime-1087-review2-spec` | Prime Agent 独立终验 | `openai-codex/gpt-5.6-sol`（继承） | 最终面按票复跑真实三窗与完整独立重算 | #1087 | FAIL：c_p raw ID 守卫未重算、CandDelta 集合非双射 |
| `prime-1087-repair2` | Prime Agent 二轮修复工蜂 | `openai-codex/gpt-5.6-sol`（继承） | 修复零窗/L2、事件双射、tick 坐标、c_p ID/level/ordinal 守卫 | #1087 | 修复提交 `2aeaab5550`；自报 PASS，待第三轮复审 |
| `fix-1148-adr-codex` | Codex CLI 决策落文修正 | `gpt-5.6-sol` | 补 exact schema、set manifest、状态机、replay、删除与异源判定 | #1148 | 已完成（`2b38d2d94a`） |
| `prime-1087-review3-standards` | Prime Agent 第三轮规范复审 | `openai-codex/gpt-5.6-sol`（继承） | 最终面 `355b839c29..2aeaab5550` 全历史假绿/CI/质量复审 | #1087 | FAIL：P2 同源 producer、趋势证据 after 回灌、accepted/event 非空未锁（3 HIGH） |
| `prime-1087-review3-spec` | Prime Agent 第三轮票面终验 | `openai-codex/gpt-5.6-sol`（继承） | 新 target 重跑真实三窗与主动负控 | #1087 | PASS：fresh 12/12 stages、5/5 tests、主动负控全有效 |
| `prime-1087-repair3` | Prime Agent 三轮修复工蜂 | `openai-codex/gpt-5.6-sol`（继承） | 修复 P2/c_p 低层 raw 重算、趋势证据、非空门与 cfg/roster 收口 | #1087 | `7b569b5cad`；Prime 核验 PASS，待第四轮独立双轴复审 |
| `review2-1148-adr-codex` | Codex CLI 修正复审 | `gpt-5.6-sol` | 靶向复核 Standards 1 + Spec 7 阻断闭合 | #1148 | FAIL：2 项精确字段遗漏 |
| `fix2-1148-adr-codex` | Codex CLI 决策落文修正 | `gpt-5.6-sol` | 补 bundle comparison_context_digest 与 model_id 原键名 | #1148 | 已完成（`032a8182a8`） |
| `review3-1148-adr-codex` | Codex CLI 最终决策复审 | `gpt-5.6-sol` | 核验 comparison context 与 model_id 最后阻断闭合 | #1148 | 已完成：PASS |
| `prime-1087-review4-standards` | Prime Agent 第四轮规范复审 | `openai-codex/gpt-5.6-sol`（继承） | 最终面全历史 hard finding 与同源回声攻击 | #1087 | FAIL：事件列表无序双射漏排序契约（1 MEDIUM）；full-trend 正向锁 1 LOW |
| `prime-1087-review4-spec` | Prime Agent 第四轮票面终验 | `openai-codex/gpt-5.6-sol`（继承） | 新 target 真实三窗 + 主动负控终验 | #1087 | PASS：12/12、5/5，主动负控全有效；forged bCenterId 疑点撤销 |
| `prime-1087-repair4` | Prime Agent 四轮窄修工蜂 | `openai-codex/gpt-5.6-sol`（继承） | 锁 CandDelta 事件稳定顺序 + full-trend 正向/反向默认锁 | #1087 | 已取消：违规派发；PID 38145 停止，未提交面已还原，不采用产物 |
| `arch-loop-watcher` | RLM heartbeat（本会话内部定时器，follow_up，15 分钟一轮） | 继承（本会话模型） | 核对架构深化循环触发三条件（#1081 CLOSED ∧ 无 open wayfinder:map ∧ #1055 索引写全）；全满足时执行下轮开场对账并启动扫描前探索，不满足则静默跳过 | #1159 | running |

## #1087 收尾（2026-08-21）

- 最终分支 `issue-1087-lean-mirror`（尾 `6be6a2b363`）→ merge `052b7f1907` 合入本地 main，获批 push；远端 origin/main=b1640cb2b7。
- 签收：AAPL/MSFT/NVDA 12/12 stages、5 passed/0 failed/0 ignored、lake 155 jobs；尾部 Standards/Spec 双轴复核 PASS。
- CI run 32527197919：test ✓ fixture-drift ✓ rust-check 红（既有，#1147 e0a52e8752 引入，去向 #1163）；DevSkim 红（#1111 基线债，本票 0 命中）。
- #1087 已关；工位已清（worktree ×2、branch ×5）。
| `trading-arch-research` (sub-bb248313) | RLM 只读考古子代理（继承模型） | 继承 | 仓内考古 trading/level_operating_unit + positional_fusion 职责/消费/测试面，产拆分建议报告 | #1189 | running |
| `rfc-1126-drafter` | RLM 子代理（继承模型） | 继承 | 写 Prime 上游 remote-child/v1 RFC 草稿（#1126 AC 7 条，自包含不泄私有） | #1126 | 已完成（草稿+5 MINOR 修订；已贴上游 #1571 comment-18124367） |
| `review-1128-registry` | RLM 只读评审子代理（继承模型） | 继承 | 双轴评审 #1128 registry 两 commit（Standards+Spec 7 AC） | #1128 | running |
| `fix-1128-registry` | RLM 实装子代理（继承模型） | 继承 | #1128 registry 修复轮：MAJOR-2 + MINOR-4 落地（worktree sandcastle/issue-1128-fix @ origin/main） | #1205 | running |
