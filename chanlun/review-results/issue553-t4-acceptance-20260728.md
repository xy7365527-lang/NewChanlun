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
**全量窗口的双跑子句（六子句 #2 / 验收 #5）因此不声称 PASS，而是按编排者执行令（dispatch 指令）
宽免**——票面（#553 / #547）并无「跑不动就取最大可行窗口」这类条款，宽免来源必须归给执行令。
中止时刻已落盘的**已跑段**另有一层字节面封印（§5.1.1：开灯双跑三面 diff=0、开灯 ≡ 关灯严格字节前缀），
但**覆盖段 ≠ 全窗**，不构成全窗完成声称。

---

## 一、关票门六子句

| # | 子句 | 状态 | 证据落点 |
|---|---|---|---|
| 1 | dump 口径文档 | **PASS** | §三 字段表 + `p123_fast_replay.rs:374-408` 模块内文档（收口批插入 13 行后的实测行号） |
| 2 | 双跑 diff=0 | **按执行令宽免**（不声称 PASS，见登记 1） | §5.2（2M 窗）与 §5.3（800k 窗）：既有行 / 事件行 / stdout 三面各自 diff=0；§5.1.1 全量已跑段三面 diff=0（覆盖段口径，非全窗）。宽免来源 = 编排者执行令（dispatch 指令），票面无此条款 |
| 3 | 双侧 SHA-256 | **PASS** | §5.2 / §5.3 SHA 表（两窗各三面双侧逐位相等） |
| 4 | 窗口 / bar 数 / 耗时 | **PASS** | §5.1 全量（terminal 全窗 4,613,599 bar 走完、prefix 至 1M 中止）+ §5.2（2,000,001 bar，5h43m）+ §5.3（800,001 bar，24m×2） |
| 5 | 既有封印未破证据（既有行双跑亦 diff=0） | **PASS** | §5.2 / §5.3 两窗既有行双跑 diff=0 + SHA 相等；§四.3 200k / 800k 三态对照（改动前 binary 参与，SHA 逐字节相同） |
| 6 | 计数三元组 + 快照 commit + 090 登记 | **PASS** | §六（三元组 + 快照）、§八（090 照实登记） |

## 二、验收逐条（票体）

| # | 验收项 | 状态 | 证据 |
|---|---|---|---|
| 1 | 事件 dump env-gated（仿既有开关模式） | **PASS** | `EventDump::from_env`（`:426-435`）读 `P123_EVENT_DUMP`；未设 ⟹ `writer=None` |
| 2 | 只写不判（零判定消费、零生产路径读取） | **PASS** | §四.1 构造性论证 + 全仓 grep 读点清单（生产路径仅 3 处：构造 `:1147`、观察 `:1428`、收尾 `:1595`） |
| 3 | dump 内容 = 候选事件流逐条（key/级别/状态/钟/区间/修订号） | **PASS** | §三 字段表（16 字段，全部落一行） |
| 4 | 既有 dump 行逐字节不动、既有封印不破 | **PASS** | §四.2 构造性 + §四.3（200k/800k 含改动前 binary）/ §5.2、§5.3（两窗双跑 SHA 相等）实测 |
| 5 | 全量双跑 diff=0 + 双侧 SHA-256 | **按执行令宽免**（不声称 PASS） | §五：2M / 800k 两窗封印成立；§5.1.1 全量已跑段三面 diff=0（覆盖段口径，非全窗）；全量 prefix 中止照实登记（§5.1、§八 登记 1）。宽免来源 = 编排者执行令（dispatch 指令），#553 / #547 票面无此条款 |
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
  `static DUMP: OnceLock<Option<Mutex<BufWriter<File>>>>`（`:196`）分属两个对象两个文件**。
  同理与 `P421_LIFECYCLE_DUMP` 也互不相干。
- **落盘点**：prefix replay 的事件应用循环（`:1422-1431`），每观察到一条候选事件写一行。
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
| **key** | `level` `side` `kind` `seg_a` `interval_b` `interval_a` `turn_source` | 与 `EventKey`（`:222-245`）字段一一对应——即 p123 账本（candidates/divergences/term_seen）与 pending 出清所用的同一身份键。`kind` 标签复用既有 `nest_kind_tag`（trend/pan），与 DIV 行同词表 |
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
| `:1147` | `let mut event_dump = EventDump::from_env()?;`（构造） |
| `:1428` | `event_dump.observe(&event, index, pending_hit)?;`（写） |
| `:1595` | `event_dump.flush()?;`（收尾刷盘） |

`revisions` / `seq` 的读写全部在 `observe` 体内（`:447` `revisions.entry` / `:453` `self.seq` 读 /
`:474` `self.seq += 1`），**无第二读点**。
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
- `diff(B.stdout, C.stdout)` = **0**（实测 **17 行**门行；比 2M / 800k 两窗的 19 行少的正是
  `P123_BIT_DIFF_LEVEL` 的 **L4 / L5 两行**——200k 窗的级别塔更矮、无 L4/L5 层输出，非缺行漏打）
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

- 本票 `git diff e8496fcaf0 --stat` = `rust/src/bin/p123_fast_replay.rs | 196 +++/1 −`，**单文件**
  （dump 侧信道 commit `dc0d61052e` 相对基座的数字）。
- **收口批的追加改动（三枚 commit）**：在**同一 bin 单文件**上共叠 **+73/−4**——

  | commit | 内容 | 面 |
  |---|---|---|
  | `439105c7e7` | `EVENT_DUMP_ENV` 常量收敛（4 处 `"P123_EVENT_DUMP"` 字面量归一到单一来源）+ `EventDump` doc 补 C2「写失败传播边界」声明 | bin 单文件 **+17/−4** |
  | `555ee412f2` | 双轴评审产物入库（`chanlun/review-results/code-review-issue553-{spec,standards}-20260728.md`） | **非代码** |
  | `756dc87763` | 错误分支两枚单测（只动 `#[cfg(test)] mod tests`，§四.5） | bin 单文件 **+56/−0** |

  文件行数 2,646 → **2,715**（`439105c7e7` 的插入点在原 `:368` 之后 ⟹ 本报告该点之后的行号引用
  一律 +13，已全文订正；`756dc87763` 插在测试区末尾 ⟹ 不再位移任何既有引用）。
  三枚合计**判据 / 控制流 / 生产代码语义零改动，lib（`rust/src/theta_v0/**`）零改动** ⟹
  §四.4 的「对照族与生产路径零触碰」结论不变，§5.1.1 / §5.2 / §5.3 的封印产物亦不需重跑
  （产物由 `dc0d61052e` 的 binary 产生，见 §5.0）。
- 判据代码（dirty (i)–(iv)、trigger 语义、TERM 反查）零改动；lib（`rust/src/theta_v0/**`）零改动 ⟹
  对照族与生产路径零触碰。
- 数据文件 `/tmp/kimi-nest-mainline/analysis/data_cache/btc_1m_full.json`（→ `analysis/data_cache/btc_1m_full.json`）
  只读打开；SHA-256 `16ea13d55f2ae7edcfc503f604a14961fd1a378227afd37c63f23386e894707b`，
  mtime `2026-06-25 10:27` 未变。

### 4.5 TDD 红绿

- **红**：先落两测（`event_dump_disabled_writes_nothing` `:2591` / `event_dump_line_format_and_revision_monotonic` `:2602`）
  与测试夹具（`sample_event` `:2572`、`SharedSink` `:2635`），`EventDump` 尚不存在 ⟹
  `cargo test --bin p123_fast_replay` = `error[E0433]: cannot find type EventDump in this scope`（2 处）。
- **绿**：实装 `EventDump` 后 `cargo test --bin p123_fast_replay` = **4 passed / 0 failed**
  （新 2 + 既有 2）。**口径 = `dc0d61052e` 时刻**；收口批追加两测后为 **6 passed**（见下与 §六）。
- 两测分管票体的两个要求：**门控**（关灯零落盘零副作用）与**行的产生**（16 字段逐字断言 +
  rev 同 key 递增 / 异 key 归 1、seq 全局单调）。

**收口批追加：错误分支两测（`756dc87763`）**——锁定本批新加的 C2 写失败传播边界声明。
补测的理由不是覆盖率数字，而是：**声明若无可执行验证即是声明与验证脱节**（090），
边界声明落地（`439105c7e7`）与其验证必须同批闭合。

| 测试 | 位置 | 锁定的行为 |
|---|---|---|
| `event_dump_observe_propagates_write_error` | `:2678` | `write` 返回 `BrokenPipe` 的 sink ⟹ `observe` 必须返回 `Err`，消息含「写 P123_EVENT_DUMP 失败」。**同时锁定 `EVENT_DUMP_ENV` 常量确实被插值渲染成原字面量**，不是占位符（`439105c7e7` 的常量收敛不改对外消息） |
| `event_dump_flush_propagates_flush_error` | `:2692` | `flush` 失败 ⟹ `EventDump::flush` 必须返回 `Err`，消息含「刷新 P123_EVENT_DUMP 失败」 |

- **夹具 `FailingSink`（`:2652-2676`）**：纯内存，**不碰文件系统、不碰进程 env**；
  **刻意不包 `BufWriter`**——包了则写失败被缓冲吞掉、这条分支根本测不到。
- **TDD 负控实测（非恒真断言的证据）**：把两枚 sink 的失败开关翻成 `false` 后，两测如期**转红**
  （panic 于 `:2686` / `:2700`）；翻回 `true` 转**绿**。⟹ 断言确实在验证行为，不是恒真绿。
- 生产区行号未受影响（新测插在 `SharedSink` `:2635` **之后**）：常量 `:372`、`from_env` `:426`、
  `observe` 调用点 `:1428`、`flush` `:1595`、doc `:374-408` 全部不变；文件总行数 2,659 → **2,715**。

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
本票遂按**编排者执行令（dispatch 指令）**给出的宽免——「跑不动就取最大可行窗口并照实登记」——
在 `2026-07-29 00:08` 中止三路（run3 提前于 `23:27` 中止以降并发），封印面改由 §5.2 的
**2,000,001 bar** 与 §5.3 的 **800,001 bar** 两个窗口承担。
**宽免来源须点明**：该句出自执行令，**#553 / #547 的票面并无此条款**——本报告早前版本把它标为
「票体条款」是**错标**，此处订正。相应地，六子句 #2 与验收 #5 降为「按执行令宽免」，不声称 PASS。
中止是**显式登记的**，不是静默截断：本节给出中止时刻、已完成 bar 数与三路里程碑读数。

**不作判负**：中止的原因是墙钟成本，不是任何 diff≠0。三路自始至终无一处 diff 现场，
#69 判负即停线未触发，判据代码零改动（§四.4）。

### 5.1.1 全量已跑段的双跑封印（覆盖段口径）

**口径先行（090）**：本节的对拍面 = 三路各自**中止时刻已落盘的前缀**，**不是全窗**。全量 prefix pass
未走完这一事实不因本节改变（§八 登记 1 仍然成立），本节**不作任何全窗完成声称**。原报告只登记了
「全量 prefix 中止」而丢掉了已跑段产物的对拍价值——此处补取为正式封印计算。

**① 开灯双跑（run1 vs run2，同配置 `P116_DUMP` + `P123_EVENT_DUMP`）—— 三面全等：**

| 面 | 产物 | SHA-256（双侧逐位相同） | 规模 |
|---|---|---|---|
| 既有 dump | `/tmp/wt553_full_run{1,2}_dump.txt` | `cee9e68654556b5124d579406ae2cde8ff8daea36672ab350e898ebf3f2de61d` | 139,219 B / 3,263 完整行 + 1 半行 |
| 事件 dump | `/tmp/wt553_full_run{1,2}_events.txt` | `25e67dc655aa7280b571bb295518cbc1db660986cbaf049d7d93ef54d4cc3a9f` | 1,239,091 B / 4,882 行（完整行结尾） |
| stdout | `/tmp/wt553_full_run{1,2}.out` | `11ebc3ec461dcd957d023b5dbc8404ca34eba1d11579a64c7179f1ac91bceb0a` | 691 B / 4 行 |

`cmp` 三面全 **diff=0**。stdout 的 SHA `11ebc3ec…` 已由编排者亲验双侧一致，作为证据链上的独立第二读。

**② 开灯 ≡ 关灯前缀（run3 vs run1）：**

- run3（关灯，仅 `P116_DUMP`）既有 dump `/tmp/wt553_full_run3_dump.txt`：114,647 B / 2,702 完整行 + 1 半行，
  SHA-256 `672cca7a242414bdaa4a04eb25e9b04f8f8083f535335a85afd06d82e9c6ea7d`。
- 实测 `head -c 114647 run1_dump | cmp - run3_dump` ⟹ **run3 是 run1 的严格字节前缀**
  （run3 于 `23:27` 提前中止以降并发，故短 24,572 B）。
- run3 **无事件 dump 文件**（关灯 ⟹ `writer=None` ⟹ 不 `File::create`），与 §3.1 的门控声明一致。

**③ 诚实边界（照实，一条不省）：**

1. **末行是半行**：run1 / run3 的既有 dump 末尾是中止时刻的**未完写入**——末字节非换行
   （run1 末字节 `0x6e`=`n`，run3 末字节 `0x3d`=`=`），即末行为**半行截断**。逐字节比较**含这半行**
   仍成立；正因如此，「双跑在中止时刻的落盘量与内容逐字节一致」这条更强（两跑落盘量同为 139,219 B）。
2. **stdout 只覆盖头部四行门行**：run1 / run2 的 stdout 各只有 **4 行**（`P123_INPUT` / `P123_RULE` /
   `P123_PROBE` / `P123_SPARSE_MODE`）——其余门行在 prefix pass 收尾才打印，中止 ⟹ 未打。
   故本面封印覆盖的是**头部四行门行**，**不含** YIELD / CERT / D3 / BASELINE / PROVIDER / SNAPSHOT /
   R7 / MISSED 等收尾读数。
3. **行类不是全套**：已跑段既有 dump 的行类计数（run1，含末半行）= DIV **2,394** / TERM **494** /
   TURN **376**，合计 **3,264**。**无 CERT / FALLBACK / TURN_CLASS 行**——这三类在收尾装配阶段才落盘，
   中止 ⟹ 未落。读者不得把本段读成 §5.2 / §5.3 那样的全套行类对拍。
4. **rev 全 1**：事件 dump 4,882 行的 `rev` **全部 = 1**（无同 key 重复观察），与 §5.4 / §八 登记 3
   的「`rev>1` 只在 2M 窗见到」一致，可交叉引用。
5. **`P123_INPUT` 行不是完成证据**：run1 的该行为 `replay_bars=4613599`（全窗**声明**值，开跑即打印），
   而实际未跑完。此行**不构成**全窗完成证据，不得据此反推 §5.1 的中止登记被推翻。

**④ 结论口径**：在全量尺度的**已跑段**上，开灯双跑三面 diff=0、开灯 ≡ 关灯严格字节前缀同时成立
⟹ 全量路径上**未出现任何 diff≠0 的现场**。这与 §5.1 的「1M 里程碑三路计数逐位一致」互补——
一个是**计数面**，一个是**字节面**。但**覆盖段 ≠ 全窗**：本节不作全窗封印声称，全量 prefix 未走完
的事实与登记（§八 登记 1）不受本节影响。

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
| stdout 门行 | `4ecc346b784b596f2655ac6f9a792180112d1ee3ac17e9a5ccfe709fc84b6c03` | 19 行 |

`diff` 三面全 **0**（`DIFF0_2M_DUMP` / `DIFF0_2M_EVENTS` / `DIFF0_2M_STDOUT`）。
产物：`/tmp/wt553_2m_{a,b}_{dump,events}.txt`、`/tmp/wt553_2m_{a,b}.{out,err}`，
封印计算落 `/tmp/wt553_seal2m.txt`；800k 侧产物 `/tmp/wt553_800k_{new,on2,newoff}_*`，
全量侧产物 `/tmp/wt553_full_run{1,2,3}.{out,err}`、`/tmp/wt553_full_run{1,2}_{dump,events}.txt`、
`/tmp/wt553_full_run3_dump.txt`（dump 末行为中止时的未完写入；已按**覆盖段**口径入 §5.1.1 对拍，
不作全窗对拍面）。

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
| stdout 门行 | `diff` = 0 | 19 行 |

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
| `cargo test --bin p123_fast_replay` 本票 + 收口批 | `756dc87763` | **6 / 0 / 0** | 新增 2（`event_dump_disabled_writes_nothing`、`event_dump_line_format_and_revision_monotonic`）+ 收口批错误分支 2（`event_dump_observe_propagates_write_error`、`event_dump_flush_propagates_flush_error`，§四.5）+ 既有 2 |

**唯一 failed = 既有红**：`theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`——
与 #551/#552 报告登记的是同一枚，基座同样失败，非本票引入（本票 lib 零改动）。照实登记不掩盖。

**收口批后复测**：收口批三枚 commit 的改动面 = bin 单文件（合计 +73/−4，§四.4）+ 评审产物与本报告
文档，**对 `rust/src/theta_v0/**` 零改动** ⟹ `cargo test --lib` 复跑仍为 **2058 / 1 / 135**（恒等，
上表 lib 两行数字不变），唯一 failed 仍是上述既有红；bin 侧因新增两枚错误分支单测由 4 → **6 passed**
（已反映在上表第三行）；`rustfmt --edition 2021 --check src/bin/p123_fast_replay.rs` 仍干净（exit 0）。

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
   （三路计数逐位一致）后按**编排者执行令（dispatch 指令）**的「最大可行窗口」宽免显式中止
   （**#553 / #547 票面无此条款**——早前版本标为「票体条款」是错标，已在 §5.1 订正），
   理由 = 段耗时超线性外推 >16h/单跑。
   完整时刻、bar 数与读数见 §5.1。**中止不是判负**：全程无一处 diff≠0。封印面由 2M / 800k 两窗承担，
   已跑段另有覆盖段口径的字节面封印（§5.1.1，**非全窗**）。
   ⟹ 本票的「全量双跑封印」在**窗口维度上是缩小的**，不声明覆盖 4.6M 全窗，不作膨胀；
   六子句 #2 与验收 #5 相应降为「**按执行令宽免**」，**不声称 PASS**。
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
9. **p123 文件体量与增量方向归所属线**：`rust/src/bin/p123_fast_replay.rs` **2,646 行**
   （**口径 = 双轴评审快照 `2a75254b06` 时的行数**，非当前值；收口批三枚 commit 后为 **2,715 行**，
   净增 69 行中 **56 行是 `#[cfg(test)]` 测试码**，见 §四.4 / §四.5；**微修批（#557 M1）doc-only
   再 +6 行 ⟹ 2,721 行**，无生产逻辑改动），
   其中 `run_targeted_prefix_pass` 约 **480 行**。体量与拆分方向属**并行线**的面——动 p123 会与
   并行线撞面，**不属本票（#553）范围**。故本票不动，登记在案，由所属线承接（结论不因行数更新而变）。
10. **三份 env→writer 同形未收编**：`P116`（`P116_DUMP`）、`P421`（`P421_LIFECYCLE_DUMP`）、
    `P553`（`P123_EVENT_DUMP`）三条侧信道是同一形状（env 门控 → 可选 writer → 只写不判）。
    收编成公共抽象会**触并行线文件**，本票不做，登记在案。
11. **`EventDump.revisions` 的 `BTreeMap` 无界累加**：每个新 `EventKey` 占一个条目、只增不删，
    长跑内存随唯一 key 数线性增长。这是**「只写不判」语境下的已知代价**——该结构不进判定路径、
    关灯时零构造（§4.1 / §4.2.3）。已在案，本票不治理，登记。
12. **`BufWriter` 的 `Drop` 冲刷吞 io error**：`EventDump` 的 writer 在 drop 时若仍有缓冲未刷且
    刷失败，错误被静默吞掉（`BufWriter::drop` 的标准语义）。与 #421 的 `P421_LIFECYCLE_DUMP`
    **既有形状同形**，非本票引入，本票不治理，登记在案。
    **与代码的互指（收口批已落，行号实测）**：`EventDump` 的 doc comment 现含一节
    「**边界（写失败的传播口径，照实声明）**」（`:386-400`，**微修批订正行号**，原 `:386-394`），
    声明 `observe` 的写失败经调用点 `?`
    上抛 ⟹ **中止 prefix pass**；此时 `run_targeted_prefix_pass` 尾部的 `P421_LIFECYCLE_DUMP`
    收尾 flush（`:1596-1600`，先执行）与 `EventDump::flush`（`:1601`，后执行）**两条收尾语句均不可达**
    ——正是本条登记的 drop 冲刷路径成为唯一出口的场景。该口径与 #421 的 `write_lifecycle_line`
    （同样以 `?` 上抛）**同形**，与既有 `dump_line`（`:198`，写失败被 `let _ = …` 吸收、不上抛、
    不中止重放）**不同款**。关灯路径不受影响：`writer=None` ⟹ `observe` 首行返回 `Ok(())`，
    不存在写失败面 ⟹ §4.2.3「关灯零行为差异」不因本边界而弱化。
    **枚举补全（微修批，影子评审 #557 M1）**：上抛**不止于 `run_targeted_prefix_pass` 尾部两条**。
    该 `Err` 继续经 `main` 内 `run_targeted_prefix_pass` 调用点的 `?`（`:744`）逃出 `main`，
    使 `main` 尾部的 `dump_flush()`（`:1057`）**同样不可达**；而 `DUMP` 是
    `static OnceLock<Option<Mutex<BufWriter<File>>>>`（`:196`），Rust 的 `static` **不执行 `Drop`**
    ⟹ 开灯写失败时，既有 `P116_DUMP` 的**缓冲尾字节静默丢失**——**受损面是既有封印文件**，
    恰是票体首要保证（「既有 dump 行逐字节不动」）所在的面。`EventDump` 自己的 `BufWriter` 是
    局部变量、drop 时会冲刷，反而不受此损。该形状与 #421 的 `P421_LIFECYCLE_DUMP` **同款**
    （同样以 `?` 上抛、同样使 `dump_flush()` 不可达），属**既有形状，非本票新引入**，
    本票不治理、登记在案；但 C2 修的正是这条传播口径声明，**枚举缺一项 = 声明未穷尽**（090），
    故 doc 与本条一并补全。关灯路径不受影响（同上）。
    **两条低阶残项已登记在案（090 照实）**：(i) 上述「上抛后收尾语句不可达」是**跨函数**声明，
    两枚新单测只锁 `observe`/`flush` 各自返回 `Err`，该跨函数可达性**无 harness 可及、未被覆盖**
    ——去向已记于登记 13；(ii) `pending` 轴三窗实测恒 1、`pending=0` 行只在单测出现
    ——已记于登记 3「结构上可达、实测未见」。两条均不构成阻断，不改本报告任何 PASS 判定。
13. **双轴评审 standards 轴 S5（新增生产码错误分支零单测覆盖）在本收口批「部分处置」**，照实登记：
    - **已处置**：`observe` 写失败、`flush` 失败两条分支已由 `756dc87763` 的两枚单测锁定（§四.5）。
      选这两条的理由**不是覆盖率数字**，而是本批新加的 C2 边界声明（登记 12）必须有可执行验证——
      否则是**声明与验证脱节**（090 声明膨胀的一种）。
    - **不做项（理由已核实，非成本借口）**：`from_env()` 的 `File::create` 失败分支**不补测试**。
      触发它必须写**进程全局 env**，而 `cargo test` 默认多线程并行执行、`environ` 是进程唯一的
      共享可变状态——**Rust 2024 edition 已将 `env::set_var` 标为 `unsafe fn`**，正是这个理由。
      选一个仓库内唯一的 var 名也**消不掉**这层结构性竞争：竞争发生在 `environ` 数组的重分配，
      与 key 是否同名无关。本仓虽已有 env 操纵测试的先例
      （`theta_v0/backtest/runner.rs:1565-1569`、`recursive_t/rec_driver.rs:478-492`、
      `theta_v0/backtest/wverify_run/tests.rs:482-484`），但**先例存在不等于该模式严格**——
      不以先例为由复制不严格的做法。
    - **严格补法（记明去向，不留 TODO）**：要严格覆盖该分支，须先**重构 `from_env`**——把 env 读取
      与路径构造拆成两个函数，暴露一个不碰 env 的**路径注入入口**，再对注入入口测 `File::create` 失败。
      这是**生产码改动**，与登记 9（p123 体量 / 拆分方向归所属线）同属**并行线**的面，
      不在 #553 收口批范围，由所属线承接。
    - 顺带记明：端到端的「既有封印零扰动」已由 §5.2 / §5.3 两窗实测 + §四.3 改动前 binary 三态对照
      承担，S5 在评审里亦注明此项**不重复计**。

---

## 九、结果包六要素

1. **结论** — §结论 + §一 六子句表。六子句 #1/#3/#4/#5/#6 为 PASS；**#2（双跑 diff=0）与验收 #5
   在全量窗口上不声称 PASS，按编排者执行令宽免**（票面无「最大可行窗口」条款），
   封印实证面 = 2M / 800k 两窗 + 全量已跑段（§5.1.1，覆盖段口径）。
2. **定义依据** — 候选事件身份键 = `EventKey`（`:222-245`），与 p123 账本/pending 出清同一键；
   字段口径逐条对齐 `NestCandidateEvent`（`level_view.rs:653-677`）的既有定义，本票不新定义任何概念。
   dump 侧信道的"只写不判"纪律锚 = p123 模块头既有 `P116_DUMP`/`P421_LIFECYCLE_DUMP` 同款声明。
3. **边界条件（结论何时翻转）** —
   (a) 若某次跑出现既有行 diff≠0 ⟹ "既有封印未破"翻转，按 #69 判负即停线归档现场（禁调判据）；
   (b) 若 `EventDump` 的 `revisions`/`seq` 出现第二个读点 ⟹ "零判定消费"翻转；
   (c) 若把落盘点移到既有 pending 短路**之后** ⟹ `pending=0` 行消失，§三.3 覆盖面声明翻转；
   (d) 若换数据/换窗口后 rev 出现 >1 ⟹ §三.3 的"本数据上恒 1"翻转（字段语义不变）；
   (e) 若 §5.1.1 的**已跑段/覆盖段**口径被读作**全窗**口径 ⟹ 本报告的封印声明整体翻转——
   §5.1.1 覆盖的是三路中止时刻的落盘前缀（末行为半行截断、无 CERT/FALLBACK/TURN_CLASS 行、
   stdout 仅头部四行门行），全量 prefix 未走完的登记（§八 登记 1）始终成立；
   (f) 若日后取得票面依据证明「最大可行窗口」确为 #553 / #547 票体条款 ⟹ §5.1 与 §八 登记 1 的
   「宽免来源 = 执行令」订正翻转（但六子句 #2 / 验收 #5 的窗口缩小事实不因此变为全窗 PASS）。
4. **下游推论** — #557（本票 blocking）可直接消费 `P123_EVENT_DUMP` 的逐条事件流做候选级审计，
   无需再改 p123 判定路径；既有 `P116_DUMP` 的全部历史对拍基准（p92/p116 跨码对照）继续有效。
5. **谱系引用** — 090（严格性 / 声明膨胀禁止）：§三.3 覆盖面边界、§八 登记均照实不膨胀；
   #69 全量双跑协议（`chanlun/review-results/wave1-5a5b-full-double-run-seal-20260727.md` 为同族先例）；
   #535 四类禁项（§七）。本票不涉及概念分离，无新谱系条目。
6. **影响声明** — 覆盖本票四枚 commit（`dc0d61052e` + 收口批三枚）：
   - `dc0d61052e`（dump 侧信道）：`rust/src/bin/p123_fast_replay.rs` 单文件 **+196/−1**——新增 `EventDump`
     结构与 2 枚单测，既有控制流唯一改写为 `pending.contains` 提出局部变量。
   - `439105c7e7`（收口批）：同一 bin 单文件 **+17/−4**——`EVENT_DUMP_ENV` 常量收敛 + `EventDump` doc
     补 C2 写失败传播边界声明（**声明面**改动，无行为改动）。
   - `555ee412f2`（收口批）：`chanlun/review-results/` 下两份双轴评审产物入库，**非代码**。
   - `756dc87763`（收口批）：同一 bin 单文件 **+56/−0**，**只动 `#[cfg(test)] mod tests`**——两枚错误
     分支单测 + `FailingSink` 夹具。
   合计：改动面仍是 **bin 单文件 + 文档**，bin 行数 2,646 → 2,715。**lib（`rust/src/theta_v0/**`）零改动、
   判据与对照族零改动、数据文件只读**；关灯（默认）时行为与基座逐位相同。封印产物由 `dc0d61052e`
   的 binary 产生，收口批不使其失效（§四.4）。
