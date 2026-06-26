# Task #50 结果包 — 几何螺旋层 T₄₇-T₅₉（层D，t-spiral）· FINAL v3（codex 裁决"选2"落地）

★★编排者代理（codex）裁决（2026-06-25，物证 /tmp/codex_spiral_ruling.md，session exit0）：**选 2**。
解耦"进 E-set"与"TrueCompleteClassification"——群论事实可进 E-set，但用 gatekeeper 标签降格，绝不冒充真完全分类。

产出文件：`formal/Tlayers/Spiral.lean`（431 行，26 theorem，namespace `Spiral`）。

## 1. 结论（v1→v2→v3 范式演化）
- v1：D∞ 群呈现 Lean-证一遍 → 被编排者口径"证群论本身=不够"否定。
- v2：只留 by-construction 侧（递归构造子导出），群关系侧标 A 上浮矛盾。
- **v3（本版，codex 裁决落地）**：两侧并存，gatekeeper 标签分离——
  - **PART I `TrueCompleteClassification`**：T₅₂/T₅₃/T₅₄/T₅₉/T₄₇，挂 `Move` 初代数 μF / `ValidTower` 余代数 νG，完全分类 by 结构归纳/余归纳导出。
  - **PART II `StructurePartitionOnly`/`QuotientByLabel`（subkind GroupTheoreticFact/FiniteGroupQuotient）**：T₅₆/T₅₇/T₅₈/T₄₅/Burnside46，真群论事实可 Lean 证，进 E-set 但**禁标 TrueCompleteClassification**。

**验证（权威 CI 路径，Tlayers 已进 defaultTargets）**：
- `lake build Tlayers` 成功（14 jobs，Spiral built）；`lake env lean Tlayers/Spiral.lean` 退出 0；零 sorry/admit/axiom。
- 下游 `Strict` lib：Decomp（t-decomp-uniq #62 文件，消费我 T₅₂/T₅₃）+ BSP/Causal/Op 全绿；仅 `Trend`（#58 独立工位 mid-edit）失败，非我依赖。

### PART I — TrueCompleteClassification（递归构造子穷尽导出）
| T 编号 | theorem | 递归构造子来源 |
|---|---|---|
| T₅₃ 连接结合律【缺瓦】 | `T53_connection_assoc`/`T53_connection_length`/`T53_connection_unit` | `List Move` append 初代数结合律（结构归纳） |
| T₅₄ 表里对偶【缺瓦】 | `T54_surface_from_interior_total`/`T54_surface_determined_by_interior`/`T54_compose_surface_eq` | `classifyMove` 里→表，`outcome_total` 结构归纳 |
| T₅₂ 多义性=覆盖多提升【缺瓦】 | `T52_fiber_exhaustive`/`T52_gauge_fixes_unique`/`T52_empty_above_floor_none`/`T52_lifts_cover_same_base` | 同 base 多 `Move` 提升 + 区间套 foldl 归约（List 初代数） |
| T₅₉ σ 自相似 | `T59_sigma_self_similar_constructors`/`T59_sigma_shift_same_constructor_set` | `ValidTower` 余代数每层同构造子（`rstar_same_constructors` 余归纳） |
| T₄₇ 无第四类 BSP | `T47_bsp_no_fourth_kind` | `BspKind` 构造子穷尽 |

### PART II — 群论 Lean 事实进 E-set（gatekeeper 降格，禁标 TrueCompleteClassification）
| T 编号 | theorem | gatekeeper 标签 |
|---|---|---|
| T₅₆ h²³=σ | `DInf.T56_h_pow_23_eq_sigma` | StructurePartitionOnly · GroupTheoreticFact |
| T₅₇ τhτ⁻¹=h⁻¹ | `DInf.T57_tau_h_tau_inv` | StructurePartitionOnly · GroupTheoreticFact |
| τ²=e / τστ⁻¹=σ⁻¹ / 群公理 / 正规形枚举 | `DInf.tau_involution`/`tau_sigma_tau_inv`/`mul_assoc`/`mul_e_left`/`mul_e_right`/`mul_inv_left`/`rot_homomorphism`/`dinf_normal_form` | StructurePartitionOnly · GroupTheoreticFact |
| T₅₈/T₄₅ ℤ/23 角向商 | `T58_angular_domain_card`/`T58_sigma_angular_zero`（+ `angularQuotient`） | QuotientByLabel · FiniteGroupQuotient |
| Burnside 46 基本域基数 | `burnside_fundamental_domain_card_eq_46` | StructurePartitionOnly · GroupTheoreticFact |

## 2. 定义依据
- 走势初代数 μF = `RecursiveConstruction.Move`（segment+compose），走势分解定理二（第17课）。
- T₅₃ necessity §9+第36课；T₅₄ reading_b_t_dual_design+第91/93课+540号；T₅₂ covering_space_interval_nesting_proof+第33课；T₅₉ necessity T₅₉+ValidTower；T₄₇ necessity T₄₇。
- PART II D∞：necessity §8.4.2/§8.4.3（G_spiral=⟨h,τ|τ²=e,τhτ⁻¹=h⁻¹⟩，h²³=σ，ℤ/23=⟨h⟩/⟨h²³⟩，基本域 23×2=46）。

## 3. 边界条件
- PART I 成立依赖 `Move`/`ValidTower` 初代数/余代数定义忠实（codex PASS session 019eff21）。
- PART II gatekeeper 标签依赖 codex 裁决"选2"；若编排者改裁（如要求群关系也升 TrueCompleteClassification），需把 D∞ 重铸为递归 datatype（当前 D∞ 元素是平坦标签 rot/refl，无递归构成）。

## 4. 下游推论
- t-gapmap §8.2 层D：T₅₂/T₅₃/T₅₄/T₅₉/T₄₇ 勾 TrueCompleteClassification；T₅₆/T₅₇/T₅₈/T₄₅/Burnside46 勾 E-set 但标 StructurePartitionOnly/QuotientByLabel（非 TrueCompleteClassification）。
- t-decomp-uniq（#62）消费我 `Spiral.T52_gauge_fixes_unique`/`T53_connection_assoc`/`connectMoves`/`isAmbiguous` 组装 ∼ₙ 商集——接口已保留，Decomp 文件 build 绿。

## 5. 谱系引用
- 603 范式（真完全分类=构造子穷尽 by initiality）；090号（声明膨胀禁止，故群论侧不冒充完全分类）；231号（有效域≠定义域，Burnside 数值侧）；no-workaround（v2 矛盾上浮，codex 裁决消解）。

## 6. 影响声明
- 改动：`formal/Tlayers/Spiral.lean`（v2→v3）+ 本结果包。未碰 lakefile/Formal.lean/其他模块。
- Tlayers 已被别的工位加进 defaultTargets（roots 含 Spiral）——我的文件现在是 CI build lib 一部分（v1 的"未进 defaultTargets" caveat 已消解）。

## 7. codex 物证（E-set 群论事实，成功）
按 Lead 指令重试 codex（`gpt-5.5 -c model_reasoning_effort=high --sandbox read-only --skip-git-repo-check`，stdin pipe），**仅作 "Lean E-set 群论事实" 物证**（审 D∞ 关系/商/计数数学正确性），**非** TrueCompleteClassification 物证。
- **session id：`019f0026-d959-7e40-82d6-aeb0aa4d3e64`**（CX_EXIT=0，19758 tokens，物证 /tmp/codex_eset_v2.txt）。
- **裁决：9 条 D∞ 群论事实全部 [有效]**——mul_assoc/单位律/逆律/T56(h²³=σ)/T57(τhτ⁻¹=h⁻¹)/τ²=e/τστ⁻¹=σ⁻¹/ℤ23角向商/基本域46。
- codex 独立验证关键非平凡点：`refl n 自逆 = (hⁿτ)⁻¹=hⁿτ 依赖 τhτ⁻¹=h⁻¹`（正确）。
- codex 第9条限定"基本域46 ≠ D∞ 本身有限"——与本文件 §8 标注一致（基数是基本域代表数，非群阶）。
- 注：D∞ 群论事实数学正确性首要由 Lean build 担保（`lake build Tlayers` 退出 0 = 所有等式 rfl/simp/omega 机器验证）；codex 是异质冗余确认。前序 3 次 codex 调用失败（exec positional/超时）已诚实记录，本次 stdin pipe 成功。
