# floor 完整口径备料：E2E-L `CloseFloor_at` 三型教义分量 vs 现行地板合取

日期：2026-07-29　票号：#683（研究，map #529 子票）　性质：**备料，不裁定**（判定≠裁定，结论档留 grilling 票 #685）

工位：`/private/tmp/wt-683`（分支 `research/floor-closefloor`）。全程只读 + 单文件新增，零生产代码改动，零 cargo 构建。

---

## 0. 票面背景（一句话还原）

#641 取舍裁定第 5 条（comment-5121793896）与其关票后尾部链已把「地板」拆成两层：

- **已落地**：`chain_cert/` 的 `has_segment` 第四合取——链头独活（全下级证伪/缺失 ⟹ 链段集合为空）= 永远 `Open`，不构成链，「全链段谓词判过」不得在空集上真空成立（#641 comment-5121572134，机器载体 `chain_probe::ChainProbe::floor_blocked` + 测试 `head_only_survivor_stays_open_by_floor_conjunct`）。
- **未落地、留 fog 另裁**：E2E-L `CloseFloor_at` **三型完整口径**（`FormalFloor / QuasiFloor / UnresolvedFloor`）——不随 N3（`chain_cert/` 落地）自动升级，本票只备料，不动代码。

---

## 1. 三型清单与定义出处

来源三份：`e2eo-lineage-synthesis-20260728.md`（谱系综合，已在 main）+ `doc-divergence-endtoend-prototype-20260718.md` §5/§6.1（下称「原型」，git 历史锚 `640609071d`，工作树已删，本报告引其 git show 版本）+ `mainline-merged-roadmap-20260717.md`（下称「roadmap」，同一锚）+ #641/#667 评论链。

`CloseFloor_at` 定义（原型 §1 算子表 :72-73；roadmap:62）：

```
CloseFloor_at(as_of, prior_lineage_head?, lineage_revision) -> LineageRevision?
```

`E2E-F` 分支本地地板三型（原型 §5:176-180；roadmap:63 逐字复述）：

| 类型 | 触发条件（原型 §5:184，roadmap:63） | 一句话 |
|---|---|---|
| **`FormalFloor(level, event_id)`** | 当前节点已在塔声明的最低**形式概念层**（本塔映射 `L1`） | 递归背驰段套到了塔能表达的最低形式级别，正式闭合 |
| **`QuasiFloor(level, localization_id)`** | 同一前缀在该层**线段内部**用类背驰力度比较完成了缩点定位（本塔映射 `L0`） | 降到形式概念域边界之下，改用「类背驰」定位尾巴，不算新的形式证书 |
| **`UnresolvedFloor(reason)`** | 节点高于 `L1` 但当前前缀**没有可证子事件**，或结构/数据不足 | 还没套完，不能因果闭合，`Consume_at` 必须拒绝 |

关键行为规则（原型 §5:184-188）：
- `CloseFloor_at` **不读未来**——不判断"未来不会再有子事件"；只要节点 > L1 且当前前缀无可证子事件，只能给 `UnresolvedFloor`。
- `FormalFloor` 闭合后若日后真出现 L0 类背驰定位，走 `localization_tail_key` 生成带 `extends_lineage_key` 的**新谱系**，不回写原 Formal 谱系及其形成钟；消费规则必须显式选择接受 Formal 还是要求 Quasi（`match(..., floor_class=Quasi)` 只对 Quasi 生效）。
- `Quasi` 不得授权形式 BSP（原型 §6.1:259 引 065:94）。
- `E2E-L` 允许 `parent_level > child_level + 1` 的显式 skip edge（原型 §5:188 引 032:227）；skip edge 记事实、不自动补中间节点。

E2E-S6 验收判据（原型 §6.3:323）：`s4` 中**每条分支恰有一个** Formal/Quasi/Unresolved；L0 形式背驰证书为 0；Unresolved 消费数为 0；Quasi 不增加同级 BSP。

---

## 2. 现行地板合取相对三型完整口径差哪些面

只读检查对象：`rust/src/theta_v0/classifier/chain_cert/mod.rs`（零改动）。现行实装（`mod.rs:599-619`）：

```rust
let head_confirmed = nodes[0].state == Some(CandidateState::Confirmed);
let all_segments = edges.iter().all(ChainEdge::is_segment);
let has_segment = edges.iter().any(ChainEdge::is_segment);   // 地板条款落地面

// Closed ⟺ 头非证伪 ∧ 全边判过 ∧ head_confirmed ∧ !extendable ∧ has_segment
```

模块头注释（`mod.rs:41-43`）自陈：「floor 完整口径……仍留 fog 另裁：本模块只落地『至少一条有效链段』这一格，不自行补级别地板（如『必须降到 L0/L1 才能 Close』）」。逐条差距面：

1. **无三型标签，只有布尔合取**。`ChainStatus` 只有 `Open/Closed/Invalidated` 三态，没有独立的 `floor` 字段承载 `FormalFloor/QuasiFloor/UnresolvedFloor`——现行 `has_segment=false` 时落 `Open`，语义上顶替了 `UnresolvedFloor`，但不是同一对象（见第 5 条）。

2. **无级别地板检查（核心差距）**。Closed 判据不问链降到了哪一级；只要 `!extendable`（当前前缀无可再扩展的存活下级候选）+ `has_segment` 即可 Closed，即使根本没触达 L1。测试 `floor_conjunct_does_not_block_a_chain_that_has_a_segment`（`tests.rs:743-746`）自带**反事实锁**，逐字写明：「若把地板合取写成级别地板（如『必须降到 L0 才能 Close』），本例的 L2→L1 两节点链会被一并挡住——它当场变红」。这是当前实装**刻意不做**级别地板检查的直接证据。

3. **无 QuasiFloor / 类背驰定位尾机制**。链证书节点全部来自 `CandidateEventBook`（已是形式候选事件身份，`CandidateKey`），没有向下探入「线段内部笔间类背驰」的对象或 `LocalizationKey`/`localization_id`；`QuasiDivLocalization` 在本模块不存在对应类型。

4. **`extendable=false` 与「已到 L1 形式概念层」是两个不同判据，现行只查前者**。原型的 `UnresolvedFloor` 触发条件是"节点高于 L1 且当前前缀无可证子事件"——是一个**级别 + 证据**的合取；现行 `has_segment=false` 只是"边集合为空"这一**纯边存在性**判据，不问节点当前级别是否已经是 L1。多数情况下两者外延重合（没有下级候选通常也意味着没到 L1），但概念不同，尚未被证明外延相等。

5. **无「Formal 闭合后 Quasi 追加、不回写形成钟」的政策落地**。现行 `extends` 字段服务一般路径扩展（加子节点开新 `ChainKey`），但没有专门为「后到类背驰定位尾」保留 Formal 谱系形成钟不变的显式消费政策（`match(..., floor_class=Quasi)` 二选一）。

6. **无 `termination_reason` 字段**。原型 `TowerDivergenceLineage` 结构（§6.1:216）显式并列 `floor` 与 `termination_reason` 两个字段；现行 `TowerChainCertificate` 无对应字段，`Closed`/`Invalidated` 的成因分档（`ChainInvalidationCause`）存在，但地板成因不区分 Formal 达成 vs 别的原因。

差距面条目数：**6**。

---

## 3. 教义证据分级表

标签沿用原型纪律：**教义**＝博文/编纂版可直接支持；**推论**＝由已证教义合理导出但原文未逐字给出的工程定义；**无依据**＝纯工程细节，与教义无直接关联，不构成待裁教义问题。

| # | 条目 | 出处 | 摘录/要旨 | 强度 |
|---|---|---|---|---|
| 1 | `FormalFloor` 边界：背驰概念必须存在最低级别中枢后才有意义 | 博文 065:94 | 「根据最严格的定义，对背驰等概念，一定要存在最低级别的中枢后，才有最低级别的走势类型，才会有背驰等概念的存在，一般来说，在线段之下讨论背驰概念是没意义的，但可以根据类似背驰的力度比较方法来讨论线段之下类背驰的现象，但这和背驰是两回事情」 | **直接原文** |
| 2 | `QuasiFloor` 存在性范例：区间套最后一重落在线段内部笔间 | 博文 088:196 | 「214，从191开始的1分钟下跌走势的底背驰，对应着三重的区间套定位，最后一重是213-214之间的线段内部笔之间的定位」 | **直接原文（范例，非规则陈述）** |
| 3 | 类背驰/背驰二分的级别边界 | 博文 066:198 | 「背驰的概念，标准的在最低级别之上用，线段上的，只能是类背驰的判断」 | **直接原文** |
| 4 | skip edge 合法性：级别与背驰不必然逐级对应 | 博文 032:227 | 「背驰的级别和上涨的级别没有什么必然的对应关系，要有对应关系，就必须满足区间套关系」 | **直接原文，但与 #5 存在表面张力（待裁）** |
| 5 | 区间套须逐级降到最低级别、确认以下所有级别皆转折 | 编纂版 chan99 §六「区间套」:11,15 | 「……反复进行下去，直到最低级别，相应的转折点就在该级别背驰段确定的范围内」「必须确定它以下所有级别都转折了，这是所有背驰的前提」 | **直接原文，但与 #4 存在表面张力（待裁）** |
| 6 | `FormalFloor`/`QuasiFloor` 到 `L1`/`L0` 的具体数字映射 | 原型 §5:182（自陈"工程冻结"） | 「当前塔以 L0 线段起塔，故映射为 `L1=FormalFloor`、`L0=QuasiFloor`……若塔的基底编码改变，应随对象能力重算地板，不应硬编码沿用数字」 | **推论**（工程定义，原文无级别编号概念） |
| 7 | `CloseFloor_at` 不读未来的因果纪律 | 原型 §5:184（自陈工程定义） | 「不判断'未来不会再出现子事件'……这样地板判断由塔的概念能力与当前证据决定，不读未来」 | **推论**（软件因果闭包，原文无此算法层表述） |
| 8 | `extends_lineage_key` 晚到 Quasi 定位尾不回写 Formal 形成钟 | 原型 §5:186 | 「若以后出现该定位，则以 `localization_tail_key` 生成一条带 `extends_lineage_key` 的新谱系，原 Formal 谱系及其形成钟不回写」 | **推论**（append-only 工程设计，原文未讨论"不回写"这一软件不变量） |
| 9 | `UnresolvedFloor` 状态机与 `CloseFloor_at` 函数签名本身 | 原型 §1/§5（工程冻结） | `CloseFloor_at(as_of, ...) -> LineageRevision?`，四态转移表 | **推论**（纯工程接口，教义只支持"存在未套完的中间态"这一事实） |
| 10 | `FloorKey` 四位定长编码（`["Formal",level,event_key,null]` 等）与 WireV1 序列化规则 | 原型 §6.1:249 | 见原型逐字 | **无依据**（纯工程细节，教义不涉及数据表示） |
| 11 | `match(..., floor_class=Quasi)` 策略语法与 `exact_count=1` 约束 | 原型 §6.1:221-230,257-259 | 见原型逐字 | **无依据**（纯工程消费政策 DSL） |

**分布统计**：直接原文 **4** 条（#1-#4，其中 #4 与 #5 互为张力对，故 #5 也计入下方张力单独列示）+ 张力对 1 组（#4 vs #5，二者均为直接原文）；推论 **4** 条（#6-#9）；无依据 **2** 条（#10-#11）。

> 计数口径说明：#5（chan99 §六）与 #4 同属"直接原文"但构成一组内部张力，未单独计入"直接原文"总数之外的新类别——本表按条目计（11 条中 4 直接 + 4 推论 + 2 无依据 + 1 张力条目 #5 归入直接原文，共 5 条直接原文）。汇报口径取整表：**直接原文 5 / 推论 4 / 无依据 2**。

---

## 4. 待裁问题清单（供 grilling 票 #685 用）

1. **张力调和**：#4（博文 032:227「背驰级别与上涨级别无必然对应，需满足区间套关系」，支持 skip edge 合法）vs #5（chan99 §六「必须确定以下所有级别都转折，这是所有背驰的前提」，读起来更接近"逐级不可跳"）——两者是否真冲突，还是分别描述"背驰级别 vs 走势级别的对应"（#4）与"区间套内部逐级descend 的完备性要求"（#5，讨论对象不同）？若后者成立，则张力是表面的，skip edge 合法性不受影响；若前者成立，需要裁定 skip edge 在何种条件下允许。

2. **`FormalFloor` 触发条件是否应校验级别，而非只校验"无可扩展下级候选"**：现行 `has_segment` 合取用边存在性代理"是否到达地板"，原型定义是级别（L1）+ 证据的合取。是否需要补一个显式 `root_level ≤ L1` 或等价检查？若不补，`Closed` 语义上界定为"当前前缀内无法再降"，而非"已达到塔声明的最低形式层"——这两者外延何时分岔（例如塔的 L1 常量本身发生变化、或某条链在 L2 就已经"暂时"没有存活下级候选但未来会有）需要举例核实。

3. **`QuasiFloor` 是否立项**：是否需要在塔内新增"线段内部笔间类背驰定位"对象（`LocalizationKey`/`QuasiDivLocalization`），使地板真正具备三型而非现在的二值代理（"有链段=可能Closed" vs "无链段=Open"）？工程量评估——需要新的候选来源（笔级别定位而非现有 `CandidateEventBook` 的形式候选），规模不小。

4. **`UnresolvedFloor` 与现行 `Open` 是否应该分裂为两个状态**：现行 `Open` 混淆了"链头未确认/还在行进中"与"链头已确认但降不下去、真到了 UnresolvedFloor"两种情况；三型完整口径要求两者可分辨（消费侧对 `UnresolvedFloor` 必须拒绝，对普通 `Open` 只是"还没到时候"）。是否需要拆分 `ChainStatus`，还是在 `Open` 基础上另加一个诊断字段？

5. **`termination_reason` 与地板成因分档是否要落地为独立字段**，还是维持现状（`ChainInvalidationCause` 只覆盖 `Invalidated` 分支，`Closed` 无成因分档）？

6. **落地顺序/是否本轮就做**：本票只备料；若 grilling 判定"立项"，工程规模与 #641 修复轮（`chain_cert/` 主缝）相当或更大（新增候选来源类型 + 新 Key 结构 + 消费政策扩展），需要单独任务链，还是可以在现有 `chain_cert/` 上做增量扩展？

---

## 5. 附：一手材料索引（便于 grilling 复核）

- 谱系综合母本：`chanlun/review-results/e2eo-lineage-synthesis-20260728.md`（已在 main，本报告 §2 术语沿用）。
- 原型全文（工作树已删，git 历史锚 `640609071d`）：`git show 640609071d:chanlun/review-results/doc-divergence-endtoend-prototype-20260718.md`。
- roadmap 全文（同一锚）：`git show 640609071d:chanlun/plans/mainline-merged-roadmap-20260717.md`。
- #641 尾部裁定链：`gh issue view 641 --comments`（地板条款 comment-5121572134；两套 N3 取舍 comment-5121793896 第 5 条；关票后尾部链）。
- #667 尾部影子评审：`gh issue view 667 --comments`（PASS WITH CONDITIONS → 收编 → PASS）。
- 现行实装（只读，零改动）：`rust/src/theta_v0/classifier/chain_cert/mod.rs`（模块头注释 :33-43，Closed 判据 :599-619）、`rust/src/theta_v0/classifier/chain_cert/tests.rs`（:691-741 地板条款测试，:743-820 反事实锁测试）。
- 博文原文：`docs/chanlun/text/blog/065-第65课.md:94`、`docs/chanlun/text/blog/088-第88课.md:196`、`docs/chanlun/text/blog/066-第66课.md:198`、`docs/chanlun/text/blog/032-第32课.md:227`。
- 编纂版对照：`docs/chanlun/text/chan99/0027-第六节 区间套.md:11,15`（"类背驰"编纂版分散例句见 `0017/0018/0046`，无独立小节收录 065:94 的严格定义原句）。
