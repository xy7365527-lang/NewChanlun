# Session — 吃跌探索 + 买卖点纠正

**时间**: 2026-06-22（连续 session d41059a8，吃跌主线）
**分支**: main，HEAD a080c7c2e4

## 死锁主线状态
- **吃涨已解**：anchor（HOLD_ANCHOR 死扣多）强牛 5/8 解踏空（552，BTC +1136.9/OKLO +458.9 超 BH），不等完成。
- **吃跌探索 5 次全 L3 否证**（谱系 553-556）：
  1. B 路递归 anchor 全程持空 → 强牛失血（552§八）
  2. cascade flip 粗识别 → 强牛误翻空穿仓（553）
  3. A 路 v1 区间套（检测对象错 level_trends 取各级别全局走势）→ 链坍缩 artifact 作废（编排者"区间套递归保证"纠正，555）
  4. A 路 c 段钻取（修检测对象）→ 链贯通 0→数百，但消费级别⊥贯通级别（554/555）
  5. 读法 B 每级别独立腿 → 顶层腿冻结穿仓（556）

## 统一根因（554/555/556）
**最高级别走势完成在样本期结构性稀疏（顶层跨年）**。所有吃跌机制都用 **type1（背驰=d_top/completed）** 作顶层翻空判据 → 最高级别 type1 顶背驰稀疏 → 顶层稀疏失败。与 pcf 坍缩/σ冻结同构。

## ★ 编排者纠正（2026-06-22，当前方向）
**"不是方向，是买卖点。"** 我两层偏移：(a) 顶层只用 type1（背驰，稀疏）；(b) 然后错误转向"方向跟随"（中枢移动方向状态）。
- 正确：顶层做空用最高级别走势类型的**买卖点**（type1/2/3 卖点），区间套定位，**不是方向跟随**。
- **关键发现**：引擎已有 type1/2/3（types.rs:177 BSPKind + operator.rs:44 detect_type3）。**type3 卖点（中枢下破回抽=下跌趋势确认）从没用于顶层做空，比 type1 频繁**（下跌趋势每中枢下破一个 type3 vs 只顶部一个 type1）。
- 编排者问题2（穿仓根因）：读法B删 sink/recover → 顶层空头死扛次级别反弹失血穿仓。recover=次级别买点平空做多对冲，必须保留。

## 中断点
- 当前：买卖点区间套理解 Workflow（w0r94jyx8）——type3 能否解顶层 type1 稀疏 + 区间套定位 type3 + 当前顶层为何只用 type1。
- 下一步：实装顶层用 type1/2/3 卖点（尤其 type3 中枢下破）区间套做空 + sink/recover 对冲（防穿仓）。四路 L3（OFF/ANCHOR/type1-only/type1-2-3）。

## worktree（未入主树，flag OFF bit-exact）
- agent-ad3eb3b39639b8306：anchor(HOLD_ANCHOR)+cascade(T_CASCADE_FLIP)+547。
- wf_5c5...（a-route-cseg-drill-fix 5c5cebdbb5）：A路c段钻取区间套 d_top（nest_chain_complete）。
- wf_844...（读法B 否证，per级别独立腿）。
- 主树 a080c7c2e4：谱系549-556 + design文档(docs/)，代码未入主树。

## 谱系
549-556 全 settled，pending 0。memory: project_deadlock_dual_open_target / feedback_filter_bank_metaphor_prove。
