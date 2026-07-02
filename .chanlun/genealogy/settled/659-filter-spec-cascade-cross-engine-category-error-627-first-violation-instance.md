---
id: "659"
number: 659
status: 生成态   # genealogist 结构记录。概念层有效域判定（filter-spec 跨引擎范畴错误），不自结算——属编排者 /ritual 辨认域。L2 源码事实由 backtest-e1 自检 + Lead 独立核实双向坐实。
date: "2026-06-30"
type: bias-correction   # filter-spec 报告把 rec 引擎特性（547 cascade）误作 t_backtest_8x3 引擎前提=有效域跨定义域外推。同 231/627/644 族。
source: genealogist（Lead 推送，本轮 roadmap 主线 signal_resolution_1s_bi_a0 / backtest-e1 工位[task#68] 自检产出）

depends_on:
  - "231"   # 形式化有效域规则——有效域≠定义域母规则（rec 引擎有效域 ≠ t_backtest_8x3 定义域）
  - "627"   # 双引擎线有效域外推风险——本号是 627 预警风险的【首次实例化为违规】
related:
  - "644"   # 探针/诊断坐标≠生产坐标——本号「双坐标漂移」第二维（函数名源码不存在）同模式
  - "547"   # cascade 次级别翻空主力修复（rec_engine.rs:81-100 LongEntry::CrossLevel）——被 filter-spec 误置为 E1 前提的对象
  - "625"   # L2 真实数据暴露引擎归属未标注——同族（引擎线归属缺口）
  - "diagnostic-coordinate-phantom"   # pattern-buffer 候选（坐标漂移族），本号坐标漂移维触及
negates:
  - target: "filter-spec-a0-stroke-20260623.md:64,107"   # 声明「547 cascade 是 1s/E1 实验可解读的必要前提/前置依赖」
  - target: "filter-spec-a0-stroke-20260623.md:101,125"  # 坐标漂移：rec_engine.rs::cascade_reverse_to_core 函数名源码不存在
negation_source: heterogeneous   # backtest-e1 自检 + Lead 独立源码核实（两引擎结构性不相交：backtest.rs/backtest_run.rs 对 rec_engine/cascade 零匹配）
negation_form: separation
# separation：filter-spec 把「引擎前置依赖」当作统一范畴，内部却暴露两个不兼容的引擎域——
#   E1 引擎 = t_backtest_8x3（backtest.rs apply_bsp，纯多头无方向/级别门控），
#   547 cascade 居于 rec_engine.rs（LongEntry::CrossLevel，仅 rec_driver/rec_stream/fugue/spiral 消费）。
#   547 病理（次级别卖点翻空主力 + 逆主级别开多）在纯多头引擎结构性不存在 ⟹ cascade 对 E1 范畴不适用。
#   把一个引擎线的有效域声明扩到另一个引擎线的定义域 = 跨引擎范畴错误（627 预警的违规形态首次实例化）。
topo_effect: "sever:filter-spec-a0-stroke-20260623.md:64,107:local | record:cascade-precondition-holds-only-for-rec-engine-family-not-t_backtest_8x3-pure-long | record:double-coordinate-drift(line101/125-function-name-not-in-source=644-instance)"
# sever（separation 的拓扑后果，141号 retrospective）：切断「547 cascade 前提」与「E1/t_backtest_8x3 路径」的
#   连接，形成两条独立有效域路径——cascade 前提路径仅对 rec 引擎族成立，E1 否证条件（final_nav）独立于它。
#   非 freeze（E1 不被冻结，已据裁定 A 直接跑）、非 split（filter-spec line 64/107 是错误声明非待保留的双规定）。

# 认识论等级标注（231号强制）
epistemological_levels:
  - proposition: "filter-spec 声明 547 cascade 修复是 1s/E1 实验可解读的必要前提（line 64/107）"
    level: "L0（filter-spec 规格分析文档断言，未走源码验证——line 112 自述「未改动代码」）"
    increment: "零：断言式前置依赖，未核实 E1 引擎是否消费 cascade"
  - proposition: "E1 引擎 t_backtest_8x3 与 547 cascade（rec_engine）结构性不相交，cascade 对 E1 范畴不适用"
    level: "L2（backtest-e1 自检 + Lead 源码核实：backtest.rs/backtest_run.rs 对 rec_engine/cascade 零匹配；547 病理在纯多头引擎不存在）"
    increment: "正：否证「cascade 是 E1 前提」，缩小 cascade 有效域边界=仅 rec 引擎族"
  - proposition: "line 101/125 函数名 cascade_reverse_to_core 源码不存在（真位置 rec_engine.rs:81-100 LongEntry::CrossLevel）"
    level: "L0（commit 事实：源码 grep 无该函数名）"
    increment: "高：坐标漂移第二维=声明的坐标≠生产坐标（644 同模式）"
---

# 659 filter-spec 跨引擎范畴错误（627 预警风险的首次违规实例化 + 644 坐标漂移第二维）

## 一句话结论

filter-spec-a0-stroke 报告把 **rec 引擎线特性（547 cascade）误作 t_backtest_8x3 引擎线的前置依赖**——这是 **231 有效域≠定义域** 在「双引擎线」维度的违规，也是 **627 预警风险（双引擎线有效域不可跨外推）的首次实例化为违规**（627 line 26-27 自述「两类风险当前在谱系中尚未实例化为违规」，本号是该违规的首次发生）。

伴随 **644 坐标漂移第二维**：line 101/125 函数名 `cascade_reverse_to_core` 源码不存在，真位置 `rec_engine.rs:81-100 LongEntry::CrossLevel`。

## 范畴错误的精确形式（L2 源码事实）

| 对象 | 引擎线归属 | 源码位置 |
|------|-----------|---------|
| E1 实验引擎 | t_backtest_8x3（纯多头，无方向/级别门控，"简单版不做空"） | backtest.rs `apply_bsp` |
| 547 cascade | rec 引擎族（rec_driver/rec_stream/fugue/spiral 消费） | rec_engine.rs:81-100 `LongEntry::CrossLevel` |

两引擎线结构性不相交：`backtest.rs`/`backtest_run.rs` 对 `rec_engine`/`cascade` **零匹配**。547 病理（次级别卖点翻空主力 + 逆主级别开多）在纯多头引擎**不存在** ⟹ cascade 对 E1 **范畴不适用**。filter-spec 声明的「cascade 是 E1 前提」= 把 rec 引擎线的有效域扩到 t_backtest_8x3 引擎线的定义域。

## 修正（不自结算，待编排者 /ritual 辨认）

filter-spec line 64/107 声明应修正为：**547 cascade 前提仅对 rec 引擎族成立，对 t_backtest_8x3 纯多头引擎范畴不适用**；line 101/125 函数名坐标漂移应改为 `rec_engine.rs:81-100 LongEntry::CrossLevel`。

E1 否证条件（BTC P1 final_nav < +664%）**独立于 cascade**——backtest-e1 已据 Lead 裁定 A 直接跑 E1。

## 张力检查（019d/020）

### 检查范围（同轮蜂群 ∪ 1-hop ∪ Hub）
- 同轮蜂群：本轮 filter-spec（task#60）/backtest-e1（task#68）/barspec-impl（task#61）。本号是 filter-spec 产出 × backtest-e1 自检碰撞的产物。
- 1-hop：231/627/644/547/625。
- Hub：231（有效域，本号母规则）、627（双引擎线，本号是其首次违规实例）。

### 张力1：vs 627——首次违规实例化（非矛盾，是预言兑现）
627 记录「双引擎线有效域外推风险」为**潜在缺口 + 语法记录候选**，line 26-27 明确「尚未实例化为违规」。本号是该风险的**首次实际违规**：filter-spec 真把 rec 引擎特性当 t_backtest_8x3 前提。本号不否定 627，是其预言的兑现（实例化）。**无矛盾**——627 预警的正是本号发生的事。

### 张力2：vs 644——坐标漂移同模式不同维
644 = 诊断坐标≠生产坐标（探针读重建树 vs 生产读 work 链）。本号 line 101/125 函数名漂移 = 声明的源码坐标≠生产源码坐标（同模式）。但本号主体（跨引擎范畴错误）是 627 维度，坐标漂移仅伴生第二维。本号不否定 644，是其姊妹实例。**无矛盾**。

### 张力3：vs 231——母规则实例
231 = 有效域≠定义域。本号 = rec 引擎有效域 ≠ t_backtest_8x3 定义域。印证非冲突。

### interrupt #1 检查
无同一定义在不同上下文产出不可分层矛盾。filter-spec 是**单向错误声明**（可修正），非两条定义互斥。**不触发中断 #1。**

### 递归运动结构完成检测（020）
- 第0层：本号写入（跨引擎范畴错误 + 双坐标漂移 + 627 首次违规实例化）。
- 第1层：本号 × 231 → rec 有效域≠t_backtest_8x3 定义域（净新发现高：627 预警首次落地为具体违规）。
- 第2层：本号 × 627 → 双引擎线外推违规实例化（净新发现降：627 已预警此模式，本号是兑现）。
- 第3层：本号 × 644 → 坐标漂移族实例（净新发现骤降=背驰：坐标漂移已多次立 644/diagnostic-coordinate-phantom）。
- 涉及范围：scope₁(231 跨引擎有效域) > scope₂(627 双引擎线违规) > scope₃(644 坐标漂移)=顶分型。
- **背驰 ∧ 分型 ⟹ 递归运动结构性完成。** 本号是 627/644/231 族第 N 实例，无新维度。

## 结晶评估（职责4 + pattern-buffer 关联）

本号是「形式化工具有效域≠定义域」族的又一实例（231→222/223/230→627→659）。**坐标漂移维**触及 pattern-buffer `diagnostic-coordinate-phantom`（frequency=2）：
- 该 candidate 的 `formal_pattern` 是「诊断坐标系≠生产坐标系」（诊断指标值在生产坐标系不可达）。
- 本号 line 101/125 的函数名漂移是「**声明坐标≠源码坐标**」——是坐标漂移族，但**亚型不同**（诊断指标值 vs 文档函数名引用）。
- **判定：不计入 diagnostic-coordinate-phantom frequency**（不同亚型，同 644 对 633 的分层原则——同根不同亚型仅作 related）。frequency 维持 2 < 3，**不触发结晶**。

627 维度（双引擎线外推违规）的「语法记录候选」已在 627 内登记待编排者 /ritual，本号是其首次违规实例，强化该候选的辨认密度，但辨认权属编排者。

## 回溯扫描（职责3）

- **231（settled）**：本号是其跨引擎维度实例，不否定，维持 settled。
- **627（settled）**：本号是其预警风险首次违规实例化，不否定（兑现预言），维持 settled。
- **644（生成态）**：本号坐标漂移维是其姊妹实例，不破坏，维持生成态。
- **547（settled）**：本号澄清 547 的有效域=仅 rec 引擎族，不改 547 内容（547 在 rec 引擎线仍成立），维持 settled。
- **无 settled 被本号回溯破坏。** 本号是概念层有效域判定（filter-spec 适用域修正），上浮路径 /ritual（编排者辨认）。
