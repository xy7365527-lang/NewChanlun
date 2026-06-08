# IBKR 量化交易系统架构

> 缠论引擎 (NewChan) + Interactive Brokers 接入的完整量化交易系统设计

## 1. IBKR Quant 平台全景

> 来源：[IBKR Quant Home](https://www.interactivebrokers.com/campus/category/ibkr-quant-news/ibkr-quant-home/)（Playwright 实时抓取 2026-06-04）

IBKR Quant 是 Interactive Brokers 面向量化交易者的官方门户，定位为"Quantitative finance articles and programming tutorials"。页面自述：**"The IBKR Quant Blog serves quantitative professionals who have an interest in programming. Discussion topics include deep learning, IBKR API, artificial intelligence (AI), Python, R, C#, Java and more."**

### 页面结构与导航

**顶部导航六大板块：**

| 板块 | 内容 | 与我们的相关度 |
|------|------|--------------|
| **IBKR Quant Home** | 量化文章聚合首页 | 参考 |
| **Data Science** | 数据科学文章（XGBoost、Markov模型、因子分析等） | 中 |
| **IBKR API** | API 开发文章（Sync Wrapper、并发、Python API 等） | **高** |
| **Quant Development** | 量化开发教程（回测、算法交易、数据基础设施） | **高** |
| **Conferences** | 量化会议信息 | 低 |
| **Languages** | 编程语言专区（未展开） | 低 |

**七大主题分类（Quant Topics）：**

1. **Data Science** — 数据科学（因子模型、ML、时间序列）
2. **IBKR API Development** — API 开发（TWS API、Client Portal API）
3. **Programming Languages** — 编程语言（Python、R、C#、Java、Julia）
4. **Quant Development** — 量化开发（回测、策略构建、交易系统）
5. **Quant Options** — 量化期权
6. **Quant Podcasts** — 量化播客
7. **Quant Regions** — 区域量化

### 内容生态：贡献者与最新文章

IBKR Quant 采用**开放贡献者模式**——文章由外部量化机构/个人撰写，IBKR 平台发布。主要贡献者：

| 贡献者 | 类型 | 典型内容 |
|--------|------|---------|
| **Alpha Architect** | 量化研究机构 | 因子投资、动量策略、PE |
| **PyQuant News** | Python 量化社区 | 债券凸性、跟踪误差、AI 研究 |
| **DataGeeek** | 数据科学博主 | XGBoost 建模、ADAM 季节性模型 |
| **IBridgePy** | IB API 封装库 | Python 回测教程、IB API 功能介绍 |
| **QuantInsti** | 量化教育平台 | 算法交易数据基础设施 |
| **Vetta Research** | 地缘政治量化 | 地缘政治碎片化交易框架 |
| **Robot Wealth** | 量化交易社区 | 实战经验分享 |
| **Visual Sectors** | AI 交易研究 | LLM 交易系统对市场波动性影响 |
| **Interactive Brokers** | 官方 | Sync Wrapper、Quant Blog 月度精选 |

**最新文章（截至 2026-06-04）：**

| 日期 | 标题 | 作者 | 与我们的关联 |
|------|------|------|------------|
| 06-03 | When Everyone Trades the Same Factor Playbook | Alpha Architect | 策略拥挤度研究 |
| 06-02 | Automated AI Equity Research with LlamaIndex | PyQuant News | AI+量化参考 |
| 05-29 | IBKR Quant Blog Highlights – May 2026 | IB 官方 | 月度精选 |
| 05-28 | Global Modeling with XGBoost: Gold vs. Silver | DataGeeek | **金银建模——与我们金油比研究直接相关** |
| 05-18 | The Data Infrastructure Behind Algorithmic Trading | QuantInsti | **数据基础设施——架构参考** |
| 05-15 | Backtesting Trade Python: How to Perform | IBridgePy | **Python 回测——工具链参考** |
| 05-08 | Backtest Trading: Step-by-Step Guide | IBridgePy | **回测指南** |
| 05-07 | Can LLM-Based Trading Systems Increase Market Volatility? | Visual Sectors | LLM 交易风险 |

### 官方 API 资源（右侧栏）

**API 培训课程（Traders' Academy）：**

1. **Python TWS API** — Python 开发者的 TWS API 入门课程
2. **Trading Using R** — R 语言交易课程
3. **Excel and the TWS API** — Excel 集成
4. **IBKR's Client Portal API** — Web API 课程

**API 文档体系：**

| 文档 | 说明 |
|------|------|
| Trader Workstation (TWS) API | **核心——我们的主要接口** |
| Client Portal API | REST/WebSocket 接口 |
| Excel ActiveX / DDE / RTD | Excel 集成（三种方式） |
| FIX | 机构级 FIX 协议 |
| Third Party Connections | 第三方连接（QuantConnect、TradingView 等） |
| Contracts | 合约定义文档 |
| Market Data Subscriptions | 市场数据订阅管理 |
| Order Types | 订单类型完整列表 |

**官方 API 团队重点推荐**（侧栏置顶）：**"The New Synchronous Wrapper for TWS API"**——这是 IBKR API 团队当前最推的新特性。

### 五个官方 Newsletter

| Newsletter | 频率 | 说明 |
|------------|------|------|
| IBKR Campus | 双周 | 平台综合更新 |
| Traders' Insight | 每日 | 市场新闻 |
| IBKR Webinars | 每周 | 网络研讨会 |
| **IBKR Quant** | **每周** | **量化开发者专属——"For those wanting to trade markets using computer-power"** |
| **IBKR API** | **每月** | **API 团队专属——"Hear about the latest tools and techniques from our own IBKR API staff"** |

**建议订阅 IBKR Quant（周刊）和 IBKR API（月刊）两个 Newsletter，跟踪 API 更新和量化社区动态。**

### 社交媒体

- Facebook: [IBKRQuant](https://www.facebook.com/IBKRQuant)
- LinkedIn: [ib-quant](https://www.linkedin.com/showcase/ib-quant)
- X/Twitter: [@IBKR_QB](https://twitter.com/IBKR_QB)

---

### 对我们的启示

1. **IBKR 官方生态以教育和社区为核心**——不是闭源 SaaS，而是开放的 API + 教育文章 + 第三方集成
2. **Python TWS API 是官方首推的量化开发路径**——有专门的课程、文档、Sync Wrapper
3. **回测生态**：IBridgePy（Python IB 封装）、QuantConnect（云回测+实盘）、Backtrader（开源框架）
4. **数据科学与 AI 是当前热点**——LlamaIndex、XGBoost、Markov 模型、LLM 交易系统的文章密集出现
5. **IBridgePy** 是 IB 官方推荐的第三方 Python 库之一，提供了更简洁的 API 封装，值得关注

### 1.1 三大 API 体系对比

| 维度 | TWS API (Socket) | Web API (REST/WS) | Lightspeed Connect |
|------|-------------------|--------------------|--------------------|
| 协议 | TCP Socket（二进制消息） | HTTPS REST + WebSocket | **纯 WebSocket + JSON** |
| 连接方式 | 需本地 TWS 或 IB Gateway | Client Portal Gateway 或 OAuth | **直连云端，无本地进程** |
| 延迟 | 低（局域网直连） | 较高（经 IBKR 服务器中转） | 低（WSS 直连） |
| 速率限制 | 50 msg/s（入站） | 10 req/s；违规 429+封禁 | 未公开（WebSocket 流式） |
| 认证 | TWS/Gateway 登录 | 2FA + OAuth 2.0 / SSO | API Key（Bearer Token） |
| 市场数据 | **完整**（Level I/II/历史） | 有（快照+WebSocket流） | **无**（需另接 polygon.io 等） |
| 每日重启 | **有**（Gateway 每天重启） | 无 | **无** |
| 7天重认证 | **有**（需手机 2FA） | 有 | **无**（API Key 长期有效） |
| Java 依赖 | **需要**（TWS/Gateway 是 Java） | 需要（CP Gateway 是 Java） | **不需要** |
| 订单类型 | 全量（100+ 种） | 全量 | 标准（LMT/MKT/STP/Bracket/多腿期权） |
| 最低费用 | 无 | 无 | **$200/月佣金** |
| Python 库 | `ib_insync` / `ib_async` | `ibind` | `ws_adapter`（官方 Python 适配器） |
| 适用场景 | **全功能量化系统** | 轻量级 Web 集成 | **纯执行层（无数据需求时）** |

### 1.2 推荐方案

**我们采用 TWS API + ib_insync/ib_async 作为主方案。** 理由：

1. 已有 `data_ibkr.py` 基于 ib_insync，迁移成本最低
2. 完整的市场数据能力（历史+实时+Level II），缠论引擎需要
3. 无额外月费
4. 社区成熟，文档丰富

**Lightspeed Connect 作为备选执行层**——如果未来需要 headless 云部署、消除每日重启/7天认证痛点，可以将执行层（下单）切换到 Lightspeed Connect，数据层仍用 TWS API。

### 1.2 TWS API 架构

```
┌─────────────────┐     TCP Socket      ┌──────────────────────┐
│   Python 客户端   │ ◄──────────────────► │  IB Gateway / TWS    │
│  (ib_insync)     │    127.0.0.1:4002   │  (本地 Java 进程)     │
└─────────────────┘                      └──────────┬───────────┘
                                                     │ 加密连接
                                                     ▼
                                          ┌──────────────────────┐
                                          │  IBKR 服务器集群       │
                                          │  (交易所路由/撮合)     │
                                          └──────────────────────┘
```

**端口约定：**

| 环境 | TWS 端口 | IB Gateway 端口 |
|------|----------|-----------------|
| 纸盘 (Paper) | 7497 | 4002 |
| 实盘 (Live) | 7496 | 4001 |

**推荐生产部署用 IB Gateway**——无 GUI、资源占用低、支持 headless 运行。

### 1.3 TWS API 版本与新特性（2025-2026）

| 版本 | 发布时间 | 关键特性 |
|------|---------|---------|
| 10.30 | 2025-03 | 最低支持版本（低版本被强制升级） |
| 10.40 | 2025-09 | **Sync Wrapper（Beta）**、Protobuf 全面支持、`reqCurrentTimeInMillis()`、自动重连后订单恢复 |
| 10.42 | 2025-12 | 增强 Python 绑定（ML 集成优化）、WebSocket 轮询速度 +30%、Protobuf 修复 |
| 10.44 | 2026-02 | `tickSize` 返回 Decimal 类型（兼容性变更）、Order 新增 `Submitter` 字段 |

**Sync Wrapper（10.40+ Beta）**：官方首次提供同步风格 Python API，合并 EClient/EWrapper 为单一接口。示例在 `{TWS API}/samples/Python/Testbed/sync_test.py`。但仍为 Beta，功能不全——生产系统仍推荐 ib_insync/ib_async。

**全局配置新选项**（10.40+）：`Maintain and resubmit orders when connection is restored`——Gateway 断线重连后自动恢复/重提交订单。

### 1.4 ib_insync 库状态

- **ib_insync**：原作者 Ewald de Wit 2024 年初去世，仓库不再维护
- **ib_async**：社区 fork，API 完全兼容，生产就绪，`pip install ib_async`
- 当前项目 `data_ibkr.py` 使用 `ib_insync`，后续应迁移到 `ib_async`
- IB 官方不支持第三方库——出问题只能靠社区

### 1.4 速率限制与约束

| 限制类型 | 具体数值 |
|---------|---------|
| 消息速率 | 50 msg/s（包括下单） |
| 活跃订单 | 每合约每方向每账户 20 单 |
| 市场数据线 | 默认 100 条（可购买 booster） |
| 历史数据 pacing | 同一合约 2s 内不超过 6 次；10 分钟内不超过 60 次 |
| BID_ASK 历史数据 | 每次请求计数为 2 次 |
| 每日重启 | TWS/Gateway 每天自动重启一次 |
| 重新登录 | 每 7 天需人工重新认证（2FA） |

### 1.6 TWS API 核心编程模型（EClient/EWrapper）

TWS API 是事件驱动的异步架构，基于两个核心类：

- **EClient**：所有出站请求（reqMktData, placeOrder, reqHistoricalData 等）
- **EWrapper**：所有入站回调（tickPrice, orderStatus, historicalData 等）

```python
# 原生 API 模式（较繁琐）
from ibapi.client import EClient
from ibapi.wrapper import EWrapper

class IBApp(EWrapper, EClient):
    def __init__(self):
        EClient.__init__(self, self)
    
    def nextValidId(self, orderId):
        # 连接确认 + 获取下一个可用订单 ID
        self.nextOrderId = orderId
    
    def tickPrice(self, reqId, tickType, price, attrib):
        # 实时行情回调（bid=1, ask=2, last=4）
        ...
    
    def orderStatus(self, orderId, status, filled, remaining, avgFillPrice, ...):
        # 订单状态回调
        ...

app = IBApp()
app.connect("127.0.0.1", 7497, clientId=1)
threading.Thread(target=app.run, daemon=True).start()
```

```python
# ib_insync 模式（推荐——我们使用这种）
from ib_insync import IB, Stock, LimitOrder

ib = IB()
ib.connect('127.0.0.1', 7497, clientId=1)

# 同步风格，自动处理回调
contract = Stock('AAPL', 'SMART', 'USD')
ib.qualifyContracts(contract)
ticker = ib.reqMktData(contract)
ib.sleep(2)
print(ticker.bid, ticker.ask, ticker.last)
```

### 1.7 Lightspeed Connect API 技术详解

Lightspeed Connect 是 IBKR 合作伙伴 Lightspeed 提供的独立交易 API。核心特点：**纯 WebSocket + JSON，无需本地运行任何软件。**

**连接端点：**

| 环境 | WebSocket URL |
|------|---------------|
| 认证测试 | `wss://onboarding.connecttrade.com:26553` |
| 生产环境 | `wss://gateway.connecttrade.com:37197` |

**消息协议：**

```json
// 下单示例（OrderSingle）
{
    "ClientID": "MyClientID",
    "Account": "MyAccount",
    "MsgType": "OrderSingle",
    "Symbol": "CL",
    "SecurityType": "FUTURE",
    "MaturityYearMonth": "202506",
    "Side": "BUY",
    "OrderType": "LIMIT",
    "OrderQty": "1",
    "Price": "70.50",
    "ExchangeDestination": "GLOBEX",
    "ManualOrderIndicator": "N",
    "SenderLocationID": "US,NY",
    "CustomerOrFirm": "CUSTOMER",
    "OperatorID": "C123456"
}
```

**消息类型：**

| 出站（发送） | 入站（接收） |
|-------------|-------------|
| OrderSingle | ExecutionReport |
| OrderMultileg | OrderSingleStatus |
| OrderCancel | OrderMultilegStatus |
| OrderReplace | PositionStatus |
| OrderMultiLegReplace | AccountSummaryStatus |
| | MonetaryInfoStatus |
| | LedgerInfoStatus |
| | OrderCancelReject |

**连接流程：**
1. WebSocket 连接成功 → `on_open()` 触发
2. 系统推送所有现有订单/持仓/账户状态（初始化同步）
3. `on_start()` 触发（初始化完成，可以开始下单）
4. 之后通过 `on_message()` 接收实时更新
5. 断线时 `on_close()` 触发，WS_Adapter 自动指数退避重连

**Python 适配器（WS_Adapter）：**

```python
from ws_adapter import WS_Adapter

adapter = WS_Adapter(
    connection_str="wss://onboarding.connecttrade.com:26553",
    apiKey="your_api_key_here",
    on_open=on_open,
    on_start=on_start,
    on_message=on_message,
    on_error=on_error,
    on_close=on_close
)
adapter.start()
```

**价格字段必须为字符串**：`"Price": "70.50"` 而非 `"Price": 70.50`——这是协议要求，避免浮点精度问题。

**期货下单额外必填字段**：`ManualOrderIndicator`（CME 规则 536-B 合规）、`SenderLocationID`、`CustomerOrFirm`、`OperatorID`。

---

## 1A. 编程语言与工具链全景

> 来源：[IB API Solutions](https://www.interactivebrokers.com/en/trading/ib-api.php)（Playwright 抓取）、[Programming Languages 专区](https://www.interactivebrokers.com/campus/category/ibkr-quant-news/programing-languages/)（32 页文章）、[IBKR API Available Programming Languages](https://www.interactivebrokers.com/campus/ibkr-quant-news/ibkr-api-available-programming-languages/)

### 1A.1 官方 API 支持的语言

IB API Solutions 页面明确声明 TWS API 支持：**"C++, C#, Java, Python, ActiveX, RTD or DDE"**。

| 语言 | TWS API 支持 | 跨平台 | 性能 | 适用场景 |
|------|-------------|--------|------|---------|
| **Python** | 官方原生 | 全平台 | 中 | **量化策略开发、回测、数据分析（我们的选择）** |
| **Java** | 官方原生 | 全平台 | 高 | 高性能生产系统、TWS 本身就是 Java |
| **C++** | 官方原生 | POSIX 全平台 | 最高 | 超低延迟交易、HFT |
| **C# (.NET)** | 官方原生 | 仅 Windows | 高 | Windows 桌面交易系统 |
| **R** | 通过 TWS API R 包 | 全平台 | 中 | 统计分析、回测、因子研究 |
| **Julia** | 第三方库 | 全平台 | 高 | 科学计算、解决"两语言问题" |
| **ActiveX/DDE** | 官方原生 | 仅 Windows | — | Excel 集成（轻量级） |
| **RTD** | 官方原生 | 仅 Windows | — | Excel 实时数据 |

**IBKR 官方评价**：Java、Python、C++ 是"very robust, help quants in building high-performance algorithms and are available for all platforms"。.NET/ActiveX/DDE 是"available for Windows only and are a good starting place for light programming projects"。

### 1A.2 官方 API Feature 对比（三大 API）

从 IB API Solutions 页面抓取的官方 Feature Comparison 表：

| Feature | Web API | FIX API | TWS API |
|---------|---------|---------|---------|
| 数字化创建/管理客户账户 | ✅ | ❌ | ❌ |
| 用户认证 Gateway | ✅ | ❌ | ❌ |
| Bearer/SSO/OAuth 认证 | ✅ | ❌ | ❌ |
| RESTful API + WebSockets | ✅ | ❌ | ❌ |
| 标准订单类型 | ✅ | ✅ | ✅ |
| **完整订单类型套件** | ❌ | ✅ | ✅ |
| 账户和 P&L 信息 | ✅ | ❌ | ✅ |
| **市场数据（快照/流式/历史）** | ✅ | ❌ | ✅ |
| 新闻 | ✅ | ❌ | ✅ |
| FA 分配交易 | ✅ | ✅ | ✅ |
| 聚合用户支持 | ✅ | ❌ | ❌ |
| 实时 Drop Copy | ❌ | ✅ | ❌ |

**结论**：TWS API 拥有**完整订单类型 + 市场数据 + 账户/P&L** 三大核心能力，是量化交易系统的最佳选择。Web API 多了账户管理能力但缺少完整订单类型；FIX API 适合机构级纯执行。

### 1A.3 四类用户场景

IB 官方将 API 用户分为四类：

| 用户类型 | 描述 | 可用服务 | **我们属于** |
|---------|------|---------|------------|
| **Retail/Day/Algo Traders** | 零售、日内、算法交易者 | Trading, Trading at Scale | **✅ 这类** |
| Institutional | 对冲基金、基金管理人、银行 | Trading, Reporting, Scale | |
| Enterprise | 介绍经纪商、RIA、银行 | 全部（含开户/资金管理） | |
| Third Party Developer | 软件供应商、平台集成 | Trading, Reporting, Scale | |

### 1A.4 Python 工具链（与我们直接相关）

#### 核心 Python 库对比

| 库 | 类型 | 维护状态 | 特点 | 推荐度 |
|----|------|---------|------|--------|
| **ib_insync / ib_async** | 第三方高层封装 | ib_async 社区维护中 | 同步风格 API、asyncio、Jupyter 集成、Watchdog | **⭐⭐⭐⭐⭐ 首选** |
| **ibapi（官方原生）** | 官方 SDK | 官方维护（v10.42） | EClient/EWrapper 异步模型、线程管理复杂 | ⭐⭐⭐ 需要底层控制时 |
| **ibapi Sync Wrapper** | 官方同步封装 | Beta（v10.40+） | 合并 EClient/EWrapper、同步风格 | ⭐⭐ Beta，功能不全 |
| **IBridgePy** | 第三方平台 | 活跃维护 | 回测+实盘一体、IB 官方推荐 | ⭐⭐⭐ 回测场景 |
| **IBind** | 第三方 Web API | 活跃维护 | REST + WebSocket、OAuth 1.0a | ⭐⭐ Web API 场景 |

#### 回测框架生态

| 框架 | IB 集成 | 特点 | 适用场景 |
|------|---------|------|---------|
| **Backtrader** | 原生支持 | 开源、本地运行、支持实盘 | **本地回测+实盘切换** |
| **QuantConnect** | 原生 LEAN Engine | 云平台、Jupyter 研究笔记本、精确费用/滑点建模 | **专业级回测+云部署** |
| **IBridgePy** | 原生支持 | 回测/实盘代码一致、IB 官方推荐 | **快速原型+IB直连** |
| **Zipline-Live** | 通过 IB 适配器 | Quantopian 遗产、开源 | 历史数据研究 |
| **QuantRocket** | IB 专用 | Jupyter 集成、Moonshot 引擎、多引擎支持 | **IB 专用量化平台** |

#### Jupyter Notebook 集成

IB 官方有专门文章 [An Introduction to TWS API with Jupyter Notebooks](https://www.interactivebrokers.com/campus/ibkr-quant-news/an-introduction-to-tws-api-with-jupyter-notebooks/) 介绍 Jupyter 集成。

- **ib_insync** 天然支持 Jupyter——可以在 notebook 中交互式下单、查看行情、调试策略
- **QuantConnect** 提供 Jupyter Lab 研究笔记本，支持加载 1998 年至今的美股数据
- **QuantRocket** 以 Jupyter 为主要界面

#### 数据科学/ML 库（IBKR Quant 文章高频出现）

| 库 | 用途 | IBKR 文章覆盖 |
|----|------|-------------|
| Pandas | 数据操作 | 密集 |
| NumPy | 数值计算 | 密集 |
| scikit-learn | ML 模型 | 多篇 |
| XGBoost | 梯度提升（金银建模等） | 近期热点 |
| statsmodels | 统计检验（协整、平稳性） | 多篇 |
| TA-Lib | 技术指标 | 多篇 |
| Matplotlib/Plotly | 可视化 | 基础设施 |
| LlamaIndex | AI 研究自动化 | 2026-06 新文章 |

### 1A.5 GitHub 社区生态

| GitHub Topic | 项目数 | 典型项目 |
|-------------|-------|---------|
| [tws-api](https://github.com/topics/tws-api) | 活跃 | C++ 自动交易框架、Options 自动化（JS）、Go 非官方 API |
| [twsapi](https://github.com/topics/twsapi) | 活跃 | IB Gateway Docker（TS, 237⭐）、Rust IB API（170⭐） |
| [ibkr-api](https://github.com/topics/ibkr-api) | 活跃 | MLM 趋势跟踪策略（Python/ib_insync） |
| [laroche/tws-api-examples](https://github.com/laroche/tws-api-examples) | — | 官方 API 示例集合 |

### 1A.6 Programming Languages 专区内容分析

IBKR Quant 的 [Programming Languages 专区](https://www.interactivebrokers.com/campus/category/ibkr-quant-news/programing-languages/) 共 **32 页文章**，是 IBKR Campus 最大的内容板块之一。

**2026 年文章主题分布（第 1 页，30 篇）**：

| 主题 | 数量 | 代表文章 |
|------|------|---------|
| **Python 回测/交易** | ~8 | IBridgePy 系列（回测指南、实盘策略、入门） |
| **R 语言建模** | ~4 | Markov 模型、K 线图绘制、RStudio vs Positron |
| **数据科学/ML** | ~5 | XGBoost 金银建模、LLM 动量投资、统计套利 |
| **AI/LLM** | ~3 | LlamaIndex、LLM 交易系统、GenAI 量化代理 |
| **Python 基础** | ~3 | 虚拟环境、数据处理、数据操作 |
| **量化理论** | ~4 | Gamma Greeks、跟踪误差、因子拥挤 |
| **月度精选** | ~3 | 1-5 月 Blog Highlights |

**趋势洞察**：
1. **Python 绝对主导**——IBridgePy 几乎每两周发一篇 Python 教程
2. **R 语言仍然活跃**——统计建模和可视化领域 R 不可替代
3. **AI/LLM 融入加速**——GenAI、LlamaIndex、Azure AI Foundry 在量化中的应用
4. **金银比研究**——DataGeeek 的 Gold/Silver Ratio 系列与我们的金油比研究方法论相通

### 1A.7 对我们 Python 系统的实操建议

```
我们的技术栈选择：
┌─────────────────────────────────────────────────────┐
│ 数据层:  ib_insync/ib_async (TWS API)               │
│ 策略层:  NewChan RecursiveOrchestrator (自研缠论引擎)  │
│ 回测:    Backtrader 或自研 (已有 replay 系统)          │
│ 研究:    Jupyter + Pandas + Matplotlib               │
│ 执行:    ib_insync placeOrder / bracketOrder         │
│ 部署:    Docker (IB Gateway) + Python 主进程          │
│ 监控:    FastAPI Gateway (已有) + SQLite 日志          │
└─────────────────────────────────────────────────────┘

不需要的：
- QuantConnect: 我们有自己的缠论引擎，不需要云回测平台
- IBridgePy: 我们已经直接用 ib_insync，不需要再加一层封装
- Excel API: 不用 Excel
- FIX: 不是机构级需求
- Julia/R: 我们的策略全在 Python 中
```

---

## 2. 当前项目现状

### 2.1 已有组件

| 模块 | 路径 | 功能 | 状态 |
|------|------|------|------|
| IBKR 数据源 | `src/newchan/data_ibkr.py` | 历史 K 线拉取 + 5s 实时 bar 订阅 | 可用 |
| FastAPI 网关 | `src/newchan/gateway.py` | REST + WebSocket（回放/实时推送） | 可用 |
| Bottle 服务 | `src/newchan/server.py` | 本地 HTTP API（缓存/指标） | 可用 |
| 递归编排器 | `src/newchan/orchestrator/recursive.py` | 五层管线：包含→笔→段→走势→中枢 | 核心 |
| 递归栈 | `src/newchan/core/recursion/` | 买卖点引擎、中枢引擎、线段引擎等 | 核心 |
| 配置 | `src/newchan/config.py` | IB_HOST/IB_PORT/IB_CLIENT_ID | 可用 |

### 2.2 缺失组件

| 缺失模块 | 说明 |
|----------|------|
| **下单执行层** | 信号→订单的转换和执行 |
| **风控模块** | 仓位限制、止损、最大回撤 |
| **持仓管理** | 持仓同步、仓位追踪 |
| **Watchdog/重连** | 生产级断线重连、IB Gateway 每日重启处理 |
| **信号桥接** | 买卖点检测结果→交易信号→订单参数 |
| **日志/监控** | 交易日志、P&L 追踪、告警 |

---

## 3. 目标架构

### 3.1 系统总览

```
┌───────────────────────────────────────────────────────────────────────┐
│                        量化交易系统                                     │
│                                                                       │
│  ┌─────────────┐    ┌──────────────┐    ┌────────────┐               │
│  │  数据层       │    │  策略层        │    │  执行层      │               │
│  │             │    │              │    │            │               │
│  │ IBKRProvider │───►│ RecursiveOrch│───►│ OrderRouter│               │
│  │ (data_ibkr) │    │ (五层管线)     │    │ (下单路由)   │               │
│  │             │    │              │    │            │               │
│  │ 历史数据      │    │ 买卖点检测     │    │ 风控过滤     │               │
│  │ 实时5s bar   │    │ 背驰判定       │    │ 仓位管理     │               │
│  │ 合约解析      │    │ 中枢边界       │    │ LMT单执行   │               │
│  └──────┬──────┘    └──────┬───────┘    └──────┬─────┘               │
│         │                  │                   │                     │
│         ▼                  ▼                   ▼                     │
│  ┌──────────────────────────────────────────────────────┐            │
│  │                  事件总线 (EventBus)                    │            │
│  └──────────────────────────────────────────────────────┘            │
│         │                  │                   │                     │
│         ▼                  ▼                   ▼                     │
│  ┌─────────────┐    ┌──────────────┐    ┌────────────┐               │
│  │  监控层       │    │  持久层        │    │  网关层      │               │
│  │             │    │              │    │            │               │
│  │ P&L 追踪    │    │ SQLite 日志   │    │ FastAPI GW │               │
│  │ 健康检查     │    │ 交易记录       │    │ WebSocket  │               │
│  │ 告警推送     │    │ 状态快照       │    │ REST API   │               │
│  └─────────────┘    └──────────────┘    └────────────┘               │
│                                                                       │
└───────────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────┐
│   IB Gateway     │  (Docker 容器，headless)
│   port 4002      │
└─────────────────┘
```

### 3.2 核心数据流

```
K线(5s bar) ──► 聚合(1min/5min/30min) ──► RecursiveOrchestrator
                                              │
                                              ├──► 笔 (BiEngine)
                                              ├──► 段 (SegmentEngine)
                                              ├──► 走势 (MoveEngine)
                                              ├──► 中枢 (ZhongshuEngine)
                                              └──► 买卖点 (BuySellPointEngine)
                                                        │
                                                        ▼
                                                   SignalBridge
                                                        │
                                              ┌─────────┴─────────┐
                                              │                   │
                                              ▼                   ▼
                                         RiskFilter          PositionManager
                                              │                   │
                                              └─────────┬─────────┘
                                                        │
                                                        ▼
                                                   OrderRouter
                                                        │
                                                        ▼
                                                  ib.placeOrder()
```

---

## 4. 各层详细设计

### 4.1 数据层 — 增强 `data_ibkr.py`

当前 `IBKRConnection` 已支持：
- 历史 K 线拉取（`fetch_historical`）
- 5 秒实时 bar 订阅（`subscribe_realtime`）
- 品种搜索（`search_symbols`）

**需要增强的能力：**

```python
# 新增：bar 聚合器——将 5s bar 聚合为任意周期
class BarAggregator:
    """将 5s 实时 bar 聚合为目标周期 K 线。
    
    支持: 1min, 5min, 15min, 30min, 1hour
    每完成一根 K 线时触发回调。
    """
    
    def __init__(self, target_interval: str, on_bar_complete: Callable[[Bar], None]):
        ...

    def feed(self, bar_5s: Bar) -> None:
        """喂入 5s bar，内部自动聚合。"""
        ...
```

```python
# 新增：Watchdog 封装——处理每日重启
class IBKRWatchdog:
    """基于 ib_insync.Watchdog 的生产级连接管理。
    
    职责：
    1. 检测连接断开（每日重启、网络中断）
    2. 自动重连（指数退避：2s/5s/10s/20s）
    3. 重连后恢复所有数据订阅
    4. 每 7 天提醒人工重新认证
    """
    ...
```

### 4.2 策略层 — 信号桥接

缠论引擎的买卖点检测输出是 `BuySellPoint` 对象，需要桥接为交易信号：

```python
@dataclass(frozen=True)
class TradeSignal:
    """从缠论买卖点到交易指令的桥接对象。"""
    timestamp: datetime
    symbol: str
    direction: Literal["BUY", "SELL"]
    signal_type: str          # "1buy", "2buy", "3buy", "1sell", "2sell", "3sell"
    level: int                # 缠论级别
    price: float              # 信号触发价
    confidence: float         # 0.0 ~ 1.0
    source_bsp: BuySellPoint  # 原始买卖点对象
    metadata: dict            # 中枢边界、背驰类型等补充信息
```

**信号生成逻辑（这是核心业务逻辑，需要你来定义）：**

```python
class SignalBridge:
    """将 RecursiveOrchestrator 的输出转换为 TradeSignal。
    
    核心决策点——需要你定义：
    1. 哪些级别的买卖点可以触发交易？（L1? L2?）
    2. 一买/二买/三买的优先级和过滤条件？
    3. 多周期确认逻辑：大级别方向 + 小级别入场点？
    4. MACD 背驰确认是否作为必要条件？
    5. confidence 如何计算？
    """
    
    def evaluate(self, orchestrator: RecursiveOrchestrator) -> list[TradeSignal]:
        # TODO: 这里是你的交易策略核心
        ...
```

### 4.3 风控层

```python
@dataclass(frozen=True)
class RiskConfig:
    """风控参数配置。"""
    max_position_size: int = 1            # 单品种最大持仓手数
    max_total_positions: int = 3          # 总持仓品种数上限
    max_daily_loss: float = 2000.0        # 日最大亏损（美元）
    max_single_loss: float = 500.0        # 单笔最大亏损（美元）
    stop_loss_ticks: int = 20             # 止损 tick 数
    trailing_stop: bool = False           # 是否启用追踪止损
    cooldown_after_loss: int = 300        # 止损后冷却期（秒）
    trading_hours: tuple[str, str] = ("09:30", "16:00")  # 交易时段


class RiskFilter:
    """风控过滤器——决定信号是否可以执行。
    
    检查清单：
    1. 当前持仓是否已达上限
    2. 日亏损是否已达上限
    3. 是否在冷却期内
    4. 是否在交易时段内
    5. 信号方向是否与现有持仓冲突
    """
    
    def check(self, signal: TradeSignal, state: PortfolioState) -> RiskDecision:
        ...
```

### 4.4 执行层 — OrderRouter

```python
class OrderRouter:
    """订单路由——将通过风控的信号转为 IBKR 订单。
    
    支持的订单类型：
    - LMT (限价单)：主要使用，在买卖点附近设置限价
    - STP (止损单)：保护性止损
    - STP_LMT (止损限价)：更精确的止损
    - Bracket (括号单)：入场 + 止盈 + 止损 三合一
    """
    
    def submit(self, signal: TradeSignal, risk_params: RiskConfig) -> Trade:
        contract = IBKRProvider.make_contract(signal.symbol)
        
        # 推荐使用 Bracket Order: 入场 + 止盈 + 止损
        bracket = ib.bracketOrder(
            action=signal.direction,
            quantity=1,
            limitPrice=signal.price,
            takeProfitPrice=self._calc_tp(signal),
            stopLossPrice=self._calc_sl(signal),
        )
        
        trades = []
        for order in bracket:
            trade = self.ib.placeOrder(contract, order)
            trades.append(trade)
        return trades
```

**ib_insync 下单 API 速查：**

```python
from ib_insync import *

ib = IB()
ib.connect('127.0.0.1', 4002, clientId=1)

# 1. 构造合约
contract = ContFuture('CL', 'NYMEX')
ib.qualifyContracts(contract)

# 2. 限价单
order = LimitOrder('BUY', 1, 70.50)
trade = ib.placeOrder(contract, order)

# 3. 括号单（入场+止盈+止损）
bracket = ib.bracketOrder('BUY', 1, 
    limitPrice=70.50,       # 入场价
    takeProfitPrice=72.00,  # 止盈价
    stopLossPrice=69.50     # 止损价
)
for o in bracket:
    ib.placeOrder(contract, o)

# 4. 查看持仓
positions = ib.positions()

# 5. 查看账户
account = ib.accountSummary()

# 6. P&L 追踪
pnl = ib.pnl()

# 7. 期权链
chains = ib.reqSecDefOptParams('AAPL', '', 'STK', 265598)

# 8. What-if（测试保证金影响，不实际下单）
whatif = ib.whatIfOrder(contract, order)
print(whatif.initMarginChange, whatif.commission)
```

### 4.5 持仓管理

```python
class PositionManager:
    """持仓状态管理——与 IBKR 同步。
    
    职责：
    1. 启动时从 IBKR 同步现有持仓
    2. 订单成交后更新本地状态
    3. 提供持仓查询接口
    4. 计算浮动盈亏
    """
    
    def sync_from_ibkr(self) -> None:
        """从 IBKR 拉取当前真实持仓。"""
        positions = self.ib.positions()
        portfolio = self.ib.portfolio()
        ...
    
    def on_order_filled(self, trade: Trade) -> None:
        """订单成交回调——更新本地状态。"""
        ...
```

---

## 5. 生产部署架构

### 5.1 推荐部署方案

```
┌─────────────────────────────────────────────────────┐
│  Docker Compose                                      │
│                                                      │
│  ┌──────────────────┐    ┌────────────────────────┐  │
│  │  ib-gateway       │    │  newchan-trader         │  │
│  │  (IB Gateway +    │◄──►│  (Python 主进程)         │  │
│  │   IBC controller) │    │                        │  │
│  │  port: 4002       │    │  - IBKRWatchdog        │  │
│  │  memory: 2-4GB    │    │  - RecursiveOrchestrator│  │
│  │                   │    │  - SignalBridge         │  │
│  └──────────────────┘    │  - RiskFilter           │  │
│                           │  - OrderRouter          │  │
│                           │  - FastAPI Gateway      │  │
│                           │  port: 8766             │  │
│                           └────────────────────────┘  │
│                                                      │
│  ┌──────────────────┐    ┌────────────────────────┐  │
│  │  sqlite-data      │    │  监控 (可选)              │  │
│  │  (持久化 volume)   │    │  Grafana + Prometheus  │  │
│  └──────────────────┘    └────────────────────────┘  │
│                                                      │
└─────────────────────────────────────────────────────┘
```

### 5.2 IB Gateway Docker 镜像

推荐使用社区维护的 Docker 镜像：

```yaml
# docker-compose.yml
services:
  ib-gateway:
    image: ghcr.io/gnzsnz/ib-gateway:stable
    ports:
      - "4002:4002"
    environment:
      TWS_USERID: ${IB_USERNAME}
      TWS_PASSWORD: ${IB_PASSWORD}
      TRADING_MODE: paper          # paper | live
      TWS_SETTINGS_PATH: /opt/ibc/config
    volumes:
      - ib-data:/opt/ibc
    restart: unless-stopped
    mem_limit: 4g

  newchan-trader:
    build: .
    depends_on:
      - ib-gateway
    environment:
      IB_HOST: ib-gateway
      IB_PORT: 4002
      IB_CLIENT_ID: 1
      TRADING_MODE: paper
    volumes:
      - ./data:/app/data
    restart: unless-stopped

volumes:
  ib-data:
```

### 5.3 Watchdog 与重连

```python
# 生产级连接管理（基于 ib_insync Watchdog）
from ib_insync import IB, IBC, Watchdog, Forex

def create_watchdog(trading_mode: str = "paper") -> Watchdog:
    """创建生产级 Watchdog。"""
    ibc = IBC(
        twsVersion=1042,         # TWS API 版本
        gateway=True,            # 使用 IB Gateway（非 TWS）
        tradingMode=trading_mode,
    )
    
    ib = IB()
    
    watchdog = Watchdog(
        ibc=ibc,
        ib=ib,
        port=4002 if trading_mode == "paper" else 4001,
        connectTimeout=60,       # 连接超时（秒）
        appStartupTime=30,       # Gateway 启动等待时间
        appTimeout=20,           # 无消息超时（触发探测）
        retryDelay=5,            # 重连间隔
        probeContract=Forex('EURUSD'),  # 探测用合约
    )
    
    # 注册事件处理
    watchdog.startingEvent += on_starting
    watchdog.startedEvent += on_started
    watchdog.stoppingEvent += on_stopping
    watchdog.stoppedEvent += on_stopped
    watchdog.softTimeoutEvent += on_soft_timeout
    watchdog.hardTimeoutEvent += on_hard_timeout
    
    return watchdog


def on_started(watchdog):
    """Gateway 启动成功——恢复数据订阅。"""
    ib = watchdog.ib
    # 重新订阅所有品种的实时数据
    for symbol in active_symbols:
        subscribe_realtime(ib, symbol)


def on_hard_timeout(watchdog):
    """硬超时——Gateway 无响应，Watchdog 将自动重启。"""
    logger.error("IB Gateway hard timeout, restarting...")
```

### 5.4 每日重启处理

IB Gateway 每天凌晨有短暂重启窗口（美东时间约 23:45-00:45）。处理策略：

1. **检测断开**：`disconnectedEvent` 触发
2. **暂停策略**：标记为 `DISCONNECTED` 状态，不生成新信号
3. **自动重连**：Watchdog 自动尝试重连
4. **恢复订阅**：重连成功后，重新订阅所有数据流
5. **状态验证**：同步持仓，确认订单状态

---

## 6. 纸盘（Paper Trading）环境配置

### 6.1 开通步骤

1. 登录 IBKR 账户管理 → Settings → Paper Trading Account
2. 记录 Paper 账户用户名（通常是 `主用户名 + "DU"` 后缀）
3. 市场数据权限与主账户共享（需在主账户设置中开启共享）

### 6.2 连接配置

```bash
# .env（纸盘环境）
IB_HOST=127.0.0.1
IB_PORT=7497          # TWS Paper 端口
# IB_PORT=4002        # IB Gateway Paper 端口
IB_CLIENT_ID=1
TRADING_MODE=paper
```

### 6.3 纸盘限制

- 市场数据：使用 15 分钟延迟数据（除非主账户同时在线并共享订阅）
- 不能同时在两台机器上登录 Paper 和 Live
- 纸盘成交是模拟的，不反映真实市场深度和滑点

### 6.4 开发建议

1. **先 Paper 30 天**：验证策略逻辑、连接稳定性、异常处理
2. **记录所有交易**：Paper 期间的每笔交易都写入 SQLite 日志
3. **对比分析**：Paper P&L vs 回测 P&L，检查执行偏差
4. **逐步放量**：Paper 通过后，Live 从最小仓位开始

---

## 7. 实时数据流接入

### 7.1 当前方案（5s Real-Time Bars）

`data_ibkr.py` 的 `IBKRConnection.subscribe_realtime()` 使用 `reqRealTimeBars`——每 5 秒一根 bar。

**优点：** 简单、不占市场数据线（实时 bar 不计入 100 线限制）
**缺点：** 最小粒度 5s，无法获取 tick 级数据

### 7.2 增强方案：Tick-by-Tick + 本地聚合

```python
# 获取逐笔成交（最细粒度）
ib.reqTickByTickData(contract, tickType='AllLast')

# 或获取 Level 1 行情（计入 100 线限制）
ticker = ib.reqMktData(contract)
```

### 7.3 推荐的混合方案

```
品种数量 ≤ 5:
  → 使用 reqMktData (Level 1 tick) + 本地聚合
  → 最精确，但占市场数据线

品种数量 > 5:
  → 使用 reqRealTimeBars (5s bar) + 本地聚合
  → 不占数据线，但粒度较粗

历史回填:
  → 启动时用 reqHistoricalData 拉取最近 N 天数据
  → 本地缓存到 SQLite，只增量拉取新数据
  → 注意 pacing 限制：10 分钟内 ≤ 60 次请求
```

### 7.4 多周期聚合

```python
class MultiTimeframeAggregator:
    """从 5s bar 聚合出多个周期，喂入各级别编排器。
    
    5s bar ──► 1min aggregator ──► RecursiveOrchestrator (L0)
           ──► 5min aggregator ──► RecursiveOrchestrator (L1)
           ──► 30min aggregator ──► RecursiveOrchestrator (L2)
    
    每个级别独立运行，SignalBridge 做多周期确认。
    """
    ...
```

---

## 8. 期权链数据获取

```python
# 获取期权链参数
chains = ib.reqSecDefOptParams(
    underlyingSymbol='AAPL',
    futFopExchange='',
    underlyingSecType='STK',
    underlyingConId=265598
)

# chains 返回 OptionChain 列表，包含：
# - exchange: 交易所
# - underlyingConId: 标的 conId
# - tradingClass: 交易类别
# - multiplier: 合约乘数
# - expirations: 到期日集合 (frozenset of str)
# - strikes: 行权价集合 (frozenset of float)

# 构造具体期权合约
option = Option('AAPL', '20260620', 200, 'C', 'SMART')
ib.qualifyContracts(option)

# 获取期权行情
ticker = ib.reqMktData(option)
ib.sleep(2)
print(ticker.bid, ticker.ask, ticker.modelGreeks)
```

---

## 9. 主程序入口设计

```python
"""newchan-trader 主进程入口。"""

import asyncio
import logging
from newchan.config import IB_HOST, IB_PORT, IB_CLIENT_ID
from ib_insync import IB

logger = logging.getLogger("newchan.trader")


async def main():
    """量化交易系统主循环。"""
    
    # 1. 连接 IBKR
    ib = IB()
    ib.connect(IB_HOST, IB_PORT, clientId=IB_CLIENT_ID)
    logger.info("已连接 IBKR: %s:%s", IB_HOST, IB_PORT)
    
    # 2. 同步账户状态
    positions = ib.positions()
    account = ib.accountSummary()
    logger.info("当前持仓: %d 个品种", len(positions))
    
    # 3. 初始化组件
    # signal_bridge = SignalBridge(...)
    # risk_filter = RiskFilter(risk_config)
    # order_router = OrderRouter(ib)
    # position_mgr = PositionManager(ib)
    
    # 4. 拉取历史数据填充编排器
    # for symbol in watched_symbols:
    #     bars = fetch_historical(ib, symbol, ...)
    #     orchestrator.feed_bars(bars)
    
    # 5. 订阅实时数据
    # for symbol in watched_symbols:
    #     ib.reqRealTimeBars(contract, ...)
    
    # 6. 主事件循环
    # ib_insync 的事件循环驱动一切——
    # 实时 bar 回调 → 喂入编排器 → 检测买卖点 → 生成信号 → 风控 → 下单
    ib.run()


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    asyncio.run(main())
```

---

## 10. 实现路线图

### Phase 1: 纸盘数据验证（1-2 周）

- [ ] 将 `ib_insync` 迁移到 `ib_async`
- [ ] 实现 `BarAggregator`（5s → 1min/5min/30min）
- [ ] 实现 `MultiTimeframeAggregator` + `RecursiveOrchestrator` 联动
- [ ] 验证实时数据流入 → 缠论引擎输出买卖点的完整链路
- [ ] 纸盘环境下跑通

### Phase 2: 信号与执行（2-3 周）

- [ ] 实现 `TradeSignal` 数据结构
- [ ] 实现 `SignalBridge`（**核心策略逻辑——需要你定义**）
- [ ] 实现 `RiskFilter`
- [ ] 实现 `OrderRouter`（LMT 单 + Bracket 单）
- [ ] 实现 `PositionManager`
- [ ] 纸盘环境下跑通完整交易循环

### Phase 3: 生产加固（1-2 周）

- [ ] 实现 `IBKRWatchdog`（断线重连 + 订阅恢复）
- [ ] Docker 化部署（IB Gateway + 交易主进程）
- [ ] SQLite 交易日志
- [ ] 健康检查端点
- [ ] 告警推送（微信/邮件）

### Phase 4: 实盘过渡

- [ ] Paper 30 天运行记录分析
- [ ] Paper vs 回测 P&L 对比
- [ ] 切换到 Live 环境（改端口 4001）
- [ ] 最小仓位运行，逐步放量

---

## 11. 关键风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| Gateway 每日重启 | 短暂断连（~15min） | Watchdog 自动重连 + 暂停策略 |
| 7 天强制重新认证 | 需人工干预 | 设置日历提醒 + 手机 IBKR APP 快速认证 |
| 网络中断 | 无法接收数据/下单 | 指数退避重连 + 本地状态持久化 |
| 滑点 | 实际成交价偏离信号价 | LMT 单而非 MKT 单 + 合理的限价偏移 |
| Pacing 违规 | 历史数据请求被拒 | 本地缓存 + 增量拉取 + 速率控制 |
| 买卖点误判 | 错误交易 | 多周期确认 + 严格风控 + MACD 背驰过滤 |

---

## 参考资源

### IBKR 官方

- [IBKR Quant 主页](https://www.interactivebrokers.com/campus/category/ibkr-quant-news/ibkr-quant-home/) — 量化交易官方入口
- [IBKR API 文档首页](https://www.interactivebrokers.com/campus/ibkr-api-page/ibkr-api-home/) — API 套件总览
- [IBKR API 入门指南](https://www.interactivebrokers.com/campus/ibkr-api-page/getting-started/) — 从零开始
- [TWS API 文档（新版）](https://www.interactivebrokers.com/campus/ibkr-api-page/trader-workstation-api/)
- [TWS API 文档（旧版 GitHub）](https://interactivebrokers.github.io/tws-api/) — 已 deprecated 但细节更丰富
- [TWS API Changelog](https://www.interactivebrokers.com/campus/ibkr-api-page/tws-api-changelog-2/)
- [Web API 文档](https://www.interactivebrokers.com/campus/ibkr-api-page/webapi-doc/)
- [IBKR API 交易方案总览](https://www.interactivebrokers.com/en/trading/ib-api.php)
- [TWS API 历史数据限制](https://interactivebrokers.github.io/tws-api/historical_limitations.html)
- [TWS API 订单限制](https://interactivebrokers.github.io/tws-api/order_limitations.html)
- [Python TWS API 入门课程](https://www.interactivebrokers.com/campus/trading-course/python-tws-api/)
- [TWS API Sync Wrapper 公告](https://www.interactivebrokers.com/campus/ibkr-quant-news/the-new-synchronous-wrapper-for-tws-api/)
- [理解 TWS API 的异步模型](https://www.interactivebrokers.com/campus/ibkr-quant-news/understanding-asynchronous-libraries-with-the-tws-api/)
- [TWS API 并发编程](https://www.interactivebrokers.com/campus/ibkr-quant-news/how-to-use-concurrency-in-the-tws-python-api/)
- [IBKR API 开发文章合集](https://www.interactivebrokers.com/campus/category/ibkr-quant-news/api-development/)

### 第三方库

- [ib_insync 文档](https://ib-insync.readthedocs.io/)
- [ib_async（ib_insync 社区维护版）](https://github.com/ib-api-reloaded/ib_async)
- [IBind（Web API Python 客户端）](https://github.com/Voyz/ibind)
- [ibkr-client（Node.js，含 RxJS WebSocket）](https://github.com/art1c0/ibkr-client)

### Lightspeed Connect

- [Lightspeed Connect 产品页](https://lightspeed.com/trading/api-trading-at-IBKR)
- [Lightspeed Connect Getting Started Guide (PDF)](https://d31x4u3ydvpof.cloudfront.net/manuals/Lightspeed_Connect_API_Getting_Started_Guide_IBKR_Prod.pdf)
- [Lightspeed Connect Demo](https://lightspeed.com/lightspeed-connect-app-ibkr)

### 部署与运维

- [IB Gateway Docker 镜像（gnzsnz）](https://github.com/gnzsnz/ib-gateway-docker)
- [IB Gateway Docker 镜像（hartza-capital）](https://github.com/hartza-capital/docker-ib-gateway)

### 第三方教程

- [Interactive Brokers API 终极指南](https://paperswithbacktest.com/wiki/interactive-brokers-api) — 最全面的第三方教程
- [IB Python API 原生指南（AlgoTrading101）](https://algotrading101.com/learn/interactive-brokers-python-api-native-guide/)
- [QuantConnect + IBKR 集成](https://www.quantconnect.com/brokerages/interactive)
- [Backtrader + IBKR 回测](https://ibkrcampus.com/ibkr-quant-news/back-testing-on-ibkr-with-backtrader-part-i/)
- [IB Python API 2026 自动交易设置](https://blog.pickmytrade.io/ib-api-python-2026-automated-trading-setup-ibkr-integration/)
