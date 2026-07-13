# C 裁决材料：塔「规范窗口分解」语义 vs 定义「走势类型完成」语义

- 日期：2026-07-12
- 任务：#57
- 性质：P0 裁决材料汇编
- 边界：只呈材料，不作 C1/C2 裁决，不修改生产代码或定义
- #54 只读参考：`/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md`

## 0. 一句话问题

当前 Rust 生产塔中的 `Move[k]` 实际是「一个成立的、恰三段次级别单元的规范窗口，经一次 compose 封成的单中枢上级单元」；已结算定义中的 `Move[k]` 则是「包含一个或多个中枢、并按盘整/趋势终结条件完成的走势类型实例」。前者是自洽的窗口递归对象，后者是自洽的走势类型对象；对象偏差从 `WindowUnit[1]` 与 `CompletedMove[1]` 的完成语义就已出现，并在构造 `Center[k>=2]` 时首次成为上级构件身份偏差。本文记录差异、证据、影响和两案代价，不判哪一个应成为最终生产语义。

为避免把争议预先写死，本文使用两个中性记号：

- `WindowUnit[k]`：塔目前产出的窗口单元。
- `CompletedMove[k]`：定义口径下完成的走势类型。

在 C 裁决前，文中的 `WindowUnit[k] != CompletedMove[k]` 表示对象构造规则不同，不表示任一方是 bug。

## 1. 已核实的两套契约

### 1.1 塔的窗口契约 W

塔文件把契约直接锚到 Lean `composeStep` 与走势分解定理二：每个上级单元取连续三段次级别 `LeveledMove`，携一个由该窗口派生的中枢，且 `descend` 可取回这三段。见 `rust/src/theta_v0/classifier/recursive_tower.rs:26-33`。

实现落点如下：

1. 扫描命中时记录窗口 `[i,i+1,i+2]`，游标前进 3；不命中时游标前进 1。见 `rust/src/theta_v0/classifier/recursive_tower.rs:180-205`。
2. `compose_level` 对每个命中窗口只取三段 `subs_moves`，调用 `LeveledMove::compose` 产一个上级单元。见 `rust/src/theta_v0/classifier/recursive_tower.rs:211-249`。
3. `LeveledMove::compose` 写入的 `centers` 是单元素向量 `[center]`。见 `rust/src/theta_v0/classifier/recursive_tower.rs:130-156`。
4. 生产循环明确声明 `upper_moves` 与窗口中枢一一对应，再把这些 `upper_moves` 投影为下一级输入。见 `rust/src/theta_v0/classifier/mod.rs:234-238,262-266`。

Lean 侧也明确采用另一套自洽的窗口族口径：`canonicalWindows` 把序列切成有序、不重叠、每窗恰三段的窗口，每窗封装一个上级走势；`composeStep` 只是对该窗口族做 `Move.compose` 映射。见 `formal/Foundation/ChanlunInstantiation.lean:105-126,135-152`。Origin 重锚材料进一步给出 `9` 段变 `3` 个窗口、再变 `3` 个上级单元的反退化见证。见 `formal/Origin/RecursiveLevelSystem.lean:14-20`。

### 1.2 定义的完成走势契约 D

已结算定义给出的对象图是：`Move[0]=Segment`，`Center[k]` 由三个连续 `Move[k-1]` 重叠构成，`Move[k]` 是包含 `Center[k]` 的走势类型实例，其中盘整含一个中枢、趋势含至少两个中枢。见 `.chanlun/definitions/level_recursion.md:19-27`。

同一文件把「完成」写成：盘整至少一个中枢且被后续走势终结；趋势至少两个中枢且末段出现背驰。见 `.chanlun/definitions/level_recursion.md:70-80`。后文又把递归消费口径结算为 `Move.settled=true`，并明确未结算尾部不进入上级构造。见 `.chanlun/definitions/level_recursion.md:231-242`。

原文固化材料也区分盘整与趋势：完成盘整只有一个中枢，完成趋势至少有两个依次同向且互不重叠的中枢；走势类型至少由三段次级别走势构成。见 `docs/chanlun/text/blog/018-第18课.md:24-36`。第 27 课答疑进一步指出：趋势至少两个中枢，因而至少六段次级别走势，两个中枢不能共用一个次级走势。见 `docs/chanlun/text/blog/027-第27课.md:846-856`。

## 2. 偏差清单

### C-a. 塔 `Move[k]` 是恰三段加一个中枢的窗口单元

`compose_level` 的每个输出固定取命中窗口的三个下级单元，并由单个 `Center` 构造上级对象；输出与命中的中枢一一对应。证据见 `rust/src/theta_v0/classifier/recursive_tower.rs:221-249` 与 `rust/src/theta_v0/classifier/recursive_tower.rs:142-156`。

因此塔中的一个上级元素满足：

```text
WindowUnit[k] = Compose(
  exactly 3 lower WindowUnit[k-1],
  exactly 1 Center[k]
)
```

这不是定义文件的通式 `Move[k]=包含 Center[k] 的走势类型实例（盘整=1，趋势>=2）`。

### C-b. 中枢延伸吸收在塔扫描中明确不存在

塔注释明确将自身与 zhongshu `ScanResume` 区分：`ScanResume` 的 `Unsettled` 中枢会用 `extend` 状态吸收后续段；塔的非重叠三段扫描没有该状态机，成立后 `+3`、永不回头、永不 extend。见 `rust/src/theta_v0/classifier/recursive_tower.rs:262-281`。

所以定义语义中的「盘整达到最小三段后可以继续延伸，直到被终结」没有落在塔单元内部；原文固化材料明确写出盘整达到三段后可以结束，也可以继续延伸。见 `docs/chanlun/text/blog/018-第18课.md:38-40`。塔只知道一个窗口何时成立，不知道同一盘整走势何时继续吸收、何时最终终结。

### C-c. 趋势型 `Move[k]` 的组装缺失

定义要求趋势型 `Move[k]` 含至少两个依次同向中枢；塔的每个输出固定只有 `[center]`。定义证据见 `.chanlun/definitions/level_recursion.md:24-26`，塔证据见 `rust/src/theta_v0/classifier/recursive_tower.rs:142-150`。

第 27 课材料把张力写得更强：趋势至少两个中枢，至少六段次级别走势，且两个中枢不能共用一个次级走势。见 `docs/chanlun/text/blog/027-第27课.md:846-856`。当前塔可以从六段得到两个相邻的三段窗口单元，但没有再把这两个单中枢窗口按同向关系、延伸与终结条件组装成一个趋势型 `CompletedMove[k]`。

生产循环确实另行把本层全部中枢分类成一个 `MoveKind`，但该分类结果写入 `LevelState.moves`；递归下一级实际接收的仍是与窗口中枢一一对应的 `upper_moves`。见 `rust/src/theta_v0/classifier/mod.rs:227-238,256-266`。因此缺的是「趋势型 `LeveledMove`/CompletedMove 的对象组装」，不是说代码里完全没有 `Trend` 分类标签。

### C-d. 完成条件被「窗口封闭」替换

本文把「第三段到达且三段重叠成立、窗口立即 compose」简称为**窗口封闭**；这是材料术语，不是代码字段。

定义完成语义是「盘整被后续走势终结」或「趋势末段背驰」，并在工程结算中以 `settled` 标记承载。见 `.chanlun/definitions/level_recursion.md:76-80,231-242`。塔的产出时点则只有窗口扫描命中时点：命中即输出并 `i+=3`。见 `rust/src/theta_v0/classifier/recursive_tower.rs:195-205,232-249`。

因此当前生产塔把「具备一个最小中枢窗口」当成了可提升上一级的封闭单元；它没有等待定义所说的走势终结证据。

### C-e. 不成立分支会产生上级谱系中的孤儿段

当 `[i,i+1,i+2]` 不成中枢时，游标从 `i` 前进到 `i+1`；此后任何新窗口都不会再包含位置 `i`。见 `rust/src/theta_v0/classifier/recursive_tower.rs:195-205`。该段仍存在于下级原始序列，但不进入任何上级 `sub_moves`，故其信息从**上级组装谱系**消失。

这与定义口径的差异不是「一段数据从仓库被删除」，而是「该段没有归属任何上级走势类型」。若消费层把塔元素直接当完整走势序列，第一次离开/回抽的对象顺序会跨过这些孤儿段。

### C-f. 这是两套契约的裁决问题，不应先验定性为修复项

塔没有背离它自己的契约：Rust 头注释、Lean `canonicalWindows/composeStep`、Origin 重锚和增量前缀不变性彼此一致。证据见 `rust/src/theta_v0/classifier/recursive_tower.rs:26-33,262-281`、`formal/Foundation/ChanlunInstantiation.lean:105-126`、`formal/Origin/RecursiveLevelSystem.lean:14-20`。

偏差发生在**契约选择**：窗口形式化把「走势分解定理二的最小三段 witness」提升为生产 `Move` 元素；定义文件则把 `Move` 保留给「已经完成的盘整/趋势走势类型」。两者各自自洽，但不能在下游无映射地互换。因此 C 必须先裁语义，再谈实现，不应把塔直接标为 bug 或直接改塔。

## 3. 下游影响链

### 3.1 `k>=2`：元素与走势类型发生范畴落差

生产循环在第 `k` 层消费的是前一级窗口 compose 的 `LeveledMove`，再把本层窗口输出作为下一层输入。见 `rust/src/theta_v0/classifier/mod.rs:217-238,262-266`。于是：

```text
实际：Center[k] 的组件 = WindowUnit[k-1]
定义：Center[k] 的组件 = completed CompletedMove[k-1]
```

`k=1` 时两者因 `Move[0]=Segment` 的归纳基底而退化重合；从需要消费 `Move[1]` 的 `k>=2` 起，窗口元素与完成走势类型的差异进入每一层递归，并随层级传播。

### 3.2 #54 结论 6 的根因假说

#54 的已核实数据是：213 个 third-closed 对象中，`completed_trend_decomposition=0/213`；短路分布为 `TREND_CONTEXT_NONE=175 / CHAIN_NOT_TREND=37 / NO_SUCCESSOR=1`。见 `/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md:11-19,473-503`。

本材料给出如下**根因假说，不把相关性写成已证因果**：

1. 塔不产趋势型 `CompletedMove`，而是产单中枢 `WindowUnit`。
2. #54 的完成分解却要求 c 内部中心链同向、从趋势块边界开始，并由不再同向延续的后继单元证明结束；该要求见只读报告 `/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md:484-503`。
3. 因此消费者是在一串单中枢窗口元素上**事后寻找**趋势块，而不是消费已由组装器产出的趋势型走势；`37/38` 有上下文对象在该事后链上失败为 `CHAIN_NOT_TREND`，另 `1/38` 缺完成后继。这与「塔没有趋势型组装」高度一致，但仍需 C 裁决后的重放对照才能确证根因。

### 3.3 `38/213` 趋势上下文的旁路来源

`38/213` 不表示塔已经产生了 38 个趋势型走势。#54 只读诊断中的 `trend_context` 直接取当前 B 中枢的前一个相邻中枢，并用 `classify_relation(previous,b)` 判断 `UpContinuation/DownContinuation`；它没有读取一个已组装的趋势型 `Move`。见 `/tmp/p54-audit-work/rust/src/theta_v0/classifier/recursive_tower.rs:1458-1469`。

所以这 38 个上下文来自**相邻中枢几何关系旁路**：

```text
adjacent Center relation -> TrendContext
```

而不是：

```text
assembled completed trend Move -> TrendContext
```

这解释了为什么「塔不产趋势型」与「仍有 38 个 trend_context」可以同时成立。`TREND_CONTEXT_NONE=175` 只说明多数对象连相邻中枢同向关系都没有；它不能单独证明或否定完成走势语义。

### 3.4 `firstRetrace` 的计数域在高层没有同名对象

第三类定义要求第一次离开后的回试，且离开段与回试段都是完成的次级别走势类型。见 `.chanlun/definitions/maimai.md:132-142`。Lean 判据也显式要求 `firstRetrace=true`。见 `formal/Origin/BspClassification.lean:107-121`。

因此严格计数域应是：

```text
按完成顺序排列的 CompletedMove[k-1] 序列
```

但塔给消费者的是含孤儿跳过、每项恰一个窗口中枢的 `WindowUnit[k-1]` 序列。对 `k>=2`，这个序列上没有与定义同一身份的「完成次级别走势类型」可数；在它上面说「第一对」只能得到 first-window-pair，不能自动推出 first-completed-move-retrace。

#54 已经发现 213 个成功对象中 93 个是在首个被评估 pair 失败后由后续 pair 成功，并谨慎结论为 `firstRetrace` 等价证书缺失，而非 93 个假阳性。见 `/tmp/p54-audit-work/chanlun/review-results/p54-atom-negation-audit-20260712.md:13-18,31-39`。C 偏差说明该证书缺失可能不只是「少了一个 bool」，而是高层计数对象本身尚未由生产塔构造。

## 4. 两个强张力例

### 例 1：`level=1` 基底退化，C 不影响构件身份

定义规定 `Move[0]=Segment`。见 `.chanlun/definitions/level_recursion.md:24-26`。生产入口也明确把 parser 线段逐个变为 L0 `LeveledMove::Segment`，首轮 `is_l0=true`。见 `rust/src/theta_v0/classifier/mod.rs:186-205,217-229`。

因此构造 `Center[1]` 时：

```text
WindowUnit[0] = Segment = Move[0]
```

C1 与 C2 对「三个构件是什么」没有分歧。level=1 的中枢与以线段为计数域的几何对象不因 C 改判而重算；这也是为什么不能把 #54 全层问题粗暴归因为同一个塔 bug。

### 例 2：`level=3` 依赖 C，构件身份分叉

构造 `Center[3]` 时，定义要求三个已经完成的 `Move[2]`；这些 `Move[2]` 可以是一个中枢的完成盘整，也可以是至少两个同向中枢的完成趋势。见 `.chanlun/definitions/level_recursion.md:24-26,76-80`。

当前塔提供的却是 `WindowUnit[2]`：每个恰由三个 `WindowUnit[1]` 加一个 `Center[2]` 构成，且不吸收延伸、不组装趋势型、可跳过孤儿段。见 `rust/src/theta_v0/classifier/recursive_tower.rs:221-249,278-281`。

因此同一批底层段在 level=3 会出现两条不同对象链：

- C1：三个 `WindowUnit[2]` 足以成为 `Center[3]` 构件。
- C2：必须先把窗口材料按 as-of 终结证据组装成三个 `CompletedMove[2]`，再构造/查询 `Center[3]`。

这会改变可见构件数、ID、首次回抽顺序、趋势完成时点和区间套父子映射；level=3 是 C 的实质分叉层，而 level=1 是控制例。

## 5. 待裁选项

### C1：承认窗口语义为生产口径

定义：把当前塔的 `WindowUnit[k]` 正式承认为生产 `Move[k]`；「完成」在生产递归中解释为窗口封闭，而不是定义文件的盘整/趋势终结。

收益：

- 保留 Rust/Lean `composeStep` 的逐字段一致、不可变前缀和增量续扫，不改生产塔。
- 元素 ID、现有重放结果、B2/S2 descend 链和缓存契约保持稳定。
- 计算对象简单、全函数、易于做前缀稳定性证明。

代价：

- 必须补一份明确的口径映射文档：`生产 Move = WindowUnit`，并把 `.chanlun/definitions/level_recursion.md` 中 `CompletedMove` 标成另一语义层，禁止同名无注释互引。
- `firstRetrace` 只能先定义为窗口序列上的首次 pair；若仍声称「完成次级别走势的第一次回抽」，需另加等价证明，不能沿用名称替代证明。
- `completed_trend_decomposition`、趋势背驰、区间套等依赖完整走势的消费者必须降格为窗口上的派生判据；#54 计数和资格名称需要重命名/重基线。
- 严格缠论文本中的盘整延伸、趋势多中枢与走势终结不再是生产塔本体语义，必须诚实声明有效域。

### C2：定义语义为准，经消费层组装器实现，不动塔

定义：保留塔为不可变 `WindowUnit` 构造层；新增消费层纯函数组装器，在给定 `as_of` 时点把窗口、延伸、终结与后继证据组装为 `CompletedMove` 视图。所有要求「完成的次级别走势类型」的消费者只读该视图。

收益：

- 保住窗口塔的 Lean/Rust 对齐与增量性能，同时恢复 `Move[k]` 的盘整/趋势/完成定义。
- `firstRetrace` 获得可数对象域；趋势型 `Move`、`completed_trend_decomposition` 和区间套可以共享同一个完成走势视图。
- 构造层与解释层分开，未来可并行比较 C1 窗口结果与 C2 完成走势结果，而不污染底层塔。

代价：

- 需要定义组装器的归属、结合律、延伸、终结、ID、confirm/judge 时点和多义分解选择；这些不是简单拼接。
- #54 的 `213`、`93`、`38`、`0` 等计数必须在新视图上重放，不能假定维持。
- 实时视图需要追加式版本账本与 supersede 关系；若直接回写历史对象会产生 repaint。
- 所有消费入口要做对象类型迁移或显式适配，测试面大于补一个局部判断。

## 6. 定向架构材料：b 路线双轨分层

本节记录已讨论的架构方向，仍不等于对 C2 的裁决。

### 6.1 两层职责

```text
构造层：Immutable Window Tower
  input  = confirmed lower-level units
  output = append-only WindowUnit[k] + stable ElementId
  rule   = current compose_level / composeStep

消费层：assemble_completed_moves(as_of, tower_snapshot, event_ledger)
  input  = as-of 可见窗口材料与终结证据
  output = ConsolidationMove / TrendMove / PendingMove views
  rule   = pure function; no mutation of tower
```

塔保持现有不可变窗口与稳定 ID。组装器负责定义语义的延伸、趋势多中枢、完成与版本关系。生产消费者必须在类型上明确声明自己读取 `WindowUnit` 还是 `CompletedMoveView`，禁止继续以同一个 `Move` 名称暗渡。

### 6.2 三道无 repaint 防线

1. **前缀稳定性 property test**：对任意 `t<T`，用 `prefix[..=t]` 运行组装器得到的视图，应等于完整事件账本按 `as_of=t` 查询得到的视图。未来数据可以追加新版本或 supersede 事件，但不能改写当时可见事实。
2. **入场时点硬门**：任何回测/实盘订单的 `entry_bar >= judge_at + 1`。`judge_at` 是完成或 supersede 事件真正可知的 source bar；同 bar 成交一律视为前视。
3. **supersede 事件账本**：对象改判不覆盖旧行，只追加 `{old_id,new_id,reason,judge_at}`。历史查询按 as-of 还原当时版本，当前查询追随 supersede 链；这样「后来知道」不会伪装成「当时知道」。

### 6.3 区间套重新落位

现有定义把多级别嵌套实现锚为 `nested_divergence_search`。见 `.chanlun/definitions/beichi.md:288-300`。在双轨架构中，区间套不应修改窗口塔，也不应直接在窗口单元上冒充完成走势递归；它应成为：

```text
nested_divergence_search_as_of(
  completed_move_views_by_level,
  parent_range,
  as_of
)
```

即在组装器视图之上的自顶向下、嵌套、as-of 只读查询。这样区间套的父子范围、背驰段与完成时点都来自同一版本快照，三道无 repaint 防线可以统一覆盖。

## 7. 待 P0 裁决的问题

### 先裁 C

1. 生产 `Move[k]` 的法定身份是 `WindowUnit[k]` 还是 `CompletedMove[k]`？
2. 「窗口封闭」是否可以正式替代「走势类型完成」进入递归构件资格？
3. 若取 C1，是否接受把盘整延伸、趋势组装与 firstRetrace 降为窗口上的派生语义，并补口径映射文档？
4. 若取 C2，是否授权 b 路线：塔不可变，完成走势只由消费层 as-of 纯函数组装器产生？
5. 孤儿段在上级谱系中允许永久无归属，还是必须由组装器吸收到某个完成走势或显式 `Unassigned` 账本？

### C 后再裁 A / B

本文采用以下依赖代号，防止在对象域未定时先裁下游：

- **A：`firstRetrace`**。先按 C 确定计数对象，再裁 93 个后续 pair 的身份与严格第一次回抽判据。
- **B：趋势完成资格**。先按 C 确定 c 内元素是窗口还是完成走势，再重放 `completed_trend_decomposition=0/213`、`TREND_CONTEXT_NONE=175` 与 38 个几何旁路上下文。

依赖顺序是：

```text
C（对象语义）
├── A（firstRetrace 计数域与 93 例）
└── B（完成趋势分解、38/175 旁路与 0/213）
```

A 与 B 可以在 C 后并行；不得在 C 未裁时把窗口序列上的「第一」或「趋势链」直接宣布为完成走势语义上的结论。

## 8. 材料边界

- 本文没有裁 C1 或 C2。
- 本文没有把当前塔定性为 bug；它忠实于自身 Lean/Rust 窗口契约。
- 本文没有把 #54 的相关计数升级为因果证明；「塔不产趋势型」是对 `0/213` 的根因假说，须在 C 后重放确认。
- 本文没有修改塔、定义、信号、回测或事件账本。

---

## 9. 补充页（2026-07-12）：与交易执行层的关系 — 级别内互斥、跨级别并行与短差

> 本页为定向材料的延伸，回答「全定义互斥策略如何做多空双开的短差」。同第 8 节口径：不构成对 C 的裁决，不修改定义与塔。

### 9.1 互斥的正确作用域是「级别内」

全定义的买卖点互斥（一买之后等待对应卖点，反之亦然）是**单级别状态机内**的语义约束：同一级别、同一标的，任一时刻至多持有一个方向主张。它从来不是跨级别约束——1 分钟级别的卖点主张与 30 分钟级别的买点主张并存，正是区间套/短差的定义性场景，不是违例。

因此「多空双开」不需要破坏互斥：

```text
每级别一个持仓状态机 S_L（级别内互斥）
执行层 = 净额合成 net(Σ_L position_L)
```

被禁止的只有一件事：同一 S_L 同时输出多头与空头主张。

### 9.2 短差的执行模型：base + overlay

- **大级别腿（base）**：由大级别状态机管理，买点建仓、卖点清仓，期间不动。
- **小级别腿（overlay）**：小级别卖点输出 `-delta`，其后小级别买点接回 `+delta`，实现「卖高接低」的短差。
- **执行头寸** = `base + Σ overlay`。
  - 现货约束：`Σ overlay ≥ -base`（短差只能在持仓内做减法，不能净卖空）。
  - 期货/可做空市场：无该约束，两腿可以是真实对冲腿或净额化，取决于账户结构，与缠论语义无关。

### 9.3 与双轨架构的对接

- 两个状态机都只消费组装器的 `completed_move_views_by_level` as-of 快照：卖出腿锚定其触发时刻的版本，接回腿锚定自己的版本，三道无 repaint 防线（§6.2）对两腿统一生效。
- 区间套查询（§6.3）自然给出短差的触发结构：父级别背驰段内、子级别的完成走势提供进出点。
- 该模型对 C1 / C2 均成立，不依赖 C 的裁决结果；但若窗口塔存在 repaint，小级别 overlay 状态机受影响最大（信号翻覆 → 短差腿被反复触发），这是 C 裁决影响交易层的主要传导路径。

### 9.4 本页边界

- 未定义 delta 的仓位算法与风控参数（属策略参数域，非定义域）。
- 未主张现货/期货哪种账户结构更优。
- 未把短差纳入回测账本；接入需在事件账本中为每级别腿单独记账，禁止事后合并改写。
