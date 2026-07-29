# pan 活窗真实频次探针 —— 票 #409

- 日期：2026-07-27
- 票：#409（`[task] pan 活窗真实频次探针：产窗数 + 反超触发数`，parent map #59，blocking #299；上游 #402 题三裁定）
- 工作面：`/tmp/kimi-nest-mainline`（worktree，分支 `kimi-nest-mainline-20260717`，HEAD `9e3cd25fd6`）
- 纪律：全程 **无 git mutation**；生产模块 `rust/src/theta_v0/**` **零改动**；只读探针走独立 bin
- v3 硬禁令合规：全部判据为确定性结构/力度谓词；无概率/统计推断、无策略回测、无 EMH 论证

---

## 0. 一句话结论（先说数字意味着什么）

**「反超」在真实行情上真的会发生，而且是多数结局，不是零发生。**

在 OKLO 前 10 万分钟（完整可跑窗，见 §3）上，pan 活窗一共产出 **213 个活假设**；其中 **95 个**在被观察到的那一刻就已经"可证背驰"，而这 95 个里 **88 个（92.6%）最终被力度反超打成 `Invalidated`**。也就是说：只要一个盘整背驰假设曾经成立过，它几乎必然会在后面某个时刻被自己打翻——中位数 **537 根 K 线**之后。

三件事同时被测出来，直接决定账本要接多完整：

1. **`Invalidated` 不是死代码**——不接 pan 产窗，账本确实退化成两态；接上了，三态全部真实可达。这条支持"接完整"。
2. **`first_provable_at` 的时钟精度问题不存在**——全部 95 个可证身份的首证时点**无一例外落在它被首次观察到的那个 prefix**（滞后 0）。所谓"时钟精度=触发点粒度"的粒度损失，实测为 **0**。
3. **但按现有契约照单全接，代价是失控的**——活窗右端跟着 `as_of` 走、永不冻结，于是每根 K 线、每个活身份都要重算一次力度、并往账本里追加一条 `Supersedes` 修订。10 万根 K 线上 213 个身份产生了 **563.7 万条修订记录**（单个身份最多 98282 条），CPU 成本随窗口长度约 **三次方**增长（§3 实测表）。**接线规格必须给活窗一个冻结/过期判据**，否则账本会被自己的延展记录淹没。

附带查清的第二问：**与 `CpLifecycleStatus{Pending, Closed}` 不构成功能重复**（§4，四条独立差异 + 一条代码内既有声明）。

---

## 1. 探针做了什么（可复现）

### 1.1 新增文件（唯一一个）

`rust/src/bin/p409_pan_live_probe.rs`（新增，独立 bin，自动 bin 发现，无 `required-features`）。
**不改动任何既有文件**——`rust/src/theta_v0/**`、`Cargo.toml`、测试全部原样。

### 1.2 上游输入怎么取（与生产逐字同调用链）

`provide_pan_live_windows`（`rust/src/theta_v0/classifier/nest_lifecycle.rs:862`）当前全项目无调用方，本探针自己搭调用。喂入的五个上游量按 `evaluate_run`（`rust/src/bin/p123_fast_replay.rs:1096-1145`）与 `collect_snapshot_candidates`（同文件 `:1146+`）的 run 分区逐字复制：

| 入参 | 取法 | 生产锚点 |
|---|---|---|
| `centers` | run 切片投影 `project_extended_windows_carried_only` 的 `seeds[].center` | `p123_fast_replay.rs:1109` |
| `kinds` | `decompose::decompose(&centers)` → `decompose::center_block_kind` | `level_view.rs:817-818` |
| `segments` | `lower_legs_from(&tower[ℓ-1])` 逐腿 `leg_as_segment` | `level_view.rs:734` |
| `anchors_self` | 逐段 `Some(segment.direction)`（**不是 `self_anchors`**） | `level_view.rs:735` |
| `as_of` | 当前 prefix 的 bar 下标（feed-every-prefix） | 卡 §6.2 |

`leg_as_segment`（`level_view.rs:436`，私有 fn）按票面授权在探针内**复制一份**（`p409_pan_live_probe.rs` 顶部），**未为探针把它改成 `pub`**——与 `rust/src/theta_v0/backtest/runner.rs:4957` 诊断臂的复制先例同款。

### 1.3 力度三值分布怎么取

`LifecycleObservation::force`（`nest_lifecycle.rs:407-428`）是私有 fn，探针**照其 pan 分支逐字复算**，用的是同一组生产原语（`recursive_tower::map_src_to_close_idx` + `divergence::segments_diverge_or`），没有引入第二查法。

复算结果与 book 自己的 revision 流做**逐 prefix 交叉校验**（`force_xcheck_mismatches`）：

- 复算 `Unavailable(x)` ⟺ 该 prefix 出 `ForceUnavailable{reason: x}`；
- 复算 `Verified(true)` ∧ 尚无 `first_provable` ⟺ 该 prefix 出 `FirstProvable`；
- 复算 `Verified(false)` ∧ 已有 `first_provable` ⟺ 该 prefix 出 `Invalidated{ForceOvertake}`。

**全部实跑 `force_xcheck_mismatches=0`。**

### 1.4 稀疏化与它的实测证明

活窗的**结构分量**（level / side / seg_a / c 窗左端 / b_center_start）只依赖 `tower[ℓ]`、`tower[ℓ-1]` 的内容；`as_of` 只经 (a) `segment.end_index <= as_of` 过滤、(b) 活窗右端 `seg_c_live.1 = as_of` 两处进入。腿端点恒 ≤ 其写入 bar ⟹ tower 内容不变期间 (a) 恒全通过（与 `p123_fast_replay.rs` 模块头 "(iii) ⊂ (ii)" 同一论据）。故探针只在 `cache.forest_epoch()` 变化的 bar 重算结构，其余 bar 只把右端推到 `as_of`。

**这不是靠推理收货的**：`P409_VERIFY=1` 打开逐 bar 全量重算并与缓存结果逐值对拍，计数器 `verify_mismatches`。实跑（OKLO 全窗臂 + BTC 5 万 bar 臂，见 §3）**全部 `verify_mismatches=0`**。

### 1.5 口径登记（090：声明 = 能力）

- **`structure_completed` 恒 `false`**。pan 的结构完成信号按契约"判据属调用方，本模块不越权自判"（`nest_lifecycle.rs:325-327`），生产侧无既存判据可读，探针**不自造**。⟹ 本探针**测不出 `Confirmed`**（实测 `confirmed=0` 全部来自这个口径，不是"确认从不发生"的证据）。票面四组数字均不依赖 `Confirmed`：反超判负在 `advance` 第 6 步，先于第 7 步的完成复核。
- **只投喂 pan 活窗通道**，不投喂事件通道（trend / pan 完成事件）——本票只测活窗。
- `Invalidated` 的两个原因码分开统计：`ForceOvertake`（= 票面"反超"）与 `IdentityVanished`（身份消失，非力度语义）。

### 1.6 复现命令

```bash
cd /tmp/kimi-nest-mainline/rust
cargo build --release --bin p409_pan_live_probe

# OKLO（全窗臂，带逐 bar 稀疏对拍）
P409_VERIFY=1 P409_PROGRESS=25000 P409_DUMP=/tmp/p409_oklo_entries.jsonl \
  ./target/release/p409_pan_live_probe ../analysis/data_cache/oklo_1m_databento.json

# OKLO 前 10 万（完整收口臂，本报告主数字来源）
P409_MAX_BARS=100000 P409_DUMP=/tmp/p409_oklo100k_entries.jsonl \
  ./target/release/p409_pan_live_probe ../analysis/data_cache/oklo_1m_databento.json

# BTC 前 50 万（切片臂）
P409_MAX_BARS=500000 P409_PROGRESS=25000 \
  ./target/release/p409_pan_live_probe ../analysis/data_cache/btc_1m_full.json

# BTC 前 5 万（稀疏对拍臂）
P409_MAX_BARS=50000 P409_VERIFY=1 \
  ./target/release/p409_pan_live_probe ../analysis/data_cache/btc_1m_full.json
```

数据文件（worktree symlink 到主仓 `analysis/data_cache/`）：
`oklo_1m_databento.json`（343282 bar，2024-05-10 → 2026-06-24）、`btc_1m_full.json`（4613599 bar，2017-08-17 → 2026-05-31）。

输出行前缀：`P409_TOTALS[...]` / `P409_LEVEL[...]` / `P409_FIRST_PROVABLE_LAG_HIST[...]`；`PARTIAL` 每 `P409_PROGRESS` 根落一次全量汇总（超预算被中止时仍有"跑到第 N bar 的完整四组数字"，不静默截断），`FINAL` 为收口值。

---

## 2. 四组数字

> 主数字窗 = **OKLO 前 100000 bar**（完整收口，`FINAL`）。BTC 与 OKLO 更长前缀的对照见 §3。
> 全部为**实测**；本节无推理成分（推理另见 §3 的外推、§4 的判定）。

### 2.1 产窗次数（按级别 ℓ 分解）

| 级别 | 活窗身份数（去重） | 逐 prefix 活窗观察数 |
|---|---|---|
| L1 | 178 | 5030996 |
| L2 | 32 | 543136 |
| L3 | 3 | 62887 |
| **合计** | **213** | **5637019** |

两个口径的区别：**身份数**是"一共冒出过多少个不同的盘整背驰活假设"（`(level, side, seg_a, c 窗左端, b_center_start)` 五元去重，即桥身份）；**观察数**是"逐 prefix 喂给账本多少次"（同一身份每延展一根 K 线算一次）。前者是"产窗次数"的语义口径，后者是接线的吞吐口径。

### 2.2 反超触发次数（按级别 ℓ 分解）

| 级别 | `Invalidated{ForceOvertake}` | `Invalidated{IdentityVanished}` | 终态 `Provisional`（未决） | `Confirmed` |
|---|---|---|---|---|
| L1 | 68 | 6 | 104 | 0（口径所致，§1.5） |
| L2 | 18 | 1 | 13 | 0 |
| L3 | 2 | 0 | 1 | 0 |
| **合计** | **88** | **7** | **118** | **0** |

**转化率（本轮最关键的一个比值）**：曾写入 `first_provable_at` 的身份共 **95** 个，其中 **88 个（92.6%）**最终被力度反超。剩下 7 个中 3 个仍 `Provisional`、其余走 `IdentityVanished`。

**反超前的存活时长**（`invalidated_at − observed_at`，单位：bar）：

| 分位 | p10 | p25 | 中位 | p75 | p90 | 最小 | 最大 |
|---|---|---|---|---|---|---|---|
| bar | 51 | 152 | 537.5 | 2813 | 6388 | 28 | 83078 |

`IdentityVanished` 的 7 例存活极短：18 / 40 / 103 / 120 / 169 / 223 / 627 bar。

118 个终态 `Provisional` 中只有 3 个曾可证；其余 **115 个从头到尾没有一个 prefix 力度成立**——它们既不会 `Confirmed` 也不会 `Invalidated`（反超定义要求"曾可证"，卡 §4.1），会永远挂在账本里。中位已存活 43861 bar。

### 2.3 `ForceCheck` 三值分布（逐 prefix 逐活窗；终态吸收后不再求值，与 `advance` 第 3 步同口径）

| 级别 | `Verified(true)` | `Verified(false)` | `Unavailable(MissingForceSeries)` | `Unavailable(CoordinateMapFailed)` |
|---|---|---|---|---|
| L1 | 217709（4.33%） | 4813287（95.67%） | 0 | 0 |
| L2 | 65572（12.07%） | 477564（87.93%） | 0 | 0 |
| L3 | 5042（8.02%） | 57845（91.98%） | 0 | 0 |
| **合计** | **288323（5.11%）** | **5348696（94.89%）** | **0（0.00%）** | **0（0.00%）** |

**`Unavailable` 占比 = 0.00%（两个原因码都是 0，全部四条实跑臂一致）。** 票面担心的"产窗条件与力度材料不匹配"在真实重放上**不成立**：`provide_pan_live_windows` 能产窗的每一个 prefix，`hist`/`dif` 都在、`map_src_to_close_idx` 都映得上。这条把 `Unavailable` 从接线形态的输入里划掉了（生产上只需要它作为防御分支，不需要为它设计通道）。

反过来，`Verified(false)` 占 94.89% 也不代表"背驰几乎不成立"——它主要来自活窗右端**永不冻结**（§0 第 3 点 / §5）：一个 c 窗被拖到几万根 K 线之后，同色面积几乎必然超过 A 段，于是恒 `false`。真正有信息量的是 §2.2 的身份级转化率，不是这里的 prefix 级比例。

### 2.4 `first_provable_at` 写入时点分布

```
P409_FIRST_PROVABLE_LAG_HIST[FINAL]  0:95
```

`first_provable_at − observed_at` 的直方图只有一个桶：**滞后 0，共 95 个身份，占曾可证身份的 100%**。

含义（对应票面"配合题一已裁『时钟精度=触发点粒度』，看粒度损失实际有多大"）：**粒度损失实测为 0**。活窗一旦被产出，力度要么当场就成立（当场写 `first_provable_at`），要么此后再也没成立过。不存在"先观察到、几十根之后才首次可证"的情形——因此把首证钟对齐到触发点粒度，在本窗上不丢任何精度。

四条实跑臂（OKLO 全窗臂各 PARTIAL、OKLO 100k、BTC 500k 臂各 PARTIAL、BTC 50k）的 lag 直方图**全部只有 `0:` 一个桶**，无一例外。

---

## 3. 跑了多少（截窗登记 + 外推依据）

### 3.1 实际跑到哪

票面定窗为「OKLO 全窗（343282 bar）+ BTC 前 50 万切片」。实跑如下，**逐条如实登记**：

| 臂 | 目标窗 | **实际跑到** | 收口 | 稀疏对拍 | 力度交叉校验 |
|---|---|---|---|---|---|
| OKLO 主臂 | 343282（全窗） | **175001 bar（50.98%）** | PARTIAL@175001，2123.3s 后人工中止 | `P409_VERIFY=1`，mismatches=0 | 0 |
| OKLO 收口臂 | 100000 | **100000（完整）** | **FINAL** | — | 0 |
| BTC 切片臂 | 500000 | **200001 bar（40.00%）** | PARTIAL@200001，2133.8s 后人工中止 | — | 0 |
| BTC 对拍臂 | 50000 | **50000（完整）** | **FINAL** | `P409_VERIFY=1`，mismatches=0 | 0 |

**未跑满的原因（成本，非数据缺失、非工具链问题）**：探针 CPU 成本随窗口长度约三次方增长，实测标度见 §3.2。**没有静默截断**——`PARTIAL` 汇总每 25000 bar 落一次完整四组数字，报告 §3.3 用的就是最后一次落盘值；进程由本工位在上表标注的耗时点主动中止（两臂各已占满一个核约 35 分钟）。

### 3.2 成本为什么爆炸（这是给 #402 接线规格的一手输入，不只是探针的麻烦）

三个因素相乘：

1. 活窗数 ~ O(窗口长度)：pan 活窗身份随中枢数线性累积（OKLO 实测同时在场活窗 `max_live_stems` 从 2 万 bar 的 35 → 10 万 bar 的 204）；
2. 每个活窗每根 K 线都要重算一次力度，而 c 窗是 `[c_start, as_of]`——**长度也 ~ O(窗口长度)**，因为契约里活窗右端永远跟着 `as_of` 走、没有冻结点；
3. feed-every-prefix ⟹ 每根 K 线全量重来一次。

⟹ 总成本 ≈ O(n³)。实测标度（OKLO 主臂，单线程 release）：

| bar | 累计秒 | 累计 emissions | 同时在场活窗 |
|---|---|---|---|
| 25001 | 5.5 | 302812 | 43 |
| 50001 | 40.1 | 1304154 | 98 |
| 75001 | 141.1 | 3089901 | 160 |
| 100001 | 365.5 | 5637137 | 204 |
| 125001 | 770.3 | 8892985 | 263 |
| 150001 | 1345.1 | 12754480 | 321 |
| 175001 | 2123.3 | 17194462 | 363 |

50k→100k：bar 数 ×2，耗时 ×9.1，emissions ×4.3；100k→175k：bar 数 ×1.75，耗时 ×5.8（`1.75³ = 5.36`）—— 与 O(n³) / O(n²) 的预期一致。按此外推 343282 bar 需 ≈ 4 小时单核（`(343/175)³ × 2123s ≈ 1.6×10⁴ s`），BTC 500k 需 ≈ 8 小时。**外推依据仅此标度表，不作为数字结论使用**——§2/§3.3 的所有数字都是实测值，不含外推。

### 3.3 更长前缀上的对照（PARTIAL，实测）

**OKLO @175001 bar（全窗的 51%）**

| 级别 | 活窗身份 | prefix 观察数 | `Verified(true)` | `Verified(false)` | `Unavailable` | 反超 | 身份消失 | 曾可证 | 仍 Provisional |
|---|---|---|---|---|---|---|---|---|---|
| L1 | 310 | 15192365 | 385631 | 14806734 | 0 | 132 | 14 | 140 | 164 |
| L2 | 63 | 1795173 | 249345 | 1545828 | 0 | 41 | 2 | 44 | 20 |
| L3 | 7 | 206924 | 25720 | 181204 | 0 | 4 | 0 | 5 | 3 |
| **合计** | **380** | **17194462** | **660696（3.84%）** | **16533766（96.16%）** | **0（0%）** | **177** | **16** | **189** | **187** |

曾可证 → 反超转化率 **177/189 = 93.7%**；`first_provable` lag 直方图 `0:189`（**全部滞后 0**）。

**BTC @200001 bar（50 万切片的 40%）**

| 级别 | 活窗身份 | prefix 观察数 | `Verified(true)` | `Verified(false)` | `Unavailable` | 反超 | 身份消失 | 曾可证 | 仍 Provisional |
|---|---|---|---|---|---|---|---|---|---|
| L1 | 418 | 19199923 | 205306 | 18994617 | 0 | 178 | 33 | 194 | 207 |
| L2 | 72 | 3224616 | 175269 | 3049347 | 0 | 40 | 3 | 41 | 29 |
| L3 | 12 | 479553 | 76782 | 402771 | 0 | 8 | 0 | 8 | 4 |
| **合计** | **502** | **22904092** | **457357（2.00%）** | **22446735（98.00%）** | **0（0%）** | **226** | **36** | **243** | **240** |

曾可证 → 反超转化率 **226/243 = 93.0%**；`first_provable` lag 直方图 `0:243`（**全部滞后 0**）。

**结论层面：四组数字的定性结构在更长前缀、跨品种上完全不变**——`Unavailable` 恒 0；lag 直方图恒只有 `0:` 一个桶；曾可证 → 反超转化率稳定在 93%±1（OKLO 100k 92.6% / OKLO 175k 93.7% / BTC 200k 93.0%）；L1 ≫ L2 ≫ L3 的级别分布稳定。**BTC 不是 OKLO 结论的反例，两品种同判。**

### 3.4 账本自身的体量（接线成本的直接量度）

OKLO 100k 收口臂：213 个身份、**5637209 条 revision 记录**、单身份最多 **98282 条**。
原因：活窗身份键的 `seg_c_full` 右端 = `as_of`，每根 K 线都变 ⟹ 每根 K 线每个活身份产生一条 `Supersedes` 修订（`nest_lifecycle.rs:591-604`，设计内行为、桥吸收"活窗延展"形态）。平均 **≈ 56 条修订 / bar**。

`retrograde_rejections = 0`（全部臂）——feed-every-prefix 下 as_of 严格单调，倒退守卫从未触发。

`assert_invariants()` 在每个收口臂末尾显式调用，**全部通过**（`P409_INVARIANTS ok`）。

---

## 4. 附带一并查清：只剩两态时与 `CpLifecycleStatus` 是否功能重复

**判定：不构成功能重复。** 五条依据，前四条是结构性差异，第五条是代码内既有声明。

先把票面前提修正一条：`Invalidated` **实测不是零发生**（§2.2），所以"退化为两态"的前提本身不成立。但即便退一步、在假想的两态情形下比，结论仍是不重复。

### 4.1 对象域不同（基数就对不上）

| | `CpScanOwnership.lifecycle` | `NestLifecycleBook` entry |
|---|---|---|
| 对象 | 一个已确认中枢 `B_p` 及其离开单元 `c_p` | 一个盘整背驰**活假设** |
| 身份 | `b_center_index`（= compose ordinal），与中枢/上级走势 **1:1**（`recursive_tower.rs:302-335`，`debug_assert_eq!(centers.len(), cp_ownership.len())`） | `LifecycleKey` 六元 `(level, side, kind, seg_a, seg_c_full, b_center_start)`，**同一中枢可挂多只**（不同 side / 不同 seg_a / 不同 c 窗） |
| 实测基数 | = 中枢数 | OKLO 100k：213 只，分布在 L1/L2/L3 |

一个是"每个中枢一份"的结构侧车，一个是"每个假设一份"的假设账本。前者的键里根本没有 `side`、没有 `seg_a`、没有力度语义。

### 4.2 闭合判据不同（一个是几何，一个是力度）

- `Pending → Closed` 由 `signal::judge_third_cert`（leave/retest 几何）触发（`recursive_tower.rs:1799-1846`）——**第三类买卖点结构证书**，全程不读 MACD。
- `Provisional → Confirmed` 由 `segments_diverge_or` 在完成窗上复核为真触发（`nest_lifecycle.rs:708-713`）——**力度背驰谓词**。
- `Provisional → Invalidated` 由同一力度谓词判负触发，CP 侧**没有任何对应转移**。

两者不是同一个判据的两种写法，是两套互不读取对方输入的判据。

### 4.3 单调性语义不同（这是最实质的一条）

票面把两者都描述为"单调只进不退"。**实测代码不是这样**：

- `CpLifecycleStatus` 的文档注释确实写"只能从 Pending 单调闭合为 Closed"（`recursive_tower.rs:452`），但 `invalidate_cp_lifecycle_dirty_dependencies`（`recursive_tower.rs:1444-1476`）里有一条 **`Closed → Pending` 的回退路径**：当 c_p 的 terminal 走势落入 dirty 前缀（frontier pop/recompose）时，`object.lifecycle = CpLifecycleStatus::Pending`，并清空 `cp_certificate_confirm_src` / `c_structure` / `third_class_in_c`。也就是说它的单调性**条件于底层 compose 前缀不变**，前缀一变就退回重判，且**不留痕**（旧证书直接被覆盖）。
- `NestLifecycleBook` 的终态是**吸收态且禁复活**（`advance` 第 1/3 步，E2E §1:83），底层结构变化不走"退回"而走**另外两条已命名的路**：桥身份成立 ⟹ 记 `Supersedes`（钟不动、链留痕）；桥身份不成立 ⟹ 记 `Invalidated{IdentityVanished}`（终态留档、禁删除）。

一个"可以悄悄退回重来"，一个"绝不退回、退不了就留档记死"。这两种语义不能互相替代——把账本换成 CP 的语义会直接违反 #64 裁定的可查账要求；把 CP 换成账本的语义会让 frontier 重折之后无法重判证书。

### 4.4 记录面与消费面不同

| | CP | 账本 |
|---|---|---|
| 钟 | 1 个（`cp_certificate_confirm_src`，首次可证 source-index） | 5 个（`observed_at` / `first_provable_at` / `structure_end_at` / `confirmed_at` / `invalidated_at`） |
| 审计留档 | 无修订链；回退时旧值被覆盖 | append-only `revisions` + `retrograde_rejections` + `ForceEvidence` 载荷 |
| 失效原因码 | 无（没有失效态） | 2 个（`ForceOvertake` / `IdentityVanished`） |
| 消费 | `Closed` 是证书装配的**唯一资格**（`relaxed_cand_delta_entries`，`recursive_tower.rs:1249-1257`），**在真值路径上** | 明确**不进真值路径**（模块头 090 登记 1；不进 `d_parent_interval_snapshot/terminal`、不改 `divergence_confirmed`、N^δ 不读） |

### 4.5 代码内既有声明

`nest_lifecycle.rs:488-491` 自述本 book 是"**范式复用**：…`recursive_tower.rs` `CpScanOwnership` 生命周期挂 book 内对象 + 显式推进函数…"。也就是说实装侧早已把两者关系登记为**同一编程范式、不同对象域**，而不是同一功能的两份实现。

### 4.6 唯一真实的共同点（如实登记，不夸大）

两者共享一个**抽象形状**：「一个挂在结构对象上的、带首证时钟的、以某个证书谓词闭合的侧车」。这是范式层面的重合，不是功能层面的重合——合并它们需要把"中枢几何证书"和"力度背驰假设"塞进同一个键域和同一套单调性契约，而 §4.1–§4.4 的四条差异说明这做不到。

**⟹ 本问答复：不重复，无需合并，`CpLifecycleStatus` 不能替代 `NestLifecycleBook` 的任何一态。** 该结论独立于 #229 §3（后者只排除了"改名"，如票面所述）。

---

## 5. 给 #402 接线规格的输入（只报事实，不替裁）

1. **`Invalidated` 通道值得接**：真实可达、且是曾可证假设的主导结局（92.6%）。不接产窗则该态生产恒空，账本实际两态。
2. **`Unavailable` 通道不需要为它设计形态**：实测 0%，只需保留防御分支。
3. **首证钟不需要更细的粒度**：滞后恒 0，触发点粒度零损失。
4. **必须给活窗一个冻结/过期判据**（本轮唯一的"红灯"）：
   - 现契约下活窗右端永远追 `as_of`，导致 (a) 力度判据在长窗上恒 `false`、失去分辨力；(b) 每 bar 每身份一条 `Supersedes`（实测 ≈56 条/bar）；(c) 成本 O(n³)。
   - 115/213 个身份从未可证却永久挂账（中位已存活 4.4 万 bar），既不 `Confirmed` 也不 `Invalidated`。
   - 这三条都指向同一个缺口：**`structure_completed` 的 pan 侧判据至今没有生产实装**（`nest_lifecycle.rs:325-327` 把它划给调用方）。接线规格必须先定这个判据，否则活窗只会无限延展。
5. **级别分布**：产窗与反超绝大多数在 L1（OKLO 100k：178/213 身份、68/88 反超），L3 只有个位数。若接线要分级别灰度，L1 是主战场。

---

## 6. 未做 / 未证（如实登记）

- **未跑满票面定窗**：OKLO 全窗与 BTC 前 50 万均未跑到底，原因=成本，登记见 §3.1/§3.2。所有报出的数字都是**实测**，无外推填充。
- **未测 `Confirmed`**：`structure_completed` 恒 `false`（§1.5），本探针对确认态无发言权。
- **未投喂事件通道**：trend 域"真实反超不可达"的既有结论（勘误 `erratum-v3-lifecycle-impl-20260721.md`）本轮**未做实测复核**，仍只有手工读码的单调性推理支撑。本票只测 pan 活窗。
- **未跨品种扩样**：只跑 OKLO / BTC 两个品种（票面定窗）。
- **交叉校验的键是三元不是五元**：§1.3 的 `force_xcheck` 用 `(level, seg_a, c 窗左端)` 索引本 prefix 的复算结果，未含 `side` / `b_center_start`。若两只同时在场的活窗在这三元上相撞，该次校验会被稀释（**不会造成假阳性，只会漏检**）。对 OKLO 100k 收口臂的 213 只身份做过键冲突扫描：**三元去重后仍是 213，零冲突**；更长臂未逐一扫描。
- **`P409_DUMP` 的 JSONL 用 Rust `Debug` 打印 `Option`**（`Some(1535)` / `None`），不是严格 JSON；下游解析需先做 `Some(x)→x` / `None→null` 替换（本报告 §2.2 的存活分位即如此解析）。
- **探针文件保留在 worktree 未提交**（禁 git mutation）；`cargo test --lib` 基线未动（未新增/修改任何测试，4 项在案失败与本票无关，未触碰）。

---

## 7. 实跑原始输出

日志原件：`/tmp/p409_oklo_full.log`（OKLO 主臂，`P409_VERIFY=1`）、`/tmp/p409_oklo100k.log`（OKLO 收口臂）、`/tmp/p409_btc500k.log`（BTC 切片臂）、`/tmp/p409_btc50k_verify.log`（BTC 对拍臂）；逐 entry dump：`/tmp/p409_oklo100k_entries.jsonl`（213 行）。

### 7.1 OKLO 收口臂（100000 bar，FINAL，本报告 §2 主数字来源）

```
P409_INPUT file=../analysis/data_cache/oklo_1m_databento.json bars=343282 replay_bars=100000 first_date=2024-05-10 08:02:00+00:00 last_date=2026-06-24 23:59:00+00:00 verify=false
P409_MODE channel=pan_live_only structure_completed=false feed=every_prefix sparse=forest_epoch_gated
P409_TOTALS[FINAL] bars_done=100000 emissions=5637019 identities=213 book_entries=213 recomputes=1135 max_live_stems=204 verify_mismatches=0 force_xcheck_mismatches=0 retrograde=0 elapsed_s=367.7
P409_LEVEL[FINAL] level=1 identities=178 emissions=5030996 force_true=217709 force_false=4813287 unavail_missing=0 unavail_coordmap=0 invalid_force_overtake=68 invalid_identity_vanished=6 confirmed=0 provisional_alive=104 entries_with_first_provable=73
P409_LEVEL[FINAL] level=2 identities=32 emissions=543136 force_true=65572 force_false=477564 unavail_missing=0 unavail_coordmap=0 invalid_force_overtake=18 invalid_identity_vanished=1 confirmed=0 provisional_alive=13 entries_with_first_provable=20
P409_LEVEL[FINAL] level=3 identities=3 emissions=62887 force_true=5042 force_false=57845 unavail_missing=0 unavail_coordmap=0 invalid_force_overtake=2 invalid_identity_vanished=0 confirmed=0 provisional_alive=1 entries_with_first_provable=2
P409_FIRST_PROVABLE_LAG_HIST[FINAL] 0:95
P409_INVARIANTS ok
```

### 7.2 BTC 对拍臂（50000 bar，FINAL，`P409_VERIFY=1`）

```
P409_TOTALS[FINAL] bars_done=50000 emissions=1359779 identities=117 book_entries=117 recomputes=792 max_live_stems=107 verify_mismatches=0 force_xcheck_mismatches=0 retrograde=0 elapsed_s=28.8
P409_LEVEL[FINAL] level=1 identities=92 emissions=996170 force_true=35973 force_false=960197 unavail_missing=0 unavail_coordmap=0 invalid_force_overtake=41 invalid_identity_vanished=10 confirmed=0 provisional_alive=41 entries_with_first_provable=44
P409_LEVEL[FINAL] level=2 identities=20 emissions=294407 force_true=34591 force_false=259816 unavail_missing=0 unavail_coordmap=0 invalid_force_overtake=10 invalid_identity_vanished=0 confirmed=0 provisional_alive=10 entries_with_first_provable=11
P409_LEVEL[FINAL] level=3 identities=5 emissions=69202 force_true=17770 force_false=51432 unavail_missing=0 unavail_coordmap=0 invalid_force_overtake=3 invalid_identity_vanished=0 confirmed=0 provisional_alive=2 entries_with_first_provable=3
P409_FIRST_PROVABLE_LAG_HIST[FINAL] 0:58
P409_INVARIANTS ok
```

### 7.3 OKLO 主臂末次 PARTIAL（175001 bar，`P409_VERIFY=1`）

```
P409_TOTALS[PARTIAL] bars_done=175001 emissions=17194462 identities=380 book_entries=380 recomputes=1701 max_live_stems=363 verify_mismatches=0 force_xcheck_mismatches=0 retrograde=0 elapsed_s=2123.3
P409_LEVEL[PARTIAL] level=1 identities=310 emissions=15192365 force_true=385631 force_false=14806734 unavail_missing=0 unavail_coordmap=0 invalid_force_overtake=132 invalid_identity_vanished=14 confirmed=0 provisional_alive=164 entries_with_first_provable=140
P409_LEVEL[PARTIAL] level=2 identities=63 emissions=1795173 force_true=249345 force_false=1545828 unavail_missing=0 unavail_coordmap=0 invalid_force_overtake=41 invalid_identity_vanished=2 confirmed=0 provisional_alive=20 entries_with_first_provable=44
P409_LEVEL[PARTIAL] level=3 identities=7 emissions=206924 force_true=25720 force_false=181204 unavail_missing=0 unavail_coordmap=0 invalid_force_overtake=4 invalid_identity_vanished=0 confirmed=0 provisional_alive=3 entries_with_first_provable=5
P409_FIRST_PROVABLE_LAG_HIST[PARTIAL] 0:189
```

### 7.4 BTC 切片臂末次 PARTIAL（200001 bar）

```
P409_TOTALS[PARTIAL] bars_done=200001 emissions=22904092 identities=502 book_entries=502 recomputes=2060 max_live_stems=463 verify_mismatches=0 force_xcheck_mismatches=0 retrograde=0 elapsed_s=2133.8
P409_LEVEL[PARTIAL] level=1 identities=418 emissions=19199923 force_true=205306 force_false=18994617 unavail_missing=0 unavail_coordmap=0 invalid_force_overtake=178 invalid_identity_vanished=33 confirmed=0 provisional_alive=207 entries_with_first_provable=194
P409_LEVEL[PARTIAL] level=2 identities=72 emissions=3224616 force_true=175269 force_false=3049347 unavail_missing=0 unavail_coordmap=0 invalid_force_overtake=40 invalid_identity_vanished=3 confirmed=0 provisional_alive=29 entries_with_first_provable=41
P409_LEVEL[PARTIAL] level=3 identities=12 emissions=479553 force_true=76782 force_false=402771 unavail_missing=0 unavail_coordmap=0 invalid_force_overtake=8 invalid_identity_vanished=0 confirmed=0 provisional_alive=4 entries_with_first_provable=8
P409_FIRST_PROVABLE_LAG_HIST[PARTIAL] 0:243
```

### 7.5 §2.2 存活分位的复算脚本（对 dump 的一次性解析，未落盘为文件）

```python
import json, re, statistics
from collections import Counter
def fix(l):  # Rust Debug 的 Option 不是合法 JSON
    l = re.sub(r'Some\((-?\d+)\)', r'\1', l)
    l = l.replace('"Some(ForceOvertake)"', '"ForceOvertake"') \
         .replace('"Some(IdentityVanished)"', '"IdentityVanished"')
    return re.sub(r':None', ':null', l)
rows = [json.loads(fix(l)) for l in open('/tmp/p409_oklo100k_entries.jsonl')]
ov = [r for r in rows if r['reason'] == 'ForceOvertake']
lives = sorted(r['invalidated_at'] - r['observed_at'] for r in ov)
# -> n=88, min=28, p25=152, median=537.5, p75=2813, p90=6388, max=83078
```
