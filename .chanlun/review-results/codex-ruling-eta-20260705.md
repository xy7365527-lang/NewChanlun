# codex-ruling-eta｜q_Θ v1 η 数值表（eta_adv / eta_same）codex 裁定 + 工位复核

**工位**：swarm/ws-etaruling（codex-reviewer，代码层异质否定源）｜ topo_address: swarm/ws-etaruling ｜ 基因 073a/274号
**日期**：2026-07-05 ｜ 分支：gap3-rework-codex9-fix ｜ parent_callback: main
**模式**：codex decide + review 混合（η = extra-缠论风险政策参数 616号 + 生效路径受代码角色分类硬约束）
**命令**：`codex exec --skip-git-repo-check --sandbox read-only -c model_reasoning_effort="high"`
**prompt 文件**：`/tmp/codex-eta-ruling-prompt-20260705.md`（完整交互记录，含实装代码原文）
**output 文件**：`/tmp/codex-eta-ruling-output-20260705.txt`（1963 行，tokens used 60099）
**认识论等级**（231号）：η 数值 = L2 extra-缠论；分级结构（按 ℓ 分行）= L0；可达性论断 = L1（代码静态核实）。

---

## 〇、工位复核结论（否定成立，HIGH 严重性，预注册设计层非代码 bug）

**codex 否定全部成立，且是未被覆盖的新否定。** 工位独立核实（coverage.rs:1187-1194 + 1337-1355 + 905-918）确认可达性论断真实——非 codex 误读上下文。

| 复核项 | 判定 |
|---|---|
| 可达性论断真实性（CASE 3 sign=−1 槽不可达） | ✅ 真实（L1 代码静态核实） |
| 是否已被其他机制覆盖 | ❌ 未覆盖——实装文档 qthetav1-impl §〇b/§一.1 记录了"sign=−1 不可达"为代码性质，但**无人推到操作后果**（Follow 套==neutral / eta_adv 空操作 / 三套塌缩两套） |
| 严重性 | **HIGH**——预注册设计层（g2 实装 PASS 不变；g3 跑数计划需调整：Follow 臂浪费算力） |
| ChatGPT 方向一致性 | codex 否定 ChatGPT"高级别逆上级强烈收缩 eta_adv"（槽不可达 + 语义错配）；与 prereg §10.2 否决"短差放大"一致 |

**工位增量洞察**（超出 codex 裁定的结构性问题）：prereg-rev4 §6 的 **Adversary 套实证动机**（§6 page4「L1 主要超 beta 来源」→ 逆上级保权假设）在当前 Vertical 三分类下**根本不可检验**——逆上级腿全部 ShortDiff 豁免，Adversary 套实际收缩的是顺上级 FollowParent 腿（语义反转）。要真正检验"逆上级 alpha"假设，须先引入 SameReverse 第 4 类 Vertical。这是比"eta_adv 不值得冻结"更深一层。

---

## 一、codex 裁定全文（总裁定 + Q1-Q4 + 元裁定）

### 总裁定

当前三套不是三套：`neutral` 与 `follow` 在角色路径上 bit-exact 塌缩，实际只有两类：`neutral/v0` 与 `adversary(eta_same)`。把 `eta_adv` 当成可检验参数是**伪问题**。

代码锚点：coverage.rs:1337 已明写 ShortDiff 先返 1.0；CASE 3 仅 FollowParent 可达，sign 恒 +1。

### Q1：eta_adv 可达性裁定

**裁定选 (b) 不值得冻结 / 跳过 Follow 套 OOS。**

理由：当前 Vertical 只有 Ambient/FollowParent/ShortDiff。所有 δ=−σ_parent 的逆上级腿都被归为 ShortDiff，在 dir_weight CASE 1 直接豁免。CASE 3 的 sign=−1 槽不可达，因此：
- `ThetaDirPreset::Follow { eta_adv }` 永远走 sign=+1 → 1.0
- `eta_adv(ℓ)` 完全不生效
- Follow 套 OOS 必然等价于 neutral/v0
- 冻结 eta_adv 不是 135号纪律，而是**空操作**

执行建议：不要跑 Follow 套 OOS；若实验框架必须保留三臂，Follow 臂应标注为 `alias(neutral)`，不计作独立 OOS。

### Q2：eta_same 数值表（唯一生效槽）

**裁定冻结 `eta_same(ℓ=0..5)`：**

```
eta_same = [0.70, 0.50, 0.70, 0.70, 0.70, 0.70]
```

性质标注：L2 extra-缠论风险政策参数；只有"按 ℓ 分级"结构可归 L0/231，有效数值非 L0 可导。

语义裁定：**这不是原 ChatGPT 所说的"高级别逆上级接飞刀收缩"**。当前代码下它实际收缩的是**顺上级 FollowParent 同向子腿**。该套应重命名理解为：**同向 beta stacking 降权策略**，而不是逆上级风险收缩策略。

与 §6 page4 一致性：**部分一致，但语义已反转**。若"L1 主要超 beta 来源"指向同向 beta 暴露，收缩 L1 FollowParent 合理；若原假设是"逆上级保权/逆上级识别 alpha"，当前代码已无法检验该假设（逆上级全 ShortDiff 豁免）。

推荐**非单调**（不随 ℓ 机械下降）：已知风险焦点是 L1，不是"级别越高越危险"L0 定理。单调表会把 ChatGPT 高级别逆上级直觉错误投射到当前同向槽。

### Q3：逐级理由

| ℓ | η_same | 理由 |
|---|---|---|
| 0 | 0.70 | 基础层同向腿仍可能带 beta 暴露，但不应强杀；0.70 温和风险预算，不把 v1 变过激反趋势器 |
| 1 | **0.50** | L1 被明确标为主要超 beta 来源 ⟹ 最强收缩。不用 0.30（语义已反转，过强收缩误杀顺上级结构性腿） |
| 2 | 0.70 | 中间层保守降权。无 L0 依据证明比 L1 更危险，避免逐级异值过拟合 |
| 3 | 0.70 | 高级别样本稀疏，不能采纳 ChatGPT"高级别逆上级强收缩"直觉（当前槽非逆上级） |
| 4 | 0.70 | 同上，统一 0.70 是风险预算，不是 alpha 断言 |
| 5 | 0.70 | 同上 |

建议使用标准网格 `{0.5, 0.7, 1.0}`，不使用逐级任意异值；0.3 暂不进入冻结表。

### Q4：翻转条件

**(a) 数值翻转**（只允许下一轮预注册改，不能用本轮 OOS 事后调参，135号）：
- L1 FollowParent OOS residual alpha 显著为负、beta/回撤贡献显著偏高、n_eff 足够 ⟹ L1 可从 0.50 下调到 0.30
- L1 FollowParent 有稳定正 alpha，降权后 Sharpe/收益显著变差 ⟹ L1 上调到 0.70 或 1.00
- 全部 FollowParent 降权只降低收益、不改善 beta/回撤/尾损/turnover-adjusted ⟹ Adversary 套应废弃
- 高级别 ℓ3-5 出现足够样本且同向 beta stacking 明确恶化 ⟹ 才考虑 ℓ3-5 从 0.70 降到 0.50

**(b) Q1 可达性翻转的代码条件**：
- 引入第 4 类 Vertical，如 `SameReverseNonShortDiff` / `ReverseParent`
- 或把 ShortDiff 缩窄为"真实对冲 overlay 腿"，让部分 δ=−σ_parent 的非对冲逆上级腿进入 CASE 3
- 或显式传入 σ_higher，不再用 role.v 把所有逆上级压成 ShortDiff

一旦 sign=−1 经生产路径可达，eta_adv 就值得冻结，且必须在新 OOS 前冻结。

**(c) 若当前代码下 Follow 套 OOS ≠ v0**：裁定为**实装 bug 或测试口径污染**，不是策略发现。优先查：ShortDiff 是否绕过 CASE 1 / Follow preset 是否影响 dir_weight 之外路径 / OOS runner seed/config/slippage/data window 是否一致 / 是否有未记录新角色路径让 sign=−1 可达。

### 元裁定

本 task 框架确实有问题：在当前三分类下，"三套预注册"已塌缩为两套。正确设计是 `neutral` vs `adversary_same_beta_shrink`；`follow_eta_adv` 只能作为未来 SameReverse 路径的**表结构占位**，不能作为当前 OOS 独立臂。

---

## 二、工位复核（判定否定成立性 + 严重性）

### 2.1 可达性论断真实性（L1 代码静态核实，亲自重核）

- coverage.rs:1187-1194 `vertical_relation`：`sigma_parent≠0 ∧ dir_sign(delta)≠sigma_parent ⟹ ShortDiff`——**任何逆上级腿落 ShortDiff**。
- coverage.rs:1337-1341 `dir_weight` CASE 1：`role.v==ShortDiff ⟹ return 1.0`——**优先于一切预设**。
- coverage.rs:1343-1354：CASE 3 仅 FollowParent 可达；FollowParent 下 `sigma_higher=dir_sign(delta)` ⟹ `sign=dir_sign(delta)·dir_sign(delta)=+1`（dir_sign∈{±1}）。
- **论断真实**：sign=−1 槽经 role 路径不可达。非 codex 误读。

### 2.2 否定是否已被覆盖

实装验收文档 qthetav1-impl §〇b/§一.1 + 代码注释 coverage.rs:1331-1333 已记录"sign=−1 槽经 role 路径不可达，是代码角色分类性质非 bug"。但**所有既有记录都止步于"性质声明"**，无人推到操作后果：
- Follow 套 bit-exact 退化 == neutral（eta_adv 空操作）
- 三套预注册塌缩为两套
- Adversary 套语义反转（收缩顺上级，非 prereg §6 设想的逆上级保权）

codex 把已记录性质推到了**操作后果 + 预注册设计层否定**——**未被覆盖的新否定**。

### 2.3 严重性判定

**HIGH（预注册设计层，非代码 bug）**：
- g2 实装验收 PASS 不变（实装严格遵循 prereg-rev4，ShortDiff 豁免正确）。
- g3 跑数计划需调整：Follow 臂 OOS == neutral 臂 == v0 基线（浪费一臂算力），eta_adv 冻结是空操作。
- Adversary 套是唯一能产生非 v0 信号的臂，但检验的假设与 prereg §6 设想不符（语义反转）。

### 2.4 ChatGPT 方向一致性

- ChatGPT"高级级别逆上级强烈收缩 η_adv≪1"：codex 否定（槽不可达 + 语义错配）——**与 prereg §10.2 形式上"采纳"但实质上落空一致**（§10.2 采纳落入 Θ[ℓ≥3][−1] 槽，但该槽不可达）。
- ChatGPT"短差适度放大 w≥1"：prereg §10.2 已否决（§7.5 违反），codex 一致。
- codex 增量：ChatGPT"高级别"直觉不应投射到 eta_same（顺上级槽），因语义不同——**合理**。

---

## 三、结果包六要素（偏完整版——涉及预注册设计层）

### 1. 结论

- **eta_adv(ℓ)**：不值得冻结（Q1 选 b）。Follow 套在当前 Vertical 三分类下 bit-exact 退化为 neutral，eta_adv 是空操作。建议 g3 跳过 Follow 臂 OOS，或标注为 `alias(neutral)` 不计独立 OOS。
- **eta_same(ℓ=0..5)**：冻结为 `[0.70, 0.50, 0.70, 0.70, 0.70, 0.70]`（非单调，L1=0.50 最强收缩）。这是当前唯一实际生效的收缩系数，语义为"顺上级同向 beta stacking 降权"（非 ChatGPT/adversary 原设想的逆上级收缩）。
- **三套塌缩为两套**：neutral == follow（bit-exact）；实际独立臂只有 neutral/v0 与 adversary(eta_same)。

### 2. 定义依据

- coverage.rs:1187-1194（vertical_relation 三分类判定）+ 1337-1355（dir_weight 三 CASE）+ 905-918（Vertical 枚举仅三值）——L1 静态核实 sign=−1 槽不可达。
- 买卖点.pdf §7.5 page6 `s_g=s_α`（ShortDiff 豁免的 formal-chain 锚，prereg §5.1 定理1）。
- 完整的策略.pdf page4「L1 是主要超 beta 来源」（eta_same ℓ1=0.50 最强收缩的实证动机）。
- 616号：η = extra-缠论风险政策参数，非 L0 可导，须外部固定。

### 3. 边界条件（结论翻转）

- **eta_adv 翻转为"值得冻结"**：仅当引入第 4 类 Vertical（SameReverseNonShortDiff）/ 缩窄 ShortDiff 定义 / 显式传 σ_higher，使 sign=−1 经生产路径可达（codex Q4(b)）。
- **eta_same 数值翻转**：下一轮预注册可改（135号），触发条件见 codex Q4(a)（L1 residual alpha 显著负 ⟹ 0.30；L1 稳定正 alpha ⟹ 上调；全降权无改善 ⟹ 废弃 Adversary 套）。
- **Follow 套 OOS ≠ v0**：裁定为实装 bug（ShortDiff 绕过 CASE 1 / Follow preset 污染其他路径 / runner 口径不一致），非策略发现。
- **不翻转**：ShortDiff CASE 1 豁免（§7.5）/ σ_higher 经 q_Θ 通道（§11 page7）/ 禁一刀切（§6 page4）——三者 formal-chain 已结算。

### 4. 下游推论

- **对 g3（跑数）**：跳过 Follow 臂 OOS（或标 alias(neutral)）；只跑 neutral（v0 bit-exact 自检）+ adversary(eta_same=[0.70,0.50,0.70,0.70,0.70,0.70]) 两臂。Follow 臂 eta_adv 不冻结（空操作）。
- **对 prereg-rev4 §6 Adversary 套假设**：当前代码下"逆上级保权/逆上级 alpha"假设**不可检验**（逆上级全 ShortDiff 豁免）。Adversary 套实际检验的是"顺上级同向 beta stacking 降权"——若要检验原 §6 假设，须先扩 Vertical 枚举（prereg-rev5+）。
- **对 ChatGPT 方向**：形式"采纳"（prereg §10.2）但实质落空——"高级别逆上级强烈收缩"在当前实装下无法实现，除非引入 SameReverse。

### 5. 谱系引用

- **无新谱系**（本裁定是 codex 异质裁决 + 工位复核，非新概念发现）。
- 相关已结算：090（严格性/声明膨胀）、231（有效域<定义域——η 数值 L2 非L0）、135（冻结先于跑数）、616（Θ 风险参数 extra-缠论）、§7.5 s_g=s_α（ShortDiff 豁免）、§6 page4（L1 超 beta 来源）。

### 6. 影响声明

- **代码改动**：零（本裁定是数值冻结 + 预注册设计复核，未改代码）。g2 实装 PASS 不变。
- **文档改动**：本文件（codex-ruling-eta-20260705.md）+ /tmp prompt/output（完整交互记录）。
- **对 g3 跑数的影响**：Follow 臂从三臂 OOS 中移除（或标 alias），实际跑 neutral + adversary 两臂；eta_same 冻结为 [0.70,0.50,0.70,0.70,0.70,0.70]。
- **对 prereg-rev4 的影响**：§6 Adversary 套实证动机（逆上级保权）在当前代码下不可检验的诚实声明；§10.2 ChatGPT"高级别逆上级强收缩"形式采纳但实质落空的记录。
- **诚实声明**：eta_same 数值表是"合理的冻结起点"非"唯一正确答案"（L2 extra-缠论，616号须外部固定）；codex 选 [0.70,0.50,...] 依据 §6 page4 + 标准网格 + 避免过拟合，ℓ1=0.50 vs 0.30 是范围选择，须 main/编排者确认采纳。

---

**裁定结束**。codex（异质代码层）+ 工位复核一致：eta_adv 不值得冻结（Follow 套==neutral 塌缩），eta_same 冻结 [0.70,0.50,0.70,0.70,0.70,0.70]（唯一生效槽，语义=顺上级同向 beta 降权）。三套预注册在当前 Vertical 三分类下塌缩为两套。 eta_same 数值待 main 确认后落 g3 跑数 config。
