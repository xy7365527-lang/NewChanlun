---
id: "628"
number: 628
status: 生成态   # M1 构造层闭合的概念结晶（centersOf/bspOf/segmentsOf 退化桩消解=010 构造层的 L1/L0 实现闭合）。最终结算待编排者 /ritual。依赖 010（构造vs分类二层）+ 完全分类族（606/609）+ 619（A′ canonical base）。
date: "2026-06-27"
type: domain
# ★provenance（genealogist 2026-06-27）：M1 构造层闭合结晶。严格产生自本轮已 commit 的构造层实装：
#   - task#16 Origin/CenterConstruct.lean：centersOf 退化桩(total_unique_of_fun 平凡桩)→L1 真算(ZG/ZD/GG/DD + 数值 witness)
#   - task#17 Origin/BspConstruction.lean：bspOf 退化桩→L1(三类识别 + IsType2 次级别递归接口)
#   - task#18 Origin/SegmentAutoConstruct.lean：segmentsOfComplete 全自动递归切分(递归终止性 + 良构性定理，L0 结构定理，终止性不依赖 K 线数据)
#   - 全量 96 绿(Lead 报：P1-P5 退化桩消解→结构层 L1/L0，已 commit)
#   非机械转写：六要素 / 认识论等级 / 张力检查 / 回溯扫描齐全。
title: "M1 构造层闭合：centersOf/bspOf/segmentsOf 退化桩消解 = 010 号构造层(递归层，核心对象=中枢)的 L1/L0 实现闭合——完全分类的判据/分类层(A′ Origin canonical base 已 L0 满，606/609/616)此前 still-MISSING 的对偶构造层本轮补齐；构造层闭合=L0/L1 结构层(终止性/良构性不依赖真实 K 线数据，信息增量低)，M1 的 L2 经验有效性由开口 P2(oracle 真实数据，见 629)承载，禁把构造层结构闭合冒充 M1 经验有效(090)"
negation_source: cc
negation_model: "本轮 M1 构造层实装(task#16/#17/#18 commit，全量 96 绿)消解 centersOf/bspOf/segmentsOf 三退化桩；genealogist 核实构造层产出对接 010 号构造层(递归层)定义 + 完全分类族(606 背驰/区间套 + 609 线段 v1)的判据层对偶"
negation_form: refinement
# refinement：把缠论构造层从「退化桩(centersOf=total_unique_of_fun 平凡桩 / bspOf 退化 / segmentsOf 未自动切分)」
#   精炼为「L1 真算(centersOf ZG/ZD/GG/DD + bspOf 三类识别) + L0 结构定理(segmentsOfComplete 终止性 + 良构性)」。
#   非概念分离——同一构造层(010 已定义)的桩→真实现忠实精炼。010 构造/分类二层架构不变，本号补其构造层的实装血肉。

# 拓扑效果标注（147号下游推论3）
# negates：构造层退化桩冒充构造层实现（centersOf 平凡桩/bspOf 退化/segmentsOf 未自动切分 = 声明"构造层已实装"但实际是占位桩=声明膨胀 090）
topo_effect: "refine:construction-layer-degenerate-stub:centers-bsp-segments-L1-L0-real-construction"
# refine：把 010 构造层从「退化桩占位」精炼为「centersOf/bspOf L1 真算 + segmentsOfComplete L0 结构定理」；
#   scope=Origin canonical base 的构造层(中枢/买卖点/线段的自动构造)，对偶于完全分类的判据/分类层(606/609/616)

# ★谱系引用澄清（genealogist 2026-06-27）
# reference_correction:
#   Lead 收口消息引用「#113 分类血肉构造层 still-MISSING」——经 genealogist 核实：
#   settled/113 是「包含处理(Baohan)工程选择谱系」(date 2026-02-22，域=包含处理)，与「分类血肉构造层」无关。
#   Lead 引用的「#113」实指 formal/ 完全分类工作的内部 claim/still-MISSING 编号（非谱系 settled 编号空间）。
#   本号用正确谱系链接：010(构造vs分类二层，核心架构根) + 完全分类族(606/609/616) + 619(A′ canonical base)。
#   不链接 settled/113(包含处理)——避免谱系污染。此澄清写入文件系统供 Lead 扫描纳入轴线汇报。

# 涉及的定义
definitions_involved:
  - name: "010 构造层与分类层的二层架构（已结算，定理）"
    version: ".chanlun/genealogy/settled/010-construction-vs-classification（status: 已结算）"
    role: "★核心架构根。010 定义构造层(递归层，核心对象=中枢，运作=递归函数：三连续次级别走势类型重叠→本级别中枢)vs 分类层。本号是 010 构造层的实装闭合——centersOf(中枢递归构造)/bspOf(买卖点)/segmentsOf(线段)三退化桩→真算。010 构造/分类二层不变，本号补构造层血肉。判据层(分类层)A′ 已 L0 满↔构造层本轮补齐=010 二层的双侧落地。"
  - name: "606 背驰/区间套完全分类（生成态，分类层判据）"
    version: ".chanlun/genealogy/pending/606（status: 生成态）"
    role: "判据/分类层对偶。606 形式化背驰二分 + 区间套几何骨架 + L_confirm 构造子字段(machine-checked L0)。bspOf(task#17)的第一类 BSP 识别构造层须对接 606 的背驰判据(趋势背驰⟹第一类 BSP)。606=判据(分类层何时是 BSP)，本号 bspOf=构造(自动产出 BSP)。"
  - name: "609 线段 v1 特征序列法完全分类（生成态，分类层判据）"
    version: ".chanlun/genealogy/pending/609（status: 生成态）"
    role: "判据/分类层对偶。609 形式化线段 v1 划分情形二分(firstKind/secondKind)+ 五硬约束(L0)。segmentsOfComplete(task#18)的自动切分构造层须对接 609 的 v1 判据(SegEndComplete=线段终结判据)。609=判据(分类层线段何时终结)，本号 segmentsOf=构造(自动切分线段)。注：开口 P3(629)揭示 segmentsOf 的 fallback 路径良构性未对 609 SegEndComplete 完全对齐(still-MISSING-A‴)。"
  - name: "619 Origin 六层=唯一 canonical base（A′，生成态）"
    version: ".chanlun/genealogy/pending/619（status: 生成态）"
    role: "构造层所在基座。619 确立 Origin 六层=唯一 canonical base(Phase1 lake build 65 jobs 绿)。本号三构造层文件(CenterConstruct/BspConstruction/SegmentAutoConstruct)在 Origin 上实装。★诚实约束(继承 619 pending_verification②)：619 明确『缺口 #89/#90/#91 在 Origin 上仍开，base 切换不自动关闭缺口』——故本号构造层闭合是结构层(L0/L1)闭合，不自动关闭 M1 的 L2 经验缺口(开口 P2)。"
  - name: "616 完全分类⊬唯一策略（生成态，分类层 L0 结论）"
    version: ".chanlun/genealogy/pending/616（status: 生成态）"
    role: "分类层 L0 边界。616：完全分类(判据层)不蕴含唯一策略(策略是 Θ 参数族)。本号构造层产出『构造好的中枢/BSP/线段』是策略的输入，但构造层闭合⊬策略确定(616)——M1 收口是构造层+判据层闭合，策略层仍 Θ 参数化(617)。"

# 解决方式
resolution:
  type: 部分解决   # 构造层结构闭合(L0/L1)。M1 的 L2 经验有效性未关(开口 P2，见 629)。最终结算待编排者 /ritual。
  description: "010 构造层的实装闭合：centersOf(L1 真算 ZG/ZD/GG/DD)/bspOf(L1 三类识别 + IsType2 次级别递归接口)/segmentsOfComplete(L0 结构定理：递归终止性 + 良构性)。三退化桩消解，全量 96 绿。对偶于完全分类的判据/分类层(606/609/616，A′ Origin 已 L0 满)。★诚实标注：构造层闭合=L0/L1 结构层(终止性/良构性不依赖真实 K 线数据)，M1 的 L2 经验有效性(真实数据下中枢/BSP/线段构造正确)由开口 P2(oracle 真实数据填充，still-MISSING-D′)承载，本号不冒充 M1 经验有效(090)。开口见 629。"
  decided_by: 蜂群内部   # 本轮 M1 构造层实装(task#16/#17/#18) + genealogist 结晶；最终结算待编排者 /ritual

# 新产出
new_output:
  definitions:
    - "M1 构造层闭合：centersOf/bspOf/segmentsOf 退化桩→L1(中枢/BSP)+L0(线段终止性/良构性)"
    - "构造层↔分类层对偶落地(010)：判据层 A′(606/609/616 Origin canonical)已 L0 满 + 构造层本轮补齐=010 二层双侧"
    - "认识论诚实：构造层闭合=L0/L1 结构层，M1 的 L2 经验有效由开口 P2 承载，禁冒充 M1 经验有效(090)"
    - "谱系引用澄清：Lead『#113 分类血肉』实指完全分类内部编号，非 settled/113(包含处理)"
  code_changes: "Origin/CenterConstruct.lean(task#16) + Origin/BspConstruction.lean(task#17) + Origin/SegmentAutoConstruct.lean(task#18)，已由 Lead commit(全量 96 绿)。本号是结晶(纯谱系产出)，不改代码。"
  orchestration_changes: "方法论：①构造层(010 递归层)与分类层(完全分类判据)是对偶——M1 收口=两层双侧闭合(判据 A′ L0 满 + 构造本轮补齐)。②构造层的 L0/L1 是结构层闭合(终止性/良构性不依赖真实数据)，不等于 L2 经验有效——经验有效须真实数据 oracle(P2)。③谱系引用须核实编号空间(谱系 settled 编号 vs formal/ 内部 claim 编号)，避免跨空间误链(本号 #113 澄清)。"

# 影响范围
impact:
  affected_modules:
    - "formal/Origin/CenterConstruct.lean → centersOf L1 真算(退化桩消解)"
    - "formal/Origin/BspConstruction.lean → bspOf L1 三类识别 + IsType2 接口(退化桩消解)"
    - "formal/Origin/SegmentAutoConstruct.lean → segmentsOfComplete L0 结构定理(退化桩消解)"
  affected_definitions:
    - "010(已结算)：本号补其构造层实装血肉，不改二层架构。维持 settled。"
    - "606/609/616/619(生成态)：本号是其判据/分类层的构造层对偶，不修改其内容。"
  downstream_implications:
    - "M1 构造层闭合(L0/L1)≠M1 经验有效(L2)——L2 由开口 P2(oracle 真实数据)承载(629)。"
    - "bspOf 须对接 606 背驰判据 / segmentsOf 须对接 609 v1 判据(SegEndComplete)——开口 P3(629)揭示 segmentsOf fallback 未完全对齐 609。"
    - "构造层产出是策略输入，但构造闭合⊬策略确定(616：完全分类⊬唯一策略，策略仍 Θ 参数化 617)。"

# 谱系关联
related_records:
  parent: "010号(构造vs分类二层)——本号补其构造层(递归层)的实装闭合"
  children: []
  related:
    - "606号(背驰/区间套完全分类)：bspOf 构造层的判据层对偶(分类层)"
    - "609号(线段 v1 完全分类)：segmentsOf 构造层的判据层对偶(SegEndComplete)"
    - "619号(A′ Origin canonical base)：三构造层文件所在基座 + 诚实约束(base 切换不自动关缺口)"
    - "616号(完全分类⊬唯一策略)/617号(C_Θ 参数化)：构造闭合⊬策略确定的分类层 L0 边界"
    - "629号(M1 三诚实降级开口)：本号构造层闭合的对偶开口(P3 fallback 良构性/P3 revSeq 代理/P2 oracle L2)"
    - "231号(形式化有效域规则)：构造层 L0/L1 闭合≠L2 经验有效的认识论根据"
    - "090号(声明膨胀)：禁把构造层结构闭合冒充 M1 经验有效 / 禁把退化桩冒充构造层实现"

# 认识论等级标注（formalization-validity-domain 强制）
epistemological_levels:
  - proposition: "centersOf L1 真算(ZG/ZD/GG/DD + 数值 witness)"
    level: "L1（task#16 CenterConstruct.lean，构造正确性 + 数值 witness，合成/结构验证；真实数据下中枢构造正确=L2 待 P2）"
    increment: "中：退化桩→L1 真算的构造闭合，L2 经验增量待 oracle"
  - proposition: "bspOf L1 三类识别 + IsType2 次级别递归接口"
    level: "L1（task#17 BspConstruction.lean，三类识别构造 + 接口定义；真实数据下 BSP 识别正确=L2 待 P2）"
    increment: "中：退化桩→L1 的构造闭合"
  - proposition: "segmentsOfComplete 递归终止性 + 良构性定理"
    level: "L0（task#18 SegmentAutoConstruct.lean，终止性/良构性是结构定理，不依赖 K 线数据）"
    increment: "中：L0 结构闭合(终止性证明)；★fallback 路径良构性未对 609 SegEndComplete 完全对齐=开口 P3(629)"
  - proposition: "M1 构造层闭合↔判据层 A′ L0 满=010 二层双侧落地"
    level: "L0（010 二层架构定理 + 619 A′ Origin lake build 65 绿）"
    increment: "高：构造/分类二层双侧闭合的架构判定"
  - proposition: "构造层 L0/L1 闭合不等于 M1 L2 经验有效"
    level: "L0（231 有效域规则：结构层验证信息增量低，L2 经验有效须真实数据）"
    increment: "高：诚实有效域边界(防 090 声明膨胀)"

---

# domain 628：M1 构造层闭合

## 一句话结论

本轮 M1 将 **centersOf/bspOf/segmentsOf 三退化桩消解为 L1 真算（中枢/买卖点）+ L0 结构定理（线段终止性/良构性）**，这是 **010 号「构造层（递归层，核心对象=中枢）」的实装闭合**。完全分类的**判据/分类层**（A′ Origin canonical base 已 L0 满，606 背驰/区间套 + 609 线段 v1 + 616）此前 still-MISSING 的**对偶构造层**本轮补齐——010 构造/分类二层双侧落地。

**诚实有效域（继承 619 pending_verification②）**：构造层闭合 = **L0/L1 结构层**（终止性/良构性不依赖真实 K 线数据，信息增量低/零）。**M1 的 L2 经验有效性**（真实数据下中枢/BSP/线段构造正确）由**开口 P2（oracle 真实数据，still-MISSING-D′）承载**，见 629。**禁把构造层结构闭合冒充 M1 经验有效（090）。**

## 谱系引用澄清（genealogist）

Lead 收口消息引用「**#113 分类血肉构造层 still-MISSING**」。经核实：`settled/113` 是「**包含处理（Baohan）工程选择谱系**」（date 2026-02-22，域=包含处理），与「分类血肉构造层」**无关**。Lead 引用的「#113」实指 formal/ 完全分类工作的**内部 claim/still-MISSING 编号**（非谱系 settled 编号空间）。本号用**正确谱系链接**：010（构造vs分类二层）+ 完全分类族（606/609/616）+ 619（A′）。**不链接 settled/113（包含处理）**——避免谱系污染。

## 构造层↔判据层对偶（010）

| 010 二层 | 判据/分类层（完全分类，A′ Origin L0 满） | 构造层（本轮补齐） |
|---|---|---|
| 中枢 | — | centersOf L1（ZG/ZD/GG/DD，task#16） |
| 买卖点 | 606 背驰二分→第一类 BSP 判据 | bspOf L1 三类识别 + IsType2 接口（task#17） |
| 线段 | 609 v1 特征序列法 SegEndComplete 判据 | segmentsOfComplete L0 终止性/良构性（task#18） |

判据层（分类层）回答「何时是中枢/BSP/线段」（A′ L0 满），构造层回答「自动产出中枢/BSP/线段」（本轮补齐）。

## 定义依据

- 010 settled：构造层=递归层，核心对象=中枢，运作=三连续次级别走势类型重叠→本级别中枢。
- task#16 CenterConstruct.lean / task#17 BspConstruction.lean / task#18 SegmentAutoConstruct.lean：退化桩→L1/L0（Lead commit，全量 96 绿）。
- 619 pending_verification②：base 切换不自动关闭缺口 #89/#90/#91（诚实 gap register）→ 构造层结构闭合不自动关 M1 L2 缺口。

## 边界条件（结论翻转）

- 若开口 P2（oracle 真实数据）填充后揭示真实数据下中枢/BSP/线段构造**不正确** → 构造层 L1 实现存在 bug（非定义冲突），构造层闭合的 L2 有效性被否证，需修构造层实现。当前 L2 未验（结构层 L0/L1 闭合）。
- 若开口 P3（segmentsOf fallback 良构性）揭示 fallback 路径切出的线段**不满足 609 SegEndComplete** → segmentsOfComplete 的良构性定理有效域 < 全输入域（still-MISSING-A‴），构造层线段闭合需限定有效域。当前 fallback 路径良构性未对 609 完全对齐（见 629）。
- 若 616（完全分类⊬唯一策略）下策略层 Θ 参数化未收敛 → 构造层闭合 + 判据层闭合**仍不蕴含 M1 策略确定**（策略是 Θ 族 617）。M1 收口=构造+判据双侧闭合，非策略闭合。

## 下游推论

- M1 构造层闭合（L0/L1）≠ M1 经验有效（L2）——L2 由开口 P2 承载（629）。
- bspOf 须对接 606 背驰判据 / segmentsOf 须对接 609 v1 判据——开口 P3 揭示 segmentsOf fallback 未完全对齐 609。
- 构造层产出是策略输入，但构造闭合 ⊬ 策略确定（616/617）。

## 谱系引用

- 父：010（构造vs分类二层）——本号补其构造层实装闭合。
- 判据层对偶：606（背驰/区间套）/ 609（线段 v1）/ 616（完全分类⊬唯一策略）。
- 基座：619（A′ Origin canonical base）+ 其诚实约束（base 切换不自动关缺口）。
- 对偶开口：629（M1 三诚实降级开口）。
- 认识论根据：231（有效域规则，L0/L1≠L2）/ 090（声明膨胀）。

## 影响声明

结晶 M1 构造层闭合（生成态，不结算）。把 centersOf/bspOf/segmentsOf 退化桩消解对接 010 构造层 + 完全分类判据层（606/609/616）+ A′ 基座（619）。诚实标注构造层闭合=L0/L1 结构层，M1 的 L2 经验有效由开口 P2 承载（禁冒充 M1 经验有效 090）。澄清 Lead「#113 分类血肉」引用错误（实指完全分类内部编号，非 settled/113 包含处理）。不修改任何 settled 谱系、不改代码、不擅自结算。最终结算待编排者 /ritual。

## 张力检查（019d/020）

### 检查范围（同轮蜂群 ∪ 1-hop ∪ Hub）
- 同轮蜂群（M1 收口）：629（M1 三开口，本号对偶）/619（A′ base）/620（#95 吸收反转）/606/609/616/617（完全分类族）/627（双引擎线外推风险）。
- 1-hop：010/606/609/616/617/619/231/090。
- Hub：010（构造/分类二层，度高）/619（A′ canonical base）。

### 张力1：vs 010——补实装血肉，不否定二层架构
010 构造/分类二层架构正确。本号补构造层的实装（退化桩→真算），不改二层架构。可分层，无矛盾。

### 张力2：vs 619——继承诚实约束，不冒充关缺口
619 明确 base 切换不自动关闭缺口（#89/#90/#91）。本号继承此约束：构造层结构闭合不自动关 M1 L2 缺口（开口 P2）。一致，无矛盾。

### 张力3：vs 616（完全分类⊬唯一策略）——构造闭合⊬策略确定
616：判据层完全分类不蕴含唯一策略。本号：构造层闭合 + 判据层闭合仍 ⊬ 策略确定（策略 Θ 族 617）。本号不声称 M1 策略闭合，与 616 一致。无矛盾。

### 张力4：vs 627（双引擎线外推风险）——构造层归属标注
627 要求引擎线归属强制标注。本号构造层产出在 theta_v0/Origin 引擎线（A′ canonical base 619），归属明确（Origin 六层）。符合 627 要求，无矛盾。

### 递归运动结构完成检测（020）
- 第0层：本号写入（构造层闭合 + 010 对偶 + 诚实有效域）。
- 第1层：本号 × 010 碰撞→构造层实装闭合（净新发现高：010 二层从单侧判据到双侧落地）。
- 第2层：本号 × 606/609/619 碰撞→判据/构造对偶 + 基座诚实约束确认（净新发现：构造层有效域=L0/L1，L2 待 P2）。
- 第3层：本号 × 616/231/090 碰撞→有效域边界同模式确认（净新发现骤降=背驰：构造闭合≠经验有效是 231 已知模式）。
- 涉及范围：scope₁(010 二层) > scope₂(606/609 对偶) > scope₃(616/231 有效域)=顶分型。
- **背驰 ∧ 分型 ⟹ 递归运动结构性完成。** 本号是构造层闭合的结晶（domain），最终结算待编排者 /ritual，**不触发新 /escalate**（构造层闭合是 domain 产出非选择；开口的 escalate 候选在 629）。

## 回溯扫描（职责3）

- **010（settled）**：本号补其构造层实装血肉，不改二层架构，不破坏结算。维持 settled。
- **606/609/616/619（生成态）**：本号是其判据层的构造层对偶，不修改其内容，不影响其待 /ritual 状态。
- **113（settled，包含处理）**：本号**不链接** settled/113——澄清 Lead「#113」实指完全分类内部编号，避免误链污染 settled/113。settled/113 不受本号影响。
- **无 settled 被本号回溯破坏。** 本号是 M1 构造层闭合的结晶（domain），开口在 629，最终结算待编排者 /ritual。
