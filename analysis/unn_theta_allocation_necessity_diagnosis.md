# unn 三问诊断 + θ 仓位分配必然性推导

> 任务（2026-06-15 用户三问）：
> 1. BTC 333 笔子空头 spawn——多少有对应 D 回补？无回补 = D 规则没消费买点 = 代码 bug。
> 2. 子空头内部有无孙 voice 降成本？无 = E 规则对子空头跳过（T8 叶节点限制误扩展到非根空头）。
> 3. θ（中枢振幅比例分配仓位）不是必然推论。从螺旋覆盖空间推导仓位分配的严格形式——
>    m 应由什么决定？是径向坐标 r（级别）的函数吗？
>
> 数据源：`analysis/data_cache/unn_pertrade_BTC.json`（4.6M bar，2017–2026，strat +340.9%，MDD −70.5%）。
> 引擎：`rust/src/trading/unified_necessity.rs`（mode="unn"）。验证：21/21 unn 测试通过（含 should_panic 反证）。
> 认识论：Q1/Q2 = L2（真实数据）；Q3 必然性 = L0（群论/覆盖论，条件 A₅）+ L2 经验签名。

---

## 摘要（三问结论）

| 问 | 结论 | 性质 |
|----|------|------|
| **Q1** 子空头回补 | **无 bug**。291 个子空头**100% 经 D 回补**（reason=recover），0 强平，0 悬挂。净 −54322（178 亏/113 盈）= 正常降成本代价，但强牛 regime 下净负（踏空确证） | 实现正确 |
| **Q2** 孙 voice | **无 bug**。**42 个孙 voice（做多）**由子空头 spawn（40@ladder2，2@ladder3）。E 规则**不跳过非根空头**——T8 限制 `is_root && Short` 仅作用于**根空头** | 实现正确 |
| **Q3** θ 分配必然性 | **θ 规则非必然，且与 T59（自相似/尺度不变）矛盾**。必然形式 = spawn 比例 f=m/p_units 是 **σ-不变常数**（级别无关），= 1/λ 若取"势∝r"。θ 全局归一化破坏 σ-不变性 | **定义冲突 → 上浮** |

---

## Q1：333 子空头 spawn 的 D 回补配对（无 bug）

### 数据（`unn_pertrade_BTC.json`，334 笔 trade）

```
spawns 总数         = 333   by_ladder = {2:257, 3:70, 4:6}
root_entries        = 1     （ladder3 入场 → 涌现爬升至 ladder6）
flips（根翻转）     = 0     （根从未翻空，符 T50 清仓罕见）
liquidations（A）   = 0     （强平规则从未触发）

按 pol × reason：
  long  / eod      n=1     pnl=+350255.1   ← 根（持有到底）
  long  / recover  n=26    pnl=   -277.7   ← 孙 voice（见 Q2）
  long  / cascade  n=16    pnl=    +28.0   ← 孙 voice（父关级联）
  short / recover  n=291   pnl= -54322.2   ← 子空头
```

### 配对核对

- 333 spawn = 291 短(recover) + 26 长(recover) + 16 长(cascade)。✓ 全部闭合。
- **291 个子空头 100% 经 D 回补**（reason=recover，在子层买点 `sig.buy_any.get(ladder)` 平仓返父，
  `unified_necessity.rs:1336-1354` D 规则）。
- **0 悬挂**（无 eod 未平的子空头）、**0 强平**（A 规则 `liquidations=0`，c≥2×basis 从未触及）。

### 判定

**D 规则正确消费了每一个买点**——不存在"spawn 了空头却没有买点平仓"的情形。
子空头净亏 −54322（178 亏 / 113 盈，中位 −64.0）= **正常降成本代价**（用户判据"有回补但亏钱→正常"）。

净负本身是**强牛 regime 的踏空**（`project_unn_btc_spawn_throwback` 已结算 L3）：上涨主趋势中，
为"降成本"开的空头在低级别买点平仓时多数已被价格上穿 ⇒ 回补价 > 开空价 ⇒ 亏。回补**及时**
（持有中位 134 bar），亏不是因为回补失败，是因为**操作域错配**（空头操作域 ⊂ 非上行 regime）。

> **边界翻转**：若 regime 转震荡/下行（如 ES 2011、CL 油崩），同一 D 回补机制净正
> （`project_unn_root_flip_mtm` L3：CL/BRN 震荡正域）。回补机制无 bug；净盈亏是 regime 函数。

---

## Q2：孙 voice 降成本（无 bug）

### E 规则逻辑核对（`unified_necessity.rs:1357-1394`）

```rust
for &id in &snap {                          // 遍历全部 voice
    if !self.voices[id].can_act(bar) { continue; }
    let is_root = self.voices[id].parent.is_none();
    // T8 根空头叶节点限制——仅作用于 is_root && Short：
    if is_root && dir == Polarity::Short { continue; }   // ← 关键：is_root 门控
    let (nf_trigger, confirmed_root) = match dir {
        Polarity::Long  => (nf_sell[ladder].is_some(), is_root && sig.sell_any.get(ladder)),
        Polarity::Short => (nf_buy[ladder].is_some(), false),   // 子空头：买点触发 spawn 孙做多
    };
    if nf_trigger || confirmed_root {
        prove_n7_spawn_self_level(ladder, ladder, bar);
        try_spawn_cost_gated(id, ...);       // child_dir = 父反向 = Long（孙做多）
    }
}
```

**T8 限制 `if is_root && dir == Polarity::Short` 由 `is_root &&` 门控**——**只跳过根空头**，
**不跳过子空头**。子空头（非根、dir=Short）触发判据是 `nf_buy[ladder].is_some()`（自层买点），
触发后 `try_spawn_cost_gated` 造 `child_dir = Long`（父空 → 子多）= **孙 voice 做多**。

这正是用户描述的预期行为："子空头持空期间低级别买点应触发孙 voice 做多"——**代码已正确实装**。

### 经验确认（BTC 真实数据）

```
孙 voice（long spawn，非根）= 42 个   by_ladder = {2:40, 3:2}
  来源链：root@4 → 子空头@3 → 孙做多@2（40 个）
          root@5 → 子空头@4 → 孙做多@3（2 个）
  pnl 合计 = -249.7（recover 26 + cascade 16）
```

递归深度实证：**root → 子空头 → 孙 voice = 深度 2 已涌现**（T23 自相似嵌套）。

### 为何子空头@2（217 个）不再 spawn 孙 voice

`FIRST_BSP_LADDER=2`（segment）、`PENDING_LO=3`（move L1）。子空头@2（segment 层）：
- nf_buy[2] 恒 None（segment 非势源，无 pending/nf）⇒ E 触发判据永不满足 ⇒ 不尝试 spawn。
- 即使尝试，sub=1 < FIRST_BSP ⇒ theta(1)=None（noref_reject，N4 自然终止）。

**这是 N4 成本门 + 势源下界的正确终止**，非 bug。孙 voice 只在子空头@3/@4（move 层以上）涌现，符合
T19/T59 递归基 = segment。

### 判定

**E 规则对子空头正确递归（无跳过）**。T8 叶节点限制**未误扩展**到非根空头。孙 voice 存在且
按 N4 成本门正确终止。孙 voice 净亏 −249.7 同样是强牛 regime 函数（量级远小于子空头层）。

---

## Q3：θ 仓位分配的必然性推导（θ 非必然，与 T59 矛盾）

### 3.1 现状：θ 全局归一化（`unified_necessity.rs:894-899`）

```rust
let (thetas, theta_total) = theta_weights(depth_ref, FIRST_BSP_LADDER); // 跨 [2,11) 全塔
let w = thetas[sub].map(|t| t / theta_total);   // w = θ_sub / Σ_{k=2}^{10} θ_k
let m_quota = w.map_or(0.0, |w| p_units * w);    // m = p_units × θ_sub/θ_total
```

θ(k) = 该层买卖点振幅的 q=0.5 分位数（`depth_ref.theta`，035:30"平均波幅"），**纯经验量（L2）**。
归一化分母 θ_total = **整塔** [FIRST_BSP, MAX) 振幅之和（**全局归一化**）。

T18（`necessity_derivation.md:443-454`）声称 `m = p_units × θ_sub/θ_total` 由"操作量 ∝ 势"推出，
但【环 15】自注："缠师原文：第 53 课配额（**留白回测裁决**）"——即**分配比例缠师未给定，θ 是回测选择**，
非缠论原文必然。

### 3.2 螺旋覆盖空间的必然约束

螺旋三坐标（`spiral_exhaustive_enumeration.md` / `necessity_derivation.md` §8.4.1）：

| 坐标 | 记号 | 算子 | 群论身份 |
|------|------|------|---------|
| 径向（尺度） | r = λᵏ | σ = h²³（升一级别） | ⟨σ⟩≅ℤ 径向塔 |

两条与分配直接相关的已结算不变量：

- **T48（Casimir）**：守恒量 = **股数 N（units）**，σ-**不变**，纯计数、量纲无关（`prove_n8` 守 Σunits=N_base）。
- **T59（尺度不变 / R₃ 自相似）**：σ W_form σ⁻¹ = W_form。"同一 step 逻辑复制每层"
  （`unified_necessity.rs` docstring：递归 = 覆盖映射 σ）。

### 3.3 必然命题：spawn 比例 f = m/p_units 是 σ-不变常数（级别无关）

**spawn 作为螺旋操作**：父 voice 占据第 k 圈（半径 r_k=λᵏ），释放 m units 给占据内圈 k−1
（半径 r_{k−1}=r_k/λ）的子 voice。定义 spawn 比例 f := m / p_units。

**证明（σ-等变）**：spawn 算子 W：(圈 k, units U) ↦ (子圈 k−1, units f_k·U)。
由 T59，W 与 σ 对易：σ⁻¹ W σ = W。对圈 k 的状态作用：
- σ：圈 k ↦ 圈 k+1（units σ-不变，T48 ⟹ U 不变）；
- W：↦ (圈 k, units f_{k+1}·U)；
- σ⁻¹：圈 k ↦ 圈 k−1（units 不变）⟹ (圈 k−1, units f_{k+1}·U)。
- 此须等于 W(圈 k) = (圈 k−1, f_k·U) ⟹ **f_k = f_{k+1}**。

故 **f 跨级别恒定（σ-不变）**。∎

**这是 units（σ-不变 Casimir，T48）与自相似（T59）的联合推论**：一个 σ-不变量（units 比例 f）
必须由 σ-不变的方式确定 ⟹ f 是级别无关常数。

### 3.4 常数的值：自由度 vs 几何固定

- **自相似单独**：f=const，值**自由**——这是仓位分配的**唯一自由度**（`project_leverage_triad_formalization`
  "唯一自由度=顶层配额一维"；第 53 课留白）。
- **加"势∝r"（T18 前提，螺旋细胞 {r}）**：若进一步要求"各圈操作的 units ∝ 该圈半径 r=λᵏ"，
  则几何 spawn 链（root@S 留 (1−f)N，子@S−1 留 f(1−f)N，…，voice@(S−j) 留 fʲ(1−f)N）实现
  units(级别 S−j) ∝ λ^{S−j} ⟺ fʲ ∝ λ⁻ʲ ⟺ **f = 1/λ**。
  即 **m = p_units / λ = p_units × (r_{k−1}/r_k)**。

**回答用户："m 是径向坐标 r 的函数吗？"**
→ **m 不是绝对径向位置 r=λᵏ 的函数**（那会破坏 σ-不变性 / T59）。m 是**径向尺度比 λ = r_k/r_{k−1}
（σ 生成元的本征值）的函数**：m = p_units/λ。"r 的函数"只在"比值 λ"意义上成立，不在"绝对位置 λᵏ"意义上成立。

### 3.5 θ 全局规则的三处缺陷（与必然形式对照）

| 缺陷 | 必然形式 | θ 全局规则 |
|------|---------|-----------|
| **σ-不变性** | f = const（级别无关） | 即使 A₅-理想 θ_k=θ₀λᵏ，f = λ^sub/Σ_{2..10}λᵏ **随 sub 变**（固定窗口不随 σ 平移）⇒ **破 T59** |
| **"势∝r"实现** | units(k) ∝ λᵏ | 递归全局-θ 给 units(S−j) ∝ Πθ/θ_total^j（**乘积级联**）≠ θ_{S−j} ⇒ **连自身目标都不达成** |
| **认识论等级** | m=units=σ-不变 Casimir（L0） | m 由 θ（经验振幅 L2）算 ⇒ **L2 注入 L0**，引入 regime 噪声 |

### 3.6 经验签名（BTC，θ 规则实际释放比例）

```
f = child_shares / nbase（nbase≈6.205）：
  median = 0.247   mean = 0.306   max = 0.958   p75 = 0.363
按层：
  子空头@2 (217): median 0.263  mean 0.357  max 0.958  ← 单笔释放根仓位 95.8% 给空头
  子空头@3 (68):  median 0.298  mean 0.303
  子空头@4 (6):   median 0.217  mean 0.279
```

**max 95.8%**：单笔 spawn 把根仓位的 95.8% 借给一个空头——这就是 `project_unn_btc_spawn_throwback`
记录的"借 units 稀释主升浪"机制。θ 全局规则释放比例**可变且 regime 放大**；σ-不变的 f=1/λ 是**固定温和比例**。

> **注（有效域）**：Q3 的必然性（f=σ-不变常数）是 **L0**（从 T59 推出），与"哪个常数回测最优"
> （L2/L3 有效域读数）是不同范畴。θ 规则的踏空贡献（L3）≠ 必然性论证（L0）。本报告只确立 L0 必然形式；
> 换引擎实测哪个常数 = 上浮后的决策（见 §3.7）。

### 3.7 为何上浮而非直接改（四分法）

| 子结论 | 四分法 | 处理 |
|--------|--------|------|
| "f 必须 σ-不变常数" | **定理**（T59 强制） | 自动结算（本报告确立） |
| "θ 全局规则破 σ-不变" | **定理**（λ^sub/Σλᵏ≠const 可证） | 自动结算（本报告确立） |
| "用 1/λ 几何 / 自由常数 / 保留 θ 作 L2 近似？换不换引擎？" | **选择 + 语法记录** | **`/escalate`** |

改 θ 规则 = 改 T18 已结算定理的形式（含义/有效域）⇒ 按 `no-workaround.md`/`testing-override.md`
"修复需改定义含义 = 定义冲突，停下来上浮"。**不擅自重写引擎分配语义**——
见 `.chanlun/escalations/2026-06-15-theta-allocation-non-necessity.md`。

---

## 结果包六要素

### 1. 结论
- Q1：**无 bug**，291 子空头 100% D 回补，净 −54322 = 正常降成本代价（强牛踏空，regime 函数）。
- Q2：**无 bug**，42 孙 voice 做多由子空头 spawn，E 规则不跳过非根空头（T8 仅限根空头）。
- Q3：**θ 全局归一化分配非必然，与 T59 矛盾**。必然形式 = f=m/p_units 是 **σ-不变常数**（级别无关），
  值在"势∝r"下 = **1/λ**（即 m=p_units/λ）。m 是**尺度比 λ** 的函数，**不是绝对径向位置 λᵏ** 的函数。

### 2. 定义依据
- Q1：D 规则 `unified_necessity.rs:1336-1354`（子层 `buy_any` 平仓返父）；A 规则 1191-1211（c≥2×basis）。
- Q2：E 规则 1357-1394（`is_root && Short` 门控 T8）；T8 叶节点 docstring 106-108；`try_spawn_cost_gated` 870-938（child_dir=父反向）。
- Q3：T48（Casimir=股数 σ-不变）`spiral §1 细胞{r}`；T59（尺度不变 R₃）；T18（势∝r，留白回测）`necessity_derivation.md:443-454`；T56（h²³=σ，绕一圈=升一级）。

### 3. 边界条件（结论翻转）
- Q1 翻转：若 regime 转震荡/下行 ⇒ 同一回补机制净正（CL/BRN L3 正域）。回补机制本身无条件正确。
- Q2 翻转：若 `is_root &&` 门控被误删为 `dir==Short` ⇒ 子空头被跳过 ⇒ 孙 voice 消失（当前代码无此 bug）。
- Q3 翻转：若 A₅ 失效（λ 非恒定）⇒ 1/λ 弱化为近似，但"f 必须级别无关常数"（定性 σ-不变）仍由 R₃ 保留；
  若否认"势∝r"前提 ⇒ f 退化为自由常数（仍 σ-不变，仍否定 θ 全局规则）。**两种翻转都不救 θ 全局归一化**。

### 4. 下游推论
- θ 全局规则若改为 σ-不变常数（1/λ 或自由）：影响全 8 标的 unn 回测有效域读数（L3 须重跑）。
- 踏空机制（`project_unn_btc_spawn_throwback`）获分配层根因之一：θ 可变大比例 ⊥ σ-不变温和比例。
- T18 文本需修订（势∝r 与全局-θ-归一化自相矛盾，§3.5 缺陷 2）；spiral 细胞 {r} T18 标 ✅ 应降级为争议。
- `theta_weights` 在 `positional_fusion.rs` 等其他引擎的消费需同步评估（本报告范围仅 unn）。

### 5. 谱系引用
- `project_unn_btc_spawn_throwback`（踏空根因：E spawn 强牛过度，借 units 稀释）——本报告补分配层机制。
- `project_leverage_triad_formalization`（唯一自由度=顶层配额一维）——印证 f 值是单一自由度。
- `project_notional_leverage_research`（53课留白回测裁决域）——印证 θ 非缠论必然。
- `project_unn_root_flip_mtm` / 539 号（根翻空有效域 ⊂ 非上行）——同 regime 函数家族。
- `spiral_exhaustive_enumeration.md` T48/T59/T56；`necessity_derivation.md` T18/T19/T20。
- **不确定性声明**：未在 `.chanlun/genealogy/settled/` 检索到"仓位分配 σ-不变性"的已结算分离记录。
  强候选语法记录：「spawn 比例必须 σ-不变（级别无关），θ 全局归一化是隐性违反」——建议 genealogist 评估开节点。

### 6. 影响声明
- **新增** 本报告 `analysis/unn_theta_allocation_necessity_diagnosis.md`（诊断+推导，L0/L2）。
- **新增** 上浮 `.chanlun/escalations/2026-06-15-theta-allocation-non-necessity.md`（Q3 定义冲突）。
- **修改** `docs/necessity_derivation.md` T18：加 ⚠️ 必然性争议注 + 修正本块过时行号（行动类）。
- **不改** `unified_necessity.rs`（无 Q1/Q2 bug；Q3 是定义冲突，待编排者裁决，不擅自重写 θ 规则）。
- **未跑** 新回测（Q3 必然性是 L0，换常数实测属上浮后决策）。
- **附带发现（已 flag）**：`necessity_derivation.md` 多处行号漂移（:187/:334-340/:293/:262/:684/:176/:344/:713 等
  全部过时），超本任务范围，单独清理。
