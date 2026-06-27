---
id: "632"
number: 632
status: 已结算   # 【结算 2026-06-27 codex异质委托：路1已实施真封·divPair L2开口诚实不阻塞】 oracle MoveJudgment 消桩矛盾上浮（escalate 候选，待编排者 /ritual）。本号精确化 629 开口③：MoveJudgment 三 Bool 字段不可从元素层 Move 纯 L0 结构推导，根因=movesOf 丢弃 Segment 价格端点（非 L2 真实数据缺口）。依赖 629(开口③定性)+ 231(有效域规则)+ 608(中枢位置三态)+ no-workaround。
date: "2026-06-27"
type: 矛盾发现
depends_on: ["629", "231", "608"]
related: ["628", "606", "619", "090", "161", "no-workaround", "no-patch-mentality"]
title: "oracle MoveJudgment 消桩=L0 结构缺口非 L2 数据缺口：MoveJudgment 三 Bool 字段(brokeCenter/leftCenter/afterTypeOne)不可从元素层 Move 纯结构推导，根因=canonical movesOf:List Segment→List Move 丢弃 Segment 携带的价格端点(startPrice/endPrice)，致下游 bspOf:List Move→List Bsp 拿不到产买卖点判据所需的价格-中枢几何；SubLevelDescent 能真下钻判破中枢仅因 RMove 携带 interval(lo/hi)，元素层 Move 无此字段。修复方向是 canonical 类型契约选择(给 Move 加价格字段 vs 改 bspOf 签名接 Segment vs 接受 oracle 注入永久外部化)，触及 P1 owner ChanlunElements，escalate 候选；与 629 开口③『等 L2 真实数据填充』定性冲突——价格几何在 Segment 层即 L0 结构可得，非 L2 经验。"
negation_source: cc
negation_model: "SG-oracle-fill 工位执行 g-complete-classification-full-strategy acceptance#2(MoveJudgment 降级为结构真实推导，消 dummyJudgmentFromMove 桩)，逐字段分析三 Bool 字段可推导性后判定：元素层 Move 结构域信息不足以纯 L0 推导任一字段，根因在 movesOf 丢弃 Segment 价格端点(canonical 契约层)，非 629 定性的 L2 真实数据缺口"
negation_form: separation
# separation：629 开口③把 MoveJudgment oracle 桩统一定性为『L2 真实数据填充缺口(still-MISSING-D′)』；
#   本号分离出该定性内部的两个不同有效域：
#   ①价格几何(brokeCenter/leftCenter)的判定数据=Segment.startPrice/endPrice(L0 结构字段，管线在笔→线段层已算出)，
#     不是 L2 经验数据——是 movesOf 构造 Move 时『丢弃』了它(canonical Move 无 price 字段)。
#   ②真正的 L2 缺口是背驰力度(divPair 的 Force/MACD 面积，Divergence.lean still-MISSING-C)，
#     那确实需 L2 数值引擎(EMA/DIF/DEA)。
#   629 把①②混为一谈(都归 L2)；本号分离：①是 L0 结构丢失(canonical 契约缺陷)，②才是 L2 数据缺口。

# 拓扑效果标注（147号下游推论3）
# negates：629 开口③『MoveJudgment 消桩=L2 真实数据填充』的单一定性——揭示其中价格几何分量是 L0 结构缺口(movesOf 丢弃 Segment 价格)，可在不等 L2 引擎的前提下结构消桩(若 canonical 契约允许 bspOf 接价格载体)
topo_effect: "split:629-opening3-oracle-L2-data-fill:L0-structural-segment-price-drop-vs-L2-divergence-force-engine"
# split：629 开口③『oracle L2』分裂为『L0 结构缺口(价格几何，movesOf 丢 Segment 价格)』+『L2 数据缺口(背驰力度，MACD 引擎 still-MISSING-C)』；
#   scope=MoveJudgment 三 Bool 字段(brokeCenter/leftCenter 几何 + afterTypeOne 时序) + divPair 力度的有效域分层

# 矛盾（type=矛盾发现 必填）
contradiction:
  description: "acceptance#2 要求 MoveJudgment 从结构真实推导(L0)消 dummyJudgmentFromMove 桩。逐字段分析三 Bool 字段(BspClassification.BspEndpoint 语义)在元素层 Move(ChanlunElements.Move：kind/startIndex/endIndex/centers，无价格端点)上的可推导性，三字段均不可纯 L0 结构推导：

  ①【brokeCenter(是否突破/跌破最后一个中枢)】语义(BspClassification §10.1)=走势末端价格相对中枢核心[zd,zg]的几何位置(破中枢=末端价越过 zd/zg，对应 CenterStates IsBelow/IsAbove，608)。判定需『走势末端价格』。元素层 Move 只有 endIndex(下标)，无 endPrice(价格)。Center 有 zd/zg 但缺另一比较元(走势价)。⟹ 缺价格端点，不可纯结构推导。

  ②【leftCenter(是否离开中枢)】语义=走势末端价格脱离[zd,zg]。同缺末端价格。且 ClassificationGuardrail.lean 已证 kind(趋势/盘整，由中枢数量决定，twoCenters)与 leftCenter(是否离开当前中枢)是正交维度(『趋势内 twoCenters=true∧leftCenter=false』≠『趋势末段 twoCenters=true∧leftCenter=true』)——故不能用 Move.kind 替代 leftCenter(用 kind 推 leftCenter=违反已证正交性)。⟹ 缺价格端点 + kind 正交不可替代，不可纯结构推导。

  ③【afterTypeOne(是否在第一类买卖点之后)】语义=当前走势相对前序第一类买卖点的时序。单个 Move 孤立无前序上下文；即使有走势序列，『识别前序是否第一类』又递归依赖 brokeCenter+背驰(①已缺)。⟹ 缺列表级时序上下文 + 递归依赖①，不可纯结构推导。

  三字段共同根因(非三个独立缺口)：元素层 Move 不携带价格几何端点。对比 SubLevelDescent.lean 能对 RMove 真下钻判 SubBrokeBelow/SubBrokeAbove(破中枢几何)——仅因 RMove(Formal.RecursiveConstruction.Move)携带 interval(lo/hi 价格区间)。元素层 ChanlunElements.Move 无 interval/price 字段。

  ★与 629 开口③定性的冲突：629 把 MoveJudgment oracle 桩定性为『L2 真实数据填充缺口(still-MISSING-D′)，等真实数据』。但本号分析揭示：brokeCenter/leftCenter 的判定数据=Segment.startPrice/endPrice，这是 L0 结构字段——管线在 strokesOf→segmentsOf 层(ChanlunElements §)已算出 Segment 价格。是 movesOf:List Segment→List Center→List Move 构造 Move 时『丢弃』了 Segment 的 startPrice/endPrice(canonical Move 结构无 price 字段)。故价格几何分量是 L0 结构缺口(canonical 契约丢信息)，不是 L2 经验缺口。真正的 L2 缺口只有 divPair 的背驰力度(Force/MACD，Divergence still-MISSING-C)。

  ★canonical 契约矛盾(不可弥合点)：
  - 接受 A『canonical bspOf:List Move→List Bsp 契约不变』⟹ Move 必须自携产买卖点的全部判据数据(价格几何+力度)，但 Move 结构无价格端点 ⟹ 要么改 Move 加 startPrice/endPrice 字段(改 canonical 类型，P1 owner)，要么承认 bspOf 无法从 Move 单独 L0 实装(永久依赖外部 oracle 注入=629 的 oracle pattern)。
  - 接受 B『Move 结构不变(信息不足是既成事实)』⟹ ElementPipeline.bspOf:List Move→List Bsp 契约本身有缺陷(声称从 Move 列表产买卖点，但 Move 信息不足) ⟹ canonical 契约须改签名(如 bspOf:List Segment→List Center→List Move→List Bsp，传入价格载体)。
  两条路均触及 P1 owner 的 canonical ElementPipeline/Move 定义，BspConstruction owner 不能单方裁定。"
  layer: formal/架构层 + 缠论域   # canonical 类型契约(架构层) + 买卖点判据数据需求(缠论域 §10.1)
  trigger: "SG-oracle-fill 工位执行 acceptance#2(消 dummyJudgmentFromMove，MoveJudgment 结构真实推导) + 逐字段可推导性分析 + 对比 SubLevelDescent RMove.interval 可下钻 vs 元素层 Move 无价格"

# 涉及的定义
definitions_involved:
  - name: "629 开口③ oracle L2 真实数据填充（生成态，本号对偶/精确化母号）"
    version: ".chanlun/genealogy/pending/629（status: 生成态）"
    role: "★母号。629 把 MoveJudgment oracle 桩定性为单一『L2 真实数据填充缺口(still-MISSING-D′)』。本号分离该定性：价格几何分量(brokeCenter/leftCenter)是 L0 结构缺口(movesOf 丢 Segment 价格)，非 L2；只有背驰力度(divPair)才是真 L2 缺口(MACD 引擎)。本号不否定 629 有开口，精确化开口③的有效域分层(L0 结构 vs L2 数据)。"
  - name: "231 形式化有效域规则（已结算，认识论 L0-L3 标注）"
    version: ".chanlun/genealogy/settled/231（status: 已结算）"
    role: "本号的母规则与判据。231 要求标注 L 级。本号核心=区分 L0(结构推导，价格几何在 Segment 层即得)与 L2(经验数据，背驰力度需真实 MACD)——629 把二者混为 L2 是 L 级误标。L0→L1 信息增量为零(合成数据)；价格几何是 Segment 真实结构字段(管线已算)，补 Move 价格字段后可 L0 推导，不产生合成确认偏差。"
  - name: "608 中枢位置三态（生成态，点相对[ZD,ZG]的 below/within/above）"
    version: ".chanlun/genealogy/pending/608（status: 生成态）"
    role: "brokeCenter/leftCenter 的判据基准。CenterStates.IsBelow/IsAbove/classifyPosition 给出『价格点 p 相对中枢[zd,zg]』的三态判定。brokeCenter(破中枢)=末端价 below(买点跌破 zd)或 above(卖点突破 zg)；leftCenter(离开中枢)=末端价 below∨above。这些判定函数已存在(L0)，缺的只是输入『走势末端价格 p』——而 p 在 Segment.endPrice 层已有，Move 丢弃了它。"
  - name: "ChanlunElements.Move / ElementPipeline.bspOf（canonical 契约，P1 owner，本号矛盾载体）"
    version: "formal/Origin/ChanlunElements.lean:65-70(Move) + :111(bspOf)"
    role: "★矛盾的 canonical 载体。Move 结构={kind,startIndex,endIndex,centers}，无 startPrice/endPrice。movesOf:List Segment→List Center→List Move 从带价格的 Segment 构造无价格的 Move(丢信息)。bspOf:List Move→List Bsp 下游拿不到价格几何。两条修复路(补 Move 价格字段 / 改 bspOf 签名)均改 canonical，P1 owner 裁定。"
  - name: "SubLevelDescent SubBrokeBelow/SubBrokeAbove（生成态，RMove 破中枢几何真下钻）"
    version: "formal/Origin/SubLevelDescent.lean:157-178"
    role: "对照证据：破中枢几何在『携带价格区间的载体』上 L0 可判。RMove(Formal.RecursiveConstruction.Move)携带 interval(lo/hi)，SubBrokeBelow m c:=m.lo<c.zd 纯 L0 结构判定。这证明 brokeCenter 本质是 L0 几何(非 L2)——只要载体携带价格。元素层 Move 缺 interval/price 是 canonical 契约缺陷，非判据本身需 L2。"
  - name: "Divergence still-MISSING-C（生成态，背驰力度 Force/MACD 无计算引擎=真 L2 缺口）"
    version: "formal/Origin/Divergence.lean §1 + 文件尾 still-MISSING-C"
    role: "唯一的真 L2 缺口。divPair 的 Force(MACD 柱子面积)无任何 Origin 模块从 K 线计算(需 EMA/DIF/DEA + 面积积分=L2 数值引擎)。MoveJudgment 的 divPair 字段填充确实需 L2。本号区分:价格几何(brokeCenter/leftCenter)=L0 结构(可补)，背驰力度(divPair)=L2 数据(需引擎)——629 开口③的 L2 定性只对后者成立。"

# 解决方式
resolution:
  type: 未解决   # canonical 类型契约选择(改 Move 加价格 vs 改 bspOf 签名 vs 接受 oracle 永久外部化)=选择类，触及 P1 owner，escalate 候选待编排者 /ritual。
  description: "本号是 629 开口③的精确化分层(L0 结构缺口 vs L2 数据缺口)。修复方向三选一均为选择类(需价值判断)+触及 P1 owner canonical 类型，escalate 候选：
  (路1) 给 ChanlunElements.Move 增 startPrice/endPrice 字段(movesOf 从 Segment 端点透传)，brokeCenter/leftCenter 即可 L0 结构推导(用 608 位置三态)——价格几何分量 L0 消桩，仅 divPair 力度留 L2 开口。
  (路2) 改 bspOf 签名为接价格载体(如 List Segment→...→List Bsp)，不改 Move 结构。
  (路3) 接受 MoveJudgment 由外部 oracle 注入(629 现状)，承认 bspOf 从 Move 单独不可 L0 实装，oracle 是契约性外部化(非桩)——但需删除『dummyJudgmentFromMove 零判据冒充完整分类』的声明膨胀(本工位已做诚实化:见影响声明)。
  三路的选择(及 brokeCenter/leftCenter 该不该等 divPair 力度 L2 一起填)=价值判断，待编排者 /ritual。蜂群不擅自改 canonical、不擅自补丁让 Move→空冒充绑定。"
  decided_by: 蜂群内部   # SG-oracle-fill 工位分析三字段不可 L0 推导 + 根因定位(movesOf 丢 Segment 价格) + L0/L2 分层；修复路径选择=选择类 escalate 候选待编排者 /ritual

# 被否定的方案
negated:
  description: "(1) 保留 dummyJudgmentFromMove 零判据桩作 fallback，宣布 acceptance#2『结构推导』完成。(2) 用 Move.kind(趋势/盘整)推导 leftCenter(趋势⟹离开中枢)。(3) 把『Move 缺价格』归为 629 的 L2 真实数据缺口，等 MACD 引擎一起解决。(4) 硬编码三 Bool 字段某个为 true 制造非空输出假装『真填充』。"
  why_negated: "(1) 补丁思维+声明膨胀(no-patch-mentality/090)：零判据恒产空列，不是『结构推导』而是退化冒充；acceptance#2 明确要『从结构真实推导，无未验证 oracle 冒充完整分类(629)』，保留零判据桩=继续冒充。(2) 违反 ClassificationGuardrail 已证的 kind⊥leftCenter 正交性(twoCenters 维度与 leftCenter 维度正交)——用 kind 推 leftCenter=用已证不蕴含的量假装推导。(3) L 级误标(231)：价格几何在 Segment.endPrice 层是 L0 结构(管线已算)，归 L2 会让本可结构消桩的分量被错误推迟到 MACD 引擎之后；629 把 L0 结构缺口与 L2 力度缺口混为一谈=有效域分层不清。(4) 硬编码特例(no-workaround：把矛盾硬编码为特例)+合成确认偏差(231)。"

# 新产出
new_output:
  definitions:
    - "MoveJudgment 三 Bool 字段不可从元素层 Move 纯 L0 结构推导(brokeCenter/leftCenter 缺价格端点；afterTypeOne 缺时序+递归依赖)"
    - "根因=canonical movesOf:List Segment→List Move 丢弃 Segment 价格端点(startPrice/endPrice)，致 bspOf 下游无价格几何"
    - "L0/L2 分层:价格几何(brokeCenter/leftCenter)=L0 结构(Segment 层已算，可补 Move 字段消桩)；背驰力度(divPair)=L2 数据(MACD 引擎 still-MISSING-C)——629 开口③把二者混为 L2 是误标"
    - "对照证据:SubLevelDescent 对 RMove(携带 interval lo/hi)能 L0 判破中枢，证明破中枢本质 L0 几何，元素层 Move 缺价格是 canonical 契约缺陷"
    - "canonical 契约矛盾:bspOf:List Move→List Bsp 契约要求从 Move 单独产买卖点，但 Move 信息不足——接受 A(改 Move 加价格)或 B(改 bspOf 签名)均触及 P1 owner"
  code_changes: "本工位在 owner 文件 formal/Origin/BspConstruction.lean 做诚实化(消声明膨胀，非补丁):删除 dummyJudgmentFromMove 零判据桩 + bspOfViaPipeline + 零判据见证(它们冒充『绑定 ElementPipeline.bspOf』，实际零判据恒产空)，保留 bspOfMoves(真遍历，judgments 外部注入是诚实接口)。MoveJudgment 结构保留(它是诚实的 oracle 接口定义)。canonical 类型契约的修复(改 Move/bspOf 签名)=选择类，escalate 候选待编排者 /ritual。"
  orchestration_changes: "方法论:①oracle 桩消解前须逐字段分析可推导性(可推导数据在哪一层=L0 结构/L1 合成/L2 经验)，不笼统归 L2。②区分『结构信息不足(canonical 丢信息)』与『经验数据缺失(需引擎)』——前者补 canonical 字段即 L0 可解，后者须等 L2 引擎；混淆=L 级误标(231)。③oracle 注入接口本身可以是诚实的(契约性外部化)，但用零判据桩冒充『管线绑定完成』是声明膨胀，须诚实化。"

# 影响范围
impact:
  affected_modules:
    - "formal/Origin/BspConstruction.lean → 本工位诚实化:删 dummyJudgmentFromMove(零判据桩)+bspOfViaPipeline+零判据见证；保留 bspOfMoves(真遍历)+MoveJudgment(诚实 oracle 接口)。零 sorry 编译。"
    - "formal/Origin/ChanlunElements.lean(canonical Move/bspOf，P1 owner，只读) → 矛盾根因载体:Move 无价格字段，bspOf:List Move→List Bsp 契约信息不足。修复需 P1 裁定，本工位不改。"
    - "formal/Origin/SubLevelDescent.lean(只读) → 对照证据:RMove.interval 使破中枢 L0 可判。"
    - "formal/Origin/Divergence.lean(只读) → 真 L2 缺口:背驰力度 Force/MACD still-MISSING-C。"
  affected_definitions:
    - "629(生成态):本号是其开口③的精确化分层(L0 结构 vs L2 数据)，不否定 629 有开口，精确化定性。维持生成态待 /ritual。"
    - "231(已结算):本号是 231 L 级标注的应用(纠正 629 把 L0 结构缺口误标 L2)。不修改 231。维持 settled。"
    - "608(生成态):本号用其位置三态作 brokeCenter/leftCenter 判据基准，不修改 608。"
  downstream_implications:
    - "acceptance#2(MoveJudgment 结构真实推导消桩)在元素层 Move 上不可纯 L0 完成——根因 canonical Move 丢价格，须 P1 裁定契约修复路径。"
    - "若编排者裁定路1(补 Move 价格字段):brokeCenter/leftCenter 可 L0 结构消桩(用 608)，acceptance#2 价格几何分量达成，仅 divPair 力度留 L2 开口(承接 629 与 still-MISSING-C)。"
    - "afterTypeOne(时序+递归依赖 brokeCenter)即使补价格字段仍需走势序列级上下文(非单 Move)——可能需 bspOf 在列表级而非逐 Move 推导。"
    - "禁把 dummyJudgmentFromMove 零判据桩冒充 acceptance#2『结构推导完成』(629 开口本就是反对此冒充)。"

# 谱系关联
related_records:
  parent: "629号(M1 三诚实降级开口)——本号是其开口③(oracle L2)的精确化分层"
  children: []
  related:
    - "231号(形式化有效域规则):L 级标注母规则，本号纠正 629 对开口③的 L 级误标(L0 结构 vs L2 数据)"
    - "608号(中枢位置三态):brokeCenter/leftCenter 判据基准(IsBelow/IsAbove)"
    - "628号(M1 构造层闭合):本号承接其 bsp 构造层，揭示 bspOf 从 Move 单独不可 L0 实装"
    - "606号(背驰/区间套完全分类):IsType1=背驰点，divPair 力度=真 L2 缺口(still-MISSING-C)"
    - "619号(A′ 诚实 gap register):诚实降级范式(缺口不自动关，禁声明膨胀)"
    - "no-workaround/no-patch-mentality/090号:禁零判据桩冒充结构推导、禁 kind 推 leftCenter、禁硬编码特例"
    - "161号(否务实):不把价格几何 L0 缺口务实推迟到 L2 引擎之后"

# 认识论等级标注（formalization-validity-domain 强制）
epistemological_levels:
  - proposition: "MoveJudgment.brokeCenter/leftCenter 的判定数据=走势末端价格 vs 中枢[zd,zg]，价格在 Segment.endPrice 层是 L0 结构字段"
    level: "L0（结构事实：Segment 携带 startPrice/endPrice，管线 strokesOf→segmentsOf 已算；movesOf 构造 Move 时丢弃）"
    increment: "高：揭示价格几何是 L0 结构(可补 canonical 字段消桩)，非 629 定性的 L2 经验数据"
  - proposition: "元素层 Move 不携带价格端点，三 Bool 字段均不可从 Move 纯结构推导"
    level: "L0（源码事实：ChanlunElements.Move={kind,startIndex,endIndex,centers}无 price；kind⊥leftCenter 已证正交）"
    increment: "高：canonical Move 信息不足的结构判定(三字段共同根因)"
  - proposition: "破中枢本质 L0 几何，SubLevelDescent 对携带 interval 的 RMove 可 L0 判定"
    level: "L0（对照证据：SubBrokeBelow m c:=m.lo<c.zd 纯结构，RMove 携带 interval）"
    increment: "高：证明缺口在载体(Move 无 interval)非判据(破中枢可 L0)"
  - proposition: "背驰力度 divPair(Force/MACD)是唯一真 L2 缺口，需数值引擎"
    level: "L2 still-MISSING（Divergence still-MISSING-C：无 Origin 模块从 K 线算 MACD 面积）"
    increment: "高：分离 L0 结构缺口(价格几何)与 L2 数据缺口(背驰力度)"

---

# 矛盾发现 632：oracle MoveJudgment 消桩=L0 结构缺口非 L2 数据缺口

## 矛盾描述

acceptance#2 要求 `MoveJudgment` 从**结构真实推导（L0）**消 `dummyJudgmentFromMove` 桩。逐字段分析 `MoveJudgment` 三个 Bool 字段（语义来自 `BspClassification.BspEndpoint` §10.1）在**元素层** `Move`（`ChanlunElements.Move`：`kind/startIndex/endIndex/centers`，**无价格端点**）上的可推导性，**三字段均不可纯 L0 结构推导**：

| 字段 | 语义（§10.1/608） | 所需输入 | 元素层 Move 提供 | 可 L0 推导？ |
|------|------------------|---------|-----------------|-------------|
| `brokeCenter` | 走势末端价越过中枢 zd/zg（破中枢） | 走势末端价格 + center.zd/zg | endIndex（无 endPrice）、centers(zd/zg) | 否——缺末端价格 |
| `leftCenter` | 走势末端价脱离 [zd,zg]（离开中枢） | 同上 | 同上；且 kind⊥leftCenter 已证正交 | 否——缺价格 + kind 不可替代 |
| `afterTypeOne` | 当前走势在前序第一类买卖点之后（时序） | 走势序列级前序分类 | 单 Move 无前序上下文 | 否——缺时序 + 递归依赖 brokeCenter |

**三字段共同根因（非三个独立缺口）**：元素层 `Move` 不携带价格几何端点。对比 `SubLevelDescent.lean` 能对 `RMove` 真下钻判 `SubBrokeBelow`/`SubBrokeAbove`（破中枢几何，纯 L0）——**仅因** `RMove`（`Formal.RecursiveConstruction.Move`）携带 `interval`（lo/hi 价格区间）。元素层 `Move` 无 `interval`/`price` 字段。

## 与 629 开口③定性的冲突（核心矛盾）

629 开口③把 `MoveJudgment` oracle 桩定性为单一『**L2 真实数据填充缺口**（still-MISSING-D′），等真实数据』。但本号分析揭示分层：

- **价格几何分量（brokeCenter/leftCenter）= L0 结构**：判定数据 = `Segment.startPrice/endPrice`，这是管线在 `strokesOf→segmentsOf` 层（已）算出的**真实结构字段**。是 `movesOf : List Segment → List Center → List Move` 构造 `Move` 时**丢弃**了 Segment 的价格端点（canonical `Move` 结构无 price 字段）。补 `Move` 价格字段即可 L0 推导，**不需要等 L2 MACD 引擎**。
- **背驰力度分量（divPair）= 真 L2 缺口**：`Force`（MACD 柱子面积）无任何 Origin 模块从 K 线计算（`Divergence` still-MISSING-C，需 EMA/DIF/DEA + 面积积分）。这才是真正的 L2 缺口。

629 把①②混为一谈（都归 L2）是 **L 级误标（231）**：价格几何是 L0 结构（Segment 层已得），归 L2 会让本可结构消桩的分量被错误推迟到 MACD 引擎之后。

## 涉及的定义

- `formal/Origin/ChanlunElements.lean:65-70`（`Move` 无 startPrice/endPrice）+ `:110-111`（`movesOf` 丢价格、`bspOf:List Move→List Bsp` 契约信息不足）—— canonical 载体，P1 owner。
- `formal/Origin/BspClassification.lean:75-84`（`BspEndpoint` 字段语义）+ §10.1 三类判据。
- `formal/Origin/CenterStates.lean`（608，`IsBelow`/`IsAbove`/`classifyPosition`——brokeCenter/leftCenter 判据基准，缺的只是输入价格 p）。
- `formal/Origin/SubLevelDescent.lean:157-178`（对照：`SubBrokeBelow m c:=m.lo<c.zd`，RMove 携带 interval ⟹ 破中枢 L0 可判）。
- `formal/Origin/Divergence.lean` still-MISSING-C（背驰力度 = 真 L2 缺口）。

## 谱系比对结果

- **629 开口③**（先例）：把 MoveJudgment oracle 桩统一归 L2 真实数据填充。本号**精确化**：分离 L0 结构缺口（价格几何）与 L2 数据缺口（背驰力度），纠正 L 级。本号不否定 629 有开口，是其下游精确化。
- **231**（已结算，母规则）：L 级标注。本号是 231 的应用——纠正 629 的 L 级误标。
- **ClassificationGuardrail**（已形式化）：`kind`(twoCenters) ⊥ `leftCenter` 正交性——封闭"用 kind 推 leftCenter"路径。

## 需要决断的问题（领域语言）

`bspOf`（从走势列表识别买卖点）这一 canonical 契约要求只从 `走势(Move)` 列表产买卖点，但**走势结构丢弃了线段(Segment)的价格端点**，致下游拿不到"走势末端价相对中枢的位置"——而这正是判定第一类（破中枢背驰）、第三类（离开中枢回抽）所必需的。请编排者裁定 canonical 契约的修复方向（三选一，均为价值判断）：

1. **给走势(Move)结构增加价格端点字段**（`movesOf` 从线段端点透传）：则 brokeCenter/leftCenter 可用中枢位置三态（608）L0 结构推导消桩，仅背驰力度（divPair）留 L2 开口（承接 629 与 still-MISSING-C）。
2. **改 bspOf 签名为接价格载体**（如 `List Segment → … → List Bsp`），不改走势结构。
3. **接受 MoveJudgment 由外部 oracle 注入**（629 现状的契约化）：承认 bspOf 从走势单独不可 L0 实装，oracle 是契约性外部化（非桩）——但零判据冒充绑定的声明膨胀须删除（本工位已诚实化）。

附带问题：brokeCenter/leftCenter（L0 结构可得）该不该**等** divPair 力度（L2 引擎）一起填，还是先 L0 消价格几何分量？

## 影响声明

本工位在 owner 文件 `formal/Origin/BspConstruction.lean` 做诚实化（消声明膨胀，非补丁）：删除 `dummyJudgmentFromMove` 零判据桩 + `bspOfViaPipeline` + 零判据见证（它们冒充"绑定 ElementPipeline.bspOf"，实际零判据恒产空列 = 退化冒充）；保留 `bspOfMoves`（真遍历，judgments 外部注入是诚实的 oracle 接口）+ `MoveJudgment`（诚实的 oracle 接口定义）。canonical 类型契约的修复（改 Move/bspOf 签名）= 选择类，escalate 候选待编排者 /ritual。不修改 settled 谱系、不改 canonical 类型、不擅自补丁、不擅自结算。
