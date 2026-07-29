# 现场台账（scene ledger）——跨 session/harness 恢复唯一入口

**来历**：harness-engineering improve-harness playbook 实装（2026-07-19）。基线证据：kimi session 恢复现场花 ~15 次工具调用做 wire 考古 + 编排者中转；同日两个 harness（kimi/CC）各挂归并脚本并发撞车（p124_merge 双写同一 dump）。
**规则**：任何改变活体状态的动作（进程起/退、监视器挂/杀、待裁定挂起/解除）后**立即改写本文件**；任何 session 恢复时**先读本文件，再考虑 wire 考古**。全 harness 共用（kimi/CC 一视同仁）。
**核验**：末行 `last_verified` 时间戳 + 核验者；内容必须与现实一致（090：声明=能力）。

## 活体进程

| 进程 | PID | 状态 | 喂给哪关 |
|---|---|---|---|
| agent-90（kimi 子代理工位，#76 出场门真链切换） | task agent-w1zou41x | 已交付并验收通过（issue76-acceptance-20260721.md） | #76 ✅ |
| agent-4（kimi 子代理工位，#77 V4 三窗三臂复验） | task agent-n26obtg0 | 已完成：wf7 ✗ 照实否定 / wf8 ✓，#77 关票 | #77 ✅ |
| agent-5（kimi 子代理工位，#93 实装卡起草） | task agent-53qz5pxl | 在跑（只读+文档，零 cargo） | #93 卡 |
| ⚠️ lib 源冻结期：5a/5b 实装前**禁改 rust/src**（cargo test 会在源变更时重编译混源）；5a/5b 期间只做设计卡（docs），lib 解冻后才动 | — | 纪律 | — |

## 已完成（本台账登记后）

- **全量双跑中止（2026-07-19，编排者令）**：6/8 片已完成、shard2 至 3.0M+ 时停。已完成片与 PREFIX 段每 500k 里程碑 pending/views 计数与旧跑逐字一致（至 3.0M）。**编排者令：先完全解决超线性（5a/5b 线形化），再做一次性总重跑（届时 wave-1+5a/5b 合并封印）。**
- **wave-1 当前验证状态（如实登记，未封 4.6M）**：静态构造证明全过（wave1-static-verification-20260719.md）+ cargo test 1732/0 + 250k/1M stdout+dump 逐字一致 + p123 SHADOW 5,278 检查 0 mismatch + 双跑在线计数一致至 3.0M。**欠：4.6M 全量 dump diff=0——与 5a/5b 实装后合并总重跑封印。**
- **wave-1 验收定谳（2026-07-19，编排者裁定）**：总重跑封印**取消**——「3M 都一致已经证明了」——wave-1 按「静态证明 + 1732/0 + 250k/1M 逐字一致 + SHADOW 0 mismatch + 在线计数一致至 3.0M」验收成立。4.6M dump diff 不再要求。
- **wave-1+5a/5b 合并封印清讫（2026-07-27，#69 线）**：5a/5b 实装落毕（ticket-69：7c4df3e118 + e8d5a47f06），全量 4,613,599 bars 双跑 **dump diff=0**（双侧 SHA 7e2b7614…，各 1,703,528 字节）；0.5M–4.0M 里程碑 pending/views/triggers/reevals 逐字一致；终态语义计数全同（term_skips=2 双侧，R1 残余双胞胎化）。:17 欠账清讫。证据群 commit 0040e1043e（wave1-5a5b-full-double-run-seal-20260727.md）。照实：post 墙钟 +24%（并发口径），超线性尾部两翼同存（主导项在 5a/5b 面外）。

- **关② 收口（2026-07-19）**：验收报告落盘 `chanlun/review-results/p124-s8-tightened-acceptance-20260719.md`。terminal_confirmed=855（硬界 [748,1213]）、**Pan=748 精确（构成 31/525/192 逐格一致）**、**877=110/122/31 精确**、Trend=107（type1∧owner=B，+12 救回效应机制归因闭合）、CERT 三栏 4385/1941(全Trend)/0、bit-exact 五项=0、C-a=572 一致。**结论：通过，进入 wave-1 与⑧⑨队列。**
- **wave-1 三卡实装落毕（2026-07-19，kimi 手装）**：方案 1 单源化（LevelAsOfView.confirm_times + assemble 统算 + moves/provide 两侧查表 + 两个测试构造点补字段）；方案 2 窗化（divergence.rs::move_range_envelope 界化、signal.rs (b) 合流 + (c) 前锚界化、level_view.rs (d) c_terminal/c_end 界化+lo>hi 守护 + (e) blocks 界化）；方案 3 去重（无条件 push + sort 后 dedup_by）。**验证进度**：cargo test 1732/0 全绿 ✓；250k stdout+dump 逐字一致 ✓；1M dump 逐字一致、stdout 仅 elapsed 墙钟行差异 ✓；实测 prefix 30.3s→14.4s（2.1×）✓；p123 SHADOW 对拍 5,278 检查 0 mismatch ✓（task bash-n6xkzzm4）；全量双跑在跑（task bash-kotncsf3）→ 绿后 wave-1 验收落盘。

- **阶段3 五步序列完成（2026-07-19）**：signal→witness→kappa→reach→rdecomp 五步全部退出，活体进程清空（ps 核对无残留计算作业）。对应执行脚本 `/tmp/stage3_execute.sh`，p127 §4.3 前置逐项收口。
- **⑧ M7 witness 报告落盘**：witness η 三行增打（additive，`cum_holding_cost` + `η_corrected`）已随实装入库；M7_WITNESS_A10 env gate 接入三测试（runner.rs :5146/:5232/:5335）。
- **⑨ M8 首跑报告已归档 + 3 项缺口处置**：§5.2 η 列口径已拍板 (i) 并入库（commit `640609071d`，`wverify_run.rs:1244-1245/1289-1295/1317-1323`，η_corrected 为唯一合法判读口径，原列保留对照）；3 项缺口已处置登记（缺口明细见归档报告）。p127 §4.3 第 4 项已同步回填。

## 监视器/自动脚本（历史登记）

| 脚本 | PID | 状态 | 说明 |
|---|---|---|---|
| p124_s2_auto_merge.sh | 57506 | 已杀（03:36，kimi） | 使命完成（shard2 已退） |
| p124_s2_watch.sh | 52152 | 已杀 | 同上 |
| p124_r2_merge_fix.sh | 67200 | 已杀（03:37，kimi） | CC session 链式脚本；其 merge(72171) 保留 |

## 待办链（按依赖序）

**已完成（本 goal 循环）**：
- 旧主线 goal（11 关）全部收口
- m8 端到端偏离诊断 goal：①OPSEM 重跑 bit-exact 全过 → ②trades.jsonl 8 维拆分 → ③6 门核验偏离清单 → ④750 行修复方案 P1-P7 → ⑤端到端结论（判定不充分，signal 独立缺口并存）→ ⑥两份深度研究（E2E 原型对照 715 行 + Consume_at 形式化审计 24KB）→ ⑦L0-L5 分层修复路线图 511 行 → ⑧5a/5b 实装前检查（行号锚核验 + 硬阻塞项登记）

**下一波（待编排者决策点）**：
- D1：L0/L1 最小诚实化可立即发包（m8-fix-plan L1-P1-P7 + roadmap §四 实装卡 C0-C6 已就绪，FIX 裁定 A/B/C 已在案不需新裁定）
- D2：L3 econ 门接入需实装授权（建议先离线过滤率探针 L3-1，再裁 gated 入口）
- D3：L5 塔内原生 Consume_at 需长线立项（Q1-Q8 预研清单在案，横切⑪长线）
- D4：5a/5b 线形化实装——阻塞②（R1-R3）已于 2026-07-21 编排者确认解除（g1-g2-window-ban-ruling §4）；阻塞③（wave-1 验收封口）2026-07-19 定谳解除；**残余阻塞①：wave-1 未提交（禁 git mutation，编排者自决）**
- D5（N3 A 线）：#74-#79/#94/#76/#77 全关 ✅。goal 判据①②③④全部闭合（① p125 B 豁免 2026-07-21 编排者拍板生效）。
- D6（文档基座）：记分卡/聚合/勘误/设计/密度调研均落盘。
- D7（B 线 runner.rs 拆分）：**全链 ✅ 关票**——#83 方案一+ → SPEC #84 → 八票 #85-92 全落地（runner.rs 8498→5888 行，五 seam 模块各自独立，1819/0 bit-exact 全绿）；评审票 #95-104 全关。
- D8（A 线后续）：#82 T2 锚定+双口径 ✅ 关票（1818/0，评审 #95 通过）；#93 churn 修复 ✅ 关票（Rc COW 指纹自溃修复，wall_on 430s→12.3s，events_seen 166M→1.27M 降 131×，入账 770 bit-exact，1819/0；评审 #96 通过）。
- D9（#106 typed miss 归因调研 ✅ 落盘）：4,673 miss 全部 B·教义结构（跨级链根本不存在），bug=0/键域错位≈0——与密度调研 ~300× 裁定一致。
- D10（待编排者）：#105 wf7 ✗ 后续方向 grilling（Q2 已呈：门形态重议 vs 接受存档；#106 归因排除覆盖率治理/出场侧先补）。

**长期挂起**：
- Gap-1 signal 定案（回填 wverify_run.rs:1234-1239 写死 25 桶文本为关②后新口径，代码改动越出文档工位）

## 待编排者裁定（挂起中）

- **R1-R3 确认清单**（`chanlun/escalate/r1-r3-ruling-confirmation-checklist-20260720.md`，5a/5b 硬阻塞②）：R1 失效链+TURN 残余 / R2 游标 LevelDerived 旁 / R3 memo RunEntry 旁——各回「确认/驳回/改裁」。
- **G1/G2**（task-101 正式版遗留，`cert-bsp-binding-ruling-20260721.md`）：G1 判据层不可见升格硬约束（推荐升格：namespace+grep 门挂 ci.yml）/ G2 240 固定 vs 级别自适应（推荐固定 240+先补分层报告）。烤料 = agent-83 调研结论（2026-07-21 会话内呈过）。

## 注意事项

- **merge 撞车实录**：68763（merge_and_check 之子）被 kimi 误杀（读错进程树）；72171（merge_fix 之子）保留。两 merge 命令逐字相同、确定性输出 ⟹ 无语义损失，但对账 §3–7 必须重跑。
- shard2 最终产量：terminal_confirmed=635、covered_b=1049、missed=0（/tmp/p124_r2_s2.out 末行）。
- **评审票规则（2026-07-21 编排者裁定，立）**：凡改 rust/src 的实装票配影子评审票（blocked-by 实装票，ready-for-agent；验收 = 双轴报告落盘 + 明确判定 + 新上下文评审禁自评 + 硬违规开修复票）；纯文档/退役票豁免。首批：#95←#82、#96←#93、#97-104←#85-92。存量不补（#74-#76/#78/#94 评审锚已在记分卡）。
- **判定≠裁定（2026-07-21 编排者明确）**：对照既定验收线的机械核对（如 #77 V4 三臂读数 vs wf7/wf8 线）= task 型判定，agent 照实自判自关、呈结果通报，**不呈拍板**；只有改定义/范围/规则/教义的判断题（grilling 型，如 p125 豁免、评审票规则、R/G/#8x 教义裁定）才呈编排者。待拍板队列当前仅 p125 备忘录一件。
- 主仓 /Users/silencehan/Projects/NewChanlun 绝对禁写；全部产物落 worktree。
- 双账号：**当前会话 = managed:kimi-code（国内 OAuth 账户验证，default_model=kimi-code/k3）**；kimi-old（API key 老账号）已见底停用。降级链：managed → zai-coding-plan → claude/fable5（claude CLI）。session 切换会杀死后台子代理（agent-88 曾被 killed 一次，resume 可续）。

## 参考仓库（耐久路径）

- **harness-engineering**（lopopolo）：`~/Projects/harness-engineering`（git clone，2026-07-19 装）。用途 = 改进本 harness 时的路由文集；入口读其 `AGENTS.md`（应用路由），改进单个作业用 `playbooks/improve-harness.md`（baseline→最早断点→最小属主干预→原生验证→全新重跑→留/改/删），仓库级评审用 `playbooks/repository-review.md`。本台账即按该 playbook 实装的第一个干预。

last_verified: 2026-07-21 ~20:45 UTC by kimi（大批量收口：#82/#93 实装+验收+评审 #95/#96 全闭合；B 线 #85-92 八票全落地关票（runner.rs 8498→5888 行五 seam），评审 #97-104 全关；p125 B 豁免签字；#106 归因落盘（miss 全 B·教义结构）；当前 1819/0 全绿；唯一待 = #105 grilling Q2 待编排者；账号 = managed:kimi-code 国内 OAuth）
