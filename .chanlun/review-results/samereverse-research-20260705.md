# SameReverse 第 4 类 Vertical 严格研究

> 工位：swarm/ws-samereverse2（基因 073a/274号，原则15 原子任务）。
> 权威来源：`docs/formal-chain/买卖点.pdf`（§3/§4/§7.3/§7.5/§15/§18）+ `docs/formal-chain/完整的策略.pdf`（§6）。
> 认识论层级：**L0**（PDF 定义对齐 + 代数互斥性），零 L2 信息增量；可达性标注 L2 未覆盖。

---

## 1. 结论（六要素-结论）

**formal-chain 明确定义 SameReverse，且 SameReverse ≠ ShortDiff。** 当前 `vertical_relation`（`rust/src/theta_v0/strategy/coverage.rs:1202-1215`）缺失级别判定 `ℓg vs ℓα`，把 PDF 的 `SameReverse`（同级别反向）误并入 `ShortDiff`（次级别短差），导致 `dir_weight` CASE 1（§7.5 `sg=sα` 豁免）错误施加于 SameReverse 腿，`eta_adv` 在 Follow preset 的 sign=−1 槽不可达（`coverage.rs:1331-1333, 1367`）。

修复 = `Vertical` 枚举加第 4 类 `SameReverse`，`vertical_relation` 的 `δg=−σ_p` 分支按 `ℓg vs ℓα` 分叉。实装所需信息（`CoverageElement.level: u32` + `CoverageElement.parent: Option<usize>`，`coverage.rs:161,163`）**已存在**，无需新数据源。

## 2. 定义依据（六要素-定义依据）

### 2.1 Relx(g) 三分类（买卖点.pdf §3，pdftotext line 230-250）

```
αx(g) = 包含 g 的最深活动声部（父声部选择器）
Relx(g) = Root,  αx(g)=∅
          Same,  αx(g)≠∅ ∧ ℓg = ℓαx(g)     ← 同级别
          Sub,   αx(g)≠∅ ∧ ℓg < ℓαx(g)     ← 严格次级别
```
PDF 原文："这里不允许模糊的'差不多是次级别'。必须严格满足 ℓg < ℓαx(g)。"

### 2.2 操作角色五分类（买卖点.pdf §4，line 270-310）

```
Rolex(g) ∈ {RootDir, SameFollow, SameReverse, SubFollow, ShortDiff}
ShortDiff:   Relx(g)=Sub  ∧ δg = −σα(g)     ← 次级别反向
SameReverse: Relx(g)=Same ∧ δg = −σα(g)     ← 同级别反向
```

### 2.3 ShortDiff 充要条件（买卖点.pdf §4，line 333）

```
Role(g) = ShortDiff ⟺ ℓg < ℓα(g) ∧ δg = −σα(g)
```
**`ℓg < ℓα(g)` 是 ShortDiff 的定义性条件，不是辅助条件。** 缺它则非短差。

### 2.4 SameReverse 语义（买卖点.pdf §7.3，line 542-560）

```
Role(g)=SameReverse ⟹ ℓg=ℓα(g), δg=−σα(g)
语义："先关闭原同级别声部，再按规则建立新方向"（反手）
"它不是短差，因为短差要求严格次级别 ℓg<ℓα(g)。所以 SameReverse ≠ ShortDiff。"
```

### 2.5 §7.5 sg=sα 约束仅对 ShortDiff（买卖点.pdf §7.5，line 626-654）

```
ShortDiff 语义："父仓保持不变，建立独立反向子声部"（Δs_child_α = 0）
同股数短差要求：sg = sα    ← 仅 §7.5 ShortDiff 段落，SameReverse 段落（§7.3）无此约束
```

### 2.6 §15 互斥性证明（买卖点.pdf line 1387-1412）

```
SameReverse(g) ⟺ Rel(g)=Same ∧ δg=−σα(g)
ShortDiff(g)   ⟺ Rel(g)=Sub  ∧ δg=−σα(g)
Same ∩ Sub = ∅ ⟹ SameReverse, SubFollow, ShortDiff 三者互斥
```

### 2.7 §18 最终分类原则（买卖点.pdf line 1620-1640）

```
同级别反向 ⇒ 同级别关闭/反手，不是短差    ← SameReverse
次级别反向 ⇒ 短差：父仓保持＋反向双开     ← ShortDiff
数学上：短差 = Sub ∧ δ = −σparent
```

### 2.8 §6 禁一刀切（完整的策略.pdf line 467-482）

```
"σhigher 的收益符号会随级别翻转，L1 是主要超 beta 来源，不能用全局'顺上级/逆上级'一刀切。
 所以完整状态必须含：(ℓ, δ, σhigher)。
 否则会把'低级别逆上级短差'和'高级别逆上级接飞刀'混在一起。"
```

### 2.9 代码现状对照

`vertical_relation`（`coverage.rs:1202-1215`）：
```rust
if sigma_parent == 0 { Vertical::Ambient }
else if dir_sign(δ) == sigma_parent { Vertical::FollowParent }
else { Vertical::ShortDiff }  // ← 仅查 δg=−σ_p，未查 ℓg<ℓα ⟹ 吞掉 SameReverse
```
`dir_weight` CASE 1（`coverage.rs:1338-1341`）：`role.v==ShortDiff ⟹ return 1.0`，把 §7.5 豁免施加到含 SameReverse 在内的全部逆上级腿。
代码自承（`coverage.rs:1331-1333`）："sign=−1 的腿即 ShortDiff，CASE 1 已截——sign=−1 槽...经 role 路径不可达"。

## 3. 边界条件（六要素-边界条件）

1. **SameReverse 可达性依赖同级别元素存在**：需 `ℓg=ℓα(g)` 的元素（走势类型内同级段反向信号）。若真实数据无此结构（全树严格 `ℓg<ℓα`），SameReverse 经验空——**L2 未覆盖**，本实装不声称可达，仅声称**定义层可达性**（CASE 3 sign=−1 槽不再被 ShortDiff 路径截断）。
2. **ℓα 读取依赖 parent 链**：`parent=None`（边界胚元 ∂）⟹ σ_p=0 ⟹ Ambient，不到 SameReverse/ShortDiff 分支，无 ℓα 读取问题。
3. **ℓg=ℓα 但 σ_p=0**：不可能——σ_p=0 已截到 Ambient。SameReverse/ShortDiff 前置条件含 σ_p≠0。
4. **若 PDF 框架未来引入 ℓg>ℓα**（g 比 α 更高级）：当前 Rel 三分类 `{Root,Same,Sub}` 穷尽（line 250），不含 `ℓg>ℓα`；若引入，需第 5 类，本实装不覆盖。

## 4. 下游推论（六要素-下游推论）

1. **eta_adv 可达**：Follow preset 下 SameReverse 腿走 CASE 3 sign=−1 槽（`coverage.rs:1367 slot(1.0, eta(eta_adv))`），`eta_adv` 不再死参数。
2. **三套预注册全可达**：Neutral（恒 1.0）/ Follow（same=1.0, opp=eta_adv）/ Adversary（same=eta_same, opp=1.0）的 sign=−1 槽均有 SameReverse 路径消费。
3. **§6 (ℓ,δ,σhigher) 三元组部分补全**：SameReverse 把 ℓ 以"同级/次级"二元形式引入 V 轴。完整 (ℓ,δ,σhigher) 连续 ℓ 进入状态仍是后续工作（与本任务正交，不在此修）。
4. **mu_estimator 桶键影响**：若 `Vertical` 进 `MuClass` 桶键（z 维），SameReverse 与 ShortDiff 分桶——旧池化结论的有效域需重标（formalization-validity-domain：定义域扩 ⟹ 旧桶有效域缩）。

## 5. 谱系引用（六要素-谱系引用）

- **无直接前序谱系**记录 SameReverse 缺失（本工位首次识别）。相关：
  - spec-execution-gap 约束链 016→032→033→034（声明-能力不一致）：代码注释声称 18 类完全分类（`coverage.rs:920-923`），但 V 轴实际仅 3 类且吞掉 PDF 第 4 类 SameReverse——典型 spec-execution-gap。
  - formalization-validity-domain（231号）：`Vertical` 三分类的"完全分类"声明有效域 < 定义域（PDF 五分类）。
  - MEMORY `project_two_pdf_z_form_conflict`：完整策略 §16 唯一权威——本任务 PDF 引用均落在 §16 权威链内。

## 6. 影响声明（六要素-影响声明）

- 改 `coverage.rs:911-918`（`Vertical` 枚举 +SameReverse）、`:1202-1215`（`vertical_relation` 分叉）、`:1322-1347`（σ_higher 解码 + CASE 1 收紧）、`:1278,1302`（role_v 文档）。
- **不动**：入场源（extract_elements / from_classification_levels）、Horizontal 轴、depth_weight、Neutral bit-exact 基线。
- 影响范围：dir_weight 输出仅在 Follow/Adversary preset + SameReverse 元素上变化；Neutral 全不变。

---

## 7. 实装设计

### 7.1 Vertical 枚举（coverage.rs:911-918 改）

```rust
pub enum Vertical {
    Ambient,     // σ_p=0
    FollowParent,// δg= σ_p
    SameReverse, // δg=−σ_p ∧ ℓg=ℓα   ← 新增（同级别反向，PDF §7.3，非短差）
    ShortDiff,   // δg=−σ_p ∧ ℓg<ℓα   ← 收紧（次级别短差，PDF §7.5）
}
```

### 7.2 vertical_relation（coverage.rs:1202-1215 改）

```rust
pub fn vertical_relation(elements: &[CoverageElement], e_idx: usize) -> Vertical {
    let e = match elements.get(e_idx) { Some(e)=>e, None=>return Vertical::Ambient };
    let sigma_parent = parent_sign(e.attached_dir);
    if sigma_parent == 0 { return Vertical::Ambient; }
    let delta = dir_sign(direction_of(e.eps));
    if delta == sigma_parent { return Vertical::FollowParent; }
    // delta == −sigma_parent：按 ℓg vs ℓα 分叉（PDF §3 Rel 定义 + §4 ShortDiff 充要条件）
    let ell_g = e.level;
    let ell_alpha = match e.parent {
        Some(p) => elements.get(p).map_or(ell_g, |parent| parent.level), // 父越界防御=同级（不触发短差）
        None => 0, // parent=None 已被 sigma_parent==0（∂）截断，理论不到此
    };
    if ell_g < ell_alpha { Vertical::ShortDiff } else { Vertical::SameReverse }
}
```
`ponytail:` 父越界/None 防御归 SameReverse（非短差），保全函数性；正常路径 parent 真存在。

### 7.3 dir_weight CASE 结构（coverage.rs:1337-1355 改）

```rust
pub fn dir_weight(role: &OperationRole, depth: u32, config: &VoiceConfig) -> f64 {
    // CASE 1: ShortDiff 豁免（§7.5 sg=sα）—— 仅次级别短差，不再含 SameReverse
    if role.v == Vertical::ShortDiff { return 1.0; }
    // σ_higher 解码（prereg §3.3）
    let sigma_higher: i8 = match role.v {
        Vertical::FollowParent => dir_sign(role.delta),
        Vertical::Ambient => 0,
        Vertical::SameReverse => -dir_sign(role.delta),  // δg=−σ_p ⟹ σ_p=−δg（与旧 ShortDiff 同解码）
        Vertical::ShortDiff => -dir_sign(role.delta),
    };
    if sigma_higher == 0 { return 1.0; } // CASE 2
    let sign = dir_sign(role.delta) * sigma_higher; // CASE 3
    theta_dir_slot(&config.theta_dir, depth, sign)   // SameReverse ⟹ sign=−1 ⟹ Follow preset 用 eta_adv
}
```
**关键**：SameReverse 不进 CASE 1（非短差，不受 §7.5），走 CASE 3 sign=−1 槽 ⟹ `eta_adv` 可达。

---

## 8. 数学证明（neutral bit-exact 不破坏）

**命题**：实装后 `ThetaDirPreset::Neutral` 下 `dir_weight ≡ 1.0`，`leg_target` 输出 bit-exact == 实装前。

**证明**：
1. `theta_dir_slot`（`coverage.rs:1361-1371`）对 `Neutral` 分支返 `1.0`（与 sign/depth 无关）。
2. CASE 1（ShortDiff）返 1.0；CASE 2（sigma_higher=0）返 1.0；CASE 3 调 `theta_dir_slot`。
3. SameReverse 从 ShortDiff 分离后：CASE 1 不命中（v≠ShortDiff）→ 非 CASE 2（σ_p≠0）→ CASE 3 → Neutral 下 `theta_dir_slot=1.0`。
4. 故 Neutral 下所有 v∈{Ambient,FollowParent,SameReverse,ShortDiff} 的 `dir_weight` 恒 1.0，`w_depth×w_dir` 与实装前一致 ⟹ `leg_target.units` bit-exact。∎

**互斥性**（L0 代数）：`SameReverse ∩ ShortDiff = ∅`，因 `ℓg=ℓα` 与 `ℓg<ℓα` 在全序 `(L,<)` 上互斥（PDF §3 line 250 已证）。四类 `{Ambient, FollowParent, SameReverse, ShortDiff}` 按 `(σ_p, δg·σ_p, ℓg vs ℓα)` 三判据互斥穷尽 V 定义域（σ_p∈{0,±1}, δg∈{±1}, Rel∈{Same,Sub}）。

## 9. 认识论等级标注（formalization-validity-domain 231号强制）

| 命题 | 等级 | 信息增量 |
|---|---|---|
| SameReverse ≠ ShortDiff（PDF 引用） | L0 | 零（PDF 已证，line 560） |
| 四分类互斥性 | L0 | 零（全序互斥同义反复） |
| Neutral bit-exact 不破坏 | L0/L1 | 零（preset 返 1.0 同义反复） |
| SameReverse 在真实数据可达 | **L2 未覆盖** | 正（待真实数据验证） |
| eta_adv 在 SameReverse 腿上产生方向 alpha | **L2 未覆盖** | 正（待回测） |

**诚实声明**：本实装仅修复**定义层**可达性（sign=−1 槽不再被错误截断），**不声称** SameReverse 经验可达或 eta_adv 产生 alpha——后者须 L2/L3 真实数据回测，是后续独立工位。

## 10. 严格性自检（no-workaround / no-patch-mentality）

- ✅ 非 workaround：不是"加分支让它跑"，是删除错误的"`δg=−σ_p→ShortDiff`"判定，用 PDF §3+§4 的严格分叉"`ℓg<ℓα→ShortDiff; ℓg=ℓα→SameReverse`"替换。
- ✅ 非声明膨胀：不声称 18 类经验全可达（保留 `coverage.rs:932-933` 的 L2 未覆盖声明），新增 SameReverse 同样标 L2 未覆盖。
- ✅ 直面 PDF 权威：formal-chain 有定义且 ≠ShortDiff，**不是否定性结果**；如实报告"定义存在、代码缺失"。
