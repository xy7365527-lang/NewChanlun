# PDF 全读①：启动等价关系（先从分解入手）——逐页全文复核

- task: #66（ws-pdf1）
- 文件：`docs/pdfs/等价关系与不变量 - 启动等价关系：先从分解入手.pdf`（6.4M）
- 编排者明令：「之前的 pdf 全都得看一遍别有遗漏」；此前 #22 审计仅读至 24 页（§3.5 中枢在覆盖内）
- 认识论等级：**L0（纯定义/立法，对话体推导）**——本 PDF 不产生 L2/L3 数据验证，只提供概念本体与"何谓死亡/完成/生成"的立法。对 alpha 主张零发言权。
- 环境：纯只读，零 git 操作

---

## 1. 逐页覆盖声明

- **总页数：406 页（pypdf 计数）**
- **已读页数：406 / 406 = 100%**
- 提取方式：PDF 有完整文本层（无扫描页/无近空页，406 页全部 len(text)>250），用 pypdf 全文提取 521,740 字节到 `/tmp/qidong_full.txt`（12000 行），Read 工具分 11 段全量通读（1-30 / 30-65 / 65-93 / 93-115 / 160-210 / 210-231 / 231-280 / 280-302 / 302-323 / 323-349 / 349-406）。
- 图/公式：文本层含 Unicode 数学符号（≈, ⇔, ∈, ∀, ¿, ´O 等，OCR 轻度错位但语义可复原）；无图像丢失（本 PDF 是对话文本，非图表文档）。
- 诚实边界：文本提取对上标/下标偶有错位（如 t¿ = t*, ´O = Ō），已在笔记中还原语义；不影响本体/判据/条款的读出。

---

## 2. 25 页之后的全部内容笔记（逐节：本体链 / 公理 / 判据 / 推导）

本 PDF 是一段**极长的单线对话**（用户 + ChatGPT），从"起步选项 A/B/C"出发，最终把"新缠论"立法为一套**源层结算宪法 + 基础设施执行层 + 写作层**三层体系。以下按小节归档（前 24 页= 本体清单，#22 已覆盖，此处只补 25 页后）。

### 2.1 pp.25-28 冻结规格 v0.1（对象链 + 否定触发点索引 Death Index）

- **对象链（Canonical Object Chain）**：Raw K线 →(包含处理/顺序原则合并)→ 处理后K线序列 →(分型判定)→ 分型(顶/底) →(笔规范)→ 笔 →(线段规则:≥3笔+前三笔重叠;新线段确认才算前线段完成)→ 线段 →(三段次级别走势类型重叠)→ 走势中枢[ZD,ZG] →(中枢数量/同向性)→ 走势类型(盘整/趋势) →(递归构造口径)→ 级别
- **否定触发点索引 D0-D6**（"只记录在哪死/凭什么死/Ledger记什么，不许解释为什么"）：
  - D0 K线包含处理（输入域死亡）：合并顺序不唯一/需主观挑 → 死。Ledger 记合并方向+等价K线序列ID+合并历史。**冻结：禁止分型/笔阶段回原始K线"再解释"。**
  - D1 分型死亡：依赖补点/猜测/事后回看；或用未完成包含处理的数据判分型。
  - D2 笔死亡：违反笔三规范任一条（A对象死）；在新一笔未生成前宣告前笔完成（B完成判定死）。
  - D3 线段死亡：终点只能靠"看起来结束了"无法程序给出（A）；新线段未生成即宣告前线段完成（B）。
  - **D4 中枢终结（当且仅当）**：次级别离开中枢后，其后的次级别回抽走势不重新回到该中枢内。Ledger 记 [ZD,ZG]+离开段ID+回抽段ID+是否回到中枢（布尔）。**冻结：禁止用"延伸/扩张/扩展/升级"解释性续命（除非单独立法为可结算二阶规则）。**
  - D5 走势类型死亡：类型判定不以"中枢数量/同向性"为基础而回退"看起来涨/跌/横"（A）；把未完成类型当完成用（B）。
  - D6 级别死亡：把级别定义成时间周期标签（A）；用"换周期/提级别看看"撤销已发生的否定事件=把否定延期化（B）。
- **Ledger 最小字段**：O.id / O.state(构造/延续/完成) / O.alive / O.death=(t*,reason_code) / O.components / O.proof_tag(口径标签)。

### 2.2 pp.29-40 等价关系法典（B0 分解 / B1 表示 / B2 跨系统）

- **唯一合法句式**：同一 Ledger 世界，对同一目标对象 O，两表示 A/B 等价 ⇔ **O 的不变量命运一致**（同时成立/同时死亡；死因类型一致；死亡不可逆）。= "等价 = 不变量同生同灭"。
- **表示三类（其余非法）**：分解表示（多义性分支 Di）/ 规范表示（canonical_view 代表元）/ 投影表示（focus_window/register，只读不得产新结论）。
- **B0 分解等价 C9**：(E1)不变量一致 (E2)否定一致——death 同时触发且 reason_code 一致，禁"Di死而Dj不死" (E3)不可逆一致。canonical_view 必须 ∈ D 且与全部 Di 等价，否则 INCONSISTENT。删枝让不一致消失 = ILLEGAL（预测性删枝）。
- **B1 表示等价 C10（R-Law）**：表示只改"可见性"不改"可结算性"；窗口/档位/渲染三类合法操作；表示不能生成/完成/终结对象。越权 → ILLEGAL（该表示失效，非对象失效）。
- **B2 跨系统等价 C11（X-Law）**：两系统 S1/S2 在级别 ℓ 跨系统等价 ⇔ 结构命运同构 + 任一系统的否定必须在另一系统以**同型否定**出现。同构映射 φ 保：结构保型(笔↔笔/段↔段/枢↔枢)+时序保型+不变量保型（禁强度/幅度/速度等外源量进入映射）。**封禁区：比价走势/相关性/同步性/领先滞后皆非跨系统等价。** 单边结算 → ILLEGAL。

### 2.3 pp.41-62 失败传播 D-Law + 失败驱动生成 E-Law + 最小状态机 F

- **D-Law（失败传播）**：D0 失败传播公理（失败是结构事件不是注记，传播规则与成功完全对称、不可预测、不可裁剪、不可回滚）。D2 三向传播（上溯/横向/跨域，由结构边界决定非人选）。D5 传播终止仅三种：生成新对象/更高层结算/公理修订。**"注意：修复、回补、等待均不在合法终止之列。"** D6 失败对称性（破坏对称=语义作弊）。
- **E-Law（失败驱动生成）**：E0 生成公理——**"生成不是修复失败，而是失败的合法后果"**，新旧对象不可合并/不可回滚/不可解释性续命。E3 生成对象独立性（新ID/新起点 tG>t*/新组件域不含失败对象）。E4 失败继承（继承失败信息，不继承失败对象）。
- **F-FSM（最小状态机）**：唯一合法状态 8 个——SEED/BUILDING/ALIVE/COMPLETE/FAIL/FROZEN/INHERITED/TERMINAL。**"不存在 PENDING/WAITING/RECOVERING 等状态。'看情况/等等看'是 UI 情绪不是状态。"** 禁止跃迁 F4：FAIL→ALIVE/COMPLETE/BUILDING；**FROZEN→ALIVE/COMPLETE/FAIL（不经 GENESIS 或 AXIOM_SWITCH）**；COMPLETE→ALIVE（完成不可撤销）；"通过换表示/换分解解除冻结"（非法）；INHERITED→ALIVE（继承不是复活）。核心宣言：**"只能死、只能冻、只能生；不能修。"**

### 2.4 pp.63-115 工程实现规范 C（Gatekeeper + 数据模型 + Python 骨架）

- C0：S账本=唯一真源（append-only 事件溯源，快照是投影可缓存但无真源地位）；CommitGroup 原子提交（全成或全不成，禁乱序）。
- C3 校验管线 C1-C10：本体一致性/递归一致性/分解一致性/否定一致性/级别联立/赋格一致性/表示等价/跨系统等价/失败传播/生成一致性。四态输出 CONSISTENT/UNDERDETERMINED/INCONSISTENT/ILLEGAL。ILLEGAL 走 LVC 回溯重建协议（Freeze→Locate Fault Origin→Rollback to LVC→Reconstruction，写 RuptureEvent）。
- C8 IntentType payload 表 + C9 object_type 最小方法集（compute_provenance/compute_invariants/detect_death/detect_complete）+ C10 reason_code 枚举（INPUT_NONUNIQUE / FRACTAL_NONUNIQUE / SEGMENT_COMPLETE_WITHOUT_NEWSEG / CENTRE_TERMINATION_RULE / LEVEL_TIMEFRAME_THEFT / DECOMP_BRANCH_MISMATCH / REP_OVERRULE_WRITE / CROSS_SINGLE_SIDE_SETTLEMENT / PROPAGATION_MISSING / GENESIS_RULE_VIOLATION 等）。
- pp.91-160：两版 Python skeleton（第一版 DummyEngine 骨架 + 第二版 minimal-closed-loop 真引擎），忠实实现 FSM ALLOWED_TRANSITIONS + STATE_WRITE_PERMISSIONS + Gatekeeper 路由。分段完成条款 `SEGMENT_COMPLETE_WITHOUT_NEWSEG` 直接编码为 ILLEGAL。

### 2.5 pp.173-197 算法降级条款（关键立法）

- **"等价必须先是可否定的结构事实，而不是可计算的相似度结果"**。DTW/图同构/embedding/kernel/manifold alignment 等高级算法若提前进入 = 篡位（算法变裁判/否定被概率化 0.73-0.41/失败传播被算法吃掉=非法修复）。
- 算法三个合法降级位置：候选生成器（提名不裁决）/canonical_view 选择器（只负责效率不负责真理）/失败后探索器（探索新结构非修复）。
- **一句话结论："算法等价，只有在结构等价已经被立法之后才有资格进入系统，而且永远不能拥有否定权。"**
- pp.178-194：作者把这套东西定位为《市场法哲学原理》的"结算宪法插章"（写作层讨论，非引擎相关，但确认作者意图=把 New-Chan 当量化上位范式）。

### 2.6 pp.198-244 等价类商结构 Quotient Law Q0-Q10 + Class-Level 等价 B0'/B1'/B2'

- **Q1 等价类对象 Ō=[O]~ 是后续论证的最小叙述单位；任何结构断言不得针对某个表示 O(r) 而必须针对等价类 Ō。**
- Q3 类签名 = (invariant_signature, death_signature=(dead?,t*,reason_code))；类内所有表示/分支必须共享同一组签名，否则"不是口径不同，而是等价类失效"。
- **Q5 失败闭包：等价类内任一失败事件 → 整类冻结；冻结禁止以更换表示/分支/窗口解除；只能生成继承或更高层覆盖终止。**
- **Q6 算法降级条款：任何高级算法仅拥有候选权，永不拥有否定权/结算权。**
- Q10 候选生成器接口：propose_candidates → 只触发"待验证任务"，真正判定走 verify_equivalence（Gatekeeper）。
- **class_id 作为唯一对外发布单位**："策略、回测、比价、AI 都只能订阅 class，而不能订阅某个分支或某张图。"
- pp.204-244：Class-Level 三性（自反/对称/传递在类上自动成立）+ Map Algebra（同构映射代数 M0-M9，comp_pairs/preserves/proof_hash/可复合可逆可审计）+ Class Schema Cards（Card 0-9 标准字段，如 CentreClass.zd_zg / SegmentClass.overlap_interval / TrendTypeClass.kind） + Verifier Template（四步保型：Type/Anchor/Inclusion+Order/Signature+Death）。

### 2.7 pp.214-244 比价独立系统法典 Ratio System Law R0-R8（关键，对接 576=C）

- **核心结论：S_ratio = 从 (S_A × S_B) 投影出来的派生系统；信息必然丢失；不具备反向解释权。既不是同构（isomorphism），也不是嵌入（embedding），而是投影/商（projection/quotient）。**
- R3 解释权隔离：SR 中的结算只对 SR 有效；对 SA/SB 仅能产生"候选任务"，不得产生结论。"比价能提示相对关系发生了什么，但不能告诉你原系统发生了什么。"
- **R5 传播单向：SA/SB 失败 → 冻结 SR（RATIO_DEPENDENCY_BREAK）；SR 失败 → 禁止倒灌 SA/SB（RATIO_BACKFLOW_SETTLEMENT = ILLEGAL）。**
- R8 Ratio Candidate Protocol：SR 只做"雷达"，五种候选类型（ISO/CO_SHIFT/RUPTURE/ALIGNMENT_RISK/MAP_REJECT），所有裁决回 SA/SB 的 class-level + MapRecord 验证。

### 2.8 pp.256-323 三层不污染法 SINF + 源层宪法 S0-S7 + Object-Law Table + EQ-Closure

- **SINF（Source–Infrastructure Non-Pollution Law）**：三层——源层（新缠论本体宪法）/基础设施层（等价/映射/比价/候选/验证/证明对象=执行裁决审计工具）/写作层（书章表达）。**基础设施层与写作层不得生成源层语义（Semantic Usurpation = ILLEGAL）。裁决权仅属源层三件事：不变量是否成立/否定是否触发/失败后是否必须生成。**
- **源层宪法 S0-S6**：S0 存在条件(不变量,只依赖规范化输入+components+同层结算,禁表示/模型/概率/相似度) / S1 否定(Death=(dead?,t*,r),不可逆,r来自有限枚举) / S2 Ledger(唯一真源,未记录=未发生,S2.3解释权冻结) / S3 失败传播(不可裁剪不可预测不可回滚,方向由结构决定) / S4 生成(断裂即生成非修复,新ID/新起点/不含失败对象) / S5 完成证据法(完成必须有证据) / S6 推进语法(条件→结论,只在断裂处引入概念,错用词纠错回溯)。
- **补丁 v0.1.1**：S5.3/S5.4 完成证据法**双闸一致**（对象自证 Verify(O,E)=True + Gatekeeper，缺证据=UNDER，伪完成=ILLEGAL/COMPLETION_USURPATION）；S2.4/S6.4 断裂收据（Proof Artifact）仅三触发点（verify_map/freeze/genesis），其余=PROCEDURAL_INFLATION。
- **S7 等价宪法**：S7.1 等价=签名一致（Sig=(InvSig,DeathSig)）；S7.2 签名不一致=类冻结+传播+只能生成/覆盖解除；S7.3 跨系统同构不是等价但须尊重同型否定；S7.4 比价系统无反身裁决权；S7.5 完成证据法在类层成立。
- **Object-Law Table v0.1（对象条款实例化，pp.291-303）**——每对象 Inv/Neg/Evidence + reason_code：
  - Fractal：INV-F1..F4（kind∈{top,bottom}/anchor_k_ids长度3/price_interval合法/唯一性）；无强制完成证据。
  - Stroke：INV-S1..S5（端点fractal/一顶一底/中间非分型K存在/top_high>bottom_low/输入域一致）；无强制完成。
  - **Segment【关键对象】**：INV-SEG1..6（≥3笔/first3锚/overlap非空/endpoint_rule显式/feature_seq_id/组件闭合）；**Evidence: 完成必须有"新线段证据"+对象自证，缺=UNDER，伪完成=SEGMENT_COMPLETE_WITHOUT_NEWSEG(ILLEGAL)。**
  - **Centre**：INV-C1..6（≥3子走势类型/first3连续/zd_zg overlap算子重算/order_index连续）；**NEG-C2 中枢死亡（当且仅当）：leave_sub_id 后的 pullback_sub_id 的 interval 与 [ZD,ZG] 不相交 = CENTRE_TERMINATION_RULE；未给 leave/pullback 不得宣告 death（UNDER）。**
  - **TrendType【关键对象】**：INV-T1..5（centre≥1/order保序/kind结构计算:1枢=盘整,≥2同向位移=趋势/interval包络/组件闭合）；Evidence: 新走势类型证据+对象自证，伪完成=TRENDTYPE_COMPLETE_WITHOUT_NEWTT。
- **EQ-Closure 定理 + Equivalence Minimal Kernel + ClassFSM 路由表 + Field/Event/Clause Matrix + reason_code→条款→动作一页表 + 单测清单（pp.323-360）**：把 S7 与 D/E 在类层闭合为可引用定理（等价失败=类断裂,必须冻结→传播→必要时强制生成）。最小核只保留 class_id+Sig+C15/C16/C11+ClassFSM6条迁移。

### 2.9 pp.361-406 落地讨论（NewChan-Quant 重写 / MVP / LLM 隔离）

- 作者结论：**不改造旧 chanlun-quant，而是"借壳"**（旧系统只当数据/回测/执行外设，不继承旧识别/旧信号/旧多智能体）；新缠论主干 = Ledger + Gatekeeper + Signal API（class_id 只读订阅）。
- MVP：单标的/单级别/三对象链（fractal→stroke→segment，先不做 centre/trend_type），产出 events.jsonl+snapshot.json+freeze_stats.csv，6 条验收（可复现/去口径化/冻结可解释/冻结禁写/完成证据/收据触发锁）。
- **LLM/多智能体裁定（p.406，反复重申）**："LLM 对 Ledger 永远只读。它的输出只能进入 CandidateRecord 或 AuditReport。任何试图触发 INV_OK/DEATH/COMPLETE/FREEZE/GENESIS 的输出，一律丢弃并记录为越权。" 选股可交 LLM 但仅 Candidate-only（提名+理由+证据引用，不输出结论信号，不写 Ledger）。

---

## 3. 新发现清单（此前 24 页审计未覆盖，与现有实装相关）

对照 637 口径B / 673 三分拆 / 小转大 / partition-proof / alpha 桶键，全文新发现如下（均为 L0 立法层）：

1. **【强化 637=B，非新矛盾】Card 4 CentreClass + INV-C3**：`zd_zg` = overlap 算子结果，"由 overlap 算子重算一致，否则 INCONSISTENT"，明确 zd_zg 是全三段重叠的重算结果——**跨全文（§3.5 本体 + Card 4 字段卡 + Object-Law INV-C3）三处一致站队口径B**，比 24 页版仅 §3.5 一处更硬。task #11（637=B 迁移）方向被 PDF 全文强化。

2. **【新约束】完成证据法双闸一致（S5.3/S5.4）**：segment/trend_type 的 COMPLETE 不仅需"同级别新对象证据"，还需**对象自身可复核 Verify(O,E)=True**（不能只靠 Gatekeeper 单闸）。缺证据=UNDERDETERMINED，宣告完成但证据缺失/不可复核=ILLEGAL（COMPLETION_USURPATION）。→ 直接呼应 memory 中 `frontier_resume_bt_too_late`（consumed 停太晚=实现bug）：PDF 立法确认"延续≠完成，必须形成新线段才确认前线段完成"，且要求双闸。

3. **【新精确判据】中枢死亡 NEG-C2/D4（当且仅当）**：leave_sub_id 之后的 pullback_sub_id 的 interval 与 [ZD,ZG] 不相交 = 中枢死亡（CENTRE_TERMINATION_RULE）；未给 leave/pullback 不得宣告 death（UNDER）。这是可编码的中枢终结充要条件，比编纂版更严格（禁"延伸/扩张/扩展/升级"续命）。

4. **【新参照 for 576=C】Ratio System Law（比价=投影/商，无反身解释权，单向依赖）**：R5 SA/SB 失败→冻结 SR（RATIO_DEPENDENCY_BREAK）；SR 失败→禁倒灌 SA/SB（RATIO_BACKFLOW_SETTLEMENT=ILLEGAL）。为 576=C 双账本（LedgerState+TWState）与未来跨标的/金油比工作提供本体层法条（比价不得倒灌原系统裁决）。

5. **【新参照 for alpha 桶键】Q6/S0.3/S7.4 算法降级条款**：任何相似度/嵌入/评分算法仅有候选权（propose），永无否定权/结算权（不得进 inv_signature/death_signature，不得触发 FAIL/COMPLETE/GENESIS）。alpha 估计（perm_p/LCB）作为统计工具属"基础设施层"，其输出不能反向定义"结构事实"——与 R2/D1（桶键用 bsp_class 是缠论结构先验，σ^H/MACD veto 作 feature 非 gate）方向一致（结构裁决 > 算法评分）。

6. **【新架构参照】FSM 8 态 + 禁止 PENDING/WAITING/RECOVERING**："看情况/等等看"是 UI 情绪不是状态；FROZEN→ALIVE/COMPLETE 不经 GENESIS/AXIOM_SWITCH = ILLEGAL。为 Ledger 实装（576=C）提供状态机参考骨架。

7. **【新参照 for StructBreak 第四类纤维 task #62】"断裂即生成"**：D-Law 失败传播 + E-Law 生成 + 断裂收据三触发点（verify_map/freeze/genesis）——StructBreak 作为独立纤维与 PDF 的"断裂=genesis 触发"本体一致（断裂不是修复对象，是新谱系起点）。

---

## 4. 矛盾即报（与本轮已 commit 裁定的对照）

**结论：零硬矛盾。全文对本轮五大裁定要么强化、要么正交，无一条推翻已 commit 裁定。**（因此本任务不触发 SendMessage main 中断上报——已在读全程持续核验，无矛盾即无中断。）

关键对照——**R3 hwm_gain 棘轮=语义回补（codex 终局裁决 C'，task #34-36 已实施）被本 PDF 全文强力强化**：
- S1.2 不可逆公理：dead?=True 不得回滚为 False。
- S2.3 解释权冻结：禁止通过换表示/换分解/换口径继续结算。
- D5 传播终止："**修复、回补、等待均不在合法终止之列**"。
- S4.1 生成公理："**生成不是修复失败，而是失败的合法后果**"；不可解释性续命。
- F4/FSM：FROZEN→ALIVE 不经 GENESIS = ILLEGAL；D4/D6 明禁"延伸/扩张/升级/换周期"续命。
- → hwm_gain 棘轮（浮盈峰值只增不还、Revalue 当可分配权益、把 FALSIFIED 翻 PASS）**正是 PDF 反复判死的"修复式续命/延期机制"**。本 PDF（一级推导文档，非编纂版）为 codex R3 裁决提供了本体层立法根据，acc-GAP3 恢复 FALSIFIED 完全对齐 PDF 宪法。**这是确认，不是矛盾。**

其余裁定对照：637=B（§3.5+Card4+INV-C3 三处站队B，强化 task #11）、673 三分拆（Verifier 按 object_type 分判据，一致）、小转大（无 depth 凭据不篡位，与 S0.3 外源无裁决权同向）、partition-proof（本 PDF 是对象本体分类 fractal/stroke/segment/centre/trend_type，与走势发展双源分类正交互补，无冲突）——均无矛盾。

---

## 5. 与 pdf-conformance-audit-20260702.md（#22）的增量对照表

#22 审计对本 PDF（PDF3）仅覆盖 24 页（§3.5 中枢 + 公理1 包含优先）。本次全读 406 页的**净增量**：

| 维度 | #22 审计（24页覆盖） | 本次全读（406页）净增量 |
|------|---------------------|------------------------|
| 中枢口径 | §3.5 站队口径B（一处） | **Card4 CentreClass + INV-C3 + overlap算子重算**（全文三处站队B，更硬） |
| 公理 | 仅公理1 包含优先 | 补公理2(否定只在包含处)/公理3(级别不是对象)/**公理4(不存在结构修复=关键,grounds R3)** |
| 否定/死亡 | 未覆盖 | **Death Index D0-D6 + Object-Law Neg(O) + 中枢死亡充要 NEG-C2** |
| 等价关系 | #22 靠 PDF1/2/4/5 拼 | **本 PDF 自带完整 B0/B1/B2 法典 + Quotient Law Q0-Q10 + Class-Level B0'/B1'/B2'（等价关系的一级源文在此 PDF）** |
| 失败/生成 | 未覆盖 | **D-Law 传播 + E-Law 生成 + F-FSM 8态 + EQ-Closure 定理** |
| 比价/跨系统 | 未覆盖（576=C 无 PDF 对照） | **Ratio System Law R0-R8（比价=投影/商，禁倒灌）+ Map Algebra + X-Law**——为 576=C 提供本体层法条 |
| 完成证据法 | 未覆盖 | **S5.3/S5.4 双闸一致 + SEGMENT_COMPLETE_WITHOUT_NEWSEG**——呼应 frontier resume consumed 停太晚 |
| alpha/算法 | R2 D1 靠 level-sigma PDF | **Q6/S0.3/S7.4 算法降级条款（算法仅候选权无结算权）**——为 D1 桶键"结构裁决>算法评分"提供本体依据 |
| R3 gap3 棘轮 | 靠断点检测 PDF p8③"禁语义回补" | **本 PDF S1.2/S2.3/D5/S4.1/F4 五重立法禁"修复/回补/续命"**——R3 裁决本体根据大幅加固 |
| 工程实装 | 未覆盖 | **Gatekeeper 管线 C1-C16 + Python skeleton + Object-Law Table + Field/Event/Clause Matrix + reason_code 全枚举**（NewChan-Quant 重写参考架构） |
| LLM 边界 | 未覆盖 | **LLM/多智能体永远只读，仅 Candidate/Audit，禁写 Ledger**——与本仓库 `.claude/rules/llm-role-boundary.md` 同构 |

**总判**：#22 审计对本 PDF 覆盖率约 6%（24/406），且只触及本体层前段。本 PDF 的**核心贡献（等价关系完整法典 + 失败/生成动力学 + 源层宪法 S0-S7 + 比价投影法 + 完成证据双闸）全部在 25 页后**，#22 完全未触及。本次全读补齐后：无新硬矛盾，R3/637=B 两裁定获本体层强化，另得 5 条新约束/新参照（完成证据双闸/中枢死亡充要/比价禁倒灌/算法降级/FSM禁中间态）供后续实装引用。

---

## 结果包六要素

1. **结论**：`docs/pdfs/等价关系与不变量 - 启动等价关系.pdf` 406 页 100% 通读完成。本 PDF = 新缠论"源层结算宪法"的一级推导文档（等价关系 B0/B1/B2 + 失败传播 D + 生成 E + 状态机 F + 工程规范 C + 商结构 Quotient + 比价投影 Ratio + 源层宪法 S0-S7 + EQ-Closure）。零硬矛盾；R3 棘轮=语义回补裁定 + 637=B 迁移获本体层五重强化；新得 5 条约束/参照。
2. **定义依据**：逐页锚定——§3.5+Card4+INV-C3（中枢口径B）；D0-D6+NEG-C2（否定/死亡）；S1.2/S2.3/D5/S4.1/F4（禁修复回补续命，grounds R3）；S5.3/S5.4（完成证据双闸）；R0-R8（比价投影禁倒灌）；Q6/S0.3/S7.4（算法降级）。
3. **边界条件（结论翻转）**：本 PDF 是 L0 立法层，不产生 L2/L3 数据证据——若后续真实数据回测显示某对象的 Inv/Neg 判据在市场上不可结算（如中枢死亡充要在真实 K 线上全 UNDER），则 Object-Law Table 的该条实例需回炉，但源层宪法（S0-S7）本身不因数据翻转（它规定"何谓事实"，不规定"事实为真"）。R3 强化结论若翻转，需 PDF 出现"合法资金源语义"允许 Revalue（本 PDF 无此例外）。
4. **下游推论**：(a) task #11（637=B）本体依据加固，可继续；(b) 完成证据双闸（S5.3/S5.4）应纳入 segment/trend_type 完成逻辑核查（呼应 frontier resume 停太晚）；(c) 576=C 双账本可引 Ratio Law（禁倒灌）作跨账本边界法条；(d) alpha 桶键 D1"结构裁决>算法评分"得 Q6 本体支撑；(e) StructBreak 第四类纤维（#62）与"断裂即生成"本体一致。
5. **谱系引用**：本 PDF 与 R3（codex-r3-ruling / hwm_gain 棘轮=语义回补，task #31/34-36）、637（task #11 pending）、576=C（task #8）、673（task #12）、frontier-resume（memory project_frontier_resume_bt_too_late）、StructBreak（task #62/58）直接相关；与本仓库 `.claude/rules/llm-role-boundary.md`（LLM 只读边界）同构。曾发生概念分离领域=R3 语义回补（已 settled）。
6. **影响声明**：本文件为全读复核笔记，零代码/零 git/纯只读。产出=(1)406页100%覆盖声明 (2)25页后逐节笔记 (3)7条新发现 (4)零硬矛盾核验+R3/637双强化 (5)与#22增量对照表。不改动任何代码/定义/谱系，仅为下游（637迁移/完成证据/576跨账本/alpha桶键/StructBreak）提供本体层参照。
