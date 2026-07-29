# #553（N1-T4）验收旁缝：p123 事件 dump + 全量双跑封印（2026-07-28）

分支 `ticket-553`，基座 = #552 tip `e8496fcaf0`（T1–T3 全部成果在底座上）。执行器 = claude CLI Opus 5 无头。
本票 2 枚逻辑提交：`dc0d61052e`（候选事件 dump 侧信道 + 红绿双测）、本报告 commit。

## 结论（一句话）

p123 dump 侧信道按票扩出**第三路**候选事件 dump（`P123_EVENT_DUMP`，独立 sink 独立文件、只写不判、
零判定消费），既有 `P116_DUMP` 全套行（CERT/DIV/TERM/TURN/TURN_CLASS/FALLBACK 含行序）逐字节不动。
双跑封印在 **2,000,001 bar**（主）与 **800,001 bar**（副）两个窗口各自成立：既有 dump / 事件 dump /
stdout 三面 diff=0，双侧 SHA-256 逐位相等。既有行在「改动前 binary / 关灯 / 开灯 / 开灯第二跑」
四态间逐字节恒等。**全量 4,613,599 bar 的 terminal pass 三路走完、prefix pass 推进到 1M 里程碑
（三路计数逐位一致）后因段耗时超线性（外推 >16h/单跑）显式中止**——照实登记，非静默截断，
亦非判负：全程无一处 diff≠0，#69 停线未触发，判据代码零改动。

---

## 一、关票门六子句

| # | 子句 | 状态 | 证据落点 |
|---|---|---|---|
| 1 | dump 口径文档 | **PASS** | §三 字段表 + `p123_fast_replay.rs:371-395` 模块内文档 |
| 2 | 双跑 diff=0 | **PASS（窗口缩小，见登记 1）** | §5.2（2M 窗）与 §5.3（800k 窗）：既有行 / 事件行 / stdout 三面各自 diff=0 |
| 3 | 双侧 SHA-256 | **PASS** | §5.2 / §5.3 SHA 表（两窗各三面双侧逐位相等） |
| 4 | 窗口 / bar 数 / 耗时 | **PASS** | §5.1 全量（terminal 全窗 4,613,599 bar 走完、prefix 至 1M 中止）+ §5.2（2,000,001 bar，5h43m）+ §5.3（800,001 bar，24m×2） |
| 5 | 既有封印未破证据（既有行双跑亦 diff=0） | **PASS** | §5.2 / §5.3 两窗既有行双跑 diff=0 + SHA 相等；§四.3 200k / 800k 三态对照（改动前 binary 参与，SHA 逐字节相同） |
| 6 | 计数三元组 + 快照 commit + 090 登记 | **PASS** | §六（三元组 + 快照）、§八（090 照实登记） |

## 二、验收逐条（票体）

| # | 验收项 | 状态 | 证据 |
|---|---|---|---|
| 1 | 事件 dump env-gated（仿既有开关模式） | **PASS** | `EventDump::from_env`（`:413-423`）读 `P123_EVENT_DUMP`；未设 ⟹ `writer=None` |
| 2 | 只写不判（零判定消费、零生产路径读取） | **PASS** | §四.1 构造性论证 + 全仓 grep 读点清单（生产路径仅 3 处：构造 `:1134`、观察 `:1415`、收尾 `:1582`） |
| 3 | dump 内容 = 候选事件流逐条（key/级别/状态/钟/区间/修订号） | **PASS** | §三 字段表（16 字段，全部落一行） |
| 4 | 既有 dump 行逐字节不动、既有封印不破 | **PASS** | §四.2 构造性 + §四.3（200k/800k 含改动前 binary）/ §5.2、§5.3（两窗双跑 SHA 相等）实测 |
| 5 | 全量双跑 diff=0 + 双侧 SHA-256 | **PASS（窗口缩小）** | §五：2M / 800k 两窗封印成立；全量 prefix 中止照实登记（§5.1、§八 登记 1） |
| 6 | 判负即停线禁调判据 | **未触发** | 三面 diff 全 0，无判负现场；本票对判据代码零改动（§四.4 diff 范围） |
| 7 | 验收报告入仓 | **PASS** | 本文件 |
| 8 | 对照族零改动 / 只读数据文件禁改禁删 | **PASS** | §四.4：本票 diff = 单文件 `rust/src/bin/p123_fast_replay.rs`（+196/−1）；数据文件只读打开，`ls -l` 时间戳未变 |
| 9 | TDD 红绿（dump 行的产生与门控先红后绿） | **PASS** | §四.5 |
| 10 | rustfmt 单文件 check | **PASS** | `rustfmt --edition 2021 --check src/bin/p123_fast_replay.rs` 干净（**禁全仓 fmt**：本仓非 rustfmt-clean） |
| 11 | #535 四类禁项不触 | **PASS** | §七 |
| 12 | GitHub issue 状态 | **未做（照实登记）** | 执行指令明令不关票，见 §八 登记 4 |

---

## 三、dump 口径文档

### 3.1 门控与落点

- **env**：`P123_EVENT_DUMP=<path>`。未设 ⟹ 关灯：`observe` 首行 `let Some(writer) = … else { return Ok(()) }`
  立即返回——不记 revision、不推 seq、不格式化，成本 = 一次 `Option::is_none`。
- **sink**：本路自有 `Box<dyn Write>`（文件由 `File::create` 独占），**与 `P116_DUMP` 的
  `static DUMP: OnceLock<Option<Mutex<BufWriter<File>>>>`（`:191`）分属两个对象两个文件**。
  同理与 `P421_LIFECYCLE_DUMP` 也互不相干。
- **落盘点**：prefix replay 的事件应用循环（`:1409-1418`），每观察到一条候选事件写一行。
  写在既有 pending 短路**之前** ⟹ 记录的是完整应用流（含同一 trigger 内被前序事件出清后
  仍到达的观察，落 `pending=0`）。

### 3.2 行格式（字段序固定，一行一条观察）

```
EVENT seq=<n> rev=<r> as_of=<bar> level=<ℓ> side=<Long|Short> kind=<trend|pan> div=<0|1>
      pending=<0|1> turn_source=<t> judge_at=<j> seg_a=<s>..<e> interval_b=<s>..<e>
      interval_a=<s>..<e> provider_window=<s>..<e> b_center_start=<b> intake_fallback=<0|1>
```

| 轴 | 字段 | 口径 |
|---|---|---|
| **key** | `level` `side` `kind` `seg_a` `interval_b` `interval_a` `turn_source` | 与 `EventKey`（`:217-240`）字段一一对应——即 p123 账本（candidates/divergences/term_seen）与 pending 出清所用的同一身份键。`kind` 标签复用既有 `nest_kind_tag`（trend/pan），与 DIV 行同词表 |
| **级别** | `level` | 事件级别 ℓ（与 DIV/TERM 行的 `level=` 同源同值） |
| **状态** | `div` | `divergence_confirmed`（背驰确认位）。**不进 key** ⟹ 同一 key 的状态迁移可在 rev 递增序列里读出 |
| **状态** | `pending` | 观察瞬间该 key 是否仍在 pending 集合中（1=仍待出清，0=同 trigger 内已被前序事件出清） |
| **状态** | `intake_fallback` | 旧进料口会丢弃该候选的审计标记（诊断口径，不进 N 真值） |
| **钟** | `as_of` | 观察 bar（= prefix replay 的当前 bar index，= 慢版 `or_insert(index)` 归集 bar） |
| **钟** | `judge_at` | provider 判定钟（`NestCandidateEvent.judge_at`，CERT 行首证钟主键的同一字段） |
| **钟** | `turn_source` | 拐点源坐标（DIV/TERM 行的 `end=`/`bar=` 同值） |
| **区间** | `seg_a` `interval_b` `interval_a` | 事件三区间，闭区间 `start..end`（源坐标） |
| **区间** | `provider_window` | provider 合法 run 的源坐标窗（prefix replay 路由用，不进 N 真值） |
| **区间** | `b_center_start` | B 中枢身份快照（`start_index`，首次观察快照纪律） |
| **修订号** | `rev` | 同一 `EventKey` 被候选事件流观察的次数，**1 基**。同 key 复现 ⟹ rev 递增 |
| **行序** | `seq` | 全局落盘行序号，**0 基**，单调 +1（= 候选事件流的观察顺序） |

### 3.3 覆盖面（诚实边界）

本路落盘的是 **targeted prefix replay 的候选事件应用流**——即每个 trigger bar 上、与当前 pending
求交后进入应用循环的事件序列。它**不是** provider 的全部物理产出（`RunEntry.events` 里已被 pending
过滤掉的部分不落盘），也**不覆盖** #421 sidecar 的活假设窗口（那是 `P421_LIFECYCLE_DUMP` 的面）。
边界如实声明，不作膨胀（090）。

推论：一个 key 一旦终态匹配出清（`event.divergence_confirmed == target.divergence_confirmed`），
其后不再进入应用流 ⟹ 不再有该 key 的 EVENT 行。实测（§5.4）：200k / 800k 两窗 rev 恒 1
（首次进入应用流即出清），**2M 窗出现 1 个 key 观察 4 次**（rev=1..4）——即该 key 的 `div` 位
在被终态匹配前迁移过，rev 字段因此在真实数据上**非平凡**、可读出状态迁移。

---

## 四、只写不判 / 既有封印未破

### 4.1 零判定消费（构造性 + 静态证据）

`EventDump` 只有 3 个字段：`writer` / `revisions` / `seq`。全仓 grep（`EventDump|event_dump|\.revisions|\.seq`）
的生产路径出现点**只有 3 处**：

| 位置 | 操作 |
|---|---|
| `:1134` | `let mut event_dump = EventDump::from_env()?;`（构造） |
| `:1415` | `event_dump.observe(&event, index, pending_hit)?;`（写） |
| `:1582` | `event_dump.flush()?;`（收尾刷盘） |

`revisions` / `seq` 的读写全部在 `observe` 体内（`:434` `:440` `:461`），**无第二读点**。
账本（`YieldBook.candidates/divergences/term_seen`）、dirty 判据（(i)–(iv)）、pending 出清、
证书装配、stdout 门行、`SparseStats` 全部不读本结构；`observe` 取 `&NestCandidateEvent` 不可变借用，
**不回写事件**。其余出现点均在模块头文档与 `#[cfg(test)]` 内。

### 4.2 既有封印零扰动（构造性）

1. **sink 分离**：本路 writer 与 `static DUMP` 是两个独立对象、两个独立文件；既有 `dump_line`
   的调用点、条件与顺序一行未改。
2. **控制流不动**：调用点唯一的既有行改写是把 `if !pending.contains(&key)` 拆成
   `let pending_hit = pending.contains(&key);` + `if !pending_hit`。`BTreeSet::contains` 是纯查询，
   提出到局部变量后中间只插入一次 `observe`（不触 `pending`）⟹ 分支条件逐值相同、既有 dump 行
   （DIV/TERM）的产生条件与落盘顺序逐位不变。
3. **关灯零行为差异**：env 未设时 `observe` 首行返回，既不建 `BTreeMap` 条目也不推 seq。

### 4.3 200k / 800k 三方对照（含改动前 binary）

| 侧 | binary | env |
|---|---|---|
| A（基线） | 改动前（基座 `e8496fcaf0` 构建） | `P116_DUMP` |
| B | 本票 `dc0d61052e` 构建 | `P116_DUMP`（关灯） |
| C | 本票 `dc0d61052e` 构建 | `P116_DUMP` + `P123_EVENT_DUMP`（开灯） |

**200k 窗口**（`P116_MAX_BARS=200001`）：

- `diff(A.dump, B.dump)` = **0**；`diff(A.dump, C.dump)` = **0**（667 行：CERT 156 / DIV 360 / FALLBACK 20 / TERM 76 / TURN 55）
- `diff(B.stdout, C.stdout)` = **0**（13 行门行全套）
- C 的事件 dump = **719 行** = 门行 `P123_YIELD candidates=719`；其中 `div=1` 计 **360** = 门行
  `divergence_confirmed=360`；`div=0` 计 359。⟹ 新路与既有门行计数逐位互证，非真空绿。

**800k 窗口**（`P116_MAX_BARS=800001`，同三态）：

| 侧 | 既有 dump SHA-256 | 行数 |
|---|---|---|
| A 改动前 binary | `38039a6da139e2984ff79586aa952a5f524827d77ea7111fba3f1f5a8ba396e8` | 3,085 |
| B 本票 binary 关灯 | 同上（逐字节相同） | 3,085 |
| C 本票 binary 开灯 | 同上（逐字节相同） | 3,085 |

stdout `diff(A,C)` = **0**、`diff(C,B)` = **0**。C 的事件 dump = **3,213 行** = 门行
`candidates=3213`；`div=1` 计 **1,586** = 门行 `divergence_confirmed=1586`；`div=0` 计 1,627。

这是"既有封印未破"的**最强口径**证据：对照的一侧是改动**前**的二进制，两个窗口尺度各自 SHA 相等。
全量路径（§5.1）改动前 binary 未参与对照（成本），且三路在 prefix 中止，照实登记（§八 登记 1、2）。

### 4.4 diff 范围与对照族

- 本票 `git diff e8496fcaf0 --stat` = `rust/src/bin/p123_fast_replay.rs | 196 +++/1 −`，**单文件**。
- 判据代码（dirty (i)–(iv)、trigger 语义、TERM 反查）零改动；lib（`rust/src/theta_v0/**`）零改动 ⟹
  对照族与生产路径零触碰。
- 数据文件 `/tmp/kimi-nest-mainline/analysis/data_cache/btc_1m_full.json`（→ `analysis/data_cache/btc_1m_full.json`）
  只读打开；SHA-256 `16ea13d55f2ae7edcfc503f604a14961fd1a378227afd37c63f23386e894707b`，
  mtime `2026-06-25 10:27` 未变。

### 4.5 TDD 红绿

- **红**：先落两测（`event_dump_disabled_writes_nothing` `:2578` / `event_dump_line_format_and_revision_monotonic` `:2589`）
  与测试夹具（`sample_event` `:2559`、`SharedSink` `:2622`），`EventDump` 尚不存在 ⟹
  `cargo test --bin p123_fast_replay` = `error[E0433]: cannot find type EventDump in this scope`（2 处）。
- **绿**：实装 `EventDump` 后 `cargo test --bin p123_fast_replay` = **4 passed / 0 failed**
  （新 2 + 既有 2）。
- 两测分管票体的两个要求：**门控**（关灯零落盘零副作用）与**行的产生**（16 字段逐字断言 +
  rev 同 key 递增 / 异 key 归 1、seq 全局单调）。

---

## 五、全量双跑封印（#69 协议）

### 5.0 数据与 binary（三个窗口共用）

- **数据**：`analysis/data_cache/btc_1m_full.json`（软链 `/tmp/kimi-nest-mainline/…`，只读打开），
  SHA-256 `16ea13d5…707b`；全量 **4,613,599 bar**（首 `2017-08-17 04:00:00`，末 `2026-05-31 23:59:00`）。
- **binary**：commit `dc0d61052e` 的 `cargo build --release --bin p123_fast_replay` 产物，
  SHA-256 `cbf5e4d5e2cd05d0e7be6a52e04b8de3a97ecf695845673a955b173e37333dc2`
  （`/tmp/wt553_p123_t4` 与其副本 `/tmp/wt553_p123_800k`、`/tmp/wt553_p123_win` 同 SHA）。
- 全部跑均未设 `P116_CKPT`（CKPT 侧信道验收本就关闭）、未设 `P123_SHADOW`（长期回归开关，不进验收面）。

### 5.1 全量 4,613,599 bar 三路跑：terminal pass 走完，prefix pass 中止（照实登记，禁静默截断）

**已完成部分**：全窗 4,613,599 bar 的 terminal pass 三路全部走完（末里程碑 `bar=4500000` 三路
elapsed 9256.9s / 9302.9s / 9280.4s），prefix pass 推进到 **1,000,000 bar 里程碑**：

| 跑 | env | prefix@1M 读数 |
|---|---|---|
| run1 | `P116_DUMP` + `P123_EVENT_DUMP` | `elapsed=2354.9s pending=14425/18297 views=17528 triggers=9291 reevals=17528` |
| run2 | 同上 | `elapsed=2353.6s pending=14425/18297 views=17528 triggers=9291 reevals=17528` |
| run3 | `P116_DUMP`（关灯） | `elapsed=2347.3s pending=14425/18297 views=17528 triggers=9291 reevals=17528` |

三路在 1M 里程碑的**语义计数逐位相同**（pending/targets/views/triggers/reevals 五项），墙钟差 ≤7.6s。
这是开灯/关灯/双跑三态在全量路径上的中途一致性读数。

**中止理由（照实）**：prefix 段耗时超线性——0→0.5M 段 295s、0.5M→1M 段 2,060s（7.0×），
1M→1.5M 段在 41 分钟后仍未收口。按 #69 同族报告的段形状（4.0M→4.5M 段为 0.5M→1M 段的 ~20×）
外推，本次全量 prefix 余下 7 段合计 >4×10⁴s ⟹ 单次全量 >16h。
本票遂按票体「跑不动就取最大可行窗口并照实登记」条款，在 `2026-07-29 00:08` 中止三路
（run3 提前于 `23:27` 中止以降并发），封印面改由 §5.2 的 **2,000,001 bar** 与 §5.3 的
**800,001 bar** 两个窗口承担。中止是**显式登记的**，不是静默截断：本节给出中止时刻、
已完成 bar 数与三路里程碑读数。

**不作判负**：中止的原因是墙钟成本，不是任何 diff≠0。三路自始至终无一处 diff 现场，
#69 判负即停线未触发，判据代码零改动（§四.4）。

### 5.2 主封印：2,000,001 bar 双跑

- 两侧同配置（`P116_MAX_BARS=2000001`，`P116_DUMP` + `P123_EVENT_DUMP`，路径不同），同一 binary，
  同机并发两路：`2026-07-29 00:08:50 → 05:51`（a）/ `05:52`（b），**墙钟 ≈ 5h43m**（并发口径）。
  其中 terminal pass 1,553.5s；prefix 段耗时 280.6s（0–0.5M）/ 2,183s（0.5–1M）/ 5,712s（1–1.5M）/
  ≈10,724s（1.5–2M）——超线性形态与 §5.1 同源。
- 窗口 = 前 **2,000,001 bar**（`P123_INPUT bars=4613599 replay_bars=2000001`；数据全窗仍为 4,613,599）。

| 面 | SHA-256（双侧相同） | 规模 |
|---|---|---|
| 既有 dump | `c26904bac2068f67a35659adf3464629d08c8513465016bbfe09c8338f39bbe9` | 8,106 行 / 662,787 B |
| 事件 dump（本票新路） | `627a1227ed13befc0931b014ffc141af38a9c1d05ffa7e53ec1789e3ce7dc8ea` | 8,076 行 / 2,080,595 B |
| stdout 门行 | `4ecc346b784b596f2655ac6f9a792180112d1ee3ac17e9a5ccfe709fc84b6c03` | 21 行 |

`diff` 三面全 **0**（`DIFF0_2M_DUMP` / `DIFF0_2M_EVENTS` / `DIFF0_2M_STDOUT`）。
产物：`/tmp/wt553_2m_{a,b}_{dump,events}.txt`、`/tmp/wt553_2m_{a,b}.{out,err}`，
封印计算落 `/tmp/wt553_seal2m.txt`；800k 侧产物 `/tmp/wt553_800k_{new,on2,newoff}_*`，
全量侧产物 `/tmp/wt553_full_run{1,2,3}.{out,err}`（dump 为中止时的未完写入，不作对拍面）。

既有 dump 行类计数（含行序 diff=0）：CERT 2,460 / DIV 3,971 / TERM 1,033 / TURN 620 / FALLBACK 22。
门行关键读数：`candidates=8073 divergence_confirmed=3971 terminal_confirmed=469`、
`PROVIDER complete=true unresolved_targets=0`、`SNAPSHOT future_violations=0 no_forward=true`、
`R7 provider_complete=true definition_faithful=true`、`MISSED missed=0`。

### 5.3 副封印：800,001 bar 双跑（另含改动前 binary 与关灯三态，见 §四.3）

- 两侧同配置（`P116_MAX_BARS=800001`，`P116_DUMP` + `P123_EVENT_DUMP`），同一 binary，
  先后两次独立跑：on1 `20:09→20:33`（24m）、on2 `00:08:50→00:33:12`（24m22s）。

| 面 | SHA-256（双侧相同） | 规模 |
|---|---|---|
| 既有 dump（CERT/DIV/TERM/TURN/TURN_CLASS/FALLBACK） | `38039a6da139e2984ff79586aa952a5f524827d77ea7111fba3f1f5a8ba396e8` | 3,085 行 / 235,540 B |
| 事件 dump（本票新路） | `e28342216132c4e22b19694a356972ca82055f2b57e4dc16820a66066572bfec` | 3,213 行 / 806,690 B |
| stdout 门行 | `diff` = 0 | 13 行 |

`diff` 三面全 **0**（`DIFF0_800K_DUMP_2RUN` / `DIFF0_800K_EVENTS_2RUN` / `DIFF0_800K_STDOUT_2RUN`）。
既有 dump 的 SHA 同时等于改动**前** binary 与本票关灯侧的 SHA（§四.3 三态表）⟹
**既有行在「改动前 / 关灯 / 开灯 / 开灯第二跑」四态间逐字节恒等**。

### 5.4 事件 dump 读数（2M 窗口，与门行逐项互证）

| 维度 | 事件 dump 侧 | 既有门行侧 | 一致性 |
|---|---|---|---|
| 唯一 `EventKey` 数 | `rev=1` 计 **8,073** | `P123_YIELD candidates=8073` | ✔ 逐位 |
| 背驰确认 | `div=1` 计 **3,971** | `divergence_confirmed=3971`；DIV 行 3,971 | ✔ 逐位（三方） |
| 未确认 | `div=0` 计 4,105 | — | 4,105 + 3,971 = 8,076 = 总行数 |
| 盘整候选 | `kind=pan` 计 **7,281** | `pan_candidates=7281` | ✔ 逐位 |
| 趋势候选 | `kind=trend` 计 795 | `trend_candidates=792` | 差 3 = 重复观察行（下行） |
| 修订号分布 | `rev=1` 8,073 / `rev=2` 1 / `rev=3` 1 / `rev=4` 1 | — | **1 个 key 被观察 4 次**；8,073 + 3 = 8,076 总行。该 key 属 trend 类 ⟹ 恰好解释 trend 计数差 3 |
| pending 命中 | `pending=1` 8,076（全部） | — | 本数据无同 trigger 内前序出清的情形 |
| 级别分布 | L1 6,357 / L2 1,401 / L3 278 / L4 40 | — | 与门行 `BIT_DIFF_LEVEL` 铺到 L5 的塔一致（L5 无候选） |

**非真空**：8,076 行覆盖 4 个级别、两种 kind、两种 div 状态，并与门行三项计数逐位互证。
`rev>1` 的存在（2M 窗）证明修订号轴在真实数据上可读出同 key 的状态迁移，不是恒常量字段。

---

## 六、计数三元组 + 快照 commit

| 面 | 快照 | passed / failed / ignored | 说明 |
|---|---|---|---|
| `cargo test --lib` 本票 | `dc0d61052e`（干净工作树） | **2058 / 1 / 135** | 2195 条 `test ` 行 |
| `cargo test --lib` 基座 | `e8496fcaf0` | **2058 / 1 / 135**（恒等，构造性） | 本票 `git diff e8496fcaf0 --stat` = 单文件 `rust/src/bin/p123_fast_replay.rs`，`rust/src/theta_v0/**` 零改动 ⟹ lib 测试集与结果逐位不变。基座数 = #552 快照 `49e1e4b284` 的 2053 + 其后收口三批新增 5 枚 `#[test]`（`git diff 49e1e4b284 e8496fcaf0` 计数）= 2058，与实测一致 |
| `cargo test --bin p123_fast_replay` 本票 | `dc0d61052e` | **4 / 0 / 0** | 新增 2（`event_dump_disabled_writes_nothing`、`event_dump_line_format_and_revision_monotonic`）+ 既有 2 |

**唯一 failed = 既有红**：`theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`——
与 #551/#552 报告登记的是同一枚，基座同样失败，非本票引入（本票 lib 零改动）。照实登记不掩盖。

---

## 七、#535 四类禁项逐条核对

| 禁项 | 本票是否触及 | 依据 |
|---|---|---|
| ① 终态几何配 prefix 首见钟 | **不触** | 本路只落盘观察当刻的 `as_of`/`judge_at`/`turn_source` 三个既有字段，不构造任何钟、不把终态几何与首见钟拼接 |
| ② 以未来完成的比较对象回填 `first_provable_at` | **不触** | 无写路径（`&NestCandidateEvent` 不可变借用），不回填任何字段 |
| ③ 丢失 `Invalidated` 路径 | **不触** | 落盘点在既有 pending 短路**之前** ⟹ 已出清（`pending=0`）的观察照样落行，不筛不丢 |
| ④ 把回试段吞入父背驰段 | **不触** | 三区间 `seg_a`/`interval_b`/`interval_a` 原值落盘，无任何区间改写/合并路径 |

---

## 八、090 照实登记

1. **全量 4,613,599 bar 的 prefix pass 未走完**。terminal pass 三路全窗走完、prefix 推进到 1M 里程碑
   （三路计数逐位一致）后按票体「最大可行窗口」条款显式中止，理由 = 段耗时超线性外推 >16h/单跑。
   完整时刻、bar 数与读数见 §5.1。**中止不是判负**：全程无一处 diff≠0。封印面由 2M / 800k 两窗承担。
   ⟹ 本票的「全量双跑封印」在**窗口维度上是缩小的**，不声明覆盖 4.6M 全窗，不作膨胀。
2. **全量层面未引入改动前 binary 作第三方对照**（成本）。改动前 binary 的对照只在 200k / 800k
   两窗做（§四.3，两窗既有 dump SHA 与本票 binary 逐字节相同）。
3. **`rev` 的非平凡性只在 2M 窗见到**：200k（719 行）/ 800k（3,213 行）两窗 rev 恒 1；
   2M 窗 8,076 行里只有 **1 个 key** 观察到 4 次。⟹ 修订号轴可用但**极稀疏**，不声明它在小窗
   有信息量。`pending` 三窗恒 1（无同 trigger 内前序出清的实例），该轴目前**未被真实数据触发**，
   照实登记为「结构上可达、实测未见」。
4. **本票不关 GitHub issue #553**：执行指令明令停在干净工作树、不动 issue 状态。核销留给收口批。
5. **run3（全量关灯侧）于 `23:27` 提前中止**以降并发。它的独有价值（全量尺度开灯/关灯对照）
   由 §四.3 的 200k / 800k 三态 SHA 相等 + §5.1 的 1M 里程碑计数三路逐位一致替代承担。
6. **既有红一枚**：`extract_signals_bit_exact_digest_guard`，基座同样失败，非本票引入（§六）。
7. **墙钟全部是同机并发口径**（2M 双跑 2 路 + 800k 1 路穿插；全量段为 3 路），带宽互染，
   **不作性能结论**——只作规模与可行性的方向读数。terminal pass 的 500k 里程碑在单跑 63.0s /
   三路并发 62.4–62.5s，是「并发未显著拖慢」的一个旁证，但不外推到 prefix 段。
8. **p123 prefix 的超线性是既有性质，本票未改也未优化**：本票只加一路只写不判的 dump，
   判据/缓存/trigger 语义零改动。超线性的归因与治理不在 #553 范围（#69 同族报告已登记
   「杀超线性在 4.6M 尺度未显效」）。本票不借"顺手优化"之名动判据（090）。

---

## 九、结果包六要素

1. **结论** — §结论 + §一 六子句表。
2. **定义依据** — 候选事件身份键 = `EventKey`（`:217-240`），与 p123 账本/pending 出清同一键；
   字段口径逐条对齐 `NestCandidateEvent`（`level_view.rs:653-677`）的既有定义，本票不新定义任何概念。
   dump 侧信道的"只写不判"纪律锚 = p123 模块头既有 `P116_DUMP`/`P421_LIFECYCLE_DUMP` 同款声明。
3. **边界条件（结论何时翻转）** —
   (a) 若某次跑出现既有行 diff≠0 ⟹ "既有封印未破"翻转，按 #69 判负即停线归档现场（禁调判据）；
   (b) 若 `EventDump` 的 `revisions`/`seq` 出现第二个读点 ⟹ "零判定消费"翻转；
   (c) 若把落盘点移到既有 pending 短路**之后** ⟹ `pending=0` 行消失，§三.3 覆盖面声明翻转；
   (d) 若换数据/换窗口后 rev 出现 >1 ⟹ §三.3 的"本数据上恒 1"翻转（字段语义不变）。
4. **下游推论** — #557（本票 blocking）可直接消费 `P123_EVENT_DUMP` 的逐条事件流做候选级审计，
   无需再改 p123 判定路径；既有 `P116_DUMP` 的全部历史对拍基准（p92/p116 跨码对照）继续有效。
5. **谱系引用** — 090（严格性 / 声明膨胀禁止）：§三.3 覆盖面边界、§八 登记均照实不膨胀；
   #69 全量双跑协议（`chanlun/review-results/wave1-5a5b-full-double-run-seal-20260727.md` 为同族先例）；
   #535 四类禁项（§七）。本票不涉及概念分离，无新谱系条目。
6. **影响声明** — 改动面 = `rust/src/bin/p123_fast_replay.rs` 单文件（+196/−1）：新增 `EventDump`
   结构与 2 枚单测，既有控制流唯一改写为 `pending.contains` 提出局部变量。lib 零改动、对照族零改动、
   数据文件只读。关灯（默认）时行为与基座逐位相同。
