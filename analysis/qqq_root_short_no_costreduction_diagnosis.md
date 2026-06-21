# QQQ 根空头持仓期 8 个 type1 买点为何未触发降成本/翻多 — 诊断报告

> 任务：QQQ 根空头（recL2, bar 23688, $304→$608）被强平，期间 8 个 type1 买点出现但降成本子 voice 没有运行。诊断为什么 E 规则没有消费这些买点。**只诊断，不修复。**
>
> 认识论等级：**L2**（QQQ 单标的真实数据 1min 728,030 bar；可否证读数）。
> 复现：`UNN_DBG_QQQ=1 PYTHONPATH=src .venv/bin/python analysis/investigate_qqq_liq.py 2>&1 | grep -E "C-noflip|E-skip"`
> 诊断探针：`rust/src/trading/unified_necessity.rs` C-Short 分支前（C-noflip）+ E 根空头跳过点（E-skip），env `UNN_DBG_QQQ` 门控，未设时 bit-exact（11 单测全过）。

---

## 1. 结论

**根空头持仓期间 8 个 type1 买点既未触发 E（降成本），也未触发 C（翻多），是两个独立机制叠加的结果——且都是引擎的设计语义，不是 bug：**

| 路径 | 是否触发 | 直接原因 | 对应用户假设 |
|------|---------|---------|-------------|
| **E 降成本 spawn** | ❌ 全 8 次跳过 | 根空头是叶节点（T8）：`unified_necessity.rs:925` `if is_root && dir == Polarity::Short { continue; }` **无条件短路**，在读 `nf_buy` 触发判据之前就跳过 | **假设4 ✓（直接命中）** |
| **C 翻多（空→长 in-place）** | ❌ 全 8 次未开闸 | `located_buy` 级联链顶 `chain_source` ≤ 3（move(L1)），**从未到达 root_ladder=4（recL2）** ⇒ `s >= root_ladder` 恒 false ⇒ C-Short 门不开 | **假设3 ✓（但作用于 C 不是 E）** |

**关键澄清（用户框架的概念错配）**：在 unn 引擎里，根空头对买点的正确响应**不是 E 降成本，而是 C 翻多**（空→长 in-place）。"空头降成本应该在买点做多"在本引擎的语义里被分配给 C 规则，不是 E。E（降成本）在概念运动链里**只作用于多头相 / 子空头层**（引擎注释 105-107 行：`根空头不嵌套降成本…有效域边界：降成本只在多头相/子空头层`）。所以"E 没消费买点"对根空头而言是**设计语义**，真正该问的是"C 为什么没翻多"——答案是 located 买链从未爬到 recL2。

逐个否定其余两个用户假设：
- **假设1（E 只看卖点不看买点）❌**：错。E 对空头 voice 读的正是买点（`unified_necessity.rs:934` `Polarity::Short => (nf_buy[ladder].is_some(), false)`）。E 不是方向盲，是根空头被 925 行**提前短路**，根本没走到 934 行。
- **假设2（E 被 pending_locate 门控）❌**：错。E 严格**不**被 pending_locate 门控（这正是 N7 的命题：E 用自层 `nf_buy` 不查全局 located 链）。证据：bar 463941 `nf_buy@4=true`（自层买点确实 fire 了），E 仍跳过——跳过原因是 T8 叶节点（925 行），与 pending_locate 门控无关。

---

## 2. 定义依据

### 输入数据特征 → 满足哪条定义的哪个条件

QQQ 1min 728,030 bar，根 voice 生命周期（`investigate_qqq_liq.py` 实测）：

```
bar    929  根做多 @266.29 (recL2，涌现爬升后)        ← F 入场 + T5 涌现
bar  23688  C 翻空 @304.08 (recL2, shares=370.9)      ← sell_source≥4 ∧ sell1[4] 满足 ⇒ T14 翻空
bar  23688→581842  持有 558,154 bar，价格 304→608     ← 8 个 type1 买点期间无操作
bar 581842  A 强平 @608.17 (c≥2×basis ⟺ 涨100%)       ← 1x 逐仓 capital 耗尽，pnl=-112,778
bar 686915  根重做多 @562.56 (shares=4.8，资本几近耗尽) ← final_nav=3,566
```

8 个 type1 买点@recL2 的内部状态（探针 C-noflip 实测，按 bar 排序）：

| # | bar | 价格 | buy_source | s≥root_ladder(4) | located_buy 链(层,源) | nf_buy@4 |
|---|-----|------|-----------|------------------|----------------------|----------|
| 1 | 34572 | 299.79 | **Some(3)** | false (3<4) | [(2,3),(3,3)] | false |
| 2 | 127311 | 368.24 | None | false | [] | false |
| 3 | 162783 | — | None | false | [] | false |
| 4 | 274114 | — | None | false | [] | false |
| 5 | 341447 | — | None | false | [] | false |
| 6 | 463941 | 484.26 | None | false | [] | **true** |
| 7 | 478559 | — | None | false | [] | false |
| 8 | 480361 | — | **Some(3)** | false (3<4) | [(2,3),(3,3)] | false |

### 定义条目

1. **E 规则 / N7（降成本不需 pending，第17环）** — `unified_necessity.rs:909-945`。E spawn 触发 = voice **自层** `nf_buy[ladder]` ∨ 根自层 `sell_any`。但**前置短路**在 925 行：`is_root && dir == Short ⇒ continue`。满足"根 ∧ 空头"⇒ 跳过 E。8 个买点全部命中此条件（根 voice 全程空头@recL2）。

2. **T8（根空头是叶节点）** — 引擎注释 105-107 / 921-924 行：根空头用 MtM-external 会计（外部市场负债 `capital−units×c`），降成本子空头用 frozen-internal（父吸收）。**同一 voice 不可兼容两套口径**（短父 spawn 长子在 MtM 下破坏守恒 +m×c）⇒ 根空头不嵌套降成本。这是 N8 守恒（`prove_n8_conservation`）的逻辑必然，不是可调参数。

3. **C 规则 / T14（根多空对称翻转，第21环）** — `unified_necessity.rs:855-884`。空→长 in-place 翻多门：`single_root ∧ s >= root_ladder ∧ sig.buy1[s]`，其中 `s = chain_source(located_buy)`（级联链顶）。8 个买点：`s` ∈ {None×6, Some(3)×2}，**无一 ≥ root_ladder=4** ⇒ 门恒不开。

4. **N5 区间套自上而下定位（第14环）** — `cascade_arm` / `chain_source`（155-184 行）。`located_buy[k]` 由 `confirm_buy[k]` 级联武装；`confirm_buy[k]` 需 `nest_buy[k]` 武装（candidate）+ `rec_sub_evidence(k-1, Buy, ...)` 递归到 a0 确认。**confirm_buy[4]（recL2 买确认）在整个 558K bar 上行段从未 fire** ⇒ located 买链顶死锁在 move(L1)=3。

---

## 3. 根因：覆盖空间不对称（买/卖 located 链的级别天花板差异）

诊断的**深层结构事实**：

> **卖 located 链能到达 recL2（bar 23688 翻空成功），买 located 链在整个上行段从未到达 recL2（8 次翻多全失败）。**

- **翻空时（bar 23688）**：`sell_source ≥ 4 ∧ sell1[4]` 满足 ⇒ 卖级联链确实武装到了 recL2。在 2020 年初的局部顶，recL2 级别卖点 candidate 被 `rec_sub_evidence` 确认，`confirm_sell[4]` fire ⇒ `cascade_arm` 填满 located_sell[2..=4]。
- **持仓后（558K bar 上行）**：8 个 raw `buy1@recL2` 信号被 BSP 引擎检测到（type1 底背驰存在于原始信号层），但 located 买级联链**最高只到 move(L1)=3**（2 次）或干脆 None（6 次）。`confirm_buy[4]` 从未 fire——因为在持续单边上行的 regime 里，recL2 级别不产生"被次级别确认的底"（区间套确认要求 candidate 武装后低级别走势完美反转确认，而上行段里 recL2 级别的底背驰反复被破极值否定，`nest_buy[4]` 武装即破，confirm 链无法在 recL2 闭合）。

这与 `docs/covering_space_asymmetry.md`（本 session git 状态中已修改）的覆盖空间不对称同源：**上行 regime 下，卖点能 located 到高级别，买点只能 located 到低级别**——这是 regime（上行）在 located 拓扑上的投影，不是引擎 bug。

---

## 4. 边界条件（结论在什么条件下翻转）

1. **若 confirm_buy[recL2] 在持仓期 fire 过一次**（即上行段中途出现被次级别确认的 recL2 级别底）：located 买链会爬到 4，`s≥root_ladder` 成立，C 翻多开闸 ⇒ 结论翻转（短头会在该点平/翻多，不会持有到强平）。**这要求非单边上行 regime**（539号有效域：根翻空有效域 ⊂ 非上行 regime）。

2. **若删除 925 行 T8 短路**（允许根空头 spawn 降成本子 voice）：会立即撞 N8 守恒 panic（`prove_n8_conservation`）——短父 spawn 长子在 MtM 口径下破坏 Σunits=N_base。所以"让根空头降成本"不是放开一个门控就行，它与 MtM 根空头会计**不可兼容**（T8 是 N8 的必然推论，不是独立开关）。

3. **若根空头用 frozen-internal 会计替代 MtM**（放弃外部负债建模）：root_emergent_ladder 的涌现读数与翻转兑现会失去价值中性（NAV 不再守恒）⇒ 撞 prove_a5_relabel / prove_t14_root_flip。即 T8 的两支（MtM 根 vs frozen 子）任选其一都关闭另一条路径，无第三态。

4. **若 root_ladder 在持仓期向上涌现到更高级别**：`s≥root_ladder` 门槛更高，C 更难开（但短空头在上行段 root_emergent_ladder 不爬升——`want=Down` 而上行段高层为 Up，故 root_ladder 全程钉死 recL2=4，已由 8 行探针 `ladder4` 确认）。

---

## 5. 下游推论

1. **unn 引擎在单边上行 regime 上对"已翻空的根"无自救路径**：E 被 T8 关（设计），C 被 located 天花板关（regime），A 强平是唯一出口。这印证 `project_unn_root_flip_mtm.md` 的 L3 否证（CL/BRN 双 P1✗，MDD−90/−71）——QQQ 是同一 regime 否证的第三个标的（−112,778/根资本全损）。

2. **"根翻空"必然性（T14/第21环）的有效域边界被本案例精确定位**：T14 在 L0 必然（多空对称翻转）+ L2 守恒（MtM），但**翻空后的回路闭合依赖 located 买链能到达 root_ladder**——这个闭合条件在上行 regime 失效。T14 缺一条"翻空有否定线/强制回补线"（已被否定线删除史 df752ea6/6faf4ec4/c6deae87 三次否定），导致翻空后只能等 located 买链或强平。**根翻空的有效域 = located 买链可达 root_ladder 的 regime = 非单边上行**（与 539号闭合）。

3. **降成本（E）与翻多（C）是正交的两个回路**，不能互相替代：用户预期"E 降成本做多"实际是 C 的职责，而 C 依赖 located 链（N5），E 不依赖（N7）。本案例暴露：当 C 的 located 前提不满足时，**没有 E 兜底**（根空头 E 被 T8 关）⇒ 出现"既不降成本也不翻多"的裸暴露窗口。这是引擎在该 regime 的结构性缺口，但修复它需要触碰 T8/N8/T14 的会计内核，不是局部补丁。

---

## 6. 谱系引用

- **539号 / `project_unn_root_flip_mtm.md`**：根翻空有效域 ⊂ 非上行 regime（CL/BRN L3 否证）。本案例 = QQQ 同 regime 第三例。
- **T8（根空头叶节点）**：`unified_necessity.rs:105-107, 921-924`。MtM⊥frozen 不可兼容。
- **N7/N5 正交**：`docs/concept_movement_chain.md` 第17环（N7 E 自层）vs 第14环（N5 C 区间套定位）。
- **否定线删除史**：df752ea6e4 / 6faf4ec45a / c6deae8780 三次否定（翻空后强制回补线被删）——本案例是"删否定线后翻空无回补"的经验代价显现。
- **覆盖空间不对称**：`docs/covering_space_asymmetry.md`（本 session 已修改）——买/卖 located 链级别天花板差异。
- 不确定是否有专门针对"C located 天花板 vs E 兜底缺口"的谱系条目——若无，这可能是一条**待结晶的语法记录**（根空头裸暴露窗口 = located 买链不可达 ∧ E 被 T8 关），建议 escalate 判定是否升格。

---

## 7. 影响声明

- **改动文件**：`rust/src/trading/unified_necessity.rs` — 新增 2 处 env(`UNN_DBG_QQQ`)门控诊断探针（C-Short 分支前 + E 根空头跳过点），未设环境变量时**零行为影响**（bit-exact，11 单测全过 + cargo test release 通过）。**未改任何引擎逻辑**（符合"只诊断不修复"）。
- **新增文件**：本报告 `analysis/qqq_root_short_no_costreduction_diagnosis.md`。
- **未改动**：任何必然性 prove、E/C/A/D/F 规则逻辑、定义、会计内核。
- **复用**：`analysis/investigate_qqq_liq.py`（既有调查脚本，未改）+ 新 env 探针。

---

## 附：诊断探针原始输出（8 个 type1 买点全捕获）

```
[C-noflip bar=34572]  root SHORT@ladder4 raw_buy1@4=T | buy_source=Some(3) s>=root_ladder=false buy1@source=false single_root=true | located_buy链(层,源)=[(2,3),(3,3)]
[E-skip   bar=34572]  root SHORT@ladder4 raw_buy1=T nf_buy@4=false — E spawn 跳过（T8 根空头叶节点）
[C-noflip bar=127311] root SHORT@ladder4 raw_buy1@4=T | buy_source=None  s>=root_ladder=false ... located_buy链=[]
[C-noflip bar=162783] ... buy_source=None ... []
[C-noflip bar=274114] ... buy_source=None ... []
[C-noflip bar=341447] ... buy_source=None ... []
[C-noflip bar=463941] root SHORT@ladder4 raw_buy1@4=T | buy_source=None ... [] | nf_buy@4=TRUE ← E 本可触发(自层fire)但被T8短路
[C-noflip bar=478559] ... buy_source=None ... []
[C-noflip bar=480361] root SHORT@ladder4 raw_buy1@4=T | buy_source=Some(3) s>=root_ladder=false ... located_buy链=[(2,3),(3,3)]
```

**单行总结**：8 次买点中，located 买链顶 ∈ {None×6, move(L1)=3×2}，**无一达 recL2=4** ⇒ C 翻多门恒闭；同时根空头被 T8（925 行）无条件排除出 E ⇒ 两路全堵 ⇒ 裸空头暴露至 100% 涨幅强平。bar 463941 的 `nf_buy@4=true` 是铁证：E 触发判据满足了，但根空头在读判据前就被短路（假设2 pending_locate 门控被排除，假设4 T8 叶节点确认）。
