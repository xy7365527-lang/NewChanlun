# 预注册：Θ_DOM 作为 D 判定口径的三口径 OOS（prereg-a2-thetadom-oos-20260704）

- 工位：ws-thdom（task #163，A2 残余）
- 冻结时间：2026-07-04（**跑数前冻结**，§5 纪律；`关于背驰.pdf` §9.2「不能先看结果再选」）
- 上位 prereg：prereg-beta-div-20260703.md（三套 Θ 进桶预注册）——本 prereg 是其 Θ_DOM 从「进桶维」升「D 判定口径」的判定层扩展，**amendment**：力度族从 𝒜₄ 修订为 𝒜₅（依据 codex-a2-thetadom-20260704.md 裁定 ISOMORPHIC-WITH-AMENDMENT——支配序结构同构，𝒜₄→𝒜₅ 是同 schema 的 proxy 参数扩展；统计/报告口径全部标 A5 amended，不冒充原 A4）
- 认识论：口径开关接入 = L1（确定性判定变换）；各口径桶 μ̂ = L2（可否证）；「哪个口径有 alpha」= L2/L3（OOS，本 prereg 不声明结果）

## 冻结项（跑数前写死，不得事后调）

### 允许力度族（A5 amended）
```
𝒜₅ = { macd_area, dif_peak, price_amplitude, price_speed, tv }   # divergence.rs ForceFeatures 现码 5 proxy
```

### 三判定口径（D = 一类买卖点 buy1/sell1 的背驰确认谓词）
```
G1  MacdArea（对照基线，默认）:
    D := Area(C) < Area(A)                       # segments_diverge 现行冻结判据，bit-exact 不变
G2  ThetaDom:
    D := ForceStateA5(C,A) == Dominated          # ForceProxies::force_state() 唯一支配序原语
    force 无源（dif/closes 空 ⟹ ForceProxies=None）⟹ D=false（无 5-proxy 无背驰确认，诚实非 fallback）
G3  Conjunction:
    D := G1 ∧ G2                                 # MACD 面积衰减 ∧ 全支配序衰减
```
- 开关承载：`ThetaConfig.divergence_gauge`（默认 `MacdArea`——判定口径变更影响信号集合，非默认切换）。
- 有效域：本口径开关只作用于 **judge_first 趋势背驰 D**（一类 buy1/sell1）。盘整背驰证书（judge_pan_div）、
  二类 divergence_of、buy1 之外的 MACD 消费点不在本 prereg 范围（Weak 判据非 MACD 化归 A3 #164）。

### Θ_SCORE（同批实装的原语层，非判定口径）
```
m := dif_peak                                   # 冻结单 proxy（第17课黄白线主）
β_norm := (m_A − m_C) / (m_A + m_C) ∈ [−1, 1];  m_A + m_C = 0 ⟹ β_norm := 0（双零无力度=非背驰）
K := 3, 边界冻结 { ≤0: 非背驰, (0,0.33): 弱, ≥0.33: 强 }
角色：分层键候选（beta-bucket-design v2 §4.3 方案 B）。分层键接入点在 mu_estimator/ResidualTrade
（ws-etab 并发域，本工位不触碰）——本批只落原语（β_norm + bin），接入登记诚实缺口。
```

### OOS 协议
```
数据:      BTC（btc_1m_full.json），PREREG_WINDOWS BTC wf_anchored 12 窗（冻结不变）
聚合:      逐窗 test 段独立 build_mu_from_bars（walk-forward OOS 残差，wverify_run 同一聚合器）
桶键:      (ℓ, bsp_class, δ, σ^H)（wverify_full 同口径）
统计:      n / n_eff / mean(Y) / LCB(z=1.645) / perm_p（stratified_delta_perm_p, N_PERM=200, seed 冻结）
           / 删尾 mean(−3) / AlphaState（decontam 同口径）
对比量:    三口径各自跑全链（config.divergence_gauge 切换 ⟹ 信号集合 ⟹ ledger ⟹ 残差）；
           报告并列三口径逐桶表 + 一类信号计数差（G2/G3 相对 G1 的信号收缩量——Dominated ⊆ 宽判）
判定纪律:  负结论功效门槛沿用 [[project_oddeven_mu_identity]]：LCB<0 前核 n_eff；不过门槛 ⟹ inconclusive 非证伪。
           三口径独立报告，不做事后挑桶；否定性结果照实（161号）。
```

## 预期方向（非结果声明）
Dominated 是 5-proxy 全序衰减 ⟹ G2 一类信号集合 ⊆ G1 的宽判集合的强子集（更少更严）；G3 ⊆ G1 ∩ G2 = G3。
样本量收缩 ⟹ n 更小、winner's curse 风险更高——n_eff 门槛照防。

## 谱系
codex-a2-thetadom-20260704.md（A5 amendment 裁定，本 prereg 的合法性来源）；prereg-beta-div-20260703.md（上位三套 Θ）；[[project_oddeven_mu_identity]]（功效门槛）；231/formalization-validity-domain（L1/L2 分级）；090（默认口径 bit-exact 不变=非默认切换的严格形式）。
