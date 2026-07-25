# Gap-3 signal 定案裁定材料（Pass ∧ n_eff 不足并存的三态裁决）

- 日期：2026-07-19
- 工位：worktree `/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`），文档工位——本文件为唯一产出，零代码改动、零 git mutation、主仓只读。
- 裁定问题：Gap-1 回填后，signal 终判「35 桶 / verdict=Pass / 双门各一桶 LCB>0 Validated」与「n_eff 不充分（主裁决桶 252 < 功效门 271 下沿；μ_R 桶 8 远低）」并存。裁「Pass 但 n_eff 不足 ⟹ INCONCLUSIVE」还是「Pass 就是 Pass」。
- 上游证据：
  - `chanlun/review-results/g4-layer1-signal-recon-20260719.md`（新旧口径对照 §3、张力归因 §4、回填规格 §5.2）
  - `chanlun/review-results/gap1-signal-backfill-20260719.md`（回填后文本 §1-§2、残留张力 §5）
  - 实测锚：`/tmp/wverify_full_g2.out:301-302`（关②后 `wverify_full` 重跑终判两行）
  - 门控实现：`rust/src/theta_v0/backtest/decontam.rs:111-154`；`rust/src/theta_v0/backtest/wverify_run.rs:300-349, 502-526`
  - 判据定义：`TARGET_STRATEGY.md` §1.1/§1.4；`TARGET_STRATEGY_MAXFULL.md` M4(:113-126) / M8(:161-168) / 措辞§5.3(:251) / §5.6(:255)
- 090 纪律：本材料只供裁定，不代裁；照实否定合格；声明=能力。

---

## 0. 先清一个事实层误读（裁定的实证地基）

「n_eff < 271 ⟹ 不足」的读法建立在一个误读上，必须先把事实层与裁定层分开：

1. **功效门是逐桶 CV 条件门槛，不是统一下限。** 预注册门控为 `powered ⟺ n_eff ≥ (z_α·CV)²`（`decontam.rs:111-118`，z_α=1.645 冻结于 `decontam.rs:8`）。旧文本「功效门 271~1083」对应 CV∈[10,20] 的区间参考值——`decontam.rs:163` 测试注释自证「CV=10 ⟹ 门槛≈270.6」。CV 是逐桶实测量（`wverify_run.rs:333`），不是常数。
2. **n_eff 门已被强制执行，不是缺失环节。** `classify_bucket`（`decontam.rs:123-142`）判定顺序「先功效门，再三态」（`decontam.rs:121`），Validated 的必要合取项为 `powered ∧ μ̂>0 ∧ perm_p<α ∧ LCB>0`（`decontam.rs:133-135`）。两桶实测判 Validated ⟹ **两桶 powered 均实测过门**，否则代码路径只会落 Inconclusive。反推：主裁决桶 252 ≥ (1.645·CV)² ⟹ 该桶 CV ≤ √252/1.645 ≈ **9.65**（< 10，故门槛 < 271）；μ_R 桶 8 ≥ (1.645·CV)² ⟹ CV ≤ √8/1.645 ≈ **1.72**。「252 < 271」只说明该桶在「CV=10 假设」下不过门，而实测 CV 更优。
3. **μ_R 桶过门余量极薄（照实标注，这是真实的残留张力）。** n_eff=8.0 对门槛 (1.645·CV)²（CV ≤ 1.72 ⟹ 门槛 ≤ 8.0）是**临界过门**：CV 点估计的微小扰动即可翻转 powered。且主路径 LCB = mean − z_α·std/√n 用的是 **raw n**（`wverify_run.rs:331-332`），自相关折扣只经 powered 门的 n_eff（Geyer IPS，`wverify_run.rs:334`）进入，不进 LCB 的标准误——μ_R 桶 raw_n 大（池化后）而 n_eff=8，LCB>0 主要由 raw n 撑起。n_eff=8 的 Validated 在稳健性意义上薄，这一经验事实不因门控过门而消失（gap1 回填 §1 已照实保留「n_eff 不充分」标注）。
4. **§5.6 的 INCONCLUSIVE 限定语是「高级别桶」。** 措辞§5.6（`TARGET_STRATEGY_MAXFULL.md:255`）：「高级别桶 n_eff≪n_min（功效门 n≥271~1083）时只准 INCONCLUSIVE」。本次两 Validated 桶均为 **L0**（低级别统计桶）——正是 M4 架构「高级别定方向、低级别统计」（`TARGET_STRATEGY_MAXFULL.md:117-121`）中承担统计的那一级；§5.6 该条款的适用域（L1-L4 高级别桶，同节 M4 注记「L1-L4 合计约 31 个/8.8 年，功效远远不足」）不覆盖本情形。

裁定问题因此精确化为：**预注册门控输出 Pass（且 powered 已实测过门），但 μ_R 桶 powered 余量临界——是否允许以 n_eff 关切事后翻转 verdict。**

---

## 1. 三个候选裁定

### 候选 A：Pass 但 n_eff 不足 ⟹ INCONCLUSIVE（事后改判）

**主张**：双门虽各一桶 Validated，但主裁决桶 n_eff=252 低于 271 下沿、μ_R 桶 n_eff=8 远低于任何合理功效门；证据承载力不足 ⟹ signal 三态改判 INCONCLUSIVE。

**教义依据**：
- 090「声明=能力」：在 n_eff=8 的 μ_R 桶上宣称「confirmed alpha」超出证据承载力，INCONCLUSIVE 是更诚实的三态。
- 措辞§5.6 精神（INCONCLUSIVE≠无 alpha，欠功效时保守落地）：宁可悬置不可过宣称。

**措辞纪律依据**：§5.6「n_eff≪n_min 时只准 INCONCLUSIVE」的保守主义延伸；INCONCLUSIVE 是合法三态而非失败（`TARGET_STRATEGY_MAXFULL.md:124`）。

**实证依据**：
- μ_R 桶 n_eff=8.0、perm_p=0.010（`/tmp/wverify_full_g2.out:302`）；powered 临界过门（§0-3）；LCB 的 SE 用 raw n 不含自相关折扣（`wverify_run.rs:331-332`）。
- 主裁决桶 252 仍低于 CV=10 参考门槛 270.6（`/tmp/wverify_full_g2.out:301` 对照 `decontam.rs:163`）。

**致命伤（照实）**：
1. **事后改判违反预注册纪律。** 判定顺序是冻结表（`decontam.rs:121`「判定顺序严格照冻结表——先功效门，再三态」）；M4 验收明记「三态输出，**且无离线补主裁决**」（`TARGET_STRATEGY_MAXFULL.md:126`）；主裁决路径自带「不事后挑桶」纪律（`wverify_run.rs:796`）。以 n_eff 关切在跑批之后叠加新判据 = post-hoc criterion override——这是比「n_eff 不足」更高阶的协议违例：它使门控口径变成可事后协商的变量，预注册的全部防伪意义失效。
2. **§5.6 条款适用域不覆盖**（§0-4：限定「高级别桶」，本情形是 L0 统计桶）。
3. **事实层前提有误读**（§0-1/§0-2：271 非统一下限，powered 已实测过门）。
4. 连带成本：Gap-1 回填（`wverify_run.rs:1173-1177 / :1237-1246 / :1348-1350`）需整体回滚重裁，再开 rust/src 闸门；且「Pass→INCONCLUSIVE」的方向与实测门控输出相悖，回填文本将声明≠实测（090 违例换位重演）。

### 候选 B：Pass 就是 Pass；n_eff 门槛充分性另行裁定（门控照收 + 协议问题分流）

**主张**：signal 三态照预注册门控输出 = **Pass**（`global_verdict`：任一 Validated ⟹ Pass，`decontam.rs:144-154`）；n_eff 关切的真实内核（μ_R 桶临界过门、LCB 标准误未含自相关折扣、CV 点估计在小 n_eff 下的惩罚缺失）是**门控协议本身的充分性问题**，另开裁定修订 prereg（前向生效），不回溯翻转本次 verdict。

**教义依据**：
- 预注册是 v3 体系防伪的根基：门控口径先于数据冻结（`decontam.rs:8`「§4 冻结」），裁决只能是门控的机械执行。照单全收 = 声明与门控能力一致（090）。
- 「照实否定合格」的对偶：**照实肯定是义务**——门控过门而人为压成 INCONCLUSIVE，同样是声明≠实测。
- 161/090 谱系：否定性结果合法（`wverify_run.rs:1191-1194` 认识论 L2 段），其对称面即肯定性结果在门控过门时同样合法。

**措辞纪律依据**：
- §5.6 纪律不变：Pass 仅述 signal 双门实测门控结果，不主张「无限制 confirmed alpha」——回填文本（`wverify_run.rs:1243-1245`）已带「n_eff 不充分照实标注 + Validated 由 perm_p+LCB 联合判定、n_eff 不单独否决」的限定语，措辞=门控能力，无过宣称。
- §5.3 不动：signal 结果不外推 max-full（`TARGET_STRATEGY_MAXFULL.md:251`）。

**实证依据**：
- 门控输出两行终判（`/tmp/wverify_full_g2.out:301-302`）；powered 实测过门（§0-2）；双门各一桶 Validated（δ-free 主裁决 L0 bsp3 σ+1 f=None；μ_R co-primary L0 bsp1 σ+0 f=Inc）。
- n_eff 关切被显式保留而非掩埋：gap1 回填 §1「照实保留项」四层（n_eff 标注 / §5.6 不变 / §5.3 保留 / perm_p 措辞限定为门控判据）。
- μ_R 桶同族口径可复算：co-primary 喂**同一** `deltafree_verdict`（`wverify_run.rs:523`），无第二查法。

**代价（照实）**：在另案裁定落地前，须维持「Pass（预注册门控口径）+ n_eff 不充分照实标注」的双层表述；若另案修订 powered 口径（如 LCB 的 SE 改用 n_eff、CV 小样本惩罚），本次两桶的 Validated 可能在新口径下重估——修订须声明前向生效，否则退化为候选 A 的事后改判。

### 候选 C：Pass 且 n_eff 门槛已由门控联合判定覆盖（张力不存在）

**主张**：「n_eff 不足」是误读衍生的伪张力——powered 是 Validated 的必要合取项且已实测过门（§0-2），「271」是 CV=10 参考值非统一下限（§0-1）⟹ n_eff 门槛在本次裁决中**已被执行**，不存在「并存」需要调和；signal = Pass，无保留。

**教义依据**：
- 与 B 同源的预注册纪律，且更彻底：连「n_eff 不足」作为悬置关切都不承认其裁定层地位，只承认其事实层存在（已在文本中标注）。
- 090：裁定应陈述机制真相——门控是什么、执行了什么，而不是接受一个建立在意象（「8 太小」）上的否决。

**措辞纪律依据**：§5.6 的 INCONCLUSIVE-only 条款适用域不覆盖 L0 桶（§0-4）；Pass 措辞仍须带 §5.3 边界与 perm_p 门控限定（同 B）。

**实证依据**：
- `decontam.rs:133-135`（powered 为 Validated 必要合取项）+ `decontam.rs:121`（先功效门后三态）+ `decontam.rs:163`（CV=10 参考门槛 270.6）+ 两桶 Validated 实测（`/tmp/wverify_full_g2.out:301-302`）⟹ powered 过门是代码级事实，非推断。
- 反推界：主裁决桶 CV ≤ 9.65、μ_R 桶 CV ≤ 1.72（§0-2）。

**两处必须精确化/照实的风险**：
1. **原表述「由 perm_p+LCB 联合判定覆盖」机制描述不准确。** n_eff 不经 perm_p 或 LCB 进入判定——它经独立的 powered 合取项进入（`decontam.rs:133`）；gap1 回填文本「Validated 由 perm_p+LCB 联合判定，n_eff 不单独否决」是**输出层概括**（n_eff 不单独否决为真），若裁定采 C 须改写为「n_eff 门槛（powered 合取项）已强制执行并实测过门」，否则裁定文本本身构成机制误述（090）。
2. **关闭审查通道的风险。** μ_R 桶 powered 临界过门 + LCB 的 SE 用 raw n（§0-3）是真实的协议级疑点；C 裁定「无保留」会被下游转引简化为「n_eff 门槛不存在/已豁免」，既失实也封死后续修订 powered 口径的合法入口。C 若被采纳，须附带「本裁定不预断 powered 口径的前向修订」条款。

---

## 2. 对端到端（max-full）传导的影响

**核心事实：max-full 三态对本次 signal 裁定不变。**

- 措辞§5.3（`TARGET_STRATEGY_MAXFULL.md:251`）：signal-full 结果**不得**当成 max-full 结论；`TARGET_STRATEGY.md` §1.4 边界互斥（`:54`，严格证明见 `docs/formal-chain/有效域定理-20260704.md` 定理1）——外推禁令是双向的，正结论同样不得外推。
- M8 层4 判据独立结算：`LCB_OOS(R_Πmax-full)>0` 才叫 confirmed alpha（`TARGET_STRATEGY_MAXFULL.md:164-168`）；当前 m8 文本层4「未过 ⟹ INCONCLUSIVE」（`wverify_run.rs:1354-1355`）。
- 因此三个候选下端到端总叙事分别为：
  - **A**：层1 INCONCLUSIVE + 层4 INCONCLUSIVE（「signal 未确认 + max-full 未确认」）。
  - **B / C**：层1 Pass + 层4 INCONCLUSIVE（「signal 已确认（门控口径）+ max-full 未确认」）——**层4 不因层1 Pass 升级**；任务提示中「signal INCONCLUSIVE → max-full INCONCLUSIVE」的传导链只在叙事层成立，max-full 三态本身永远由层4 自有判据产出。
- 文本连带：A 要求回滚 Gap-1 回填三处（`wverify_run.rs:1173-1177 / :1237-1246 / :1348-1350`，rust/src 闸门重开）；B/C 保留回填，其残留张力（`:1192`、`:1226`、`:1352` 三处「signal 无 alpha」旧措辞，gap1 回填 §5 已照实上报）需另开授权同步——该同步义务在 B/C 下等同，不构成分候选差异。

## 3. 对 L0 χ 接入阻塞链的影响

阻塞链现状（`chanlun/review-results/p127-stage3-precheck-20260719.md:18-25`）：

```
② shard2 完成 → 自动归并对账 → ①终验落盘
   → (0) signal 层终判重跑（wverify_full，新 BSP 集）
   → ⑧ M7 witness 重跑 → ⑨ M8 四层报告
```

第 (0) 步已完成（产物 `/tmp/wverify_full_g2.out`），本裁定即对 (0) 结论的定性，直接决定 ⑨ 层1 转引文本口径，并间接定义 L0 χ 接入的立项前提：

- **χ 门机制**：`filter_gamma_with_admission`（`selector.rs:357-388` 转引自 `l3-econ-gate-filter-rate-20260719.md:13`）按 z 桶准入量 `LCB > θ` 过滤候选；生产路径当前 χ≡1 全覆盖（`runner.rs:707`，同文档 :11），χ 接入属 L0 项（`l1-l2-implementation-20260719.md:28`「χ 未来接入（L0）」）。其立项前提是 **signal 层存在 validated z 桶**——否则 χ 是在未确认 alpha 的 μ 估计上设门。
- **A 下**：前提回摆未决 ⟹ L0 χ 接入只能以「机制实装、前提未确认」立项，措辞禁称「接入已验证信号门」；阻塞链 (0) 实质重开（回填回滚 + 重裁），⑧⑨ 的层1 转引对象失稳。
- **B 下**：前提在门控口径下成立，χ 可立项；但两道边界不变——(i) §5.3：χ 是执行路径门（影响订单流），signal Pass 不外推其净值效应；(ii) 实测未定：l3-econ 探针（`l3-econ-gate-filter-rate-20260719.md` §3-F1/§4）证明当前参数量纲下裸接入 χ 会把 L3 盈利引擎一并拒掉、net_r 方向不定【待实测】。**Pass 解锁立项前提，不解锁接入本身。** 另案若修订 powered/LCB 口径，χ 准入量公式（LCB=mean−z_α·std/√n）须联动复核。
- **C 下**：立项前提最干净（n_eff 通道关闭），其余边界同 B；附带风险：若后续 powered 口径前向修订，C 的「已覆盖」结论须连带回滚，阻塞链 (0) 面临二次定性。

## 4. 推荐排序（不代裁，供编排者拍板）

**B > C > A。**

1. **首选 B**。唯一同时满足三条硬纪律的候选：预注册纪律（门控输出照收，`decontam.rs:121` + M4「无离线补主裁决」）、090（Pass 措辞带 n_eff 照实标注 = 声明与门控能力一致；n_eff 关切不掩埋而是路由到正确的裁定对象——门控协议本身）、措辞§5.3/§5.6（边界不动、限定语保留）。它把 μ_R 桶临界过门这一真实疑点保留为前向审查项，而不是用一次事后翻转来「解决」它——事后翻转的代价比疑点本身更高。
2. **次选 C**。事实层最彻底（§0 的代码级证据链完整支持「n_eff 门已执行、271 系误读」），若裁定文本按 §1-C 风险1 精确化（powered 必要合取项已过门，非「perm_p+LCB 覆盖」）并附加「不预断 powered 前向修订」条款，则与 B 实质等价且叙事更干净。排 B 之后的原因：原表述机制误述需先修正，且「无保留」措辞在下游转引中有被简化为「n_eff 门槛不存在」的 090 隐患。
3. **末选 A**。经验内核真实（μ_R 桶 n_eff=8 临界、LCB 的 SE 未含自相关折扣），但救济手段错误：事后叠加判据翻转预注册门控输出，违反冻结判定顺序与 M4「无离线补主裁决」，且其措辞依据（§5.6 INCONCLUSIVE-only 条款）适用域为高级别桶、不覆盖本情形的 L0 统计桶。A 的合理关切应经 B 的「另案修订门控协议」通道吸收，而非以 verdict 翻转实现。

**三候选共有的不变量（无论拍哪个都不许丢）**：§5.3 不外推 max-full；perm_p 只作预注册门控判据引用、非策略择优论据（v3 硬禁令）；n_eff=252/8 数值在任何层1 文本中照实出现。

---

## 5. 自检（090 + 纪律）

- [x] 唯一产出 = 本 .md；`rust/`、`TARGET_STRATEGY*.md` 零改动；`rust/Cargo.toml` 未触碰。
- [x] 无 git mutation（无 commit/stash/checkout）；主仓 `/Users/silencehan/Projects/NewChanlun` 零写入（全部读取在 worktree 内完成）。
- [x] 实证地基（§0）全部落到代码锚：`decontam.rs:111-118/121/133-135/144-154/163`；`wverify_run.rs:331-336/346/502-526`；实测锚 `/tmp/wverify_full_g2.out:301-302`。
- [x] 推导界标注为推导（CV ≤ 9.65 / ≤ 1.72 由 powered 过门反推；271~1083 = CV∈[10,20] 区间与 `decontam.rs:163` 注释互证）。
- [x] 三候选均给出教义/措辞纪律/实证三层依据 + 各自代价；推荐排序附理由且不代裁（§4）。
- [x] v3 硬禁令复核：本材料未引入任何概率/统计推断作决策基础（门控判定为预注册频率主义口径的机械执行描述）；未做回测择优；无 EMH 假设。
- [x] 照实否定保留：μ_R 桶临界过门与 LCB raw-n SE 两处疑点未掩埋（§0-3、§1-B 代价、§1-C 风险2）。
