# 跨国 K4 闭合验证（Rust 引擎）

| 项 | 值 |
|----|----|
| 日期 | 2026-06-09 |
| 引擎 | `newchan_rust.RecursiveOrchestrator(max_levels=6, stroke_mode="wide")`（逐位等价 Python，1min 量级唯一可行） |
| 认识论等级 | **L2/L3**（真实多标的、多分辨率 1min/1h 数据，非合成，可产生否定性结果） |
| 数据 | US 1min（ES 852k / DBC 242k / VNQ 242k）+ 折叠通道 1min（GC 851k / CL 790k）+ 货币层 1h（EUR/JPY/CNH ~60k）+ EU/JP 生产边 1h（FESX/NKD）+ CN 本土 1min sina（IF/SC/AU9999，~1k bar 探索） |
| 总耗时 | 2.2s（13 条边，全 Rust） |
| driver | `analysis/cross_national_k4_verification.py` → `analysis/cross_national_k4_verification_data.json` |
| 管线 | `src/newchan/topology/cross_national_pipeline.py`（本次实装：递归引擎 Python→Rust） |

---

## 结论

跨国 K4 三层结构（254号定义3）在真实数据上的读数：

### 层 0 — 结算尺空间 Σ（货币边 M_i/M_US）

| 货币边 | σ（最高涌现级别走势方向） | move_kind | settled | max_level |
|--------|------|-----------|---------|-----------|
| EUR/USD | **UP**（美元兑欧元贬） | trend | False | 3 |
| JPY/USD | **UP**（美元兑日元贬） | trend | False | 2 |
| CNH/USD | **DOWN**（美元兑离岸人民币升） | trend | False | 2 |

**同步断裂候选（254号定理2）= 否**。判据：≥2 条货币边方向相同且均已结算趋势。
实际：EUR/JPY 同向（up）但 CNH 反向（down），且三条均 `settled=False`（move 未结算）。
→ 结算尺空间**未断裂**，方向分化（美元对欧日走弱、对人民币走强）。

### 层 1 — 各经济体截面 K4 配置 Γ_i=(σ_P, σ_C, σ_R)

| 经济体 | σ_P | σ_C | σ_R | 配置完整性 |
|--------|-----|-----|-----|-----------|
| **US** | **UP**(ES) | **FLAT**(DBC) | **FLAT**(VNQ) | **完整 ✓** Γ=(UP, FLAT, FLAT) |
| EU | FLAT(FESX) | — 缺口 | — 缺口 | 部分（仅生产边） |
| JP | UP(NKD) | — 缺口 | — 缺口 | 部分（仅生产边） |
| CN（探索） | DOWN(IF) | DOWN(SC) | — | L0/L1 短窗口，不可定论 |

### C 路径 — 全局共享折叠通道（292号 Au 全局共享）

| 通道 | 标的 | σ | last |
|------|------|---|------|
| Au = C↔M | GC | **UP** | 4353.9 |
| Oil = C→P | CL | **FLAT** | 92.97 |

ω = 金/油 = **46.83**，方向 **up**（金强于油）→ **信用收缩**读数（482号；仅长期方向，非实时门控——实时门控已证伪）。

### 闭合判定（三层级）

定义跨国 K4 拓扑闭合为递进的三层级：

| 层级 | 判据 | 结果 |
|------|------|------|
| **L-闭合①** 截面内闭合 | 经济体 K4 三独立边（P/M、C/M、R/M）齐全 → Configuration 完整 | **仅 US 成立**；EU/JP/CN 因 C/R 缺口不成立 |
| **L-闭合②** 层间连通 | 货币层（FX 边）连接各本币截面 + 全局折叠通道（Au/Oil）跨截面共享 | **成立 ✓**：3 条货币边连接 US↔EU/JP/CN，Au/Oil 一条序列锚定所有截面 |
| **L-闭合③** 完全跨国闭合 | 所有经济体截面都闭合 ∧ 层间连通 | **未达成（否定性结果）**：受 C/R 跨国数据缺口限制 |

**总判定**：跨国 K4 拓扑**骨架连通（L-闭合②成立）但非完全闭合（L-闭合③不成立）**。
只有 US 截面 K4 完整闭合；其余经济体为部分配置。这是诚实的 L2/L3 结论——
否定性结果缩小了有效域：当前真实数据下，**跨国闭合的有效域 = {US 截面完整 + 全局折叠/货币层连通}，
严格小于定义域 {四经济体全 K4 闭合}**（形式化有效域规则 222/223/230）。

---

## 1 定义依据

- **三层结构（254号定义3）**：层0 结算尺空间 Σ（货币边）、层1 截面内 K4、C路径折叠通道。
  输入数据满足：货币层有 3 条真实 FX 1h 序列（EUR/JPY/CNH 兑 USD）→ 层0 可读；
  US 有 P(ES)/C(DBC)/R(VNQ) 三条本币计价 1min 序列 → 层1 US 截面 Configuration 三分量齐全。
- **独立边 = 本币计价序列本身**（026号双重身份）：M（货币）是度量基准，故 σ(X/M) = 对本币 X
  序列跑递归取最高涌现级别走势方向，无需显式比价。US 的 ES/DBC/VNQ 均以 USD 计价 → 直接为 P/M、C/M、R/M。
- **走势方向 σ（缠论正典）**：σ 由最高涌现级别 move 的 (kind, direction) 决定——
  consolidation→FLAT，trend+up→UP，trend+down→DOWN。move 元组取自
  `newchan_rust` 的 `current_moves()`（`rust/src/lib.rs:469`，逐位等价 Python）。
- **折叠通道全局共享（292号§3.4）**：黄金是同一个黄金，一条 GC 序列锚定所有经济体的 C↔M 折叠；
  Oil 半共享 C→P。故 Au/Oil 作为跨经济体共享观测量，不在每经济体重复建模。
- **ω 信用读数（482号）**：ω=金/油方向 → 长期信用环境方向（金强=信用收缩），**非实时门控**（已证伪）。

## 2 边界条件（结论翻转条件）

1. **σ 读数随时间窗口翻转**：本验证用各序列**全量**（最高涌现级别 = 数据窗口内的级别）。
   换时段或截断窗口 → 最高级别走势可能翻转（记忆[最高级别σ短窗口冻结]：跨年 move 在单年窗口
   退化为常数）。US Γ=(UP,FLAT,FLAT) 是 2024-01..2026-06 全窗口读数，非永久态。
2. **L-闭合③ 翻转**：若补齐 EU/JP/CN 的 C（本币商品）/R（本币不动产）1min 源 → 完全跨国闭合可能成立。
   当前否定性结果**纯由数据缺口驱动**，非拓扑结构失败（254号 OQ2 预言：不动产高度本地化，无统一源）。
3. **层0 同步断裂候选翻转**：当前 settled=False。若 ≥2 条货币边转为已结算趋势且同向 → 断裂候选触发。
   注意单边事件无法区分分子/分母变化（254号），断裂仍是**候选**非确认。
4. **CN 探索读数不可定论**：IF/SC/AU9999 sina 源窗口仅 ~1 周（~1k bar），**不足以涌现最高级别**
   （L0/L1，信息增量≈零）。CN 截面的任何 σ 结论在补齐长序列前**不成立**。
5. **ω 方向粗粒度**：omega_direction 由 au.sigma 与 oil.sigma 的**枚举值比较**得出（UP>FLAT→up），
   非连续比价序列的递归。若对 GC/CL 比价序列直接跑递归，方向判定可能不同。

## 3 下游推论

- **US Γ=(UP, FLAT, FLAT)** 是 K4 配置空间（81 边图，527号）的一个节点：生产资本金融化上行、
  商品与不动产盘整。该节点可接入 K4 转换矩阵（记忆[K4配置转换矩阵真实结果]）作为当前 regime 锚。
- **ω up（金强于油，46.83）= 信用收缩 regime**（482号长期方向）。与用户押注方向相关：
  用户押注金油比下降（油涨）（记忆[用户交易方向]）——当前 regime（ω↑ 金强）与该押注**相反**，
  即押注是逆当前 σ 的反转下注，需结构反转信号（次级别背驰）确认入场，非顺势。
- **结算尺未断裂 + 方向分化**：美元对欧日走弱、对人民币走强 → 不是单一美元 regime，
  跨国资本流转（254号）呈分化态，无法用单一美元强弱叙事覆盖。
- **骨架连通可作跨国 C 路径基础**：Au/Oil 全局折叠通道 + 货币层已连通所有截面，
  即使各截面 K4 不完整，跨国 C 路径（折叠通道联合读数）仍可计算（292号区间套）。

## 4 谱系引用

- **254号** 多经济体资本流转（三层递归、结算尺空间、定理2 同步断裂）——本验证的结构定义来源。
- **292号** 折叠区间套 / Au 全局共享——折叠通道单序列锚定所有截面的依据。
- **482号** ω 信用读数（仅长期方向，实时门控已证伪）——C 路径信用信号的边界。
- **528/529号** 折叠通道重构（金/油降为折叠通道观测量）——Au/Oil 不作 K4 主轴顶点的依据。
- **527号** σ 为走势方向态、配置空间 81 边图——US Γ 作为配置节点的依据。
- **231号 / 形式化有效域规则** 222/223/230——有效域 < 定义域的诚实标注（数据缺口不伪造，
  CN 短窗口标 L0/L1，闭合③否定性结果缩小有效域边界）。
- 概念分离谱系：本验证未引入新的概念分离；US 截面 K4 顶点映射沿用正典（P/C/R 对 M=USD），
  **与非正典 GC 货币锚版（记忆[K4 1min期货映射分歧]）不同**，两者读数不可直接比较。

## 5 影响声明

- **改动 `src/newchan/topology/cross_national_pipeline.py`**：`_run_recursive` 的递归引擎从
  `newchan.orchestrator.recursive.RecursiveOrchestrator`（Python，O(N²) 流式，1min 量级不可行）
  替换为 `newchan_rust.RecursiveOrchestrator`（Rust，逐位等价，852k bar 0.6s）。
  连带移除未用 import（`Bar`/`datetime`/`field`/Python 编排器）与 `_parse_dt`（date_range 改由
  序列 dates 字符串切片，不再依赖时间戳）。**接口（公共函数签名、`EdgeReading`/`Configuration` 产物）不变**，
  下游 `run_cross_national_k4` / `compute_economy_k4` / `compute_currency_layer` /
  `compute_cross_national_c_path` 全部透明走 Rust。
- **新增 `analysis/cross_national_k4_verification.py`**（验证 driver）+
  `analysis/cross_national_k4_verification_data.json`（边读数 + 闭合判定数据）+ 本报告。
- **未改动任何缠论定义、谱系、K4 顶点映射**——纯引擎替换 + 验证产出。

---

## 附：完整边读数 + 复现

13 条边全 Rust，总 2.2s（ES 852k=0.62s / GC 851k=0.7s / CL 790k=0.5s — 引擎已增量化，
非 O(N²)，记忆[segment级增量化]）。

```bash
PYTHONPATH=src .venv/bin/python analysis/cross_national_k4_verification.py
```

逐边数据见 `analysis/cross_national_k4_verification_data.json` 的 `edges` 字段。
