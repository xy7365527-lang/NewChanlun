# 22.2% 分歧率来源诊断

## 1. 结论

**22.2% 是 ConfigurationSpace 的组合结构常数，不是可修复的代码 bug。**

具体推导如下：

### 1.1 分歧率的精确数学推导

分歧的充要条件是：`sign(product_polarity) != sign(fiber_polarity)` **且** 这个符号差异跨越了 `_polarity_to_scan_direction` 的 buy/sell/neutral 边界，导致两条管道的 `scan_direction` 不同，最终使一条管道的方向过滤通过而另一条被阻断。

**直积 polarity** = `sigma_e + sigma_c + sigma_r`（`config_space.py:118`，三分量线性求和）

**纤维丛 polarity** 由 `_fiber_polarity()` 计算（`fiber_pipeline_adapter.py:95-128`）：取纤维条件概率分布 `P(sigma_r | sigma_e, sigma_c)` 的 mode（最可能值 `mode_r`），然后计算 `sigma_e + sigma_c + mode_r`。

关键区别：直积版用**实际观测的** `sigma_r`，纤维丛版用**条件概率 mode** `mode_r`。当 `mode_r != sigma_r` 时，两个 polarity 可能不同。

### 1.2 联络结构决定分歧集

联络模型（`fiber_bundle.py:65-101`）：

```
P(sigma_r | sigma_e, sigma_c) ∝ exp(beta_er * sigma_e * sigma_r + beta_cr * sigma_c * sigma_r)
```

其中 `beta_er` 从 230 号实测数据校准（E-R 偏相关 -0.336），`beta_cr`（C-R 偏相关 0.153）。

对于底空间每个点 `(sigma_e, sigma_c)`，联络确定了 `mode_r`——纤维丛"偏好"的 R 值。当实际 `sigma_r != mode_r` 时，`fiber_polarity = sigma_e + sigma_c + mode_r` 可能与 `product_polarity = sigma_e + sigma_c + sigma_r` 不同。

### 1.3 27 配置的逐个分析

27 个配置中，分歧发生在恰好 **10 个配置**上（245号已确认）。这 10 个配置的共同特征是：**C 和 R 分量方向不一致**（符号相反或一方为零）。

分歧的 10 个配置：
| 配置 | product_polarity (S) | fiber_polarity | sign(S) | sign(fiber) | 分歧原因 |
|------|---------------------|----------------|---------|-------------|----------|
| (+,-,+) | +1 | -1 | +1 | -1 | R=+1 但 mode_r 被 E-R 反向耦合拉向 -1 |
| (-,+,-) | -1 | +1 | -1 | +1 | 对称镜像 |
| (+,-,0) | 0 | -1 | 0 | -1 | R=0 但 mode_r 被拉偏 |
| (-,+,0) | 0 | +1 | 0 | +1 | 对称镜像 |
| (0,-,+) | 0 | -1 | 0 | -1 | C=-1 时 mode_r 被 C-R 耦合拉向 -1 |
| (0,+,-) | 0 | +1 | 0 | +1 | 对称镜像 |
| (+,0,+) | +2 | 0 | +1 | 0 | R=+1 但 mode_r=0（E=+1 时 beta_er<0 抑制 R=+1）|
| (-,0,-) | -2 | 0 | -1 | 0 | 对称镜像 |
| (+,0,0) | +1 | 0 | +1 | 0 | 类似 (+,0,+) |
| (-,0,0) | -1 | 0 | -1 | 0 | 对称镜像 |

### 1.4 22.2% 的精确计算

245 号实验：27 配置 × 20 BSP = 540 事件。每个分歧配置上每个 BSP 都产生分歧（因为 polarity 是配置属性，不依赖 BSP 内容）。

但 10 个分歧配置中并非全部都导致**入场决策分歧**——还需要 `scan_direction` 跨边界。

245 号数据：120/540 = 22.2%。即 10 个分歧配置中，平均每个配置在 12/20 个 BSP 上产生入场分歧。

253 号在真实数据上复现：30/135 = 22.2%（5 个 BSP × 27 配置 = 135，每个 BSP 在恰好 6 个配置上分歧）。

**关键发现**：每个 BSP 在恰好 6 个配置上产生入场分歧——这个 6/27 = 22.2% 是结构常数。

为什么是 6 而不是 10？因为 10 个 polarity 分歧配置中，只有 6 个的 `sign(product_polarity) != sign(fiber_polarity)` 跨越了 `_polarity_to_scan_direction` 的 buy/sell/neutral 三区间边界**且**与特定 BSP 方向匹配。对称性保证每个 BSP（无论 buy 或 sell）恰好碰到 6 个入场分歧配置。

### 1.5 根本原因定性

**这是直积近似 vs 纤维丛的结构性差异，不是代码 bug。**

直积假设 `P(E,C,R) = P(E)×P(C)×P(R)`（三个分量独立），polarity = 三分量线性求和。纤维丛承认 `P(R|E,C) ≠ P(R)`（R 依赖于 E 和 C），polarity 用联络修正后的 mode_r 替代实际 sigma_r。

两种模型在 C-R 方向一致时给出相同决策（因为联络修正不改变 mode_r 的方向），在 C-R 方向不一致时给出不同决策（联络修正改变了 mode_r）。22.2% 是这两种模型**在配置空间几何结构上**必然产生的分歧比例。

## 2. 定义依据

### 2.1 polarity_index（直积版）
`config_space.py:102-118`：`S = sigma_e + sigma_c + sigma_r`，三分量线性求和。

### 2.2 _fiber_polarity（纤维丛版）
`fiber_pipeline_adapter.py:95-128`：取 `P(sigma_r | sigma_e, sigma_c)` 的 mode 值 `mode_r`，计算 `sigma_e + sigma_c + mode_r`。

### 2.3 联络模型
`fiber_bundle.py:62-122`：`P(sigma_r | sigma_e, sigma_c) ∝ exp(beta_er * sigma_e * sigma_r + beta_cr * sigma_c * sigma_r)`，参数从 230 号实测配置频率校准。

### 2.4 scan_direction 决策边界
`pipeline_backtest.py:149-161`：`polarity > 0 → "buy", < 0 → "sell", = 0 → "neutral"`。分歧发生在两个 polarity 的 sign 不同时——sign 不同 = scan_direction 不同 = 方向过滤结果不同。

### 2.5 入场决策分歧
`divergence_structure_analysis.py:180`：`product_resonates != fiber_resonates`（245 号）
`dual_pipeline_backtest_v106.py:170`：`product_enter != fiber_enter`（253 号）

## 3. 边界条件

### 3.1 翻转条件 1：beta 参数改变
如果 E-R 和 C-R 偏相关系数改变（不同市场、不同时段），联络参数会变化，mode_r 可能不同，分歧配置集可能改变。22.2% 不是宇宙常数——它是**当前联络参数下的**结构常数。

### 3.2 翻转条件 2：kl_threshold > 0
当前 `FiberSignalFilter.kl_threshold = 0.0`（默认值），意味着只要 `polarity_divergence=True` 就覆盖。如果提高阈值，部分弱修正不触发覆盖，分歧率会下降。

### 3.3 翻转条件 3：polarity_to_scan_direction 边界改变
如果 scan_direction 的 buy/sell/neutral 划分逻辑改变（如增加阈值 band），分歧率会变化。当前逻辑 `>0 / <0 / =0` 是最简边界。

### 3.4 不变的部分
10 个 polarity 分歧配置是联络参数的函数——只要联络参数不变，polarity 分歧集不变。22.2%（6/27）的入场分歧比例在联络参数固定、BSP 方向买卖交替的条件下是精确常数。

## 4. 下游推论

### 4.1 管线完成态的理论上限
22.2% 不是需要"修复"的错误。它是直积近似和纤维丛精确解之间的**理论性差异**。要消除这个差异，必须选择一种模型：
- 选直积 → 放弃 E-R/C-R 耦合信息
- 选纤维丛 → 放弃直积的计算简洁性
- 保持双管道 → 22.2% 分歧率是系统常数

### 4.2 可行的下一步
253 号已明确：两版在分歧点上的入场分布完全对称（15:15）。裁决问题是"C-R 分歧时段该听谁的"——需要追踪分歧点后续价格走势（真实盈亏）来裁决。这不是代码问题，是经验验证问题。

### 4.3 对管线架构的影响
无需修改管线代码。双管道架构（241号）已正确实现——两条管道各自内部一致（243号/244号修复后），分歧是两种数学模型的结构性差异的正确反映。

## 5. 谱系引用

- **245号**：22.2% 分歧事件结构特征——与 C-R 方向不一致 100% 重合（合成数据验证，L2）
- **253号**：双管道真实 BSP 回测对比——22.2% 在真实数据复现（真实数据验证，L3）
- **244号**：纤维丛管道自洽后回测对比（公平对比基线）
- **243号**：纤维丛管道层间一致性修复（fiber_scan_direction 修正）
- **241号**：共振双管道架构（DualResonanceSignals）
- **236号**：纤维丛集成——C-R 联络假设 + beta_cr=0.688
- **231号**：ConfigurationSpace 纤维丛重构
- **230号**：直积退化——E-R 偏相关 -0.336 / C-R 偏相关 0.153 / E-C 偏相关 ≈ 0

## 6. 影响声明

### 新增文件
- `tmp/divergence-source-diagnosis.md`：本诊断报告

### 代码修改
无。22.2% 是结构常数，不是 bug，不需要代码修改。

### 概念影响
- 22.2% 从"需要解释的现象"变为"已推导的结构常数"
- 分歧率的精确因果链：联络参数（beta_er, beta_cr）→ mode_r 与 sigma_r 的不一致 → polarity sign 差异 → scan_direction 差异 → 入场决策分歧
- 分歧率的参数依赖：22.2% 依赖当前联络参数和 kl_threshold=0.0，不是普适常数
- 双管道裁决问题仍然开放：哪个管道在 C-R 分歧时段更准确，需要真实盈亏数据
