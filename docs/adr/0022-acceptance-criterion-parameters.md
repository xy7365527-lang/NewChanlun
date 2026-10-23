# ADR 0022：验收判据的参数——成本下降速率的定义、判读纪律与样本门槛

**日期**：2026-08-14
**裁定人**：编排者，走 [grilling 新验收判据的参数 #853](https://github.com/xy7365527-lang/NewChanlun/issues/853)（map [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787)）六问全 A 当场拍板
**状态**：已采纳。纯决策 ADR——实装归 SPEC [#847](https://github.com/xy7365527-lang/NewChanlun/issues/847) S8，本 ADR 只定口径与数字。

## 背景

[ADR 0015](0015-cost-trajectory-acceptance-metric.md) 把验收尺子换成成本轨迹（闸门前=成本下降速率／闸门后=净头寸价值／全程配=操作频率），但「成本下降速率」始终只有四字标签：无定义、无阈值、无判读纪律。ADR 0015 裁定六要求参数在重算任何旧实验**之前**落盘——本 ADR 即那一盘。事实基座：`.chanlun/review-results/issue853-acceptance-params-facts-20260808.md`（A–H 全节，只摆事实不裁）。

## 裁定

### 一、速率的可算定义：事件口径（#853 Q1 = A）

「成本下降速率」是**从零定义**（成本基账本已有，速率标量无实装）。与 `049-第49课.md:74`【正文】利润率「每次操作的平均利润／需要占用资金的平均时间」同形（ADR 0015 裁定二已把两者接死为同一量）：

- **采样单位 = 一次操作**——成本实际下降的落袋事件（成本只在已实现盈亏落袋时变动 ⟹ 轨迹是阶梯函数，信息全在事件上）；
- **每期读数 = 相对降幅** `(C_before − C_after)/C_before`（与「成本归 0」的闸门同形；绝对 bp 作报告伴随，不作门）；
- **速率 = 相对降幅 ÷ 占用时间**（自上次事件起）；
- 样本序列 = 事件序列。bar 口径（每 bar 平均斜率）只作报告读数。

### 二、通过阈值的零假设：费后 ≤ 0（#853 Q2 = A）

**费后口径**：手续费与资金费率扣进速率本身的成本口径（与 ADR 0015 裁定四「0 扣手续费与资金费率」同一口径，不在阈值处再减一个基线）。零假设 = 速率 ≤ 0——无优势 ⟹ 费后期望为负 ⟹ 成本不降反升（ADR 0015 裁定三的论证）。

**未裁（照实）**：费后口径的具体费率数字，等执行场所重启（[ADR 0021](0021-venue-selection-deferred.md) 重启条件三条）后、费率簿注入时接入——属实施面（#847 S8）。资金费率来源本仓仍无（事实报告 H.4）。

### 三、判据读法与置信下界：单边 95% 下界 + 按月分块 block bootstrap（#853 Q3 = A）

- **主判据 = 单边 95% 置信下界**，算法 = **按月分块 block bootstrap**（下界 = bootstrap 分布的 5% 分位；块 = 日历月）。中位数/分位数**降级为报告读数，不作门**。
- 依据：N-6 判例已立「门必须是下界不是点估计」；现有正态近似 LCB（`z=1.645` + 样本标准差 SE）对这条**单边截断、强右偏**（均值 ≈ 2–3× 中位数）分布左尾覆盖失真（事实 B.5 六条）；#907 在案「逐例与逐簇数字差 4 倍」⟹ 簇稳健必须。
- 生产 `z_alpha` 默认 0.0（LCB 实为关着）同步打开，但现有正态路径只用于过渡，验收读数一律走 block bootstrap。
- **方法标名**：凡出具读数必写「按月分块 block bootstrap，单边 95% 下界」（`docs/agents/stat-provenance.md` 轻档要求）。

> 记录化（#1677）：本裁定的数值为 #1677/evidence-gate/v1 的默认值，字段与版本规则见文末修订节。

### 四、频率不合成：两道独立门（#853 Q4 = A）

- **速率 = 验收唯一判据**；频率**不乘进**验收标量（乘积会抹平两个正交的失败模式：速率差但频率高→假过；速率好但频率低→假死。「不能偏废」的直译是各设各的门，不是乘在一起）。
- 频率的两个既有用途维持并当场对齐：ADR 0015「全程配套」**=** ADR 0016 裁定四「开重准入门」+ 验收报告配套读数——**同一个量、两个用途、各归各的**（此前两处无任何文件对齐，事实 C.3）。
- 报告读数按 #915 落地要求**买/卖分列**（只报总数会整类漏掉，GC L5 = 150 买/0 卖在案）。

### 五、闸门后判据（净头寸价值）参数：暂缺 + 触发条款（#853 Q5 = A）

**暂缺，待生产首次触达闸门后再定。** 事实：判据零实装（全库零命中）、生产从未触达（两条独立断言 count=0）。**触发纪律**：触达闸门后、判据第一次启用前，先定参数再启用——参数落盘时点移到该判据自己的生效前，不违反 ADR 0015 裁定六的时间序。禁止在零样本下硬编数字。

### 六、最小样本量：事件数 ≥ 30 且月块 ≥ 12（#853 Q6 = A）

- **事件样本 ≥ 30** 才出判据（`n<30` 结构性稀疏的 Le Cam 判例，从报告门升为判定门）；
- **月块 ≥ 12** 才跑 block bootstrap（块数下限；块太少区间无意义）；
- 不足则明写「**结构性稀疏，判不出**」——不算通过、不算失败、不出验收读数。

> 记录化（#1677）：本裁定的数值为 #1677/evidence-gate/v1 的默认值，字段与版本规则见文末修订节。

### 七、窗口纪律：N-6 四条判例收进正本（套判例，非新裁）

凡「比例/速率超阈值即触发动作」的验收判据，必须同时写清四件事（#853 票面元教训，N-6 判例四条）：

1. 用**下界**不用点估计（方法须标名，见裁定三）；
2. **≥ 3 个互不重叠窗**；
3. **每窗各自过线**，不看合计；
4. 合计覆盖 ≥ 50% 全史。

> 记录化（#1677）：本裁定的数值为 #1677/evidence-gate/v1 的默认值，字段与版本规则见文末修订节。

## 未裁 / 待接

- Q2 的具体费率数字：等场所重启（ADR 0021 重启条件）+ 资金费率来源，费率簿注入时接入——#847 S8 实施面。
- 实装 = SPEC #847 S8（含形式化侧 S8-a/b/c）；本 ADR 不实装。
- 闸门后判据的参数：触发条款见裁定五。

## 落点（均已核实存在，非声明）

- 本文件（新建）
- ADR 0015「本 ADR 未裁」首条回指本 ADR
- `CONTEXT.md` 新词条「成本下降速率」
- [#853](https://github.com/xy7365527-lang/NewChanlun/issues/853) resolution comment + 关票
- map [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787) Decisions-so-far 一行
- SPEC [#847](https://github.com/xy7365527-lang/NewChanlun/issues/847) 评论一条（依赖 #853 已满足）

## 溯源标签汇总

- 裁定一（定义与 `049:74` 同形）：`[旧缠论]` 的利润率定义 + `[新缠论:推论]`（事件口径的取法）。
- 裁定二（费后 ≤ 0）：`[新缠论:推论]`（承接 ADR 0015 裁定三/四）。
- 裁定三（block bootstrap 下界）：`[新缠论:选择]`（原文留白且无统计法可援）。
- 裁定四（两道门不合成）：`[旧缠论]` `049:74` 的「不能偏废」+ `[新缠论:推论]`（对齐两处 ADR）。
- 裁定五（暂缺+触发条款）：`[新缠论:选择]`（流程规则）。
- 裁定六（样本量）：`[新缠论:选择]`（套仓内 Le Cam 判例）。
- 裁定七（窗口纪律）：套 N-6 判例，非新裁。

## 修订（[#1677](https://github.com/xy7365527-lang/NewChanlun/issues/1677)，2026-10-22）：验收判据参数记录化为 evidence_gate

（append-only：裁定一至七原文一字不删，裁定三、六、七末各加一行指针，本节为现行修订。依 [ADR 0041](0041-degrees-of-freedom-configurable-not-excluded.md):51「既有裁定不追溯自动改写」，本修订走 ADR 0022 自己的裁定口径。本修订只把参数记录化，**不改任何数值**；改签验收统计数值不在本修订范围。）

**原因**：裁定三、六、七把成本下降速率验收门的参数写成正文单值，无字段、无版本。ADR 0041 裁定一、三要求标为 `[新缠论:选择]` 的量落成具名版本化记录中的可调字段（默认值、出处、可选值、代价），改值即新版本（ADR 0041:22、:23）。来源：`.chanlun/review-results/dof-audit-20261020/AUDIT.md:49`（A-02）。

### 1. 字段集

记录名 `evidence_gate`，id 族 `#1677/evidence-gate/vN`。字段为 ⟨policy_id, version, supersedes, confidence, block_unit, min_events, min_blocks, min_windows, min_coverage, authorizer, authorized_at⟩，全部定点整数或枚举，不出现浮点。

下表文件名未写目录者均在 `rust/src/theta_v0/backtest/` 下（行号以 main `66c409e7e3` 为准）。

| 字段 | 默认（v1） | 出处 | 值域（约束，越界拒收） | 可选值（出处 / 代价） | 溯源 |
|---|---|---|---|---|---|
| confidence | 95/100（单边） | ADR 0022:30 | 整数对，∈ (1/2, 1) | 975/1000（metrics.rs:401 下侧；要更多事件）；99/100（mu_estimator.rs:516；最严，稀疏最多） | `[新缠论:选择]`（:79） |
| block_unit | CalendarMonthUTC | ADR 0022:30；data.rs:16-17 | 枚举 | FixedBars{43200, 1m}（l3_delta_r_alpha.rs:329-332；块界不对齐日历月，只限 1m 粒度） | `[新缠论:选择]`（:79） |
| min_events | 30 | ADR 0022:47 | 整数 ≥ 2 | 50（econ_positive/tests.rs:4069；更晚出读数） | `[新缠论:选择]`（:82） |
| min_blocks | 12（以 block_unit 计） | ADR 0022:48 | 整数 ≥ 2 | 无其他有出处值 | `[新缠论:选择]`（:82） |
| min_windows | 3 | ADR 0022:56 | 整数 ≥ 2 | 无其他有出处值 | `[新缠论:选择]`（本节补标） |
| min_coverage | 1/2 | ADR 0022:58 | 整数对，∈ (0, 1] | 无其他有出处值 | `[新缠论:选择]`（本节补标） |

字段说明：

- confidence：下界取 bootstrap 分布的 (1−confidence) 分位，与裁定三「5% 分位」同式。
- block_unit = FixedBars{43200, 1m}：24/7 品种一块恰为 30 天，有休市品种一块跨 ≥ 30 个日历日，块长不短于月级簇，满足裁定三「簇稳健必须」；粒度不是 1m 即拒收。代价：块界不对齐日历月，`rust/src/bin/p907_cost_gate_level.rs:120`、`:124` 的月簇诊断不能直接对照。
- min_events / min_blocks 下限 2：低于此 bootstrap 退化（同 metrics.rs:346）。
- min_windows 下限 2：只开 1 窗时「每窗各自过线」退化成单点，等于取消固定规则。

排除项（ADR 0041:30 真错，写明依据）：

- block_unit = EventCount{n}（如 metrics.rs:186 的按 5 笔分块）：不按时间聚簇，挡不住月级簇，违反裁定三（:31）「簇稳健必须」。该取法只在研发回测的 C-19 记录里保留。
- block_unit = 日历周：仓内与 tracker 均无先例，且会切开月级簇，不列入。

- 固定规则（不入字段）：单边（零假设速率 ≤ 0 为单侧，ADR 0022:24，[新缠论:推论]）；下界不用点估计；窗口互不重叠；每窗各自过线不看合计。后三条若做成字段，即与现行判据构成同一判断的宽严两档（#799；ADR 0041:13、:30）。
- 裁定七溯源补标：四条结构 `[新缠论:推论]`（qujiantao.md:994「不是新偏好」）；「3」与「50%」`[新缠论:选择]`（原文与推导均未给出数目）。
- 计数单位：引 [ADR 0036](0036-operating-statistics-sampling-and-post-gate-parameter-protocol.md):13、:22，不另立——样本 = (重, S 稳定点) 操作，统计单位 = 重。
- 「全史」：读数所用数据集 [首 bar, 末 bar]，读数中写出。
- 「分层」不入字段：它只出现在 [ADR 0026](0026-scan-unify-defense-lean-mirror.md):27 的镜像签收清单里，ADR 0022 无此项。
- 窗口边界不入字段：每份读数自带预登记窗口表，读数前冻结，与版本号写在一起；默认按全史等分成 min_windows 份，不等分时写理由（ADR 0015:121-125；`prereg_windows.rs` 体例）。
- 不绑 profile：所有级别、所有 profile 共用同一版本记录，读数自带 profile_id（#804，AGENTS.md:66）。
- 与 C-19 分开：evidence_gate 只装 ADR 0022 门的六个字段。研发回测验收常数（AUDIT.md:180，C-19）判据与采样单位都不同，由 [#1605](https://github.com/xy7365527-lang/NewChanlun/issues/1605) 另立具名记录，`decontam` §4 预注册值原样登记；合成一份会让无关改值触发「不同版本不可比」（ADR 0041:48）。

### 2. v1 记录实例

```evidence-gate-record
policy_id: #1677/evidence-gate/v1
version: v1
supersedes: none
confidence: 95/100
block_unit: CalendarMonthUTC
min_events: 30
min_blocks: 12
min_windows: 3
min_coverage: 1/2
authorizer: 编排者（#853 resolution 5289198250 第 3/6/7 条）；记录化 #1677
authorized_at: 2026-08-14
```

### 3. 生效

v1 的值就是 ADR 0022 在 2026-08-14 预登记的值（[#853 resolution](https://github.com/xy7365527-lang/NewChanlun/issues/853#issuecomment-5289198250)），不属于 ADR 0041:21 的签署门清单，不处于 `AwaitingPolicy`。evidence_gate 不在 #1323「门槛由用户签署」之列：v1 的值由 #853 裁定，ADR 0036:5 已声明不动。**v2 起改任一字段须用户签署**；签署前新版本不生效，旧版本继续有效。票面所称「字段未签」统一读作「记录缺失、字段缺失或值越界」（见第 5 条）。

### 4. 版本规则

- 版本规则：
  - 每份验收读数头部写：
    - `evidence_gate_ref: #1677/evidence-gate/vN`
    - `run_started_at`（本份读数运行首次读取评价数据的时点；同一数据集过去被别的运行碰过不算）
    - 预登记窗口表及其冻结时间戳 `prereg_frozen_at`
    - 全史区间
  - `authorized_at ≥ run_started_at` 或 `prereg_frozen_at ≥ run_started_at` ⟹ 拒收：门的版本与预登记块都必须早于本次运行读数据（ADR 0015:121「新判据的参数必须在重算任何旧实验之前钉死」、:125「先落盘，再跑数」；ADR 0036:32 版本早于启用的同族时间序）。禁的是读数后换尺子，不是数据被碰过；v1 对既有数据集（含 `prereg_windows.rs` 冻结窗口与 BTC 全史）照常可用。
  - 改任一字段 ⟹ 出 vN+1，`supersedes: vN`，只对其 authorized_at 之后启动的读数生效。
  - 旧读数保留原版本号，不回算、不覆盖；跨版本比较必须并列标出版本号（ADR 0041:48）。

### 5. 缺失语义

- 缺失即报错：记录缺失、字段缺失或值越界 ⟹ 显式报错并停止，不出验收读数；不回退 ADR 0022 正文常数，也不回退任何备用值（#799；ADR 0041:15）。门的实现不得把这六个值写成代码常数（ADR 0041:25）。

### 6. 与 cost-stop 记录对接

- 等同声明：[ADR 0044](0044-cost-stop-policy-signed-instance-v1.md) 两份 v1 记录里的 `evidence_gate: ADR 0022 裁定六/七` 按值等同 `#1677/evidence-gate/v1`（含 confidence、block_unit），不出 cost-stop v2，ADR 0044 正文不动。ADR 0035:12 读法中「单边 95% 下界（按月 block bootstrap）」改读「所引 evidence_gate 版本的 confidence 与 block_unit」，该改写归 [#1598](https://github.com/xy7365527-lang/NewChanlun/issues/1598) A-10。ADR 0035/0034/0036 字段名里带「月」（`max_month_cluster`、「最大月簇」、`Formal{…, months, …}`）的问题，留给各自下次改签。

### 7. 受影响代码清单

- 受影响代码：main `66c409e7e3` 重核，分两栏。
  - (a) evidence_gate 的消费点：main 上没有实装。下列查询均在 `rust/ src/ tests/ trading_system/ scripts/` 上执行：
    - `rg -n "ADR 0022|adr/0022"`：0 命中
    - `rg -n "evidence_gate|min_events|min_blocks|min_windows|min_coverage|block_unit"`：0 命中
    - 待接读点：[#1398](https://github.com/xy7365527-lang/NewChanlun/issues/1398)（正文 :12「原样实现单边95%下界、日历月block bootstrap……」）、[#1410](https://github.com/xy7365527-lang/NewChanlun/issues/1410)（ADR 0036:30）、[#1441](https://github.com/xy7365527-lang/NewChanlun/issues/1441)（ADR 0035 A_ℓ 下界）
    - 诊断参照（不是门）：`rust/src/bin/p907_cost_gate_level.rs:120`、`:124`
  - (b) 不消费本门，归 #1605 的 C-19 记录（以下均在 `rust/src/theta_v0/backtest/` 下）：
    - `metrics.rs:132`、`:186`、`:221`、`:401`、`:562`、`:677`、`:718`
    - `highlow_mu.rs:113`
    - `decontam.rs:12-13`、`:168-169`
    - `l3_delta_r_alpha.rs:109`、`:332`、`:1317`、`:1601`、`:2228-2229`
    - `l3_pi_probe.rs:38`
    - `l3_fullwindow.rs:73`、`l3_pi_falsify.rs:74`
  - 实施由 #1605 承接：建 evidence_gate 读取器，另立 C-19 记录。
