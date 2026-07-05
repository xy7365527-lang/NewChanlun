---
id: 694
number: 694   # 候选编号（当前最大占用=693 策略对象三分冻结，同 goal 姊妹轴）。最终编号 /ritual 统一分配。
title: "acc-highlevel-mu（稀疏高级别买卖点直接估 μ̂）被 acc-highlow-power（高级别定方向 + 低级别出执行样本，区间套统计版）扬弃承接——高级别 alpha 检验节点分裂为『方向/容器角色』⊥『执行/样本源』两正交层"
status: 生成态   # goal SUPERSEDE（events.jsonl line 293）携带的方法论扬弃落谱系；转 settled 待编排者 /ritual 追认 + acc-highlow-power 实装落地回溯结算。693 覆盖同一 goal 的对象冻结轴，本号覆盖 estimand/样本源轴，二者姊妹非重复。a3（highlow-a3-20260704.md）已闭合 pending_verification 实测半（无稳健增量→回 645 分支），见文末「a3 实测闭合注记」。
date: 2026-07-04
type: bias-correction   # 订正 fullhist-multilevel goal 的 acc-highlevel-mu 隐含前提「高级别买卖点可在其自身稀疏样本上直接估 μ̂ 做 alpha 检验」——被证伪为稀疏无功效（Le Cam 硬墙），估计对象须重构。
source: "[新缠论] docs/formal-chain/问题1.pdf（16页《推导完全分类》终局裁定，INDEX.md 权威链）+ 编排者令 2026-07-04（『问题1.pdf 作为 goal 严格执行』）+ goal SUPERSEDE 事件 .chanlun/goals/events.jsonl:293（old=g-20260630T2010Z-fullhist-multilevel-3ecaf6 / new=g-20260704T1756Z-strategy-object-freeze-cbc36d，reason 明文『acc-highlevel-mu 被 acc-highlow-power 扬弃承接』）；被扬弃对象出处 = old goal acc-highlevel-mu（events.jsonl:250 acc 定义）+ acc-multilevel-sample 自陈 Le Cam 硬墙分支"
negation_source: heterogeneous   # 裁定依据=问题1.pdf 外部形式化推导文档（编排者 INDEX.md 权威链）的直接应用；无外部模型 session 裁量。
negation_model: null   # 同 690 口径：编排者既有权威链裁定（问题1.pdf）的直接应用，非 codex/gemini 模型裁量。
negation_form: aufhebung   # 否定（稀疏高级别自估 μ̂ = 无功效，acc-highlevel-mu 的直接估计路径）+ 保留（高级别买卖点确是编排者关注的 alpha 方向候选，价值不否）+ 提升（分离为『高级别=方向/容器/过滤』⊥『低级别=执行/样本源』双正交层，区间套统计版）。内含 separation：高级别 alpha 检验节点分裂为方向层/样本源层。

# 拓扑效果标注（147号下游推论3：negates 非空必填）
# negates：fullhist-multilevel goal 的 acc-highlevel-mu 隐含前提——「高级别（level2+）买卖点可在其自身逐信号样本上直接估 μ̂ 并据此下 alpha 结论」。
# 实际拓扑后果（retrospective 141号结论1）：该前提被证伪——高级别买卖点结构性稀疏（几个月一次，n_eff≪n_min），
#   直接估 μ̂ 撞 Le Cam 功效硬墙（old goal acc-multilevel-sample 自陈的『level3+ 全历史 <30 ⟹ 结构性稀疏』分支坐实），
#   稀疏样本上『μ̂≤0』不构成否证、只能 INCONCLUSIVE（660/661 第三态）。
#   「高级别 alpha 检验」节点分裂：一支保留原意图（高级别 = 方向/容器/过滤角色，alpha 方向价值保留）；
#   一支携带违反记录（高级别自估 μ̂ = 无功效 Le Cam 硬墙，被否）。样本源边 re-anchor 到低级别执行层
#   （区间套统计版：高级别定方向 + 低级别提供执行与样本量 + 各桶报 n_eff/功效门对照）。
topo_effect: "split:acc-highlevel-mu-direct-sparse-estimation:downstream"
# split：高级别 alpha 检验节点分裂为『方向/容器角色（高级别）』⊥『执行/样本源（低级别）』两正交层；
#   稀疏高级别自估 μ̂ 分支携带违反记录（Le Cam 无功效）；样本源 re-anchor 低级别。
#   scope=downstream：影响 new goal acc-highlow-power 实装 + acc-tristate-rerun 复验口径（n_eff 不足桶只准 INCONCLUSIVE）。
depends_on: ["660", "661", "231"]
related: ["693", "663", "665", "645", "690", "647"]

# 矛盾（bias-correction 的被订正对象）
contradiction:
  description: |
    old goal g-...-fullhist-multilevel-3ecaf6 的验收 acc-highlevel-mu 直接要求：
    「高级别（level2+）买卖点逐信号 μ̂：全历史下 level2-4 各类 μ̂>0？train-only 挑类 +
    holdout（防选择偏差 663/665）；报高级别 holdout μ̂ 的 LCB>0 占比」——即在高级别买卖点
    **自身样本**上直接估 μ̂ 并据 LCB 下 alpha 结论。同 goal 的 acc-multilevel-sample 已自陈
    悬念：「若 level3+ 全历史仍 <30 → 高级别结构性稀疏（非窗口问题），确认 Le Cam 硬墙」。
    问题1.pdf《推导完全分类》裁定：高级别买卖点结构性稀疏（几个月一次），其自身样本 n_eff≪n_min，
    直接估 μ̂ 撞功效硬墙——稀疏样本上『μ̂≤0』既非 VALIDATED 也非 FALSIFIED，只能 INCONCLUSIVE
    （660/661 已结算：inconclusive 第三态 = 伪负 / 无功效，非真无信号）。故 acc-highlevel-mu
    的『直接估稀疏高级别 μ̂』路径无功效、不可下 alpha 结论——须重构估计对象。
  layer: 概念   # goal 验收方法论的 estimand 重构（估计对象/样本源）。非代码 bug：old goal 已 CLOSED（events.jsonl:267），本号订正的是被 SUPERSEDE 承接的验收前提，new goal acc-highlow-power 是承接实装。
  trigger: "编排者 2026-07-04 令问题1.pdf 作 goal 严格执行 → goal SUPERSEDE（old fullhist-multilevel / new strategy-object-freeze-cbc36d）reason 明文『acc-highlevel-mu 被 acc-highlow-power 扬弃承接』 → genealogist 检查扬弃是否携带概念增量 → 坐实（estimand/样本源轴，693 未覆盖）立本条。"

# 扬弃三环节（negation_form: aufhebung 必记）
aufhebung:
  negated: "acc-highlevel-mu 的『高级别买卖点在其自身稀疏样本上直接估 μ̂ + LCB 下 alpha 结论』路径——稀疏 n_eff≪n_min 撞 Le Cam 功效硬墙，稀疏样本 μ̂≤0 不构成否证（660/661 INCONCLUSIVE）。"
  preserved: "高级别买卖点确是编排者关注的真 alpha 方向候选（old goal 立意保留）——高级别的方向/容器/过滤价值不被否定，被否的只是『在其自身样本上直接估 μ̂』这一估计路径。"
  elevated: "acc-highlow-power（区间套统计版）：高级别桶只作方向/容器/过滤，低级别提供执行与样本量；输出报各桶 n_eff 与功效门对照。高级别 alpha 检验空间分离为『方向层（高级别）』⊥『样本源层（低级别执行）』两正交层——estimand 从『高级别自身 μ̂』重构为『高级别条件下低级别执行的 μ̂』。"

# 概念分离（aufhebung 内含）
separation:
  before: "『高级别 alpha 检验』被当作单一操作：在高级别买卖点自身样本上估 μ̂（acc-highlevel-mu）。方向判定与样本来源焊死在同一级别。"
  after:
    - name: "方向/容器/过滤层（高级别，level2+）"
      definition: "高级别买卖点提供交易方向、区间容器与过滤门——不承担样本量责任。高级别稀疏（几个月一次）在此层是特征非缺陷（它就该稀疏地定大方向）。"
      source: "问题1.pdf 高级别定方向裁定 + old goal 立意（高级别是编排者关注的 alpha 方向候选）。"
    - name: "执行/样本源层（低级别）"
      definition: "低级别在高级别方向条件下提供执行触发与统计样本量，μ̂ 在此层估计（n_eff 充足）。区间套统计版：μ̂(高级别方向条件下的低级别执行)。"
      source: "new goal acc-highlow-power（events.jsonl:292）：『高级别桶只作方向/容器/过滤，低级别提供执行与样本量；报各桶 n_eff 与功效门对照』。"
  pending_verification: "acc-highlow-power 实装后，低级别执行样本在高级别方向条件下的 μ̂ 是否 n_eff 达功效门且 LCB>0——若达标且方向条件真收窄有效，扬弃成立并回溯结算；若低级别执行 μ̂ 亦无稳健增量，则高级别 alpha 假设本身受质疑（回 645 wrong-object 检查）。【a3 已闭合：实测无稳健增量 → 进 645 分支，见文末注记】"

# 涉及的定义
definitions_involved:
  - name: "acc-highlevel-mu（被扬弃的 old goal 验收）"
    version: "g-20260630T2010Z-fullhist-multilevel-3ecaf6 / acc-highlevel-mu（events.jsonl:250）"
    role: "被否定的估计路径。直接在稀疏高级别样本估 μ̂——撞 Le Cam 功效墙。"
  - name: "acc-highlow-power（承接的 new goal 验收）"
    version: "g-20260704T1756Z-strategy-object-freeze-cbc36d / acc-highlow-power（events.jsonl:292）"
    role: "扬弃承接。高级别定方向 + 低级别出样本，区间套统计版，报 n_eff/功效门。"
  - name: "660/661 inconclusive 第三态（功效门 / 伪负 vs 真无信号）"
    version: ".chanlun/genealogy/settled/660、661"
    role: "约束来源。稀疏高级别 μ̂≤0 = INCONCLUSIVE（无功效伪负），非 FALSIFIED——直接估计路径不可下 alpha 结论的判据依据。"
  - name: "231 形式化有效域规则"
    version: ".claude/rules/formalization-validity-domain.md（settled）"
    role: "母规则。样本 n_eff 不足 ⟹ 有效域收缩至 INCONCLUSIVE，L2 实测结论不可外推。"

# 结算状态
settlement_status: |
  概念层扬弃已辨认（estimand/样本源轴，693 未覆盖，二者同 goal 姊妹轴）。
  转 settled 待：(1) 编排者 /ritual 追认（同 693/690 口径）；
  (2) new goal acc-highlow-power 实装落地 + 低级别执行 μ̂ 的 n_eff/功效门实测（pending_verification 闭合后回溯结算）。
  【2026-07-04 更新】条件(2)已由 a3（highlow-a3-20260704.md）闭合——实测无稳健增量，进入 645 wrong-object
  分支（见文末注记）。条件(1)编排者 /ritual 追认仍待。故本号维持生成态：a3 的实测结果闭合了「验证半」，
  但扬弃的干净成立被证伪（低级别执行 μ̂ 亦无增量 ⟹ 高级别 alpha 假设本身受质疑），须待编排者对
  「回 645 wrong-object 检查」这一新开分支的处置 + /ritual 追认后方可结算。

# 与 693 的关系（同 goal 双轴，非重复）
sibling_note: |
  693（策略对象三分冻结 + A11/A10 硬裁决）覆盖本 goal 的 acc-target-freeze 轴——
  「被检验对象在裁剪版/完整体间漂移」的对象冻结（Π_tested⊊Π_full 有效域纪律）。
  本号（694）覆盖 acc-highlow-power 轴——「高级别自估 μ̂ 无功效」的 estimand/样本源重构。
  两轴同源于问题1.pdf 但否定不同对象：693 否定「不冻结对象即下 alpha 结论」；
  694 否定「稀疏高级别自身样本可直接估 μ̂」。姊妹条目，互不蕴含。
---

# 694号：acc-highlevel-mu 被 acc-highlow-power 扬弃承接（高级别 alpha 检验分裂为方向层 ⊥ 样本源层）

## 一句话结论

编排者 2026-07-04 令问题1.pdf 作 goal 严格执行，触发 goal SUPERSEDE。除 693 记录的
**策略对象冻结轴**外，SUPERSEDE reason 明文点名第二半扬弃：old goal 的 `acc-highlevel-mu`
（在稀疏高级别买卖点自身样本上直接估 μ̂）被 `acc-highlow-power`（高级别定方向 + 低级别出
执行样本，区间套统计版）**扬弃承接**。本号锚定该方法论扬弃的出处链与三环节，693 未覆盖。

## 为何需要独立立号（而非并入 693）

693 覆盖 `acc-target-freeze`（对象三分冻结 + A11/A10 硬裁决 + 有效域定理），否定的是
「不冻结被检验对象即下 alpha 结论」。本号覆盖 `acc-highlow-power`，否定的是「高级别买卖点
可在其自身稀疏样本上直接估 μ̂」——**不同的被否对象、不同的拓扑效果**（693=对象冻结轴的
validity-domain；694=estimand/样本源轴的功效墙重构）。SUPERSEDE reason 两半分别对应两号，
姊妹非重复。

## 扬弃三环节

- **否定**：稀疏高级别自估 μ̂（n_eff≪n_min，Le Cam 功效硬墙；μ̂≤0 只能 INCONCLUSIVE，见 660/661）。
- **保留**：高级别买卖点作为 alpha 方向候选的价值（编排者立意不否）。
- **提升**：分离为「方向/容器/过滤层（高级别）」⊥「执行/样本源层（低级别）」两正交层；
  estimand 从「高级别自身 μ̂」重构为「高级别方向条件下低级别执行的 μ̂」（区间套统计版）。

## 结算路径

概念层扬弃已辨认。待编排者 /ritual 追认 + acc-highlow-power 实装落地（低级别执行 μ̂ 的
n_eff/功效门实测）后回溯结算。若低级别执行 μ̂ 亦无稳健增量 → 回 645 wrong-object 检查。

## settle-sweep 增量扫描交叉引用注记（2026-07-04，问题G）

`formal-chain-deepresearch-20260704.md` §3「问题1.pdf 十核实点判定表」#7（本号对应"问题G"）
判定为 **GAP_CONFIRMED（三分定性）**："高级别定方向+低级别执行统计"：交易执行层已有等价物
（`route_bsp` top-down 方向传导）；**alpha 统计层为全新缺口**（三套桶键 level 全是独立分层维，
无跨级 pooling）；σ^H 半个零件已在（塔真值填第 9 维）但被 codex-q1 G2 裁定不进桶键。落地成本
低于从零，需新 prereg（i_class×δ 共线教训在案）。

**交叉核对结论**：deep-research 的判定与本条目的核心主张**一致且互补**——本条目断言
「acc-highlevel-mu 直接估计路径无功效，须重构为高级别定方向+低级别样本源」，deep-research
独立核实了该重构在**实装层的现状分解**：执行层（route_bsp 方向传导）已具备等价机制，但
**alpha 统计层本身是全新缺口**（本号 pending_verification 中"acc-highlow-power 实装后...
n_eff/功效门实测"正是指向这个统计层缺口的闭合）。deep-research 额外补充的细节
（σ^H 已算但 codex-q1 G2 裁定不进桶键、需新 prereg 避免 i_class×δ 共线教训重演）是本条目
`pending_verification` 尚未纳入的**实装约束**，应在 acc-highlow-power 实装工位启动时一并
纳入设计，防止重蹈 657 号（i_class×δ 共线置换自毁）的覆辙。

**本条目状态不变**（仍待编排者 /ritual 追认 + 实装落地回溯结算），本注记补充：
acc-highlow-power 的实装范围须显式排除"σ^H 直接进桶键"这一路径（已被 codex-q1 G2 裁定
排除），改走"高级别方向条件"的过滤/分层机制，避免共线退化。

## a3 实测闭合注记（2026-07-04，pending_verification 验证半闭合）

`highlow-a3-20260704.md`（工位 swarm/ws-highlow，L2 BTC 全历史 wf OOS）已按本号 elevated 环节
（「高级别方向条件下低级别执行的 μ̂」）实装 acc-highlow-power 并跑数，**闭合了本号
`pending_verification` 的实测半**。结果指向 pending_verification 的**第二分支**（回 645）：

**实测结论**（无稳健增量，非扬弃干净成立）：
- 估计量 μ̂(z_L0 | D_hi) 已形式化+实装+跑数（区间套统计版字面落地）。L0 执行样本 2022，高级别 234
  （弃估计只作条件源，694 口径兑现）。
- **D1 σ^H_tower（相邻上级 L1 方向条件）**：条件化改变 2 格判定，但**未产生任何非 beta-drift 的
  Validated 格**。基线唯一 Validated 桶（L0 bsp3 δ+1 σ_p+1，已知 beta 漂移伪结构）的正残差**完全
  集中在 σ^H=+1 条件**（上级方向净涨时），且同条件反 δ 卖格 mean 同正 ⟹ 条件化把 beta 漂移解释
  **定位到高级别方向载体**，不构成方向 alpha；σ^H=−1 条件下退为 Inconclusive。
- **D2 nest_anchor（区间套锚）**：维退化坐实——π fill loop 样本链 nest_depth 全 None（anchored=0），
  D2 在本样本链**无生产者**，不判定（预声明兑现，与「区间套已接入但 95% 退化」memory 同族）。
- **停机条款**：非 beta-drift-suspect 的 Validated 条件格 = 无 → 不触发终局翻转。

**对本号结算路径的影响**：本号 `pending_verification` 明文分两支——「若达标且方向条件真收窄有效
⟹ 扬弃成立并回溯结算；**若低级别执行 μ̂ 亦无稳健增量，则高级别 alpha 假设本身受质疑（回 645
wrong-object 检查）**」。a3 实测坐实**第二分支**：低级别执行在高级别方向条件下**无稳健增量**（唯一
候选是 beta 漂移伪结构的条件化再定位）。故：

1. **实测半闭合**（条件(2)达成）：acc-highlow-power 已实装落地并产出 n_eff/功效门实测。
2. **但扬弃未干净成立**：a3 结果是 pending_verification 的「否定分支」——高级别 alpha 假设本身受质疑，
   进入 **645 wrong-object 检查**分支。这不是本号 elevated 环节（方向层⊥样本源层分离）的失败——分离
   在方法论上成立且已实装；失败的是「高级别方向条件下低级别执行存在可交易方向 alpha」这一**经验假设**。
3. **本号维持生成态**：a3 闭合了实测半，但 (a) 编排者 /ritual 追认仍待；(b) 645 wrong-object 新分支
   已开启（高级别 alpha 假设受质疑）——扬弃从「acc-highlevel-mu 被 acc-highlow-power **成功**承接」
   收窄为「被承接但承接后**亦无增量**，触发对高级别 alpha 方向假设本身的 wrong-object 复查」。结算须
   待该 645 分支处置 + /ritual 追认。

**有效域声明（231）**：a3 的有效域 = **相邻上级方向条件（L0 样本的 D1=L1 方向态）**，σ^H 生产者定义域
是执行级相邻上一级（tower[level+1]）。「更高级别（L2+）方向条件」无逐笔生产者——**不外推**到跨级方向
条件。本号 elevated 环节所述「高级别定方向」在 a3 实测中被操作化为「相邻上级方向」，跨级方向条件的
alpha 检验仍是有效域外的未测区（a3 边界条件(b)：若 σ^H 生产者扩展到跨级方向且高阶切片出现非 suspect
Validated ⟹ 有效域外推成立，届时须重估本号结算方向）。

**下游谱系联动**：本注记开启对 `645`（wrong-object）的实测触发——终局 q4/final-alpha INCONCLUSIVE
判定在 a3 后**不变**（a3 未产出可翻转终局的非 suspect Validated 格）。acc-highlow-power 轴在现有
样本链（相邻上级 + π fill loop）上**未翻转无方向 alpha 判定**，其信息增量是把基线唯一 Validated 桶的
正残差**进一步定位**为「σ^H=+1 条件承载的 beta 漂移」（oddeven 判读的条件化加强版）。

## M8 终局佐证注记（2026-07-04，645 wrong-object 分支根因坐实，本号维持生成态）

**追加人**：genealogist（本工位）。**依据**：`maxfull-e2e-round1-20260704.md`（M8 终报告，
@ af8910d062）§2.2 因果链解读 + §1 一行结论。**本小节不改上文既有内容，仅补 a3 开启的 645
分支在 M8 全链终局下的根因佐证。**

### a3 的 645 分支在 M8 全链下获得根因佐证

上文 a3 实测闭合注记把本号推入 **645 wrong-object 分支**（高级别 alpha 假设本身受质疑）。M8 端到端
首轮 OOS 终报告为该分支提供了**全链根因佐证**——不是新的翻转，而是把「高级别方向条件下低级别执行
无增量」放进整条 signal→execution→treasury→total 因果链的最上游：

- M8 §2.2 因果链：**「signal 无方向 alpha（双门不过，唯一正桶=beta 漂移）」是全链四层 fail 的
  单一根因**，向下传导为 execution net_r 负 → treasury 停 Stage I → total LCB<0。
- a3 的实测结论（高级别方向条件下低级别执行的唯一正残差 = σ^H=+1 承载的 beta 漂移）**正是这条
  根因在「高低配 estimand」维度的一个切面**：无论是 signal-full 主裁决的 25 桶双门，还是 a3 的
  高低配条件化，唯一的正 μ̂ 都收敛到同一个 beta 漂移伪结构（memory `oddeven_mu_identity`），
  非可交易方向 alpha。
- 故 645 wrong-object 分支的方向获得佐证：被质疑的「高级别 alpha 方向假设」在 M8 全链下坐实为
  「signal 层无 confirmed 方向 alpha」的一个观测点，**不是 estimand 重构（本号 elevated 环节）
  的失败**，而是 signal 层假设本身在 BTC/L2 上无 alpha 的下游必然。

### 结算判定：维持生成态（645 分支处置 + /ritual 追认双待）

**判定 = 维持生成态。** M8 佐证**强化**了 645 分支的方向（wrong-object 复查指向 signal 假设本身），
但**不清除**本号的两个结算前置：

1. **645 wrong-object 分支的处置是选择类**：M8 §6 明列三方向候选（跨标的 L3 / A股 K4 / 新信号
   假设）为「**互斥性由编排者判断，四分法=选择类，不自决**」。645 分支的下一步（是否换新信号
   假设复查高级别 alpha 方向、还是判定高级别方向假设在 BTC/L2 上已被否）正落在这三候选的选择域内，
   genealogist 无权自决。M8 全链佐证提供的是**证据**（signal 无 alpha 是根因），不是**处置裁决**。
2. **编排者 /ritual 追认仍待**：同 693 口径，本号 aufhebung 落谱系需 /ritual 追认。
3. **有效域纪律（231）**：M8 佐证的有效域 = BTC/L2 单标的。a3 边界条件(b) 未被 M8 触及——跨级
   方向条件（L2+）的 alpha 检验、跨标的（L3）signal alpha 仍是未测区（M8 §6 候选1 明列 L3 扩容），
   故「高级别方向假设已被否」这一强结论**不成立**，只成立「BTC/L2/相邻上级方向条件下无 confirmed
   方向 alpha」。645 分支不因 M8 而关闭，仅方向获佐证。

**结论**：本号维持生成态。剩余待裁项 = (a) 编排者对 645 wrong-object 分支的处置选择（M8 §6 三方向
候选之一）；(b) 编排者 /ritual 追认本号 aufhebung。M8 全链根因佐证已记录在案，二者裁定后可回溯结算。
