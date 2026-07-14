# 完整的策略.pdf（严格实装版 16 节验收规格）对照定案 — Lead 本体逐节审读

日期 2026-07-03。源=docs/formal-chain/完整的策略.pdf（编排者 7-02 22:50 下载，钦定「严格的实装版」）。对照基线 HEAD=a25db9aef1 后。

## 逐节对照（✅已实装 / ◐部分 / ✗缺口）

| 节 | 要求 | 判定 | 证据 |
|---|---|---|---|
| §1 状态空间 x_t 四账本分开 | 结构/声部/资金/净额账本分立 | ✅ | 576=C 双层并置(R,TW)+声部树 P^sep+净额 N_t+M_t 身份层(anc 已修)+Ω_t(#113) |
| §2.1 bit-exact T^inc=T^full | 增量=全量，frontier 必重算 | ✅ | #88/#93 advancing c546b5633c 全档 bit-exact |
| §2.2 AncOK 祖先生命期包含 | A_{t+1}=AncOK[(A\D)∪B] | ✅ | pi_theta_step AncOK 活动集+f2 P0-2 |
| §3 真递归区间套 J_child⊆J_parent | 区间包含非端点相等 | ✅ | #77 否证伪影+#101 bottom-up bit-exact+#100 四验收测试 |
| §4 Cand 非 MACD 一票否决 | 力度签名 𝔉+三种预注册 Θ | ◐ **G1** | ForceStateA4 机器+prereg 三套 Θ 已冻结(71f78103c7)；但①热路由未激活(#115 在飞)②生产 Cand 门仍 MACD 面积单 proxy 硬门③A4 缺 TV/SubMovePower |
| §5 六类+StructBreak 进状态键 | I_γ⊆{B1..S3,StructBreak} | ✅ | #62 四纤维+#83 6-bit 不压扁 |
| §6 完整互斥状态 z（~20 维） | 含 Ndepth/CandType/Jchain/σ_higher/TStage/ηBucket/RiskMode/CostBucket/MarginState/ExitType/e | ◐ **G2+G3** | 现 MuClass 8 维。**G2 冲突**：PDF 明令 z 必含 (ℓ,δ,σ_higher)，codex #81 修正4 裁「σ_higher 不并入 z」——PDF 权威>codex，须重裁。**G3**：其余 ~10 维未进 z（数据源大多在：Ndepth=NestCert 深度/TStage=TW/RiskMode+MarginState=#113/ExitType=typed exit），消费侧接入工程；统计层稀疏须 UClass 并列（i_class×δ 共线教训适用） |
| §7 全互斥解释器 P1..P10 固定优先级 | 强平/TW 事件统一进解释器 | ◐ **G5** | interp ≺_Θ 三桶+#94 桶级等价证明✅；但 P1 强平=KThetaRiskGate 独立兜、TW 事件=GAP3 桥独立——未统一进 P1..P10 序（#92 显式留界） |
| §8 声部树 v=(carrier,γ,σ,n) | 短差 σ=−σ_p+祖先闭合 | ✅ | attach_bsp_carrier_indexed+f2 P0-2 边界声明+AncOK |
| §9 正规出场 typed exit | X^full 用 τ^typed 非 τ^reverse | ◐ **G4 待核** | π 层 typed 动作在（interp close 桶/K_Θ force_flat）；**统计层 ResidualTrade 出场时点是否 typed exit 待核**——若 walk-forward 窗口/反向信号出场则 §15 P7 缺口成立 |
| §10 三阶段资金 GAP3 | 双账本+事件桥+γ_t 四态+Ready/EnterReady | ✅ | GAP3 rework barrier-gated+funded_campaign（memory：EarningShares 可达）+576=C |
| §11 仓位风险投影 | q_Θ 按完整 z 分维+K_Θ 含保证金/强平/毛头寸/OQ-9/多空双开 | ◐ **G7** | LexArgmin+Schedule+K_Θ✅；保证金/强平 #113✅；OQ-9✅；多空=逻辑双记执行净额✅；**最大毛头寸约束待核**；q_Θ 现按 z 子集分维 |
| §12 alpha 选择器 χ=LCB_OOS>θ | 非 p<0.05 | ✅ | χ门+decontam LCB+功效门口径完全一致 |
| §13 策略全定义证明（11 假设整合） | ∀x ∃!O=π(x) | ◐ **G6** | 分项证明齐（partition/shadow-fold/AncOK/LexArgmin 平局），整合定理文档未写（小） |
| §14 策略互斥证明 | ΣC_j=1 | ✅ | #43/#54 partition proof+#94 |
| §15 checklist 九项 | — | 7✅ 2✗（P2=G1、σ_higher=G2）+P7=G4 待核 | — |
| §16 π^full 压缩公式 | 十环链 | 链各环大部分在，缺口即 G1-G7 | — |

## 缺口清单（按工作量）
- **G1（P2，中）**：Cand 门从 MACD 硬门改为可替换力度签名（#115 激活后+门改造为 Weak_Θ 三口径预注册）
- **G2（冲突重裁，小改动大裁定）**：σ_higher 进 z——PDF vs codex #81 修正4 冲突，须 codex 重裁（PDF=编排者钦定权威）
- **G3（大）**：z 扩维至 §6 完整形态（~10 新维消费侧接入；统计层 UClass 并列防碎裂）
- **G4（待核→可能中）**：统计层 X_i 出场时点是否 typed exit
- **G5（中）**：强平/TW 事件统一进解释器 P1..P10 固定优先级
- **G6（小）**：全定义证明整合文档
- **G7（小待核）**：K_Θ 最大毛头寸约束
