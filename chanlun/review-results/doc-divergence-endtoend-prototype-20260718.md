# 背驰端到端原型：定义、时钟、地板与塔内谱系

日期：2026-07-18 ｜ 性质：**教义考据收束 + 工程原型定义**（只改文档；零代码改动、零数据重放、零 cargo、零 git mutation）

任务：把趋势背驰、盘整背驰、回拉 0 轴、确认时钟、类背驰地板与区间套谱系收束成一个可判真伪的端到端原型，并明确当前 sidecar/nest 管线只能算到哪里。本文不重做考据，直接复用：

- `doc-trend-divergence-predicate-20260717.md`（趋势谓词、MACD 口径、确认语义）；
- `doc-pan-divergence-doctrine-20260717.md`（盘背对象、力度、定位、区间套地位）；
- `doc-divergence-validity-test-20260717.md`（活假设、成立/不成立、回中枢测试）；
- `nest-rigor-textual-audit-20260717.md`（包含对象、逐级非必然、类背驰地板）；
- `nest-tower-native-gap-audit-20260717.md`（塔内 N0–N7 缺口与 L1–L6 对拍草案）。

所有教义引文均已回到 `docs/chanlun/text/blog/` 对应行亲读；引用格式 `0XX:行号` 即该课 Markdown 的实际行号。标签纪律：**教义**＝博文可直接支持；**工程定义**＝本文冻结的原型接口；**原文未决**＝博文没有给出足以唯一实现的规则。本文与 `chanlun/plans/mainline-merged-roadmap-20260717.md` 共用编号：`E2E-D1`–`E2E-D6`、`E2E-O`、`E2E-F`、`E2E-L`、`E2E-N0`–`E2E-N7`、`E2E-S1`–`E2E-S8`。

---

## 1. 六要素表（`E2E-D1`–`E2E-D6`）

| 编号 | 要素 | 趋势背驰 | 盘整背驰 | 塔内原型字段 | 教义锚 |
|---|---|---|---|---|---|
| `E2E-D1` | 定义域与种类 | `a+A+b+B+c` 必须是趋势，A/B 为同级别中枢 | 第一个中枢或中枢震荡域；两比较段不要求都是趋势 | `kind`、`event_level`、`center_ids`、`domain_proof` | 037:16；027:16、027:20 |
| `E2E-D2` | 比较对象 | 最后中枢 B 的进入段 `b` 与离开段 `c`，比较 `c` 对 `b` | 同一中枢前后的第一、三段（或 `a+A+b` 中 `b` 对 `a`） | `candidate_group_id`、`pair_id`、左右走势 ID、`pair_proof` | 031:883、033:26；027:20、037:148、037:152、061:26 |
| `E2E-D3` | 结构前提 | `c` 为次级别且含 B 的第三类买卖点、`c` 创新高/低、`b` 级别不大于 `c` | 两段可比较力度；正名为盘背仍须创新高/低 | `structural_predicates[]`、`extreme_proof`、`third_class_proof?` | 037:18、037:20；044:234 |
| `E2E-D4` | 力度与代理 | 本体是后段力度弱于前段；度量可用原文允许的代理 | 同样比较后一段与前一段的力度 | `force_measure_id`、`force_evidence`、`proxy_preconditions`、`zero_rule_id?` | 015:38；033:26；MACD 仅为辅助见 024:14、024:50，信号并联见 025:38、027:32 |
| `E2E-D5` | 状态与因果钟 | 行进中可先作背驰段活假设；完成时仍弱才结算，力度反超即失效 | 同样不得把未完成后一段冻结成已成立盘背 | `state: Provisional｜Confirmed｜Invalidated｜Unresolved`、事件五钟 | 061:26；C 完成见 024:24、024:40，结构破坏见 064:28；回中枢是后果见 024:46 |
| `E2E-D6` | 递降、谱系与地板 | 子背驰段落在父背驰段内，逐次向低级别定位；不承诺每个中间级别都有背驰 | 盘背产生的后一段同样是合法背驰段，可进入区间套 | `lineage_id`、`parent_edge`、`divergence_interval`、`floor`、`termination_reason` | 027:22、027:38、027:46；逐级非必然见 032:227；形式地板/类背驰见 065:94、088:196 |

### 共通类型与坐标

- **`event_level`**（工程定义）＝运行该背驰谓词的塔桶/中枢级别，与现行 `events_by_level[event_level]` 同域（现状对象锚见 `nest-tower-native-gap-audit-20260717.md` §2.G7）；`left_move_level/right_move_level` 另存，`bsp_formation_level` 由消费策略另定。037:18 的“`c` 为次级别”描述比较段内部级别，不把事件本身改名为 `c` 的级别。
- **`side` 与 `δ`**（工程定义）：`δ=Long`＝底背驰/买侧，`δ=Short`＝顶背驰/卖侧；谱系边“同方向”严格指父子 `side` 相等。
- **`divergence_interval`**（工程定义）＝后一比较段/背驰段的闭区间；现行 `interval_b` 只是迁移别名，字母 `b` 在此不表示趋势进入段小写 `b`。
- **Pan 多参照**（工程定义）：每一合法比较对产生一个独立事件，多个事件以同一 `candidate_group_id` 归组；默认全部保留，不把一组不同力度证据压成单事件。

### `E2E-O`：端到端算子

对因果前缀 `tower_prefix(as_of)`、上次修订 `prior_state` 与版本化 `ManagedBspPolicy`，定义状态算子：

```text
PriorState {
  state_id, state_as_of, prev_state_id?,
  input_prefix_sha256, config_sha256, policy_sha256,
  event_heads: Map<EventKey, EventRevision>,
  lineage_heads: Map<LineageKey, LineageRevision>,
  bsp_by_key: Map<BspKey, ManagedBspCreation>,
  links_by_key: Map<LinkKey, BspLink>
}

Delta {
  event_revisions[], lineage_revisions[],
  managed_bsp_creations[], bsp_links[]
}

CallHashes {
  prior_slice_sha256, next_prefix_sha256,
  config_sha256, policy_sha256
}

E2E-O(as_of, tower_prefix, prior_state, ManagedBspPolicy)
  -> (Delta, next_state = advance(prior_state, as_of, CallHashes, Delta))
   | InvalidPolicy | InconsistentState | LateAuthorization

ObserveCandidate_at(as_of, tower_prefix, E2E-D1..E2E-D3)
  -> NoCandidate | CandidateObservation

ReviseEvent_at(as_of, prior_event?, observation, E2E-D4..E2E-D5)
  -> EventRevision?                // 规范载荷未变化则无新 revision

BuildLineage_at(as_of, prior_lineage_heads, event_heads)
  -> LineageRevision[]             // 产出 E2E-L，不把 E2E-L 当入参

CloseFloor_at(as_of, prior_lineage_head?, lineage_revision)
  -> LineageRevision?              // floor 随修订落盘

Consume_at(as_of, prior_bsp_by_key, prior_links_by_key,
           closed_lineage, ManagedBspPolicy)
  -> (ManagedBspCreation[], BspLink[])
   | NoConsumption | InvalidPolicy | InconsistentState | LateAuthorization
```

`ObserveCandidate_at` 只负责 `E2E-D1`–`E2E-D3` 与稳定身份；`ReviseEvent_at` 才根据 `E2E-D4`–`E2E-D5` 更新同一个事件。转移表固定如下：无旧事件且 `NoCandidate` → 无输出；有旧事件但候选身份消失 → 新 revision=`Invalidated`；候选存在但后一段未完成 → `Provisional`；后一段已冻结完成且全部前提可证 → `Confirmed`；结构或力度被证伪 → `Invalidated`；所需 provider/规则/数据无法判定 → `Unresolved`。`Confirmed` 只允许在上游比较段身份已冻结时产生；`Unresolved` 可在后续前缀继续修订但不可消费。`first_provable_at` 只是 `E2E-D1`–`E2E-D4` 在某前缀首次同时可证的时刻，不等于 `confirmed_at`；旧修订保留，禁止用删除模拟失效。

**修订协议（工程定义）**：输出 `Delta` 只含本次追加量，不是截至 `as_of` 的全量快照；`next_state` 只能由 `advance(prior_state, as_of, CallHashes, Delta)` 得到。`prior_state.state_id` 必须按 §6.1 `StateKey` 重算一致。推进时，`prior_slice_sha256` 必须从本次 `tower_prefix` **重新截到旧 `state_as_of`** 后计算并等于 prior 保存值；`next_prefix_sha256` 则计算到本次 `as_of`，写入 next state。配置与政策摘要必须与 prior 相等；任一不符即 `InconsistentState`、零 Delta。除 Genesis 外要求 `as_of ≥ state_as_of`；`as_of = state_as_of` 时还要求两个前缀摘要相等，并强制 `Delta=∅、next_state=prior_state`；`as_of > state_as_of` 时新状态令 `prev_state_id=prior_state.state_id`。新 key 从 `revision=1`、`supersedes_revision=null` 起；同 key 的**业务载荷投影**变化才追加 `revision=n+1`，并令 `supersedes_revision=n`、`revision_at=as_of`，投影相同则零输出。同一次 `E2E-O` 内各子步骤的中间态先合并，每个 key 至多输出一个最终 head revision，禁止把内部流水步数变成额外修订。事件允许 `∅→Provisional/Unresolved/Confirmed`，允许 `Provisional/Unresolved→Provisional/Unresolved/Confirmed/Invalidated`；同状态仅在业务载荷变化时修订；`Confirmed/Invalidated` 对同一 key 为终态。终态后若上游身份或规则版本改变，必须生成新 key，禁止复活旧 key。`observed_at/first_provable_at` 一旦首次写入即不后移，`confirmed_at/invalidated_at` 只在首次进入对应终态时写入。

`BuildLineage_at` 可为低延迟生成 `Open` 谱系修订，但只有全部被消费节点均为 `Confirmed` 且 `CloseFloor_at` 非 `UnresolvedFloor` 时才转为 `Closed`；任一必需节点失效则谱系转 `Invalidated`，证据不足则转 `Unresolved`。同一不可变路径 key 允许 `∅→Open/Unresolved`、`Open/Unresolved→Open/Unresolved/Closed/Invalidated`，`Closed/Invalidated` 为终态；新增子节点或类背驰定位尾端属于路径扩展，生成带 `extends_lineage_key` 的新 `LineageKey`，不是篡改旧路径。`Consume_at` 只接收 `Closed` 谱系，并以旧 BSP/链接映射保证同一投影幂等。

**教义边界**：趋势比较对象是 `c/b`（031:883、033:26），盘背比较对象是同一中枢前后两段（027:20、061:26）；两者形成的后一段均在「背驰段」定义域内（027:22）。区间套递归的是父背驰段内部的背驰段（027:38、027:46），并非回试段。**工程定义**：所有阶段读取同一个 `as_of` 前缀；`BuildLineage_at` 产出有逐边见证的谱系；`Consume_at` 在同一生产事务中授权受管 BSP 形成并写入双向链接，不是给既有 BSP 事后贴标。原文没有规定 Rust 中 BSP 与证书的数据流，后一句是项目路线冻结，不冒充教义。

---

## 2. 回拉 0 轴语义判定

### 2.1 判定

1. **在 MACD 代理内部，它是前提门，不是可选加分项。**原文逐字说「用 MACD 判断背驰首先要有黄白线对 0 轴的回拉」（025:761），并用「不是柱子缩短就是背驰」反问同一前提（025:296、025:339、025:345）。因此：

   ```text
   MacdWeak
     = ZeroPullback
     ∧ (DiffDeaFailsNewExtreme ∨ HistAreaSmaller ∨ HistHeightFailsNewExtreme)
   ```

   后半的“或”由 025:38 与 027:32 直接支持；仅 `HistAreaSmaller` 不能冒充完整 MACD 口径。

2. **它不是背驰本体的普遍定义。**MACD 被原文定为「辅助判断」「不绝对精确」（024:14、024:50），不用 MACD 还可比较均线面积，且「还有很多方法」（040:34）。所以 `force_measure_id != MACD` 时，`ZeroPullback` 必须记为 `NotApplicable`，不得拿 MACD 专属前提否决另一种已声明的力度代理。

3. **它不能选择比较对象。**原文顺序是先由中枢与走势确定哪两段，再选择何周期的 MACD 辅助（050:18）。因此回拉 0 轴只属于 `E2E-D4.proxy_preconditions`，不属于 `E2E-D2.pair_proof`。

4. **趋势/盘背不因有无 0 轴图形而互换身份。**趋势身份由 037:16、037:18、037:20 的结构合取决定；盘背身份由同一中枢前后段比较及其新极值条件决定（027:20、044:234）。024:40 的标准盘背例确有 B 段回拉 0 轴，但该图形不能补出第二个同级别中枢。

5. **线段下例外不反向放宽形式背驰。**056:54、056:60 的 1 分钟以下实例把无内部结构对象当作线段并只比较柱面积，而 065:94 明定线段以下属于类背驰、与背驰是两回事。原型把前者登记为 `QuasiDivLocalization` 力度证据，不让它绕过形式背驰的代理前提。

### 2.2 原文未决与原型纪律

- 「回拉到 0 轴附近」的数值容差、观察窗、必须穿越还是只需接近，原文只给“附近/回拉”的语义（024:24、025:761），**原文未决**；禁止虚构百分比或阈值并倒签成教义。
- **工程冻结**：原型字段取四态 `Present｜Absent｜Undecidable｜NotApplicable`。选择 MACD 代理时，`Absent` 使该 MACD 见证 `Invalidated`，`Undecidable` 使其保持 `Unresolved`，二者均不得产出 `Confirmed`；具体容差须由另立工程裁定提供 `zero_rule_id`，并进入事件证据。

---

## 3. 比较对象唯一性

### 3.1 趋势背驰：语义对象唯一

在已确定的 `a+A+b+B+c` 趋势与最后中枢 B 下，比较对只能是 `c` 对 `b`：031:883 逐字为「cb 间的比较」，033:26 再述「c 段的力度比 b 的小」。024 课的 A/B/C 记号中，A 是进入最后中枢前的同向段、C 是离开段（024:22、024:24、024:28），与 `b/c` 是同一对对象；它不是趋势首段 `a` 对 `c`。比较错到 108-109 的实例也被 064:28 明确否掉。

**原型约束**：`kind=Trend` 时，`pair_proof` 必须同时指向最后中枢 B、进入段 `b` 与离开段 `c`；缺任一身份边即 `Undecidable`，不得由 MACD 图形反推一对方便比较的段。

### 3.2 盘背：给定中枢/分解后唯一，全局选择不唯一

给定一个中枢与一次同级别分解，盘背比较该中枢前后的同向段，即第一/第三段（027:20）或 `a+A+b` 的 `b/a`（037:148、037:152）；061:26 的统一表述是“围绕一中枢的两段走势”。但当多个历史同向段都可作参照时，原文只说最近一段是「最标准的情况」（049:36、049:38），并明确「不是光比较最近这一段」（080:364）。因此“全局唯一的 Pan A 锚”**原文未决**，任何 `Sel_Θ` 都只能是工程策略。

**原型约束**：同一 Pan 上下文的每个合法比较对各产一个 `DivergenceEvent`，共享 `candidate_group_id`；默认输出组内全部合法见证。若消费者要求单一见证，必须同时保存 `selection_policy_id`、排序键与未选 `EventKey`，不能把 DFS 顺序伪装成教义唯一性。这一约束对应路线图 `E2E-N6`。

### 3.3 共同顺序

两类对象都遵守 `结构定对象 → 力度代理取证 → 状态结算`；“先定两段、后选 MACD 周期”是原文明确顺序（050:18）。因此 `E2E-D2` 的稳定对象身份是 `E2E-D4` 可比性的前置条件，不能以面积较小者倒推对象。

---

## 4. 滞后与降延迟

### 4.1 因果状态机

下表的状态名、字段名与禁止项均为**工程定义**；“教义锚”只说明为何需要相应阶段，不表示原文使用了 `first_provable_at` 等软件术语。事件验收钟固定为五个：`observed_at / first_provable_at / structure_end_at / confirmed_at / invalidated_at`；谱系另有 `opened_at / closed_at` 两钟；`postcondition_at?` 是可选诊断钟，不参与事件是否确认。

| 阶段 | 原型状态/时钟 | 可做什么 | 不可做什么 | 教义锚 |
|---|---|---|---|---|
| P0 进入候选段 | `Provisional` / `observed_at` | 先假设进入背驰段，并立即观察内部结构 | 把未走完的段冻结为最终背驰 | 061:26 |
| P1 首次可证 | `first_provable_at` | 用当下力度或部分柱面积外推形成可证但可撤销的信号 | 读取未来完成段后回填早钟 | 即时平均力度见 015:42；面积乘 2 见 024:28 |
| P2 区间套定位 | `LineageRevision.opened_at` | 在父候选仍行进时并行向低级别寻找内部背驰 | 等父级反弹后才开始向下找 | 061:26、061:32、061:34；精确点后才反弹见 029:18 |
| P3 结构结算 | `Confirmed` / `structure_end_at`、`confirmed_at` | 后一段完成/结构被破坏时复核力度仍弱，再闭合节点 | 用“曾经弱过”替代完成时复核 | 完成口径见 024:24、024:40；结构破坏见 064:28 |
| P4 证伪 | `Invalidated` / `invalidated_at` | 力度反超前段或小级别延伸使大级别摆脱背驰时撤销 | 保留僵尸证书 | 061:26、043:30 |
| P5 后果观察 | `postcondition_at?`（可选诊断钟） | 检查回中枢等定理后果、做归因审计 | 把后果倒置为确认前提 | 024:46、029:16、029:30 |

原文在操作层明确反对等待事后确认（025:126），同级别顶背驰可直接卖出（044:32）；同时又要求活假设可被力度反超证伪（061:26）。所以“低延迟”与“可撤销”必须同时存在：只保留前者会把活假设伪装成不可撤销确认，只保留后者而等回中枢才发信号则是人为滞后。

### 4.2 合法降延迟与非法回填

- **合法**：同一因果前缀内做部分面积外推（024:28）、计算即时平均力度（015:42）、父段行进时同步递降（061:26、061:34）、把回中枢留作后果审计（024:46）。
- **非法近似**：用终态 `divergence_interval`（现行名 `interval_b`）几何配 prefix 首见钟、以未来完成的比较对象回填 `first_provable_at`、丢失 `Invalidated` 路径、或把回试段吞入父背驰段。当前“终态几何 + prefix 首证钟”缝合事实见 `nest-tower-native-gap-audit-20260717.md` §3；它可作对照实验，不可称全程因果原生。
- **原文未决**：各代理的统一风险预算、实时交易撤单协议及数值阈值。本文只冻结事件语义与钟，不把操作参数伪装成原文。

---

## 5. 背驰 vs 类背驰（`E2E-F`）

| 对象 | 概念域 | 比较/定位 | 可产出 | 不可冒充 | 教义锚 |
|---|---|---|---|---|---|
| 趋势背驰 | 至少两个同级别中枢的趋势 | `c` 对 `b`，满足 037:16、037:18、037:20 合取 | `DivergenceEvent(kind=Trend)`；严格第一类买卖点来自趋势背驰 | 盘背、类背驰 | 027:14、027:66；037:16、037:18、037:20 |
| 盘整背驰 | 单一中枢/中枢震荡 | 同一中枢前后段；后一段可定义为背驰段 | `DivergenceEvent(kind=Pan)`，可进入区间套；多数二、三类点与盘背有关 | 严格第一类点；线段下类背驰 | 027:18、027:20、027:22、027:66；盘背区间套实例见 099:192 |
| 类背驰 | 线段及其下、尚无最低级别中枢/形式走势类型的域 | 类似力度比较，服务最后缩点 | `QuasiDivLocalization`，只能作为已存在谱系的定位尾端 | 形式背驰证书、同级 BSP、缺失中间级别证书 | 065:94、066:198；线段内笔间最后一重见 088:196 |

### `E2E-F`：分支本地地板

```text
FloorLocal(branch) =
  FormalFloor(level, event_id)       当前节点已在塔声明的最低形式概念层；
  QuasiFloor(level, localization_id) 当前前缀在该层线段内部完成了类背驰缩点；
  UnresolvedFloor(reason)            尚在更高层却无当前可证子事件，或结构/数据不足。
```

065:94 直接把形式背驰的概念存在系于最低级别中枢，并把线段下类背驰与背驰分开；088:196 又证明区间套的最后一重可以落到线段内部笔间。因此本地地板不是“到 L0 一律算背驰”，而是“到形式概念域边界后换型”。**工程冻结**：当前塔以 L0 线段起塔，故映射为 `L1 = FormalFloor`、`L0 = QuasiFloor`（现状对象锚见 `nest-tower-native-gap-audit-20260717.md` §1）；类背驰只作已闭合形式谱系的可选定位尾端，不单独授权同级 BSP。若塔的基底编码改变，应随对象能力重算地板，不应硬编码沿用数字。

`CloseFloor_at(as_of)` 不判断“未来不会再出现子事件”：节点高于 `L1` 而当前前缀没有可证子事件时，只能给 `UnresolvedFloor`，且 `Consume_at` 必须拒绝；只有节点已到静态能力边界 L1，或 L1 后在同一前缀得到 L0 类背驰定位，才能因果闭合。这样地板判断由塔的概念能力与当前证据决定，不读未来。

`FormalFloor` 闭合不宣称未来不会再出现可选的 L0 类背驰定位：若以后出现该定位，则以 `localization_tail_key` 生成一条带 `extends_lineage_key` 的新谱系，原 Formal 谱系及其形成钟不回写。需要 L0 精定位的消费规则必须令 `match(..., floor_class=Quasi)`；只允许 Formal 的规则可以更早消费。这样“等待类背驰”是显式政策选择，不潜伏成未来函数。

原文不保证相邻每一级都有对应背驰（032:227），所以 `E2E-L` 允许 `parent_level > child_level + 1` 的显式 skip edge；skip edge 记录事实，不自动补中间节点。是否要求某类生产 BSP 必须经过哪些特定级别，是**工程裁定**，原文未给连续级别清单。

---

## 6. 端到端原型 vs 近似（含缝合线实证清单）

### 6.1 `E2E-L`：一等谱系最小对象

```text
TowerDivergenceEvent {
  event_key_version, event_key, event_id, revision, supersedes_revision?,
  predicate_rule_id, predicate_rule_version,
  candidate_group_key, candidate_group_id, event_level, kind, side,
  center_ids, left_move_id, left_move_level,
  right_move_id, right_move_level, divergence_interval,
  domain_proof, pair_id, pair_proof,
  structural_predicates, extreme_proof, third_class_proof?,
  force_measure_id, force_evidence,
  proxy_preconditions, zero_rule_id?,
  state, revision_at, observed_at, first_provable_at,
  structure_end_at?, confirmed_at?, invalidated_at?
}

TowerDivergenceLineage {
  lineage_key_version, lineage_key, lineage_id, revision, supersedes_revision?,
  lineage_rule_id, lineage_rule_version,
  extends_lineage_key?, root_event_id, node_event_keys[], edge_keys[],
  revision_at, opened_at, closed_at,
  floor, termination_reason,
  selection_policy_id?, selection_policy_version?, unselected_event_keys[],
  status
}

ManagedBspPolicy {
  policy_id, policy_version, projection_version,
  rules[] {
    rule_id,                        // policy 内唯一
    match(kind, side, event_level, floor_class),
    point_class, bsp_formation_level,
    anchor_rule: TerminalEventEnd | QuasiLocalizationEnd,
    exact_count: 1                 // WireV1 唯一允许值
  }
}

ManagedBspCreation {
  bsp_key_version, bsp_key, bsp_id,
  point_class, bsp_formation_level, side, source_index,
  formed_at, formation_tx_id, authorization_keys[]
}

BspLink {
  link_key, projection_key, formation_tx_id,
  lineage_key, lineage_id, bsp_key, bsp_id,
  policy_id, policy_version, projection_version, rule_id, slot
}
```

**WireV1 工程冻结**：跨实现只允许 `CanonicalV1(x) = "v1:" + lowerhex(sha256(RFC8785-JCS(x)))`。被哈希的 `x` 必须是下列固定长度 JSON 数组，不得改成对象。所有 ID/规则名是 NFC 规范化 JSON 字符串；所有版本、level、bar、`as_of`、revision、`source_index`、slot 是无前导零的非负十进制 JSON 字符串（零只写 `"0"`），禁止 JSON 浮点数；可选位必须保留并写 `null`，不得缩短数组。枚举字面量固定为 `"Trend"/"Pan"`、`"Long"/"Short"`、`"Contains"/"Skip"`、`"Formal"/"Quasi"/"Unresolved"`。列表除下述结构顺序外，一律按其元素 RFC 8785 规范字节的无符号字典序排列；重复项不得去重。

- `CandidateGroupKey=["CG1",predicate_rule_id,predicate_rule_version,event_level,kind,side,ordered_center_ids,right_move_id]`；`ordered_center_ids` 只按走势结构从旧到新排列，同起点歧义即 `InconsistentState`。`CandidateGroupId=CanonicalV1(CandidateGroupKey)`。
- `PairKey=["PAIR1",CandidateGroupKey,left_move_id]`，`PairId=CanonicalV1(PairKey)`；`EventKey=["EV1",CandidateGroupKey,left_move_id,PairId]`，`EventId=CanonicalV1(EventKey)`。状态、钟与行进中的区间不进入事件身份键。
- `LocalizationKey=["LOC1",side,enclosing_segment_id,terminal_stroke_id,source_index]`。`FloorKey` 只作修订载荷且固定四位：Formal=`["Formal",level,event_key,null]`，Quasi=`["Quasi",level,LocalizationKey,null]`，Unresolved=`["Unresolved",null,null,reason_code]`；`reason_code` 来自规则版本冻结的 ASCII 闭枚举。
- `EdgeKey=["EDGE1",parent_event_key,child_event_key,parent_level,child_level,edge_kind]`。`ordered_EventKey_path` 按 root→leaf，`ordered_EdgeKey_path` 与相邻节点一一对应。`LineageKey=["LIN1",lineage_rule_id,lineage_rule_version,selection_policy_id_or_null,selection_policy_version_or_null,ordered_EventKey_path,ordered_EdgeKey_path,localization_tail_key_or_null]`，不含 Floor/status/clocks/revision；`LineageId=CanonicalV1(LineageKey)`。
- `BspKey=["BSP1",bsp_formation_level,point_class,side,source_index]` 是全局点身份，`BspId=CanonicalV1(BspKey)`。每个谱系/规则授权实例另取 `ProjectionKey=["PROJ1",LineageKey,policy_id,policy_version,projection_version,rule_id,"0"]`；每条规则只产一个 projection，需要多个点必须拆成不同 `rule_id`。`LinkKey=["LINK1",BspKey,ProjectionKey]`。`FormationTxKey=["TX1",as_of,BspKey,ordered_ProjectionKeys]`，`formation_tx_id=CanonicalV1(FormationTxKey)`。

路径新增事件或增加 localization tail 时，只有谱系规则与选择政策 ID/版本完全相同，才令新对象的 `extends_lineage_key` 指向唯一最长 proper-prefix key；否则为 `InconsistentState`。选择政策换版生成新 `LineageKey`，但 `extends_lineage_key=null`，不得在多个旧政策键中自行挑父键。完全相同的不可变路径与政策才使用 revision。

业务载荷投影也由 WireV1 冻结：EventPayload 是 `TowerDivergenceEvent` 按字段列序去掉 `revision/supersedes_revision/revision_at` 后的固定数组；LineagePayload 同理；CreationPayload 是 `ManagedBspCreation` **全部字段**按 schema 列序形成的固定数组；LinkPayload 是 `BspLink` **全部字段**按 schema 列序形成的固定数组。`structural_predicates/proof` 集按 `(predicate_or_proof_type,provider_id,evidence_key)` 规范字节排序，`node_event_keys/edge_keys` 保持 root→leaf，`authorization_keys/unselected_event_keys` 按规范字节排序。四种 `*PayloadHash` 均严格为 `CanonicalV1(["PAYLOAD1",payload])`，不存在另选排除字段的自由。`StateKey=["STATE1",state_as_of,prev_state_id_or_null,input_prefix_sha256,config_sha256,policy_sha256,ordered_[EventKey,revision,EventPayloadHash],ordered_[LineageKey,revision,LineagePayloadHash],ordered_[BspKey,CreationPayloadHash],ordered_[LinkKey,LinkPayloadHash]]`，`state_id=CanonicalV1(StateKey)`。Genesis 唯一取 `state_as_of=null`、`prev_state_id=null`、空输入前缀 SHA-256、给定配置/政策摘要及四个空映射；只有 Genesis 允许 null 时钟，首次调用从它推进到首个 `as_of`。这一定义同时冻结 no-op 比较投影与唯一 prior-state 链。

**消费合法性（工程冻结）**：`ManagedBspPolicy` 内 `rule_id` 必须唯一、`exact_count` 必须严格等于 1、不得匹配 Unresolved floor；多个点用多个唯一规则表达。`QuasiLocalizationEnd` 只可与 Quasi floor 配对且必须存在 localization tail，`TerminalEventEnd` 必须存在路径末节点区间。`projection_version` 或投影语义一变必须提升 `policy_version`；同一 `policy_id/version` 只能对应一个 `policy_sha256`。任一违反即整个调用 `InvalidPolicy`、零 Delta，不能降格为 `NoConsumption`。`formed_at` 固定等于本次 `as_of`。

每条 `edges[]` 保存 EdgeKey、同 `side` 见证及 `child.divergence_interval ⊆ parent.divergence_interval` 的闭区间证据。对 effective next-state 中每个匹配规则的 `Closed` 谱系，`TerminalEventEnd` 取路径末节点 `divergence_interval.end`，`QuasiLocalizationEnd` 取 localization tail 的 `source_index`；每个 slot 产生一个 ProjectionKey 与目标 BspKey。随后按 BspKey 分组：同一点只创建一个 `ManagedBspCreation`，其 `authorization_keys[]` 是该组全部有序 ProjectionKey，并为每个 ProjectionKey 在同一 `formation_tx_id` 下写一个 BspLink。若 prior state 已有该 BspKey，全部期望 LinkKey 也已存在才是幂等零输出；只要出现新缺失授权即 `LateAuthorization`、整次零 Delta，禁止事后补链；creation/link 不一致则 `InconsistentState`。只有本次没有任何 ProjectionKey 才是合法 `NoConsumption`。`consumed_bsp_ids[]` 是按 LinkKey 排序的反向索引视图，不回写终态 LineageRevision。类型边界不得由策略推翻：严格第一类点只由趋势背驰构成（027:66），Pan 只能进入政策明列的 Pan/二三类或类一通道（盘背与二、三类及类一关系见 027:18、027:66），Quasi 不得授权形式 BSP（065:94）。

### 6.2 判定表

本节术语只描述现状缝合线：`C2 seam`＝默认关闭的 `level_view` 候选提供接口；`typed/旧 Cand`＝结构宽候选与旧力度确认候选两套语义；`sidecar`＝不被塔主分类路径消费的旁路装配；`Sub`＝跨级背驰段闭区间包含；`bin book`＝p92 二进制内登记 prefix 首证钟的临时账；`Classification`＝塔主分类输出；`ManagedBspPolicy` 的“受管 BSP”＝其 `rules[].point_class` 明列、必须由谱系授权形成的点，未列点类不在本原型消费断言域；“关闭臂/开放臂”＝同输入同配置下仅关闭/开放原生谱系消费的 control/treatment 两臂。

| 维度 | 端到端原型 | 只能称近似 | 当前审计锚 |
|---|---|---|---|
| 谓词 | `E2E-D1`–`E2E-D5` 在塔主路径只有一套语义 | typed/旧 Cand 各自定义，靠后续对账解释 | gap audit §2.G3、N0 |
| 事件 | 每级原生产出稳定事件、比较对与 `divergence_interval` | C2 seam/sidecar 临时提供 `seg_c/interval_b` | §4、N1 |
| 跨级边 | 塔对象上直接保存包含见证与 skip 级别 | `is_sub` 只在 nest 装配时计算 | §2.G4、N2 |
| 身份 | `EventId/LineageId/BspId` 双向可追 | `source_index == turn_source` 等值拼接 | §2.G1、N4 |
| 时钟 | 几何与所有钟来自同一因果前缀 | 终态几何与 prefix 首见钟拼合 | §3、N5 |
| 谱系 | `E2E-L` 是 `Classification` 原生输出 | `NestCertificate` 是事后 sidecar | §2.G8、N3 |
| 地板 | 每分支显式 `Formal/Quasi/Unresolved` | 无类型地截在 L0/L1，或把类背驰反注为背驰 | §5 与本文 `E2E-F` |
| 消费 | BSP 级别形成消费已闭合谱系并留下稳定反向边 | BSP 先产、证书事后标注 | §5.N7 |

路线图 `E2E-N0`–`E2E-N7` 给出从这些缝合线迁到完成态的逐项映射。只把 `nest.rs` 搬进塔目录、只共享 helper、或只让 sidecar 输出字段改名，均未改变对象所有权与因果数据流，仍是近似。

### 6.3 缝合线实证清单（`E2E-S1`–`E2E-S8`）

每次验收必须落一个可复跑证据包 `chanlun/review-results/e2e-native/<run_id>/`；缺任一清单项则该次验收无效：

```text
manifest.json
  run_id, worktree_head, dirty_file_sha256[], input_uri, input_sha256,
  oracle_name, oracle_file_sha256[], native_file_sha256[],
  policy_id/version, policy_file, policy_sha256,
  validator_file, validator_sha256, config_canonical_json,
  levels = [有限枚举], as_of_set = [有限枚举],
  as_of_set_source = union(oracle_changes,native_changes,ckpt,{0,final}),
  normalization_version, normalization_file, normalization_sha256,
  control_schema_uri, control_schema_sha256,
  closed_arm_fields_file, closed_arm_fields_sha256,
  eligible_projection_count, expected_artifacts[]
commands.txt                 // 每条命令、cwd、退出码；实现后按此逐字复跑
normalization.json           // 只列排序、单射字段映射、本文闭名单忽略字段
closed-arm-fields.json       // 从 control schema 机械枚举的叶字段全名单
closed-arm-completeness.json // schema 叶字段集与 fields 文件的双向差集
s1_predicates.jsonl
s2_events.jsonl
s3_edges.jsonl
s4_lineages.jsonl
s5_clocks.jsonl
s6_floors.jsonl
s7_identity.jsonl
s8_consumption.jsonl
s8_expected_projection.jsonl
summary.md                   // 各集合基数、双向差集、失败样本主键
```

`levels` 必须列出输入上全部可达塔级别。先让 oracle/native 各自全前缀枚举状态变化 bar，再取两者并集，加既有 CKPT 与 `{0, final}`，冻结为同一个 `as_of_set` 后重跑两端；小型 fixture 另跑全部前缀。全量基线必须记录 `/tmp/p92_ckpt_dump.txt`（若使用）的内容摘要与 oracle 源文件摘要，不能只写“66 张”。

规范化忽略字段闭名单只允许 `diagnostic.wall_clock_ns / diagnostic.absolute_path / diagnostic.process_id / diagnostic.log_offset`；任何 `E2E-*` 身份、证据、状态、钟、失败、重复数与消费字段均不可忽略。字段映射必须一一单射到本文 canonical schema，排序只可按 WireV1 key；`normalization.json` 不能自行扩充闭名单。`closed-arm-fields.json` 必须由 hash 固定的迁移前 control schema 机械枚举全部叶字段，`closed-arm-completeness.json` 的双向差集必须为空；schema、字段表、validator 的路径与摘要都进入 manifest。关闭/开放臂的唯一变量必须是 manifest 中的谱系消费策略开关；`ManagedBspPolicy` 缺失即 `E2E-S8` 自动失败。

`E2E-S8` 的重算联结也固定：`s4_lineages.jsonl.node_event_keys[]` 只存 EventKey 引用；validator 对每个 `as_of` 分别从 `s2_events.jsonl` 与 `s4_lineages.jsonl` 选择满足 `revision_at ≤ as_of` 的最高 revision 作 head，每个 node EventKey 必须恰好联结一个事件 head，否则验收失败。规则匹配取该事件的 `kind/side/event_level`，`TerminalEventEnd` 取其 `divergence_interval.end`；floor 与 Quasi localization tail 取 lineage head。不得仅凭 `s4` 猜事件字段，也不得接入清单外旁路数据。

| 编号 | 必须提交的实证 | 通过判据 | 对应迁移 |
|---|---|---|---|
| `E2E-S1` | `s1_predicates.jsonl` | manifest 全部 `levels × as_of_set × CandidateGroupKey` 上，Trend/Pan 的 `E2E-D1`–`E2E-D5` 逐门值与状态转移双向零差；不得只测 ℓ=0 | `E2E-N0` |
| `E2E-S2` | `s2_events.jsonl` | 规范 `EventKey` 多重集及各 revision 的 `kind/side/divergence_interval/state` 双向零差；重复数一致，不比较临时 ordinal | `E2E-N1` |
| `E2E-S3` | `s3_edges.jsonl` | 规范父子 EventKey、实际级别、side、Contains/Skip 种类及闭区间包含证据双向零差 | `E2E-N2` |
| `E2E-S4` | `s4_lineages.jsonl` | 规范 `LineageKey` 多重集、全部节点/边/未选见证双向零差；现有 66 张（A=41/B=25）仅作带摘要快照的历史基线 | `E2E-N3`、`E2E-N6` |
| `E2E-S5` | `s5_clocks.jsonl` | manifest 的全部前缀上，事件五钟与谱系 `opened/closed` 逐修订相等且均不大于产生该修订的 `as_of`；`postcondition_at?` 若输出则单列诊断对拍 | `E2E-N5` |
| `E2E-S6` | `s6_floors.jsonl` | `s4` 中每条分支恰有一个 Formal/Quasi/Unresolved；L0 形式背驰证书为 0，Unresolved 消费数为 0，Quasi 不增加同级 BSP | `E2E-F` |
| `E2E-S7` | `s7_identity.jsonl` | Event/Lineage key 不得对应不同业务载荷；每个 ProjectionKey 恰连一个 LineageKey 与一个 BspKey，每个 LinkKey 唯一且无 orphan；同一 BspKey 可有多个 ProjectionKey，但必须恰等于唯一 creation 的 `authorization_keys[]`，未选 EventKey 仍可追 | `E2E-N4`、`E2E-N6` |
| `E2E-S8` | `s8_consumption.jsonl` + `s8_expected_projection.jsonl` | 关闭臂对完备性已证的 `closed-arm-fields.json` 全字段逐字节相等；validator 按上段规则联结 `s2 + s4 + policy_file`，独立重算 ProjectionKey 集并断言 `eligible_projection_count = |ExpectedProjectionKey| > 0`，再按 BspKey 分组，要求 distinct ExpectedBspKey 集与 creation 集、Expected LinkKey 集与 link 集均双向零差，组内 `formation_tx_id` 唯一；零值只能记 `NotExercised` | `E2E-N7` |

八项全部通过，才可把声明从「独立 nest 对照实现」升级为「塔内端到端原生」。任一项缺证，只能写明具体缝合线与近似范围；090 纪律下禁止用“逻辑等价”“目录已合并”或少量样本绿替代逐字段实证。

### 6.4 收口判定

- **端到端原型**：`E2E-O` 的每一步都由塔拥有输入、对象、因果钟与稳定身份，`E2E-F` 有类型地终止，`E2E-L` 被生产 BSP 消费，并通过 `E2E-S1`–`E2E-S8`。
- **当前近似**：依据 `nest-tower-native-gap-audit-20260717.md`，事件区间、Sub、首证钟、证书与 BSP 消费分别跨 C2 seam、bin book、nest sidecar 与 runner 环境门拼接；它是有价值的独立 oracle，但还不是塔内原生。
- **原文未决**：0 轴数值容差、多个 Pan 参照段的唯一选择器、非 MACD 代理之间的支配序、生产侧对 skip edge 的具体消费政策。本文分别用显式规则 ID、保留全部见证、代理声明与 typed skip edge 承载这些未决，不虚构课号。

### 6.5 边界与纪律声明

- 本文只收束定义与原型，不授权 Rust 实装，不运行 cargo，不重放数据，不修改 M7/M8 验收列。
- 教义权威只取博文；工程事实只复用五份既有审计/考据，不把代码现状写成教义。
- 路线图端到端定义、迁移表与本文编号已逐项对齐；后续评审应按 `E2E-*` 编号提异议，避免同名异义。
