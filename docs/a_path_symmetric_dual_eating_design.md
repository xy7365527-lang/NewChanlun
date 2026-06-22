# A 路对称双吃设计文档

> 来源：A 路 understand/design Workflow (wf_167eb4de-b5c)，2026-06-22
> 编排者平空洞察整合：§3.3 已覆盖空头 anchor 对称（平空用该级别跌势完成 D_TOP，非次级别买点）

关键行号已核实。勘察报告引用准确(emergence_upgrade 594-610、sink 612-651、recover 653-696、route_bsp 722+、A'' trend_done_clear 865-878、A' emergence 880-889、B 路由 891-906、direction check 910-916)。现在我有足够信息产出设计文档。

---

# A 路架构设计：对称双吃（每级别骑该级别走势）严格形式

**作者**：A 路架构设计师 | **日期**：2026-06-22 | **类型**：plan/设计（不写代码）
**遵守**：no-patch-mentality / formalization-validity-domain / no-workaround

---

## 0. 目标的严格重述（编排者 2026-06-22 校正）

**目标 = 吃每级别涨跌幅绝对值 = 每级别骑该级别走势**

形式：`收益 = Σ_k [ |该级别k涨段幅度| + |该级别k跌段幅度| ]`

每级别 k 维持一个独立 T 实例，**该实例的方向 = 它所骑走势节点的方向**：
- 该级别走势处于涨段 → 持多（吃涨）
- 该级别走势处于跌段 → 持空（吃跌）
- **该级别走势完成时刻** → 精确切换方向（涨段终完美→翻空吃跌；跌段终完美→翻多吃涨）

**关键区分（已被 L3 否证的是错误形式，不是目标）**：

| 错误形式 | 病灶 | L3 结果 |
|---------|------|---------|
| anchor 死扣多（永不切换） | 只吃涨不吃跌 | 强牛 5/8 解踏空（对的一半），但跌段全漏 |
| cascade flip（emergent_top.completed 切换） | 切换**时机错**（粗识别误把强牛中途次级别完成当顶部） | OKLO +458.9→−106.6 穿仓，有效域空集 8/8 |
| B 路递归 anchor 空腿（全程持空） | 全程不切换 | 强牛失血 |

**A 路赌注**：用**区间套级联（次级别背驰逐级确认）** 替代 emergent_top.completed 的粗识别，使切换时机精确到"真顶部"，从而既吃涨又吃跌。

---

## 1. A 路严格形式：最高级别走势完成的精确切换

### 1.1 切换触发条件（替代当前 A'' 的 `view.t1sell[cc]`）

**当前实现（rec_engine.rs:865-878）的缺陷**：

```rust
let core_trend_done = match self.instances[cc].direction {
    Polarity::Long => cc < MAX_LEVEL && view.t1sell[cc],  // 单层 type1 卖点
    ...
};
```

`view.t1sell[cc]` 是 **cc 级别单层** 的 type1 卖点投影。在强牛中,cc 级别的 type1 卖点可能来自:
- (i) cc 级别**真顶部**背驰(应切换),或
- (ii) cc 级别**次级别**(cc−1)走势完成被投影到 cc 层(不应切换,强牛中途)。

单层判据无法区分 (i) 与 (ii) → cascade 误触发根因(勘察 cascade-mechanism 原因2)。

### 1.2 A 路严格形式:区间套级联完成判据

定义"最高级别 cc 走势完成"的严格条件为**区间套逐级收缩链全部成立**(缠论第27课 + 第37课趋势背驰):

```
TrendDone(cc) ⟺
  G(cc)  : cc 级别走势 ≥2 同向中枢 ∧ c 段创该级别新高/新低        [几何门,divergence.rs:166-205]
  ∧ F∧S(cc) : cc 级别 c 段力度衰减 ∧ 守沿(条件2)∧ 嵌套不膨胀(条件3)∧ c含≥2中枢(条件5)  [真背驰5条件]
  ∧ NestChain(cc) : 区间套级联确认 —— ∃ 收缩链 D_cc ⊃ D_{cc-1} ⊃ ... ⊃ D_a0,
                    每层 D_k 的背驰在 k 级别独立成立(非借力下层),
                    且范围严格收缩(D_{k-1} 时间窗 ⊂ D_k 时间窗)         [第27课区间套定理]
```

**与当前判据的本质区别**:
- 当前:`completed` = 单层 cc 背驰判定(divergence.rs 返回 Some)。
- A 路:`completed` = cc 背驰 **∧** 区间套链贯通到 a0。

**为什么区间套链能区分 (i)/(ii)**(勘察 interval-nesting 的可复用性分析):

| 场景 | cc 级别背驰 | 区间套链贯通 a0 | A 路判定 |
|------|-----------|----------------|---------|
| (i) 真顶部 | 成立 | **贯通**(cc 背驰被 cc-1,cc-2,...,a0 逐级背驰嵌套确认) | TrendDone=true → 切换 |
| (ii) 强牛中途次级别完成 | 误投影成立 | **不贯通**(cc-1 完成,但 cc 本级中枢链未收缩,链断在 cc 层) | TrendDone=false → **不切换** |

第27课原文(勘察引用 027 L15):**"低级别背驰是本级别背驰的必要条件而非充分条件"**。
逆否:cc 级别背驰成立 ⟹ 低级别必有完整收缩链;低级别有背驰 ⇏ cc 级别成立。
A 路用"链是否贯通"作为 cc 级别背驰的**充分性补全**——这正是 cascade 缺失的精度维度。

### 1.3 精确切换触发条件(代码层语义)

```
on_bar 末段(替换现 A'' 865-878):
  cc = highest_active()
  IF A_PATH_NESTED 启用 AND TrendDone(cc) 成立:
      记录 cc 走势完成方向 d_done = instances[cc].direction
      释放 anchor 死扣(见 §3)
      clear_all → reset_campaign         # 解死锁,复用现 546 号机制
      标记 pending_flip = (cc, flip_pol(d_done))   # 下一个反向 BSP enter
  # 不在本 bar 反向 enter(避免 545 做空陷阱),
  # 由下一个该级别反向 BSP 经核心级 enter 重建反向仓
```

切换 = **两阶段**(承袭 546 号死锁解 + 545 号避陷阱):
1. **确认阶段**:TrendDone(cc) 成立 → clear_all 到现金 + 武装 pending_flip。
2. **重建阶段**:下一个 cc 级反向 BSP → enter(cc, 反向)。

---

## 2. 强牛误触发能否避免(A 路核心赌注 —— 诚实标注风险)

### 2.1 可达性论证(基于现状勘察)

**正面依据**:
- 第27课区间套定理在原文层面**确实**给出真顶部的判据(逐级收缩链),精度严格高于 cascade 的单层 completed。
- 勘察 interval-nesting 确认 `pending_locate` 状态机(NestWin)+ `cascade_arm` 级联 + `Move.settled` 已实装自上而下级联逻辑,**可复用度 ★★★★**。
- 勘察 trend-completion 确认 divergence.rs:145-315 的 5 条件真背驰门(G·F∧S·M)已实装,提供链的**每一层节点**的背驰判据。

### 2.2 风险:识别精度可能仍不足(诚实标注)

**风险 1 — 区间套链坍缩(最严重)**:
勘察 interval-nesting + pcf 内存明确:**1min a0 上高层完整链罕见**(source 坍缩 91-96% 在 segment 层,重构后形式上降为 0% 但"真实数据产生高层链"未达成)。

后果:若 cc 级别(L3/L4)的区间套链**因数据稀缺无法贯通到 a0**,则:
- 保守解读(链不贯通 → TrendDone=false):**永不切换** → 退化为 anchor 死扣(只吃涨,跌段漏)。这是**安全失败**(不重蹈 cascade,但 A 路价值打折)。
- 激进解读(链贯通到某中间级别即算完成):**重新引入 cascade 的粗识别风险** → 可能重蹈覆辙。

**A 路必须选保守解读**(no-patch-mentality:不允许为了"看起来吃到跌幅"而放松链贯通要求)。这意味着 **A 路在数据上的有效域可能严格小于定义域**(formalization-validity-domain)。

**风险 2 — 链贯通的滞后性**:
区间套链要求**逐级背驰逐级确认**,每级背驰确认本身有滞后(勘察 trend-completion:确认滞后是级别阶梯函数,L4 跨年)。真顶部的链贯通可能在顶部之后数十~数千 bar 才完成 → 切换滞后 → 跌段前半段漏吃。这是**确认滞后税**,不是误翻空,但削减"吃跌幅"的幅度。

### 2.3 区分"真顶部" vs "强牛中途次级别完成"的具体判据

```
判据 D_TOP(cc):  # cc 级别走势是否真顶部
  必要条件 1(链贯通): NestChain(cc) 贯通 —— ∃ 完整收缩链 cc → cc-1 → ... → a0,
                     每层背驰独立成立(divergence.rs Some),范围严格收缩。
  必要条件 2(本级中枢链): cc 级别本身 ≥2 同向中枢且末中枢相对首中枢满足趋势方向收敛
                     (trend.rs:145-162 的 classify 返回 UpTrend/DownTrend,非 Consolidation)。
  必要条件 3(力度衰减在 cc 级别成立): cc 级别 c 段力度 < a 段力度(leg_strength,嵌套深度档 level≥1)。

  D_TOP(cc) = 必要条件1 ∧ 必要条件2 ∧ 必要条件3

  反例(强牛中途次级别完成): cc-1 完成(t1sell[cc-1]=true)但 cc 本级中枢链未收缩
                          ⟹ 必要条件2 在 cc 层失败 ⟹ D_TOP(cc)=false ⟹ 不切换。
```

**核心机制**:cascade 的错误是把 `emergent_top.completed`(返回 `last.level`,即次级别封装后的上级单元 level,生长中)误当顶部(勘察 cascade-mechanism 原因1:`last.level` vs `i` 归属混淆)。A 路用 `D_TOP(cc)` 强制要求 **cc 级别本身的中枢链收缩**(必要条件2),而非依赖次级别封装单元的 level 标记。这是**结构性修复**,不是补丁。

---

## 3. 与 anchor 的时间分离

### 3.1 冲突的精确形式

在"最高级别顶部"这一时刻:
- anchor 语义:**永不翻空**(死扣底仓,保住强牛收益)。
- A 切换语义:**顶部翻空**(吃跌幅)。

二者在**同一时刻**对**同一底仓**给出相反指令 → 直接冲突。

### 3.2 编排者方案:时间分离

冲突在**时间轴上消解**:
```
时间轴:  [大趋势进行中] ──────────────→ [走势完成确认点] ──────────────→
         anchor 死扣(永不翻空)          A 释放(底仓平掉 + 翻空)
         吃涨段                          吃跌段
```

- **死扣期**:`TrendDone(cc) = false`(区间套链未贯通) → anchor 死扣底仓,A 路休眠。强牛收益受保护(解踏空,保留 anchor 对的一半)。
- **释放期**:`TrendDone(cc) = true`(区间套链贯通确认真顶部) → anchor 解锁 → A 路接管:clear_all 释放底仓 + 武装 pending_flip 翻空。

**关键**:anchor 不是"永远死扣",是"**走势完成确认前**死扣"。死扣的有效期 = 区间套链未贯通的整个区间。这把 anchor 从"静态恒仓"重构为"**条件恒仓**"——条件 = `¬TrendDone(cc)`。

### 3.3 代码层实现(语义,不写代码)

引入 anchor 的**门控**而非独立 flag 簇:

```
状态:  instances[cc] 携带 anchor_locked: bool   # 该级别底仓是否处于死扣态

死扣期:  anchor_locked = true
         ⟹ route_bsp 中,cc 级别(核心级,nearest_active_parent=None 分支)的反向 BSP
            不触发 flip / clear_all(被 anchor_locked 拦截)
         ⟹ 但次级别 sink/recover 短差正常运行(吃次级别涨跌,不动 anchor 底仓)

释放点:  TrendDone(cc) 成立的那个 bar
         ⟹ anchor_locked = false
         ⟹ clear_all 释放底仓
         ⟹ 武装 pending_flip

释放后:  下一个 cc 反向 BSP → enter(cc, 反向) → 新的 anchor 在反向走势上重新死扣
         (对称:跌段也有 anchor,跌段走势完成确认前死扣空头底仓)
```

**与现有 clear_all 的关系**:clear_all(rec_engine.rs:501-512)逐层 reduce_at + reset_campaign 会清掉 anchor 底仓(勘察 cascade-mechanism §4)。A 路**复用** clear_all,但**前置 anchor_locked 门控**——只有 `¬anchor_locked`(已确认走势完成)才允许 clear_all 动 cc 级别核心仓。次级别 sink/recover 不受门控影响(它们本就不动核心底仓,只动 1/3 短差)。

---

## 4. 实装方案(rec_engine 具体改动点)

### 4.1 改动点(引用勘察行号)

| 改动 | 位置 | 当前 | A 路 |
|------|------|------|------|
| **新增链贯通判据** | divergence.rs(新函数 `nest_chain_complete`) | 单层 judge_divergence | 递归调用每级 judge_divergence,验证收缩链 |
| **A'' 替换** | rec_engine.rs:865-878 | `view.t1sell[cc]` 单层判据 | `D_TOP(cc)` = 链贯通 ∧ 本级中枢链 ∧ 力度衰减 |
| **anchor 门控** | rec_engine.rs route_bsp None 分支(746-762) | 核心级反向 BSP 直接 flip | 前置 `anchor_locked` 检查,死扣期不 flip |
| **anchor 状态字段** | rec_engine.rs:159-185 TInstance | 无 anchor 字段 | 加 `anchor_locked: bool` |
| **pending_flip 武装** | rec_engine.rs:865-878 释放点 | 现 546 号 clear+等下一买点 | 同机制,但触发改为 D_TOP |

### 4.2 新 flag 名与现有 flag 的关系

| flag | 关系 | 理由 |
|------|------|------|
| `A_PATH_NESTED`(新) | **替代** `enable_trend_done_clear`(现 T_CASCADE_FLIP 对应) | A'' 的 `view.t1sell[cc]` 单层判据被 `D_TOP(cc)` 链判据替代。no-patch-mentality:不保留旧单层判据作 fallback,直接重写。 |
| `anchor_locked`(新,实例字段) | **新增**(HOLD_ANCHOR 的条件化形态) | 现架构无 HOLD_ANCHOR(勘察 cascade-mechanism §4:"anchor 不在当前递归引擎中实现")。A 路把 anchor 实装为**条件门控**而非静态死扣。 |
| `emergence_upgrade`(594-610)方向门控 | **保留但剥离所指(b)** | 545 号:emergent_top 承载三所指,(b)方向锚已被否定。A 路把方向锚移交 `D_TOP` 区间套判据,emergence_upgrade 只保留所指(a)r* 上界 + (c)升级归属。 |

**T_CASCADE_FLIP 的处置**:cascade flip 的 `emergent_top.completed` 粗识别被 `D_TOP` 链识别**完全替代**。按 no-patch-mentality,cascade 的单层判据应**删除**,不保留为 fallback(否则又是兼容性垫片)。

---

## 5. 递归推广:完整对称双吃

### 5.1 从最高级别到每级别

A 路本体(§1-§4)是**最高级别**(cc)的精确切换。完整对称双吃要求**每级别**都骑自己的走势:

```
对每个级别 k(从 cc 到 a0):
  k 级别走势涨段 → k 实例持多
  k 级别走势完成(D_TOP(k) 成立,区间套链在 k 层贯通到 a0) → k 实例切换方向
  k 级别走势跌段 → k 实例持空
```

**架构上的可行性分析**:

| 维度 | 可行性 | 依据 |
|------|--------|------|
| 数据结构 | ✅ 已支持 | 勘察 trend-completion:每 level 一个 TInstance(rec_engine.rs:232),各骑独立 TrendNode |
| 每级别独立方向 | ✅ 已支持 | TInstance.direction 单一字段,sink/recover 已实现父多+子空异级别多腿 |
| 每级别 D_TOP 判据 | ✅ 可复用 | D_TOP(k) 对任意 k 同构(divergence 判据级别无关,leg_strength 自动切换嵌套档/振幅档) |
| 资金隔离 | ⚠️ 需设计 | 现 sink/recover 是父子资本转移(同资本 sizing),非每级别独立资金池。完整对称双吃需决定:是单一 free 池 + 每实例仓位隔离(现架构),还是每级别独立池。 |

### 5.2 sink/recover 与 A 路递归的关系(关键张力)

**现状**(勘察 ride-trend-intent):sink/recover 实现"次级别短差"——父级减 1/3,子级开**反向**短差,次级别完成 recover 升回。这是 **"父级骑大走势 + 子级吃次级别回调"** 的形态。

**完整对称双吃要求**:每级别骑**自己的走势**(涨持多/跌持空),不是"子级永远做与父级反向的短差"。

**张力**:
- sink/recover 的子级方向 = `flip_pol(父向)`(rec_engine.rs:620) —— 子级方向由父级决定,**不是由子级自己的走势决定**。
- 对称双吃要求子级方向 = **子级自己走势节点的方向**。

**这是一个真实的概念分叉,不应在本设计中 workaround**(no-workaround)。两种读法:
- **读法 A(短差嵌套)**:子级是父级走势内的回调短差,方向必反父向。吃的是"父级涨段中的次级别回调跌幅"。
- **读法 B(独立骑乘)**:子级骑自己的走势,方向由自己的走势节点决定。吃的是"子级别自身的涨跌幅"。

读法 A 是现 sink/recover 语义;读法 B 是编排者"每级别骑该级别走势"的字面。**当父级涨段中子级也涨**时,两读法冲突:读法 A 要子级做空(回调短差),读法 B 要子级做多(骑子级涨段)。

**升级处理**:A 路本体(最高级别)**不触及此张力**(最高级别无父级,直接骑自己走势)。递归推广时此张力必须先经 `/escalate` 裁决,**不在 A 路本体实装中预判**。A 路本体验证成功后,递归推广作为独立工位,带着这个明确的概念分叉上浮。

### 5.3 递归可行性结论

- **A 路本体(最高级别)**:架构完全可行,改动局部(§4.1 五处)。
- **递归到每级别**:数据结构可行,但**子级方向语义存在概念分叉**(读法 A vs B),必须先裁决再实装。**诚实标注:A 路本体成功 ≠ 完整对称双吃自动成立**。

---

## 6. L3 验证方案

### 6.1 回测设计(8 标的 × 3 模式 × 3 配置)

**对照组**:

| 配置 | anchor | A 切换 | 含义 |
|------|--------|--------|------|
| **OFF** | 无 | 无 | 基线(纯 sink/recover,= 现 main flat 递归化) |
| **ANCHOR** | 死扣(永不翻空) | 无 | 只吃涨(已验证 5/8 解踏空) |
| **A_PATH** | 条件死扣(D_TOP 释放) | 区间套级联切换 | 对称双吃(待验证) |

8 标的(BTC/CL/BRN/ES/QQQ/GC/DX/OKLO)× 3 模式(Structural/And/Or)× 3 配置 = 72 组。

### 6.2 判据(formalization-validity-domain:必须可证伪)

**判据 1 — 识别精度达标(A 路核心赌注)**:
```
强牛标的(BTC/OKLO/ES)上:  A_PATH 收益 ≈ ANCHOR 收益(不显著低于)
  ⟺ A 路在强牛中**没有误翻空**(anchor 收益保住)
  反之: A_PATH < ANCHOR 显著(如 OKLO +458→负) ⟹ 识别精度不足,重蹈 cascade,A 路否证。
```

**判据 2 — 吃到跌幅(对称双吃的价值)**:
```
震荡/熊段标的(CL/DX)上:  A_PATH 收益 > ANCHOR 收益
  ⟺ A 路在顶部后翻空吃到了跌幅(ANCHOR 漏吃的部分)
  per-level 短空 pnl > 0(吃到跌段而非失血)。
```

**判据 3 — per-level 多空 pnl 归因**:
```
逐级别拆 long_pnl / short_pnl(复用现 short_pnl_by_level: rec_engine.rs:684, drain 715):
  cc 级别 short_pnl > 0(顶部翻空吃到跌) ⟹ A 路机制成立
  cc 级别 short_pnl < 0(翻空失血) ⟹ 切换时机错(误翻空或滞后过大)
```

**判据 4 — 区间套链坍缩诊断(风险 1 实测)**:
```
记录每标的 D_TOP(cc) 触发次数 + 链贯通深度分布:
  若 cc=L3/L4 的链贯通次数 ≈ 0(链坍缩到 segment) ⟹ A 路退化为 ANCHOR(安全失败,只吃涨)
  这是 formalization-validity-domain 的有效域读数,不是 bug。
```

### 6.3 验证等级标注

- 本设计:**L0**(纯定义/架构推导,零信息增量)。
- 回测产出:**L3**(8 标的 × 3 模式真实数据交叉验证,可否证)。
- **否定性结果优先**:若判据 1 否证(强牛仍误翻空),则确认 A 路区间套精度不足,缩小有效域边界——这比确认性结果更有价值。

### 6.4 必须的诚实预声明(避免不可证伪陷阱,backtest_benchmark_falsifiability 内存)

- 全历史超长上行数据上"逊于 buy-hold"不可证伪,信息增量为零 → **判据 1/2 必须 per-segment 拆分**(牛段/熊段/震荡段分别看),不用全历史聚合。
- A_PATH vs ANCHOR 的差异若来自暴露差而非择时,需对照(暴露归一化或随机切换门控对照,P3 内存教训)。

---

## 结果包(六要素)

1. **结论**:A 路严格形式 = 用区间套级联(`D_TOP(cc)` = 链贯通 ∧ 本级中枢链 ∧ 力度衰减)替代 cascade 的单层 `emergent_top.completed`,实现最高级别走势完成的精确切换;anchor 重构为**条件死扣**(`¬TrendDone(cc)` 期死扣,确认后释放翻空),与 A 切换在时间轴上分离。实装局部(rec_engine.rs 五处),新 flag `A_PATH_NESTED` 替代 `enable_trend_done_clear`,`anchor_locked` 实例字段实装条件 HOLD_ANCHOR。

2. **定义依据**:第27课区间套定理("大级别转折点可通过不同级别背驰段逐级收缩范围确定",D_cc⊃D_{cc-1}⊃...⊃D_a0)+ 第27课 L15("低级别背驰是本级别背驰的必要条件而非充分条件",逆否给出真顶部链贯通要求)+ 第37课趋势背驰5条件(divergence.rs:166-315 的 G·F∧S 已实装)。cascade 的输入特征(单层 t1sell[cc] / emergent_top 返回 last.level)不满足"本级中枢链收缩"条件,故误识别。

3. **边界条件**:A 路结论翻转的条件——(i)若 cc=L3/L4 区间套链因 1min a0 数据稀缺**无法贯通到 a0**(勘察 pcf 坍缩 91-96%),A 路退化为 ANCHOR(安全失败,只吃涨,跌段漏);(ii)若放松链贯通要求(激进解读)以"看起来吃到跌幅",则重新引入 cascade 粗识别风险,A 路否证;(iii)若判据1实测强牛 A_PATH < ANCHOR 显著(如 OKLO 重蹈 +458→负),则识别精度不足,A 路本体否证。

4. **下游推论**:若 A 路本体(最高级别)成立,递归推广到每级别需先裁决子级方向语义的概念分叉(读法 A 短差嵌套 vs 读法 B 独立骑乘)——sink/recover 现语义(子级方向=flip_pol(父向))与"每级别骑自己走势"字面冲突,**完整对称双吃不随 A 路本体自动成立**。

5. **谱系引用**:545 号(emergent_top 三所指,方向锚(b)已被 §11.6 否定但未物理移除——A 路彻底移除(b),方向锚移交 D_TOP);546 号(僵尸核心死锁——A 路复用 clear+等下一买点的解锁机制);547 号(cascade 翻错级别,e_level>=cc 太松——A 路用本级中枢链收缩修复级别归属);539 号(根翻空有效域⊂非上行 regime——A 路 anchor 条件死扣是对此的直接回应)。本设计涉及"区间套定位"概念,谱系 interval_nesting_forward / located_direction_discovery 记录了自下而上(错)vs 自上而下(对)的分离,A 路采用自上而下(D_cc 先,链向下收缩)。

6. **影响声明**:本产出是设计文档,**不改任何代码**。若实装,将改动 rec_engine.rs(A'' 865-878 替换、route_bsp None 分支 746-762 加 anchor 门控、TInstance 加字段)+ divergence.rs(新增 nest_chain_complete 链判据);删除 `enable_trend_done_clear` 单层判据(no-patch:不留 fallback);剥离 emergence_upgrade(594-610)的方向锚所指(b)。影响 backtest 对照设计(新增 ANCHOR/A_PATH 两配置)。

---

**风险诚实总结**:A 路的核心赌注是"区间套链识别精度 > cascade 单层粗识别"。理论层面(第27课)精度更高;**但实证层面 1min a0 数据稀缺可能使高层链坍缩,导致 A 路退化为 ANCHOR(安全失败)而非达成对称双吃**。这是 formalization-validity-domain 的有效域 < 定义域风险,必须由 §6 的 L3 判据4(链坍缩诊断)实测裁定,不可在设计层声称已规避。