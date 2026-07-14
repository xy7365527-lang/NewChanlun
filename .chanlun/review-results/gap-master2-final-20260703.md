# gap-master-2 终稿：全策略缺口总排查 2.0 全量口径（v2）

工位：ws-gap2 | 任务 #150/#156 | 编排者穷尽令扩展版（对照面=docs/formal-chain/ 全部推导链）
本文件是汇总层终表；逐 PDF/逐节明细以下列落盘件为准，不在此复制：
- v1 主文（完整的策略 16 节 / 一类买卖点 11 页+8 问 / proofs §D / 0703 结果包收割 / L4-L5 核查）：`gap-master2-20260703.md`（fe649ab779）
- 分片：shard1 完全分类/买卖点族（21623575a8，s1b 独立复核一致+4 处补强）、shard2 递归/级别族（056f794c54）、shard3 alpha/检验族（1c3eb2553d）、shard4 策略/资金/对冲族（93477f63f0）、shard5 gap/杂项族（a5ec2ffaf0）

## 穷尽性基底（对照完备性论证链）

| 对照面 | 覆盖 | 凭据 |
|---|---|---|
| 完整的策略.pdf 16 节 | 16/16 逐节对当前代码验证 | v1 §A 逐节表 |
| 一类买卖点.pdf 11 页 | §9 管线四环+修复序 #142→#145 逐一对应；8 问 7 落地+1 形态存疑 | v1 |
| 其余 35 份 formal-chain PDF | 五分片逐份：已装给凭据（review-results+代码锚点，锚点当前 HEAD 复核）/缺口分类 | shard1-5 |
| proofs-full-strategy §D+加固清单 | 全部 OPEN 项收割 | v1 §B |
| 0703 结果包缺口小节 | 大部逐节收割；**7 文件仅 header 级定位**（rerun5/q4-fullpi-results/highlevel-rerun/stopguard-fix2/type1-zero-probe/g5-interpreter-mapping/strategy-spec-for-external-review）——遗留留白，见 B37 | v1 覆盖声明 |
| 高级别塔 L4/L5 | 健康判定+翻转条件 | v1 §塔 |

A 类非空 ⟹ 不触发「无未知必装缺口」论证义务。

## A 类终表（必装缺口，12 条；实装建议+依赖序）

| # | 缺口 | 严重性 | 实装建议 | 依赖序 |
|---|---|---|---|---|
| A1（收窄修订） | z 缺维**残余**=Jchain + d（结构止损距离，shard4 三无项）。原 v1 所列 σ_higher/Ndepth 经分片核实**已装**（MuClass 第9维 #132 / nest_depth 第11维，mu_estimator.rs:87 十三维）；TStage/ηBucket/CostBucket/CandType 归 B（D-3 登记）。新存疑：MuClass 侧与解释器状态侧（mutex.rs）维度同构性未核对（680 双侧判据） | 中（原高，收窄后降） | Jchain/d 维生产者+入 MuClass；先做双侧同构核对定实装面 | 依赖 A6 透传通路；入置换分层须三处同批改（B22） |
| A2（收窄修订） | 支配序**原语已装**（divergence.rs:343-352 ForceStateA5 四态，#112/#115 入 z 第8维热路由 bit-exact）；残余=Θ_DOM 作为 D 判定口径接入 judge + Θ_SCORE + 三口径 OOS（divergence.rs:286 自注未跑） | 中 | judge_first 增 Θ 口径开关+三口径 OOS 批 | 依赖 A6；边界：若 codex 认定 ForceStateA5≠Θ_DOM 同构则撤销收窄（shard4 边界③） |
| A3 | Weak 力度判据仅 MACD 面积（一类买卖点 p6/p10 明文 secondary defect）；含 D-1 子项 TV/SubMovePower proxy | 中 | force_conformance.rs 接入 judge_first；TV/SubMovePower proxy 实装后 ForceStateA5 升名 ForceState | 依赖 A6 |
| A4 | AncOK 放宽偏差（recursive_tower.rs:64 Stale 分支伪造 parent:None，spec §13；s1c 核验订正行号）。部分缓解在案：anc persistent overlay 已装（persistent.rs） | 中·交裁 | 交 Lead 裁是否升必修；若修=held_leg_tree_index 走 persistent registry 真 parent | 与 B33（增量 extract 覆盖确认）联动 |
| A5 | γ_t 四桶（Deficit/Zero/PositiveUnsafe/PositiveSafe）未见对应物 | 中·存疑 | 专项确认 closed_loop 是否以其他形态承载；无则实装枚举+路由 | 独立 |
| A6 | Candidate 不携 ForceProxies ⟹ collect_signals z 构造点透传断裂，fullz 置换 force_state 恒 None | **高** | 打通 classifier→strategy 透传（Candidate 结构体加 proxies 字段） | A1/A2/A3 共同上游，最先做 |
| A7 | 出场侧 μ(z,exit)：ExitType 出场时刻 z 构造点缺 | 中 | 出场事件处快照 z+exit_type 入 μ 样本流 | 依赖 #124 typed exit（已 completed，shard1 证实——通路已备） |
| A8（降级待核） | econ_positive.rs τ^reverse 残留——本分支未提交改动已无 grep 命中 | 低 | merge 后复核，命中为零即销项 | 独立 |
| A9 | position_node 持仓节点=容器非买卖点叶子（级别容器.pdf；nesting-fugue P0-2） | **高（P0）** | 声部=(carrier,entry_certificate,σ) 三元组入账本头寸层 | 与 A11 同族；先裁 s2 存疑①（#114/#140 覆盖关系） |
| A10 | margin M2/M3 接线+ADL/资金费强平未建模（「margin 分段快照」在办项不覆盖此部分） | 中 | 接 IBKR/多品种前收口；BTC 当前不 binding | 外部数据项后置 |
| A11 | 区间套 P0-1 下沉触发状态机（实测 95% 退化 d=1）+ P0-3 多空对冲 overlay 账本（nesting-fugue P0） | **高（P0）** | 按 nesting-fugue 26 条规格实装缺口 7 条中的 P0 两条 | 先裁 s2 存疑①；#148 落地后 depth 分布须回归（s2 存疑②） |
| A12 | 子声部双视图（结构树 T_i vs carrier 森林 K_i endpoint-complete）——P1-3 OPEN，恒零是定理非 bug | **高** | 双视图实装（gap-master-list P1-3，依赖 P1-1）；谱系 676/648 在案 | 依赖 P1-1 |

依赖序总结：**A6 最先**（三条力度族的上游）→ A1/A2/A3 → A7；A9/A11 待 s2 存疑①裁定后并行；A12 走 P1-1→P1-3 链；A5/A8 独立随时；A10 外部数据后置。

## B 类（登记留白）：v1 B1-B28 不变 + 新增 9 条

- B29 去根化平移等变缺口（原 dlpdf-a 矛盾-B）：level0 免门破 𝒞_Θ(S_k x)=S_k𝒞_Θ(x)。s1b 性质修订：当前码 lvl==0 语义=「递归底无次级别⟹存在性免门」（econ_positive.rs:816/835，引第29课下沉锚），非任意特殊规则而是**有限塔截断**（dlpdf-a §5 定性、231 有效域实例）。关闭条件=level0 统一区间套判据或谱系结算接受性截断。建议 genealogist 立条。
- B30 「alpha 重启 prereg 前置清单」（shard3 四项打包，重启 alpha 检验时**全部自动升 A**）：①B̂ᵢ 残差减法+h/time-block 分层维；②生产 selector LCB 门控（selector.rs 现无 LCB）；③方向不对称回归 μ_sell−μ_buy（block bootstrap/cluster SE）；④shrinkage 层级收缩。
- B31 χ(ℓ,q) 统计门：终局四口径 INCONCLUSIVE 降级（谱系 666/667 结构维已装）。
- B32 trim 删尾语义未逐行核实（命中 perm_test/wverify_run/econ_positive 三文件）——若非删尾诊断则 alpha检验.pdf「已装大半」降级。
- B33 anc 增量 extract 是否已被 #84-#93 增量塔 O(n) 实质覆盖——确认后可销项或转 A。
- B34 wfcontain 核对边界条件2（compose 子区间连续无跳跃/无重叠实现正确性）未展开验证。
- B35 κ（barrier 缓冲）无敏感性网格（margin-design §3.5 仅对 buffer 强制）。
- B36 背驰 4.2 Comparable_ℓ 是否作为四条件严格合取接入判定，无显式凭据。
- B37 本排查自身留白：7 个 0703 结果包文件边界条件节未逐行收割（清单见穷尽性基底表）。

## 闭合移除表（stale 判定，从一切遗留清单移除）

| 旧判定 | 闭合凭据 |
|---|---|
| dlpdf-a P0-① 角色维 | coverage.rs OperationRole+econ_positive.rs:142 消费侧已落（680 订正） |
| dlpdf-a P0-② σ_p | 639 settled+#132 MuClass 第9维 |
| dlpdf-a P1-⑤ 短差 | #117 f3c-shortdiff completed |
| dlpdf-a 矛盾-A StructBreak | **679 settled**（第四类纤维，三处门控恒拒，350K dx 重测）——终局结算级 |
| gap-master-list P1-2 frontier OPEN | bottomup-nest NO-SHIP（三窗 bit-exact 差异全0+parity assert）+parser-frontier-fix+codex-87+highlevel-rerun #97 维持；INDEX 行50 后记为准 |
| v1 交裁项① #124 状态 | shard1 证实 completed（interp.rs:151-169 P1..P10 解释器） |

## C 类（在办）

#148 升级语义 / #149 两维+A4/C3（挂#148）/ margin 分段快照（外部数据）/ gemini 补验（配额）/ Lean Realize D-8 / 3 doctest / #135 prereg 冻结批（B2/B6/B8/B9/B21c/B30 触发时归口）/ GAP3 返工（本分支在途）。终局勿重开：I_γ 集合桶键（beta-bucket 线已否证）。

## 存疑交裁清单（汇总，8 条）

1. s2①：P0-2/P0-3 与 #114 f3-fugue/#140 gap3-realize 覆盖关系（裁定后 A9/A11 定实装范围）。
2. s1①：γ/gate 分派层 2B/3B 重合类坍缩残余（#124 覆盖 exit/interpret 侧，gate 分派侧无专项凭据）。
3. s1②：二值背驰谓词 D_t^δ 是否需独立形式化（或由 A2/A3 自然承载）。
4. s4：κ 敏感性网格是否补 B 登记（→已按 B35 预登记，裁否决则删）。
5. s4：Comparable_ℓ 严格合取（→B36 同上）。
6. v1：A4 AncOK 放宽是否升必修。
7. v1：A5 γ_t 四桶存在形态。
8. 新：MuClass 侧与 mutex.rs 解释器状态侧 z 维度同构性（680 双侧判据的遗留应用）。

## 结果包六要素

1. **结论**：全量口径 A=12（高4：A6/A9/A11/A12）/B=37/C=8+1 终局；闭合移除 6 条 stale。
2. **定义依据**：37+ 份 formal-chain 推导链全量+proofs §D+0703 结果包+塔核查（基底表）。
3. **边界条件**：①B37 留白可能藏未收割缺口；②A2/A1 收窄各有撤销条件（shard4 边界③④）；③A8 以 merge 后码为准；④分片对照多数经 dlpdf 系列全读凭据中继（未重读 PDF 原文），继承其提取遗漏风险（自声明逐页全读，风险低）。
4. **下游推论**：A6 是力度族三条的解锁键；A9/A11/A12 三条 P0/P1 级结构缺口决定账本-声部层能否承载多 instance；B30 是 alpha 任何重启的前置门。
5. **谱系引用**：676/648（子声部）、679（StructBreak settled）、680（双侧判据）、685（归因订正）、639/666/667（σ 族）、231（有效域）、裁定C。
6. **影响声明**：新增本文件；v1 的 A1/A2 表述被本文收窄修订、交裁项①被解除（以本文为准）；不改代码。
