# 摩擦+杠杆调整 L3：钉死 prop4 两个「突破」真伪

> 工位：缠论线 friction-model（topo_address: swarm/friction-model）。日期：2026-06-23。
> 落盘：主仓库绝对路径（worktree 会清理）。
> 认识论等级：**strat%/摩擦/杠杆敞口 = L3**（8 标的真实数据 + CL 成本敏感性 4 点）；**去杠杆收益 = L2 估计**（峰值/均值杠杆一阶归一，非严格 leverage-cap 回测，诚实标注）。
> 实装：隔离 git worktree（`/tmp/friction-bidir` = 分支 prop4-bidir-consume-C；`/tmp/friction-current` = 分支 prop4-nest-readingB detached）。摩擦+敞口全 flag 门控（`cost_bps_per_side` 默认 0），**bit-exact 守住**（cost0 逐字复现头条 + 102 单测绿）。

---

## 0. 一句话判决

**两个「突破」是两种不同的伪影机制，必须分别用摩擦/杠杆钉死：**

1. **CL +1429%（双向 consume，单仓 5447 翻转）= 摩擦稀释的真 range alpha**：摩擦机制（单仓 gross==net，非杠杆）。在 CL 真实 taker 成本区间（0.58–1.45 bps/侧）**残存超 BH**（1 bps → **+414%**，1.45 bps → **+215%**，皆 ≫ BH+28.2%），盈亏平衡 ≈ 2.0 bps（高于实盘区间）。**结论：CL range alpha 真实但 magnitude 被零摩擦夸大 3.5–7×**；它是 8 标的中**唯一**摩擦后仍超 BH 的标的。

2. **ES +678/681%（多重赋格 reading_b）= 杠杆溢价，非 alpha**：摩擦可忽略（仅 238 笔），但实测多腿叠加 **峰值毛 2.61× / 净 1.74×、均值毛 1.32× / 净 1.12×** = 真杠杆（BH 无法复制）。去杠杆后 ES 收益落在 **+389%（峰值归一）~ +604%（均值归一）**，**横跨 BH+594.3%**——+84pp 超额 ⊂ 杠杆溢价，**非稳健 alpha**。QQQ 多重赋格甚至 raw +159.6% < BH+174.6% 且杠杆 2.42×。**结论：ES 超 BH 是杠杆伪影，去杠杆后 ≈ BH（无稳健 alpha）。这关闭 563号开放轴#1（「杠杆调整后真超 BH」由「存疑」→「无稳健 alpha」）。**

**总判决：摩擦+杠杆调整后，吃跌/吃盘整唯一可信的真超 BH 是 CL 的 consume range alpha（震荡 regime），且 magnitude 大幅缩水；强牛标的的「突破」（ES 杠杆 / BRN consume）全部是伪影。539 做空腿 regime 税不可约的结论被再次确认。**

---

## 1. 摩擦+杠杆模型（L0 实装）

### 1.1 摩擦：每 fill 双边 taker 成本
- `EngineConfig.cost_bps_per_side`（bps/侧，默认 0.0 ⇒ bit-exact）→ 引擎 `cost_rate = bps/1e4`。
- `charge_cost(notional)`：每个 fill 点扣 `notional × cost_rate` 出 `free`，累计 `total_cost_paid`/`n_fills`/`total_notional`。
- **fill 点全覆盖**（两路径）：
  - instances 路径（OFF/ANCHOR/NEST/consume）：`enter`(建仓 m·c)、`clear_all`(平仓 Σuₖ·c)、`sink`(父减+子开)、`recover`(平短差+升回)、`drain`(减暴露)、`deploy_earning`(增股数)、consume(`nest_flip_to`=clear_all+enter)。
  - reading_b 路径（多重赋格）：`open_leg`(m·c)、`close_leg`(uₖ·c)。腿 switch = close_leg+open_leg = **自动双边**。
  - **整仓翻转 = clear_all(平) + enter(反向) ⇒ 双边成本**（headline CL consume 每翻转扣两侧）。
  - 强平（margin call）**不扣费**（NAV≤0 的破产事件，成本无意义；不破坏守恒）。
- **TW 守恒升级**：摩擦是有意财富流出非会计泄漏。守卫基准从 `total_wealth` 改为 `tw_base = total_wealth + total_cost_paid`（cost0 ⇒ total_cost_paid=0 ⇒ 逐字不变）。`prove_tw_neutral` 验证仓位转移 NAV 中性，成本作为显式扣减被守恒律纳入。

### 1.2 杠杆：毛/净敞口峰值 + 均值
- `max_gross/max_net`（相对 NAV，×100）：单仓 ⇒ gross==net；多腿叠加（同时多+空）⇒ gross>net=真杠杆。
- `mean_gross/mean_net`（仅持仓 bar 采样）：**给去杠杆收益更紧的估计**（峰值归一只给下界 strat/peak；均值归一 strat/mean_net）。

### 1.3 per-标的 taker 成本（bps/侧，L2 取值依据）
依据 memory `project_cl_1s_maker_execution_verdict`（CL 盈亏平衡 0.58–1.45 bps/侧）+ 流动性分级：
CL 1.0 / BRN 1.5 / DX 1.0 / GC 1.0 / ES 0.75 / QQQ 0.75 / BTC 5.0（加密 taker）/ OKLO 10.0（小盘新股宽价差）。env `T_COST_BPS` 覆盖（敏感性）。

### 1.4 bit-exact 验证（cost0 逐字复现 = 摩擦钩子正确性的 L1 守卫）
| 标的×变体 | cost0 实测 | 已发布基线 | 匹配 |
|---|---|---|---|
| CL NEST_CONS | **+1429.2%** | 561/bidir CL +CONS +1429.2% | ✓ 逐字 |
| ES RB_DIVERGE | **+678.4%** | 563号 ES DIVERGE +678.4% | ✓ 逐字 |
| QQQ RB_DIVERGE | **+159.6%** | 563号 QQQ +159.6% | ✓ 逐字 |
| OKLO NEST_CONS | **−92.8%** | bidir OKLO +CONS −92.8% | ✓ 逐字 |
102 单测绿（含 `rec_flat_bit_exact`）。⇒ cost_rate=0 路径完全不改行为，摩擦钩子无副作用。

---

## 2. 突破 (a)：CL +1429% 双向 consume —— 摩擦稀释的真 range alpha（L3）

### 2.1 杠杆诊断：单仓非杠杆（先排除杠杆嫌疑）
CL NEST_CONS `maxGross == maxNet = 3.05×`。**gross==net ⇒ 单一方向、无多空叠加 ⇒ 非借贷杠杆**——3.05× 是做空腿浮亏导致 notional/当前NAV 比值短暂膨胀（MtM-underwater），不是借来的暴露。故 CL 的判决**纯粹是摩擦问题**，与杠杆无关。

### 2.2 摩擦敏感性（CL，5447 consumes / ~10896 fills）
| cost (bps/侧) | strat% | vs BH+28.2% | cost_paid（初始$100k）|
|---|---|---|---|
| 0（零摩擦）| **+1429.2%** | 超 50× | 0 |
| **1.0**（CL 中位）| **+414.3%** | **超 14.7×** | $213,498 |
| **1.45**（CL 高位）| **+214.9%** | **超 7.6×** | $227,588 |
| 3.0（远超实盘）| −41.9% | **跌破/转负** | $191,836 |

**盈亏平衡 ≈ 2.0–2.2 bps/侧**（1.45→+215% 与 3.0→−42% 之间）。CL 实盘 taker 区间 0.58–1.45 bps **完全低于**盈亏平衡 ⇒ **CL range alpha 跨实盘成本区间残存超 BH**。

### 2.3 判决
- **真 alpha（残存）**：CL consume 在震荡/range regime 真捕捉双向波动，摩擦后仍 7.6–14.7× BH（实盘成本区间）。
- **magnitude 是零摩擦伪影（确认 bidir 报告 caveat）**：+1429% → +215~414%，**缩水 3.5–7×**。「机制 real，magnitude 零摩擦夸大」精确成立。
- **边界**：盈亏平衡 2bps 之上（如 OKLO 级 10bps、BTC 级 5bps）alpha 消失。CL 之所以存活是其低 taker 成本 + 强震荡 regime 共同作用，不可迁移到高成本标的。

---

## 3. 突破 (b)：ES +678/681% 多重赋格 —— 杠杆溢价，非 alpha（L3 敞口 / L2 去杠杆）

### 3.1 摩擦可忽略（先排除摩擦）
reading_b 几何塔腿 switch 稀疏（ES 仅 **238 fills**，cost_paid $360 = 0.36% 初始）。ES cost0 +678.4% → costR(0.75bps) **+676.9%**（几乎不变）。**ES 的「突破」不是摩擦问题**——与 CL consume 的 10896 fills 形成鲜明对比。

### 3.2 杠杆诊断（决定性）：真多腿叠加
| 标的 | BH | RB_DIVERGE costR | 峰值毛× | 峰值净× | 均值毛× | 均值净× | fills |
|---|---|---|---|---|---|---|---|
| **ES** | +594.3 | **+676.9** | **2.61** | **1.74** | **1.32** | **1.12** | 238 |
| QQQ | +174.6 | +159.3 | 4.65 | 2.42 | 1.51 | 1.32 | 380 |
| GC | +257.3 | +148.0 | 1.85 | 0.91 | 1.08 | 0.52 | 3960 |
| BRN | +87.4 | +60.0 | 1.25 | 0.99 | 0.85 | 0.74 | 1286 |
| DX | +4.1 | +3.6 | 1.81 | 1.02 | 1.06 | 0.89 | 368 |

- **gross > net（ES 2.61>1.74，QQQ 4.65>2.42）= 同时持多腿+空腿 = 真借贷式杠杆**（机制：做空腿卖空 refund `free+=m·c` ⇒ 毛敞口 > free）。BH 无法复制 2.61× 毛敞口。**精确复现 563号裂隙2 实测（ES 净 1.74× / QQQ 净 2.42×）。**

### 3.3 去杠杆收益（L2 一阶估计）
| 标的 | costR | 峰值归一 (÷净峰值) | 均值归一 (÷净均值) | BH | 判决 |
|---|---|---|---|---|---|
| **ES DIVERGE** | +676.9 | **+389%**（÷1.74）| **+604%**（÷1.12）| +594.3 | **横跨 BH ⇒ 边际/无稳健 alpha** |
| ES DTOP（563号 +681.4%@净1.27×）| +681.4 | **+537%**（÷1.27）| — | +594.3 | **峰值归一已 < BH** |
| QQQ | +159.3 | +66%（÷2.42）| +121%（÷1.32）| +174.6 | **raw 已 < BH，去杠杆更差** |

**ES 判决**：ES 是唯一 raw 超 BH 的多重赋格标的。其 +84pp 超额（+678 vs +594）**完全落在杠杆溢价区间内**——去杠杆后 +389%（峰值）~ +604%（均值）横跨 BH+594%。无论用峰值还是均值归一，**ES 不展示稳健 alpha**；它是一个 ~1.1–1.7× 净杠杆的多空策略，超额来自暴露而非择时。**563号开放轴#1（杠杆调整后真超 BH 存疑）由本工位关闭为「无稳健 alpha」。**

### 3.4 杠杆归一的方法论诚实声明（formalization-validity-domain）
- 「÷净杠杆」是**一阶 L2 估计**，非严格 leverage-cap 回测（后者需改 geom_tower 限毛敞口≤1×，改变策略机制）。峰值归一给下界，均值归一给上界。
- **`net<1` 标的（GC 0.52×/BRN 0.74×/DX 0.89×）的均值归一（÷<1）会虚高 = 伪影**（奖励欠投资），**不可声明为 alpha**。这些标的 raw 已 < BH（GC+148<+257，BRN+60<+87，DX+3.6≈+4.1），honest 判决 = **未杠杆、单纯跑输/打平 BH**。杠杆是 ES/QQQ 的问题，不是 GC/BRN/DX 的问题。

---

## 4. 突破 (c)：摩擦+杠杆调整后哪些是真 alpha（L3 8 标的全景）

### 4.1 双向 consume（bidir 分支，full-cover+reverse）8/8 摩擦矩阵
| 标的 | BH | OFF(costR) | ANCHOR(costR) | NEST_CONS cost0 | **NEST_CONS costR** | bps | 毛=净× | consumes | cost_paid |
|---|---|---|---|---|---|---|---|---|---|
| CL | +28.2 | +16.7 | −63.2 | +1429.2 | **+414.3*** | 1.0 | 3.05 | 5447 | $213k |
| BRN | +87.4 | −28.1 | −90.1 | +178.6 | **+30.4** | 1.5 | 1.64 | 2528 | $200k |
| DX | +4.1 | −2.3 | −12.8 | +0.2 | **−34.8** | 1.0 | 1.04 | 2144 | $37k |
| GC | +257.3 | −25.5 | +84.2 | −5.9 | **−69.8** | 1.0 | 1.15 | 5676 | $85k |
| ES | +594.3 | −9.5 | +300.5 | −28.8 | **−65.0** | 0.75 | 1.17 | 4739 | $39k |
| QQQ | +174.6 | −10.5 | −54.5 | +16.2 | **−0.8** | 0.75 | 1.08 | 1054 | $16k |
| BTC | +1380.4 | +12.1 | +1029.2 | −99.3 | **−100.0** | 5.0 | 1.51 | 5052 | $24k |
| OKLO | +307.1 | +39.5 | −89.9 | −92.8 | **−97.2** | 10.0 | 2.53 | 462 | $48k |

**consume P1（超 BH）：cost0 = {CL, BRN} 2/8 → costReal = {CL} 1/8。** 摩擦把 BRN（+178.6→+30.4 < BH+87.4）从超 BH 打到跌破。

### 4.2 跨变体真 alpha 清算（摩擦+杠杆后）
- **CL consume**：唯一摩擦后真超 BH（range regime，实盘成本区间）。✓ 真 alpha（magnitude 缩水）。
- **BRN consume**：零摩擦超 BH（+178.6），**摩擦后跌破**（1.5bps→+30.4 < +87.4）。盈亏平衡 ≈ 0.8bps（0.75bps→+90.6 刚超，1.5bps→+30.4 跌破），**低于 BRN 实盘 taker（1.5bps）⇒ 实盘失败**。✗ 摩擦伪影（razor-thin，仅在亚实盘成本存活）。
- **ES 多重赋格**：raw 超 BH，**杠杆溢价**（去杠杆 ≈ BH）。✗ 杠杆伪影。
- **QQQ/GC/BRN/DX 多重赋格**：raw 已 ≤ BH。✗ 非 alpha。
- **ANCHOR（吃涨）**：BTC +1029/ES +300/GC +84 全 **< BH**（BTC+1380/ES+594/GC+257）。ANCHOR 是回撤缩减器（552号），**非超 BH alpha**。
- **OFF/其余**：全 < BH。

**唯一摩擦+杠杆双重存活的真超 BH = CL consume（震荡 regime）。所有强牛标的的「突破」皆伪影（ES 杠杆 / BRN-consume 摩擦 / ANCHOR 不及 BH）。**

---

## 5. 六要素结果包

1. **结论**：prop4 两个「突破」是两种伪影机制。**(a) CL+1429% = 摩擦稀释的真 range alpha**（单仓非杠杆；实盘成本 0.58–1.45bps 区间残存 +215~414% ≫ BH+28%，盈亏平衡~2bps；magnitude 零摩擦夸大 3.5–7×）。**(b) ES+678/681% = 杠杆溢价非 alpha**（摩擦可忽略 238 笔；峰值毛 2.61×/净 1.74×、均值净 1.12×；去杠杆 +389~604% 横跨 BH+594，超额⊂杠杆溢价）。**(c) 摩擦+杠杆后唯一真超 BH = CL consume（震荡）；强牛突破全伪影。**

2. **定义依据**：
   - alpha 定义 = 风险/暴露可比下的超额（杠杆调整 = 同等净暴露比较；摩擦调整 = 含 taker 交易成本的净收益）。BH = 1× 满仓多头无杠杆基准。
   - 第27课区间套 / 557 顶层闸门对偶（consume = type1∨2∨3 买卖点 full cover+反向）/ 556 读法B（多重赋格每级别独立腿）= 两突破的引擎来源。
   - taker 成本依据 `project_cl_1s_maker_execution_verdict`（CL 盈亏平衡 0.58–1.45bps/侧）。
   - 杠杆来源：做空腿 `free+=m·c` refund ⇒ 毛敞口>free（563号裂隙2 机制）。

3. **边界条件（结论翻转处）**：
   - (a) **CL alpha 翻转**：taker 成本 > ~2bps（盈亏平衡）⇒ CL 跌破 BH（3bps 实测 −41.9%）。CL 低成本+强震荡 regime 是存活前提，不可迁移到高成本标的（BTC 5bps/OKLO 10bps consume 全 −100%~−97%）。
   - (b) **ES 杠杆判决翻转**：若严格 leverage-cap 回测（毛敞口≤1×）显示去杠杆收益 > BH，则 ES 有 alpha。本工位是一阶归一（峰值/均值），ES 横跨 BH ⇒ 边际；严格 cap 回测未做（有效域边界）。若 ES 均值净杠杆 < 1.14×（=678/594）则去杠杆超 BH——实测 1.12×，**恰在边界内侧** ⇒ 判决「无稳健 alpha」鲁棒于该 1pp 边际。
   - (c) **去杠杆方法**：`net<1` 标的（GC/BRN/DX）均值归一虚高是伪影，不作 alpha 声明。

4. **下游推论**：
   - **吃跌/吃盘整唯一可信路径 = 低成本震荡标的的 consume（CL）**。吃跌探索的「突破」叙事（ES 超 BH / BRN range alpha）被摩擦+杠杆双重证伪——这是 539 做空腿 regime 税不可约的第 N 次确认（强牛标的无论 consume 还是多重赋格，扣除杠杆/摩擦后都不超 BH）。
   - **ANCHOR（吃涨）与 CL-consume（吃震荡）是两个正交的非-alpha-vs-BH 结果**：ANCHOR 缩回撤但不超 BH；consume 在震荡超 BH 但 magnitude 缩水。无单一引擎在强牛超 BH（去杠杆后）。
   - **任何报告 raw strat% 超 BH 的 prop4 结果，必须同时报告 (毛/净敞口, fills/cost) 才可信**——本工位的摩擦+杠杆敞口测量应成为 prop4 系回测的标准验收项（已实装为可复用 harness）。

5. **谱系引用**：
   - **563号**（吃跌 L3 经验侧，开放轴#1「杠杆调整后真超 BH 存疑」）：本工位**关闭该开放轴**——ES 去杠杆 ≈ BH（无稳健 alpha），QQQ 去杠杆 ≪ BH。563号裂隙2（ES 净 1.74×/QQQ 净 2.42×）实测**逐字复现**并补全均值杠杆（ES 1.12×/QQQ 1.32×）。
   - **561号 / bidir consume**（CL+1429% range alpha「magnitude 是零摩擦伪影」caveat）：本工位**确认 caveat 但精化**——magnitude 确实零摩擦夸大 3.5–7×，但 alpha 在实盘成本区间**存活**（非纯伪影）。
   - **539号**（做空腿 regime 失血不可约）：强牛突破全伪影 = 第 N 次确认。
   - **552号**（ANCHOR 吃涨缩回撤）：ANCHOR 全标的 < BH，确认其为回撤缩减器非 alpha。
   - **231号 / formalization-validity-domain**：有效域≠定义域第 N 例——CL alpha 的有效域 = {低 taker 成本 ∩ 震荡 regime}（单标的），非全标的；杠杆归一的有效域 = {net>1 标的}（GC/BRN/DX 的 net<1 归一是伪影）。
   - memory：`project_cl_1s_maker_execution_verdict`（盈亏平衡 bps）、`project_t_short_leg_regime_function`、`project_h4_amp_admission_verdict`（零摩擦伪影先例）。

6. **影响声明**：
   - **改动文件（隔离 worktree，未合入主树避免并发碰撞）**：
     - `prop4-bidir-consume-C` 分支（`/tmp/friction-bidir`）：`rec_engine.rs`（+`cost_bps_per_side`/`cost_rate`/`total_cost_paid`/`charge_cost`/`tw_base`/`update_exposure` + 9 fill 点扣费 + 单仓敞口跟踪）、`rec_stream.rs`（新 harness `prop4_friction_leverage_l3`）。
     - `prop4-nest-readingB` 分支（`/tmp/friction-current` detached）：同上 + open_leg/close_leg 扣费 + 均值敞口跟踪 + 新 harness `prop4_readingb_friction_l3`。
   - **bit-exact 守住**：cost0 逐字复现 4 个头条 + 102 单测绿。摩擦/敞口全 flag 门控（default off）。
   - **patch 存档**（复现）：`.chanlun/review-results/friction-patch-bidir-prop4-20260623.diff`、`friction-patch-readingb-prop4-20260623.diff`。
   - **不改任何已结算定义**；关闭 563号开放轴#1；确认 561 caveat + 539 不可约。**不触发中断**（摩擦/杠杆是收益层有效域读数，非概念矛盾）。

---

## 6. 认识论等级标注

| 命题 | 等级 |
|---|---|
| cost0 逐字复现头条（CL+1429/ES+678/QQQ+159/OKLO−92.8）| **L1**（管线/钩子正确性，bit-exact）|
| CL 摩擦敏感性（+414@1bps/+215@1.45bps/−42@3bps，盈亏平衡~2bps）| **L3**（真实数据 4 成本点）|
| CL range alpha 实盘成本区间存活 | **L3**（否证鲁棒：3bps 转负= 边界）|
| nest_consume 8/8 摩擦矩阵（P1 cost0=2 → costReal=1）| **L3**（含否定性：BRN/QQQ 摩擦后跌破）|
| reading_b 杠杆敞口（ES 峰值2.61/1.74×、均值1.32/1.12×；QQQ 4.65/2.42×）| **L3**（真实数据，复现563裂隙2 + 补均值）|
| ES 去杠杆 ≈ BH（无稳健 alpha）| **L2**（峰值/均值一阶归一，非严格 leverage-cap 回测）|
| 强牛突破全伪影（杠杆/摩擦/regime）| **L3**（8 标的，否定性主导）|
| 严格 leverage-cap 回测（毛敞口≤1×）| **未做**（有效域边界，诚实标注；一阶归一已给横跨 BH 的边际判决）|
| 异质审计（Codex/Gemini） | **未执行**（与 563号同日双 429 同类约束；本工位为数值实证非概念质询，缺口标注）|

---

## 7. 开放轴（未关闭，诚实标注）

1. **严格 leverage-cap 回测**：改 `geom_tower_quota`/`open_leg` 限毛敞口≤1× NAV，跑 ES 去杠杆收益（消除峰值/均值一阶归一的横跨-BH 不确定性）。本工位一阶归一已判「无稳健 alpha」，cap 回测可将 ES 钉死为确定性 < BH 或确认边际。
2. **CL alpha 的 regime 边界**：CL 存活 = 低成本 ∩ 震荡。需在更多低成本震荡标的（如其他能源/外汇期货）验证 consume range alpha 是否可迁移（当前 n=1，有效域未交叉验证）。
3. **maker 执行**：本工位用 taker 成本。若 consume 翻转能挂 maker（rebate），CL alpha 的盈亏平衡上移（`project_cl_1s_maker_execution_verdict`：fill rate ≥ 92% 才转正）——maker 可行性未建模。
