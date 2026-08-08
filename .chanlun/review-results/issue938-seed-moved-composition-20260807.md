# #938 探针二：`seed_moved` 2,423 次的谱系构成——迁移还是重建

- 票：[#938](https://github.com/xy7365527-lang/NewChanlun/issues/938)（wayfinder，裁「限价单身份键用裸 `CenterId.start_index` 还是 `CenterOscillationBook::resolve()` 的归一锚」）
- 类型：AFK 只读勘察 + 新增探针 bin（**未改任何既有 `.rs`**）
- 基线：本地 `main` = `09ed7224f8`（worktree 初始 HEAD 是祖传废线 `19b4015927`，开工前已 `git reset --hard main` 对齐）
- 上游：探针一 `.chanlun/review-results/issue938-center-open-or-sealed-20260807.md`（commit `86e33e6f3b`）
- 日期：2026-08-07

---

## 0. 一句话结论

**问题的前提被实测改写了。** 那 2,423 次 `seed_moved` 里 **2,405 次（99.26%）根本不是重基**——是链正常往前长了一节（`ChainConsumed::Advanced`），旧链尾还在链上；**真正的重基只有 18 次**，而这 18 次**全部**（18/18 = 100%）被谱系簿判为「无证书 ⟹ 保持核销」，即**全是重建，零迁移**。

反过来，探针一那 258 次 `core_drift`（`start_index` 不变、核心被改写）**全部 258/258 = 100%** 拿到 `continued_1to1` 构造证书、被判为**迁移**。

⟹ **在本窗（BTC 前 100 万 bar，4.68M 帧）上，「谱系证书判迁移」与「`start_index` 未变」是同一个集合，逐帧重合、零例外。** 两个键给出的撤单/不撤单决定在本窗**一次都没有分歧**：裸键误撤 **0** 次，锚键漏撤 **0** 次。

**它支持哪个键**：支持**裸 `start_index`**——不是因为锚不对，而是因为在本窗上锚**给不出任何额外的判别力**，却要拖上 `LineageBook` + `rebase_txn` 证书链 + 挂起簿状态三样东西。⚠️ 这是**「本窗零差异」**，不是**「两者等价」**的证明：见 §5 有效域，锚在两个已知方向上仍可能与裸键分叉，只是本窗没出样本。

---

## 1. Q1：`anchor_of` 那条谱系迁移边什么时候建立（纯静态，逐条打开确认）

### 1.1 唯一写入点

`anchor_of` / `current_of` 两张表在 `rust/src/theta_v0/strategy/center_oscillation_trade.rs` 内声明（`:500` / `:503`），**全仓写入点唯一**。

**检索式与覆盖目录**（可复跑）：

```bash
cd /Users/silencehan/Projects/NewChanlun
grep -rn "anchor_of\|current_of\|current_identity_of\|on_chain_rebase_lineage" \
    --include="*.rs" rust/ | grep -v "^rust/target"
```

覆盖目录 = `rust/`（`src/` + `tests/` + `bin/`）全量，排除 `target/`。命中 51 行，逐条打开确认后分类：

| 分类 | 行 | 说明 |
|---|---|---|
| **同名异物（不是本题的 anchor）** | `rust/src/moves.rs:76-100`、`rust/src/level.rs:189-213` | `current_offsets` 局部变量，与中枢身份无关 |
| **同名异物** | `rust/src/theta_v0/backtest/admission.rs:847,858`、`classifier/nest.rs:3414,3421`、`classifier/nest_index.rs:257,267,285` | `event_anchor_of` = 区间套候选事件的**两元锚查找闭包**（`(Tick, usize)`），与 `CenterId` 谱系无关 |
| **`anchor_of` 真写入** | `center_oscillation_trade.rs:862` | `self.anchor_of.insert(new_current, anchor)` |
| **`anchor_of` 真删除** | `:860`（中间修订身份退役）、`:540`（`gc_anchor`） | |
| **`current_of` 真写入** | `:863` | `self.current_of.insert(anchor, new_current)` |
| **`current_of` 真删除** | `:539`（`gc_anchor`） | |
| **唯一读者（正向）** | `:521` `resolve()` | |
| **唯一读者（反向）** | `:530` `current_identity_of()` | |

写入点 `:857-865` 全文只有一个循环，其输入 `migrations` 只在 `:834` 一处 push：

```rust
// center_oscillation_trade.rs:857-865
for (anchor, old_current, new_current) in migrations {
    if old_current != anchor {
        self.anchor_of.remove(&old_current);   // :860
    }
    self.anchor_of.insert(new_current, anchor); // :862
    self.current_of.insert(anchor, new_current);// :863
    tally.migrated += 1;
}
```

⟹ **`anchor_of` 与 `current_of` 恒同步写入、互为逆**；`current_of` **没有**独立的写入判据，它就是 `anchor_of` 的逆表，两者判据完全相同（Q1 第三小问的答案）。

### 1.2 建边的完整判据链（五道门，全部 fail-closed）

调用链（逐跳已打开）：

```text
classifier/mod.rs:864  classify_with_tower_incremental      ← IncrementalClassifier::classify_at_with_l0 的实体
  ├─ :879  txn_bar = l0.merged_bars.last().source_index     ← ★证书的 bar 坐标（见 §4）
  ├─ :967 / :1521 / :1827  rebase_txn::emit(...)            ← 三个产出点，全在本函数体内
  └─ classifier/rebase_txn.rs:981  emit
       ├─ :1004  continued_center_pairs(strict)             ← 严格读法边
       ├─ :1006-1007 classify_edges(..., None) → continued_center_pairs(wide) ← 宽读法边（生产默认）
       └─ :1010  lineage_book::record_edges(bar, level, txn_id, digest, strict, wide)

backtest/fill.rs:1017  lineage_book::view_for(bar, lvl)     ← bar = step_center_oscillation_impl 的入参
backtest/fill.rs:1018  osc_books[lvl].on_chain_rebase_lineage(chain, lineage)
  └─ center_oscillation_trade.rs:788  ⟹ :862 写 anchor_of
```

**门一：链必须真的重基。** `on_chain_rebase_lineage` 只在 `ChainConsumed::Rebased` 分支被调用（`fill.rs:1004` 的 `match` 臂）。`Rebased` 的判据在 `classifier/center_lifecycle.rs:536-561`：

```rust
let k = self.consumed.len();
if k > 0 {
    let tail_same = chain.get(k - 1).map(|c| CenterId::of(c) == self.consumed[k - 1]).unwrap_or(false);
    if chain.len() < k || !tail_same { ... return Rebased ... }
}
```

即「链回缩」**或**「已消费末条身份被改写」。该守卫的**已登记盲区**（更深前缀被改写而末条不变）由 `:529-535` 自陈，本报告不改判。

**门二：挂起表非空。** `center_oscillation_trade.rs:794-796`：`if self.suspended.is_empty() { return }`。⟹ **没有挂起仓位时，`anchor_of` 一个字节都不会写**。这是 Q3 里对锚键最要紧的一条限制（§5.2）。

**门三：当前身份已不在新链上。** `:814-817`：

```rust
let current = self.current_of.get(&anchor).copied().unwrap_or(anchor);
if chain_ids.contains(&current) { continue; }   // 仍在链上：不建边，一动不动
```

⟹ 「跟随迁移」路径**不建边**（锚本来就没变，无需别名）。

**门四：谱系簿给出 `Continued`。** `:824` `book.lookup(current)`；四个 fail-closed 桶在 `:842-853`（`NoCert` / `Ambiguous` / `BarMismatch`）与 `:825-830`（`Continued` 但目标不在新链 ⟹ `target_absent`）。

**门五：目标未被第二个锚认领。** `:831-841`，`claimed` 表；重复认领 ⟹ `duplicate_claim`，双方全拒。

### 1.3 谱系簿里那条 `Continued` 边本身的判据（构造层）

`lineage_book::record_edges`（`rust/src/theta_v0/lineage_book.rs:198`）只收 `rebase_txn::continued_center_pairs` 的产出，后者的门槛（`rebase_txn.rs:757-759`）逐字：

```rust
if e.relation != Relation::Continued1To1
    || !(e.functional && e.injective && e.unique && e.bijective) { continue; }
```

而 `Relation::Continued1To1 ∧ 四布尔全真` 在 `classify_edges`（`rebase_txn.rs:543`）里**只有两条产出路径**：

- `:606-618`：旧、新实体按**种子键**唯一配对且 `1↔1`（`basis = "ordered_seed_lineage"`）；
- `:668-687`：同父窗口升级重切组 `a↔b`（a,b>1）**逐位**比较，`ok == new.seed_key()` 为真那几位（`basis = "positional_seed_lineage"`）。

**种子键** = 该输出对象**前三个**源单元的 `(level, ordinal)`（`rebase_txn.rs:245-252` `TxnNode::seed_key`）。生产默认走**宽读法**（`lineage_book.rs:100-110` `wide_reading()` 默认真，`THETA_REBASE_MIGRATE_STRICT=1` 才切严格），宽读法 = `classify_edges(.., None)` 不追下级 lineage 重映射，只比原始种子 ordinal（`rebase_txn.rs:1006`）。

**⟹ 「迁移 vs 重建」的分界线一句话**：**前三个构造种子的 `(level, ordinal)` 三元序列没变 ⟹ 迁移；变了 ⟹ 重建。** 中枢的 `zd`/`zg` 怎么漂都不进这条判据。

### 1.4 「拒绝过继，保持核销」（D1 裁定）在代码里落在哪

- **裁定原文**：`.chanlun/review-results/rebase-identity-d1-research-20260728.md:87`（seq=66 / bar 240310 / L1 / Long 那一行末列逐字「拒绝过继，保持核销」），归因「旧构造窗撤出 + 新构造窗建立；一旧一新，不是 1→N 分裂；同 ordinal `#121` 只是位置复用」。同文 `:406` 复述 `RebaseVanished 8→1`。
- **代码落点**：`center_oscillation_trade.rs:842-853` 四个 fail-closed 桶 + `:867-890` 的核销段——`vanished_anchors` 逐（侧, 锚）摘表、产 `SuspensionTerminationSource::RebaseVanished`、`cover_action: None`（不回补）。
- **口径声明**：`lineage_book.rs:21-22` 模块 doc 逐字「fail closed：证书缺失 / bar 不匹配 / 目标不在新链上 / 同一旧身份有多个候选 ⟹ 保持现有 `RebaseVanished` 核销 + 工程错误观测计数，**禁 fail-open**」。
- **实装验收**：`.chanlun/review-results/issue679-impl-20260729.md:171`（seq=66 判 `no_cert` 保持核销）、`:183`（8→1）。

---

## 2. Q2：实测 2,423 次 `seed_moved` 的构成

### 2.1 探针

新增 `rust/src/bin/p938_seed_moved_composition.rs`（**不改任何既有 `.rs`**）。它把每一次链尾 `CenterId` 变化按**生产判据同源**判成互斥八类，判据逐条对应 §1.2 的门一/门三/门四/门五：

```bash
cd rust
cargo build --release --features backtest_bin --bin p938_seed_moved_composition
./target/release/p938_seed_moved_composition ../analysis/data_cache/btc_1m_full.json 1000000
```

窗口：**BTC 1m 前 1,000,000 bar**（2017-08-17 04:00 起），与探针一**同窗口**，读数可直接对账（`tail_same + core_drift + seed_moved` 逐级、合计均与探针一一致：4,675,066 / 258 / 2,423）。耗时约 5 分 40 秒。

八类含义：

| 类 | 判据（生产同源 `file:line`） |
|---|---|
| `advanced` | `consume_chain` 返回 `Advanced`（`center_lifecycle.rs:594`）——链尾追加新中枢，旧尾**仍在链上**。**不是重基**，`on_chain_rebase_lineage` 根本不会被调 |
| `rebased_survived` | `Rebased` 但旧尾仍在新链 ⟹ 「跟随迁移」（`center_oscillation_trade.rs:815-817`） |
| `rebased_migrate` | `Rebased` + `Continued(new)` + `new ∈ 新链` ⟹ 生产**迁移身份锚**（`:831-835` → `:862`） |
| `rebased_target_absent` / `_no_cert` / `_ambiguous` / `_bar_mismatch` | `Rebased` + fail-closed 四桶（`:825-853`）⟹ 生产**保持 `RebaseVanished` 核销** = 判重建 |

### 2.2 `seed_moved`（`start_index` 变）——按级别拆开

| level | advanced | rebased_survived | rebased_migrate | target_absent | no_cert | ambiguous | bar_mismatch | 合计 |
|---|---|---|---|---|---|---|---|---|
| L0 | 1,866 | 0 | **0** | 0 | **10** | 0 | 0 | 1,876 |
| L1 | 424 | 0 | **0** | 0 | **7** | 0 | 0 | 431 |
| L2 | 93 | 0 | **0** | 0 | **1** | 0 | 0 | 94 |
| L3 | 20 | 0 | **0** | 0 | 0 | 0 | 0 | 20 |
| L4 | 2 | 0 | **0** | 0 | 0 | 0 | 0 | 2 |
| **合计** | **2,405** | 0 | **0** | 0 | **18** | 0 | 0 | **2,423** |

**读法（这一条是本报告的主结论，不许省）**：

1. **2,405 / 2,423 = 99.26% 是 `advanced`——不是重基，是链正常长了一节。** 探针一那张表把「`start_index` 变」笼统叫 `seed_moved`，实际上绝大多数是**新中枢诞生、旧中枢被 `Superseded`**（`center_lifecycle.rs:570-580`），教义上旧中枢确实**在场终结**了（`death_form() = ArenaTermination`，`:447`）。⟹ 这一族**两个键都会撤单，且撤得对**，不构成任何分歧。
2. **真重基只有 18 次**，**18/18 = 100% 判重建**（全在 `no_cert` 桶）。**零迁移。**
3. **按级别拆开后结论不翻转**：真重基集中在 L0（10）/ L1（7）/ L2（1），L3/L4 各 0；每一级的迁移比例都是 **0/n**。消费链跑在 L1+，L1 的读数是 7 次全判重建。**没有出现探针一 `core_drift` 那种「总体 9.62% 但 L4 占 71.43%」的级别反转。**

### 2.3 `core_drift`（`start_index` 不变、核心被改写）——对照组，按级别拆开

| level | advanced | rebased_survived | **rebased_migrate** | target_absent | no_cert | ambiguous | bar_mismatch | 合计 |
|---|---|---|---|---|---|---|---|---|
| L0 | 0 | 0 | **25** | 0 | 0 | 0 | 0 | 25 |
| L1 | 0 | 0 | **151** | 0 | 0 | 0 | 0 | 151 |
| L2 | 0 | 0 | **62** | 0 | 0 | 0 | 0 | 62 |
| L3 | 0 | 0 | **15** | 0 | 0 | 0 | 0 | 15 |
| L4 | 0 | 0 | **5** | 0 | 0 | 0 | 0 | 5 |
| **合计** | 0 | 0 | **258** | 0 | 0 | 0 | 0 | **258** |

**全部 258 次都是重基，且全部拿到 `continued_1to1` 证书迁移成功——每一级都是 100%，无级别反转。**

### 2.4 两表合起来看：证书与 `start_index` 在本窗上是同一个判据

| | 谱系判**迁移** | 谱系判**重建** |
|---|---|---|
| `start_index` **不变**（core_drift） | **258** | 0 |
| `start_index` **变**（真重基那 18 次） | **0** | **18** |

对角线满、反对角线空，**276 次真重基上零例外**。这跟 §1.3 的静态判据完全自洽：种子三元序列不变 ⟹ 证书连续，而 `start_index = 种子第一段的 `start_index``（`classifier/center.rs:228` / `:265` 逐字 `start_index: a.start_index`，即首段起点；`:83-84` doc「中枢 start_index 取首单元起点」）—— 种子首段不变则 `start_index` 不变。两者在构造上本来就高度耦合，本窗实测把这条耦合坐实为**逐帧重合**。

⚠️ **但耦合不等于恒等**：种子键是**三段的 `(level, ordinal)` 序列**，`start_index` 只反映**第一段的起点**。理论上「首段起点不变但第二/三段 ordinal 变了」（⟹ start_index 同、证书判重建）与「首段 ordinal 不变但起点被下级改写」（⟹ start_index 变、证书判迁移）都构成分歧样本。**本窗 276 次真重基里一个都没出现**，这是**实测缺席**，不是**结构性不可能**（090 照实）。

---

## 3. Q3：这个构成对两个键的实际后果

### 3.1 两个键各自何时撤单

| 事件族 | 次数 | 裸 `start_index` 键 | `resolve()` 锚键 | 一致？ |
|---|---|---|---|---|
| `advanced`（新中枢诞生，旧尾被 `Superseded`） | 2,405 | 键变 ⟹ **撤** | 未建任何别名（`resolve` 恒等）⟹ 键变 ⟹ **撤** | ✅ |
| 真重基 + 判重建（`no_cert`） | 18 | 键变 ⟹ **撤** | 拒绝过继、保持核销 ⟹ **撤** | ✅ |
| 真重基 + 判迁移（core_drift 那 258） | 258 | `start_index` 未变 ⟹ **不撤** | `anchor_of[new]=old` ⟹ `resolve(new)=old` ⟹ **不撤** | ✅ |
| 身份完全不变 | 4,675,066 | 不撤 | 不撤 | ✅ |

### 3.2 两个方向的错各多少（两侧都算，不只算一边）

- **裸 `start_index` 会误撤多少次**（本该保住却撤了 = `seed_moved` 里被谱系判为迁移的）：**0 次 / 2,423**（本窗，bar 次）。
- **锚会漏撤多少次**（本该撤却因锚没变而没撤 = `core_drift` 里被谱系判为重建的）：**0 次 / 258**（本窗，bar 次）。

**两侧都是零。在本窗上两个键逐帧给出同一串决定。**

### 3.3 按挂单存续期加权——**测不出来，照实说**

本探针**不重放买卖点触发、不构造挂单、不模拟撤单/成交**（与探针一同一口径声明）。因此：

- 上面所有数字都是 **bar 次**（链尾身份变化的帧数），**不是**挂单次数；
- 「一张单活了多少 bar、期间碰上几次身份变化」**没有测**，因而**无法**按存续期加权；
- 要拿到存续期加权的读数，必须跑完整 `fill` 回路并把限价单生命周期落账——那是另一个探针（须先有裁定三的实装 spec）。

**不许把「两侧都是 0 次」读成「换成挂单口径也是 0」**：0 是在「所有 bar × 所有级别」这个**超集**上测的，挂单只在其子集上活着，子集上的计数只会 ≤ 超集，**方向上仍是 0**——这一步外推**成立**，但「加权后的经济影响」（每次误撤/漏撤值多少钱）**完全未测**。

---

## 4. 顺带查出的一条：证书 bar 坐标与生产查簿 bar 坐标不是同一个数

- 证书产出点用 `txn_bar = l0.merged_bars.last().source_index`（`classifier/mod.rs:879`，doc `:874-875` 自称「与 trades/tower_events/rebase_observability 的 `bar` 同一坐标系」）；
- 生产查簿用 `step_center_oscillation_impl` 的入参 `bar`（`fill.rs:1017`），实参 = 回测主循环的 `i`（`fill.rs:5099`）。

**实测两者在 415,114 / 1,000,000 = 41.51% 的 bar 上不相等**（inclusion 合并把本 bar 吸收进上一根 merged bar 时，`source_index` 落后于 `i`）。

**但对本题无影响**：探针把两种 bar 各查一遍（表见 `/tmp` 原始输出与 bin 的「反事实对照」两节），**两种口径逐格完全相同**，`rebased_bar_mismatch` 两边都是 **0**。原因是：链尾发生重基的那 276 个 bar 上，`txn_bar == i` 恒成立（前 24 条样本逐条可见 `bar=… txn_bar=…` 相等）。

⚠️ **有效域**：这是「本窗上两坐标在重基 bar 上恰好一致」的**实测**，不是「它们恒等」的**证明**。`fail_bar_mismatch` 桶在生产里非零就说明这条不是永真——本窗只是没触发。**不建议**据此认定 `fill.rs:1017` 传 `i` 是安全的；这条差异值得单独挂一张票。

---

## 5. 有效域与未能确认项（090 照实）

### 5.1 已测的

- 单标的（BTC）、单窗（前 100 万 bar，2017-08 起）、`ThetaConfig::default()`、L0–L4 五级、4,677,747 帧链尾比对。
- 判据与生产同源：同一张 `classification.levels[lvl].centers`（`fill.rs:959`）、同一台 `CenterEventMachine`（`fill.rs:965`）、同一本 `lineage_book`（thread-local，同 bar 同线程）。

### 5.2 **没测**的（最要紧的一条在这）

1. **`anchor_of` 在生产里到底会不会被写，取决于挂起表非空**（门二，`center_oscillation_trade.rs:794-796`）。本探针测的是**谱系证书的可得性**，不是**挂起簿的实际状态**。限价单如果不走挂起簿（`suspended` 表键是 `(VoiceSide, CenterId)`，服务的是**中枢震荡短差挂起**，不是限价挂单），那么 `resolve()` 对限价单**根本不会有别名可查、恒等返回**——**「用锚为键」这条路要成立，先得回答「限价单凭什么进挂起簿」**。⚠️ 这一条本探针没有查，是 #938 裁定前必须补的前置。
2. **挂单存续期加权**（§3.3）。
3. **非链尾中枢**：本探针只看链尾。绑定中枢恒为链尾已由探针一在同窗证到 100%（零反例），故对**绑定中枢**这个用途是充分的；但对「任意中枢的身份稳定性」不充分。
4. **前缀守卫盲区**（`center_lifecycle.rs:529-535` 自陈）：「链长不变或增长、末条身份未变、但更深前缀被改写」这类分叉**检不出** ⟹ 本报告的 `advanced` 桶里理论上可能混有未检出的深层重基。**无检出即无观测**，本窗无证据说明其规模。
5. **其它标的 / 其它窗口**：未跑。跨标的外推无依据（本仓在案：`project_l3_cross_symbol_btc_idiosyncratic`）。
6. **严格读法（`THETA_REBASE_MIGRATE_STRICT=1`）下的构成**：未跑。生产默认宽读法，本报告全部读数是宽读法。

### 5.3 与已有结论的关系（开工前搜索的结果）

**#466**（OPEN，「重基身份保持：把 RebaseVanished 警报桶打到 0」）与其实装子票 **#679**（CLOSED，「LineageBook + 挂起随谱系迁移」）**建立了本题所依赖的全部机制**，但**没有回答本题的问题**：

- #466 票面范围是「让重基后挂起能跟住原中枢」+ 验收「wf8 RebaseVanished 跑到 0」——**样本是 wf8 的 8 条**；
- `rebase-identity-d1-research-20260728.md:80-87` 的八条案例归因表是本仓关于这件事的最细读数，**8 条 = 本报告 276 次真重基的 2.9%**；
- ADR 0019 `:158` 已裁「seq=66/59 两例中枢确已重建 ⟹ 撤单是正确语义」，与本报告 `no_cert` 那 18 条同向，**但 2 条 vs 18 条**；
- **`seed_moved` 那 2,423 次的构成，此前仓内无任何读数**——尤其「99.26% 根本不是重基」这条，此前没有任何文档提出过。

⟹ 本题**不在册**，未重复劳动。

---

## 6. 产物

- 探针：`rust/src/bin/p938_seed_moved_composition.rs`（新增，只读，不改任何既有 `.rs`）
- 原始输出：本报告 §2.2 / §2.3 / §4 的表格即探针 stdout 逐格转录（表头分组与「反事实对照」两节合并入 §4 叙述）
- **未入仓的临时物**：`analysis/data_cache/btc_1m_full.json` 在本 worktree 内是指向主仓同名文件的 **symlink**（`analysis/data_cache/` 在 `.gitignore` 覆盖内，不随 commit 进版本库）
