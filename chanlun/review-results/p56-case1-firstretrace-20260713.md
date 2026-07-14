# P56 例 1 案卷：严格 `firstRetrace` 违背候选复核

- 日期：2026-07-13
- 对象：`level=1 / B=L2#777 / cp_departure=L1#3456`
- 性质：P0 裁决材料；**只给倾向，不下最终裁决**
- 代码边界：#54 案例产生基线 `3d162754bf`，审计提交 `29fb04c129`；#54 明示未修改 `judge_third_cert`、`cand_delta`、事件集或定义（`/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md:3-9`）。
- 标签纪律：`【已验证】`只用于文件、生产分支或本次同输入重放直接支持的事实；`【推断】`用于语义归纳、对象身份解释与裁决倾向。

## 0. 结论摘要（非最终裁决）

1. **【已验证】首对不是方向或边界伪冲突。** `L1#3456 → L1#3457` 是 `Down(anchor=Down) → Up` 的合法卖侧离开/回抽：离开端点 `10136.82 < ZD=10424.00`；回抽端点 `10424.72 >= ZD`，进入闭中枢 `[10424.00,10491.32]`，故在 `bar=1626497` 确切触发 `RETEST_REENTERS`。#54 全量审计同时验证本批 `boundary_equalities=0`、`anchor_direction_diffs=0`（`/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md:13-16,548-562`）。
2. **【已验证】“后续对成功”实际经过三对，而非首对后立即成功。** 时间序为：`3456→3457: RETEST_REENTERS`，`3457→3458: ANCHOR_NONE`，`3458→3459: SUCCESS`。成功对是新的买侧几何：`Up(anchor=Up) → Down`，离开 `10795.24 > ZG=10491.32`，回试 `10572.00 > ZG`，在 `bar=1627583` 成功。
3. **【已验证】生产对象没有随新离开重建身份。** 最终证书的实际 `departure_move_id=L1#3458`，但所属对象仍是原 `CpId=(level=1,B=L2#777,cp_departure=L1#3456)`；#54 报告据此把它列为同一 `CpId` 的 late success（`/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md:330-355,430`）。
4. **【推断】当前最强倾向是“对象身份/firstness 实装偏差”，不是简单宣布后来那段价格几何永远非法。** 若 `firstRetrace` 对同一 `CpId` 全局计数，首对重入已经耗尽资格，晚成功违背定义；若允许“重入后以新方向离开”重启，`3458→3459` 可成为新对象的合法首次回试，但仓库尚无把它从 `cp_departure=3456` 重启为新身份的规则或证书。
5. **【推断】因此不把本案裁成纯“定义未覆盖的合法路径”。** 合法路径解释只能作为待裁的“新离开/新对象”分支；在现有旧身份上静默继续扫描到首次成功，没有满足严格 `firstRetrace` 的证明义务。

## 1. 权威证据

### 1.1 原文、编纂定义与形式化锚

- **【已验证】第三类的基本几何。** 第20课规定：一个次级别走势类型离开中枢，再以一个次级别走势类型回试/回抽，不重新进入 ZG/ZD 才构成第三类（`docs/chanlun/text/blog/020-第20课.md:54-60`）。
- **【已验证】“第一次”是独立必要条件。** 原文紧接着强调“并不是任何回调回抽……必须是第一次”（`docs/chanlun/text/blog/020-第20课.md:60-62`）；chan99 编纂版再次单列该限制（`docs/chanlun/text/chan99/0025-第四节 走势中枢与买卖点.md:31-39,78-82`）。
- **【已验证】仓库严格定义把 firstness 和完成性并列。** `.chanlun/definitions/maimai.md` 要求“第一次离开后的回试”，且离开、回试均为完成的次级别走势类型（`.chanlun/definitions/maimai.md:132-142`）；边界条款又写明后续离开—回试不构成第三类（`.chanlun/definitions/maimai.md:214-219`）。
- **【已验证】Lean 把 `firstRetrace=true` 写成显式合取项。** 买侧、卖侧都同时要求 `leftCenter=true ∧ firstRetrace=true` 与严格价格不等式（`formal/Origin/BspClassification.lean:107-121`）。Lean 这里给出布尔条件，但没有定义“重入后新离开是否创建新的 firstRetrace 身份”。
- **【已验证】确认时点在完成回试。** 编纂定义规定第三类正式确认于回试走势 `Move.settled` 且价格越过 ZG/ZD（`.chanlun/definitions/maimai.md:245-250,264-278`）；原文答疑也明确次级别完成须在再次级别结构上完成（`docs/chanlun/text/blog/020-第20课.md:266-274`）。

### 1.2 本案不受 C 口径分叉影响

- **【已验证】递归定义的基底是 `Segment = Move[0]`，且仓库标注 `Center[1]` 的组件为 Segment/Move[0]。** 锚：`.chanlun/definitions/level_recursion.md:70-84`。
- **【推断】** 本案 `level=1` 的判定元素就是 parser 线段的基底退化，故离开/回试对象不存在 `WindowUnit` 与 `CompletedMove` 的口径落差。C2 对高级层消费者的裁决不会改变本案三条线段 pair 的完成身份；它最多影响以后统一的对象/账本接口。

### 1.3 #54 基线生产判定链

- **【已验证】价格谓词无 first 参数。** `judge_third_cert(c, leave_seg, leave_anchor, retest_seg)` 只接收中枢、离开、方向锚、回试四项（`/tmp/p54-audit-work/rust/src/theta_v0/classifier/signal.rs:399-406`）。买侧成功条件为 `Up/Down ∧ leave>ZG ∧ retest>ZG`，卖侧为 `Down/Up ∧ leave<ZD ∧ retest<ZD`（同文件 `:407-445`）。
- **【已验证】失败不会关闭或消费对象。** `advance_cp_lifecycles` 对每个新到达的相邻 pair 扫描（`/tmp/p54-audit-work/rust/src/theta_v0/classifier/recursive_tower.rs:1698-1718`）；`judge_third_cert=None` 后直接 `continue`（同文件 `:1757-1805`），只有后续 `Some(cert)` 才把对象置为 `Closed` 并写入证书（同文件 `:1806-1849`）。
- **【已验证】#54 诊断只复演失败原子。** 对 `Down/Up`，若 `leave_end >= ZD` 则是 `LEAVE_NOT_OUTSIDE`，否则 `RETEST_REENTERS`（`/tmp/p54-audit-work/rust/src/theta_v0/classifier/recursive_tower.rs:537-560`）；逐 pair 字段与 `judge_at=retest.end_index` 的记录结构见同文件 `:507-520,639-669`。
- **【已验证】当前主工作树仍显示同类局部缺口，但不把它冒充 #54 基线。** 当前 `judge_third` 仍只检查相邻 pair 的方向和 ZG/ZD 几何（`rust/src/theta_v0/classifier/signal.rs:303-350`），外层对每个相邻 pair 独立调用（同文件 `:614-625`）；`EndpointSituation::is_third` 也只合取 `left_center && retrace_not_reenter`（`rust/src/theta_v0/classifier/bsp.rs:27-41,51-65`）。这证明局部谓词本身仍不携 firstness；当前全链是否由别层补足，需另案证明。

## 2. 案例 1 原样重放

### 2.1 输入与复现边界

- **【已验证】输入与 #54 一致。** `analysis/data_cache/btc_1m_full.json` 为 4,613,599 bars，文件大小 `329485099` bytes，SHA-256 为 `16ea13d55f2ae7edcfc503f604a14961fd1a378227afd37c63f23386e894707b`；#54 记录同一 bar 总数、末端 `source bar=4613598`、`ThetaConfig::default()` 与 batch 模式（`/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md:3-8`）。
- **【已验证】价格口径。** #54 基线默认 `tick_size=1e-8`，内部使用整数 Tick（`/tmp/p54-audit-work/rust/src/theta_v0/config.rs:18-31`）。下表同时给出整数 Tick 与 `Tick×1e-8` 的价格。
- **【已验证】本次运行没有修改生产源码。** 先运行既有 release `cp_capability_smoke` 重现 #54 摘要，再以同一已编译库公开的 `cp_recall_upper_bound_audit` 只读打印目标 `pair_history`；该 API 明示绕过事件入口、复用生产几何（`/tmp/p54-audit-work/rust/src/theta_v0/classifier/mod.rs:563-601`）。

### 2.2 固定对象与三条 pair

**【已验证】固定对象：**

| 字段 | 值 | 锚 |
|---|---:|---|
| `B` | `L2#777` | #52 全量表 `/tmp/p54-audit-work/chanlun/review-results/p52-recall-upper-bound-audit-20260712.md:145-153` |
| `B_source` | `[1622008,1625254]` | 同上 |
| `B_core` Tick | `[ZD=1042400000000, ZG=1049132000000]` | 同上 |
| `B_core` 价格 | `[10424.00,10491.32]` | Tick 默认值换算；`config.rs:18-31` |
| `cp_departure / cp_start` | `L1#3456 / 1625292` | #52 同上；#54 late-success `/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md:330-355` |

**【已验证】完整 pair 时间线：**

| 顺序 | leave（source；方向/锚；端点 Tick=价格） | retest（source；方向；端点 Tick=价格） | `judge_at` | 判定 |
|---:|---|---|---:|---|
| 1 | `L1#3456 [1625292,1626098]`; `Down/Some(Down)`; `1013682000000=10136.82` | `L1#3457 [1626098,1626497]`; `Up`; `1042472000000=10424.72` | `1626497` | `RETEST_REENTERS` |
| 2 | `L1#3457 [1626098,1626497]`; `Up/None`; `1042472000000=10424.72` | `L1#3458 [1626547,1626953]`; `Up`; `1079524000000=10795.24` | `1626953` | `ANCHOR_NONE` |
| 3 | `L1#3458 [1626547,1626953]`; `Up/Some(Up)`; `1079524000000=10795.24` | `L1#3459 [1626961,1627583]`; `Down`; `1057200000000=10572.00` | `1627583` | `SUCCESS` |

- **【已验证】摘要交叉检查。** #54 原报告保存了首原子/首时点、成功 pair ID/成功时点（`/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md:330-355`），并明确本案与例 2 是仅有的两条“合法定向重入后同 `CpId` 晚成功”强冲突候选（同文件 `:430`）。本次运行的 `P54_CASE` 还重现 `pairs=3`、成功对全字段和 `third_confirm=1627583`；#54 报告的 full-case 锚为同文件 `:505-517`。

### 2.3 首对 `RETEST_REENTERS` 的逐条件复演

1. **【已验证】对象和时序门通过。** `leave.start=1625292 >= cp_start=1625292`；pair 是相邻完成元素 `L1#3456→L1#3457`；生产路由要求同级、序号不早于 `cp_departure`（`/tmp/p54-audit-work/rust/src/theta_v0/classifier/recursive_tower.rs:1772-1791`）。
2. **【已验证】方向门通过。** `leave_anchor=Down` 且 `retest.direction=Up`，命中卖侧定向分支（`/tmp/p54-audit-work/rust/src/theta_v0/classifier/signal.rs:426-443`）。
3. **【已验证】离开门通过。** `10136.82 < ZD=10424.00`，下破 `287.18`；不是 `LEAVE_NOT_OUTSIDE`。
4. **【已验证】回抽不入门失败。** 卖侧要求 `retest_end < ZD`；实际 `10424.72 = ZD+0.72`，已进入闭中枢，且不是等号。`judge_third_cert` 返回 `None`，诊断复演为 `RETEST_REENTERS`（`/tmp/p54-audit-work/rust/src/theta_v0/classifier/recursive_tower.rs:552-557`）。
5. **【推断】** 若 firstness 绑定同一 `CpId`，资格在 `judge_at=1626497` 已被首个合法定向回抽消耗；之后任何 pair 都不能回写 `firstRetrace=true`。

### 2.4 中间 pair 与后续成功路径

1. **【已验证】第二对不是回试成功候选。** `L1#3457→L1#3458` 的 leave anchor 为 `None`，且两者都是 `Up`；诊断按最前置短路记为 `ANCHOR_NONE`（`/tmp/p54-audit-work/rust/src/theta_v0/classifier/recursive_tower.rs:543-560`）。失败后对象仍 `Pending`，生产循环继续。
2. **【已验证】第三对几何完整成功。** `L1#3458` 向上离开：`10795.24 > ZG=10491.32`（高 `303.92`）；`L1#3459` 向下回试：`10572.00 > ZG`（高 `80.68`）；命中买侧 `Some(cert)` 分支（`/tmp/p54-audit-work/rust/src/theta_v0/classifier/signal.rs:407-425`）。
3. **【已验证】成功时生产写入实际 pair 与旧对象身份。** 证书记录 `departure_move_id=3458`、`retest_move_id=3459`，同时 `CpStructureIdentity.departure_move_id` 仍取原 `cp_departure_move_id=3456`（`/tmp/p54-audit-work/rust/src/theta_v0/classifier/recursive_tower.rs:1806-1826`），随后在 `1627583` 关闭对象（同文件 `:1844-1849`）。
4. **【推断】** 这证明“后来有合法买侧价格几何”，但没有证明“它是原 `cp_departure=3456` 的第一次回抽”。两者是不同命题。

## 3. 三种解释的判定矩阵

| 解释 | 严格口径 | 对本案证据的解释力 | 与权威/代码的冲突 | 暂定评价 |
|---|---|---|---|---|
| A. **定义违背：同一 B/`CpId` 只认首个合法定向回抽** | `3456→3457` 已是 first；其重入后该对象永久不得成 3 类 | 强：首对方向、完成性、价格均已满足“可评估”，只差不重入；晚成功仍挂旧 `CpId` | 原文/Lean/`.maimai` 支持 firstness；但原文未明写“失败后反向新离开是否必须永久禁用同一 B” | **较强**；若 P0 采用全局身份，当前 `SUCCESS` 是定义违背 |
| B. **实装偏差：定义要求 first，但生产实现的是 first success** | 失败 pair 不消费身份，扫描到首个 `Some(cert)` 即关单 | 最强：逐行对应 `None→continue`、后续 `Some→Closed`；`judge_third_cert` 无 first 参数 | 与 Lean 显式合取、编纂定义的 firstness 缺机器桥 | **最强倾向**；这是当前对象层可直接坐实的缺证/偏差 |
| C. **定义未覆盖的合法重启：重入后新方向离开创建新对象** | 首个下离/上抽失败后中枢继续；`3458` 是新上离，`3459` 是该新离开的第一次回试 | 几何上强：第三对确是新方向的直接首次回试；第20课把“离开→其后回试”写成局部组合（`docs/chanlun/text/blog/020-第20课.md:54-62`） | 当前仍沿用 `cp_departure=3456`，没有 `RESTART/SUPERSEDE/new CpId`；`.maimai` 又写“后续离开-回试不构成”（`.chanlun/definitions/maimai.md:214-219`） | **可保留但未证成**；只有先裁出新对象身份，才可称合法路径 |

**【推断】矩阵解释：** A 与 B 不是互斥答案——A 描述输出相对严格定义的状态，B 描述造成该状态的实现层原因。C 则是能挽救后来市场几何的另一条定义扩充路径，但它必须改变对象身份，不能把“继续扫旧对象”直接改名为合法重启。

## 4. 倾向（不下最终裁决）

1. **【推断】对象级倾向：B（实装偏差）优先，A（定义违背）随严格全局身份成立。** 证据已足以否定“现有 `CpId=.../3456` 自带严格 firstRetrace 证书”：谓词不接 first，生命周期失败不消费，成功仍落在旧身份。
2. **【推断】市场结构级倾向：不否定 C 的可能性。** 首次下破回抽重入意味着原中枢未被该方向破坏；之后从中枢内重新向上离开并直接回试不入，具有第三类买点的局部几何。缺的是“重启后谁是新对象”的规范，而不是价格条件。
3. **【推断】若 P0 选 A/B 路线，最小语义要求是：** 首个合法定向 pair 判负后，把旧 `CpId` 标为 first-retrace-consumed/失败，不允许后续 pair 使其 `Closed(SUCCESS)`。
4. **【推断】若 P0 选 C 路线，最小语义要求是：** 在重入后由 `L1#3458` 创建新 departure identity（或显式 `SUPERSEDE/RESTART` 事件），把 `3458→3459` 的成功归给新对象；禁止继续沿用 `cp_departure=L1#3456` 而无桥接证书。#54 谱系记录也把“唯一 pair”与“可证明的新对象重启”列为两个待裁回溯条件（`/tmp/p54-audit-work/.chanlun/genealogy/pending/2026-07-12-p54-third-negation-representative-atom-vs-first-retrace.md:103-118`）。
5. **【推断】在上述身份规则落定并全量重放前，本案应保持“强冲突候选”，不能升级为最终假阳性，也不能升级为已证合法第三类。** 这与 #54 对 93 个 late success 的收窄表述一致（`/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md:581-595`）。

## 5. 例 2 挂起登记（不展开）

- **【已验证】挂起对象：** `level=3 / B=L4#47 / cp_departure=L3#273`。#54 只登记其首原子 `RETEST_REENTERS@2906047`，后续 `L3#276→L3#277` 在 `2943378` 成功；它与例 1 同为两条最强候选（`/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md:414-430`）。本报告不展开其价格、pair 或归属。
- **【已验证】C 已从“待选 C1/C2”裁为 C2。** 需要“完成走势类型”的消费者必须使用 `CompletedMove[k]` 的 as-of 组装视图，塔对象正名为 `WindowUnit[k]`，窗口封闭不得替代走势完成（`chanlun/escalate/c-ruling-decision-20260713.md:9-14`）。
- **【已验证】该决定没有完成例 2 所需重放。** 裁决明确把组装器生产化、#54 计数重放和消费入口迁移留给后续任务（`chanlun/escalate/c-ruling-decision-20260713.md:21-24`）。
- **【推断】挂起状态不变，但挂起原因已收窄。** 例 2 不再等待“选 C1 还是 C2”；它现在等待 C2 组装器生产化后，把 `L3#273/...` 映射为不同、相邻、已完成的 `CompletedMove`，再重放 firstRetrace 与对象重启语义。未完成该映射前，#54 的 WindowUnit pair 不能直接用于最终裁决。

## 6. 裁决者需回答的最小问题

1. **【推断】`firstRetrace` 的计数域究竟是 `(B)`、旧 `CpId=(B,cp_departure)`，还是“每次被证明的新 directional departure identity”？**
2. **【推断】一次合法定向回抽重入后，旧对象应进入永久失败、继续 Pending，还是被 `SUPERSEDE/RESTART`？**
3. **【推断】若允许重启，例 1 的成功应归给 `cp_departure=L1#3458` 还是保留 `L1#3456` 并附重启证书？**
4. **【推断】裁后必须以同一规则全量重跑 #54 的 2,630 对象与 `213/93` 集合；禁止只为两个样本特判。** #54 已明确当前 `U=213` 只是现行生命周期 judge 的几何上界（`/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md:13-19,589-595`）。

## 7. 2026-07-14 复核更新（#71 C2 生产化重放后）

- **【已验证】#71 已完成 C2 组装器生产化重放。** 见 `chanlun/review-results/c2-production-replay-20260714.md`；`cargo test --release` exit=0，lib 1578 passed / 0 failed / 127 ignored，`git diff --check` 通过。
- **【已验证】严格 raw-prefix 下，例 2 两组回抽均未形成已完成 `Move[3]`，未进入严格 C2 pair 域。** exact-three 主树中亦不存在可与 #54 ID 等同的 pair（`c2-production-replay-20260714.md` §7）。
- **【推断】复核意见：维持 p55 §5.3 的“否/未进入严格 C2 合格域”倾向；撤回“history-end 对象按 end 截断即可代表 as-of”的证据方法。** 当前证据不能推翻 firstRetrace 口径，也不能把几何 `SUCCESS` 裁成严格 C2 反例。
- **【推断】挂起状态不变，等待条件收窄为：** exact-three↔当前扩展塔的显式 projection、方向 provider 与 A/C hook 均版本化后，以同一对象宇宙重开复核。Pending/Unassigned 不得重述为几何“回试失败”，不得据此结算 #56。
