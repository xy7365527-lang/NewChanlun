# 预注册：β^div 力度支配态进桶（prereg-beta-div-20260703）

- 工位：ws-beta（task #112 实装）
- 冻结时间：2026-07-03（**跑数前冻结**，§5 纪律；`关于背驰.pdf` §9.2「不能先看结果再选」）
- 设计依据：`beta-bucket-design-20260703.md` v2 §5.1（三套 Θ）+ codex-beta-ruling conditional 4 必修
- 认识论：桶键构造 = L1（确定性变换）；桶 μ̂ 值 = L2（可否证）；「哪个 Θ 有 alpha」= L2/L3（OOS，本 prereg 不声明）

## 冻结项（改动前不可调）

### 允许力度族（冻结）
```
𝒜₄ = { macd_area, dif_peak, price_amplitude, price_speed }   # 现有 4 proxy；缺 TV/递归 = ForceStateA4 宽近似
```

### Θ_DOM（主键，默认）— 已实装
```
β^div := ForceStateA4(C, A) ∈ { Dominated, Dominates, Tie, Incomparable }
  实现: divergence.rs ForceProxies::force_state()（唯一支配序原语）
  判据（C=seg_c 后段 vs A=seg_a 前段）:
    Dominated    = ∀m∈𝒜₄ m_C ≤ m_A ∧ ∃m m_C < m_A     # 确定背驰
    Dominates    = ∀m∈𝒜₄ m_C ≥ m_A ∧ ∃m m_C > m_A     # 力度延续
    Tie          = ∀m∈𝒜₄ m_C = m_A
    Incomparable = 部分 <、部分 >                        # 不作背驰确认（codex-beta ②）
  进桶: MuClass.force_state = Some(ForceStateA4)（z_of_candidate_with_force）; None（无力度源口径）
  selection: 默认不进 UClass（保 divergence bool，抗 winner's curse；codex #81 同构）
  fullz 置换: perm_test 六元组 base 已含 force_state；进 base 前逐层过 δ-共线检查（下）
```

### Θ_LEX（备选，OOS 对照）
```
β^div := WeakThetaMode::Lex（现有 weak_theta）∈ { Weak, NotWeak }
  级别结构 ≻ DIF ≻ 面积：DIF 可判用 DIF，DIF 相等退 macd_area
```

### Θ_SCORE（备选，连续→K 分箱，OOS 对照）
```
m := dif_peak                                          # 冻结单 proxy（第17课黄白线主）
β_norm := (m_A − m_C) / (m_A + m_C) ∈ (−1, 1)
K := 3, 边界冻结 { ≤0 非背驰, (0,0.33) 弱, ≥0.33 强 }    # 边界不得事后调
进桶: 分层键（先过 δ-共线检查）
```

### δ-共线检查（准入门，冻结阈值）
```
每 (ℓ, i_class) 层，force_state × δ 列联表:
  通过 = min(n_{fs,+1}, n_{fs,−1}) ≥ n_min ∧ mix(fs) ≥ mix_min ∧ Cramér's V ≤ V_max ∧ exact_p > 0.05
  冻结: n_min = 5, mix_min = 0.1, V_max = 0.2
  不过 ⟹ 该 (ℓ,i_class,fs) 桶降级「仅 μ 分层，不进 fullz 置换 base」（谱系 iclass-delta-collinearity）
```

## 纪律
- 三套 Θ 各自 OOS 回测，事前注册（本文件），不 peek。
- 负结论功效门槛沿用 [[project_oddeven_mu_identity]]：LCB<0 前须核 n_eff 过门槛，否则 inconclusive 非证伪。
- 不预设结果（231号）。force_state 生产接线（BspPoint.force + 增量真传）落地后方可跑 L2；接线前 force_state 恒 None，桶退化回 7 维 fullz（诚实，非虚报）。

## 谱系
[[project_iclass_delta_collinearity_perm_degeneracy]]（force_state 进 fullz base 的 δ-共线前置门依据）；[[project_oddeven_mu_identity]]（绝对量 proxy 规避 δ-共线 + 负结论功效门槛）；231/formalization-validity-domain（ForceStateA4 宽近似标注）；codex-beta-ruling-20260703（4 必修）。
