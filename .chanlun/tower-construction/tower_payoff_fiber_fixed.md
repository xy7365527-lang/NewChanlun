# payoff 纤维完全分类（修复版）：payoff = case 空间 Σ' 上的 L3 纤维值，分类层只给 L0 结构判据（task#15，塔-子11）

**工位**：tower-sub11-payoff（topo_address: swarm/tower-formalize/sub11-payoff | parent_callback: tower-formalize）
**任务**：#15（修复 codex #95 对子8 payoff 纤维完全分类的四 FAIL 判决）
**认识论等级**：payoff 纤维的**结构层 L0**（哪些 case 是结构孤儿 = 必失血，从 H¹ 闭合循环 + 区间套递归推导，不依赖数据）+ **payoff 符号 L3 未决**（同一可操作 case 在 regime 不利时也失血 = regime 函数，OKLO seg-short 正/GC seg-short 负）。**严守分类层 L0 ≠ payoff 层 L3 的诚实分层**——分类层只声明结构判据，不声明 payoff 符号。
**约束遵守**：no-patch-mentality（c=f(L_confirm) 声明膨胀须删重写，§二把 c 的自变量集据实改为整个 case 向量 + regime，删除"c=f(L_confirm)"单变量声明）+ formalization-validity-domain（L0/L2/L3 严格标注，§六）+ result-package 六要素（§五）+ 生成性完备（payoff 纤维分类 = case 空间 Σ' 上的纤维丛，生成所有 case 含边界，不断言完备 + 补丁，§四主动吐出对角线/可操作但失血两个边界 case）。
**承接（已读，不重推）**：子8 原文件丢失（工作树态），payoff 概念内容从以下重建——tower_intrinsic_quota.md §3.2 split 自适应定理（payoff 讨论）+ §三 + payoff-regime-investigation-75（高级别逆势僵尸空腿 = 支配失血分量，OKLO 反例）+ tower-sub9-operability-93（可操作性 = L_confirm ⪯ L_move = α*>0 布尔前驱）+ tower-sub10-continuous-94（δ 连续轴）+ codex #91/#95/#96（G' 维度 + L_confirm 独立轴判死）。

---

## 原子性自评（sub-swarm-ceremony 第一步）

**判定：原子（atomic），直接做，不再 TaskCreate 子 DAG。**（TaskUpdate #15 owner=tower-sub11-payoff in_progress）

理由（why-atomic）：四个 FAIL 点不是 ≥2 个独立子任务，而是**同一个 payoff 纤维丛的四个不可分割面**：
- FAIL① 失血⟸孤儿单向 与 FAIL③ 降级声明膨胀是**同一个区分的两个侧面**——结构孤儿（L0）⊊ 失血（L3 含 regime payoff 失血）。把"结构孤儿失血"与"regime payoff 失血"分开正是消除"孤儿⟺失血等价"和"可操作⟹超BH降级"两个声明膨胀的**同一构造**。
- FAIL② c=f(L_confirm) 的自变量集检验**依赖** payoff 纤维的丛结构定义——只有先确定 payoff 是 case 空间 Σ' 上的纤维值（底空间 = 整个 case 向量，纤维 = payoff 标量 × regime），才能据实声明 c 的自变量是整个 case 向量而非仅 L_confirm。
- 三者共享同一构造前提：**payoff 是建在形态层 case 空间 Σ' 之上的纤维丛**（Σ' 由 #90/#94/#96 生成，本任务不重建 Σ'，只构造其上的 payoff 纤维）。

强拆 = 把一个纤维丛拍扁成三个碎片 = 正是 576号/#80 判定的「纤维拍扁 = 单分量修复必在其它分量错配」的元层重演（违反编排者硬约束「构造性」）。父工位 #79 已挂审查 #16 + 异质重测 #20（codex），覆盖 (c) 验证链。

---

## 〇、一句话结论

**payoff 不是 case 空间 Σ' 的一个维度，是建在 Σ' 之上的纤维丛 π: P → Σ'**——底空间 = 整个形态层 case 向量（级别 k × E_k × F_k × ρ_k × d_k × δ_k × completion_source_k，由 #90/#94/#96 生成），纤维 = payoff 标量 c × regime 参数。

三个 FAIL 的统一修复 = **严格分离两个失血概念 + 据实声明 c 的自变量集 + 守住 L0/L3 分层**：

```
结构孤儿失血（L0，分类层判据）：
  孤儿(case) ⟺ L_confirm ≻ L_move ∨ d_k=Up（永不产平腿买卖点，L_confirm 发散）
  孤儿 ⟹ 必失血（腿不闭合，留在 1-链层永不模掉边界，∮ 浮亏到 eod）   [L0 充分]

regime payoff 失血（L3，纤维值符号）：
  可操作(case) ⟺ L_confirm ⪯ L_move ∧ d_k=Down（腿可闭合，非孤儿）   [L0 结构前驱]
  但 可操作 ⇏ payoff>0：可操作的腿在 regime 不利（无足够下跌段）时 net<0   [L3 regime 函数]
  ⟹ 失血 ⊋ 孤儿：可操作的腿也可失血（regime payoff 失血），故失血 ⇍ 孤儿   [FAIL① 修复]

c 的真实自变量集（FAIL② 据实声明，删 c=f(L_confirm) 单变量膨胀）：
  c(case) = c( 整个 case 向量 ⟨E_k,F_k,ρ_k,d_k,δ_k,completion_k,α*_k⟩_{k=a0..r*}, regime )
         ≠ c(L_confirm) 单变量   [删重写，no-patch §禁止模式5]

分类层只声明结构判据，不声明 payoff 符号（FAIL③ 守 L0/L3）：
  分类层 L0 给：哪些 case 是结构孤儿（必失血）/ 哪些 case 结构可操作（payoff 符号 L3 未决）
  分类层 L0 不给：可操作 ⟹ 超 BH（这是 L3 regime 函数，OKLO 正/GC 负，降级声明=膨胀）
```

---

## 一、payoff 纤维丛的丛结构定义（修复 FAIL② 的构造前提）

### 1.1 payoff 不是 case 维度，是 case 空间上的纤维丛

**codex #91/#96 已确立**：形态层 case 空间 Σ' 的自由维度 = (级别 k) × (E_k) × (F_k) × (ρ_k) × (d_k) × (δ_k 区间套确认深度独立轴) × (completion_source_k BSP_kind 轴，#96 新增)。这是**形态层**——"在哪个级别、什么中枢事件、什么力度、什么角色、什么方向、确认到什么深度、由什么 BSP 源完成"。

**payoff 是建在 Σ' 之上的纤维丛**，不是 Σ' 的第八个维度：

```
纤维丛 π : P → Σ'
  底空间 Base = Σ'（形态层 case 空间，#90/#94/#96 生成）
  纤维 Fiber_c = (payoff 标量 net_c ∈ ℝ) × (regime 参数 r ∈ Regime)
  π(p) = 该 payoff 点所属的 case
```

**为什么是纤维丛而非维度（关键区分）**：
- case 维度（E_k/F_k/ρ_k/d_k/δ_k/completion_k）是**形态层结构读数**——它们由递归语法 + 区间套链**唯一生成**（给定走势树，每个 case 的维度值确定）。
- payoff（net_c）**不由形态层唯一确定**——同一个形态 case（同 ⟨E,F,ρ,d,δ,completion⟩）在不同 regime（OKLO 有下跌段 vs GC 强牛无下跌段）下 net 符号相反（payoff-regime §2.2：同 (seg, short) 分量 OKLO +70k 正 / GC −130k 负）。**payoff 在每个 case 上是一根纤维（取值随 regime 变），不是一个确定的标量。**

**∴ payoff 纤维丛的截面（section）s: Σ' → P 才是"每个 case 的实际 payoff"——而截面的选取依赖 regime（L3），不由底空间（形态层 L0）唯一决定。** 这是 FAIL② 和 FAIL③ 的共同构造根：payoff 是纤维值，底空间是 case 空间，纤维方向是 regime。

### 1.2 c 的真实自变量集 = 整个 case 向量 + regime（删 c=f(L_confirm) 单变量膨胀）

**codex #95 FAIL②（任务摘要）**：「payoff c（净收益纤维值）若声明 c=f(L_confirm)（净收益是确认级别的函数）须严格检验——若是声明膨胀（c 实际还依赖 L_pullback/d_k/regime，非仅 L_confirm）必须删重写。」

**检验 c 的真实自变量集（逐自变量据实核验，no-patch §禁止模式5 声明膨胀删除）**：

| 候选自变量 | 是否 c 的真实自变量 | 证据 |
|-----------|-------------------|------|
| L_confirm（区间套确认级别/深度 δ） | ✅ 是（但非唯一） | δ 决定确认时机 → 平腿在循环内闭合 vs 落循环外（孤儿）。但 δ 只决定**腿是否闭合**，不决定**闭合后 net 符号** |
| L_move / L_pullback（移动/回调级别） | ✅ 是 | 配对闭合判据 L_confirm ⪯ L_move 含 L_move；L_pullback 决定捕获跌幅大小（α*→0 vs >0），直接影响 net 量级 |
| d_k（级别方向） | ✅ 是 | d_k=Up 却开空 ⟹ L_confirm 发散 ⟹ 孤儿 ⟹ net 浮亏到 eod（payoff-regime §3.2 因子①级别-方向对齐） |
| ρ_k（核心/机动角色） | ✅ 是 | 核心腿 H⁰(long) 8/8 全正（骑主升浪）vs 机动腿 H¹(short) 高级别逆势失血——net 符号随角色翻转（payoff-regime §2.1/2.2） |
| completion_source_k（type1/type3 BSP 源，#96） | ✅ 是 | type3 真顶减仓 vs type1 背驰平腿，平腿时机不同 ⟹ 捕获跌幅不同 ⟹ net 不同（#96 codex 新反例） |
| regime（标的的下跌段结构，外生于形态层） | ✅ 是（**L3 维度**） | 同形态 case OKLO 正/GC 负（payoff-regime §2.2）——regime 是 net 符号的**决定性 L3 自变量**，不在形态层 Σ' 内 |

**∴ c 的真实自变量集 = 整个 case 向量 + regime**：

```
c(case) = c( ⟨E_k, F_k, ρ_k, d_k, δ_k, completion_k, α*_k⟩_{k=a0..r*},  regime )
```

**声明膨胀的删除（no-patch §禁止模式5：声明代码不具备的能力）**：「c=f(L_confirm)」是把 c 声明为**单变量函数**——这是声明 c 只依赖确认级别，实际 c 依赖整个 case 向量（含 d_k/ρ_k/L_pullback/completion）+ regime。**删除「c=f(L_confirm)」单变量声明，重写为整个 case 向量 + regime 的函数。** 这不是补丁（不在 c=f(L_confirm) 上加分支），是据实重写自变量集——L_confirm 是 c 的自变量之一（决定腿是否闭合），但远非唯一（net 符号由 d_k/ρ_k/regime 共同决定）。

**为什么 c=f(L_confirm) 是膨胀而非简化**：单变量声明会推出"调 L_confirm 就能改 payoff 符号"——但 payoff-regime §2.2 实测：高级别逆势空腿即使 δ 确认深度充分（L_confirm 可达），在强牛 regime 下仍失血（GC seg-short −130k），因为 d_k=Up 方向错 + 无下跌段（regime）。**L_confirm 充分 ≠ payoff>0**，故 c≠f(L_confirm)。

---

## 二、失血 ⟸ 孤儿是单向蕴含（修复 FAIL①：分离两个失血概念）

### 2.1 codex #95 FAIL① 的精确陈述

**任务摘要**：「失血 ⟸ 孤儿是单向蕴含，不是双向等价。子8 把"失血"与"孤儿"当等价（孤儿⟺失血），但实际是：孤儿⟹失血（孤儿必失血），但失血⇏孤儿（可操作的腿在 regime 不利时也失血=payoff层L3）。修复：分离"结构孤儿失血"（L0判据 L_confirm≻L_move）与"regime payoff失血"（L3，可操作腿净亏）。两者不是同一概念。」

### 2.2 严格分离两个失血概念（构造，非补丁）

**概念 A — 结构孤儿失血（L0，分类层判据）**：

```
孤儿(case) ⟺ ¬可操作(case)
          ⟺ L_confirm(case) ≻ L_move(case)   （确认太慢，配对 Y 落在 X 循环外）
          ∨ d_k=Up（永不产平空买卖点，L_confirm=∞，子9 §2 末）
孤儿 ⟹ 必失血   [L0 充分蕴含]
```

**L0 证明（孤儿 ⟹ 失血，从 H¹ 闭合循环推导，子9 §1.1 纳入）**：
- H¹_k 腿要成合法 H¹ 类（闭合循环），必须有配对闭合转折节点 Y（recover）在 X（sink）的循环内 fire。
- 孤儿 = 无配对 Y ⟹ 腿不闭合 ⟹ 不是 1-上闭链 ⟹ 留在 1-链层永不模掉边界（项目记忆 project_isolated_fugue_forest_verdict 孤儿不可能定理 L3 结算）。
- 腿不闭合 ⟹ ∮≠0 但无 recover 兑现 ⟹ 浮亏持续累积到强制平仓（eod/liq）⟹ **必失血**。
- **这是 L0**：从 H¹ 闭合循环代数结构推导，不依赖 regime，不依赖具体标的。孤儿是结构性必失血。□

**概念 B — regime payoff 失血（L3，纤维值符号）**：

```
可操作(case) ⟺ L_confirm ⪯ L_move ∧ d_k=Down（非孤儿，腿可闭合）   [L0 结构前驱]
但 可操作(case) ⇏ payoff(case)>0   [L3：payoff 符号是 regime 函数]
  ∃ regime（无足够下跌段，强牛）使可操作的腿 net<0（捕获跌幅 < 摩擦+滞后成本）
⟹ 失血(case) ⊋ 孤儿(case)：可操作的腿也可失血（regime payoff 失血）
```

**L3 证据（可操作腿在 regime 不利时失血，payoff-regime §2.2 纳入）**：
- 同一 (seg, short) 可操作 case（L_confirm ⪯ L_move 满足，d_k=Down 满足）：
  - OKLO（有足够下跌段 regime）：seg-short +70k **正**（捕获下行）。
  - GC（强牛无下跌段 regime）：同 case 类型在 GC 下若开仓则 net<0（捕获跌幅不足，回调即被强牛吞没）。
- **这是 L3**：payoff 符号随 regime 翻转（同形态 case OKLO 正/GC 负），不能从形态层 L0 推出，须真实数据。

### 2.3 失血 ⇍ 孤儿（FAIL① 核心：反向不成立）

**反向蕴含（失血 ⟹ 孤儿）不成立的反例**：

```
反例 case：可操作的机动腿在强牛 regime 下净亏
  L_confirm ⪯ L_move（确认够快，腿能闭合）∧ d_k=Down（方向对齐，开空合法）
  ⟹ 可操作（非孤儿，有配对闭合 Y）
  但 regime=强牛无下跌段 ⟹ 捕获跌幅 0.68% < 574 滞后成本 2.3%（project_selloff_root_cause_granularity）
  ⟹ 腿闭合了（非孤儿）但 net<0（失血）
  ⟹ 失血 ∧ ¬孤儿 ⟹ 失血 ⇍ 孤儿   □
```

**∴ 失血 ⊋ 孤儿（严格包含，非等价）**：
- 孤儿 ⊊ 失血：孤儿是失血的真子集（结构性必失血）。
- 失血 ∖ 孤儿 = regime payoff 失血（可操作腿净亏，L3 regime 函数）≠ ∅。

**子8 的等价声明（孤儿⟺失血）是声明膨胀**：它把 L3 的 regime payoff 失血**降级**为 L0 的结构孤儿（声称失血都是结构孤儿，可由 L0 判据 L_confirm≻L_move 完全刻画）——但 regime payoff 失血（可操作腿在不利 regime 净亏）不满足 L_confirm≻L_move（它 L_confirm⪯L_move 可操作），故 L0 判据刻画不了它。**修复 = 把等价 ⟺ 改为单向 ⟸，并显式声明失血 ∖ 孤儿 = regime payoff 失血是 L3 维度。**

### 2.4 两个失血概念的诊断接口（区分修复方向，纳入 §四 quota bleed_k 对接）

| 失血概念 | 等级 | 判据 | 修复方向 | 谱系 |
|---------|------|------|---------|------|
| **结构孤儿失血** | **L0** | L_confirm ≻ L_move ∨ d_k=Up（孤儿，腿不闭合） | 不开孤儿腿（仅 d_k=Down ∧ L_confirm⪯L_move 开 H¹_k）= 内在结构修复 | project_isolated_fugue_forest_verdict 孤儿不可能定理（L3 结算） |
| **regime payoff 失血** | **L3** | 可操作（L_confirm⪯L_move ∧ d_k=Down）但 net<0（无足够下跌段 regime） | 不在分类层修——payoff 符号是 regime 函数，分类层只标"可操作"（结构 L0），符号交 L3 实证 | payoff-regime #75 §2.2（OKLO 正/GC 负）/539 |

**关键（FAIL① 与 FAIL③ 在此对接）**：结构孤儿失血可由分类层 L0 判据消除（不开孤儿腿）；regime payoff 失血**不能由分类层消除**——它是可操作腿在不利 regime 的 net 符号，是 L3 纤维值。分类层若声称"消除失血⟹超 BH"，等于声称能消除 regime payoff 失血（L3），这是降级声明膨胀（FAIL③）。

---

## 三、分类层只给 L0 结构判据，不声明 payoff 符号（修复 FAIL③：守 L0/L3 分层）

### 3.1 codex #95 FAIL③ 的精确陈述

**任务摘要**：「payoff 分类层若把 L3 regime 函数「降级」声明为 L0/L2 完备（如"可操作⟹超BH"），是声明膨胀。须严格守 formalization-validity-domain：payoff 是 L3 regime 函数（同 (seg,short) OKLO 正/GC 负），分类层只给 L0 结构判据（哪级可操作），不声明 payoff 符号。」

### 3.2 分类层的有效域边界（formalization-validity-domain 强制）

**分类层（本任务）的定义域 = case 空间 Σ'（全形态 case）。分类层的有效域 = L0 结构判据**：

```
分类层 L0 声明（有效域内，可声明）：
  (a) 哪些 case 是结构孤儿（L_confirm≻L_move ∨ d_k=Up）⟹ 必失血（L0 充分）
  (b) 哪些 case 结构可操作（L_confirm⪯L_move ∧ d_k=Down）⟹ 腿可闭合（非孤儿，L0）
  (c) 可操作是 α*_k>0 的布尔前驱（子9 分离定理，L0）

分类层 L0 不声明（有效域外，声明=膨胀）：
  (d) ✗ 可操作 ⟹ payoff>0（错：可操作腿在不利 regime 净亏，L3 regime 函数）
  (e) ✗ 可操作 ⟹ 超 BH（错：超 BH 是 net 累积，L3，"自适应后超 BH"未决）
  (f) ✗ 消除孤儿 ⟹ 0/8 转正（错：消除结构孤儿失血≠消除 regime payoff 失血）
```

**为什么 (d)/(e)/(f) 是降级声明膨胀**：它们把 L3 的 regime 函数（payoff 符号随 OKLO/GC 翻转）**降级**声明为 L0/L2 的结构必然（可操作⟹赚）。formalization-validity-domain §禁止模式3「定义域=有效域假设：声称代数结构在全定义域上经验有效而未提供 L2+ 证据」——分类层 L0 的定义域是全 case 空间，但其有效域只到"结构可操作/孤儿"，不到"payoff 符号"。声称 L0 判据决定 payoff 符号 = 定义域=有效域膨胀。

### 3.3 payoff 符号是 L3 regime 函数（不降级，据实标注）

**payoff 截面 s: Σ' → P 的符号是 regime 函数（L3，§1.1 纤维丛承接）**：

```
对每个可操作 case c（结构 L0 判定可操作）：
  payoff 符号 sign(net_c) = sign( 捕获跌幅(c, regime) − 摩擦(c) − 滞后成本(c, L_confirm) )
  = regime 函数（L3）：
    regime=有足够下跌段（OKLO）⟹ 捕获跌幅 > 成本 ⟹ net>0
    regime=强牛无下跌段（GC）  ⟹ 捕获跌幅 < 成本 ⟹ net<0
  ⟹ 同形态 case 在不同 regime 下符号相反 ⟹ payoff 符号不可从形态层 L0 推出   [L3 未决]
```

**这与 #94 δ 连续轴的 L0/L3 分层一致**（tower-sub10-continuous-94 §六）：「分类层连续轴完备（定义域=有效域 L0）≠ payoff 层有效域（哪些 δ 位置盈利 L3）」。本任务的 payoff 纤维丛是该分层的 payoff 侧严格化——分类层给 case 空间（L0 完备），payoff 符号是建在其上的 L3 纤维截面。

**∴ 分类层只给 L0 结构判据（哪级结构孤儿/可操作），payoff 符号严守 L3 未决——不降级声明"可操作⟹超BH"。** FAIL③ 修复 = 删除任何"可操作⟹超BH/转正"的 L0 声明，payoff 符号显式标 L3 regime 函数。

---

## 四、生成性完备：payoff 纤维丛生成所有 case 含边界（不断言完备+补丁）

### 4.1 payoff 纤维丛的生成函数 G_payoff

**底空间 Σ' 由形态层生成函数 G'（#90/#94/#96）机械吐出**（本任务不重建 Σ'，承接）。payoff 纤维丛在 Σ' 上**逐 case 生成纤维**：

```
G_payoff : case c ∈ Σ' ↦ (失血结构类 ∈ {结构孤儿, 结构可操作}, payoff 纤维 net_c × regime)
  Step1（L0 结构类，分类层吐出）：
    读 c 的 ⟨d_k, δ_k(=L_confirm), L_move⟩：
      L_confirm ≻ L_move ∨ d_k=Up  ⟹ 结构孤儿（必失血，L0）
      L_confirm ⪯ L_move ∧ d_k=Down ⟹ 结构可操作（腿可闭合，L0）
  Step2（L3 纤维，不在分类层吐符号）：
    可操作 case 的 payoff 纤维 net_c(regime) 是 regime 函数（L3 截面，分类层只标"待 L3"）
```

G_payoff 是**生成性**的（非断言）：它不是"先列失血类再断言完备"，而是 case 向量 ⟨d_k, L_confirm, L_move⟩ 在结构判据下的**机械商**——每个 case 必落"结构孤儿 / 结构可操作"二分之一（L0），可操作 case 再带一根 regime 纤维（L3）。任何 case 经形态层生成后，必被 G_payoff 分类，无逃逸。

### 4.2 无遗漏证明（每个 case 可证属某失血类）

- 全部 case 由形态层 Σ' 生成（#90/#94/#96 已证 Σ' 无逃逸）。
- 每个 case 有确定的 ⟨d_k, L_confirm(δ_k), L_move⟩（形态层读数）。
- L0 结构判据是全序商（L_confirm vs L_move 在级别阶梯 L 上可比 + d_k 二值）⟹ 每个 case 必落「结构孤儿（≻ ∨ d_k=Up）/ 结构可操作（⪯ ∧ d_k=Down）」之一。
- **无第三结构类**：⪯/≻ 全序二分 + d_k 二值穷尽。可操作 case 上的 payoff 纤维（L3）是纤维方向，不增加底空间结构类。□

### 4.3 主动枚举两个未被指出的边界 case（生成性证据，feedback_generative_completeness_not_asserted）

生成函数 G_payoff 吐出**两个连任务摘要都未显式列举的边界 case**，作为完备性证据（不是被指出后补的分支）：

**边界 case ①「可操作但 regime 失血」（失血 ∖ 孤儿 的非空证据，§2.3 反例的生成式来源）**：
- G_payoff Step1 判：L_confirm ⪯ L_move ∧ d_k=Down ⟹ 结构可操作（非孤儿）。
- G_payoff Step2 判：regime=强牛无下跌段 ⟹ net_c<0（L3）。
- **这是 (结构可操作 ∧ 失血) case**——它**证明失血 ⊋ 孤儿**（§2.3）。它不是补丁，是 G_payoff 的底空间「结构可操作」× 纤维方向「regime 不利」的笛卡尔积的一个自然格点。**子8 等价声明（孤儿⟺失血）漏掉的正是这个格点——它在 Σ'×Regime 上必然存在（可操作 case 非空 × 不利 regime 非空），G_payoff 机械吐出。**

**边界 case ②「对角线 L_confirm=L_move 的临界 payoff」（承接子9 §4.3 对角线 case 的 payoff 侧）**：
- G_payoff Step1 判：L_confirm = L_move（对角线，弱序含等号 ⟹ 满足 ⪯）⟹ 结构可操作（腿勉强闭合，配对 Y 在循环末端 fire）。
- G_payoff Step2 判：捕获跌幅 ∮ → 0（绕环但回调已几乎走完）⟹ net_c → 0（临界，无论 regime 都近零）。
- **这是 (结构可操作 ∧ payoff→0) case**——它揭示 payoff 纤维在底空间「对角线」上的退化（截面值趋零）。它区别于边界 case ①（①是可操作但 net<0 失血，②是可操作但 net→0 no-op）。**两个边界 case 共同证明：可操作（结构 L0）下 payoff 纤维（L3）有三种符号——net>0（regime 有利）/ net<0（regime 不利失血，边界①）/ net→0（对角线 no-op，边界②）。可操作 ⇏ payoff>0 由生成函数三符号吐出，非断言。**

### 4.4 完备性自检（枚举 (失血结构类 × payoff 符号 × regime) 组合，无遗漏）

| 结构类（L0） | payoff 符号（L3 纤维） | regime | G_payoff 吐出？ | 失血？ |
|------------|---------------------|--------|----------------|-------|
| 结构孤儿（L_confirm≻L_move ∨ d_k=Up） | net<0（必） | 任意 regime | ✅ Step1 | ✅ 必失血（L0） |
| 结构可操作（⪯ ∧ d_k=Down） | net>0 | 有足够下跌段（OKLO） | ✅ Step2 截面正 | ❌ 赚 |
| 结构可操作 | net<0 | 强牛无下跌段（GC，**边界①**） | ✅ Step2 截面负 | ✅ regime payoff 失血（L3） |
| 结构可操作（对角线 L_confirm=L_move） | net→0 | 任意（**边界②**） | ✅ Step2 截面零 | ≈ no-op（开≈不开） |

**结论：G_payoff 无未覆盖组合。底空间（全 case）= 结构孤儿 ⊔ 结构可操作（L0 二分穷尽）；可操作 case 上 payoff 纤维三符号（>0/<0/→0）由 regime 截面吐出（L3）。失血 = 结构孤儿（必，L0）⊔ {结构可操作 ∧ net<0}（regime 函数，L3）⊋ 孤儿。** 顶部 r* 与任意级别自相似（结构判据 L_confirm⪯L_move 对所有 k 同一，§承接子9 §3 / #94 §2.3），无边界例外。

---

## 五、result-package 六要素

### 1. 结论
payoff 不是 case 空间 Σ' 的维度，是建在 Σ' 之上的纤维丛 π: P→Σ'（底空间 = 形态层 case 向量，纤维 = payoff 标量 × regime）。三 FAIL 统一修复：**①失血 ⟸ 孤儿是单向蕴含**——结构孤儿失血（L0，L_confirm≻L_move ∨ d_k=Up，必失血）⊊ 失血；失血 ∖ 孤儿 = regime payoff 失血（L3，可操作腿在不利 regime 净亏，OKLO 正/GC 负），故失血 ⇍ 孤儿。**②c 的真实自变量集 = 整个 case 向量 + regime**，删除「c=f(L_confirm)」单变量声明膨胀（L_confirm 是 c 自变量之一决定腿是否闭合，但 net 符号由 d_k/ρ_k/L_pullback/completion/regime 共同决定）。**③分类层只给 L0 结构判据**（哪级结构孤儿/可操作），不声明 payoff 符号（"可操作⟹超BH"是降级声明膨胀，payoff 符号是 L3 regime 函数严守未决）。生成函数 G_payoff 机械吐出所有 case（结构孤儿 ⊔ 结构可操作 L0 二分 + 可操作上 payoff 纤维三符号 L3），主动吐出两个边界 case（可操作但 regime 失血 = 失血⊋孤儿证据 / 对角线 net→0 = no-op），证可操作 ⇏ payoff>0。

### 2. 定义依据
- **payoff 纤维丛**：[[project_trend_dev_two_source_classification]]（踏空=纤维拍扁；payoff 标量=纤维投影同构）+ #94 §六（分类层 L0 ≠ payoff 层 L3 分层）。底空间 Σ' = #90/#94/#96 形态层 case 空间（E_k/F_k/ρ_k/d_k/δ_k/completion_k）。
- **结构孤儿（L0）**：子9 tower-sub9-operability-93 §1.3（可操作 ⟺ L_confirm⪯L_move）+ §2（孤儿=不可操作=α*>0 否定）；project_isolated_fugue_forest_verdict（孤儿不可能定理，孤儿=失血=违反闭合，L3 结算）。
- **regime payoff 失血（L3）**：payoff-regime #75 §2.2（同 (seg,short) OKLO +70k 正 / GC −130k 负 = G 轴 regime 函数）+ project_selloff_root_cause_granularity（捕获跌幅 0.68% < 574 滞后 2.3% ⟹ 可操作腿净亏）。
- **c 自变量集**：payoff-regime §3.2 三因子（级别-方向对齐 d_k ∧ 574 确认滞后 L_confirm ∧ 强平可达性）+ #96（completion_source type1/type3 影响平腿时机）+ #91（L_confirm 是独立内生量非导出）。
- **输入满足定义条件**：每个 case 由形态层读出 ⟨d_k, L_confirm(δ_k), L_move⟩（rec_engine.rs:113-121 区间套链 + tree.levels[k].direction），满足结构判据二分；payoff net_c 由 fugue_v3_<SYM>.json per-leg dump 读出（L3 实证），随 regime 翻转。

### 3. 边界条件（结论翻转）
- 若 **payoff 符号能从形态层 L0 唯一推出**（同形态 case 在所有 regime 下符号一致）⟹ payoff 退化为 case 维度（非纤维丛），FAIL③ 分层消失。当前 payoff-regime §2.2 实测同 (seg,short) OKLO 正/GC 负，符号随 regime 翻转，此翻转不成立。
- 若 **失血都是结构孤儿**（可操作腿在任何 regime 都 net≥0）⟹ 失血 ⟺ 孤儿（等价恢复），FAIL① 修复回退。当前边界 case ①（可操作但强牛 regime net<0，payoff-regime §3.1 selloff 捕获<成本）使等价不成立。
- 若 **c 真的只依赖 L_confirm**（d_k/ρ_k/regime 对 net 符号无影响）⟹ c=f(L_confirm) 成立，FAIL② 删除回退。当前 payoff-regime §3.2 三因子合取（d_k 方向 + L_confirm 滞后 + regime 下跌段）使单变量声明不成立。
- 若 **分类层声明"可操作⟹超BH"被 L3 证实**（内在 split 实装后可操作 case 全标的转正）⟹ FAIL③ 降级声明非膨胀（L0 判据决定 payoff）。当前"自适应后超 BH"L3 未决（tower_intrinsic_quota §3.3），不声明。
- 若 **区间套链在 a0=segment 尺度坍缩**（554号 L3，#94 §六）⟹ L_confirm 读数在底部断裂 ⟹ 结构孤儿判据在该尺度读不出（退化）。本任务标记此 L3 约束（结构判据 L0 在 a0 尺度有效域 L3 未决，承接 #94）。

### 4. 下游推论
- **子4 #83 配额 α*_k（已纳入本任务区分）**：bleed_k 孤儿诊断量（tower_intrinsic_quota §4）= 结构孤儿失血（L0）的逐级指认；regime payoff 失血（L3）不进 bleed_k（它不是结构孤儿，是可操作腿的 net 符号）。诊断量 bleed_k 只诊断结构孤儿（L0），payoff 符号交 L3 实证——这与本任务 §2.4 诊断接口一致。
- **子5 #84 实装映射**：可操作性门（is_rstar_pullback_leg 读 d[rstar] + L_confirm⪯L_move）= 结构孤儿过滤（L0，不开孤儿腿）；但实装**不能**在分类层加 payoff 符号门（"仅 net>0 才开"=用 L3 信息做 L0 门=未来函数/降级膨胀）。实装只过结构孤儿，payoff 符号是回测 L3 输出。
- **审查节点 #16**：核验 payoff 纤维丛的 L0/L3 分层无降级声明（"可操作⟹超BH"已删）+ 失血⟸孤儿单向无等价残留 + c 自变量集据实（无 c=f(L_confirm) 单变量）。
- **异质重测 #20（codex）**：质询点见 §七。

### 5. 谱系引用
- **574（确认滞后形式化解）**：L_confirm 是 c 的自变量之一（决定腿是否闭合），但 payoff-regime §3.2 精化——574 在 payoff 维度不是"浅回调捕获<成本"单变量，是"d_k 方向 ∧ L_confirm 滞后 ∧ regime 下跌段"三因子合取，故 c≠f(L_confirm)。
- **539 / [[project_t_short_leg_regime_function]]**：d_k 级别-方向失配（卖点误读为方向）= 结构孤儿（d_k=Up 开空 ⟹ L_confirm=∞）的根因，是 c 的自变量 d_k。
- **payoff-regime #75 §2.2**：同 (seg,short) OKLO 正/GC 负 = regime payoff 失血（失血 ∖ 孤儿）的 L3 实证根，是 FAIL① 反向不成立的证据。
- **561（Ω 坍缩完全分类）b2（净收益 L3 未决）**：payoff 纤维各分量的 G 轴 regime 函数 = 本任务 payoff 截面符号 L3 未决，不擅自结算 561 b2。
- **[[project_trend_dev_two_source_classification]]**：踏空=纤维拍扁；payoff 标量=纤维投影同构 + payoff 纤维丛是 case 空间（E⊗F×可操作性）之上的 L3 截面。
- **tower-sub9-operability-93 / tower-sub10-continuous-94**：可操作性 ⟺ L_confirm⪯L_move（结构孤儿 L0 判据）+ 分类层 L0 ≠ payoff 层 L3 分层（本任务严格化 payoff 侧）。
- **codex #91/#95/#96**：L_confirm 独立轴 + δ/completion_source 轴 = 底空间 Σ' 维度（本任务承接，payoff 是其上纤维）。
- **谱系不确定声明**：本任务是 payoff 纤维丛构造（L0 结构层 + L3 符号分层），**不新增谱系节点**（不擅自结算 561 b2）。**潜在新分离**：「结构孤儿失血（L0）⊊ 失血（含 regime payoff 失血 L3）」+「payoff 纤维丛底空间=形态层 case 空间，纤维方向=regime」若经异质重测（#20 codex）确认，可结晶——本任务仅构造，不结晶。

### 6. 影响声明
- **不改动代码或定义**（L0 结构层 + L3 分层的理论构造，实装映射=子5 #84）。
- **新增** `tmp/tower_payoff_fiber_fixed.md`（本文件）+ `.chanlun/review-results/tower-sub11-payoff-fix-20260625.md`（落盘版）。
- **影响判断层面**：
  - **修复 FAIL①**：失血 ⟸ 孤儿改为单向蕴含（删等价），显式声明失血 ⊋ 孤儿（失血 ∖ 孤儿 = regime payoff 失血 L3）。
  - **修复 FAIL②**：删除 c=f(L_confirm) 单变量声明膨胀，据实重写 c 的自变量集 = 整个 case 向量 + regime（no-patch §禁止模式5）。
  - **修复 FAIL③**：分类层只给 L0 结构判据（结构孤儿/可操作），删除"可操作⟹超BH"降级声明，payoff 符号严守 L3 regime 函数未决。
  - **守护** 561 b2 不擅自结算 + payoff 符号 L3 未决（不声明自适应后超 BH）。
  - **重构** payoff 从"case 维度/单变量函数"为"case 空间上的纤维丛 + 整个 case 向量 + regime 的函数 + L0/L3 严格分层"。
- **不影响**：引擎行为（无代码改动）、底空间 Σ' 维度（#90/#94/#96 生成，本任务不重建）、O6 方向轴（payoff 是仓位轴 payoff 侧，与节点轴对偶无冲突）、payoff 有效域（L3 未决未声明）。

---

## 六、认识论等级标注（formalization-validity-domain 强制）

| 命题 | 等级 | 信息增量 |
|------|------|----------|
| payoff = case 空间 Σ' 上的纤维丛（底=形态 case 向量，纤维=payoff×regime） | **L0**（§1.1，纤维丛结构定义 + payoff-regime §2.2 同形态符号翻转佐证纤维方向=regime） | 高：payoff 非维度=纤维，FAIL② 构造前提 |
| 结构孤儿 ⟹ 必失血（L_confirm≻L_move ∨ d_k=Up） | **L0**（§2.2，从 H¹ 闭合循环代数推导，孤儿不可能定理 L3 结算支撑） | 高：孤儿是结构性必失血 |
| **失血 ⊋ 孤儿（失血 ⇍ 孤儿，反向不成立）** | **L0 结构 + L3 反例**（§2.3，反例「可操作但强牛 regime net<0」由 payoff-regime §3.1 selloff L3 支撑） | 高：FAIL① 核心，分离两失血概念 |
| c 的真实自变量集 = 整个 case 向量 + regime（删 c=f(L_confirm)） | **L0 据实**（§1.2，自变量逐一核验）on **L3 证据**（payoff-regime §3.2 三因子 + §2.2 regime 翻转） | 高：FAIL② 删声明膨胀 |
| 分类层只给 L0 结构判据，不声明 payoff 符号 | **L0**（§3.2 有效域边界，formalization-validity-domain §禁止模式3） | 高：FAIL③ 守分层 |
| **payoff 符号 sign(net_c) 是 regime 函数** | **L3 未决**（§3.3，OKLO 正/GC 负 payoff-regime §2.2，本任务不声明符号） | 否定性：分类层不降级声明可操作⟹赚 |
| G_payoff 生成性完备无遗漏（结构孤儿⊔可操作 + 三符号纤维） | **L0**（§4.2/4.4，全序二分 + 纤维方向穷尽） | 高：生成所有 case 含两边界 |
| 边界 case①「可操作但 regime 失血」+ ②「对角线 net→0」 | **L0 生成 + L3 符号**（§4.3，G_payoff 机械吐出，符号 L3） | 高：可操作⇏payoff>0 由生成函数三符号吐出非断言 |
| 结构判据 L_confirm 在 a0=segment 尺度坍缩 | **L3 未决**（§3 边界条件，554号继承 #94 §六） | 否定性：结构判据 L0 在 a0 尺度有效域 L3 约束 |

**核心诚实分层**：本任务交付 **payoff 纤维丛的构造（L0 结构层完备 + L0/L3 严格分层）**——payoff 为什么是纤维丛非维度（§1.1）、c 的自变量为什么是整个 case 向量非 L_confirm 单变量（§1.2）、失血为什么 ⊋ 孤儿（§2.3）、分类层为什么只给 L0 结构判据（§3.2）。本任务**不**交付：哪个 case 实际 payoff 符号（L3 regime 函数，payoff-regime #75）、payoff 实装代码（子5 #84）、"自适应后超 BH"（L3 未决）。**分类层 L0 完备（结构孤儿/可操作二分穷尽）≠ payoff 层有效域（哪些 case 净赚 L3）**——守住此分层 = 守住 formalization-validity-domain（FAIL③ 修复的本质）。三 FAIL 共同根 = 子8 把 L3 的 regime payoff（纤维值）降级为 L0 的结构判据（孤儿⟺失血等价 / c=f(L_confirm) 单变量 / 可操作⟹超BH）——修复 = 把纤维值还给 L3，分类层只留 L0 底空间结构。

---

## 七、给审查节点（#16）与异质重测（#20 codex）的关键质询点

供异步自指审查与 codex 异质否定聚焦（自标可攻击面，no-patch 透明）：

1. **纤维丛 vs 维度的区分是否真实，还是换名？** §1.1 称 payoff 是 Σ' 上的纤维丛（底=case 向量，纤维=payoff×regime）。质询：regime 是不是就该是 Σ' 的第八个 case 维度（和 δ_k/completion_k 并列）？若 regime 进 Σ' 作维度，payoff 就由扩展后的 case 向量唯一决定（退回维度），纤维丛消失。判据：regime 是否可由形态层（走势树）内生读出（若可 = 维度），还是外生于单标的形态（若外生 = 纤维方向）？payoff-regime §2.2 用"标的有无足够下跌段"——这是单标的全历史的统计属性，不是单个 case 的形态读数，故外生（纤维方向）。但这是否意味着 regime 是 L3 标的级常数而非 per-case 纤维？若 regime 是标的级常数，纤维丛是否退化为"每个标的一个截面"（trivial bundle）？

2. **失血 ⊋ 孤儿的反例②（对角线 net→0）算不算"失血"？** §2.3 用边界 case① 证失血⇍孤儿。但 §4.3 边界 case②（对角线 net→0）是 no-op（开≈不开），net→0 不是 net<0。质询：net→0 算失血吗？若不算，失血 ∖ 孤儿 是否只有边界①（强牛 regime net<0）一个来源？若 net→0 因摩擦实际 net<0（开仓必付摩擦），它是否也进失血 ∖ 孤儿？这影响"失血 ⊋ 孤儿"的严格性——失血集合的边界是否清晰。

3. **c 的自变量集含 regime 是否使"分类层 L0"自相矛盾？** §1.2 c(case)=c(case 向量, regime)，§3.2 分类层只给 L0 不含 regime。质询：c 含 regime（L3 自变量）但分类层 L0 不读 regime——那分类层算的是什么？是 c 的 L0 投影（固定 regime 的截面）还是 c 的定义域结构（不算 c 值只算 case 归属）？若分类层只算 case 归属（结构孤儿/可操作）不算 c 值，那 c 的自变量集声明（§1.2）和分类层 L0（§3.2）是否在谈两件事（c 值 vs case 归属），FAIL② 和 FAIL③ 是否其实是一个分层问题的两面？

4. **结构孤儿 ⟹ 必失血是否在 a0=segment 尺度坍缩？** §2.2 称孤儿 ⟹ 必失血是 L0。但 §六标记 554号 L3 否证 D_TOP@a0=segment 区间套链坍缩。质询：若 L_confirm 在 a0=segment 读不出（链断），结构孤儿判据（L_confirm≻L_move）在该尺度无法判定 ⟹ 孤儿 ⟹ 失血的 L0 充分性是否在 a0 尺度失效（既判不出孤儿也判不出失血）？这是否使"结构孤儿失血 L0"的有效域严格小于定义域（仅 a0 以上级别有效）？

5. **payoff 纤维丛 vs 子8 原构造的差异是真修复还是重新表述？** 本任务承接子8 原文件丢失，从 payoff-regime + intrinsic_quota 重建。质询：子8 原构造是否真的声明了"孤儿⟺失血等价 / c=f(L_confirm) / 可操作⟹超BH"（codex #95 判的），还是 codex #95 的摘要本身可能误读了子8（子8 丢失无法核对）？本任务修复的是 codex #95 摘要描述的三 FAIL，若子8 原文实际未犯这三错（摘要失真），本任务是否在修一个不存在的问题？——但 no-patch 角度：即使子8 原文未犯，本任务的 L0/L3 分层 + 纤维丛构造本身是 payoff 纤维的严格形式（不依赖子8 是否犯错），故构造有效。
