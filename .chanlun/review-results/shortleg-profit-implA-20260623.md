---
trigger: "做空腿赚 T3-implA（TaskList #113）：实装 Face A = 核心能动（核心翻转=cc走势完成区间套级联）+ 接受杠杆涌现 + 两regime整合 + 默认开启"
target: "shortleg-profit-spec §5.2 Face A 实装 + §8.1 默认开启 + L2 单标的诊断（BTC net-up 自动regime + bear 窗核心翻空）"
mode: "impl（rec_engine.rs face_a()/production() + rec_stream.rs new_production + ffi.rs 生产入口）+ L2 验证（单标的可否证）"
authority_spec: ".chanlun/review-results/shortleg-profit-spec-20260623.md（权威）§5.2/§6/§7/§8.1/§8.2/§14"
epistemic_level: "代码=L2（实装正确，4 Face A 单测 + 全 112 非ignored 单测绿 + rec≡flat BTC bit-exact 逐位一致）；L2 BTC/bear 读数=L2（单标的/单窗，可否证）；做空腿全 regime 赚=L0 待 L3（C #117 八标的）"
topo_address: "swarm/leverage-accept/implA"
parent_callback: "leverage-accept"
date: "2026-06-23"
depends_on: ["107(spec)", "110(implB Face B 基座)", "126(走势状态机/547修复)", "91(6模式判别器)", "567(否定线)", "574(确认滞后)", "577(regime=级别)", "578/579(解压制/杠杆来源A)", "547(覆盖层隔离)", "106(mid-scale-bleed)"]
recursion_decision: "atomic（原则15+576：单一相干引擎集成，子件强耦合 face_a()/production()/bit-exact OFF 守卫 + prove_tw_neutral/prove_pair_isolation；HF1 单一信号无回退）"
---

# 做空腿赚 implA（#113）：Face A 核心能动 — 核心翻转=cc走势完成 + 接受杠杆 + 默认开启

> 工位：implA（TaskList #113，topo_address `swarm/leverage-accept/implA`，parent_callback `leverage-accept`）。
> 权威规格：`shortleg-profit-spec-20260623.md` §5.2（Face A）/§6（接受杠杆）/§7（两regime）/§8.1（默认开启）/§8.2（L3矩阵）/§14（mid-scale 解耦）。
> 基座：Face B（#110 implB，`shortleg-profit-implB-20260623.md`）——均匀定仓（删 geom_tower）+ 真否定线 [ZD,ZG]。
> 递归分解裁决（原则15/576）：**atomic**——单一相干引擎集成，子件（face_a 配置 / production 默认开启 / OFF bit-exact 守卫）经 EngineConfig + g_pair 核心 churn 段 + prove 守恒守卫强耦合，任一单独变更破 bit-exact 与守恒原子性。HF1 单一信号：核心翻空唯一闸门 = `d_top[cc]` 走势完成，无 regime 回退分支。

---

## 〇、一句话结论

Face A 实装 = **Face B 基座（uniform 定仓 + 真否定线）叠加核心翻转吃熊**——核心多腿（cc=`highest_active_long`）在**自己级别走势完成**（`d_top[cc]`=全深度区间套链贯通真顶=第一类卖点，区间套级联减滞后）翻空镜像（`enable_pair_core_short`，g_pair 核心 churn 段，已存在自 bear-validate），次级别卖点绝不翻核心（547，走 `below_core_long` 独立空腿）。两 regime 由「哪级别走势完成」**自动整合**（零 `if regime`）。**默认开启**：`EngineConfig::production()` 令 Face A 为生产/默认回测引擎（FFI/python 入口 `new_production`），`T_OFF_BASELINE` ⇒ instances OFF 基线（bit-exact 回归守卫）。**核心增量 = `face_a()` = `face_b()` + `enable_pair_core_short`（唯一差=核心翻空）**，**不开** `enable_pair_core_short_open`（=移除 below_core_long 门=net-up 假顶翻空灾难，§5.2.3）。

**L2 关键读数（BTC net-up + 3 bear 窗，可否证）**：
- ✓✓ **自动 regime 闸门生效（§5.2.4）**：BTC 强牛核心翻转闸门只开 **3 次**（`d_top[cc]` 稀疏），增量 short_pnl **−596**（−38317→−38913），**有界微负 ≠ leverage-accept 灾难 −106256**（裸 t3sell 大额做空）。strat +431.1% ≈ FACE_B +431.7%，liq=0。
- ✓ **liq=0 全窗**（BTC + 3 bear，即便 max_gross 1.04-1.42× 来源A 涌现）。
- ✗ **否定性 L2（§8.3 如实报告）**：Face A 核心翻转增量在本 3 bear 窗**边际**（ES +3963→+3949 / BRN +5692→+5692 churn=0 / CL +1965→+1924）——bear 利润由 **Face B 次级别独立空腿主导**，Face A 核心翻转（最高级别大反转，罕见，§5.4）在 6-20 月窗口内**未充分显形**（窗口不含跨年最高级别 d_top）。

---

## 一、Face A 集成（纯级别×买卖点，§5.2，复用 Face B 基座）

### 1.1 核心翻转机制 = cc 走势完成区间套级联（547 修复，已存在）

**关键发现：Face A 核心翻转逻辑已存在于 g_pair（bear-validate `enable_pair_core_short`）**——本 implA 是**集成 + 默认开启**，非新建机制。核心翻转判据（rec_engine.rs g_pair）逐条对账 spec §5.2：

| spec §5.2 判据 | g_pair 实装 | 满足 |
|---------------|------------|------|
| ① 级别收紧到 cc=`highest_active`（547，非 ≥cc 次级别）| `is_core_long_level = highest_active_long()==Some(k)`，翻转仅 `is_core_long_level && core_done` | ✓ 核心翻转只在 cc 级 |
| ① 走势完成闸门 = `d_top[cc]`（非裸 allsell type2/3）| `core_done = view.d_top[k]`（全深度区间套链贯通真顶=第一类卖点）| ✓ 走势完成最深确认 |
| ② 区间套级联减滞后（第27课构成）| `d_top[k]` = `divergence::d_top` 区间套链贯通（信号层 rec_stream 计算）= 高级别买卖点由次级别走势完成构成 | ✓ 级联在信号层（减滞后≠零滞后 R3）|
| ③ type2/3 仅作级联加速器不作独立触发（leverage-accept 对账）| **不开** `enable_pair_core_short_open`（=保留 below_core_long 门，t3sell 只在严格次级别开空）| ✓ 闸门=走势完成（§5.2.3）|
| ④ 自动 regime（R1，零 if regime）| net-up `d_top[cc]=false` ⇒ 闸门不开；bear `d_top[cc]=true` ⇒ 闸门开 | ✓ L2 坐实（下 §三）|
| ⑥ 核心能动无 clear_all 异物 | g_pair：`close_long_leg(k)` 全量 + `open_short_leg(k)` 全量（**同级别**，无跨级 clear_all）| ✓ 核心自己级别全量翻转 |

### 1.2 接受杠杆涌现 + 新定仓（§6，Face B 已实装，Face A 继承）

- **geom_tower 删 → 均匀基准单元**（`uniform_base_units = initial_capital × MOBILE_FRAC`，级别无关无 depth 衰减，Face B 实装）。核心翻空短腿亦用 `leg_open_units` ⇒ uniform 定仓（非 geom_tower 大配额）。
- **杠杆 = 多级别独立腿叠加（来源A，579）**：`max_gross` 观测量无 clamp（L2：BTC 0.98× / bear 1.04-1.42×）。核心翻空在 uniform 定仓下与次级别同尺度（非「大额」——见 §三否定性）。
- **否定线封顶**：核心空腿否定线 = cc 中枢 ZG（涨破=牛市恢复止损，`faceb_stop(view.zg[k])`，Face B 接通真否定线）⇒ liq=0。

### 1.3 默认开启（§8.1）——`production()` + 生产入口 `new_production`

**关键架构决策（no-workaround）**：「默认开启」落在**生产入口**（FFI/python 回测路径），**不改 `from_env()`/`RecStream::new`**：

| 层 | 配置来源 | 行为 | 理由 |
|----|---------|------|------|
| **生产/默认**（FFI `PyRecStream::new` → `RecStream::new_production`）| `EngineConfig::production()` | 默认 Face A；`T_OFF_BASELINE`⇒off()；显式变体 env⇒尊重 | python 回测走 FFI ⇒ **默认开启 Face A** |
| **Rust 内部/守卫**（`RecStream::new`/`new_with_a0`）| `from_env()`（不变）| OFF 基线 | `rec_flat_btc_bit_exact` 等 OFF 守卫 + 合成单测 base 契约（改默认会破）|
| **受控实验**（L2/L3 测试）| `new_with_config(EngineConfig::xxx())` | 显式变体 | 不受默认开启影响 |

`production()` 三档（互斥，env 显式优先）：`T_OFF_BASELINE`⇒`off()`（instances bit-exact 回归守卫）/ 显式变体 env⇒`from_env()`（受控实验）/ 无变体 env⇒`face_a()`（**默认开启**）。

**mid-scale #106 硬约束满足**：默认开启生产路径 = Face A = LegPair 路径（`enable_reading_b_pair`）⇒ on_bar 最先分叉 `consume_leg_pairs` **return**，**永不到达 instances sink/recover 路径**（rec_engine.rs:1849）⇒ **生产路径不残留无保护 sink**。instances sink 仅在 `off()` 基线（回归守卫）可达。t_engine.rs（flat）sink 同理只在 OFF/flat 基线，非 Face A 生产路径。

---

## 二、bit-exact OFF 守卫（R4）+ 守恒（R6）+ 测试

- **rec≡flat BTC bit-exact 逐位一致（at-scale，330MB BTC 全史）**：`rec_flat_btc_bit_exact` ⇒ `flat_nav=126027.46378992868 == rec_nav=126027.46378992868`，走势完成清仓 flat=2==rec=2。**OFF 基线在 Face A 默认开启实装后逐位不变**（`new`/`new_with_a0`/`from_env` 全不动，仅新增 `new_production`/`production()`/`face_a()`）。
- **守恒零违反**：`prove_tw_neutral` + `prove_pair_isolation`（547 隔离）全 BTC + 3 bear 窗零 panic。
- **全 112 非 ignored recursive_t 单测绿**（含 `rec_flat_bit_exact_含走势完成清仓路径` / 合成 `new默认等价segment来源` / t_engine 24 / Face B 6 / **新增 Face A 4**）。

**新增 Face A 单测 4（rec_driver.rs）**：
1. `facea_集成契约_facebase_叠加核心翻空`：face_a = face_b + pair_core_short，不开 pair_core_short_open（唯一增量=核心翻空）。
2. `facea_核心翻转_cc走势完成_翻空吃熊`：核心多腿在 cc `d_top` 翻空（close_long 全量 + open_short 全量），否定线=cc 中枢 ZG，liq=0，bear 持空。
3. `facea_次级别不翻核心_net_up闸门不开_547`：次级别 t3sell 不翻核心（独立 below_core_long 空腿）；cc 卖点非 d_top（net-up 回调）⇒ 核心不翻空（churn 门控 + 闸门不开）。
4. `facea_production_默认开启_off_baseline回归守卫`：production() 无变体 env⇒Face A；`T_OFF_BASELINE`⇒off()。

---

## 三、L2 验证（单标的可否证，§8.2）— `facea_l2_validate`

### 3.1 ① BTC 全史（强牛，核心翻转闸门最差窗——自动 regime 验证）

| 变体 | strat | bh | short_pnl | long_pnl | s_op | core_churn | l_stop/s_stop | liq | max_gross |
|------|-------|-----|-----------|----------|------|-----------|--------------|-----|-----------|
| OFF（instances/sink）| +26.0% | +1380.4% | — | — | 0 | 0 | 0/0 | **5 穿仓** | 0.00× |
| FACE_B（均匀+真否定线）| +431.7% | +1380.4% | −38317 | +470004 | 1127 | 3 | 1946/480 | **0** | 0.98× |
| **FACE_A**（+核心翻转）| **+431.1%** | +1380.4% | **−38913** | +470004 | 1130 | 3 | 1946/482 | **0** | 0.98× |

- ✓✓ **自动 regime 闸门生效（§5.2.4）**：核心翻转闸门只开 **3 次**（s_op 1127→1130=+3，core_churn=3=`d_top[cc]` 稀疏），增量 short_pnl **−596**（−38317→−38913）= **有界微负**。**对照 leverage-accept 裸 t3sell 大额做空 net-up 灾难 short_pnl +9623→−106256**：Face A 的 `d_top` 走势完成闸门（非裸 t3sell）使 net-up 核心翻空有界（574 确认滞后税），**无假顶翻空打主升浪灾难**。
- ✓ **liq=0（vs OFF 穿仓 liq=5）**：否定线封顶在核心翻空下仍生效。
- ✓ **strat +431.1% ≈ FACE_B +431.7%**：核心翻转在强牛是小额 overlay（3 笔），不破坏 Face B 解压制吃涨主力（long_pnl +470004 不变）。

### 3.2 ② 真 bear 窗（核心翻空吃熊——FACE_B vs FACE_A）

| 窗口 | 变体 | strat | bh | short_pnl | core_churn | s_op | l_stop/s_stop | liq | max_gross |
|------|------|-------|-----|-----------|-----------|------|--------------|-----|-----------|
| ES 2022 标普熊（−27%）| FACE_B | −6.9% | −22.9% | +3963 | 3 | 27 | 174/8 | 0 | 1.42× |
| | **FACE_A** | −6.9% | −22.9% | **+3949** | 3 | 30 | 174/11 | 0 | 1.42× |
| BRN 2022H2（−40%）| FACE_B | −4.4% | −36.3% | +5692 | 0 | 5 | 96/1 | 0 | 1.04× |
| | **FACE_A** | −4.4% | −36.3% | **+5692** | 0 | 5 | 96/1 | 0 | 1.04× |
| CL 2014-16 油崩（−75%）| FACE_B | −17.8% | −74.3% | +1965 | 3 | 50 | 370/17 | 0 | 1.37× |
| | **FACE_A** | −17.8% | −74.3% | **+1924** | 3 | 53 | 370/20 | 0 | 1.37× |

- ✓ **strat ≫ BH 全窗**（避险吃跌）：−6.9% vs −22.9% / −4.4% vs −36.3% / −17.8% vs −74.3%。liq=0 全窗。
- ✗ **否定性 L2（§8.3 如实报告）——Face A 核心翻转增量在本 bear 窗边际**：
  - ES：核心翻转 3 次（s_op +3），short_pnl +3963→**+3949（−14）**。
  - BRN：core_churn=0 ⇒ **核心未翻转**（该窗 cc 走势未经 `d_top` 完成），FACE_A≡FACE_B。
  - CL：核心翻转 3 次（s_op +3），short_pnl +1965→**+1924（−41）**。
  - **bear 利润由 Face B 次级别独立空腿主导**（short_pnl +3963/+5692/+1965 的主体）；**Face A 核心翻转（最高级别大反转，§5.4 罕见）在 6-20 月窗口内未充分显形**——窗口不含跨年最高级别（L4/L5）`d_top`，核心翻转走的是较低 cc 的 churn（0-3 次），uniform 定仓下短腿小且滞后（574 税），增量 ±14~41 微负。

### 3.3 两 regime 整合（§7 地基6）+ 与 §5.4 对账

- **自动整合验证（零 if regime）**：同一 `face_a()` 引擎，BTC 强牛核心不翻空（闸门稀疏 3 次有界）⊕ bear 窗次级别独立空腿吃跌（short_pnl 正）= 「哪级别走势完成」自动整合，零 `if regime`/`if level==N`。
- **与 spec §5.4 对账（一致）**：spec 明示「**Face B 是吃跌幅主力**（各级别独立 + 中间级别独立吃中间回调），**Face A 主治最高级别大反转（罕见）**」。L2 坐实：本 bear 窗（非跨年最高级别反转）下 Face B 主导、Face A 边际——**与 spec 框架一致，非否证**。Face A 价值（架构完备性：核心**能**翻转，解死锁「僵尸恒占 highest_active」face）须**含最高级别 d_top 的长窗口**或 **C #117 八标的全史**显形。

---

## 四、结果包六要素

### 1. 结论
Face A（#113）实装 = Face B 基座（均匀定仓 + 真否定线）+ 核心翻转吃熊（`enable_pair_core_short`：核心多腿在 cc `d_top` 走势完成区间套级联翻空镜像，547 隔离次级别不翻核心，§5.2.3 不开 below_core_long 拆除门）+ 默认开启（`production()`/`new_production`，FFI 生产入口默认 Face A，`T_OFF_BASELINE`⇒OFF bit-exact 守卫）。两 regime 由「哪级别走势完成」自动整合（零 if regime）。L2：BTC 强牛自动 regime 闸门有界（3 次/−596，非灾难 −106256），liq=0 全窗，rec≡flat bit-exact 逐位一致。**核心增量 = `face_a()`=`face_b()`+`enable_pair_core_short`**。

### 2. 定义依据
| 输入特征 | 定义 | 条目 |
|---------|------|------|
| 操作=级别×买卖点二维，删配额 | 编排者根本约束 + 操作语义二维完备 | spec 地基0/580 |
| 核心翻转只在 cc 走势完成 | 第17课走势终完美 + 第37课背驰=第一类买卖点 + 第27课区间套定位 | spec §5.2 / 574 |
| 次级别不翻核心（547）| 覆盖层隔离（r-1 层 deck 变换不施于 r 层）| 547 / 126(走势状态机 T4) |
| type2/3 不作核心翻转独立触发 | leverage-accept L3 否证（裸 t3sell net-up 灾难）| spec §5.2.3 / leverage-accept |
| 自动 regime（零 if regime）| regime=级别（走势递归自相似）| 577 / spec §7 |
| 接受杠杆=来源A 叠加 | 26课恒仓主动释放 + 多级别独立腿叠加 | 579 / spec §6.1 |

### 3. 边界条件（结论翻转处）
- **Face A 核心翻转价值须最高级别 d_top 显形**：本 3 bear 窗（6-20 月，非跨年最高级别反转）下核心翻转边际。若 C #117 八标的全史（含跨年顶）仍显示核心翻转零/负贡献 ⇒ Face A「核心能动」在 uniform 定仓下退化为「Face B + 小额滞后短腿」，须重审核心翻空定仓（uniform vs 核心仓尺度，§6.3 开放点）。
- **uniform 定仓削弱「核心大额吃熊」**：bear-validate 的「大额吃熊（max_gross>1×=核心仓尺度）」在 geom_tower 下成立；Face A 删 geom_tower 改 uniform ⇒ 核心翻空与次级别同尺度（非大额）。「大额」改由多级别叠加（来源A）涌现，非单核心大配额。若 L3 显示来源A 叠加不足以吃熊 ⇒ §6.3 定仓精化（风险预算定仓候选）。
- **net-up 核心翻转有界微负不可消除**（574 真539 税）——3 笔 −596 是确认滞后结构税，消除=over-claim。
- **纯级别×买卖点 L3 否证 ∧ 旧 sink 路径更优** ⇒ 编排者根本约束重审（spec §2.4，不预设）。

### 4. 下游推论
- **C #117（L3 验收）**：8 标的 + bear 子窗跑 `face_a()`（默认开启引擎），判 short_pnl 组合符号 + liq=0 + OFF bit-exact。本 implA 提供 BTC + 3 bear 窗 L2 子集 + **Face A vs Face B 增量画像**（核心翻转贡献分离）。**判据须含最高级别 d_top 窗口**以显形 Face A 价值，否则只验 Face B。
- **#108（mid-scale 透镜）**：Face A 默认开启生产路径无 sink（LegPair）⇒ 中间级别 per-level 解压制透镜消费 `face_a()` 输出。
- **#115 审查 / #116 异质审计**：审查 Face A 集成正确性 + 默认开启层（production 不动 from_env 是否引入隐性双路径）+ uniform 定仓下「核心大额吃熊」声明是否膨胀。
- **谱系候选（交 genealogist via C #117）**：① bear-validate「大额吃熊」（geom_tower 核心仓尺度）⊥ Face A uniform 定仓（来源A 叠加）——定仓机制改变「大额」语义，须张力检查（579 来源A vs 来源B 核心大额）；② Face A「核心能动」L2 边际 = 「最高级别大反转罕见」（§5.4）的实证，非否证。

### 5. 谱系引用
- **spec 地基0/580**（操作=级别×买卖点二维完备）：Face A 核心翻转=级别(cc)×买卖点(d_top走势完成)，无第三维。
- **574**（确认滞后）：net-up 核心翻转有界微负（−596）= 真539 税不消除（R3）；区间套级联减滞后≠零滞后。
- **547**（覆盖层隔离）：次级别卖点不翻核心（`is_core_long_level` 门 + `below_core_long` 次级别空腿）。
- **577/578/579**（regime=级别 / 解压制 / 杠杆来源A）：自动 regime（零 if regime）+ uniform 定仓来源A 叠加（max_gross 1.04-1.42×）。
- **567**（否定线封顶）：核心空腿否定线=cc 中枢 ZG ⇒ liq=0。
- **leverage-accept**（裸 t3sell net-up 灾难 −106256）：Face A 用 `d_top` 闸门（不开 pair_core_short_open）使 net-up 核心翻空有界 −596（§5.2.3 强制对账）。
- **★谱系候选（交 genealogist）**：bear-validate「核心大额吃熊」（geom_tower 核心仓尺度）⊥ Face A uniform 定仓（来源A 叠加）——「大额」语义随定仓机制改变，张力待检查。

### 6. 影响声明
- **改代码**：
  - `rust/src/recursive_t/rec_engine.rs`：新增 `EngineConfig::face_a()`（= face_b + pair_core_short）+ `production()`（默认开启三档调度）+ `any_variant_enabled()`（私有 helper）。**核心翻转机制 g_pair 未改**（已存在 `enable_pair_core_short`，bear-validate）。
  - `rust/src/recursive_t/rec_stream.rs`：新增 `RecStream::new_production`（生产入口，用 `production()`）；`new_with_a0`/`new` **不变**（OFF base 契约 + bit-exact 守卫）；新增 `facea_l2_validate` L2 测试。
  - `rust/src/recursive_t/ffi.rs`：`PyRecStream::new` → `new_production`（FFI/python 回测**默认开启 Face A**）。
  - `rust/src/recursive_t/rec_driver.rs`：新增 4 Face A 单测。
- **不删基线**：instances sink/recover/drain + geom_tower 保留作 OFF 回归守卫（`off()`/`T_OFF_BASELINE`）。
- **影响模块**：C #117（L3 验收消费 face_a 默认引擎）、#108（中间级别透镜）、#115/#116（审查/异质审计）。
- **bit-exact**：OFF（`new`/`new_with_a0`/`from_env`/`off()`）逐字不变（rec≡flat BTC 逐位一致 + 112 非 ignored 单测绿）。Face A 默认开启仅在 FFI 生产入口 `new_production` 经 `production()`。

---

## 五、认识论等级标注（formalization-validity-domain）

| 命题 | 等级 |
|------|------|
| Face A 代码实装正确（face_a 集成 + production 默认开启 + g_pair 核心翻转复用）| **L2**（4 Face A 单测 + 112 非 ignored 单测 + rec≡flat BTC bit-exact 逐位一致 + 守恒零 panic）|
| 默认开启不破 OFF bit-exact（new_production 隔离，from_env 不动）| **L2**（rec_flat_btc_bit_exact 逐位一致 + 合成单测绿）|
| 自动 regime 闸门生效（net-up 核心翻转有界 3 次/−596，非灾难）| **L2**（BTC 单标的，可否证；对照 leverage-accept −106256）|
| liq=0 即便 max_gross 1.04-1.42×（核心翻空+来源A）| **L2**（BTC + 3 bear 窗，否定线封顶实测）|
| net-up 核心翻转微负=确认滞后税 | **L2 + L0 推导**（574 结构税）|
| **Face A 核心翻转增量在本 3 bear 窗边际（Face B 主导）** | **L2 否定性**（核心翻转 0-3 次，±14~41 微负；最高级别大反转罕见 §5.4，非否证）|
| 做空腿全 regime 赚（含核心翻转价值）| **L0 待 L3**（C #117 八标的+含最高级别 d_top 窗口）|

> **核心诚实声明**：本 implA = **L2 实装 + L2 单标的/单窗验证**。四诚实约束：
> （a）**默认开启正确落位**——FFI 生产入口 `new_production`→`production()` 默认 Face A，`from_env()`/OFF 守卫不动（rec≡flat BTC 逐位一致证实）。
> （b）**自动 regime 闸门生效**——`d_top` 走势完成闸门使 net-up 核心翻空有界（3 次/−596），对照 leverage-accept 裸 t3sell −106256 灾难，证实 §5.2.3/§5.2.4。
> （c）★**否定性优先**——Face A 核心翻转增量在本 3 bear 窗**边际**（bear 利润由 Face B 次级别空腿主导），如实报告：Face A「核心能动」价值（最高级别大反转）罕见（§5.4），须含跨年最高级别 d_top 的窗口/全史显形，**不在本 L2 窗口硬断言 Face A 吃熊主力**。
> （d）**uniform 定仓削弱「核心大额」**——bear-validate「大额吃熊」（geom_tower 核心仓尺度）在 Face A uniform 定仓下退化为来源A 叠加（max_gross 1.04-1.42×），「大额」语义改变须 genealogist 张力检查。做空腿全 regime 赚 = L0 待 L3（C #117 判据看 short_pnl 符号）。
