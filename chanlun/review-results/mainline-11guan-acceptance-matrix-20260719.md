# 总主线 11 关验收矩阵（2026-07-19，编排者=claude-fable-5 session 438f5dc7）

依据 goal（承接 kimi 030bee0b，2026-07-19 由用户正式设入本会话）："已收关以既有落盘产物直接验收、不重做"。
状态分级：**亲核**（本会话逐数核对）/ **在档待自核**（产物在，未逐项对照 Done-when）/ **在途**（工位在跑）/ **待启动**。
路径均相对 /tmp/kimi-nest-mainline/chanlun/。

| 关 | Done-when 摘要 | 落盘产物 | 状态 |
|---|---|---|---|
| ① | p116 存在性探针报告 | review-results/p116-turnpoint-anchor-existence-20260717.md | **亲核**：定稿报告六段全（定义/容差/§5 沉默案归因判定树/§4.3 两本账/§6 有效域/复现）；L0 锚结构性不可见已如实登记（090 合规） |
| ② | BSP 口径修复 + p92 全量重放对照 | review-results/p117-bsp-repair-design-{s0,s1a,s2s4,03720}-20260717.md、p117-h4-red-attribution-20260718.md、wave1-plan{1,2,3}-implementation-card-20260719.md、**p124-s8-merge-recon-20260719.md** | 机械收口**亲核** + 语义收口**裁定完结**（PASS-with-registered-gap，见 p124-s8-semantic-adjudication-20260719.md；20 存续=T1 §5 在册缝隙表达，源码 types.rs:202-221 亲核）|
| ③ | 877 池终端裁定 + U1–U3 落 escalate | escalate/nest-leftover-rulings-u1-u3-20260718.md、pan-terminal-endorsement-ruling-20260718.md、bsp-terminal-endorsement-t1-review-20260718.md | 裁定在档 + 硬界**亲核**：C-3 对账 terminal 1213→855、pan 748 零漂移、877 池 110链/122事件/272基例(31背书) 与裁定全等 |
| ④ | 小转大显式分支实装+单测 | 设计：p118-xiaozhuanda-branch-design-20260718.md；实装证据：`NestTurnClass` 进码（p92_nest_replay_postruling.rs:26,966 TURN_CLASS 投影）、econ_positive.rs:358 二通道准入门、nest.rs:421/446 力度真值 | 实装在码**亲核**；单测**亲核 PASS**：cargo test --lib 1732 绿 0 败（/tmp/c4c_test.out，2026-07-19），小转大相关测在内 |
| ⑤ | 嵌套子声部 + fill 双账本实装+单测 | 设计：p120-nested-voice-dual-ledger-design-20260718.md；实装证据：backtest/dual_ledger.rs 在档、`recognize_nested`×11（strategy/mod.rs）、exit.rs:72/93/97 `parent_invalid` 实义化（关⑤方案 A §3.5 注释自证） | 三件增量在码**亲核**；26 单测**亲核 PASS**：p120 §6 逐名核对 A6+B4+C4+D8 全绿 + E 组 3 绿 1 合规 ignore（E2 按 §6 原文「未接线前标记 ignore」，注释逐字相符；/tmp/c4c_test.out） |
| ⑥ | recover 触发裁定 + 衰减边界文档 | escalate/recover-trigger-nesting-ruling-20260718.md（另有 DRAFT 版并存） | **亲核**：正式版明示 supersede DRAFT（DRAFT 按不可篡改纪律原样保留，非缺口）；「衰减边界有效域声明」入题、文内 19 处；代理裁定生效授权域记载完备 |
| ⑦ | A10 成本模型实装+守恒断言+费率标定 | review-results/p121-a10-cost-model-design-20260718.md；生产证据 wverify_run.rs:1066-1074（q4_margin_model/m6_cost_model，冻结近似费率带 [L1机制/费率未标定]） | **亲核**：`conservation_residual` 入 wverify 输出表（wverify_run.rs:1146/1152，守恒残差≈0 资金无泄漏物证）；费率=未标定+强制口径标签（A10 附则B 裁决2，:1108/:1226）系被裁定接受口径，非缺口 |
| ⑧ | M7 witness 报告（A10 可行集内） | runbook：p126-stage3-runbook-20260718.md；预检：p127-stage3-precheck-20260719.md | 前置实装**完结 RECON_PASS**（#111：gate+η 三行入码，κ=0 基线 100k 三行读数在档 p126-s21-additive-c4 §5.2）；⑧⑨ 跑批 **C-5 收口**（#112 完结）：κ 网格 GRID_RC=0（四档全 CostReduction/n_orders=13034 不变，纯 L2 诊断）+ 多窗 Reach REACH_RC=0（8 窗全 Stage I/Reach=false）+ R 分解 RDECOMP_RC=0；总报告 p126-stage3-c5-20260719.md（provenance：HEAD fe03fd5c28 + 数据 shasum256 16ea13d5…、终局门 1732 passed/0 failed） |
| ⑨ | M8 Π_max-full 四层报告 | 裁定材料：p128-m8-eta-column-ruling-material-20260719.md | η 列**已拍板 (i) 且实装收口**（p128-eta-column-ruling-20260719.md → wverify_run.rs:1241-1295 additive，原列保留，1732 全绿门）；四层报告在档 p126-stage3-m8e2e-20260719.md（η_corrected 唯一判读口径注记 + [L1机制/费率未标定] 全标；#112 完结） |
| ⑩ | 090 修复（fugue:79 降级 + 悬空引用） | review-results/090-mirror-fixes-20260718.md | **亲核**：条目1（fugue:79 伪原文降级，含上游 spiral_physical_reinterpretation.md:106 双伪标归因）+ 条目2（TARGET_STRATEGY.md:54 / MAXFULL.md:270 悬空引用补档）两件齐；**主仓已应用+逐字节校验 ✓**（2026-07-19，438f5dc7 收口，用户解禁指令「帮他完成」） |
| ⑪ | 端到端定义段 + 原型文档 | review-results/doc-divergence-endtoend-prototype-20260718.md + plans/mainline-merged-roadmap-20260717.md 扩写段（14318B @18 Jul 23:51） | #113 完成 + **亲核**：双向交叉引用各 1 处在档（原型→roadmap / roadmap→原型） |

## 在途工位
- C-3（#110）已退场收割：产物四件自沙箱归运在档；semantic FAIL 经编排者复裁改判 PASS-with-registered-gap（p124-s8-semantic-adjudication-20260719.md）；#110 关账
- C-4（#111，codex）首派让位退场：WORKTREE_OWNER 互斥正常触发（台账第 3 行「待并发场景验证」**首次实证 ✅**，台账回写已完成：主仓 chanlun/harness/harness-ledger-20260719.md 第 3 行 ✅）；遗留改动已随 438f5dc7 收口提交归档
- C-4b（#111 续派，codex，pid 10459）：锁文件委派行授权进场，补 release 桥接 assert! + 门规单测 + 100k smoke + 报告，落标 C4B_EXIT → /tmp/c4b_impl.out；CARGO_BAN 已退休（cargo_gate.sh 现为透传）

## 编排者自核队列（按物质依赖排序）
队列已收敛：①②③⑥⑦⑩⑪ 亲核完结；⑨ η 已拍板 (i)；仅余 ④⑤ 单测计数核（取 C-4b 全量单测日志比对 p120 §6 五组 26 测）与 ⑧⑨ 跑批（C-5，blockedBy #111）。
