# Codex 裁决：β^div 背驰强度进桶键设计稿审计（#104 实装前置）

- 工位：codex-challenger（task #106），审计对象 `.chanlun/review-results/beta-bucket-design-20260703.md`
- 完整交互记录：`.chanlun/review-results/codex-review-20260703-020518-f92a.md`（自动持久化，含完整 prompt/response）
- 日期：2026-07-03

## 总体裁定：conditional

方向正确——`ForceState`（4值支配序）比连续标量更忠实原文，但设计稿不能直接冻结实装：定理2 措辞、δ-独立性证明力度、full-z 接入完整性、路由缺口修复方案都有硬修项。

## 逐条裁定（team-lead 五个审查焦点）

### 1. 定理2 引用的推理链——**fail**

定理2（p7）证明的是"不存在无额外公理的唯一全序/唯一二值判定"，**不是**否定单 proxy、词典序、加权和的工程合法性——它们是需要预注册的 Θ 选择，不是"非法"。设计稿 §1 用"非法"（illegal）表述，同时又把 Θ_SCORE 列为合法预注册候选，内部自相矛盾。

**编排者复核**：直接对照 PDF 原文（p7）确认——证明结论是"任意 AND、OR、词典序、加权和，都是额外选择 Θ，不是原文唯一推出"，未使用"非法"或等价措辞。Codex 裁定成立，非误读。

**修复**：把"非法"改为"不能作为原文强制推出的 canonical β 定义；只能作为 Θ_SCORE/Θ_LEX 等预注册工程选择"。

### 2. ForceState 四值支配序的定义忠实度——**conditional**

`ForceFeatures` 现只有 4 proxy，原文 𝒜_ℓ（p6）还含 TV（全变差）和递归次级别力度。诚实标注"有效域<定义域"可接受，但不能无限定地宣称这是"原文忠实 ForceState"。补 TV/递归力度后，当前 `Dominated`/`Dominates`/`Tie` 可能变成 `Incomparable`（新增口径冲突）；但反向不会发生（已有冲突不会因加维而消失，`Incomparable` 不会变成支配态）。

`Incomparable` 单列一态进 μ 分层（不并入 `Dominated`）合理，但**不得**作为背驰确认使用。

**修复**：命名改为 `ForceStateA4` / `ProxyForceState`，显式标注"4-proxy sufficient state，非完整 𝒜_ℓ"；或先补 `segment_total_variation` 进 `ForceFeatures` 再定名 `ForceState`。

### 3. δ-共线双层检查——**conditional**

- 构造级：四 proxy 全 `.abs()` 只证明"proxy 不直接编码方向符号"，**不能**证明"支配序输出 ⊥ δ"——数据生成过程仍可能让涨跌两个方向的支配态分布产生统计相关（非逻辑必然但经验可能）。§3.1 措辞应改为"mirror-invariant / not sign-coded by construction"，而非"β ⊥ δ by construction"的强声明。
- 经验级：`mix > 0` 判据太弱（1 vs 999 仍通过）；小样本下卡方检验功效与适用条件不足。

**修复**：mix 判据加最小双侧样本阈值；补 Cramér's V 或 exact/permutation test 作为卡方的补充；低样本桶合并或直接禁用置换。

### 4. 第 8 维接入方案——**conditional**

`Option<ForceState>` 与既有 `horizontal: Option<Horizontal>` 模式一致，`None` 语义在类型上安全（不会被误判为 `Incomparable`）。但"现有全部路径零改动"的声明**不完全成立**：`perm_test.rs:206/215` 的 `stratified_delta_perm_p_fullz` 已把 δ-free base 显式列为 `(level, i_class, parent_dir, position, horizontal)` 五元组——这是 full-z 置换路径，新增 `force_state` 若不进这个 base，不同背驰支配态会被并入同一置换层，稀释检验力度。

**编排者复核**：`grep` 坐实该函数签名与 base key 定义，确认此路径确实会受影响，design doc 遗漏了这个具体函数。

**修复**：`stratified_delta_perm_p_fullz` 的 base key 加 `c.force_state`；报告格式加 force 维；加 fill-rate 断言防止生产路径全 `None` 静默通过测试。

### 5. 路由缺口方案 (b)（collect_signals 就地重算）——**fail（致命）**

**编排者复核（坐实）**：
- `rust/src/theta_v0/classifier/signal.rs:543` 生产路径 `extract_signals` 明确传空 `dif`/`closes_tick` ⟹ `force` 恒 `None`（注释自陈"生产/测试 BspPoint 入口不消费 force"）。
- `rust/src/theta_v0/backtest/econ_positive.rs:250` 的 `collect_signals` 完全走 `Candidate → z_of_candidate(c)`（`selector.rs:144`），没有任何 `ForceProxies` 数据通路。

设计稿提议的"collect_signals 就地重算 ForceState"会构成**第二套力度比较逻辑**——`divergence.rs` 已有 `ForceFeatures`/`weak_theta` 一套比较原语，若 `collect_signals` 独立重新实现支配序比较（而非调用同一原语），是 no-patch-mentality 明确禁止的重复实装；且诊断/dx 手写循环也需同步维护，两套逻辑漂移风险高。

**修复**：在 `divergence.rs` 新增 `ForceState` 枚举 + `ForceProxies::force_state()` 单一比较方法（`weak_theta` 是 Θ 布尔判据，不是 DOM 四态，二者不可互相替代但应共享同一份 proxy 数据结构）。然后把 `Option<ForceProxies>`（或更小的 `Option<ForceState>`）透传到 `Candidate`/`RawSignal`，用 `z_of_candidate_with_force(c, fs)` 构 z——不在 `collect_signals` 另写比较逻辑。

## 实装前必须修（团队共识）

1. 定理2 措辞改为"非 canonical，需 Θ 预注册"，删除"非法"表述。
2. `ForceState` 命名或补 TV——二选一：改名 `ForceStateA4` 诚实标注四proxy近似，或先补 `segment_total_variation`。
3. `stratified_delta_perm_p_fullz` 的 base key、报告格式、fill-rate 测试须与 `force_state` 第8维同步改动。
4. `ForceProxies::force_state()` 是唯一支配序比较原语——路由方案改为透传数据到 `Candidate`，不在 `collect_signals` 重算比较逻辑。

## 边界条件（本裁定何时翻转）

- 若原设计稿作者能证明"支配序输出的经验独立性"已有更强证据（而非仅构造级 `.abs()` 论证），则第3项裁定可从 conditional 升级为 pass。
- 若 `force_state` 确定不进入任何置换/OOS 管线（只做报告展示、不做统计推断），第4项的 fullz base 修复要求可豁免。
- 若路由方案改用 (a)（Candidate 携带 ForceProxies）而非 (b)，第5项的"第二套力度逻辑"风险自动消除，但需承担"改动面大"的实装成本（设计稿已自陈）。

## 谱系引用

- `project_oddeven_mu_identity`：i_class×δ 共线的先例，本次 δ-共线检查设计据此展开但证明力度不足。
- `formalization-validity-domain`（231号）：ForceState=4-proxy 近似的有效域<定义域标注方式，本裁定要求更明确的命名/补维二选一。
- codex #81：H 轴 accept in Z / conditional in selection 的裁决模式，ForceState 沿用同构降维策略（本次未被否定）。
- no-patch-mentality：路由方案(b)"就地重算"被裁定为重复实装的直接依据。

## 影响声明

本裁定不改任何代码，只审查设计稿。受影响对象：`beta-bucket-design-20260703.md` 需按上述4项修复后才可进入实装排期；`divergence.rs`（需加 `ForceState`/`force_state()` 方法）、`interp.rs`/`selector.rs`（Candidate 携带力度数据的路由改动）、`perm_test.rs`（fullz base 扩维）是下游实装工位的直接改动对象。
