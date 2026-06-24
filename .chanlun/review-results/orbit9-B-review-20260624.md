# 代码审查报告：9 轨道操作分派 + O3 add 原语（task#40 / orbit9-B-dispatch）

审查工位：orbit9-B-review（异工位，约束3）
被审工位：orbit9-B-dispatch（commit `faaa0d6d93`）
审查文件：`rust/src/recursive_t/rec_engine.rs`（+202 行新增）
依据：`.chanlun/review-results/orbit9-B-dispatch-20260624.md`（B 自述报告）
审查基准：`#39 exhaustive-operation-classification`（9 τ对称轨道终版）
日期：2026-06-24

---

## 概念层质询结果

**通过——无矛盾**。

六要素验核：

1. **结论**：`EngineConfig::enable_orbit9_dispatch`（env `T_ORBIT9_DISPATCH`）门控 route_bsp 显式 9 轨道分派。OFF 时（缺省）route_bsp 走旧二元分支 bit-exact。ON 时补 O3 add 缺口（同父向持短差 + 回调未完成 → 部分买回，委托 `orbit9_sub_trend_done` 占位 fallback=true 保未整合 bit-exact）。

2. **定义依据**：
   - `#39 §3.2 轨道表`：9 τ对称轨道 O1-O9 完全对称（多空9=9），O3=add/add_short，来源态 S2（type1后回调中枢上），op=开，极性不变加仓
   - `#39 §3.1b`：add = 不动 h 同级别短差腿重建（多头买回 ↔ 空头卖回 τ 镜像，莫比乌斯对 O3 平凡对称）
   - `#39 §3.4`：O3 精化「不动 h」（与 543 `h⁺∘σ` path 层表述的张力已由 #39 终版裁决）
   - 三禁令（#35 §3.6/§5.4）：禁扁平门、禁单触发器、禁 H¹ 当 H⁰ 方向滤波器——route 内零账本分支

3. **边界条件（结论翻转条件）**：
   - 若 O3 add 配额改用全量（`u_sub`）而非 `quota(u_sub)/3`，则 add = recover（整条），O3 与 O7 语义合并，9 轨道退化为 8 轨道，违反 #39 §3.1b「部分买回」定义
   - 若 `orbit9_sub_trend_done` 占位从 `return true` 改为 `return false`，则 ON 状态下同父向持短差全走 add（非 recover），OFF bit-exact 失效——但此处 fallback 明确标注为占位，不影响当前行为
   - 若 `T_ORBIT9_DISPATCH` 设置时同时设置了 `T_OFF_BASELINE`，production() 走 off() 覆盖，orbit9 实际不激活——两 env 互斥语义未有明文约束（见 MEDIUM-1）

4. **下游推论**：
   - D 工位（bit-exact OFF 守卫 + L3 验证）需在 `T_ORBIT9_DISPATCH` ON/OFF 状态下各跑八标的，验证 OFF=facea bit-exact
   - C 工位（区间套 H¹ 定位）交付 `is_sub_trend_done` 实现后，替换 `orbit9_sub_trend_done` 占位即可激活 O3 真路径（无其他接口变更）
   - `add_pnl_by_level` 观测计数可作 O3 买回腿收益归因基础（供 L3 验收哪级别买回赚/亏）

5. **谱系引用**：
   - **#39**（操作语义穷尽分类——本实装直接依据）
   - **543**（操作=word，OP_REBUY；#39 §3.4 精化张力裁决）
   - **541**（D∞/莫比乌斯/P2：τ 镜像对称的群论根）
   - #35 §3.6/§5.4（三禁令；账本⊥操作语义分离——C7/NL7 正解）
   - 不确定是否有已结算谱系关于「recover vs add 判别时序」——审查期间未找到相关结算

6. **影响声明**：
   - `EngineConfig` 新增 `enable_orbit9_dispatch: bool` 字段（`from_env`/`off`/各 preset 覆盖）
   - `TRoot` struct 新增 `enable_orbit9_dispatch: bool`、`n_adds: u64`、`add_pnl_by_level: [f64; MAX_LEVEL]`
   - 新增 `fn orbit9_sub_trend_done`（接口占位）、`fn add`（O3 原语，36 行）
   - `route_bsp` 同父向分支新增 orbit9 条件分支（+6 行）
   - `new_with_config` 初始化新字段（+3 行）
   - 八轨道原语（enter/ascend/flip/sink/recover/drain/no-op）**行为 bit-exact 不变**（仅轨道标注注释，无代码改动）

---

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 1 | info |
| LOW | 0 | pass |

**结论：PASS**

---

## 逐项检查结果

### 分类符合性

**PASS**。

9 轨道映射逐项核验（依据 #39 §3.2）：

| 轨道 | #39 定义 | B 代码 | 验证 |
|------|---------|--------|------|
| O1 flip | τ@φ=0 | route_bsp None 反向 clear_all+enter | 复用，无行为改变 |
| O2 enter | e→建仓 | route_bsp None 首建 enter | 复用，无行为改变 |
| **O3 add** | **OP_REBUY，不动 h，部分买回** | **新增 fn add：quota(u_sub)，pdir 参数化 τ 对称** | **PASS，结构完整** |
| O4 ascend | σ relabel riding | route_bsp None 同向 j>cc ascend | 复用 |
| O5 emergence_upgrade | σ relabel level | step A' emergence_upgrade | 复用 |
| O6 sink | h⁻∘τ_sub | route_bsp Some(p) 反父向 sink | 复用 |
| O7 recover | σ⁻¹∘τ | route_bsp Some(p) 同父向持短差 recover | 复用 |
| O8 drain | τ@k | route_bsp Some(p) 反父向遗留同向 drain | 复用 |
| O9 hold/no-op | e | route_bsp 各 no-op 分支 | 复用 |

`add` 方法第 1407-1442 行：使用 `pdir = instances[parent].direction`（父向极性），`mob = flip_pol(pdir)`（反父向 = 短差方向），无任何 `if Polarity::Long / if Polarity::Short` 硬编码分支。τ 镜像通过 `pdir` 参数化实现——多头买回（pdir=Long, mob=Short）↔ 空头卖回（pdir=Short, mob=Long）自然对称。单测 `add_short_τ镜像_父空卖回对称` 验证（通过）。

### 三禁令（#35 派生）

**PASS，三禁令均未触犯**。

1. **禁扁平门**：`enable_orbit9_dispatch` 门在 `route_bsp` 同父向分支内部（第 1478 行），不在 `on_bar` 顶层。
2. **禁单触发器**：route_bsp 9 轨道分派依据 `nearest_active_parent(j)` + `highest_active()` + `is_buy`——这些是 Σ_state 结构涌现投影，非单一全局触发器。
3. **禁把 H¹ 当 H⁰ 方向滤波器**：route 内无任何 `if regime / if account / if level == N` 条件分支。O3 路径进入条件仅为「enable_orbit9_dispatch && !orbit9_sub_trend_done(j)」，后者是区间套 H¹ 定位结果（C 接口），不是方向过滤。

route 内账本分支核查（第 1445-1515 行全文）：无任何 `pair_long_pnl`/`pair_short_pnl`/`stage`/`regime` 读取。`account_reduce` 在 add 方法内部被调用，这是**原子操作自带的会计**（与 sink/recover/drain 同质），不是 route 内的账本门控分支——符合 #35 §5.4「账本过滤是 route 之后独立 gate」规约。

### OFF bit-exact

**PASS（L1 管线级别）**。

逻辑分析：
- `off()` 显式将 `enable_orbit9_dispatch = false`（第 229 行）
- `from_env()` 缺省：`std::env::var("T_ORBIT9_DISPATCH").is_ok()` = false（不设 env 则 is_err → false）
- route_bsp 同父向分支：`if self.enable_orbit9_dispatch && !self.orbit9_sub_trend_done(j)` — enable=false 时短路，逐字走旧 recover 分支
- 单测 `off_bit_exact_orbit9未激活_全走recover`（第 2700 行）验证：OFF 时同父向持短差全走 recover，n_adds=0，n_recovers=1（通过）

注：OFF bit-exact = 与 `facea 54279a503e` 逐位一致的 L1 验证，委托 D 任务在真实数据上执行 L2 级别的八标的对比验证。

### τ 对称性

**PASS**。

`fn add` 第 1407-1442 行无任何 `match dir` 或 `if pdir == Long` 分支——pdir 参数化贯通到 `rec_add`/`rec_reduce`，实现手性 τ² = e（买回↔卖回镜像）。`rec_add` 内部的 `assert_eq!(inst.direction, dir)` 守卫确保层内单一方向不被污染（第 465 行）。

### coding-style

**PASS，无违规**。

- **不可变性**：`fn add` 采用局部变量模式（`let mut free = self.free; ... self.free = free`），与 `sink`/`recover`/`drain` 逐字一致，不原地修改。
- **函数长度**：`fn add` 36 行（第 1407-1442 行），`orbit9_sub_trend_done` 2 行，均在 50 行限制内。
- **错误处理**：所有数值门控（`m > 1e-12`/`give > 1e-12`/`give.is_finite()`）均有 `return` 提前退出，无静默吞错。
- **无硬编码魔法数**：配额使用 `quota(u_sub)` = `u_sub * MOBILE_FRAC`，常量已有命名（第 39 行 `MOBILE_FRAC = 1.0/3.0`）。
- **文件规模**：2732 行，超过 800 行上限。**注**：此为历史累积，非 B 工位本次引入（B 本次增量 202 行，文件已长期超限）。不计为 B 工位本次新引入问题。

### 接口面（供 D-整合）

B 工位暴露的接口：
- **字段**：`EngineConfig::enable_orbit9_dispatch`（pub）
- **env 变量**：`T_ORBIT9_DISPATCH`
- **fn add**：private，经 route_bsp 间接调用
- **fn orbit9_sub_trend_done**：private，接口占位（C 整合替换）
- **观测**：`n_adds: u64`（pub）、`add_pnl_by_level: [f64; MAX_LEVEL]`（pub）

**D-整合关键接口冲突点**：

A 工位（commit `0190470722`，独立 worktree）为 `route_bsp` 增加了 `flip_confirmed: bool` 参数（A-review 报告第 33 行：「`route_bsp` 签名改变（新增参数）」）。B 工位基于 `facea 54279a503e`，其 `route_bsp` 签名为 `fn route_bsp(&mut self, j: usize, is_buy: bool, node: TrendNode, c: f64)`（第 1445 行），**无 `flip_confirmed` 参数**。

D-整合时需合并 A 工位的 `flip_confirmed` 参数到 B 工位所在的 `route_bsp` 签名——二者修改同一函数（A 改签名、B 改函数体内同父向分支），存在 git merge conflict 可能。整合策略：以 A 的签名为基础（加 `flip_confirmed` 参数），在其函数体内保留 B 的 orbit9 分支（无逻辑冲突，双方修改不同代码段）。

---

## MEDIUM-1：`T_ORBIT9_DISPATCH` 与 `T_OFF_BASELINE` 互斥语义未声明

**位置**：`rec_engine.rs:282-290`（`production()`），`rec_engine.rs:212`（`from_env` 读 `T_ORBIT9_DISPATCH`）。

**问题**：`production()` 中，若 `T_OFF_BASELINE` 设置则走 `off()`（强制 `enable_orbit9_dispatch=false`）。若同时设置 `T_ORBIT9_DISPATCH` 和 `T_OFF_BASELINE`，二者优先关系在代码注释层有隐含（`T_OFF_BASELINE` 显式回归守卫说明覆盖所有变体），但对 D 工位实验脚本作者来说存在非直觉行为：设了 `T_ORBIT9_DISPATCH` 但 orbit9 不激活。

**严重度**：MEDIUM（不影响 OFF bit-exact 正确性，但可能造成 D 任务受控实验配置混淆）。

**建议**：在 `from_env` 的 `T_ORBIT9_DISPATCH` 注释中补一行：「与 `T_OFF_BASELINE` 同时设置时，`production()` 走 off() 覆盖此字段——受控实验勿混用」。

---

## 测试验证

5 个 orbit9 单测全部通过（cargo test orbit9_add_dispatch）：

```
test orbit9_add_dispatch_tests::add_无短差腿_noop_不pyramid ... ok
test orbit9_add_dispatch_tests::add_short_τ镜像_父空卖回对称 ... ok
test orbit9_add_dispatch_tests::add_部分买回_不污染父向极性 ... ok
test orbit9_add_dispatch_tests::off_bit_exact_orbit9未激活_全走recover ... ok
test orbit9_add_dispatch_tests::on_orbit9_回调未完成_走add部分买回 ... ok
```

全量 lib 560 测试通过（0 failed）。

*认识论等级：本报告结论 = L0/L1（代码结构审查 + 单测管线）。OFF bit-exact（vs facea 54279a503e 真实数据）= L2，委托 D 工位验证。*
