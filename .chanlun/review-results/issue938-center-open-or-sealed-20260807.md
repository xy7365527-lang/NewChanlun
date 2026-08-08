# #938 探针：中枢震荡短差挂单绑定的中枢，是「还在延伸中（开放）」还是「已封口（sealed）」

**日期**：2026-08-07 ｜ **票**：[#938](https://github.com/xy7365527-lang/NewChanlun/issues/938)（挂 map [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787)）
**口径**：AFK 只读探针。既有 `.rs` 一行未改；新增只读探针 bin 一个（`rust/src/bin/p938_bound_center_openness.rs`）。
**基线**：worktree HEAD == 本地 `main` `77a8c9660e`（开工前已从 `origin/main` 祖传线 `git reset --hard main` 纠正，见 §7）。

---

## 0. 一句话结论

> **开放。100%，零例外，且不是「有时开放有时封口」——是结构上永远开放。**
>
> 更进一步：这个开放中枢的核心 `zd`/`zg` **在生产中确实被原地改写过**，本探针在 BTC 前 100 万根 1min bar 上直接数到 **258 次**「中枢还活着（`start_index` 不变）、核心却变了」。
>
> ⟹ **ADR 0019「缺口一」（键说同一张单、价却该改了）不是空的。[#938](https://github.com/xy7365527-lang/NewChanlun/issues/938) 该继续裁，不该关。**

判据对照票面给的二选一：

| 票面假设 | 是否成立 | 依据 |
|---|---|---|
| 挂在**已封口**中枢上 ⟹ 核心永不改写 ⟹ 缺口是空的、票可关 | **不成立** | 绑定对象恒为链尾 = frontier 开放中枢（§2），且核心实测被改写（§3） |
| 挂在**还开放**中枢上 ⟹ 核心随 frontier 重扫改写 ⟹ 缺口成立、票要继续裁 | **成立** | §1–§3 |

---

## 1. Q1：那个 `center` 是从哪来的？——完整调用链

**每一跳都逐条打开确认过**（不靠 grep 命中数）。

| # | `file:line` | 这一跳做了什么 |
|---|---|---|
| ① | `rust/src/theta_v0/backtest/fill.rs:4740` | 生产 fill 主循环第 `i` 根 bar：`let (classification_i, tower_i, confirmed_lens, tower_gen, forest_epoch) = ...` —— 本 bar 的分类结果 |
| ② | `rust/src/theta_v0/backtest/fill.rs:4749` | `let classification_step = newly_confirmed_step(&classification_i, &mut seen_bsps);` —— 「本 bar 新确认」子集，派生自 ① |
| ③ | **`rust/src/theta_v0/backtest/fill.rs:5099`** | **生产唯一调用点**：`let osc_step = step_center_oscillation_historical(i, &classification_i, &classification_step, ...)` |
| ④ | `rust/src/theta_v0/backtest/fill.rs:907-929` | 薄壳 `step_center_oscillation_historical` → `step_center_oscillation_impl(...)` |
| ⑤ | `rust/src/theta_v0/backtest/fill.rs:959` | `let chain: &[Center] = &classification_i.levels[lvl].centers;` —— **喂的就是本级本 bar 的整条中枢链** |
| ⑥ | `rust/src/theta_v0/backtest/fill.rs:965` | `cl_machines[lvl].consume_chain(chain)` —— 生命周期机消费该链 |
| ⑦ | `rust/src/theta_v0/backtest/fill.rs:1227` | `let alive_with_index = cl_machines[lvl].alive_center();` |
| ⑧ | `rust/src/theta_v0/backtest/fill.rs:1269-1271` | `let center = alive_with_index.map(\|(center, _)\| center).expect("触发构造成功蕴含完整在场中枢存在");` |
| ⑨ | **`rust/src/theta_v0/backtest/fill.rs:1283`** | `osc_books[lvl].on_trigger_side_bound(side, trigger, center)` |
| ⑩ | `rust/src/theta_v0/strategy/center_oscillation_trade.rs:620-631` | 入口本体：`assert_eq!(CenterId::of(&center), trigger.center(), "挂起四边框必须与触发 CenterId 精确同源")` |

**⟹ 传进 `on_trigger_side_bound` 的 `center` = `CenterEventMachine::alive_center()` 的快照。**

### 1.1 ⚠ 一个差点踩进去的坑（记下来，因为它正是本仓在案的那类错）

`step_center_oscillation*` 在 `fill.rs` 里出现 **30+ 次**，其中 **绝大多数在 `#[cfg(test)] mod`（起于 `fill.rs:2531`，止于 `fill.rs:4102`）内**。若只看 grep 前若干行（默认 `head` 截断），会得出「**这条链根本没接生产、只有测试在调**」的结论——那是**错的**。生产调用点在 `fill.rs:5099`，落在测试模块**之后**。

另有一条会把人带偏的旁证：`cargo build --lib --release` 的 dead-code 报告里 `fill.rs` **一条都不出现**，看起来像「fill.rs 里没有死代码 ⟹ 都被调用了」。真相是 **`fill.rs` 那次压根没被编译**——`theta_v0::backtest` 挂在 `#[cfg(any(test, feature = "backtest_bin"))]`（`rust/src/theta_v0/mod.rs:110-111`）。**该编译器读数对本题零信息量**，不得据它下任何结论。

---

## 2. Q2：那个中枢对象，被传进来的一刻是开放还是封口？

### 2.1 先查清楚「开放 / 封口」在代码里怎么表示

**没有 `is_open` / `sealed` 字段，也没有状态枚举。** `Center` 的全部字段只有 `zd / zg / dd / gg / start_index / end_index`（构造点 `rust/src/theta_v0/classifier/center.rs:224-231`）——**开放性不是中枢自己的属性，是它在链上的位置属性**。

判据有三种表述，指同一件事：

| 层 | 表述 | 出处 |
|---|---|---|
| **教义** | 「最后一个中枢在未出现 non-extension 单元前是**开放**的（尾部追加单元可延伸它）」 | `rust/src/theta_v0/classifier/recursive_tower.rs:709-711` |
| **机检分界（可直接读的数）** | `confirmed_watermark = upper_moves.len() − scan_cursor.last_window_emitted`，注释逐字「**本 bar frontier 末窗口（下 bar pop 重产）排除**」。下标 `< w` = 已封口的 confirmed 前缀；`≥ w` = 开放 frontier | `rust/src/theta_v0/classifier/mod.rs:1542-1554`；字段文档 `rust/src/theta_v0/classifier/tower_cache.rs:136-144`；公开取数口 `tower_cache.rs:286` `tower_confirmed_len()` |
| **扫描期哨兵** | 窗口未被 non-extension 单元终止时 `read_end_src = usize::MAX`，注释逐字「`j==len` ⟹ **窗口开放无哨兵** ⟹ +∞（永不进保留前缀）」 | `rust/src/theta_v0/classifier/recursive_tower.rs:754-756` |

**封口那一侧的保护条款**（票面引的 frontier 协议第 3 条）原文在
`rust/src/theta_v0/classifier/recursive_tower.rs:714-716`：

> 3. 保留的前缀 centers（seed 与延伸全部结束于 `start_i` 前者）不可变——每个前缀中枢的延伸由一个显式 non-extension 单元终止（该单元的判定只读该单元 vs **冻结核心**，units 前缀不可变 ⟹ 判定冻结），扫描永不回访其窗口。

**这条只保护「保留的前缀 centers」。链尾不在其中。**

### 2.2 生产每 bar 真的把链尾 pop 掉重扫

`rust/src/theta_v0/classifier/mod.rs:1376-1415`（**注意：不是 #929 报告写的 `mod.rs:1937-2001`，见 §6**）：

- `:1369` `let had_emitted_window = lc.scan_cursor.resume_from < lc.scan_cursor.consumed;`
- `:1380` `let pop_n = lc.scan_cursor.last_window_emitted;`
- `:1402-1403` `let cs = Rc::make_mut(&mut lc.centers); cs.truncate(cs.len().saturating_sub(pop_n));`
- 注释 `:1377-1379` 逐字：「**pop 最后成立窗口的全部产出**（frontier 域 = 整窗，重扫从窗口起点重产）」

重扫时 `center_from_segments`（`rust/src/theta_v0/classifier/center.rs:216-231`）用**已修订的** frontier 源重新求交：`zd = compute_zd(a,b,c)` / `zg = compute_zg(a,b,c)`，而 `start_index = a.start_index` 不动。

### 2.3 判定：绑定中枢恒为链尾

**`alive_center()` 的返回值恒是「链尾」或 `None`。** 这是穷举结论——`center_lifecycle.rs` 全文对字段 `self.alive` 只有 **8 处**引用（检索式 `grep -n "self\.alive" rust/src/theta_v0/classifier/center_lifecycle.rs`，覆盖整文件），其中**写点只有 3 处**：

| `file:line` | 写的是什么 | 是不是链尾 |
|---|---|---|
| `center_lifecycle.rs:586` | `self.alive = Some((*c, idx));` 在 `for (idx, c) in chain.iter().enumerate().skip(k)` 循环体内 —— 循环结束时 `idx == chain.len()-1` | **是** |
| `center_lifecycle.rs:611` | `self.alive = chain.last().map(\|c\| (*c, chain.len() - 1));`（`adopt()`，首次消费 / 重基共用） | **是**（字面 `chain.last()`） |
| `center_lifecycle.rs:742` | `let taken = self.alive.take();`（教义死亡）⟹ `None` | 不构造触发（见下） |

`None` 那一支不会走到绑定：`CenterOscillationTrigger::new` 第一行就是
`let center = alive.ok_or(TriggerError::CenterNotAlive)?;`（`rust/src/theta_v0/strategy/center_oscillation_trade.rs:156`）。

读点只有 `center_lifecycle.rs:878-880` 的 `alive_center()`，其 doc 逐字「**在场实例（出生快照 + 链下标）**」。

**⟹ 绑定中枢 = 链尾 = 每 bar 被 pop 重扫的那个开放中枢。不存在「有时封口」的分支。**

同一结论在仓内已被独立坐实（**非本探针自造**）——`rust/src/theta_v0/lineage_book.rs:5-8` 模块 doc 逐字：

> 开放 frontier 每 bar 被 pop 后重扫，`CenterId=(start_index, zd, zg)` 是当次前三个构造单元的派生值：**第三源外缘一改，父核心就重新求交，旧三元组消失、新三元组出现**。生命周期层随后以新链全量 `adopt`，挂起簿只做 `CenterId` 精确集合核对，旧身份即被报作 `RebaseVanished`——这是**工程丢身份**，不是教义生死。

而 `RebaseVanished` 是**生产的挂起终结来源之一**，不是诊断项：`center_oscillation_trade.rs:307` 是枚举变体，`:392` 把它落成 `TerminationSettlement::WriteOffUnclosed`（未闭合减出核销）。**「绑定中枢的身份会因核心被改写而消失」这件事，本仓已经为它写了一整套善后机制**（#679 `LineageBook` 谱系迁移）——这本身就是「绑定的是开放中枢」的最强旁证。

---

## 3. Q3：实测

票面说「如果两种都可能，就要回答比例」。**结论是两种并非都可能**（§2.3 穷举），故比例问题在本题上退化。本探针仍做了实测复核，并**顺带量出票面真正关心的那个数**（核心到底漂不漂、漂多少次）。

### 3.1 探针口径

新增 `rust/src/bin/p938_bound_center_openness.rs`（**只读，不改任何既有 `.rs`**）：

```
cargo run --release --features backtest_bin --bin p938_bound_center_openness -- \
    ../analysis/data_cache/btc_1m_full.json 1000000
```

逐 bar 走 `IncrementalClassifier::classify_at(i)`（= 生产 fill 循环第 ① 跳的同一入口），逐级
`consume_chain(&classification.levels[lvl].centers)`（= 第 ⑤⑥ 跳逐字同源），然后读两件事：

- **读数一（命题 P）**：`alive_center()` 为 `Some` 时，其链下标是否 == `chain.len()-1`；
- **读数二**：本级链尾 `CenterId` 与**上一 bar** 链尾的三分迁移。

**探针在全部 bar × 全部级别上检 P，而不只在触发 bar 上检**——这是**更强**的断言（触发 bar 是全体 bar 的真子集，全集零违反 ⟹ 触发子集零违反）。代价是本探针**不重放买卖点触发**（不调 `push_point`、不构造 `CenterOscillationTrigger`），因此**不报触发绝对次数**，只报占比性质的 P。这一点在 bin 模块头已逐字声明。

### 3.2 读数一：P =「`alive_center()` 恒为链尾（= 开放 frontier）」

BTC 1min，前 **1,000,000** bar（2017-08-17 起；跑时 5 分 03 秒）：

| level | alive_bars | is_tail | **not_tail** | is_tail 占比 |
|---|---|---|---|---|
| L0 | 999,433 | 999,433 | **0** | 100.000000% |
| L1 | 998,307 | 998,307 | **0** | 100.000000% |
| L2 | 991,821 | 991,821 | **0** | 100.000000% |
| L3 | 976,423 | 976,423 | **0** | 100.000000% |
| L4 | 711,768 | 711,768 | **0** | 100.000000% |
| **合计** | **4,677,752** | **4,677,752** | **0** | **100.000000%** |

> **绑定中枢「当时还开放」的占比 = 100.000000%（4,677,752 / 4,677,752 级-bar，零反例）。**

前 20 万 bar 的独立较小窗同样零反例（765,984 / 765,984），两窗一致。

### 3.3 读数二：链尾 `CenterId` 逐 bar 迁移 —— 核心到底漂不漂

| level | same | **core_drift**（`start_index` 同、核心变） | seed_moved（换中枢） | core_drift 占变化 |
|---|---|---|---|---|
| L0 | 997,531 | **25** | 1,876 | 1.32% |
| L1 | 997,724 | **151** | 431 | 25.95% |
| L2 | 991,664 | **62** | 94 | 39.74% |
| L3 | 976,387 | **15** | 20 | 42.86% |
| L4 | 711,760 | **5** | 2 | 71.43% |
| **合计** | **4,675,066** | **258** | **2,423** | **9.62%** |

**读法（先人话）**：这 100 万根 K 线里，中枢链的末端一共换了 **2,681** 次内容。其中 **2,423 次是「换了另一个中枢」**（正常推进，旧单本来就该撤），但 **258 次是「还是那个中枢、只是它的上下沿被重新算了」**——**约每 10 次末端变化就有 1 次属于后者**。

前 20 万 bar 窗独立复核：`same=765,495 / core_drift=48 / seed_moved=437`，占比 **9.90%** —— 与 100 万窗的 **9.62%** 一致，**该比例对窗口长度稳定**，不是单窗偶然。

**注意级别分布**：`core_drift` 占变化的比例**随级别单调上升**（L0 1.32% → L4 71.43%）。级别越高、中枢链越短、末端越少被整体换掉，「同一个中枢核心被重算」就越是末端变化的**主要形态**。**中枢震荡短差恰恰跑在 L1 及以上**（`fill.rs:1223` `for lvl in 1..n_levels`，L0 无次级别）——**即缺口最严重的那几级，正是这条链实际工作的那几级**。

**这 258 次正是 ADR 0019 缺口一描述的情形**：身份键（只含 `start_index`）说「同一张单，续挂」，而挂价读的 `ZD`/`ZG` 已经变了。#929 逐数的那条（`zd` 从 `2580000000000` → `2571704000000`，约 **−0.32%**）就是这类事件的一个样本。

**不要外推的地方**：258 是「链尾核心被改写的 **bar 次**」，**不是**「会导致限价单错价的**订单**数」——后者取决于触发时点与挂单存续期，本探针没测（见 §5）。

---

## 4. Q4：现有震荡触发 ≠ 将来的限价单，两者结论分开写

| | **现有的中枢震荡短差触发链**（本报告 §1–§3 查的就是它） | **ADR 0019 的三把限价单** |
|---|---|---|
| 状态 | **已实装、已接生产**（`fill.rs:5099` 是活的调用点） | **未实装**，ADR 0019 是裁定文，实施归 [#943](https://github.com/xy7365527-lang/NewChanlun/issues/943)（ADR 0019 `:278` 明写「归 #938 裁」的是改裁部分） |
| 绑定对象 | `alive_center()` 快照，恒为开放链尾（§2.3） | 尚不存在。ADR 0019 裁定三只定了**身份键** = `(CenterId.start_index, 方向)`，挂单价从 `ZD`/`ZG` 读（ADR 0019 `:97`、`:265-269`） |
| 「核心会漂」是否已发生 | **是**，258 次 / 100 万 bar（§3.3） | 不适用（还没有单） |

**所以严格说**：「限价单挂在哪个中枢上」现在**还不存在**，本报告**没有**、也**不能**回答它。

本报告回答的是它的**前置事实**：**限价单将来要接上去的那个位置（`on_trigger_side_bound`），拿到的中枢是开放的、其核心会被改写。** ADR 0019 `:269` 自己就把这条路指出来了——

> 仓内有同形先例可直接抄：`strategy/center_oscillation_trade.rs:619-631` 的 `on_trigger_side_bound`（#487 生产接线入口）……⟹ 语义闭合的写法是**「键管身份、冻结框管价格」**。

**⟹ 缺口不是空的。** #487 的「首次绑定冻结四边框」之所以存在，恰恰**因为**绑定对象是开放的、会漂——它是**已经为这个病打过的补丁**，不是「不会漂」的证据。**把「已有冻结框」误读成「核心不会漂所以票可关」，方向正好反了。**

---

## 5. 未能确认 / 诚实边界（不省略）

1. **本探针未重放买卖点触发**（§3.1 已声明）。故「触发中枢震荡挂单的那些时刻」这个**子集**没有被单独取样——结论靠的是「全集零反例 ⟹ 子集零反例」这条包含关系，不是对子集的直接观测。**触发绝对次数本报告没有数**。
2. **理论角落未测**：`last_window_emitted == 0` 时 `confirmed_watermark == len`，此刻链尾在形式上会落进「已封口」一侧。该角落**是否可达、可达时链是否非空，本探针没测**（探针检的是 `alive == 链尾`，不是 `alive 下标 ≥ confirmed_watermark`）。**「没测到」不等于「不存在」**——若要闭合，可经公开取数口 `TowerCache::tower_confirmed_len()`（`tower_cache.rs:286`）加检，但须先核对 `Classification.levels[lvl]` 与 `LevelCache` 的下标错位（该口对 `level>=1` 返回的是 `levels[level-1].confirmed_watermark`）。
3. **窗口是前 100 万 bar，不是全史**。全史共 4,613,599 bar；同一探针的全史跑已启动，但分类器随链增长**超线性**（20 万 bar=12 秒、100 万 bar=5 分 03 秒 ⟹ 约 n^1.9，外推全史需 1.5–2 小时），**超出本次 AFK 预算，已主动终止**。**故全史读数本报告不报**，报的是前 100 万 bar（占全史 21.7%，覆盖 2017-08 起连续区间）。**不得把 258 这个数当作全史数字外推**——但 P=100% 那条是结构性结论（§2.3 穷举），不依赖窗口长度。
4. **`core_drift` 的口径限制**：它比的是**相邻 bar 的链尾**。若某中枢在若干 bar 内先被换掉、后又被重基换回，本三分法会记成两次 `seed_moved` 而非 `core_drift`——**本探针不追踪跨 bar 的中枢个体轨迹**，只做相邻帧差分。
5. **`core_drift` 未按「是否落在挂单存续期内」加权**（§3.3 末已声明）。
6. **本探针只跑 BTC**。跨品种未测。
7. **`consume_chain` 之外的生命周期驱动（`push_point` 死亡事件）未接入探针**。接入后 `alive` 会多出 `None` 的 bar（分母变小），但 `Some` 时仍恒为链尾（§2.3 是结构性结论，不依赖是否喂死亡事件）——**方向不受影响，绝对分母会变**。

---

## 6. 顺带发现：#929 报告有一处行号援引错（结论不受影响）

`.chanlun/review-results/issue929-centerid-stability-20260807.md:134` 写：

> 接线在 `classifier/mod.rs:1937-2001`（pop 末个开放构造整窗、从旧 `win_start` 重扫）

**该行号是错的。** `classifier/mod.rs:1937-2001` 是 `extract_second_for_level` 与 `second_for_parent`（次级别背驰提取），**区间内没有任何 pop / truncate / win_start 逻辑**（检索式：`awk 'NR>=1937 && NR<=2001' rust/src/theta_v0/classifier/mod.rs | grep -nE "^(pub )?fn |pop|truncate|win_start|resume"`，命中只有两个 `fn` 声明）。

**真正的 frontier pop 在 `classifier/mod.rs:1376-1415`。**

已排除「报告写时是对的、后来漂了」这一解释：`git diff d6e9fd12e2 HEAD --stat -- rust/src/theta_v0/classifier/mod.rs` **无输出**（该文件自 #929 报告落盘的 commit 起未再改动）。

**结论不受影响**——#929 的实质结论（生产延伸走 frontier pop + 重扫、`zd`/`zg` 会漂）本报告独立复核成立（§2.2）。但这是**「结论对、论据假」的又一例**；ADR 0019 `:21` 记的在案数为 **6/6**，本条使之成为 **7 例**。建议在 ADR 0019 或 #929 上补一条订正。

---

## 7. 开工前的基线纠正（留痕）

worktree 初始 HEAD = `19b4015927`，经 `git merge-base --is-ancestor HEAD main` 判定**不是** `main` 的祖先，且 `git rev-parse origin/main` == `19b4015927` —— 即落在**零共同祖先的祖传废线**上。工作区干净（`git status --porcelain` 零行）后执行 `git reset --hard main`，HEAD 归位 `77a8c9660e`。**若不纠正，本报告全部行号读数作废。**

---

## 附：援引清单（逐条已打开确认，非 grep 命中数）

| `file:line` | 承重内容 |
|---|---|
| `rust/src/theta_v0/backtest/fill.rs:4740` / `:4749` | `classification_i` / `classification_step` 的绑定（本 bar） |
| `rust/src/theta_v0/backtest/fill.rs:5099` | **生产唯一调用点** `step_center_oscillation_historical` |
| `rust/src/theta_v0/backtest/fill.rs:907-929` | 薄壳 → `step_center_oscillation_impl` |
| `rust/src/theta_v0/backtest/fill.rs:959` / `:965` | `chain = &classification_i.levels[lvl].centers`；`consume_chain(chain)` |
| `rust/src/theta_v0/backtest/fill.rs:1227` / `:1269-1271` / `:1283` | `alive_center()` → `center` → `on_trigger_side_bound` |
| `rust/src/theta_v0/backtest/fill.rs:2531` / `:4102` | `#[cfg(test)] mod` 起止（§1.1 那个坑的边界） |
| `rust/src/theta_v0/mod.rs:110-111` | `backtest` 挂 `#[cfg(any(test, feature = "backtest_bin"))]` |
| `rust/src/theta_v0/strategy/center_oscillation_trade.rs:620-631` | `on_trigger_side_bound` 本体 + 同源 assert |
| `rust/src/theta_v0/strategy/center_oscillation_trade.rs:156` | `alive.ok_or(TriggerError::CenterNotAlive)?` |
| `rust/src/theta_v0/strategy/center_oscillation_trade.rs:307` / `:392` | `RebaseVanished` 变体 + 落 `WriteOffUnclosed` |
| `rust/src/theta_v0/classifier/center_lifecycle.rs:162-169` | `CenterId { start_index, zd, zg }` |
| `rust/src/theta_v0/classifier/center_lifecycle.rs:586` / `:611` / `:742` | `self.alive` 全部 3 个写点（穷举） |
| `rust/src/theta_v0/classifier/center_lifecycle.rs:878-880` | `alive_center()` 定义 + doc「出生快照 + 链下标」 |
| `rust/src/theta_v0/classifier/mod.rs:1369` / `:1376-1415` | **frontier pop**：`had_emitted_window` + `truncate(len - pop_n)` |
| `rust/src/theta_v0/classifier/mod.rs:1542-1554` | `confirmed_watermark = len − last_window_emitted`（封口/开放机检分界） |
| `rust/src/theta_v0/classifier/tower_cache.rs:136-144` / `:286` | 水线字段文档 / 公开取数口 `tower_confirmed_len()` |
| `rust/src/theta_v0/classifier/recursive_tower.rs:709-711` | 教义：最后一个中枢是**开放**的 |
| `rust/src/theta_v0/classifier/recursive_tower.rs:714-716` | frontier 协议第 3 条：**保留的前缀** centers 不可变 + 冻结核心 |
| `rust/src/theta_v0/classifier/recursive_tower.rs:754-756` | `read_end_src = usize::MAX` ⟹「窗口开放无哨兵」 |
| `rust/src/theta_v0/classifier/center.rs:216-231` | `center_from_segments`：`zd/zg` 重新求交、`start_index = a.start_index` |
| `rust/src/theta_v0/lineage_book.rs:5-8` | 模块 doc：开放 frontier 每 bar pop 重扫、旧三元组消失 |
| `docs/adr/0019-limit-order-state-flow-interface.md:265-269` | 缺口一原文 + 「键管身份、冻结框管价格」的同形先例 |
| `.chanlun/review-results/issue929-centerid-stability-20260807.md:134` | §6 那处错行号 |

**新增文件（本票产出）**：

- `rust/src/bin/p938_bound_center_openness.rs` —— 只读探针 bin。**建议入仓**（与仓内既有 38 个 `p*` 探针 bin 同例，可复跑复核；不入仓则 §3 的读数无法被第二人重放）。
- 本报告。

**未入仓的临时物**：`analysis/data_cache/btc_1m_full.json` 在本 worktree 内是指向主仓同名文件的 **symlink**（`analysis/data_cache/` 已在 `.gitignore` 覆盖内，不随 commit 进版本库）。
