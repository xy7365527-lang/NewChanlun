# Cand^δ 修复段2：定律一下沉锚定（Type2/3 定位走次级别 Type1 区间套）——结果包

- task: #13（编排者 2026-07-02 两次明令「必装 + 一定要严格」）
- owner: ws-cand-nest
- date: 2026-07-02
- 认识论等级: 实装 L0（结构下钻 + 确定性 div_cand）+ 单元验证 L1（合成塔，逐条可证伪）；
  L2 全历史 depth/小转大分布 = 待 release 运行（同段1 由 Lead 跑，见 §3.4 方法学）
- 谱系: 673号（Cand^δ_ℓ 范围误用，段2 是其修复方案第二段）、606号（区间套有效域=Type1）、
  定律一（第17课L66）、区间套定理（第27课L43，背驰段含盘整背驰第27课L21）
- 基线: HEAD=78cb6940b7（段1 已 commit）；本 diff 相对该基线，唯一交付文件 econ_positive.rs

## 1. 结论

`build_nest_certificate`（econ_positive.rs）新增**定律一下沉锚定**：
- **Type1**：本级趋势背驰段 `div_cand`，逐字不动（bit-exact，区间套原文对象，606号有效域）。
- **Type2/Type3 @ lvl≥1**：精确定位**锚次级别 Type1**——从候选段 `s`（=回抽次级别走势，
  `end_index==source_index`）下钻 `s.sub_moves`（次级别 ℓ-1），找 `end_index==source_index`
  的段跑**完整 div_cand**（Extreme+Weak，含盘整背驰）。锚点成立后**真递归下沉**：继续钻入该
  次级别 Type1 段，逐级收缩到最低可用级别（`sub_moves` 空=递归底 level0），返回下沉深度 d≥1。
- **小转大**（次级别无 Type1 锚点：div_cand 假 / 无回抽端点对齐段）：证书 `None` ⟹ 门直接拒
  （显式可测判别，非 catch-all fallback）。
- **Type2/3 @ lvl==0**（最低可用级别，无次级别）：下沉不适用，保留段1 存在性免门（Type3@level0
  不误拒）。

新函数 `descend_type1_anchor_depth(s, source_index, delta, hist) -> Option<usize>`：
`Some(d)`=次级别 Type1 锚定成立且下沉深度 d；`None`=小转大。门控前置于 `build_nest_certificate`
入口（`!is_type1 && lvl>=1 && anchor.is_none() ⟹ return None`）。

## 2. 定义依据

- **定律一下沉**（第29课L396，逐字）："所有买点，归根结底都是第一类买点，要找第二、三类，
  **其精确的，都要下次级别以下找第一类**。" ⟹ Type2/3@lvl 的精确点 = 次级别(lvl-1) Type1。
- **区间套用于第二买点**（第27课L60）："600685 第二买点的确认方法（区间套）"。
- **区间套定理**（第27课L43）："某大级别的转折点，可以通过不同级别背驰段的逐级收缩范围而确定。"
  ⟹ 真递归下沉逐级收缩（严格性要求1，不只下沉一级）。
- **背驰段含盘整背驰**（第27课L21）："某级别的某类型走势，如果构成**背驰或盘整背驰**，就把这段
  走势类型称为某级别的背驰段。" ⟹ 次级别锚点用完整 div_cand（Extreme+Weak，涵盖趋势/盘整，
  严格性要求2，非弱化版）。
- **小转大**（缠论知识库.md L410）："小转大是用区间套和背驰不可解释的情况的一种补充；小转大
  没有标准的一类买卖点。" ⟹ 次级别无 Type1 锚点 = 小转大 ⟹ 诚实 None（严格性要求3，显式可测）。
- **下钻语义锚**（nest.rs 契约头 `Origin.SubLevelDescent.descend : RMove → List RMove`）：
  本函数用 `LeveledMove.sub_moves`（compose 的逆）下钻，与该契约同向。
- **级别对齐**（codex H2 诊断，econ_positive.rs h2_sample_exclusion_dx 头）：Type2@lvl 由
  level-(lvl+1) parent 产出，`m1/m2` 是 level-lvl 次级别走势，`source_index=m2.end_index`；
  `s`（=build_nest_certificate 找到的 end==src 候选段）正是 m2（回抽走势）。段2 下钻 m2.sub_moves
  = level-(lvl-1)，恰是定律一「下次级别」。

## 3. 验收（可证伪）

### 3.1 单元测试（L1，合成塔，逐条可证伪）— 见 §3.5 运行结果

1. `nest_cert_type2_sublevel_type1_anchor_passes`：Type2@lvl1 的 m2 内部含次级别 Type1 背驰段
   ⟹ `descend_type1_anchor_depth = Some(1)` ⟹ 证书非 None ⟹ n_delta=true。**depth≥1 实例存在**。
2. `nest_cert_type2_no_sublevel_anchor_is_xiaozhuandaa_none`：同结构但 hist=0（次级别无一类背驰）
   ⟹ `descend = None` ⟹ 证书 None ⟹ 门拒。**小转大判别可证伪**。
3. `descend_anchor_recurses_below_one_level`：次级别 Type1 内再含次次级别 Type1（end 同为
   source_index）⟹ `descend = Some(2)`。**真递归下沉证据（不止一级）**。

### 3.2 Type1 路径 bit-exact（等价守卫）

- `div_cand` 逐字不动；Type1 分支（is_type1=true）实参与调用不变。
- 段1 既有测试全绿：`multilevel_nest_cert_type2_bypasses_divergence_gate`（Type2@lvl0 免门，
  lvl==0 下沉不适用 ⟹ 存在性免门保留 ⟹ n_delta=true 不变）、`..._cand_false_propagates_zero`、
  `..._wrong_direction_returns_false`。**段1 解封语义（Type2@lvl0 免门 + Type1 守门）不破坏**。

### 3.3 全量 cargo test（见 §3.5）

### 3.4 L2 全历史 depth/小转大分布（方法学，待 release 运行）

复用 `h2_sample_exclusion_dx`（econ_positive.rs）收集路径（bit-exact 生产），对 level1-4 每条
Type2 信号计 `descend_type1_anchor_depth`：`None` 计数=小转大剔除数，`Some(d)` 计入 depth 直方图。
锚点正确性抽样：锚到的次级别段 `end_index==source_index` 且方向=−δ（回抽方向）。
修复前（段1）：level1-4 无下沉，depth 概念不存在（全 0）；修复后：depth≥1 实例按 §3.1 存在。
全历史精确计数需 release 二进制（段1 全历史 2314s，由 Lead 运行）——本工位交付 L0+L1，
L2 计数报告移交 Lead/verify 工位（诚实等级标注，不声明未运行的 L2 数字）。

### 3.5 运行结果

隔离环境 /tmp/wcn-1782966380（HEAD=78cb6940b7 段1 基线 + 本 econ_positive.rs 段2 改动；剔除
并发未提交 broken WIP `wverify_run.rs` 以放行编译——该文件引用不存在函数 `data::load_bars_from_cache`
等，是 task#3 工位 WIP，非本交付，非我改动）。

- 段2 三测试 + 段1 三测试全绿：
  `nest_cert_type2_sublevel_type1_anchor_passes ... ok`（depth=Some(1) 实例）
  `nest_cert_type2_no_sublevel_anchor_is_xiaozhuandaa_none ... ok`（小转大 None）
  `descend_anchor_recurses_below_one_level ... ok`（真递归下沉 depth=Some(2)）
  `multilevel_nest_cert_type2_bypasses_divergence_gate ... ok`（段1 Type2@lvl0 免门不破坏）
- econ_positive 模块：24 passed; 0 failed。
- **全量 `cargo test --lib`：1373 passed; 0 failed; 94 ignored**（段1 基线 1369 + 段2 新增 3 测试
  + 无跨库回归；Type1 bit-exact 守卫测试全过）。

## 4. 边界条件（结论翻转条件）

- 若 bsp 分类器对次级别走势结构判定有误（`s.sub_moves` 不是真回抽次级别序列），下沉锚点会锚错
  ——段2 信任塔的 `sub_moves` 拓扑（recursive_tower compose 不变量）。
- 若认为「Type3@level0 也须下沉定位」（严格定律一），则 lvl==0 应 None 而非免门——当前实装
  选择：level0=最低可用级别，无次级别可锚，存在性免门（保段1 level0 Type3 解封；翻转此选择
  ⟹ 全 level0 Type3 被拒，与段1 冲突）。此为 lvl==0 的口径选择，已显式化。
- 真递归下沉的深度上限=塔高（level0 递归底停）；深层无锚不否决锚定，只封顶深度（逐级收缩到
  「最低可用」级别，非「必须到 level0」）。若要求必达 level0 则深度语义翻转。
- 小转大判别依赖 `div_cand` 四条件——若 Θ_MACD 力度代理更换，锚点真值可变（div_cand 边界条件5）。

## 5. 谱系引用

- 673号（生成态）：段2 是「按 bsp 类型分叉」修复方案的第二段（精确定位），接段1（存在性免门）。
- 606号：区间套有效域=Type1——段2 下沉锚定使 Type2/3 经「次级别找第一类」接入区间套原文对象。
- 定律一（第17课L66）/ 区间套定理（第27课L43）：段2 的原文权威。
- vs 段1：无矛盾——段1 装存在性（免本级门），段2 装精确定位（次级别锚+小转大拒），分层正交
  （source-audit L1 存在性 ↔ L4 精确定位）。

## 6. 影响声明

- 改动文件：`rust/src/theta_v0/backtest/econ_positive.rs`（唯一交付）——新增
  `descend_type1_anchor_depth` 函数 + `build_nest_certificate` 入口门控前置 + cand_k 注释更新
  + 3 个段2 回归测试。`div_cand`/`nest.rs`/`cand_predicate.rs` 逐字不动。
- 下游影响：
  - Type2/3@lvl≥1 门后信号集从「存在性全通过」（段1）收窄为「次级别 Type1 锚点成立」；小转大
    被拒。level1-4 sig_post 从段1 的 1473 收窄为「有次级别一类锚点」的子集（L2 精确计数待运行）。
  - Type2/3@lvl==0（level0 Type3）不变（免门保留）。
  - Type1 路径 bit-exact 不动。
- 未自 commit（Lead 唯一 commit 者）。改动在主树 econ_positive.rs（相对 HEAD=78cb6940b7）；
  隔离验证副本 /tmp/wcn-1782966380（已剔除并发 broken WIP wverify_run.rs 以放行编译，非交付）。
