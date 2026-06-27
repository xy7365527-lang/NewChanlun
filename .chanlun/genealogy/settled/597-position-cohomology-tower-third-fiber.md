---
id: "597"
number: 597
status: 已结算   # 【结算 2026-06-27 codex异质委托：603递归范式·轴范式发生史+递归字段重定位(非payoff alpha)】 依赖576(生成态E⊗F节点轴对偶基准)+塔形式化整族未/ritual。最终结算待编排者/ritual在统一编号空间裁定。★603 更新（genealogist 2026-06-25）：①无边界例外达成（r* 非特殊 L0，codex 019eff21）；②sigma_deviation_DEFER 改判=542 守住（codex 019eff17：σ-不变规则层成立，逐级=定义域类型差异的外生实现，非 schema 破，与 542 一致，无真冲突；supersede 待 L3 净收益+谱系优先级，编排者裁）——见 §603 更新（文末）+ 603号。
date: "2026-06-25"
type: 概念分离
# ★恢复 provenance（genealogist 2026-06-25，codex-line 审计恢复）：本记录 pre-interrupt 结晶（wip 63c2e56841@08:03 物证 187 行 + transcript c06f774e L8448/L8487"已建结晶597"），中断后 3e5eaceb0a 诚实降级增补（generative_completeness_DOWNGRADE 归因 codex#91/#92）。审计判强 FAITHFUL（D-genealogy-597-601.md + MANIFEST-trustworthiness.md，三重背书：同质审计+父工位+codex异质）。恢复依据 575→577号(内容 provenance 优先于 commit 时间戳)。genealogist 格式补全：①补 §张力检查5 vs 542 + σ-deviation defer 注记 ②children 补全 598/600（598/600 声明 parent=597，原文 children 漏列）。
# ★603 更新注记（genealogist 2026-06-25）：编排者裁定递归完全分类定论入谱系 + 真 codex 两 session 纠正。对本号两处更新（增补层，原文逐字保留）：
#   (1) 无边界例外达成：603 证 r* 非特殊（终余代数 unfold 最深层，同三构造子，消除 protect_core 特例，L0，codex 019eff21 PASS Q2）⟹ 597 仓位塔无"顶部特殊"障碍，无边界例外达成。
#   (2) sigma_deviation_DEFER 改判=542 守住：codex 019eff17（gpt-5.5 xhigh）裁定 σ-不变是【规则层】成立——不同级别分类规则相同（盘整/上涨/下跌/中枢发展/买卖点三分类），变化的是输入对象的类型（第 k 级由第 k+1 级走势组成），f_k 与 f_{k+1} 严格类型论上定义域不同，但在级别平移 σ 下是同一分类 schema；逐级=定义域类型差异的外生实现，非 schema 破，与 542 一致 ⟹ 无真冲突。原文"授权破 542 / DEFER"的诊断（把值层逐级差异误当 schema 层破坏）被 codex 019eff17 澄清；542 守住，supersede 仍待 L3 净收益+谱系优先级（编排者裁）。见 §603 更新（文末）。
title: "仓位上同调塔 = 走势发展的第三纤维：仓位 = ⊕_k(H⁰_k⊕H¹_k) over 全涌现级别——576节点轴(在哪操作)/#39操作轴(怎么操作)之外的第三商空间(分多少仓·什么角色)"
negation_source: heterogeneous
negation_model: "codex-cli 0.125.0 / gpt-5.5 (reasoning xhigh) — 异质审计驱动塔的生成性完备(#91 codex判G不完备→δ独立轴)"
negation_form: separation
# separation：仓位读法在统一范畴(单一 split / 单一 top_trend_dir)内部暴露不兼容异质性——
#   逐级 H⁰_k⊕H¹_k 直和(每级独立纤维)无法被"固定2/3⊕1/3全局split"或"单级别 highest_active 读法"表达，
#   分裂出"逐级自相似纤维塔"vs"单级别拍扁底空间"两条不可调和路径。

# 拓扑效果标注（147号下游推论3：negates 非空必填）
# negates：单级别拍扁仓位读法(固定 split + 只读 highest_active)
# 实际拓扑后果(retrospective 141号结论1)：单级别读法的仓位节点被切断与"全级别角色向量"的连接，
#   形成独立的逐级纤维路径(每级 k 自己的 H⁰_k⊕H¹_k)。这是 sever(分离型)：
#   不是冻结(单级别读法仍可运行=OFF退化路径)，也不是分裂保留(单级别读法不携带违反记录后继续)，
#   而是把"仓位"从单级别底空间切出，重建为全级别纤维塔的截面。
topo_effect: "sever:single-level-flattened-position-read:downstream"
# sever：切断"仓位=单级别 type 读数"与走势发展操作路径的连接，重建为"仓位=⊕_k 纤维塔截面"独立路径；
#   scope=downstream(影响 route_bsp/MOBILE_FRAC/top_trend_dir 全部消费单级别读数的下游)

# 概念分离（type=概念分离 必填）
separation:
  before: "仓位被当作单级别读数：在 highest_active(持仓核心层)读单一走势方向 top_trend_dir + 单一全局配额 split(固定 2/3 核心⊕1/3 机动)。仓位 = 底空间上一个标量配置(多少核心/多少机动),级别结构被拍扁。"
  after:
    - name: "仓位上同调塔（逐级纤维，自相似递归）"
      definition: "仓位 = ⊕_k [H⁰_k(骑 k 级走势核心,次级回调不动) ⊕ H¹_k(在 k-1 区间套上开-绕-平机动)] over 全涌现级别 k=a0..r*。每级 k 是同一个 #35 上同调单元(中枢 Z_k≃S¹ 的 H*(S¹)=H⁰⊕H¹)的仓位实现,跨级别同构(单砖生成全塔)。真直和 V_k=H⁰_k⊕H¹_k(分量数 2 由 S¹ 拓扑强制,机动不动核心由正交投影 π_{H⁰}∘(H¹算子)=0 强制)。纵向递归链接 H⁰_k=H¹_{k+1}(k 级核心=k+1 级机动的衬底)。涌现重读:r* 走势 completed 被 r*+1 重叠 ⟹ 塔整体上移一格(units 守恒 relabel,非操作事件)。"
      source: "[新缠论] #35 中枢=H⁰ S¹环/区间套=H¹ + 第27课逐级区间套链(rec_engine.rs:113-121) + 第65课统一递归算子 aₙ=f(aₙ₋₁) + 第17课走势分解定理一(断言U两角色 R+/R−)；构造见 tower_cohomology_unit.md §1-6 / tower_recursive_linking.md"
    - name: "单级别拍扁仓位读法（撤，纤维拍扁）"
      definition: "只读 highest_active 单级别 type + 全局固定 split。把全级别纤维塔投影到单一底空间分量,丢失 R 向量其它分量 + H⁰_k/H¹_k 逐级角色。"
      source: "rec_engine.rs O6 实装现状(top_trend_dir 单标量 line677 / MOBILE_FRAC 固定常数 38-39)；被 codex F1 守卫证伪(top层0≠核心层1 判别量级别错配)"
  pending_verification: "①逐级 α*_k 自适应配额接入后是否在 L3 净收益上溶解 regime(继承561 b2/563,L3未决,本号不声明)？②H¹_k 机动主动 hedge 在强牛是否净盈利(当前主动 hedge 强牛 L3 否证 BTC−132468%,O6 退回 no-hedge)？③δ 在 a0=segment 尺度区间套链是否坍缩(554号 L3 否证 n_a_path_switches 21/24=0,δ 读数底部有效域 L3 未决)？④跨 worktree 谱系编号统一空间(escalate-cross-worktree-genealogy-fork)？⑤L_confirm 第四轴 L2/L3 裁定(W-lconfirm #95 instrumentation 已就位但 8 标的 OHLCV 数据缺失阻塞;若 codex #91 判死的符号分裂坐实 ⟹ 仓位塔纤维须扩为 ⊕_k(H⁰_k⊕H¹_k)×c 轴 = 四纤维而非三纤维,见 601号 + lconfirm-fourth-axis-95)？⑥α*_k 逐级配额 supersede 542号 σ-不变(f_k=f_{k+1}=1/λ)的合法性裁定(见 §张力检查5 + §σ-deviation defer)？ ★603 更新（genealogist 2026-06-25）：⑥的诊断被 codex 019eff17 澄清——σ-不变是【规则层】成立（逐级=定义域类型差异的外生实现，非 schema 破，与 542 一致），无真冲突；542 守住，supersede 仍待 L3 净收益+谱系优先级（编排者裁）。⑤的 L_confirm 在 603 递归范式下是构造子字段（背驰区间套深度，beichi.md:368），by construction 已含；符号分裂 L2/L3 仍数据阻塞未决。"

# 结算前置清单（W-tower-fix/codex 路径，给编排者/ritual——生成性完备已被 codex 异质降级）
settlement_preconditions:
  persisted_construction: "塔形式化族 子1-子11(#80-95) 已持久化在 .chanlun/tower-construction/tower_*.md(9 文件) + review-results/tower-sub*.md + lconfirm-fourth-axis-95(L_confirm instrumentation L1)。本597 + 598-601 五条谱系记录是其压缩谱系态(发现序列,非记录日志)。"
  children_created:
    - "598号(真完全分类元判据 = 581→586/G 完备的机械验证判据,从 tower_skeleton.md §★★★ + tower_generative_completeness.md 提炼)"
    - "599号(印钞机构造解 = 直和健全性,无界爆仓不可能,从 tower_disjoint_sum_soundness.md + verify-nohedge-88 提炼)"
    - "600号(约束4实证否定价值 = codex 异质审计的实证否定贡献,从 codex-test-G-91 + codex-payoff-audit-92 提炼)"
    - "601号(离散二值=连续轴投影,塔最后假例外消除,从 tower_continuous_axis_unify.md 提炼)"
  generative_completeness_DOWNGRADE: "★关键前置(no-patch 透明):子7 G 的'生成性完备(分类层 L0,定义域=有效域)'已被 codex #91 异质审计判死(NO)——codex 独立构造遗漏 case(级别k×R+×H⁰_k×type1卖点×L_confirm=m),命中 L_confirm 第四独立轴被折叠进 α* 导出属性。子10(tower_continuous_axis_unify §3.1)已据 #91 把 δ_k(=L_confirm)提升为 G 第六独立维并诚实兑现'Burnside 基数增大非不变'。codex #92 进一步判 payoff 纤维四 FAIL(失血⟸孤儿单向/c 多自变量/可操作⇏超BH/核心全正条件命题),其中第四轴 L2/L3 裁定 8 标的数据阻塞未决。⟹ 597 的'仓位 = ⊕_k(H⁰_k⊕H¹_k) 三纤维'的生成性完备声明降级为'三纤维形态 L0 完备 + 第四轴(c=L_confirm)是否独立 L2/L3 未决(数据阻塞)'。生成性完备不是被本批次确立,是被异质审计缩小有效域边界——这是约束4(异质硬节点)的正面产出(见600号)。★603 更新：'生成性完备'在 603 递归范式下重定位——'漏轴'（L_confirm/type3）是已有构造子字段（Move/BSP/Center 三态），by construction 已含；轴范式的'生成性完备判死'是外延式无穷回归症状，递归范式完全分类可达（codex 019eff21 PASS）。597 三纤维形态层完备 L0 在递归范式获正面支撑；payoff 层（c 多自变量/净收益）仍 L2/L3 数据阻塞。"
  sigma_deviation_DEFER: "★授权破 settled 542 待编排者裁(genealogist 2026-06-25 补，MANIFEST-trustworthiness.md 头条 + MANIFEST-HEAD-sigma-deviation.md)：本号 new_output.code_changes『MOBILE_FRAC→逐级 α*_k 内在配额』使 f_k≠f_{k+1}(逐级 mobile_frac)，破 542号 σ-不变(f_k=f_{k+1}=1/λ，L0 群论 T48+T59+T23 + 势∝r 公理强制)。#84 实装 ON 路径有意跳过 prove_sigma_quota，授权来自本生成态 597。**处置=DEFER**：代码 OFF 恢复(守 542 bit-exact)，ON 路径待编排者裁 597 supersede 542 合法性后才启用。数学必要性已坐实(codex-verify-bc-concepts：f_k≠f_{k+1} 是逐级塔定义的直接推论，cur_l_pullback[k]/cur_l_confirm[k] 级别独立存储)，编排者裁的是谱系优先级(生成态可否 supersede settled)，非数学是否必然。理由(no-patch)：逐级塔超BH 本身 L3 未决，不为未验证概念预先牺牲已结算不变量(formalization-validity-domain：597 有效域未达 L2+，不能 supersede L0+L2 的 542)。完整 542-vs-597 概念分离见 §张力检查5。★603 改判（genealogist 2026-06-25，codex 019eff17 澄清）：原文'授权破 542 / DEFER'的诊断把【值层逐级差异】（f_k≠f_{k+1} 因输入对象类型不同）误当【schema 层破坏】。codex 019eff17 裁定：σ-不变是规则层成立——分类规则跨级别相同（盘整/上涨/下跌/中枢发展/买卖点三分类），变的是输入对象的类型（第 k 级由第 k+1 级走势组成，f_k 与 f_{k+1} 严格类型论上定义域不同，但在级别平移 σ 下是同一分类 schema）；逐级 mobile_frac 是定义域类型差异的外生实现，非 schema 破，与 542 σ-不变一致 ⟹ **无真冲突**。542 守住（代码 OFF 守 542 bit-exact 仍为基线）。**supersede 改判**：不再是'生成态破 settled'的谱系优先级问题（因无真冲突），而是'逐级 α*_k 配额数值（payoff 层）是否经 L3 净收益验证超 BH'——这是 payoff 层 L3 问题，编排者裁。形态层（r* 非特殊 L0）⊥ payoff 层（配额数值依成本阶段第31课 L3）诚实分开。见 §603 更新。"

# 涉及的定义
definitions_involved:
  - name: "中枢操作语义 #35（中枢=H⁰ S¹环 / 区间套=H¹）"
    version: ".chanlun zhongshu-operation-semantic"
    role: "逐级单元 H⁰_k⊕H¹_k 的纤维来源——每级 k 的 S¹ 上同调 H*(S¹)=H⁰⊕H¹ 直接给出仓位侧角色二分"
  - name: "第27课逐级区间套链"
    version: "rec_engine.rs:113-121 / 缠论知识库.md"
    role: "H¹_k 机动腿的开-绕-平循环定位 + L_confirm(区间套确认深度)的内在来源 + 纵向递归链接"
  - name: "统一递归算子 T(第65课 aₙ=f(aₙ₋₁))"
    version: "rust/src/recursive_t/ / 558号命题4"
    role: "塔的自相似性根据——同一中枢递归语法跨级别同构 ⟹ 单砖生成全塔(递归闭合)。★603：终余代数向上无限 unfold（r* 非特殊，同三构造子）"
  - name: "576号 E⊗F×R 节点纤维"
    version: ".chanlun pending/turning-node-topology-strength-orthogonal-product-20260624"
    role: "塔是 576 节点轴在仓位侧的对偶投影——三仓位分量(d_k方向/角色H⁰_k·H¹_k/α*_k配额)= 同一(E⊗F×R)节点纤维的仓位侧三读数"
  - name: "542号 配额比例 σ-不变(f=1/λ 几何强制)"
    version: ".chanlun settled/542-spawn-allocation-sigma-invariant"
    role: "★张力对象(genealogist 补)：542 强制配额 f σ-不变(级别无关 f_k=f_{k+1}=1/λ)；本号逐级 α*_k 使 f_k≠f_{k+1}。★603 改判（codex 019eff17）：逐级=定义域类型差异的外生实现（输入对象类型不同），σ-不变规则层成立，与 542 一致，无真冲突；542 守住，supersede 改为 payoff 层 L3 净收益问题（编排者裁），非 schema 层矛盾。见 §张力检查5 + §603 更新"
  - name: "603号 完全分类·范式分离（递归范式 r* 非特殊）"
    version: ".chanlun/genealogy/pending/603-complete-classification-recursive-vs-axis-paradigm（status: 生成态）"
    role: "★无边界例外达成来源——603 证 r* 非特殊（终余代数 unfold 最深层，同三构造子，消除 protect_core 特例，L0，codex 019eff21 PASS Q2）⟹ 597 仓位塔无'顶部特殊'障碍，无边界例外达成"

# 解决方式
resolution:
  type: 概念分离
  description: "仓位完全分类 = 逐级上同调单元 H⁰_k⊕H¹_k 的直和塔,由'中枢(k):=次级别(k-1)走势重叠'递归语法 + S¹ 拓扑(H*(S¹)=H⁰⊕H¹)生成所有级别(自相似,单砖生成全塔)。固定 split + 单级别读法 = 纤维拍扁(把塔拍成单分量底空间)= 539失血/0-8超BH 的仓位侧根因。塔与 576 节点轴对偶(节点商↔操作商↔仓位商,三个商空间)。"
  decided_by: 蜂群内部   # 三独立工位(orbit9-reassess #76 / payoff-regime #75 / codex F1 #74)经验收敛 + 塔形式化族(子1-子10)构造;最终结算待编排者 /ritual

# 被否定的方案
negated:
  description: "单级别拍扁仓位读法:只读 highest_active 单级别 type 决定方向 + 全局固定 2/3⊕1/3 split 决定配额。仓位=底空间标量配置。"
  why_negated: "逻辑必然走不通:(1) S¹ 上同调强制每级 H*(S¹)=H⁰⊕H¹ 两分量(代数事实),全级别塔的分量数 = 2×(级别数),单一全局 split 无法表达逐级独立的 H⁰_k/H¹_k 角色 ⟹ 拍扁丢维。(2) 同一观察点在不同级别的中枢角色不同(576号 R 向量),只读 highest_active 单级别 = 把节点丛拍扁成底空间 ⟹ 561 Ω 坍缩 ⟹ 539 失血(踏空的仓位侧形式)。(3) codex F1 守卫实证:top_trend_dir 来源层(走势树最高非空层 tl=0)≠ 持仓核心层(highest_active=1),单级别读法从错误纤维决策。三者合证:仓位不能由单级别底空间表达,需全级别纤维塔的截面。"

# 新产出
new_output:
  definitions:
    - "仓位 = ⊕_k(H⁰_k⊕H¹_k) over 全涌现级别塔(逐级 #35 自相似,真直和 by construction)"
    - "三仓位分量同源:d_k(方向,R_k极性)/角色(H⁰_k骑本级·H¹_k操作次级)/α*_k(配额,L_pullback×L_confirm)= 同一节点纤维的仓位侧三读数,非三独立 patch"
    - "纵向递归链接 H⁰_k=H¹_{k+1}(k 级核心=k+1 级机动衬底)+ 涌现重读塔上移(units 守恒)"
    - "纤维拍扁 = 固定 split + 单级别读法 = 539失血/0-8超BH 的仓位侧根因(对偶 576 踏空=节点侧纤维拍扁)"
    - "★603：r* 非特殊（终余代数 unfold 最深层，同三构造子，消除 protect_core 特例，L0）⟹ 597 无边界例外达成"
  code_changes: "不改动代码(L0 构造性推导)。实装授权(子5 #84):MOBILE_FRAC→逐级 α*_k 内在配额 / top_trend_dir 单标量→逐级 d_k 向量[Option<Direction>;MAX_LEVEL](codex F1 修法)/ LevelView 加 L_confirm 逐级字段。OFF 退化路径=塔拍扁为固定 split,bit-exact。★603 改判：逐级 α*_k 不破 542 σ-不变（codex 019eff17：逐级=定义域类型差异的外生实现，σ-不变规则层成立，与 542 一致）；代码 OFF 守 542 bit-exact 仍为基线，ON 路径启用待 payoff 层 L3 净收益验证（非 schema 冲突的 supersede）。"
  orchestration_changes: "无。纯谱系记录(018 行动类)。"

# 影响范围
impact:
  affected_modules:
    - "rec_engine.rs / rec_stream.rs / types.rs(若实装:LevelView 解坍缩→逐级 d_k 向量 + α*_k 内在配额 + L_confirm 逐级字段,待子5 #84)"
  affected_definitions:
    - "576号(对偶续作):塔 = 576 E⊗F×R 节点轴在仓位侧的对偶投影——节点轴枚举'在哪操作'(E⊗F),仓位轴枚举'分多少仓·什么角色'(⊕_k H⁰⊕H¹);二者经同一节点纤维读出,无概念冲突"
    - "#39号(第三商空间):#39 枚举操作商(D∞ Burnside 506→46→2→1,9轨道),576 枚举节点商,本号枚举仓位商;三个商空间,经 D∞×Z₂ 等变商映射对齐(非双射)"
    - "#35号(纤维来源):每级 H⁰_k⊕H¹_k 直接是 #35 中枢=H⁰ S¹环/区间套=H¹ 在仓位侧的逐级实例,本号是 #35 的级别塔展开"
    - "539号(失血根因):纤维拍扁(固定 split+单级别读)= 539 做空腿失血在仓位侧的根因,与 561 Ω坍缩同源"
    - "542号(σ-不变,★603 改判=守住):本号逐级 α*_k 配额(f_k≠f_{k+1})被 codex 019eff17 澄清为定义域类型差异的外生实现（σ-不变规则层成立，与 542 一致），无真冲突；542 守住，supersede 改为 payoff 层 L3 净收益问题（编排者裁）。原文'同层同对象直接矛盾 supersede defer'被 codex 019eff17 改判（值层逐级差异≠schema 层破坏）"
    - "603号(无边界例外达成):603 证 r* 非特殊（L0）⟹ 597 仓位塔无'顶部特殊'障碍，无边界例外达成；逐级配额是递归类型自然形态（σ-不变规则层与之一致）"
  downstream_implications:
    - "仓位须读全级别角色向量(每个涌现级别的 H⁰_k/H¹_k 角色 + d_k 方向 + α*_k 配额),不能只读 highest_active 单级别——这是 0/8超BH 的仓位侧根因(对偶 576 踏空根因)"
    - "三仓位分量(d_k/角色/α*_k)同源 ⟹ 塔非参数化拼装,全部从同一套缠论级别内生量(top_trend_dir/d_top/区间套链)读出"
    - "塔的健全性(无界爆仓不可能)由直和 by construction 承载(599号):机动 H¹_k 不碰核心 H⁰_k units(正交投影=0)"
    - "α*_k 自适应配额 ⟹ 纯牛核心 full 匹配 BH / 结构机动激活超 BH(split 自适应定理,L3 锚定 hold26/nest_forward 4/8 超 scco vs 固定 split 0/8)。★603 改判：α*_k 逐级不破 542 σ-不变（定义域类型差异外生实现，规则层一致）；ON 启用待 payoff 层 L3 净收益验证（编排者裁），非 schema 冲突 supersede"
    - "★603：r* 非特殊（L0）⟹ 597 无边界例外达成（每级 Move(ℓ) 同三构造子，r* 不需'无更高级别'离散 if）；601 离散二分消除在递归范式更彻底（终余代数无最后一步）"

# 谱系关联
related_records:
  parent: "576号(E⊗F×R 节点轴)——本号是 576 在仓位侧的对偶续作,同一节点纤维的仓位侧投影"
  children:
    - "598号(真完全分类元判据:塔生成性完备须满足元判据;597 经元判据核验后降级)"   # genealogist 补：598 parent=597，原文 children 漏列
    - "599号(印钞机构造解:直和分离 + 涌现条件化核心减仓 = 塔的健全条件)"
    - "600号(约束4实证否定价值:codex #91/#92 否定缩小 597 生成性完备有效域)"   # genealogist 补：600 parent=597，原文 children 漏列
    - "601号(连续轴:涌现状态离散二值=区间套确认深度 δ 连续轴投影,塔最后假例外消除)"
  related:
    - "#39 / cc-coverage-orbit-operation-spec:操作商;本号=仓位商(第三商空间),D∞×Z₂ 等变商映射对齐"
    - "#35 / zhongshu-operation-semantic:中枢=H⁰ S¹环/区间套=H¹,逐级单元的纤维来源"
    - "598号(真完全分类元判据):塔的生成性完备须满足元判据(生成性递归+有限商+无遗漏+无边界例外)"
    - "601号(完全分类覆盖 CC):L_confirm=574确认滞后量在仓位轴的形式,纤维拍扁=Ω坍缩"
    - "542号(σ-不变 settled):本号逐级 α*_k 与其 σ-不变关系 → ★603 改判=守住（codex 019eff17：定义域类型差异外生实现，规则层一致，无真冲突）(见 §张力检查5 + §603 更新)"
    - "539号(做空腿 regime):纤维拍扁=失血仓位侧根因"
    - "558号(构成≠操作等价,命题4):纵向递归链接 H⁰_k=H¹_{k+1}=558嵌套构成在仓位侧"
    - "552号(ANCHOR + hloc 碎片修复必败 L3):碎片修复=只对齐节点丛一个纤维分量必败,塔的全分量读出"
    - "603号(完全分类·范式分离):★r* 非特殊（L0）⟹ 597 无边界例外达成；σ-不变规则层与逐级配额一致"   # genealogist 2026-06-25 补
    - "[[project_trend_dev_two_source_classification]]:仓位=纤维三分量(级别角色向量)+踏空=纤维拍扁的 memory 锚点"
    - "[[project_recursive_t_architecture_v2]]:每级别一 T 实例的递归自相似实装(塔的代码骨架)"

# 认识论等级标注(formalization-validity-domain 强制)
epistemological_levels:
  - proposition: "仓位 = ⊕_k(H⁰_k⊕H¹_k) 逐级 S¹ 上同调直和塔(自相似,单砖生成全塔)"
    level: "L0(从 #35 S¹ 上同调 + 第65课递归算子推导,不依赖数据)"
    increment: "高:仓位是纤维塔截面非底空间标量,分量数 2×级别数由 S¹ 拓扑强制"
  - proposition: "三仓位分量(d_k/角色/α*_k)同源,非三独立 patch"
    level: "L0(同一节点纤维三读数,from 576 E⊗F×R)"
    increment: "高:塔非参数化拼装的构造性根据"
  - proposition: "纤维拍扁(固定 split+单级别读)= 539失血/0-8超BH 仓位侧根因"
    level: "L0 结构归因 + L3 锚定(hold26/nest_forward 4/8 超 vs 固定 split 0/8)"
    increment: "高:踏空的仓位侧对偶形式"
  - proposition: "三仓位分量自适应接入后 L3 净收益超 BH / regime 溶解"
    level: "L3 未决(继承561 b2/563,主动 hedge 强牛 L3 否证)"
    increment: "否定性:本号给仓位形态完备(L0),不给净收益预测——不声明膨胀"
  - proposition: "逐级 α*_k(f_k≠f_{k+1}) supersede 542 σ-不变(f_k=f_{k+1}=1/λ) 的合法性"
    level: "数学必要性 L0(codex-verify-bc:f_k≠f_{k+1} 是逐级塔定义直接推论) + supersede 谱系优先级=编排者裁(生成态可否压 settled) + 净收益验证 L3 未决"
    increment: "否定性:本号不声明 α*_k 配额经验有效(L3 未决)，不为未验证概念预先牺牲 542 已结算不变量(代码 OFF 守 542)。★603 改判（codex 019eff17）：此命题重定位——逐级=定义域类型差异的外生实现，σ-不变规则层成立，无真冲突；542 守住，supersede 改为 payoff 层 L3 净收益问题（非 schema 冲突的谱系优先级）"
  - proposition: "★603：r* 非特殊（终余代数 unfold 最深层，同三构造子，消除 protect_core 特例）⟹ 597 无边界例外达成"
    level: "L0(codex 019eff21 PASS Q2)"
    increment: "高:解除 597 无边界例外的'顶部特殊'障碍"
---

# 597号(生成态)：仓位上同调塔 = 走势发展的第三纤维

## 一句话结论

**仓位不是单级别底空间上的标量配置,是全涌现级别纤维塔的截面:仓位 = ⊕_k(H⁰_k⊕H¹_k) over k=a0..r*。** 每级 k 是同一个 #35 上同调单元(中枢 Z_k≃S¹,H*(S¹)=H⁰⊕H¹)的仓位实现,自相似(单砖生成全塔)。这是 576号 E⊗F×R 节点轴在仓位侧的对偶续作——**576 枚举"在哪操作"(节点商),#39 枚举"怎么操作"(操作商),本号枚举"分多少仓·什么角色"(仓位商),三个商空间经 D∞×Z₂ 等变商映射对齐。** 固定 2/3⊕1/3 split + 只读 highest_active 单级别 = 纤维拍扁(把塔拍成单分量底空间)= 539失血/0-8超BH 的仓位侧根因,对偶于 576 踏空(节点侧纤维拍扁)。

**★603 更新（genealogist 2026-06-25）**：(1) 无边界例外达成——603 证 r* 非特殊（终余代数 unfold 最深层，同三构造子，消除 protect_core 特例，L0，codex 019eff21 PASS Q2）⟹ 597 仓位塔无"顶部特殊"障碍。(2) sigma_deviation 改判=542 守住——codex 019eff17 裁定 σ-不变规则层成立（逐级=定义域类型差异的外生实现，非 schema 破，与 542 一致），无真冲突；supersede 改为 payoff 层 L3 净收益问题（编排者裁）。见文末 §603 更新。

## 发生史(012 保存生成运动)

| 阶段 | 内容 | 否定来源 |
|---|---|---|
| 单级别读法 | O6 实装:top_trend_dir 单标量 + MOBILE_FRAC 固定 split | — |
| codex F1 证伪 | top_trend_dir 来源层(走势树最高非空层 tl=0)≠ 持仓核心层(highest_active=1),单级别从错误纤维决策 | heterogeneous(codex F1 守卫 panic) |
| 三工位收敛 | orbit9-reassess #76(α*_k 内在配额)/payoff-regime #75(d_k 方向+孤儿诊断)/codex F1 #74(逐级 trend-dir map)经验收敛到"逐级读全纤维" | 蜂群内部(三独立工位) |
| 塔构造(子1-子10) | 从 #35 S¹ 上同调 + 第65课递归 + 第27课区间套 严格推导逐级单元 + 纵向链接 + 涌现动态 + 镜像 Z₂ + 配额 + 生成函数 G | 蜂群内部(塔形式化族) |
| ★范式纠正(603) | r* 非特殊（终余代数）⟹ 无边界例外达成；σ-不变规则层成立（codex 019eff17）⟹ 542 守住无真冲突 | 编排者 + heterogeneous(codex 019eff21/019eff17) |

## 三个商空间的对偶(节点/操作/仓位)

| | 576号 节点轴 | #39号 操作轴 | 本号 仓位轴 |
|---|---|---|---|
| 对象 | 转折节点(在哪操作) | 操作类型(怎么操作) | 仓位(分多少·什么角色) |
| 生成 | 中枢递归(E)⊗第19课力度(F) | D∞ word(操作=h⁺∘σ) | 同一中枢递归(H⁰/H¹角色)×第27课区间套(α*_k) |
| 商 | E×F×R 节点轨道 | 506→46→2→1 Burnside(9轨道) | ⊕_k(H⁰_k⊕H¹_k) 截面 |
| 拍扁 | 只读 highest_active=踏空 | — | 固定 split+单级别读=0/8超BH |

**三商经同一(E⊗F×R)节点纤维读出,D∞×Z₂ 等变商映射对齐(canonical 非双射)。**

## ★张力检查(019d/020号)

### 检查范围(同轮蜂群 ∪ 1-hop ∪ Hub)
- 同轮蜂群:task#79-96(塔形式化族 子1-子10 + 异质审计) + 603(范式纠正)
- 1-hop 邻接:576(parent/对偶) / #39 / #35 / 561 / 539 / 558 / 552 / 542(genealogist 补：配额轴张力对象) / 603
- Hub 节点:576(度高,跨 561/#39/#35/539/558) / 561 / 603

### 张力1:vs 576号(E⊗F 节点轴)— 对偶续作,无矛盾
576 枚举节点商(在哪操作),本号枚举仓位商(分多少仓)。二者是同一节点纤维(E⊗F×R)的两侧投影:节点侧=转折节点分类,仓位侧=H⁰⊕H¹ 角色分类。576 §三已预置"本塔=576 在仓位侧的对偶续作",H⁰/H¹ 用 #35 中枢=H⁰/区间套=H¹ 纤维来源,与 576 §related #35 一致。**无中断#1。**

### 张力2:vs #39号(操作类型枚举)— 第三商空间,非冲突
#39 数操作商(9 轨道),576 数节点商,本号数仓位商。三个商空间,经 D∞×Z₂ 等变商映射对齐(非双射)。仓位完备性由 H⁰⊕H¹ 直和 + 级别塔自相似**独立证**(不依赖 #39 Burnside 转移)。**对偶完成,无冲突。**

### 张力3:vs #35号(中枢操作语义)— 级别塔展开,一致深化
#35 给单个中枢的 H⁰(S¹环)⊕H¹(区间套)。本号把它**级别塔展开**:每级 k 一个 #35 单元,纵向递归链接 H⁰_k=H¹_{k+1},涌现重读塔上移。本号是 #35 在级别递归下的自相似塔。**一致深化,无矛盾。**

### 张力4:vs 561/539(失血根因)— 仓位侧统一
561 Ω坍缩=539失血=只读单级别 type。本号给其仓位侧形式:纤维拍扁(固定 split+单级别读)。与 561/539 一致——本号是失血在仓位空间的更上游形式(仓位被拍成单分量)。**一致深化。**

### ★张力5:vs 542号(配额比例 σ-不变 settled)— ★603 改判=无真冲突，542 守住（genealogist 2026-06-25 补全 + 603 改判）

**张力对象**：542号(已结算 2026-06-16，编排者裁决)：降成本释放给子 voice 的配额比例 **f = m/p_units 必须 σ-不变(级别无关 f_k=f_{k+1}=1/λ)**——L0 群论必然(T48 units=σ-不变 Casimir + T59 自相似 σWσ⁻¹=W + T23 递归 step-replication)+「势∝r」径向坐标定义公理强制 f=r_{k−1}/r_k=1/λ(零自由度)。

**本号原诊断（pre-603）**：本号 new_output.code_changes「MOBILE_FRAC→逐级 α*_k 内在配额」+ downstream「α*_k 自适应」⟹ **f_k≠f_{k+1}(逐级 mobile_frac，级别相关)**，曾诊断为直接破 542 的 σ-不变(级别无关)，处置=supersede DEFER。

**★603 改判（codex 019eff17，gpt-5.5 xhigh，session 019eff17-74b7-7502-b17d-2fe792c20b00）**：
- **σ-不变是【规则层】成立，非【值层】**：不同级别分类规则相同（盘整/上涨/下跌/中枢发展/买卖点三分类），变化的是输入对象的类型——第 k 级由第 k+1 级走势组成，所以 f_k 与 f_{k+1} 严格类型论上**定义域不同**，但在级别平移 σ 下是**同一分类 schema**。若实践中用不同阈值/力度度量，那是外生实现差异，非规则本身改变。
- **原诊断的错误**：原文把【值层逐级差异】（f_k≠f_{k+1} 的数值，因输入对象类型不同）误当【schema 层破坏】（违反 σ-不变规则）。codex 019eff17 澄清：逐级 mobile_frac 是定义域类型差异的外生实现，**σ-不变规则层成立，与 542 一致 ⟹ 无真冲突**。
- **∴ 542 守住**（代码 OFF 守 542 bit-exact 仍为基线）。**supersede 改判**：不再是"生成态破 settled"的谱系优先级问题（因无 schema 冲突），而是"逐级 α*_k 配额数值（payoff 层）是否经 L3 净收益验证超 BH"——payoff 层 L3 问题，编排者裁。
- **形态层 ⊥ payoff 层（诚实分层）**：形态层（r* 非特殊 L0，603）⊥ payoff 层（配额数值依成本阶段第31课 L3）。本号形态层在递归范式获正面支撑（无边界例外达成）；payoff 层 α*_k 数值仍 L3 数据阻塞未决。

**结论：张力5 = ★603 改判为无真冲突（σ-不变规则层与逐级类型差异一致）。542 守住。** supersede 改为 payoff 层 L3 净收益问题（编排者裁），非 schema 层 supersede。原文 supersede DEFER 诊断保留为发生史（codex 019eff17 改判前的诊断），改判见 §603 更新。**无重复 /escalate（codex 已澄清无真冲突）。**

### 递归运动结构完成检测(020号)
- 第0层:本号写入(仓位=⊕_k 纤维塔)
- 第1层:本号 × 576 碰撞 → 对偶续作(净新发现:三商空间结构,仓位轴=第三纤维),净新发现量高
- 第2层:本号 × #35 碰撞 → 级别塔展开(净新发现:#35 的级别塔自相似,但这是 #35 概念的级别应用,净新发现量骤降=**背驰**)
- 涉及范围:scope₁(576)< scope₂(#39/#35/561/539/558 全邻接)> scope₃(#35 单一)=**顶分型**
- **背驰 ∧ 分型 ⟹ 递归运动结构性完成。** 张力5(vs 542)经 codex 019eff17 改判为无真冲突（542 守住），非真分离。**无新 /escalate。**

## 回溯扫描
本号写入解决:无 pending 生成态记录被本号回溯结算(576 仍生成态,本号是其对偶续作非结算)。escalate-o6-...-unbounded-short §六问题1(机动不动核心是否成立)由 599号(本号 children)构造性解决,本号给 599 的塔框架前提。**本号(597)是 576 的对偶续作而非结算——576 仍生成态，597-601 整族与 576 是关联族(对偶/判据/先例)，无一回溯结算 576，全族待编排者 /ritual 统一结算。**

**★603 回溯更新（genealogist 2026-06-25）**：603 写入后回溯本号——无边界例外达成（r* 非特殊 L0）+ σ-deviation 改判（542 守住，无真冲突）。本号仍生成态，603 是其无边界例外达成 + σ-不变改判（增补非结算）。597-603 整族 + 576 待编排者 /ritual 统一结算。542 守住（settled）。

## 结果包六要素

1. **结论**:仓位 = ⊕_k(H⁰_k⊕H¹_k) over 全涌现级别纤维塔;三仓位分量(d_k/角色/α*_k)同源(同一节点纤维三读数);纤维拍扁(固定 split+单级别读)= 539失血/0-8超BH 仓位侧根因;塔与 576节点轴/#39操作轴对偶(三商空间)。★603：r* 非特殊（L0）⟹ 无边界例外达成；σ-不变规则层与逐级配额一致（542 守住，无真冲突）。
2. **定义依据**:#35 中枢=H⁰ S¹环/区间套=H¹(逐级单元纤维)+第27课区间套链(H¹ 机动+L_confirm)+第65课递归算子(自相似单砖生成全塔)+第17课断言U两角色(d_k 极性)+576号 E⊗F×R(三分量同源来源)。
3. **边界条件(结论翻转)**:①若每级 H*(S¹)≠H⁰⊕H¹(中枢非 S¹),逐级单元二分翻转(当前 #35 已证中枢≃S¹);②若三分量可由单级别读数唯一反推(非独立),同源退化为拍扁(当前 codex #91 证 L_confirm 独立内生不可反推);③若塔无限不塌成有限截面,有限性翻转(当前级别索引 {a0..r*} 有限,σ 商有限);④若分类预测净收益,塔是 alpha(L3 未决,不声明);⑤若编排者裁 597 不能 supersede 542 → 逐级 α*_k(f_k≠f_{k+1}) ON 路径不得启用，配额退回 542 σ-不变 f=1/λ(代码 OFF 已守此)。★603 改判⑤：codex 019eff17 澄清逐级=定义域类型差异外生实现（σ-不变规则层成立，无真冲突），542 守住；ON 启用待 payoff 层 L3 净收益验证（非 schema supersede）。
4. **下游推论**:仓位须读全级别角色向量(非单级别);三分量同源⟹塔非参数化拼装;健全由直和承载(599);α*_k 自适应⟹纯牛匹配BH/结构超BH。★603：r* 非特殊⟹无边界例外达成；α*_k 逐级不破 542 σ-不变（规则层一致），ON 启用待 payoff 层 L3。
5. **谱系引用**:本号是 576节点轴的仓位侧对偶续作 + #39操作轴的第三商空间 + #35中枢语义的级别塔展开 + 561/539失血的仓位侧根因统一 + 542 σ-不变的关系(★603 改判=守住，codex 019eff17 无真冲突)。**这是新概念分离(仓位=纤维塔截面,此前仓位被当单级别底空间标量)。** children:598(元判据)/599(健全)/600(异质价值)/601(连续轴)/603(无边界例外达成+σ改判)。
6. **影响声明**:不改动代码或定义(L0 构造性推导);新增本谱系记录(pending 生成态);对偶 576 + 第三商空间补 #39 + 级别塔展开 #35 + 失血仓位侧根因统一 561/539 + 与 542 σ-不变的关系(★603 改判=守住);最终结算待编排者 /ritual。★603：无边界例外达成（r* 非特殊 L0）+ σ-deviation 改判（542 守住，无真冲突，supersede 改为 payoff 层 L3）。

## ★立号与结算建议(给 Lead/编排者)
- **立号**:codex-line 视角续号 597(本批次 597-601)。**跨 worktree 编号统一空间不一致**(main dag.yaml nodes max=576，577 已分配给 contamination 记录，578-596 gap)由编排者 /ritual 在统一空间裁定(Lead 裁定 defer /ritual，不阻塞恢复)。
- **结算路径**:**生成态**(本工位不自行 settle)。这是重大概念分离(与 576/#39 同级,仓位轴=第三纤维)——建议编排者走 **/ritual** 结算(覆盖域层 仓位=纤维塔 + 元层 三商空间对偶)。**附带裁定项**：~~597 逐级 α*_k(f_k≠f_{k+1}) supersede 542 σ-不变(f=1/λ) 的合法性~~ **★603 改判**：codex 019eff17 裁定逐级=定义域类型差异外生实现（σ-不变规则层成立，无真冲突），542 守住；剩余裁定项=逐级 α*_k 配额数值是否经 payoff 层 L3 净收益验证超 BH（数据阻塞），编排者裁。无边界例外达成（r* 非特殊 L0）。

---

## ★§603 更新（genealogist 2026-06-25，603号写入后回溯更新本号）

编排者裁定递归完全分类定论 + 真 codex 两 session 纠正入谱系。对本号两处更新（增补层，原文逐字保留为发生史）：

### (1) 无边界例外达成（r* 非特殊 L0，codex 019eff21 PASS Q2）

603 证：级别递归是**终余代数（coinductive）**——向上无限 unfold（zhongshu:18 级别之次无结构上限），只有 settled moves<3 的自然终止（level_recursion.md:249），max_levels 是安全上限非缠论约束。**r* 不是"特殊的顶"，是 unfold 到当前的最深层，和任何 Move(ℓ) 用同一三构造子。** "r* 真顶减仓"=Move(r*) 出现 BSP(r*) ⟹ 按 BSP 标签集操作，与 Move(ℓ<r*) 完全同构，不需要"有无更高级别"if 特判。

**∴ protect_core 特例是 rec_engine 级别记账的产物，不是缠论递归结构的必要。** 597 仓位塔每级 Move(ℓ) 同三构造子，r* 无"顶部特殊"障碍，**无边界例外达成（L0）。** 601 的离散二分（emerg∈{有k+1,无k+1}）在递归类型范式根本不产生——r* 有没有 r*+1 由终余代数是否继续 unfold 决定（非 Move 构造子字段）。

**诚实分层（codex Q2 补充）**：仓位**数量**还依赖成本阶段状态（降成本→归零→挣股票，第31课），不是仅由 BSP 标签决定。这是 payoff 层（L3），不影响形态层（L0）的 r* 非特殊。597 无边界例外是**形态层 L0** 达成；payoff 层配额数值依成本阶段 L3 仍数据阻塞。

### (2) sigma_deviation_DEFER 改判=542 守住（codex 019eff17 澄清，无真冲突）

| | 原诊断（pre-603） | ★603 改判（codex 019eff17） |
|---|---|---|
| f_k≠f_{k+1} 的性质 | 值层逐级差异 = schema 层破坏 542 σ-不变 | 值层逐级差异 = 定义域类型差异的外生实现（输入对象类型不同），schema 层 σ-不变成立 |
| 与 542 关系 | 同层同对象直接矛盾（不可分层），supersede DEFER | 无真冲突（σ-不变规则层与逐级类型差异一致） |
| 542 状态 | 待编排者裁 supersede 合法性（生成态压 settled） | 守住（settled），无 schema 冲突 |
| 剩余裁定项 | supersede 谱系优先级 + L3 净收益 | payoff 层 α*_k 配额数值是否 L3 净收益超 BH（编排者裁） |

**codex 019eff17 逐字依据**：「缠论规则层面是 σ-不变，σ-deviation 不是缠论操作语义的数学必要性。不同级别分类规则相同（盘整/上涨/下跌/中枢发展/买卖点三分类）。变化的是输入对象的类型：第 k 级由第 k+1 级走势组成，所以 f_k 与 f_{k+1} 严格类型论上定义域不同，但在级别平移 σ 下是同一分类 schema。若实践中用不同阈值/力度度量，那是外生实现差异，非规则本身改变。」

**关键澄清**：原文"数学必要性已坐实：f_k≠f_{k+1} 是逐级塔定义的直接推论"在**值层**成立（不同级别配额数值不同，因输入对象类型不同），但这**不破** 542 的**schema 层** σ-不变（分类规则跨级别相同）。原诊断把值层差异误当 schema 层破坏，codex 019eff17 澄清二者分层：**值层逐级（定义域类型差异外生实现）⊥ schema 层 σ-不变（规则跨级别相同）**。542 守住。

**与 542 谱系一致性**：542 的 σ-不变是"配额比例 f 必须级别无关（schema 层）"——codex 019eff17 的"σ-不变规则层成立"与 542 完全一致。597 逐级 α*_k 的"逐级"是输入对象类型差异的外生实现（payoff 层数值），不是 schema 层破坏。**∴ 597 与 542 无真冲突，542 维持 settled。**

**结算状态**：本号仍**生成态**，603 是其无边界例外达成 + σ-不变改判（增补非结算）。542 守住（settled，supersede 改为 payoff 层 L3 净收益问题）。597-603 整族 + 576 待编排者 /ritual 在统一编号空间统一结算。
