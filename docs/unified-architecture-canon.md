# 统一架构正本：递归式目标模块图

> **正本声明**：本文件是「目标模块图」的架构正本——map [统一架构正本 #787](https://github.com/xy7365527-lang/NewChanlun/issues/787) 乙层产出，由子票 [乙层目标模块图 #960](https://github.com/xy7365527-lang/NewChanlun/issues/960) 收口落盘（2026-08-14）。
> **性质**：对既成事实的描述（#799 裁定一：先纪律与闸门、后模块图），不是蓝图。图的节点主键 = **判定**（Q1）；阶段（数据→结构→对位→决策→执行→账本）是第二投影。
> **与教义正本的关系**：本图不重述任何判定内容——每个判定模块只锚 `.chanlun/definitions/` 对应正本；本图管「模块有哪些、谁消费谁、谁落在哪、怎么验收」。

## 一、前提：双向递归 + 递归式统一

- **纵向·往上**（级别的递归生成）→ 塔（构造链，现役）
- **纵向·往下**（区间套下钻）→ 树（机制层真自递归；消费层收敛中）
- **横向**（同级别分解）→ 一等模块（ADR 0011，操作分解层）
- **递归式统一前提**（编排者 2026-07-30）：目标模块图的验收标准不是「模块划清楚了」，而是「同一套判定在每一级复用同一个模块」。

## 二、总体验收标准（三句 + 一句声明；#960 Q4 裁定）

1. **同一套判定，每一级复用同一个模块**——级别差异只准进数据（总缝规则 #804）；角色差异（中间产物／最终产物）也是数据（#827）。
2. **每条判定三档载体验收，且 CI 真执行**——结构性质→Lean 定理／行为不变式→测试锁／禁令→grep gate；每条声明载体（#799 裁定十一）；只编译不执行按无锁计。
3. **两轴之间只有「结构事实」一条边**——判定模块只消费构造轴产出的结构事实（Classification），不重复构造逻辑；构造轴不掺判定。凡两轴之间冒出第三套东西，必是句 1 的违规。
4. **（声明，并入句 1）多重赋格的 N 重 = 判定模块在同一套数据上的实例化投影，不新增任何判定**（ADR 0010）——句 1–3 不因 N 重并存而失效。

## 三、判定层：16 个判定模块

图的一等公民。每个模块 = 一套判定 + 一个生产实现；换装权受「六、换装管制」约束。

| # | 判定模块 | 正本锚点 | 复用级 | 主落点（现役） | 收敛候选（待收） |
|---|---|---|---|---|---|
| 1 | K线包含处理 | `definitions/baohan.md` | L0 专属（#799 裁定二收口处） | `src/newchan` + `rust/src` 顶层 | — |
| 2 | 分型 | `definitions/fenxing.md` | L0 专属 | `src/newchan` + `rust/src` 顶层 | — |
| 3 | 笔 | `definitions/bi.md`（#813） | L0 专属 | `rust/src` 顶层（族I）+ `theta_v0/` | — |
| 4 | 线段 | `definitions/xianduan.md`（#813） | L0 专属 | `rust/src` 顶层 + `theta_v0/` | — |
| 5 | 中枢构造 | `definitions/zhongshu.md`（#812，Rust20/Lean8/Py13 清单） | **每级** | `theta_v0/` | `trading/` `spiral/` `fugue_v3/` `recursive_t/` 变体 |
| 6 | 走势类型·趋势/盘整三分 | `definitions/qushi.md`+`zoushi.md`（#815） | **每级** | `theta_v0/` | `recursive_t/` `trading/` 变体 |
| 7 | 级别递归·纵向造级别 | `definitions/level_recursion.md`（#815） | 构造链自身 | `theta_v0/`（`recursive_tower.rs`） | — |
| 8 | 背驰确认 | `definitions/beichi.md`（#814） | **每级**（次级别确认） | `theta_v0/` | `trading/` `spiral/` `fugue_v3/` `recursive_t/` 五套实现（#862 处置） |
| 9 | 买卖点判定 | `definitions/maimai.md`（#816） | **每级**（一/二/三类） | `theta_v0/` | `trading/` 变体 |
| 10 | 区间套下钻 | `definitions/qujiantao.md`（#817） | **每次下钻**（同一模块） | `theta_v0/` | `trading/`（含伪引文落点 `positional.rs:458` 待订正） |
| 11 | 力度比较 | `definitions/bijia.md` + #862 | **每级** | `theta_v0/` | 五套实现（#862 处置清单） |
| 12 | 同级别分解·横向读级别 | `definitions/level_recursion.md` 横向节 + ADR 0011 | 每操作级别（角色是数据） | 一等模块（本仓尚无生产实现，ADR 0011） | — |
| 13 | 准入门（χ/nest/k_Θ） | 装置，锚 #799/#566 线裁定 | 全局 | `theta_v0/` | `trading/` 变体 |
| 14 | 量层判据（容量/频率门） | 装置，锚 ADR 0016 | 全局 | 判定在仓，实现待接 | — |
| 15 | 风控判据 | `definitions/fengkong.md` + 生产 `RiskPolicy` | 全局 | `theta_v0/strategy/` | — |
| 16 | 账本/总账判定 | **挂起**（#836 被 #937 挡；重启走 ADR 0021） | — | `theta_v0/ledger/`（净额/逐仓对账区，#849） | `recursive_t/` `trading/` 账本族 |

**边类型（不是模块）**：等价关系（`dengjia.md`）、流转关系（`liuzhuan.md`）——结构关系，画成边。
**总纲**（`chanlun-trading-system.md`）= 本图正本的教义总纲参照，不是第 17 个模块。
**已裁掉、图上显式标注「无此模块」**：跨区仲裁（#834/ADR 0014：重间不仲裁，无仲裁人）；「筛下跌段」开关（#827 裁定五：不存在）；换股选股（map #787 Out of scope）。

## 四、落点层：代码区归类（2026-08-14 审计重建）

并集规则（审计 §6）：三份旧清单 ∪ 裁定受影响代码清单的宿主 ∪ pymodule 导出反查 ∪ 编译树 `pub mod` ∪ CI 三个 job 的执行面。旧三份清单（疆域表/范围条/#789 区代号）互不一致、数字已漂，以本表为准。

| 区 | 归类 | 状态/名分 | 备注 |
|---|---|---|---|
| `rust/src/theta_v0/` | **现役主线**（判定主落点） | π 唯一现役引擎 | 零 Python 出口；消费面全在 `bin/` |
| `rust/src/*.rs` 顶层（17 文件） | 现役主线（族I Rust 8 层） | 14+2 单列+lib.rs | 142 个 .py 经 `newchan_rust` 消费 |
| `src/newchan/` | 现役（Python 引擎） | 活 | analysis 72 脚本消费 |
| `rust/src/trading/` | **收敛候选区**（31 文件/34,431 行） | 现役（GUARD-ROLE 两次核查） | 五 pymodule 导出、96 个 .py 调用文件；ADR 0017/qujiantao 伪引文落点 |
| `rust/src/spiral/` + `fugue_v3/` | **收敛候选区** | 现役（#762 改判） | ADR 0016 双互斥 λ 宿主（`LAMBDA` 2.0 vs 3.0） |
| `rust/src/recursive_t/` | **收敛候选区**（待杀） | 现役（#762 二次订正） | 退场 = 执刀票 #951（#808 条款） |
| `rust/src/bin/`（53 文件） | 装备区 | 活 | π 引擎全部消费面（`bi.md:342`）；探针宿主 |
| `rust/tests/` + `tests/` | **验收载体区** | 活（CI cargo test） | 对拍锁宿主（#804：载体只能是测试锁）；formal↔rust E7 边落点 |
| `formal/` | 验收载体区（Lean 对拍） | 活（fixture-drift job） | 154 文件/144 .lean |
| `analysis/` | 研究消费区 | 活 | #844 订正：698 条 ≈ 一半文档；E4 sys.path 注入覆盖 |
| `scripts/` | 研究装备区 | 活 | `check_fixture_drift.py` = CI fixture-drift 执行者；与 `analysis/` 是两个独立节点 |
| `trading_system/` | 研究消费区（离下单最近） | 活 | `rec_t_strategy.py` 生产策略在此 |

**出界声明（R3：一行一条，不许无声缺席）**：
- `topological-computation/` —— 独立研究系统（signifier_net）；承载语料第二副本（109 份，行号体系与正本不同）——**语料正本 = `docs/chanlun/text/blog/`，副本行号一律无效**（AGENTS.md 已钉）。
- `frontend/` —— 图表前端（React/Vite），消费 `src/newchan/server.py`；bi.md U-13 已登记「未核、无人跟进」，本图销账。
- `prototypes/`、`experiments/`、`external/`、`spec/theorems/` —— 原型/实验/外部/定理 spec 区。
- `chanlun/` —— 工作草稿区（与 Python 引擎 `newchan` 近名、与 `.chanlun/` 同名异物，引用时勿混）。
- `.chanlun/` —— 谱系与草稿（#790 边界条已排除）。
- `tradingview-mcp-plugin/` 等 —— TV 集成侧（K4 支柱）。
- 根级散件 `tws_margin_check.py`/`vcp_screener_v2.py` —— IBKR/VCP 边角（K4/执行支柱）。
- `tmp/`、`.scratch/` 等 —— 工作垃圾，图上不列。

## 五、装备插件层（Q2 分层裁定）

可换装、不参与生产判定路径（#799 裁定六「不适用」类）：数据源适配（ADR 0020/0021 场所面）、观测（opsem/witness）、诊断对照臂、探针（`bin/p*.rs`）、harness 工具（skills/MCP）。判定层与装备层的边界 = 句 3（装备不得掺判定）。

## 六、换装管制（判定层；Q2 裁定）

- 判定模块无运行期换装权；「想换一套」走裁定 + **退场条款**（#799 裁定四：另起一套必须写清什么时候死），白名单上限 3（触顶做强版）。
- **目录即层**（#799 裁定七：GUARD-ROLE nest 三件搬出判据目录，ADR-0005 搬家案）——实装归下游实施链，成本未估。
- 已裁掉的模块显式标注（见第三节尾），不留空槽位。

## 七、与下游 SPEC 的对齐表

| 下游件 | 对应判定 | 订正项 |
|---|---|---|
| SPEC #847（结构层实施总单） | 判定 7/10/12/16 + 量层 | 落点参考本图第四节；S 编号不变 |
| SPEC #756（端到端模块化） | 深模块 C1–C6 | C1（BSP 绑定单源化）= 对位判定的落点收敛；与收敛候选区（trading/spiral/fugue_v3）交叉 |
| #951（执刀票，图外） | recursive_t 退场 | 落点层收敛候选区的第一个执行件 |
| ADR 0022（验收参数） | 判定层外（验收尺子） | 本图句 2 的执行细则 |

## 八、画图定死的三个小口（#960 Q5 随附）

1. **单列读**：trading/spiral/fugue_v3/recursive_t 四族各自单列（#762 已判现役、各有独立 pymodule 面；折叠读会让 34K 行的 trading 族继续隐形）。
2. **`analysis/` 与 `scripts/` = 两个独立节点**（E4 只覆盖 analysis）。
3. **出界声明 = 一行一条 + 理由**（第四节末）。

## 090 照实

- 本图全部数字来自 2026-08-14 审计（main @ `a04e5f4a19`，`git ls-files` 实测 + 四条承重断言经本体会话独立复核）；main 前进后数字须重测，模块边界不受影响。
- 行数含空行注释，是量级。未跑 `cargo build`（静态读，参照 #849 先例）。
- 每判定的「受影响代码清单」正本在各定义文件内，本图不复制；区域级映射以上表为准。
