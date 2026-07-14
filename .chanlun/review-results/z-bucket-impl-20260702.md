# b2 实装结果包：econ_positive.rs 桶键扩维 Y→Z（task #83）

- 工位：ws-zimpl（task #83，goal g-full-mutex-impl）
- 日期：2026-07-02
- 规格：`codex-review-20260702-214731-ec29.md`（codex #81 审计 5 修正 + H 轴裁定）+ `full-mutex-b1-design-20260702.md`（b1 设计稿）+ `alpha分离.pdf` p2 方框
- 认识论等级：**L1**（分桶/z 构造是确定性变换，验证管线正确，零信息增量）；产出的 μ̂ 值 **L2**（真实数据可否证，见 L2 报告 `econpositive-*.md`）
- 环境：Rust 编译 + 1402 lib 测试全绿；dx harness 20K 窗 bit-exact 对拍过；L2 alpha 20K 窗跑通产 Z 分桶报告

---

## 1. 结论

`econ_positive.rs` 的 alpha 回测 per-class 分桶键从**粗投影 Y=(level,δ,bsp_class)** 升到**完整状态 Z=`MuClass`**（含 σ_p/短差/仓位态/H=完整 R(g)），闭合编排者点破的"回测跑在未完全实装策略上"缺口。核心机制：`collect_signals` 在候选组装循环内调既有 `z_of_candidate(c)` 桥（复用 `selector.rs` role→z 映射），信号载体从裸 6 元组升具名 `RawSignal`（携带 `z: MuClass`），`SignalDecomp` 增 `z` 字段，三处生产分桶键改 keyed on `d.z`。

codex #81 五修正逐条落地：
1. **信号携带 z:MuClass 不扩裸元组**（修正1）：`RawSignal` 具名 struct 消除位置错配（原 6 元组第 5 位是 `sigma_higher` 被误标 δ）；`z` 由 `z_of_candidate` 构，`sigma_higher` 保留独立诊断字段。
2. **canonical key 用 i_class 不用 bsp_class()**（修正2）：分桶 keyed on 整个 `MuClass`（含未压缩 6-bit `i_class`），报告列展示 `z.i_class` 非 `bsp_class()`（后者压主类会丢 2B/3B 重合）。
3. **H 进 canonical Z**（修正3/H 轴裁定 `accept`）：`MuClass` 增 `horizontal: Option<Horizontal>`，`z_of_candidate` 填 `Some(c.role.h)` 升完整 R(g)=(H,V,δ)18 类；`UClass::project_to_u` 默认丢 H（selection 层抗 winner's curse，裁定 `conditional`）。
4. **sigma_higher 不并入 z**（修正4）：诚实保持诊断字段——它是结构层上级走势净方向，非持仓树父声部 σ_p，二者不合并。
5. **dx 守恒写维度无关通用形式**（修正5）：新增断言 `Σ_{z∈Z 桶} actual_pnl == Σ_全体 decomps`（细 key 求和回粗 key），对任意细化维恒成立，不写死 σ_p 一维。

**H 轴字段类型订正（相对 codex 字面建议）**：codex 建议 `horizontal: Horizontal`（非 Option）。实装为 `Option<Horizontal>`——因 `from_certificate` 的裸证书分量（ℓ,δ,I_γ,σ_p,仓位态）不含前兄弟关系 H，`pi_bsp_timing` 从 `Voice` 构 z 无 H 源。强制 `Horizontal` 会逼这些路径填 `First` 占位=**声明膨胀**（231/no-claim-inflation：伪造"无前兄弟"）。`None`=此口径未定 H（诚实），`Some(h)`=真候选路径（`z_of_candidate`）填。alpha 回测路径恒走 `z_of_candidate` ⟹ 恒 `Some` ⟹ H 真参与分桶（L2 报告实测 role 列显 F/SF/SR 无 None）。此偏离满足 codex **意图**（MuClass 载 H，R(g) 忠实）且更严格。

## 2. 定义依据

- **原文 z**：`alpha2.pdf §12/§16` `z=(ℓ,δ,I_γ,父声部方向,短差/顺势,仓位态)`；`递归完全分类买卖点.pdf §7-8/P6-P7` `R(g)=(H,V,δ)`18 类；`细分类优势定理 §13`（回测须按 z 分桶，补维不降 oracle alpha）。
- **代码锚点**：`selector.rs:144 z_of_candidate`（role→z 桥，本次加 `Some(c.role.h)`）；`mu_estimator.rs:69 MuClass`（加 `horizontal` 第 7 维 + `from_certificate` 填 None）；`coverage.rs:758 Horizontal`（加 Hash/Ord derive）；`econ_positive.rs` `RawSignal`（新 struct）/`collect_signals:355`（push z）/`pair_signals:403`（携 z 入 SignalDecomp）/桶键 3 处 keyed on `d.z`/dx 守恒断言/`signals_dx` dx harness 同步 z。

### 九维对照表（`alpha分离.pdf` p2 方框 Z_i=(ℓ,δ,I_γ,σ_higher,r,ω,β^div,d,c)）

| 原文维 | 语义 | 状态 | 代码锚点 / 诚实缺口说明 |
|--------|------|------|------------------------|
| **ℓ** 级别 | z 分量，去根化前沿 | ✅ 已接 | `MuClass.level` ← `z_of_candidate(c.level)`；桶键含 |
| **δ** 方向 | ∈{±1} 绝对方向 | ✅ 已接 | `MuClass.delta`；桶键含 |
| **I_γ** 一二三类 | {0,1}⁶ 非坍缩 64 类 | ✅ 已接 | `MuClass.i_class`（未压缩 6-bit `class_index`）；桶键含（修正2） |
| **σ_higher** 上级方向 | 结构层上级走势净方向 | ⚠️ 数据源在，**未接入 z**（诚实） | `sigma_higher_at`→`SignalDecomp.sigma_higher`（666 号，CSV 诊断）。codex 修正4：结构层量≠持仓树 σ_p，不并入 canonical z（否则冗余/混淆）。保留独立诊断，不入桶键 |
| **r** 角色 R(g)=(H,V,δ) | 18 类完全角色 | ✅ **本次接入** | H=`MuClass.horizontal`（`Some(role.h)`）；V=`parent_dir`+`short_swing`+`position`（`role.v` 推）；δ 同上。补 H 前 z 只投 V 轴（codex 修正3 坐实"称完整 R(g) 不忠实"），本次补齐 |
| **σ_p** 父声部方向 | ∈{-1,0,+1} | ✅ **本次接入** | `MuClass.parent_dir` ← `z_of_candidate` 从 `role.v`（Ambient→0/FollowParent→δ/ShortDiff→−δ）。是 r 的 V 轴推导量，桶键含 |
| **ω** 仓位状态 | 根/子声部 | ✅ **本次接入** | `MuClass.position`（Root/Child）← `z_of_candidate` 从 `role.v`。桶键含（L2 报告 role 列 R/C 可见） |
| **β^div** 背驰强度 | 力度 K 分箱细分 | ⚠️ 数据源部分在，**未接入 z**（诚实缺口，非 P0） | 力度代理在 `ForceProxies`/`div_cand`（#10/#19），但 β=细分变量非定义必要（D 二值谓词未形式化，b1 定案表 P2-⑧）。不进 z 分桶。**231 诚实**：背驰强度维在 z 缺失，不虚报 |
| **d** 风险距离 | 风险退出距离 | ❌ **无 z 构造路径数据源**（诚实缺口） | 风险距离是 `RiskPolicy`/gate 层概念（stop/force_flat），非 `MuClass` 分类分量。z 是"操作状态分类"，d 是"该状态下的风险实现量"——范畴不同，不进 z。**231 诚实**：z 无 d 维 |
| **c** 成本状态 | 成本模型状态 | ❌ **逐笔量非分类维**（诚实缺口） | `SignalDecomp.ce_unit` 携成本，但成本是**类内逐笔实现量**（`(P_in+P_out)·fee`），非 z 全互斥分类维。μ(z)=E[X_γ\|z] 的 X_γ 已扣 C（成本进收益值，不进桶键）。**231 诚实**：c 不是 z 分桶维 |

**接入小结**：9 维中 ℓ/δ/I_γ/r(H+V)/σ_p/ω 六维**已在 z**（本次补 r 的 H 轴 + σ_p/ω 接入 alpha 桶键）；σ_higher/β^div 数据源在但**故意不接**（诊断/非定义必要）；d/c **无 z 分量语义**（范畴不同）。无虚报（231）。

## 3. 边界条件（结论翻转）

- (a) 若 `z_of_candidate` 与 `from_certificate` 构的 z 在**同一 estimator 内混用**（一路 observe 一路 query）⟹ `horizontal` None vs Some(h) 桶不命中 ⟹ μ 查空。**已核**：生产 `build_walk_forward_mu`（l3）observe+query 均 `z_of_candidate`（恒 Some）；`pi_bsp_timing` 均 `from_certificate`（恒 None）——各路径内自洽。仅 2 个 runner 测试手建 est 混用，已修（H=First 匹配）。**未来新增混用路径会静默丢桶**——这是设计正确行为（未知 H 的 z ≠ 已知 H 的 z），非 bug。
- (b) 若 codex 复审裁定 H 应删（对 μ 无 L2 贡献）⟹ `horizontal` 字段作废，z 退 6 维。本实装 `Option<Horizontal>` 使此回退低成本（`z_of_candidate` 改填 None + 删字段）。
- (c) 若 Z 桶普遍 n̄<Le Cam 下界（高维稀疏）⟹ alpha 估计走 `UClass` 降维（`project_to_u` 已丢 H，抗 winner's curse）。Z 桶键仅作 oracle 上界（§13），不作 selection（codex `h_axis_in_default_selection: conditional`——`train_winner_class` 选类层**保持粗 Y 键**未升 z，正是此裁定的落地）。

## 4. 下游推论

- (a) 任何"回测验证了完全互斥分类 alpha"的声明现核 `econ_positive.rs` 桶键=Z（本次前是 Y）。当前 alpha 数字变为**完整 Z 状态的 μ̂**——L2 20K 窗实测已见 Z 细分效应：`(level=0,δ=-1,sell2)` 单一 Y 桶按 σ_p/H 分裂为 3 个 Z 桶（`σ_p=-1/Child/SR` n=4、`σ_p=0/Root/First` n=19、`σ_p=0/Root/SR` n=1），旧混合池被解开。
- (b) `W-VERIFY`(#13/#51)/`perm_test` 统计路径**仍按 (ℓ,bsp,δ,σ_p) 4 维**手取桶键（不读 `MuClass.horizontal`）——codex 修正5 订正"统一到 Z=MuClass"不成立，实为"共享 σ_p 维"。二者 z 构造口径不同（perm_test 用 `from_certificate`=None H），互不影响。
- (c) H 轴裁定影响 `MuClass` 维数（6→7），是 goal 后续所有 z 分桶断言的前置。`from_certificate` 40+ 调用点（多为测试）零改动（`horizontal` 默认 None）——H 只在真候选路径 `z_of_candidate` 显式填。

## 5. 谱系引用

- **231/formalization-validity-domain**（有效域<定义域）：alpha 回测跑 Y 桶=在 Y 有效域声明 Z 完备性的代码级坐实，本工位闭合。H 字段 `Option` 而非强制 `Horizontal`=避免 `First` 占位声明膨胀（231 应用于类型设计）。
- **230/231**：`sigma_higher`/`β^div`/`d`/`c` 未接入 z 的诚实缺口标注（不虚报数据源不在或范畴不同的维）。
- **codex #81**（`codex-review-20260702-214731-ec29.md`）：5 修正 + H 轴 `h_axis_in_canonical_z: accept` / `h_axis_in_default_selection: conditional` 全权裁定的落地。
- **b1 定案表**（#79）：P0 缺口订正（role/σ_p 已实装分类层+状态层，独缺 alpha 桶键接入），本工位=接入侧。
- **建议 genealogist 新结晶**："判别维已在分类层实装 ≠ 回测消费它——alpha 桶键滞后于分类层是'有效域<定义域'(231)在实装路径的具体形态"（codex 建议）。

## 6. 影响声明

**改动**（5 文件，Bash heredoc 落盘读回验证，不 git）：
- `mu_estimator.rs`：`MuClass` 加 `horizontal: Option<Horizontal>` 第 7 维 + `from_certificate` 填 None + import Horizontal + `project_to_u` H 丢弃文档；`Horizontal` 无关（derive 在 coverage）。
- `coverage.rs`：`Horizontal` 加 `Hash, PartialOrd, Ord` derive（作 MuClass 分量 + BTreeMap 键）。
- `selector.rs`：`z_of_candidate` struct-update 覆盖 `horizontal: Some(c.role.h)`。
- `econ_positive.rs`：`RawSignal` 具名 struct（续做 pre-existing WIP，替 6 元组）+ `SignalDecomp` 加 `z` + `collect_signals`/`pair_signals`/dx harness `signals_dx` 携 z + 3 处生产桶键 keyed on `d.z` + 报告表展 σ_p/role/H 列 + dx 守恒断言（细 key 求和回粗 key）+ L1 自检测试升级（`bucket_key_is_full_z_l1` 验 I_γ+σ_p 细分）。
- `runner.rs`：2 个 χ 过滤测试手建 z 加 `Some(Horizontal::First)` 匹配 `z_of_candidate` 查询口径（否则 None vs Some 桶不命中）。

**不改**：`coverage.rs`/`mu_estimator.rs` 分类层判别函数（已实装正确）；`train_winner_class` selection 层保持粗 Y 键（codex 裁定 selection 不无条件升 z 防 winner's curse）；`W-VERIFY`/`perm_test` 4 维手取键（共享 σ_p，非全 MuClass）；任何 settled 定理。

**护航验证**：`cargo test --release --lib` **1402 passed / 0 failed / 100 ignored**（含 mu_estimator 27+、perm_test、selector、#82 残差管线全绿）；dx harness `acc_classification_level_hole_dx` 20K 窗 **`signals_dx == signals_prod` bit-exact 含 z 对拍过**；L2 `l2_btc_capturable_spread_diagnosis` 20K 窗跑通产 Z 分桶报告 + dx 守恒 debug_assert 未触发。

**认识论如实**：z 分桶接入=L1（管线正确性）；接入后 μ̂ 值可否证=L2；接入本身不声明"Z 桶有 alpha"（那是 goal 后续全量 L2/L3 rerun 的产出）。role/σ_p/H 接入为真，σ_higher/β^div/d/c 未接入为真（诚实缺口，非虚报）。
