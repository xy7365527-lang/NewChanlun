# Codex 审计：全互斥定义策略 b1 P0 接入设计 + H 轴进 z 裁定

- 工位: ws-b1audit（task #1，goal g-20260702T2200Z-full-mutex-impl s1 前置）
- 审计对象：`.chanlun/review-results/full-mutex-b1-design-20260702.md`（b1 设计稿）
- Codex 调用：`newchan.codex review`，完整交互原始持久化于 `.chanlun/review-results/codex-review-20260702-220553-0379.md`
- 本文件 = Codex 输出 + Claude（ws-b1audit）逐条代码事实核实（简化质询）+ 判定

## Codex 结论（原始输出）

**P0 接入方向成立，但不能按设计稿当前形式直接实装**。最小正确改法不是在 `econ_positive.rs` 里重写 `parent_dir/position`，而是在 `collect_signals` 现场直接复用 `z_of_candidate(c)`，把 `MuClass` 随信号带到 `SignalDecomp`，后续桶键用 `d.z` 或 `d.z.i_class` 投影。

**H 轴裁定：不补 H 进 `MuClass` 的 P0**。源 B（`推导完全分类.pdf`，z 定义直接出处）是 alpha z 定义的直接出处，r 是操作角色枚举，不含 H；源 A（`递归完全分类买卖点.pdf`）的 R(g)=(H,V,δ) 更像递归分类/唯一性层的结构角色。当前无证据证明 H 对 μ 有 OOS 增量，补 H 只会把样本再切 3 倍。

## Codex 四条主要否定（原文）

1. **`parent_dir + position` 不足以构造 canonical `MuClass`**（econ_positive.rs:222/324, mu_estimator.rs:92）——重要。修复：信号元组直接携带 `MuClass`（`let z = z_of_candidate(c)`），不要等 downstream 用 `bsp_class: u8` 反推。
2. **`MuClass` 只有 `Eq+Hash`，无 `Ord`，不能作 `BTreeMap` key**（mu_estimator.rs:46/68, econ_positive.rs:2068）——重要。设计稿"直接以 MuClass 为桶键"照写会编译失败。
3. **`MuClass::bsp_class()` 把 `i_class` 压成 1/2/3，与 `econ_positive.rs` 当前 6-bit 掩码 `bsp_class` 不同口径**（mu_estimator.rs:79, wverify_run.rs:83, econ_positive.rs:240）——重要。dx/Y 投影必须用 `z.i_class`（未压缩），不是 `z.bsp_class()`（压缩主类号）。
4. **影响边界不止两处**——`signals_dx` 镜像循环 + `SignalDecomp` 构造测试 + winner helper 都要同步（econ_positive.rs:3337/1437/2812/2841）——建议级。

## Claude（ws-b1audit）逐条代码事实核实

四条否定全部核实为**真实代码事实，成立**：

1. **成立，改进级**：设计稿 §2.2 确实只说"加 `parent_dir:i8` + `position`（或直接携带 `OperationRole`）"，是拆解重建路径。`selector.rs:144 z_of_candidate(c)` 已是完整桥接函数（读 `c.level/c.dir/c.bits/c.role.v`，产出完整 6 维 `MuClass`），信号元组直接调用它比手工拆解字段更少代码、零遗漏风险——Codex 建议更优，采纳。

2. **成立，阻断性**：核实 `mu_estimator.rs:68` `MuClass` 的 derive 列表为 `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]`，确无 `PartialOrd/Ord`。`econ_positive.rs` 现有三处桶键容器均为 `BTreeMap<(u32,i8,u8), Bucket>`（2070/2729/2823 一线）——若不加 `Ord` 派生直接把 key 换成 `MuClass` 会编译失败。此否定必须在 b2 实装中处理（方案二选一：给 `PositionState`+`MuClass` 加 `PartialOrd,Ord`，或改用 `HashMap<MuClass,Bucket>` + 输出时另排序）。

3. **成立，但需精确表述**：核实 `types.rs:229 class_index()` 与 `econ_positive.rs:192 bsp_disc()` 的位编码**逐位相同**（`buy1|buy2<<1|buy3<<2|sell1<<3|sell2<<4|sell3<<5`）——即 `MuClass.i_class` 字段（未压缩 6-bit，`from_certificate` 直接赋值 `bits.class_index()`）与 `econ_positive.rs` 现有 `bsp_class: u8`（来自 `bsp_disc`）**同一编码，直接兼容**。真正的陷阱不是"MuClass 提供不了 6-bit 掩码"，而是 `MuClass` 另外提供了一个**容易被误用**的压缩方法 `bsp_class()`（把 6-bit 压成 1/2/3 主类号，丢失 2B/3B 重合信息，P4 codex 判决明确要求不能压缩）——b2 实装必须在护航清单中显式禁止调用 `z.bsp_class()`，一律用 `z.i_class`。

4. **成立，已用 `grep`/行读核实**：`econ_positive.rs:3337` 是 `signals_dx.push((i, c.dir, p.source_index, lvl as u32, sigma_higher, bsp_class))`——注释明标"C1：与生产 collect_signals 同序同字段"，是独立的人工维持 bit-exact 同步的诊断镜像循环，必须与 `collect_signals` 同步升级到 `MuClass`。`econ_positive.rs:1437/2812/2841` 是硬编码 `SignalDecomp { ... bsp_class: 1<<3, ... }` 字面量测试构造，字段扩展后需同步。设计稿 §2.2"只改两处"的范围声明确认为低估，b2 规格必须扩大护航清单覆盖面。

## H 轴裁定核实

Codex 裁定基于证据包中"发现的定义歧义"一节做出：源 A（`递归完全分类买卖点.pdf` §7-8，唯一性定理必要条件）的 R(g)=(H,V,δ) 是**递归系统唯一性证明层**的角色分类；源 B（`推导完全分类.pdf`，z 状态向量与"细分类优势定理"的直接出处）的 r∈{开根声部,平仓,短差,顺势,反手} 是**收益估计 z 状态**的操作角色枚举，5 类不含 H。Codex 判定二者不是同一粒度字段，P0 只需补 B 已要求且代码已承载的 V/σ_p。

这一裁定逻辑自洽：`MuClass`（`mu_estimator.rs:54` 注释"六维全互斥"）本就是按源 B 的 z 实现的，代码从未声称是源 A 的 R(g) 载体。裁定**采纳**，且附带明确边界条件（见下），满足"选择类全权裁定"的要求。

## 结果包（六要素）

1. **结论**：P0 接入方向（把 role/σ_p 接入 alpha 回测状态）成立，但设计稿当前形式的具体实装方案需修订——最小正确改法是信号元组携带完整 `z_of_candidate(c)` 产出的 `MuClass`（而非拆解重建两个字段），且需先给 `MuClass` 补 `Ord`（或改用 `HashMap`）、护航清单需覆盖 `signals_dx` 镜像循环与多处硬编码测试。H 轴裁定：P0 不补，`MuClass` 维持现有 6 维。
2. **定义依据**：源 A `递归完全分类买卖点.pdf` §7.1/§7.2/§8（H(g)/V(g)/R(g)=(H,V,δ) 18 类，唯一性定理必要条件）；源 B `推导完全分类.pdf`（Z_t=(ℓ,δ,I_γ,r,σ_p,ω,β,d,c,m) 细分类优势定理，r∈{开根声部,平仓,短差,顺势,反手} 5 类不含 H）；代码 `mu_estimator.rs:54-76`（MuClass 六维定义，按源 B 实装）、`selector.rs:144-156`（z_of_candidate 完整桥接）、`types.rs:229`（class_index 位编码）、`econ_positive.rs:192`（bsp_disc 同编码）。
3. **边界条件（H 轴裁定翻转）**：若发现更晚权威文档明确写 `z.r = R(g)=(H,V,δ)`（即源 A/B 实为同一概念不同阶段表述，B 是 A 的简化误记），或 H 轴对 μ 在 OOS/LCB/置换去污后有稳定增量价值的 L2/L3 证据，则裁定翻转为补 H（`MuClass` 加 `horizontal: Horizontal` 字段，桶键升 3×54 类）。b2 实装若在这之前进行，只需按当前 6 维接入。
4. **下游推论**：(a) `MuClass` 补 `Ord` 后可安规直接作 `BTreeMap` key，桶键类型从 `(u32,i8,u8)` 三元组统一为 `MuClass`；(b) W-VERIFY 与 alpha 回测路径"统一到 Z"的说法目前**不成立**（Codex 指出 `wverify_run.rs:83` 桶键口径与 `MuClass.i_class` 尚未核实是否同一编码，b2 实装范围内不处理，留待独立核对）；(c) dx 守恒断言需扩展为"Σ over MuClass == Σ over (level,delta,i_class)"，且必须用未压缩 `i_class` 而非 `bsp_class()` 主类号，否则聚合守恒测试会静默丢失 2B/3B 重合信息。
5. **谱系引用**：230/231（有效域≠定义域，本次审计坐实为"z 定义存在多源版本，代码只忠实其中一源"的具体实例）；本设计稿 §结果包 5 已建议 genealogist 结晶"判别维已在分类层实装 ≠ 回测消费它"——本审计补充第二条候选结晶点："z 状态向量存在源 A（结构唯一性层 R(g)）与源 B（收益估计层 r）两个不同粒度定义，代码实装遵循源 B，H 轴缺失是遵循源 B 的忠实结果而非实装疏漏"。
6. **影响声明**：纯只读，零 git，零代码改动。本文件产出 = 审计判定 + b2 施工级规格输入。待实装（b2 任务）：`econ_positive.rs` 信号元组/桶键类型/dx 断言 + `mu_estimator.rs` MuClass 补 Ord 派生。不改 `coverage.rs`（H 轴判别函数已正确，只是不接入 z）、不改 `selector.rs`（z_of_candidate 已正确，直接复用）。
