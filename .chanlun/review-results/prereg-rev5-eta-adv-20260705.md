# prereg-rev5｜eta_adv(ℓ=0..5) 数值表冻结（g3 OOS 前置，SameReverse 实装后首冻结）

**工位**：swarm/ws-etadv（codex-challenger，代码层异质否定源）｜ topo_address: swarm/ws-etadv ｜ 基因 073a/274号
**日期**：2026-07-05 ｜ 分支：gap3-rework-codex9-fix ｜ parent_callback: main
**模式**：codex decide（数值冻结裁定，read-only，未改代码）
**命令**：`codex exec --skip-git-repo-check --sandbox read-only -c model_reasoning_effort="high"`
**prompt 文件**：`/tmp/codex-eta-adv-prompt-20260705.md`（完整交互记录）
**output 文件**：`/tmp/codex-eta-adv-output-20260705.txt`（5699 行，tokens used 152,355）
**认识论等级**（231号）：分级结构（按 ℓ 分行 + sign 槽）= L0；eta_adv 具体数值 = L2 extra-缠论；可达性论断 = L1（sr-impl 已 bit-exact 自检）；L3+ 接飞刀实证 = L2（BTC 单标）。

---

## 〇、工位复核结论（Codex 异质否定成立，推翻蜂群初步裁定，HIGH 严重性）

**本轮 Codex 否定成立，且推翻了蜂群基于文档的初步裁定。** 工位独立核实 Codex 的两条关键引用，均为真：

| 复核项 | 判定 |
|---|---|
| prereg-rev4 §10.2 已冻结「eta_adv[ℓ≥3] 强收缩 ✅ 采纳 ≪1」 | ✅ 真实（亲读 line 408，已 commit 冻结意图） |
| highlevel-mu 文档「L3+ 逆上级=接飞刀最大失血 / 符号翻转边界 L2→L3」 | ✅ 真实（亲读 line 12/65/77/126/132，L2 BTC 实证） |
| Codex 否定是否已被覆盖 | ❌ 未覆盖——蜂群初步裁定（L1 焦点 + eta_same 同构）**违背 §10.2 已冻结意图**，是真实设计层错误 |
| 严重性 | **HIGH**——若按蜂群初步裁定冻结，会把 §6 page4 两重信息压缩成单一 L1 焦点，丢失「高级别逆上级接飞刀」语义 |

**蜂群初步裁定的两个真实错误**（Codex 否定的内核）：
1. **违背 prereg-rev4 §10.2 已冻结意图**：§10.2（commit 958f722674）明确「✅ 采纳 高级别逆上级(ℓ≥3)强烈收缩 ⟹ Θ[ℓ≥3][-1]=η_adv(ℓ)≪1」。SameReverse 实装使此槽**真正生效**，蜂群初步裁定却把它重新解释为 L1 焦点（与 eta_same 同构），等于**撤销已冻结的 §10.2 设计**——违反 135号（冻结先于跑数，已冻结不可本轮重释）。
2. **L1 超 beta 来源归属错误**：§6 page4「L1 是主要超 beta 来源」+ highlevel-mu line 12「超 beta 收益来自**级别相位**（L1 层贡献最大），**不来自上级方向过滤**」——L1 超 beta 是顺上级/级别相位性质，已由 eta_same[1]=0.50 捕获。蜂群初步裁定把它再分配给 eta_adv[1]=0.50（逆上级 SameReverse）是**重复分配**（L1 超 beta 不可能同时在顺上级和逆上级两个方向）。

**工位增量洞察**（超出 Codex 裁定）：eta_same 与 eta_adv 的**非对称分工**才是 §6 page4 两重信息的最严格形式化——
- eta_same = [0.70, **0.50**, 0.70, 0.70, 0.70, 0.70] → 捕获「L1 主要超 beta 来源」（顺上级同向腿，级别相位）
- eta_adv = [0.70, 0.70, 0.70, **0.50**, **0.50**, **0.50**] → 捕获「高级别逆上级接飞刀」（逆上级 SameReverse，符号翻转后）

蜂群初步裁定（强行对称化）的根因：把 §6 page4 的**两重不同信息**（L1 顺上级焦点 + 高级别逆上级焦点）误读为**单一焦点**（L1），导致 eta_adv 错误镜像 eta_same。Codex 的「经济语义不对称」论证正确指出了这点。

---

## 一、eta_adv(ℓ=0..5) 冻结数值表（最终裁定）

```
eta_adv(ℓ=0..5) = [0.70, 0.70, 0.70, 0.50, 0.50, 0.50]
                    ℓ0    ℓ1    ℓ2    ℓ3    ℓ4    ℓ5
```

**性质**：L2 extra-缠论风险政策参数（616号）；分级结构（按 ℓ 分行 + ℓ≥3 强收缩组分界）= L0（§6 page4 + §10.2 + highlevel-mu 符号翻转边界 L2→L3 三重锚）；具体数值 0.50/0.70 = L2（标准网格 {0.5, 0.7, 1.0}，0.30 排除）。

### 逐级理由

| ℓ | eta_adv | 理由 |
|---|---|---|
| 0 | 0.70 | 低级别逆上级的主要风险（次级别短差）已由 §7.5 ShortDiff 豁免（w_dir≡1.0）隔离；SameReverse（同级别反手）在 L0 无强负证据（highlevel-mu line 77 低级别逆上级「更赚」，虽混合 ShortDiff 但偏正）；温和降权，不把 v1 变过激反趋势器 |
| 1 | **0.70** | **L1 超 beta 来源（§6 page4）是级别相位/顺上级方向**（highlevel-mu line 12），已由 eta_same[1]=0.50 捕获，不重复分配给 eta_adv；L1 逆上级读数偏正（短差反弹，line 77）；温和降权，把 L1 焦点让给 eta_same |
| 2 | 0.70 | **符号翻转边界层**（highlevel-mu line 77「符号翻转边界在 L2→L3」）；L2 是过渡层，证据不足，不机械单调，统一 0.70 是风险预算非 alpha 断言 |
| 3 | **0.50** | **符号翻转后第一高级别**；highlevel-mu 判定3 line 65「L3+ 逆上级=接飞刀，最大失血（−191.7 vs −46.9）」+ line 132「L3+ 禁止逆上级」；§10.2 已冻结「ℓ≥3 强收缩 ≪1」；用 0.50（标准网格最强收缩档） |
| 4 | 0.50 | 同 ℓ3，高级别组统一强收缩 |
| 5 | 0.50 | 同 ℓ3 |

**0.30 排除理由**（与 codex-ruling-eta 一致 + Codex Q2 强化）：
- L3+ 证据是 BTC 单标 L2（highlevel-mu 边界条件 line 118「仅 BTC，跨标的不可外推」）
- L3+ 逆上级 n=35 小样本，−191.7 均值受少数极端亏损主导，需 bootstrap CI 确认（line 126）
- SameReverse 语义是「先关闭原同级别声部，再建新方向」（反手，samereverse-research §2.4 line 42）——**反手不等于裸接飞刀**，可能是走势转折信号腿，过强收缩（0.30）误杀转折信号
- 0.30 进入冻结表须等 SameReverse 专属 OOS（非混合逆上级桶）确认稳定负 alpha + n_eff 充足

---

## 二、语义裁定（SameReverse 同级别反向腿的风险预算）

### 2.1 eta_adv 现在的精确语义（SameReverse 实装后订正）

SameReverse 实装（commit 67d20d292f）前，codex-ruling-eta 记录 eta_adv「空操作」（sign=−1 槽不可达，Follow 套 bit-exact==neutral）。实装后：

- Follow preset（config.rs:123 亲读）：`Θ[ℓ][+1]=1.0`（顺上级 FollowParent 保权），`Θ[ℓ][-1]=eta_adv(ℓ)`（逆上级 SameReverse 降权）
- SameReverse = Rel=Same（同级别 ℓ_g=ℓ_α）∧ δ_g=−σ_α（买卖点.pdf §7.3 line 619-628 亲读）
- dir_weight CASE 3：sign(δ_g·σ_higher)=−1 ⟹ 查 Θ[ℓ][-1]=eta_adv(ℓ)

**eta_adv 收缩的是「同级别反手腿（SameReverse）」的风险预算**——这才是 ChatGPT 原设想的「逆上级强烈收缩」的真正语义载体（codex-ruling-eta §一.1 记录的语义反转现在订正）。

### 2.2 SameReverse ≠ 裸接飞刀（语义二义性诚实声明）

SameReverse 语义（samereverse-research §2.4）：「先关闭原同级别声部，再按规则建立新方向」。这有两种经济解读：
1. **走势转折信号腿**：同级别走势完成，反手建仓捕获反向走势 → 正 alpha 来源
2. **高级别接飞刀**：在强趋势中同级别反手 → 负 alpha 来源

highlevel-mu 的「L3+ 逆上级接飞刀」实证（line 65）支持高级别 SameReverse 偏向解读 2，但**该读数混合了 ShortDiff**（未 SameReverse 专拆，line 126 自述），对 SameReverse 专属证据力是间接的。故：
- 高级别（ℓ≥3）SameReverse **倾向接飞刀**（降权 0.50，但留转折信号余地，不硬杀 0.30）
- 低级别（ℓ0-2）SameReverse **无明确接飞刀证据**（温和 0.70）
- 二义性的最终裁决须 SameReverse 专属 OOS（翻转条件见 §四）

---

## 三、与 ChatGPT 建议 + eta_same 的一致性/差异分析

### 3.1 与 eta_same 的关系：非对称分工（非镜像同构）

| 参数 | 数值表 | 焦点 | 捕获的 §6 page4 信息 |
|---|---|---|---|
| eta_same | [0.70, **0.50**, 0.70, 0.70, 0.70, 0.70] | L1（执行级之上第一超级别） | 「L1 是主要超 beta 来源」（顺上级同向腿，级别相位） |
| eta_adv | [0.70, 0.70, 0.70, **0.50**, **0.50**, **0.50**] | ℓ≥3（高级别组） | 「高级别逆上级接飞刀」（逆上级 SameReverse，符号翻转后） |

**两者分工而非镜像**：§6 page4 同时给出两重信息——(a) L1 是超 beta 主要来源（点状，顺上级方向），(b) 高级别逆上级是接飞刀（符号翻转，逆上级方向）。eta_same 捕获 (a)，eta_adv 捕获 (b)。

**蜂群初步裁定的错误**：强行把 eta_adv 镜像 eta_same（都用 L1=0.50 焦点），等于把 §6 page4 的两重信息压缩成单一焦点，丢失 (b)。Codex「经济语义不对称」论证正确：SameReverse（反手/转折风险）与 FollowParent（顺势 stacking 风险）是不同风险对象，不应同强度同焦点。

**与 eta_same 数值档位的一致性**：两者都只用标准网格 {0.5, 0.7}（0.30/1.00 不进冻结表），最强收缩档统一 0.50（与 codex-ruling-eta eta_same 冻结口径一致）。

### 3.2 与 ChatGPT 建议的关系：部分采纳

ChatGPT 建议「高级别（ℓ≥3）逆上级强烈收缩 eta_adv≪1（如 0.3）」。

| ChatGPT 条款 | 本轮裁决 | 依据 |
|---|---|---|
| 高级别（ℓ≥3）逆上级强收缩 | ✅ **采纳** | §10.2 已冻结 + highlevel-mu 判定3（L3+ 接飞刀最大失血）+ line 132（级别分段）；eta_adv[ℓ≥3]=0.50 |
| 收缩强度 0.30（≪1 硬杀） | ❌ **否决（用 0.50）** | L3+ 单标 BTC L2 + n=35 小样本 + 极端值主导 + SameReverse 反手≠裸接飞刀（可能转折信号）；0.30 须等专属 OOS（翻转条件） |
| 顺上级(q=+1)正常 | ✅ **采纳**（同 §10.2） | Follow 套 Θ[ℓ][+1]=1.0 |

**与 codex-ruling-eta 的一致性**：codex-ruling-eta 否决 ChatGPT「高级别强收缩」投射到 **eta_same**（理由：当前槽非逆上级，语义反转）——本轮不翻案。但 codex-ruling-eta Q4(b) 同时声明「一旦 sign=−1 经生产路径可达，eta_adv 就值得冻结」——本轮正是此条件满足后的冻结，ChatGPT 直觉在 **eta_adv**（真正的逆上级槽）上**部分恢复适用性**（高级别焦点采纳，0.30 强度否决）。

### 3.3 与 prereg-rev4 §10.2 的关系：真正生效

prereg-rev4 §10.2 line 408「高级别逆上级(ℓ≥3)强烈收缩 ✅ 采纳 落入 Θ[ℓ≥3][-1]=η_adv(ℓ)≪1」——codex-ruling-eta 记录此条款当时「形式采纳但实质落空」（槽不可达）。**SameReverse 实装后，§10.2 真正生效**：eta_adv[ℓ≥3]=0.50 是 §10.2 冻结意图的数值落地（0.50 满足「≪1」，0.30 因证据不足排除）。

---

## 四、翻转条件（OOS 结果触发的数值调整规则，135号：仅下一轮预注册可改）

### 4.1 数值翻转（下一轮预注册，非本轮 OOS 后追参）

1. **ℓ≥3 SameReverse 稳定负 alpha + 0.50 改善风险** ⟹ eta_adv[ℓ≥3] 从 0.50 降到 0.30
   - 触发条件：ℓ≥3 SameReverse 专属 OOS residual alpha 显著负 + n_eff/CI/跨窗或跨标确认 + 0.50 降权改善回撤/尾损/beta 暴露
2. **ℓ0-2 SameReverse 有正 residual alpha** ⟹ eta_adv[ℓ0-2] 升到 1.00（或废弃 Follow 套）
   - 触发条件：低级别 SameReverse 是转折信号（正 alpha），降权损害 Sharpe/收益且不改善风险
3. **ℓ≥3 SameReverse 反而是转折信号（正 alpha）** ⟹ Follow 套整体质疑
   - 触发条件：高级别 SameReverse 不是接飞刀而是转折，降权=误杀；Follow 套假设（顺上级保权）被否证
4. **全部 SameReverse 降权只降收益不改善 beta/回撤/尾损/turnover-adjusted** ⟹ Follow 套废弃
   - 与 eta_same 废弃条件对称（codex-ruling-eta Q4(a)）
5. **SameReverse 多数是关闭旧同级腿而非新增裸逆势暴露** ⟹ 不能用「接飞刀」口径下调
   - 若 SameReverse 实证是减仓/反手而非新增暴露，高级别强收缩失去依据

### 4.2 不翻转的锚（formal-chain 已结算）

- ShortDiff CASE 1 豁免（§7.5 s_g=s_α）——不随数据翻转
- σ_higher 经 q_Θ 通道（§11 page7）——定义性
- 禁全局一刀切（§6 page4）——w_dir 必须分级
- SameReverse ≠ ShortDiff（§7.3 vs §7.5，ℓ_g=ℓ_α vs ℓ_g<ℓ_α）——sr-impl 已实装

### 4.3 样本不足保护（135号）

若 ℓ≥3 SameReverse 样本不足或结果由少数极端交易主导（highlevel-mu line 126 警告 n=35 极端值主导），**保持本表，不做 OOS 后追参**。0.30 须等样本充足 + bootstrap CI 确认。

---

## 五、对 g3 OOS 跑数的影响

### 5.1 Follow 臂现在独立可达（三套全部独立）

SameReverse 实装使 sign=−1 槽经生产路径可达 ⟹ Follow preset 不再 bit-exact 退化为 neutral ⟹ **三套预注册现在真正独立**（修正 codex-ruling-eta「三套塌缩两套」结论——该结论基于 SameReverse 未实装的旧代码状态）。

| 套 | eta 表 | 检验假设 | ℓ 焦点 |
|---|---|---|---|
| Θ_dir_neutral | w_dir≡1.0 | （v0 bit-exact 自检基线） | — |
| Θ_dir_adversary | eta_same=[0.70, 0.50, 0.70, 0.70, 0.70, 0.70] | 顺上级同向腿是 beta stacking 噪声 | L1 |
| Θ_dir_follow | **eta_adv=[0.70, 0.70, 0.70, 0.50, 0.50, 0.50]** | 高级别逆上级 SameReverse 是接飞刀噪声 | ℓ≥3 |

### 5.2 三套并行 OOS（§9.2 不能事后选）

三套产生三个独立信号集，并行 OOS，事后选 = 数据窥探（§9.2）。neutral 套作 g2 实装 bit-exact 自检（OOS 须 = v0 基线 af8910d062，否则实装 bug）。

### 5.3 归因隔离（prereg-rev4 §10.3 保留）

本三套 OOS 只检验 q_Θ σ_higher 维的 signal 层效果，不混入 typed exit 改动（exit 路径 bit-exact 保留 v0）。typed exit 触发质量修复是 separate 预注册（prereg-rev4 §10.3 已声明）。

### 5.4 OOS 首跑硬测试

1. Θ_dir_neutral 套 OOS == v0 基线（af8910d062）——否则 w_dir 实装 bug（sign 解码错 / ShortDiff 未豁免 / SameReverse 误归 ShortDiff）
2. Follow 套 OOS ≠ neutral 套——否则 SameReverse 经验不可达（sr-impl 边界条件 1：L2 未覆盖 SameReverse 真实数据可达性），eta_adv 仍是空操作（本轮冻结不翻转，但须诚实标注 Follow 套 == neutral 即 SameReverse 经验不可达）

---

## 六、Codex 裁定全文摘要（Q1-Q4）

### 总裁定

Codex 否定蜂群初步裁定（「数值完全同构」理由不成立），但不采纳 ChatGPT ℓ≥3=0.30 强收缩。冻结表：

```
eta_adv(ℓ=0..5) = [0.70, 0.70, 0.70, 0.50, 0.50, 0.50]
```

### Q1 对称性

代码结构上 Follow/Adversary 是镜像槽位（config.rs:123 + coverage.rs:1406 亲读核实），但**经济语义不对称**：SameReverse 是同级别反手（关闭原声部+建新方向），FollowParent 是顺父延续腿（samereverse-research §42 已钉死边界）。用同一数值表会把「反手/转折风险」和「顺势 beta stacking 风险」混成同强度假设，不是最干净的隔离。

### Q2 高级别逆上级

ChatGPT 高级别逆上级风险**未被正确彻底否决**：prereg-rev4 §10.2 本身明确采纳「高级别逆上级强烈收缩」落入 eta_adv[ℓ≥3]（line 408）。SameReverse 实装后此直觉恢复适用性。但 0.30 过强：L3+ 证据是 BTC 单标 L2，文档自标 n=35、受极端亏损影响、需 bootstrap CI（highlevel-mu line 121/126）；SameReverse 可能是转折信号腿，不应直接当裸接飞刀硬杀。

### Q3 逐级理由

- ℓ0=0.70：低级别逆上级主要风险已由 ShortDiff 豁免隔离；SameReverse 无强负证据，温和降权
- ℓ1=0.70：蜂群 0.50 的 L1 对称理由不稳。L1 主要超 beta 来源 ≠ L1 SameReverse 必须强降权；历史分层 L1 逆上级反而正读数（highlevel-mu line 12/77），虽未 SameReverse 专拆但足够阻止强收缩
- ℓ2=0.70：中间层证据不足，不机械单调
- ℓ3-5=0.50：高级别逆上级接飞刀是有依据风险组；样本和 SameReverse 专属语义都不足冻结 0.30，用 0.50 作高层组收缩

### Q4 翻转条件

下一轮只按 SameReverse 专属 OOS 调整，不再用混合逆上级桶（已纳入 §四）。

---

## 七、结果包六要素

### 1. 结论

- **eta_adv(ℓ=0..5) 冻结为 [0.70, 0.70, 0.70, 0.50, 0.50, 0.50]**（ℓ≥3 强收缩组分界，非单调，标准网格 {0.5, 0.7}）。
- **语义**：eta_adv 收缩同级别反手腿（SameReverse，§7.3）的风险预算；SameReverse 实装后这是 ChatGPT「逆上级强烈收缩」的真正语义载体（codex-ruling-eta §一.1 语义反转订正）。
- **与 eta_same 非对称分工**：eta_same 捕获 §6 page4「L1 主要超 beta 来源」（顺上级，L1 焦点），eta_adv 捕获「高级别逆上级接飞刀」（逆上级，ℓ≥3 焦点）——两者分工捕获 §6 page4 两重信息，非镜像同构。
- **Codex 异质否定成立**：推翻蜂群初步裁定（L1 焦点 + eta_same 同构），理由是初步裁定违背 prereg-rev4 §10.2 已冻结意图 + L1 超 beta 归属错误。

### 2. 定义依据

- **买卖点.pdf §7.3**（pdftotext 亲读 line 619-628）：`Role(g)=SameReverse ⟹ ℓ_g=ℓ_{α(g)}, δ_g=−σ_{α(g)}`——同级别反手定义。
- **完整的策略.pdf page4**（pdftotext 亲读）：「σ_higher 收益符号随级别翻转，L1 是主要超 beta 来源，不能一刀切...否则把低级别逆上级短差和高级别逆上级接飞刀混在一起」——eta_adv ℓ≥3 强收缩 + L1 焦点归 eta_same 的双锚。
- **prereg-rev4 §10.2**（line 408，commit 958f722674 已冻结）：「高级别逆上级(ℓ≥3)强烈收缩 ✅ 采纳 ⟹ Θ[ℓ≥3][-1]=η_adv(ℓ)≪1」——本轮 eta_adv[ℓ≥3]=0.50 是此冻结意图的数值落地。
- **highlevel-mu-sigma-alpha-20260701.md**（L2 BTC 实证）：line 12（超 beta 来自级别相位非上级方向）/ line 65（L3+ 逆上级接飞刀 −191.7 vs −46.9）/ line 77（符号翻转边界 L2→L3）/ line 126（n=35 极端值主导需 bootstrap CI）/ line 132（级别分段 L0/L1 允许 L3+ 禁止）。
- **代码锚点**（L1 亲读）：config.rs:120-128（Follow/Adversary preset 槽位定义）+ coverage.rs:1406（same/opp 槽读表）+ sr-impl commit 67d20d292f（SameReverse 实装 + 936 passed）。
- **先例**：codex-ruling-eta-20260705（eta_same 冻结 + Q4(b) 翻转条件）、sr-impl-20260705（SameReverse 实装结果包）、prereg-rev4 §4.3（三套预注册结构）。

### 3. 边界条件（结论翻转）

- **eta_adv[ℓ≥3] 从 0.50 翻到 0.30**：仅当 ℓ≥3 SameReverse 专属 OOS（非混合逆上级桶）稳定负 alpha + n_eff/CI/跨窗跨标确认 + 0.50 改善回撤/尾损/beta（§四.1 条1）。
- **eta_adv[ℓ0-2] 从 0.70 翻到 1.00 或废弃 Follow 套**：仅当低级别 SameReverse 有正 residual alpha（转折信号），降权损害 Sharpe 不改善风险（§四.1 条2）。
- **Follow 套整体质疑**：仅当高级别 SameReverse 是转折信号非接飞刀（§四.1 条3）。
- **不翻转**：ShortDiff §7.5 豁免 / σ_higher 经 q_Θ 通道 / 禁一刀切 / SameReverse≠ShortDiff——四者 formal-chain 已结算。
- **有效域边界**：eta_adv 数值 L2（616号 extra-缠论）；L3+ 接飞刀实证 L2 BTC 单标不可外推（highlevel-mu line 118）；SameReverse 经验可达性 L2 未覆盖（sr-impl 边界条件 1）——若 Follow 套 OOS == neutral 则 SameReverse 经验不可达，eta_adv 空操作（冻结不翻，但须诚实标注）。

### 4. 下游推论

- **对 g3（跑数）**：三套预注册（neutral/adversary/follow）现在**真正独立**（SameReverse 实装使 Follow 臂可达），并行 OOS 不能事后选（§9.2）。neutral 作 bit-exact 自检；adversary 用 eta_same=[0.70,0.50,0.70,0.70,0.70,0.70]；follow 用 eta_adv=[0.70,0.70,0.70,0.50,0.50,0.50]。
- **对 codex-ruling-eta「三套塌缩两套」结论**：该结论基于 SameReverse 未实装的旧代码状态，**本轮订正**——三套现在独立（sr-impl 已实装 sign=−1 可达）。
- **对 §6 page4 双重信息的形式化**：eta_same（L1 顺上级焦点）+ eta_adv（ℓ≥3 逆上级焦点）分工，是 §6 page4「L1 超 beta 来源」+「高级别逆上级接飞刀」两重信息的最严格参数化。
- **对 ChatGPT 方向**：高级别焦点采纳（ℓ≥3=0.50 满足≪1），0.30 强度否决（证据不足）；与 codex-ruling-eta 否决 ChatGPT 投射到 eta_same 一致（本轮只在 eta_adv 真逆上级槽上部分恢复适用性）。
- **对 135号冻结纪律**：本轮冻结后 g3 OOS 过程中不改 eta_adv；数值翻转仅下一轮预注册（§四）。

### 5. 谱系引用

- **无新谱系**（本裁定是 codex 异质裁决 + 工位复核，非新概念发现）。
- **相关已结算**：
  - **090号**（严格性/声明膨胀）：蜂群初步裁定违背 §10.2 已冻结意图 = 重释已冻结设计，违反严格性；本裁定恢复 §10.2。
  - **231号**（有效域<定义域）：eta_adv 分级结构 L0，数值 L2；L3+ 接飞刀实证 L2 BTC 单标不可外推。
  - **135号**（冻结先于跑数）：本轮冻结后 g3 OOS 不改；翻转仅下一轮预注册。
  - **616号**（Θ 风险参数 extra-缠论）：eta_adv L2 须外部固定，数值是「合理冻结起点」非「唯一正确答案」。
  - **§6 page4**（L1 超 beta 来源 + 高级别接飞刀）/ **§7.3**（SameReverse 定义）/ **§7.5**（ShortDiff s_g=s_α 豁免）——formal-chain 三锚。
- **不确定是否有相关谱系**：SameReverse 经验可达性是否已在某谱系中标为「L2 未覆盖」——sr-impl 边界条件 1 已登记，未检索到独立谱系。

### 6. 影响声明

- **代码改动**：零（本裁定是数值冻结 + 异质审查复核，未改代码；sr-impl commit 67d20d292f 是上游实装，本裁定消费其可达性）。
- **文档改动**：新建 `.chanlun/review-results/prereg-rev5-eta-adv-20260705.md`（本文件）+ `/tmp/codex-eta-adv-prompt/output-20260705`（完整 Codex 交互记录）。
- **对 g3 跑数的影响**：Follow 臂从 codex-ruling-eta 的「跳过/alias(neutral)」恢复为独立 OOS 臂，用 eta_adv=[0.70,0.70,0.70,0.50,0.50,0.50]。三套并行 OOS。
- **对 codex-ruling-eta 的影响**：Q1「eta_adv 不值得冻结」结论**被本轮翻转**（翻转条件 Q4(b) 满足）；eta_same 冻结 [0.70,0.50,0.70,0.70,0.70,0.70] 不变；「三套塌缩两套」结论订正为「三套独立」。
- **对 prereg-rev4 §10.2 的影响**：「形式采纳但实质落空」订正为「真正生效」（eta_adv[ℓ≥3]=0.50 落地 §10.2 冻结意图）。
- **诚实声明**：eta_adv 数值表是「合理冻结起点」非「唯一正确答案」（L2 extra-缠论，616号须外部固定）；ℓ≥3=0.50 vs 0.30 是证据强度选择（0.30 须等 SameReverse 专属 OOS + 样本充足）；ℓ0-2=0.70 vs 0.50 是焦点归属选择（L1 焦点归 eta_same，eta_adv 低级别温和）；本裁定推翻蜂群初步裁定，依据 Codex 异质否定 + 工位独立核实（§10.2 + highlevel-mu 双锚亲读确认），eta_adv 数值表待 main/编排者确认采纳后落 g3 config。

---

**prereg-rev5 冻结结束**。eta_adv(ℓ=0..5)=[0.70,0.70,0.70,0.50,0.50,0.50]（ℓ≥3 强收缩组分界），与 eta_same=[0.70,0.50,0.70,0.70,0.70,0.70]（L1 焦点）非对称分工，共同形式化 §6 page4 双重信息。Codex 异质否定成立，推翻蜂群初步裁定（违背 §10.2 已冻结意图 + L1 归属错误）。下游 g3 跑数依赖本文件 commit 冻结哈希 + sr-impl commit 67d20d292f。
