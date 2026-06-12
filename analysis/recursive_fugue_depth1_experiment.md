# 递归赋格深度1实验 — SlotKey 路径键 + LOU 自引用实装 + OKLO O_sub 判决

> 任务（2026-06-11 编排者）：两个工作——(1) SlotKey 路径键 + LOU 自引用递归机制；
> (2) OKLO 深度1实验（O_sub1/O_sub0 预注册判据）。
> 设计来源：`analysis/recursive_fugue_ultimate_design.md`（§2.4 mini-fugue、
> §4 三范畴终止、§5.4 路径裁决——L2 先于全量架构投入，231号）。
> 实验脚本：`analysis/recursive_fugue_depth1_backtest.py`；
> 数据：`analysis/data_cache/recursive_fugue_depth1.json`。

---

## 0. 结论先行

1. **机制已实装且零侵入**：SlotKey 路径键（任意深度）、方向参数化子 LOU
   （符号交替塔 (−1)^n）、账本双向短差（DiffSide）、三范畴终止条件、父腿
   级联闭腿不变式全部落地。`cargo test` 97/97 全过；O0≡P5 四面对账 PASS；
   V2of 在册基线逐位复现（OKLO Δ(vs P5)=+410.4pp）。
2. **OKLO 深度1判决：O_sub0——以"空域"形式成立**。depth=1 与 depth=0 的
   trades 逐位相同（Δ复利 = +0.00pp），因为子反弹腿**一次都没有开出**：
   全程仅 6 次触发步进，5 次"子级别无存活中枢"拒 + 1 次振幅拒。
3. **否证的精确形式是定义域空，不是经济负**。被否证的命题是"V2o 震荡型
   镜像（存活中枢锚定）的反弹腿在 OKLO 上有非空有效域"——设计报告 §5.4
   预注册的否证分支"递归赋格的有效域为空"逐字命中。"反弹腿开出后亏钱"
   这一更强命题未被检验（无样本）。
4. **空域的两级结构根因**（漏斗见 §4）：(a) REV 腿质量集中在存在论地板
   ladder 2（93%），其子级别是 bi 级——存在论终止条件直接排除；(b) 残余
   ladder 3 窗口内，REV 窗口本身就是子级别中枢被向下终结的 regime，
   confirmed Buy1（趋势背驰端点，按定义出现在价格离开中枢之后）与
   "存活中枢"共现概率先验极低——震荡型镜像在反向窗口内结构性失配。

---

## 1. 工作1：递归机制实装（代码摘要）

### 1.1 SlotKey 路径键（`types.rs`）

```rust
pub const MAX_REV_DEPTH: usize = 4;
pub struct RevPath { segs: [u8; MAX_REV_DEPTH], len: u8 }   // 定长保 Copy
pub enum LegClass { Main, Osc, Rev(RevPath) }
```

- `segs[0]` = tranche 确认级别（旧 `Rev(usize)` 同义）；`segs[1..]` = 嵌套
  子 LOU 级别坐标。深度 0 的 `py_key`（+200）/`leg_kind`（"rev"）逐位不变
  ——在册对账面零接触。
- 深度 ≥1 投影显式扩区：`py_key = ladder + 200 + 100×depth`（depth1=+300），
  `leg_kind = "rev_sub"`——Python parity 边界重划（递归层无 oracle，设计
  §5.1），与在册 +200 区不混表。

### 1.2 账本双向短差（`ledger.rs`）

```rust
pub enum DiffSide { Short, Long }   // Short=先卖后买（在册全部腿）；Long=先买后卖
```

- `OpenCycle.sell_price` 改名 `open_price`（声明=能力：Long 腿存的是买价）；
  `ClosedCycle` 不变，profit 公式 `(sell−buy)×shares` 两侧对称。
- 新入口 `open_sub(key, shares, price, bar, anchor, side)`：**绝对股数**预算
  （预算基 = 父腿敞口，设计 §3.3"budget(node) = 父层在本节点域内释放的
  敞口"），非 `total_shares × frac`。
- **EarningShares 拒绝**：金额守恒（卖 V 买 V）对 Long 循环的 total_shares
  算术未定义——显式拒绝并计数（`n_sub_earning_rejects`），不静默降级；
  `close_diff` 中 AmountConserving × Long 以 assert 标记不可达（构造保证）。

### 1.3 SubLou 自引用节点（`level_operating_unit.rs`）

```rust
pub struct SubLou { ladder: usize, path: RevPath, child: Option<Box<SubLou>> }
```

- **方向 = 深度奇偶**（符号交替塔）：奇数深度 Long（反弹腿低买高卖），
  偶数深度 Short。槽占用在账本（单一真相源，osc 先例），节点只持坐标。
- **532 双通道每层复制**：开腿谓词（反向段存在陈述：confirmed type1 +
  盘背三岔镜像，仅震荡型）与闭腿谓词（段终结陈述：hard/pre type3 > 同锚
  type1 > 边界触线）是两个独立 match。
- **三范畴终止**：存在论（子级别 < FIRST_BSP_LADDER ⇒ 不实例化）、经济
  （锚中枢振幅 < 2×`sub_friction_rt`，默认 0.001 ≈ be9.7bps 先例量级 ⇒
  拒开计数）、深度预算（`path.depth() ≥ cfg.rev_sub_depth` ⇒ 不再生成
  子节点）。
- **级联不变式**：父腿闭合（T7/T6/T5/T2c/ZG/ZD 全路径）先平掉全部子树腿
  （最深优先，`n_sub_forced_close` 可观测）——开放问题 Q2 的本实验裁决：
  父腿兑现优先，未决子腿同价强闭。master 清仓走 `open_keys()` 全平（与
  REV 腿强平同语义，无日志即强平）。
- 子层开窗 bar 不步进（钩子位于 UpLeg 开腿块之前，与 voice"入场 bar 不
  步进"同语义）。

### 1.4 配置与守卫（`config.rs` / `runner.rs`）

- `rev_sub_depth: usize`（默认 0 = 在册行为）+ `sub_friction_rt: f64`
  （默认 0.001）；变体 `V2ofS1` = V2of + depth 1。
- capability guards：`rev_sub_depth > 0 ⇒ rev_paired`（legacy 腿无 kind/锚，
  R/SC 位同先例）；`rev_sub_depth + 1 ≤ MAX_REV_DEPTH`（路径容量）。
- master REV 腿（earning 反作用，legacy 语义）不挂子腿——不在本任务作用域，
  且全部已观测数据 earning 触发率 = 0（空有效域在册）。

### 1.5 守卫面验证

| 守卫 | 结果 |
|------|------|
| `cargo test --lib` | 97 passed / 0 failed（含新增 9 项：路径键、Long 循环守恒、子腿开闭、级联、振幅门、存在论终止、earning 拒） |
| O0≡P5 四面对账（OKLO 447K 真实磁带） | PASS（195 笔 / 317 trace 行） |
| V2of 在册基线复现 | +1499.20%，Δ(vs P5)=+410.4pp（与 C 段修复后在册 +410pp 一致） |
| depth=0 ≡ 当前行为 | V2ofS1 与 V2of trades 逐位相同（本次因空域恰好同时验证了 depth=1 的非侵入下界） |

---

## 2. 工作2：OKLO 深度1实验

### 2.1 设置

- 标的：OKLO 447,738 bar（1min，BH +307.08%）；磁带指纹 bsp=10,706 /
  div=8,462 / flips=24,417（C 段修复后语义）。
- 对照：V2of（depth=0，O_sub0 基线）vs V2ofS1（depth=1）。
- 预注册判据（设计 §5.4 + 任务原文）：
  - **O_sub1**：depth1 复利 > depth0 ∧ 子腿聚合净现金 > 0 ∧ REV 胜率不降
    ⇒ 递归有效，投入递归化；
  - **O_sub0**：depth1 ≤ depth0 或子腿净现金 ≤ 0 ⇒ 否证，
    "递归赋格的有效域为空，与 bar churn 证据链合并"。

### 2.2 结果

| 变体 | 复利% | Δ(vs P5) pp | rev对 | rev胜率% | rev净现金 | sub开 | sub对 | sub净现金 | 振幅拒 | 无中枢拒 |
|------|-------|-------------|-------|----------|-----------|-------|-------|-----------|--------|----------|
| V2of（depth0） | +1499.20 | +410.4 | 185 | 51.9 | +51,403 | — | — | — | — | — |
| V2ofS1（depth1） | +1499.20 | +410.4 | 185 | 51.9 | +51,403 | **0** | **0** | **0.0** | 1 | 5 |

判据求值：Δ复利 = +0.00pp（非 >0）∧ 子腿净现金 = 0（非 >0）→ **O_sub0**。

---

## 3. 空域漏斗（否证的结构分解）

```
185 个 REV 腿
 └─ 存在论终止：172 腿在 ladder 2（子级别 1 = bi 级，无中枢/买卖点概念）
     → 13 腿在 ladder 3 可挂子腿（7.0%）
        └─ 窗口域：13 个窗口共 1,935 bar（全磁带 0.43%），中位窗长 155 bar
            └─ 触发共现：窗口内 ladder 2 买侧事件仅 7 个
               （confirmed Buy1 = 1，盘背买 = 6；全磁带基率折算期望 ≈14，
                 下跌窗口内买触发被压制约 2×——方向上自洽，非管线缺陷）
                └─ 锚定守卫：6 次步进中 5 次无存活中枢拒、1 次振幅拒
                    → 0 次开腿
```

上游瓶颈补充：195 笔 trade 中 160 笔 entry_ladder=2 ⇒ `active_levels =
(2..2) = ∅`，**无任何 voice**——REV 机制本身只活在 35 笔 entry≥3 的
trade 里；rev_opens_by_ladder = [‥, 172@L2, 13@L3, 0, ‥]。

### 根因陈述

1. **存在论地板吸收了 93% 的 REV 质量**。深度1的宿主必须是 ladder ≥ 3 的
   REV 腿，而 OKLO 的 REV 腿几乎全部开在 FIRST_BSP_LADDER（设计 §4.3 预言
   "有效递归深度 ≤ 2 且深度 2 仅 entry≥5"——实际比预言更紧：深度 1 的
   宿主都只剩 7%）。
2. **震荡型镜像在反向窗口内结构性失配**。子腿继承了 V2o"存活中枢锚定"
   （逃逸型在父层三标的一致为负被关闭）。但 REV 窗口 = 本级别向下段 =
   子级别中枢正被向下终结的 regime；confirmed Buy1 是趋势背驰端点，按
   17课定义出现在价格已离开（跌穿）原中枢之后——"Buy1 共现存活中枢"在
   该窗口内先验稀薄（实测 5/6 无中枢）。卖侧的 V2o 裁决镜像到买侧反弹腿
   时，把锚条件也镜像了过去，而锚的可得性不是方向对称的。

---

## 4. 判决与边界

**判决：O_sub0 成立（预注册字面）。递归赋格在 OKLO 当前信号质量 ×
V2o 镜像锚定下不可行——有效域为空。** 设计报告 §5.4 的否证分支
（"聚合现金 ≤ 0 ⇒ 递归赋格的有效域为空，转否定性结算"）逐字命中。

但必须区分否证的两种形式（formalization-validity-domain，231号）：

- **本实验确立的（L2）**："深度1反弹腿 + 存活中枢锚定"在 OKLO 上定义域
  ≈ 空（6 触发 / 0 开腿）。这是锚定条件层面的否证。
- **本实验未检验的**：反弹腿开出后的经济符号（无样本）。"更细短差先验
  为负"链（bar churn −11973 / shared bar0 爆仓）因此**未被本实验加固也
  未被削弱**——空域结果与该链正交。

任何放宽（逃逸型镜像开腿、被终结中枢/新生中枢作锚、candidate Buy1）都是
**新的预注册实验**，不在本判决覆盖内；若未来要继续，优先级最高的单一变更
是把子腿锚源从"存活中枢"换为"Buy1 事件自带锚（含被终结中枢）"——它直接
针对 5/6 的主拒因。但按 §3 根因 1，即使锚条件完全放开，宿主域上限仍是
7% 的 REV 腿 × 0.43% 的 bar 覆盖——深度1在 OKLO 的天花板由 REV 级别分布
决定，不由锚条件决定。

---

## 结果包

1. **结论**：机制实装完成（路径键 + 自引用 SubLou + 双向短差 + 三范畴
   终止 + 级联不变式，97/97 测试 + O0≡P5 PASS）；OKLO 深度1判决 O_sub0
   （空域形式：0 开腿，Δ=+0.00pp）。
2. **定义依据**：38课程式方向对称（"所有操作刚好反过来就可以"→ SubLou
   方向参数化）；40课 N 重层次（预算基 = 父层敞口）；27课区间套（域嵌套）；
   17课 type1 定义（Buy1 在价格离开中枢后出现——§3 根因 2 的定义基础）；
   525号 FIRST_BSP_LADDER（存在论终止）；532号（双通道每层复制）。
3. **边界条件**：(a) 判决限 OKLO 单标的（L2）——多标的 REV 级别分布若
   不同（如 entry_ladder 普遍 ≥4 的标的），宿主域结论需复验；(b) 判决限
   "存活中枢锚定"的震荡型镜像——锚源放宽是新实验；(c) 若信号层未来产出
   ladder ≥3 的更密 REV 腿（如 θ 自适应改变开腿分布），空域结论失效需
   重跑；(d) sub_friction_rt=0.001 仅拒了 1/6，不是主导拒因，其取值不
   影响本判决。
4. **下游推论**：(a) 设计报告的全量递归化路径（SlotPath 终态架构）按
   §5.4 预注册规则**不投入**——本文档即否定性结算的载体；(b) 已实装的
   机制（路径键/DiffSide/SubLou）保留为 config 门控的休眠能力（默认
   depth=0 零接触，O0≡P5 守卫面不变）；(c) "更细短差先验为负"链不因本
   实验改变状态；(d) 若未来做锚源放宽实验，仅需改 SubLou 开腿分支的锚
   解析（机制其余部分复用）。
5. **谱系引用**：532（双通道分离——每层复制已实装）、525（存在论下限
   ——空域漏斗第一级的依据）、231（L2 先于架构投入——本实验存在的理由）、
   090/161（实装中拒绝 dead code / 拒绝声明膨胀：OpenCycle 字段改名、
   earning×Long 显式拒绝）、267（多重赋格术语）。挣股数空有效域先例
   （cost_basis≤0 零笔）与本判决同构：机制正确实装 + 域为空。
6. **影响声明**：改动 `rust/src/trading/{types,ledger,config,
   level_operating_unit,runner}.rs` + `lib.rs`（marshal 一行）；
   `LegClass::Rev` 表示变更（usize→RevPath）但深度 0 投影逐位不变；
   `OpenCycle.sell_price` 改名 `open_price`（账本内部，无外部消费者）；
   新增配置位 `rev_sub_depth`/`sub_friction_rt` 与变体 `V2ofS1`；新增
   计数器 8 个 + `sub_net_cash` marshal。不改变任何已结算定义；不触碰
   master 出场（C1）、osc 域腿（C2）、在册全部变体语义。

**认识论等级**：O0≡P5 守卫 L1（管线等价）；O_sub0 判决 L2（OKLO 单标的
真实数据，否定性结果——空域边界确立）。
