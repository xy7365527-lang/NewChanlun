# Pending Backlog 四分法分类（018号）——2026-06-29 genealogist 系统性消费

Lead Stop-Guard 触发：23 条 pending 生成态积压，按四分法逐条分类。
genealogist 工具有效域 Read/Grep/Glob/Write（无 Bash，624 硬墙），不改别工位文件、不行使 /ritual（019c 编排者权）。

## 回溯扫描结论（历史 pending 是否已被后续 settled 隐式覆盖）

**无可批量 settle 的隐式覆盖。** 坐实：
- settled/ 目录无 642-647（本轮新条目未被结算，不重复处置）。
- block-topology `meta.json`：`last_mapped_genealogy: 530`，id_mapping 至 565——626/566a 诊断的映射停滞缺口**仍开放**，未被后续结算覆盖。
- 615/634-638/576/577/622 各为独立概念分离/语法记录/工程缺口候选，无后续 settled 触及。

## 分类总表

| 号 | type | 四分法分类 | 处置 |
|----|------|-----------|------|
| 642 | bias-correction（误判降级） | **定理类** | 误升级降回定理=no-unnecessary-escalation 逆向必然；codex+codex-challenger 双向收敛坐实；settled 落盘走 /ritual（携带 memory 修正广播=编排者权 019c） |
| 643 | bias-correction（L1 假 PASS） | **定理类** | 231/625 逻辑必然；codex 真实守卫入口反例坐实；cascade reset 已定修复路径。/ritual 落盘 |
| 644 | bias-correction（14166 伪证+命题M否） | **定理类（含1行动类子项）** | 14166 数学不可能（ChatGPT§7+codex 双向证伪）+命题M 被§12/§14 否，三方坐实。**父 carrier 注入缺口=行动类**（coverage.rs，待 Lead 派工位） |
| 641 | bias-correction（增量塔 exp≈1 膨胀） | **定理类** | 090 声明膨胀逻辑必然；exp 1.65 实测坐实 centers.clone 残留。/ritual 落盘 |
| 626 | 矛盾发现（缺增量补齐脚本） | **行动类** | 工程缺口（增量补齐脚本）。566a 是其配套规格。Lead hold（topology-manager 标 LFS 墙 549，判定冲突待编排者） |
| 566a | 行动类（dag.yaml 修复规格） | **行动类** | 567-575 节点修正规格全备。Lead 派 Edit 工位执行中 |
| 622 | 矛盾发现/waiting（LFS smudge 失败） | **行动类（waiting 阻塞）** | 同根因 549/476（125MB LFS object 本地不可达）。阻塞=外部数据缺口 |
| 645 | source-tracing（π^cov vs π^bsp 分离） | **选择类** | 语义分离已澄清；「goal 是否转向 π^bsp」=方向价值判断。上呈 |
| 646 | domain（§9 vs rust depth_weight 冲突） | **选择类** | 哪边对=价值判断。上呈 |
| 647-object-identity | meta-rule（否证设计前置同一性） | **语法记录** | 上呈 |
| 647-pibsp-subvoice | bias-correction（同质代理降级） | **语法记录（待异质补验）** | 子声部=0 实装根因坐实（L0），但异质源 GPT-5.5 429 降级，待恢复补真异质验证。上呈+阻塞 |
| 647-lossy | meta-rule | **已撤回** | 与 object-identity 重复（其子集）；净增量已建议并入 |
| 615 | 概念分离（μ_f L1⊊缠论 L2） | **选择类** | 最严标准+codex 6 次代理裁决；待统一编号。上呈 |
| 577 | expansion（污染边界=内容溯源） | **语法记录** | 上呈 |
| 576-ledger（R vs TW 三阶段语义） | 概念分离（选择类） | **选择类** | 上呈 |
| 576/turning-node（转折节点拓扑⊗力度正交积） | 概念分离/aufhebung | **选择类（重大 #39 级）** | 跨 worktree 编号待 /ritual。上呈 |
| 634 | 矛盾发现/separation（督导常驻⊥四象限③） | **选择类** | 存在论冲突边裁定待编排者。上呈 |
| 635 | meta-rule（守卫自膨胀新维度） | **语法记录** | 上呈 |
| 636 | meta-rule（idle 语义⊥Lead spawn 竞态） | **语法记录** | 上呈 |
| 637 | 概念分离（中枢核心区间口径） | **选择类** | reference 裁决触发口径分离。上呈 |
| 638 | 语法记录（BSP 附着判准=本级右端点命中） | **语法记录** | 缠师博文+codex+spec/Lean 三权威链。上呈 |

## 汇总计数

- **定理类（分类已定，/ritual 仪式落盘——019c 编排者权，Lead 已裁决不授权 genealogist 自落盘）**：4 = 642/643/644/641
- **行动类**：3 = 626（Lead hold）/ 566a（执行中）/ 622（waiting）；644 含 1 行动子项（Lead hold 待 goal 裁决）
- **选择类（上呈）**：7 = 645/646/615/576-ledger/576-turning-node/634/637
- **语法记录类（上呈）**：6 = 647-object-identity/647-pibsp-subvoice/577/635/636/638
- **已撤回**：1 = 647-lossy

## 阻塞项明细

| 号 | 阻塞原因 |
|----|---------|
| 642/643/644/641 | 定理类分类已定；settled 落盘=/ritual 编排者权（携带概念/memory 变更广播），Lead 已裁决不授权自落盘 |
| 622 | 外部依赖：125MB LFS object 本地不可达 |
| 647-pibsp-subvoice | 异质源 OpenAI GPT-5.5 429，待恢复补真异质验证 |
| 626 | Lead hold：与 topology-manager LFS 墙判定冲突（549 relations.jsonl 冻结），待编排者 |
| 644 子项 | Lead hold：coverage.rs 父 carrier 注入与 goal 转向纠缠，待编排者 goal 裁决 |
| 选择/语法记录 13 条 | 须编排者 /escalate→/ritual |

## 编号碰撞协调（待 /ritual）

本轮三个 647（lossy[已撤回]/object-identity/pibsp-subvoice）。撤回 lossy 后剩两个 647 碰撞
（object-identity=meta-rule / pibsp-subvoice=bias-correction，不同轴不合并）。统一编号待编排者 /ritual。
