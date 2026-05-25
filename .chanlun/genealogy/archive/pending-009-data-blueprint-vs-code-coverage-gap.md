---
id: pending-009-data-blueprint-vs-code-coverage-gap
timestamp: 2026-04-27
status: 已上浮-待编排者裁决
settlement: 部分（行动:logger修复完成；选择:数据层方向A/C + 命名/双栈/482-L2 上浮）
settled_date: 2026-05-24
settled_classification: 选择
settlement_scope: "gateway.py logger ordering 行动类已修(NameError实为误诊,晚绑定);数据缺口A/C(B被161号否)+命名+双栈+482-L2过度声明=选择/语法记录,上浮编排者"
type: domain
negation_source: homogeneous
negation_form: expansion
topo_effect: "split:blueprint_data_layer:downstream"
---

## 推进（2026-05-24，混合类型——行动部分结算 + 选择/语法记录上浮）

**行动类（已自主结算）——gateway.py logger 问题：**
- W-9 标注的 "L77/L99 NameError 运行时 bug" 经实查**是误诊**：`logger`(L99旧位) 仅在函数内（L77 异常处理器/L871/L874）使用 = **晚绑定**，调用时模块已加载完，`import newchan.gateway` 实测干净无 NameError。
- 但 ordering 确实脆弱（定义在首个使用点之后，骗过了 W-9）→ 已修：`logger = logging.getLogger(__name__)` 上移到 imports 后(L61)，所有使用点之前。import 验证通过。这是**真行动**（消除脆弱 + 误诊源），非补丁。

**选择/语法记录类（上浮编排者）：**
- **数据缺口 22:0:6:16 方向**（路径 A 建 provider 扬弃 / 路径 C 缩声明域标实验代码）——B(分阶段务实)被 **161号** 否定（务实=把缺口留后面）。A vs C 取决于逢亮是否要数据能力（资源裁决）。
- **隐式依赖**（yfinance/fredapi 不在 pyproject）= 090号违反——其修法即 A(加依赖) 或 C(标实验)，归入上述选择。
- **482号 L2 过度声明**：482 声称 L2 但管线在 `tmp/exp_macro_verification.py` 一次性脚本 = 231号反向缺口（L2标注 > 工程严格性）。修法（降级标注 or 建可重复管线）也归入 A/C 选择。**注：此项触及已结算的 482号，需编排者确认是否重开 482 修正其 L2 标注。**
- **gateway 三重命名碰撞**（gateway.py/409 channel adapter/IBKR Gateway）= 语法记录（命名归并决断）。
- **双栈并存**（bottle 8765 / FastAPI 8766 同名 `/api/live/status`）= 迁移路径决断（选择）。

**蜂群侧工作已完成**（行动修复 + 误诊纠正 + 5 项选择/语法记录厘清），移 archive 等编排者裁决。已纳入向用户待裁清单第 3 项（数据投入）并新增 482-L2/命名/双栈子项。

---

# 蓝图数据需求 vs 代码覆盖——22:0:6:16 全面缺口

## 矛盾

W-9 自标：蓝图 v3 列出 22 项数据需求，代码生产管线（`src/newchan/data_*.py`）支持情况：
- 完整支持：**0 项**
- 半支持（一次性脚本/部分覆盖）：6 项
- 完全缺失：16 项

包括：
- 和乐序列计算（蓝图圈2核心）→ 完全缺失
- OVX → 完全缺失
- 25-delta RR（skew）→ 完全缺失
- M1-M2 价差 → 半支持（仅 M2，无 M1）
- GLD/USO（=ω）→ 关键缺失（482号 L=M×ω 在生产管线断裂）
- EIA 库存 / CFTC 持仓 / TIC 外资持债 / WGC 央行黄金 / EPFR 资金流 / H.4.1 联储资产负债表 / SOFR/ON RRP / MBS 利差 / CLO 指数 / PE/VC 规模 / 加密总市值 / 稳定币流通量 / 可转债规模 → 全部完全缺失

附加问题：
1. **gateway 三重命名碰撞**：`src/newchan/gateway.py`（FastAPI 数据网关）vs 409号 channel adapter gateway（在 `topological-computation/`）vs IBKR Gateway（端口 4001/4002 桌面应用）
2. **gateway.py L77/L99 NameError 运行时 bug**（W-9 详标）
3. **隐式依赖未声明**：`yfinance`/`fredapi` 不在 pyproject.toml，但 scripts/tmp 直接 import → 声明—能力一致性违反（090号实例）
4. **server.py（8765 bottle）+ gateway.py（8766 FastAPI）双栈并存**且 `/api/live/status` 同名端点

## 否定了什么

否定的是"蓝图 v3 数据具体化层 = 已落地"的隐含假设。和 pending-003（圈4 缺口）+ pending-007（圈1 vs 330）+ pending-008（圈6 命名错误）一同构成蓝图工程化的结构性距离——CROSS-LINKS 聚集点 2 的最广泛证据。

同时否定了 482号 L2 验证的工程严格性：482号声称 L2（金油比与 GDP 协整 p=0.033），但其管线**完全在 `tmp/exp_macro_verification.py`**——一次性脚本，不是可重复生产管线。L2 标注与工程严格性不匹配（231号有效域规则的反向缺口）。

## 推导链

- 蓝图 v3 数据具体化层：22 项需求
- W-9 实测：5 个 data_*.py 文件 1480 行——只有"价格 OHLCV 流"
- pyproject.toml 声明 `databento/akshare/ccxt/ib_insync`——不含 yfinance/fredapi
- scripts/k4_monitor.py + tmp/exp_macro_verification.py 直接 `import yfinance` / 直接 HTTP fetch FRED CSV
- 482号 L2 验证依赖隐式管线 → 与 231号"L2 = 真实数据假设检验"标注的工程严格性不匹配
- 484号 multi_tf 链路缺口（W-9 自标）：`align_bars_by_timestamp` 已实装但 nested_divergence 未接入

## 谱系链接

- 482号、484号、485号（W-3/W-7 拓扑底座）
- 219/220号（资本三流/卢麒元——蓝图理论基础）
- 231号（有效域规则——L0/L1/L2/L3 标注）
- 519号（v3 蓝图命名错误）
- 090号（声明—能力一致性）
- 409号（channel adapter——gateway 命名碰撞源头）
- 016号（规则没有代码强制）

## 影响声明

- 影响：蓝图 v3 数据层、`pyproject.toml`、`src/newchan/data_*.py`、`src/newchan/server.py`、`src/newchan/gateway.py`、`scripts/k4_monitor.py`、`tmp/exp_macro_verification.py`
- 改动方向（待编排者裁决）：
  - 路径 A：新增 4 类 provider（FRED-macro / CFTC / EIA / EPFR-WGC）+ 和乐计算管线 → 工程化达 ~12 项
  - 路径 B：承认蓝图 22 项是长期愿景，分阶段落地（明确划分 must/should/nice）
  - 路径 C：把 scripts/tmp 标注为"实验代码不构成项目能力"——缩小声明域
- 路径 A 是严格扬弃，路径 C 是严格收缩，路径 B 是务实（161号否定边缘）

## 同步处理（次级矛盾）

- gateway 三重命名碰撞：W-9 已建议高层归并 → 单独 issue
- gateway.py L77/L99 NameError → 工程类 bug，直接修复（行动类）
- 双栈并存（bottle 8765 / FastAPI 8766）：明确迁移路径或合法化共存

## 异质审计降级

本 session 全程 gemini-challenger 不可用。
