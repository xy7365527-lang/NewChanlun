# unn × NautilusTrader 真流式回测 — 八资产完整结果

> 生成日期：2026-06-14
> 脚本：`trading_system/backtest_unn_stream.py`（逐 bar `process_bar→push_bar`）
> 标的：OKLO, QQQ, GC, CL, BTC, BRN, DX, ES（全量 1min K线）
> 认识论等级：**L1**（bit-exact = 管线一致性验证）+ **L2**（N1-N8 必然性在真实数据上零 panic）

---

## 1. 结论：八资产流式回测结果

| 标的 | bars | BH% | unn strat% | P1(≥BH) | MDD% | BH_MDD% | trades | max_kids | roots | spawns | bit-exact |
|------|-----:|----:|-----------:|:-------:|-----:|--------:|-------:|---------:|------:|-------:|:---------:|
| OKLO |   447,738 |  +307.1 |   +15.5 | ✗ | -55.9 | -76.8 |  277 | 4 | 10 |  260 | **PASS** |
| QQQ  |   728,030 |  +174.6 |  +127.1 | ✗ | -22.8 | -25.6 |   89 | 2 |  5 |   34 | **PASS** |
| GC   | 5,544,556 |  +257.3 |  +213.2 | ✗ | -21.1 | -45.6 |  306 | 3 | 52 |   81 | **PASS** |
| CL   | 5,528,156 |   +28.2 |  +132.4 | **✓** | -55.7 | -94.0 | 2,259 | 7 | 37 | 2,049 | **PASS** |
| BTC  | 4,625,119 | +1380.4 |  +954.9 | ✗ | -71.8 | -84.0 |  581 | 4 | 27 |  536 | **PASS** |
| BRN  | 2,418,058 |   +87.4 |  +270.9 | **✓** | -44.2 | -78.8 | 1,085 | 6 | 12 |  993 | **PASS** |
| DX   | 2,046,008 |    +4.1 |    +3.4 | ✗ | -15.3 | -16.8 |   78 | 1 | 23 |    0 | **PASS** |
| ES   | 5,589,928 |  +594.3 |  +349.4 | ✗ | -35.7 | -36.0 |  408 | 4 | 12 |  228 | **PASS** |

**汇总**：8 标的 / 26,927,593 bar / **bit-exact 8/8 PASS** / **N1-N8 零 panic** / **P1 = 2/8 = {CL, BRN}**

- **MDD 8/8 全优 BH_MDD**：unn 回撤逐标的浅于买入持有（最显著 CL -55.7% vs -94.0%、BRN -44.2% vs -78.8%）。
- **N1 森林深度**（max_kids）：CL=7（最深，强 churn 油链）、BRN=6、OKLO/BTC/ES=4，DX=1（无 spawn，单根树）。

---

## 2. bit-exact 验证（L1 管线一致性，双重）

每个标的同时通过两条独立的 bit-exact 检查：

### 2a. stream vs batch（脚本内置，逐键 ==）

NT `BacktestEngine` 逐 bar 回放 → `on_bar(StreamingSignalReader.process_bar → UnnStream.push_bar)` → `on_stop(finish)` 的流式结果，与同数据 `compute_organic_signals → run_positional_rust(mode="unn")` 批量结果**逐键 ==**（8 键全部一致，8/8 PASS）。

证明：NautilusTrader 逐 bar 驱动的流式引擎与批量引擎产出**逐位相同**的结果。两层（信号层 `process_bar`／引擎层 `UnnStreamCore::step`）与各自批量路径共享同一段循环体代码 ⇒ bit-exact 由构造保证。

### 2b. NT 流式 vs 独立 analysis 基线（外部交叉核对）

NT 流式 strat% 与 `analysis/data_cache/unn_<SYM>.json`（独立的 `analysis/unified_necessity_backtest.py` 基线）**8/8 逐字一致**：

| 标的 | NT 流式 strat% | 独立基线 strat% | 一致 |
|------|---------------:|----------------:|:----:|
| OKLO |  +15.5 |  +15.5 | ✓ |
| QQQ  | +127.1 | +127.1 | ✓ |
| GC   | +213.2 | +213.2 | ✓ |
| CL   | +132.4 | +132.4 | ✓ |
| BTC  | +954.9 | +954.9 | ✓ |
| BRN  | +270.9 | +270.9 | ✓ |
| DX   |   +3.4 |   +3.4 | ✓ |
| ES   | +349.4 | +349.4 | ✓ |

两条独立管线（NT 托管流式 + 独立批量脚本）产出同一数字 ⇒ NT 托管不引入任何数值偏差。

---

## 3. N1-N8 必然性验证（L2）

引擎内 8 个 prove 函数（N1 森林会计 / N2-N8 概念运动链）**逐 bar 检查**，violation = panic。8 标的 26.9M bar 全程**零 panic** ⇒ N1-N8 在逐 bar 驱动下、在 8 个真实标的上同样成立（与批量路径 L2 结论一致，新增"逐 bar 驱动"维度的鲁棒性）。

---

## 4. vs URS 基线对比

URS 基线取自 `analysis/data_cache/urs_<SYM>.json`（同 settle on + 仅 dir_flips 口径，唯一变量 = 引擎层选择机制）。

| 标的 | URS strat% | unn strat% | Δ(unn-URS) pp | URS P1 | unn P1 |
|------|-----------:|-----------:|--------------:|:------:|:------:|
| OKLO | +321.9 |  +15.5 | **-306.4** | ✓ | ✗ |
| QQQ  | +166.2 | +127.1 |  -39.1 | ✗ | ✗ |
| GC   | +245.0 | +213.2 |  -31.8 | ✗ | ✗ |
| CL   |   +4.7 | +132.4 | **+127.7** | ✗ | ✓ |
| BTC  | +549.9 | +954.9 | **+405.0** | ✗ | ✗ |
| BRN  |  +56.5 | +270.9 | **+214.4** | ✗ | ✓ |
| DX   |   +4.4 |   +3.4 |   -1.0 | ✓ | ✗ |
| ES   | +488.4 | +349.4 | -139.0 | ✗ | ✗ |

- **unn > URS：3/8 = {CL, BTC, BRN}**（强趋势油链/加密 regime——unn 的 N1 森林 spawn 机制在此域捕获 URS 单根树错失的 alpha：CL spawns=2049、BRN=993、BTC=536）。
- **URS 实测 P1 = 2/8 = {OKLO, DX}**，**unn P1 = 2/8 = {CL, BRN}**——**两者域不相交**（不同 regime：URS 在弱趋势/区间标的占优，unn 在强趋势标的占优）。

> ⚠️ **诚实声明（formalization-validity-domain.md 禁止声明膨胀）**：任务描述中"URS 5/8 基线"与本次实测 URS P1=2/8 **不符**。"5/8" 不是 URS 的 unn-baseline P1 计数——可能指 535 号谱系中 `fusion_tr` 在册白名单的"5/8 不劣"判据（不同形态、不同对比对象）。本报告只报告**实测值**，不照搬任务文字中的"5/8"。

---

## 5. 边界条件（数据格式 vs NT Price/Quantity 精度）

任务约束："如某标的数据格式不匹配 NT Price 精度，要报告。" 本次发现并修复**两类**精度边界条件：

### 5a. price_precision（全量扫描精确值，bit-exact 前提）

stream 路径经 NT `Bar→Price(px, price_precision)` 取整，batch 路径用裸 float。bit-exact ⟺ `price_precision ≥ 数据实际小数位`。各标的 `price_precision` 按**全量扫描的精确最小小数位**注册（numpy `round(a,k)==a` 全量判定）：

| 标的 | 全量精确小数位 | 注册 price_precision | 备注 |
|------|:--:|:--:|------|
| OKLO | 4 | 4 | 拆股复权后 4 位 |
| QQQ  | 3 | 3 | |
| GC   | 1 | 1 | 黄金 tick 0.1 |
| CL   | 2 | 2 | （原已正确） |
| **BTC** | **3** | **3（原=1，已修）** | **回测数据=Binance 归档 3 位小数；旧值 1 会令 NT 取整 63085.99→63086.0 破坏 bit-exact** |
| BRN  | 2 | 2 | |
| DX   | 3 | 3 | 美元指数 tick 0.005 |
| ES   | 2 | 2 | tick 0.25 |

> **BTC 精度边界条件（重点）**：`InstrumentSpec.price_precision` 是**回测精度**（`make_instrument` 仅服务回测；实盘 instrument definitions 由 adapter 从 venue 拉取，HL tick 可能 ≠ 3，二者不混表）。旧注册值 1 反映了对 HL 实盘 tick 的假设，**对这份 Binance 归档回测数据是错的**——已严格修正为 3（按数据精度，非补丁）。

### 5b. size_precision（volume Quantity 精度，NT Bar 不变量）

NT `Bar` 要求 `bar.volume.precision == instrument.size_precision`。`bars_from_dataframe` 原默认 `size_precision=0`，与期货/股票 `lot_size`（精度 0）匹配，但 BTC `CryptoPerpetual` `size_precision=5`（HL szDecimals）→ 报错 `invalid bar.volume.precision=0 did not match instrument.size_precision=5`。

**严格修复**：把 `instrument.size_precision` 贯通 `load_bars → bars_from_dataframe`（volume=0 不被 unn 消费，仅满足 NT 装配的结构性不变量——忠实于 instrument 定义，非削 BTC szDecimals 就 0）。

---

## 6. 改动声明

| 文件 | 改动 |
|------|------|
| `trading_system/config/instruments.py` | `InstrumentSpec` 加 `asset_type/multiplier/exchange/asset_class` 字段；注册 OKLO/QQQ/GC/BRN/DX/ES（精度按全量扫描）；**BTC price_precision 1→3**；`make_instrument` 按 asset_type 分派；`_glbx_future`→通用 `_future_contract` + 新增 `_equity` |
| `trading_system/backtest_unn.py` | `load_bars` 加 `size_precision` 参数并贯通 `bars_from_dataframe`；main 调用传 `instrument.size_precision` |
| `trading_system/backtest_unn_stream.py` | main 调用 `load_bars` 传 `instrument.size_precision` |

---

## 结果包六要素

1. **结论**：8 标的 unn × NautilusTrader 真流式回测全部跑通，bit-exact 8/8 PASS（双重：stream-vs-batch + stream-vs-独立基线），N1-N8 零 panic（26.9M bar，L2），P1=2/8={CL,BRN}，MDD 8/8 全优 BH。
2. **定义依据**：unn = 8 条概念运动链必然性叠加（`analysis/unified_necessity_backtest.py`）；P1 = unn strat% ≥ BH%（有效域读数，非验收标准——必然性检验才是）；bit-exact = stream `finish()` 与 batch `run_positional_rust(unn)` 逐键 ==。
3. **边界条件**：bit-exact 在 `price_precision < 数据小数位` 时翻转（NT 取整改数）——故按全量扫描精确精度注册；BTC volume 在 `size_precision` 不匹配 instrument 时 NT 直接报错（已贯通修复）。若数据文件重拉导致小数位变化，精度注册须重新全量扫描。
4. **下游推论**：NT 真流式路径已对全部 8 在册标的 bit-exact ⇒ 阶段4 实盘可在 NT 框架下逐 bar 驱动 unn 而不改变回测数字；unn 与 URS 域不相交（regime 函数）⇒ 部署须按标的 regime 选引擎，非全局单一引擎。
5. **谱系引用**：535 号（osc 相位词汇耦合，`fusion_tr` 5/8 在册判据——与本报告 URS 5/8 文字辨析相关）；formalization-validity-domain.md（L1/L2 标注 + 声明膨胀禁止）；no-patch-mentality.md（BTC 精度严格修正 vs 补丁）。
6. **影响声明**：改动 `instruments.py`（注册表 + 构造器）、`backtest_unn.py`（load_bars 签名）、`backtest_unn_stream.py`（调用）；新增 8 个 `trading_system/data_cache/unn_stream_<SYM>.json`；本报告。不改 Rust 引擎，不执行实盘交易。
