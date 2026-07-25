# v1 真链端到端判定记分卡（goal 判据④ 前置骨架，随实装逐段回填）

- 日期：2026-07-21 起
- 性质：goal「v1 真链端到端实现 × 严格化」判据④ 要求的「v1 端到端判定（N2/N3/状态机逐段 ✓/◐/✗）」的记分骨架。每段判定必须带可检验证据锚（文件:行号 / 报告路径 / 测试名），不允许无锚判定。
- 图例：✓ 端到端实现且验收过；◐ 部分实现/有披露边界；✗ 未实现或验收未过。

## V1 级别标定（判据①）

| 子项 | 判定 | 证据锚 | 边界（照实） |
|---|---|---|---|
| 91 证书 lvl 绑定归因 | ✓ | v1-level-calibration-attribution-20260720.md（三层叠加机制归因：bar 距离判据无区分力/级别语义分叉/T1 移位错位；裁定 = 语义分叉非赋值 bug） | task-107 原文不在本 worktree（转引标注） |
| 判据升级（区间包含）下断裂消失 | ✓ | v1-level-calibration-fix-20260720.md：可判定 89/89=100%（exec=1 87/87、exec=2 2/2）；路 A 不动赋值 | — |
| 标定一致性断言 | ✓ | A1-A4 通过；A5 exist_mono=24/25 **如实 fail 登记**（exit=1，唯一残存 idx48 跨 src 链中间点 d=0，已归因落账） | A5 失败是照实登记不是通过 |
| cargo test 全绿零变红 | ✓ | 1770/0（V1 时点，v1-level-calibration-fix-20260720.md:115） | — |
| p125 级对账 | ✓（豁免定义） | 处置备忘录 escalate/p125-level-recon-disposition-memo-20260721.md——两总体不同类无可比主键，语义已被 V1 归因 + 关③ P1-P4 + task-101 T5 三处覆盖；**编排者 2026-07-21 拍板 B 豁免生效** | — |
| code-review 独立评审 | ✓ | code-review-v1-v3-64-20260721.md「V1 标定：逐锚吻合，无发现」（judgement：exit-code 固化经验期望待裁定确认豁免） | — |

## N2 rung 级力度门（判据②）

| 子项 | 判定 | 证据锚 | 边界（照实） |
|---|---|---|---|
| rung 级背驰力度谓词落地 | ✓ | nest.rs:775（`if event.side != side \|\| !event.divergence_confirmed { continue; }`）；T1-T8 单测 | — |
| cargo test 全绿零变红 | ✓ | 1770/0（V2 时点）→ 1794/0（#74 后） | — |
| 全量 4.6M 双跑 I1-I5 | ✓ | V2 验收报告 | XiaozhuandaCandidate 150→0（R-2 已裁迁出） |
| 链深分布重测——不再 95.36% 退化、有结构意义 | ✓ | v2-rung-force-gate-impl-20260720.md §6.3 全量 4.6M pre/post 双跑：深度表 pre→post（A 2767→1980、B 2518→984，收缩全在深度≥2）；I1 门后 2,964 张证书 confirmed_vec 恒全 '1'——存活链深 = 逐 rung 背驰确认层数，链深携带结构意义 | 判词精确化：95.36% 退化是 econ 塔外近似链（§7.3 未触，另一条链）；typed 链门后单级占比**上升**（A 30.9%→43.2%、B 34.0%→86.9%）是力度门裁剪效应非退化——「有结构意义」的载体是 I1 全真，不是单级占比下降。#74 索引 stats 口径（nest_index.rs NestChainDepthStats）备 #77 复测 |
| code-review 独立评审 | ◐ | code-review-v1-v3-64-20260721.md Standards #3（supersede 程序 judgement） | 裁定链程序确认待编排者 |

## N3 塔内原生证书链（判据③ 前半）

| 子项 | 判定 | 证据锚 | 边界（照实） |
|---|---|---|---|
| 生产侧证书索引构建器（#74） | ✓ | nest_index.rs（build_nest_certificate_index + NestCertificateIndex + 9 测试）；1794/0 | 键域 = 基例身份（单键）；rung 身份反查为多键扩展预留 |
| 进场门消费真链替代 v0 基例（#75） | ✓ | agent-84 交付：NestChainGate（runner.rs:1137-1468 + 接线 :2162-2180）；判定 = cert.certificate().n_delta() 单一来源；门关直通锁 :2179 逐字节不变；1799/0 | 评审（2026-07-21 双轴）发现 2 硬违规（声明不属实/cross_agree 灌水）→ #94 修复票；门开全量 4.6M 派生 churn 不可行照实登记 → #93 |
| 身份桥（source_index↔证书坐标系） | ✓ | #75 交付 §1：同 L0 原始 K 序确证（bsp.rs:114-115/signal.rs:399/level_view.rs:773-776/nest.rs:607-628）；值桥 = source_index == seg_c_full.1（turn_source 等值桥证否，#64 seg_c_full 正是载体）；级别桥 lvl↔ℓ+1（V1 T1 移位） | G2 窗口禁后身份绑定路径前提成立 |
| 出场门消费真链（#76） | ✓ | 交付 issue76-impl-20260721.md（方案 (b) 预注入 ExitNestGateCtx runner.rs:1618-1698 + exit.rs 第 8 参 reverse_cert）+ 验收 issue76-acceptance-20260721.md（21 处代码锚全吻合、cargo test 1807/0 实跑复核逐字一致、四红线全守住）；本票 5 新测试单跑 5/0 | π 路径出场不经 exit.rs 属后续票；30k 冒烟 typed_found=0（0/35），真链准出选择性待 #77 窗口评；评审 2 Low（旧术语注释残留/报告列举不全）不阻塞 |
| 进出场同装配源（禁第二查法） | ✓ | #76 验收红线②：判定唯一源 = runner.rs:1432 `cert.certificate().n_delta()`；装配四方法（sync_events/has_bridge_key/sync_index/typed_lookup）与 π 进场门（:1477/:2386/:2398/:2401）共用，零新查法 | — |
| nest_confirm 基例降级兼容读数、默认路径 bit-exact | ◐→✓（门关侧） | interp.rs:369-395 未动；门关直通锁 runner.rs:1731；#76 门关 == 基线 bit-exact 由 runner #76-T2（v1）/T3（dual）锁死，验收复核成立 | 门开臂全窗对照读数待 #77 三臂复验 |

## 活假设状态机（判据③ 后半）

| 子项 | 判定 | 证据锚 | 边界（照实） |
|---|---|---|---|
| 三态 Provisional→Confirmed/Invalidated | ✓ | nest_lifecycle.rs（T1-T8 + #64 升格 T9-T13 + #78 修复 T14/T15）；#78 code-review 落盘 code-review-issue78-20260721.md（带边界通过：untracked 新文件无法 diff 隔离，提交后追平；Low×1 retrograde_rejections 无去重不阻塞） | — |
| first_provable 钟落地、首次写入不后移 | ✓ | nest_lifecycle.rs（first_provable 仅 Verified(true) 分支写，#78） | — |
| Provisional 入谱系构建、Consume Closed-only | ✓ | #64 裁定 (a) + lineage_nodes()/consumable_closed() | — |
| Invalidated 可触发可查账 | ✓ | T11 真实 pan 活窗反超（ForceEvidence 载荷）+ **假反超已修**：缺数据 → ForceUnavailable 注记不判 Invalidated、数据补齐可恢复（#78，T14 真实夹具） | trend 域真实反超不可达（勘误在案，pan 活窗通道承担） |
| as_of 单调守卫 | ✓ | #78：倒退喂入显式拒绝 + RetrogradeRejection 审计注记、同 as_of 幂等（T15） | — |

## 时钟严格化（判据③ 横切）

| 子项 | 判定 | 证据锚 | 边界（照实） |
|---|---|---|---|
| judge_at 一钟多职拆分 | ◐ | 状态机五钟独立（book 内）；judge_at 本体一个 bit 未动 | 生产 judge_at 语义拆分未做（裁定不授权触碰） |
| 终态回填（后移覆盖）纠正 | ◐ | T12 observed_at 不后移 | 同上 |

## task-101 DRAFT 正式化（判据③ 尾）

| 子项 | 判定 | 证据锚 | 边界（照实） |
|---|---|---|---|
| DRAFT → 正式版 | ✓ | chanlun/escalate/cert-bsp-binding-ruling-20260721.md（T1-T8 代理签署生效） | G1/G2 已终裁（2026-07-21 编排者）：窗口绑定全域禁用 + T1 由「只能当门」supersede（g1-g2-window-ban-ruling-20260721.md）；p101 探针退役+grep 门实装落盘（#79 关票，ci.yml step）；签字位已补终裁指针行。E1 66 口径复跑未做 |

## #43 重审材料落盘（判据④ 尾）

| 子项 | 判定 | 证据锚 |
|---|---|---|
| 重审材料落 escalate | ✓ | v3-lifecycle-r43-review-request-20260720.md |
| SUPERSEDE 裁定书 | ✓ | r43-lifecycle-ruling-20260721.md（编排者 2026-07-21 拍板，issue #64 关票） |

## V4 三窗三臂验收（判据④ 主体）

| 子项 | 判定 | 证据锚 | 边界 |
|---|---|---|---|
| 门关/v0门/v1真链门三臂对照 | ✓（验收完成，判定照实） | v4-three-window-typed-chain-acceptance-20260721.md（§2 全表/§3 判定/§5 归因/§6 可复算索引）+ runbook 对照表回填；#77 已关票 | **wf7 硬线 ✗ 未过（照实否定）**：Long +5597.59→-4521.68 翻负（7.0× 于臂B）、Short 2.50× 超阈值；**wf8 线 ✓ 过**（execR +4724613≥臂B +3465366）。归因：身份桥命中 ~4%（typed_found=71/1724）⟹ 准入坍缩 Xzd 回退；装配侧 single_level_share 0.9579/0.9605/1.0000 单级退化未消除。缺格照实：臂B CHAIN（旧跑无此行）、出场侧 EXIT（overlay 主 loop 未接 #76 统计，需 v1/dual runner 票） |

## 当前总判（2026-07-21）

N2 ✓（链深重测落盘：门后 confirmed 全真 = 链深携带结构意义，econ 链 95.36% 为另一条链照实并记）；V1 ✓（p125 级对账 ◐ 处置备忘录在 escalate 待编排者拍板）；状态机 ✓（#78 三项修复验收 + code-review 落盘，带边界通过）；N3 ✓（构建器 ✓、进场门真链 ✓、出场门真链 ✓、#94 修复 ✓，四票 GitHub 关票）；task-101 ✓；#43 重审 ✓；**V4 ✓ 验收完成（#77 关票）：wf7 硬线 ✗ 照实否定——typed 真链门 wf7 仍有害（Long 翻负 7.0× 于 v0 门、Short 2.50×），wf8 线 ✓；归因 = 身份桥命中 ~4% 坍缩 Xzd 回退 + 单级退化未消除（single_level_share ≥0.958）**。goal 判据①②③④全部闭合（① p125 子项处置备忘录待拍板，不影响④）。#77 暴露的后续方向（桥命中率/退化治理/出场侧 v1-dual 接线/臂C 14min 成本）属新方向裁定（grilling 型），待编排者。
