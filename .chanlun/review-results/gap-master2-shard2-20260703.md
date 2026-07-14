# gap-master-2 分片2：递归/级别族 PDF 对照（7份）

> 工位 ws-gap2s2 | task #152 | 2026-07-03 | 认识论：L0/L1（凭据核对 + 当前代码锚点验证，无新实证断言）
> 方法：以 7-02 全读报告（dlpdf-c / dlpdf-e / dlpdf-d）+ nesting-fugue-conformance-20260703 为凭据基础；涉及中枢/级别语义的锚点已于本日在当前 HEAD 复核存在性（recursive_tower.rs / nest.rs / pi_bsp_timing.rs 逐一 grep 确认，非凭记忆）。

## 逐 PDF 对照表

| PDF | 核心可实装主张 | 分类 | 锚点 | 严重性 |
|---|---|---|---|---|
| 递归证明.pdf | Lift 谓词四条件 `Lift^δ_{j,ℓ}(c,g)=Host∧Nest∧Context^δ_j∧Fresh`（上级买卖点须由次级别终端证书递归确认，必要非充分；纠正"次级别一买⇒上级三类"的方向性错误） | **已装** | 全读：dlpdf-c-level-recursion-20260702.md §4；实装：`rust/src/bin/pi_bsp_timing.rs:70`（Lift.Context^δ_j，codex 裁决口径 A 2026-06-30）、`pi_bsp_timing.rs:213`（Lift.Nest，codex YES#1）——本日复核仍在；谱系 647/648 | — |
| 级别容器.pdf | 持仓语义对象错误诊断：声部=（carrier, entry_certificate, σ）三元组，激活对象必须是容器/position instance，不是买卖点叶子 g；级别轴与声部轴两条不同轴 | **部分：A 必装缺口在案** | 全读：dlpdf-c §1 + "声部 vs 级别"两轴定义节；诊断已吸收（AncOK 过滤器 `pi_bsp_timing.rs:147`）；缺口：nesting-fugue-conformance-20260703.md **P0-2**（持仓节点=容器非买卖点叶子）在账本头寸层仍 MISSING | 高（P0） |
| 级别容器2.pdf | AncOK 是过滤约束不是生成器；父声部由归属该 carrier 的 accepted opening certificate 开启；定理 `active_depth≥3>0 ⟹ depth1∧depth2>0`（祖先闭合必然）+ 违反时六种口径病因枚举 | **已装** | 全读：dlpdf-c §2；实装：`pi_bsp_timing.rs:16,147,150`（§4 AncOK 迭代到不动点，先平后开后 AncOK）——本日复核仍在；无反向"子开自动开父"生成逻辑（PDF 禁止路线未犯） | — |
| 最高级别走势类型.pdf | 方向 δ(g) 是低级别元素内在属性，持久性公理（复合不改写）；角色 R(g;c)∈{A,F,S} 是相对当前操作容器的关系属性，三态严格互斥；"在线不可回溯"只属订单层不属方向层 | **已装** | 全读：dlpdf-c §7；实装：`rust/src/theta_v0/classifier/recursive_tower.rs` `compose/compose_level/compose_level_resume`（本日复核存在）从不改写 subs.direction，测试 `compose_descend_roundtrip_preserves_subs`；三态=`Vertical::{Ambient,FollowParent,ShortDiff}` 枚举（dlpdf-c 核对：short_swing 仅衍生显示标签，无语义丢失） | — |
| 级别和sigma.pdf | σ_higher×级别交互项判据 q_i=δσ_higher、ℓ*≈3 临界级别、χ(ℓ,q)=1[LCB_OOS>θ] 非全局门控 | **已装（结构维）+ B 登记留白（统计判据）** | 全读：dlpdf-c §5；谱系 666/667（settled，逐字段吸收）；结构实装：task #132 G2-impl σ_higher 并入 canonical z（MuClass 第9维）已 completed；留白：χ(ℓ,q) 统计门终局四口径 INCONCLUSIVE（i_class×δ 共线⟒置换自毁，μ̂门定位=防灾难非 alpha），不再作为必装项 | 低 |
| 多级别检验.pdf | 严格定理 `Π_0⊆Π_{≤L} ⟹ V_{≤L}≥V_0`：L0-only 无 alpha 不能推出多级别无 alpha；正确实验设计=三实验（上级方向分桶/多级别买卖点本身/区间套确认） | **已装（三实验均有承接）** | 全读：dlpdf-e-gap-verification-20260702.md §7；实验1=σ_higher 桶键（wverify 桶键出处 + #132）；实验2=#123 hl13-impl 高级别一/三类候选生成 completed + #97 高级别重测维持；实验3=区间套确认（见下行区间套.pdf 现状） | — |
| 区间套.pdf | rung=区间包含（非端点相等，7-02 晚 ChatGPT 双裁决定义层）；下沉入场 A1–A10 + 多重赋格 B1–B16 共 26 条规格；FullNest 语义验收不能降级 | **部分：A 必装缺口 + C 在办混合** | 全读：dlpdf-d-structure-20260702.md；规格定案：nesting-fugue-conformance-20260703.md（26 条：已装9/部分10/缺口7，P0×3）；已闭环：rung=区间包含合规（#77 否证归因 + #84-#93 bit-exact+O(n) 修复 + #97 高级别重测维持，INDEX.md 回测的问题.pdf 条目）；`NestCertificate`/`n_delta` 在生产回测路径（`rust/src/theta_v0/classifier/nest.rs:163` 本日复核仍在）；A 缺口：**P0-1 下沉触发状态机**（实测 95% 退化 depth=1）、**P0-3 多空对冲 overlay 账本**；在办/已推进须复核：#114 f3-fugue 多重赋格回测 completed、#139/#140 GAP3 Realize 已实现利润入账 completed——P0-2/P0-3 是否被其覆盖须以 #114/#140 产出复核，本分片存疑保留 | 高（P0） |

## 存疑项（宁多勿漏）

1. **P0-2/P0-3 与 #114/#140 的覆盖关系未判定**：nesting-fugue 报告（20260703）与 f3-fugue 回测（#114 completed）、GAP3 Realize 实装（#140 completed）同期推进，账本头寸层"持仓节点=容器"与 overlay 对冲账本是否已被后两者部分吸收，本分片只登记不裁定——留给 #156 汇编或 upgrade/账本工位复核。
2. **中枢链重构大版本影响**：#142 ext1-impl（中枢延伸）、#148 upgrade-impl（延伸≥9段升高级别中枢，in_progress）改动中枢/级别语义；本表"已装"结论的代码锚点已在当前 HEAD 复核存在，但 #148 落地后区间套/级别容器相关行为需回归（尤其 depth 分布 95% 退化数字可能变化）。

## 汇总口径（供 #156 合并）

- 已装（含结构维已装）：递归证明 / 级别容器2 / 最高级别走势类型 / 级别和sigma / 多级别检验 —— 5 份
- A 必装缺口（已在 nesting-fugue P0 清单登记，非本分片新发现）：级别容器（P0-2）、区间套（P0-1/P0-3）—— 2 份
- B 登记留白：级别和sigma 的 χ(ℓ,q) 统计门（终局 INCONCLUSIVE，降级为留白）
- C 在办：区间套 P0 族与 #114/#140/#148 的交叠部分
