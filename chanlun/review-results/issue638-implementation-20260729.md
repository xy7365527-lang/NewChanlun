# #638 实施报告——交易层消费三类点成立证据包（#575 消费方接线 2/3）

> 日期：2026-07-29；性质：实施车收口报告（opus 实施车，高难档）
> 工位：`/private/tmp/wt-638`；分支 `ticket-638`（基 = main HEAD `1b7ba1a1a5`，开工 `git log --oneline -1` 实测）
> 全程前台单线程（无子代理、无后台进程，符合票面硬约束）；**未 commit**——全部改动留存 worktree
> 未提交面，由编排方统一提交。

## 0. 一句话

买卖点账本的成立档 `ThirdPointPack` 经**新独立模块** `trading/third_point_book.rs`
（`ThirdPointBook` = 三类点成立登记账）接入交易层：`register`（签名 bound 即备战档禁区闸门）+
`sync_from_ledger`（读 `RetraceLedger::established`）+ `register_from_ledger`（读
`established_pack`）三个消费入口，**照实全字段登记 + 幂等 + 同身份改口 fail-loud + 只读观测面**，
零判据零信号产出；迟到处置按裁定一 = #587 字面（登记观测、不作独立信号消费），量化判据留白归
**#680**；账本侧代码零改动（只改 `mod.rs` 登记表 doc 文字，row 2 翻「已接线（#638）」、计数
2/3 → **3/3（入口计数口径）**）。

## 1. 改动文件清单

| 文件 | 改动性质 | 规模 |
|---|---|---|
| `rust/src/trading/third_point_book.rs` | **新增**（未跟踪）：登记账类型 + 3 个消费入口 + 5 个只读观测方法 + 8 条测试 | 528 行（生产段 1-262 行，测试段自 264 行起 ~264 行） |
| `rust/src/trading/mod.rs` | 模块注册 `pub mod third_point_book;` + 模块头「图外挂件（零入边）」声明 | +6 |
| `rust/src/theta_v0/classifier/retrace_ledger/mod.rs` | **仅文档**：消费方登记表 row 2 改写 + 计数行改写（票面禁区：`retrace_ledger/` 内只许改登记表 doc 文字） | +7/−5 |

```
$ git status --porcelain
 M rust/src/theta_v0/classifier/retrace_ledger/mod.rs
 M rust/src/trading/mod.rs
?? rust/src/trading/third_point_book.rs

$ git diff --stat
 rust/src/theta_v0/classifier/retrace_ledger/mod.rs | 12 +++++++-----
 rust/src/trading/mod.rs                            |  6 ++++++
 2 files changed, 13 insertions(+), 5 deletions(-)
```

**账本侧代码零改动核验**（票面「不动买卖点账本侧」）：

```
$ git diff --stat -- rust/src/theta_v0/classifier/retrace_ledger/{adapter,book,log,portal,audit}.rs \
                     rust/src/theta_v0/classifier/retrace_ledger/tests/
（零行输出 = 全部确认零改动）
```

`formal/` 全程未触碰 ⟹ fixture 漂移 gate 不适用本轮（CLAUDE.md 节拍条件「凡改动 `formal/` 的
session」未触发）。

## 2. 信号通道权威核查结论 + 接入位置理由（票面范围第 1 条点名要）

### 2.1 开工亲核的现状（本轮读码实测，非转引）

| 事实 | 锚 |
|---|---|
| `trading` 是**私有模块** | `rust/src/lib.rs:49` `mod trading;` |
| `run_organic` 逐字移植 Python、**V0≡P5 逐位等价承重面** | `rust/src/trading/runner.rs:1-6` 头注 |
| 交易层 BSP 事件源 = legacy Python-parity 磁带，**非** theta_v0 塔链 | `rust/src/trading/tape.rs`（`SignalTape`） |
| 入场判据读**布尔行**，不经 `BspEvent` | `runner.rs:807`（`sig.buy1.get(k)`）/ `runner.rs:824`（`sig.buy_any.0 & sub_mask`） |
| 接线前 `trading → theta_v0` 唯一生产接触面 | `rust/src/trading/center_book.rs:35`（#637 引入） |
| `ThirdPointPack` 接线前**零生产消费方** | `grep -rn "ThirdPointPack\|established_pack" src/ tests/` 在 `retrace_ledger/` 之外零命中（仅 `level_view/tests/feature_gating.rs` 一处同名测试函数，非类型引用） |
| pack 形状（两拍口径）与产出面 | `retrace_ledger/book.rs:112-127`（类型）/ `:632`（`established`）/ `:641`（`established_pack`） |
| 备战档禁区闸门现成机制 | `retrace_ledger/portal.rs:39-41`（`TradableSignal` 只 pack 实现）+ `portal.rs:51-70`（`compile_fail` doctest）+ `tests/standby.rs:83-95`（正面对拍） |

### 2.2 位置裁量 = 候选 B（`trading/` 下新独立模块），理由三条

票面要求「选与 #587 迟到过滤同侧的位置，说明理由」。#587 对迟到三类点的裁定原文（**开工亲核**，
`docs/adr/0001-graded-exit-and-shortdiff-doctrine.md:273`，ADR-0001 补充十六「消费矩阵」）：

> **迟到三类点**（086:76 形态）：不追——缠师自认「几乎没有任何操作意义，反而是要考虑下来的那个
> 第二类买点」；**登记观测，不作独立信号消费**。

据此，「同侧」= **观测登记侧**，不是入场判据侧。三条理由：

1. **与 #587 同侧**：本票交付的就是那个「登记观测」面，故它必须落在**不参与入场判据**的位置。
   候选 C（`runner.rs` 主循环挂点）在入场判据侧，且触碰 V0≡P5 parity 承重面，方向相反 ⟹ 排除；
2. **不占用中枢账词汇**：候选 A（`center_book.rs` 同族）的词汇是**中枢生死**，本票登记的是
   **三类点成立本体事件**——同一教义事件的不同断言面（§5）。塞进 `CenterBook` 要扩它的只读面
   并在一个类型里混两套词汇，且它已有 #637 的证明消费通道，语义会重叠成两条同义入口 ⟹ 排除；
3. **隔离最净**：新模块**零入边**（`runner`/`master`/`allocator`/`level_operating_unit` 无一
   `use` 它，`trading/mod.rs` 模块头已如实登记为「图外挂件」），只依赖 theta_v0 账本公开产出面，
   parity 面一行未改。

**副作用如实登记**：新模块的 `pub` 项在非测试构建下无调用者 ⟹ `cargo build` 报 10 条
`dead_code` 警告（`third_point_book.rs:129/138/149/160/179/211/228/242/247/251`）。与 #639 的
`t3_in_c_grade_reason_to_pan_div_subtype` 等项同族（仓内既有先例，本轮 doc 构建同时命中）；
**不加 `#[allow(dead_code)]` 掩盖**——驱动链接上即消失（#575 后续票）。

## 3. 两项裁定登记（编排者 2026-07-29）

### 裁定一（迟到判据）：照实登记，不新造判据

- **落地形态**：`ThirdPointBook` 对 `confirmed_as_of` **不设任何判定分支**——无阈值、无「当前
  时刻」入参、无排序/丢弃规则；`register` 的判定次序只看「身份是否在册 + 内容是否逐字段相等」。
- **票面「迟到 pack 由交易层侧过滤规则处置」按裁定一结**：交易层侧的处置 = 登记观测、不产独立
  信号（#587 字面），**无新量化规则**。测试 `late_pack_is_registered_verbatim_regardless_of_confirmed_clock`
  钉死：`confirmed_as_of=12` 的迟到 pack 与 `confirmed_as_of=5000` 的新鲜 pack 同样 `Registered`，
  知情时原样在册（供 #680 读取），且**账本侧零参与**（`assert_eq!(late_ledger, ledger_before)`
  —— `RetraceLedger` 全值相等，登记不回写账本任何一个 bit）。
- **判据留白理由**（如实登记，非遗漏）：判「迟到」需要与 `confirmed_as_of` 可比的当下时钟坐标；
  交易层 bar 坐标（磁带行号）与账本知情时是**两套引擎各自的坐标**，仓内无对齐依据。同族约束 =
  #637 裁定 2A（`RetracePoint.price` 是 theta_v0 `Tick` 刻度计数、trading 侧价格是原始 `f64`，
  跨量纲比较禁止）。⟹ 本账任何「位置」「时点」字段一律只登记、不比较。判据归 **#680**。

### 裁定二（接线口径）：不立生产驱动挂点

- 无 `RetraceLedger` 生产实例、不进 `runner` 主循环；「生产调用点 ≥1（grep 可证）」按字面结：

```
$ grep -n "ThirdPointPack\|established_pack\|ledger.established()" src/trading/third_point_book.rs | grep -v "^\(1[0-9]\|[1-9]\)[0-9]*://!"
152:    established: BTreeMap<RetraceKey, ThirdPointPack>,      # 生产（非 cfg(test)）
183:        let pack: ThirdPointPack = signal.into();            # 生产
216:        for pack in ledger.established() {                   # 生产：读账本全量成立档门户
233:        match ledger.established_pack(key) {                 # 生产：读账本单身份成立档门户
242:    pub fn established(&self) -> Vec<ThirdPointPack> {       # 生产：观测面
247:    pub fn pack(&self, key: &RetraceKey) -> Option<ThirdPointPack> {   # 生产：观测面
```

  `#[cfg(test)]` 起于 `third_point_book.rs:264` ⟹ 上列全部位于生产段。达标口径 = 类型 /
  `established_pack` 出现于生产代码，**不是**「已被生产调用方驱动」。
- **端到端由靶向驱动**：测试内经 `RetraceLedger::observe` 真实产 Confirmed → `established_pack`
  取包 → 交易层入口（与 #637 的 8 测试模式同构，不手搓 pack 字面量；唯一手搓处 = 改口负控
  `ThirdPointPack { confirmed_as_of: +7, ..pack }`，用途仅 fail-loud 负控）。
- **登记表措辞**（`retrace_ledger/mod.rs` row 2）：「**已接线（#638）：消费入口已立；驱动链归
  #575 后续票**」；计数行 2/3 → **3/3（入口计数口径）**，并显式声明「#2'（`StandbyWatch` 备战档）
  仍是未接线且按裁定八禁区不进本轮计数」。

## 4. 禁区证明说明（票面验收第 2 条）

**闸门形态**：消费入口签名 `pub fn register<S>(&mut self, signal: S) -> …  where S: TradableSignal
+ Into<ThirdPointPack>` —— 两条 bound 各自独立否掉备战档：

1. `S: TradableSignal`：`portal.rs:39-41` 只对 `ThirdPointPack` 实现，`StandbyWatch` 故意不实现；
2. `S: Into<ThirdPointPack>`：账本 `portal` 模块头明文声明两型**互无 `From`/`Into`**。

泛型在此不是扩展点，是把禁区写进签名——具体型入参会让闸门退回注释。

**测试面证据**（`standby_watch_is_barred_from_the_registration_entry`）：

- ① 正面：`fn gate<S: TradableSignal + Into<ThirdPointPack>>` 收成立档通过（与入口同 bound）；
- ② 反面结构墙：对 `StandbyWatch` **字段表全解构**，证明它结构性缺 `retest_end` /
  `confirmed_as_of` / `death_certificate` 三项载荷——即便日后有人显式给它实现 `TradableSignal`
  （破坏性改动，非静默扩权），也拿不出本入口所需载荷；字段表一变本行即编译失败（活守卫，
  仿 `tests/standby.rs:66-81` 先例）；
- ③ 路径面：`sync_from_ledger` 后登记账内**只有**成立档身份，`book.pack(&watch.identity).is_none()`。
  另有 `provisional_and_failed_identities_never_reach_the_book`：未决 / 判败身份均不进册。

**编译期负向强制的位置声明（如实登记，非偷工）**：`trading` 是私有模块，`compile_fail` doctest
以**外部 crate** 身份编译，够不到本模块路径——在此写 `compile_fail` 只会因「模块私有」而失败，
是**假证明**，故本票不写。同一 trait 上的负向编译期强制由账本侧既有 doctest 承担
（`retrace_ledger/portal.rs:51-70`，本轮 `cargo test --doc` 实跑命中：
`portal.rs - …portal::StandbyWatch (line 51) - compile fail ... ok`），本票 bound 与它是同一道闸；
本模块侧的等价证据 = 上述 ②③。

## 5. 语义重叠声明（三条观测通道）+ #664 关系声明

18 课定理三充要条件下「三类点成立」与「中枢死亡」是同一教义事件的两个断言面。仓内三条通道：

| 通道 | 位置 | 登记的是 |
|---|---|---|
| ① | `CenterBook::ingest` confirmed Type3 kill 分支 | 中枢死亡（自 BSP 事件锚 diff） |
| ② | `CenterBook::consume_death_certificate`（#637） | 中枢死亡（消费账本外部证明） |
| ③ | 本票 `ThirdPointBook::register`（#638） | **三类点成立本体**（不发中枢死亡语义） |

本账**不发**任何中枢生死语义、**不动** `CenterBook` 的 `dead`/`frozen`/`version`、**不递**
`pack.death_certificate` 给通道 ②。不递的技术理由（如实登记）：`consume_death_certificate(ladder,
cert)` 按 ladder 消费，而 `ThirdPointPack` 不带 ladder/级别坐标（级别在账本
`RetraceProvenance.level` 上，与 trading 的 ladder 是两套引擎坐标，仓内无对齐依据）⟹ 跨引擎级别
坐标对齐归 #575 驱动票。

**#664 关系声明**：`ThirdPointPack` 自带 `side`（`book.rs:114`）⟹ #664（`CenterDeathCertificate`
无 `side` 的方向盲区）**不约束**本票的 pack 消费面——在册每条成立档方向明确。若 #575 驱动票日后
把 `pack.death_certificate` 转投通道 ②，#664 盲区照旧适用（cert 本身仍无 side），走 #664 修，
本票不修。

**#636 只引两条**（票面限定）：命名纪律——本票任何对象不自称/不混用「链证书」词汇（`chain_cert`
不进范围，全文零引用）；「源头照实、判断留给消费端」哲学——与本票「账本照产全素材、交易层照实
登记不判」同构。

## 6. 名分四态（世代宪法 §1）

本模块 = **新立消费入口，驱动链未接**。按 §1 机械判据，「现役」要求**有非测试调用者**，本模块
当前只有测试调用者 ⟹ 字面落「生产零可达但有测试调用者」那一格，而该格标签是「deprecated
待退役」——**语义方向相反**（新生入口，非退役残留）。三面同构（#637 通道 ② / #639 短差通道 /
本票）均处此境。**如实登记这处判据张力，不自裁**：是否为「新生入口」补判据例外或立第五格，
上浮宪法线（图 #500）。声明已写入模块头 §名分，避免后来者按字面判据给新生入口「看病」。

## 7. 测试清单（8 条，全部 in-crate `trading::third_point_book::tests`）

| # | 测试 | 覆盖 |
|---:|---|---|
| 1 | `established_pack_registers_end_to_end_verbatim` | 端到端：真实产 Confirmed → `established_pack` → `register` → 观测面可见；`book.established() == ledger.established()`（零投影） |
| 2 | `ledger_sync_registers_every_established_pack_in_key_order` | `sync_from_ledger` 读 `established()`；多身份按键序；重复同步零新增（幂等）+ 两计数器 |
| 3 | `repeated_registration_of_the_same_pack_is_idempotent_zero_action` | 幂等确认零动作，`registrations` 不前进 |
| 4 | `conflicting_pack_for_the_same_identity_fails_loud` | 同身份改口 → `ConflictingPack`，在册内容不被覆盖 |
| 5 | `late_pack_is_registered_verbatim_regardless_of_confirmed_clock` | 裁定一：迟到照实登记、知情时原样在册；**账本侧零参与**（账本全值前后相等） |
| 6 | `standby_watch_is_barred_from_the_registration_entry` | 禁区三重证据（正面 bound / 字段表结构墙 / 路径面不在册） |
| 7 | `provisional_and_failed_identities_never_reach_the_book` | 未决 + 判败身份不进成立档登记账 |
| 8 | `register_from_ledger_reads_the_single_identity_portal` | `established_pack` 生产读点；非 Confirmed → `Ok(None)`；side/位置/死亡证明照实可读 |

TDD 纪律：先写 8 条测试 → `cargo test --lib trading::third_point_book` 报 `error[E0433]: cannot
find type ThirdPointBook`（红）→ 落实现 → 8/8 绿。

## 8. 指纹对照

| 时点 | 命令 | 结果 |
|---|---|---|
| 开工干净基线 | `cargo test --lib` | **2550 passed; 0 failed; 138 ignored** |
| 终跑 | `cargo test --lib` | **2558 passed; 0 failed; 138 ignored** |
| 终跑 | `cargo test --doc` | **3 passed; 0 failed; 3 ignored**（含 `portal.rs:51` 禁区 `compile_fail` doctest ok） |
| 终跑 | `cargo test --lib trading::third_point_book` | 8 passed; 0 failed |

差额 2558 − 2550 = **+8 = 本票新增测试数**，无既有测试增减。**0 failed 硬杠达成**；known flaky
`incremental_tower_scaling_dominates_full_synthetic`（#619 L9 在册）本轮两次均 pass。

`cargo doc --no-deps --document-private-items`：新模块**零 intra-doc 链接告警**（10 条告警全为
上文 §2.2 登记的 `dead_code`）。`retrace_ledger/mod.rs` 登记表内的 `[`ThirdPointPack`]` /
`[`book::RetraceLedger::established`]` 等链接**在本票之前即为未解析**（表格 row 1 的同类链接同样
命中，crate 全局共 569 条 unresolved link 告警）⟹ 本票 row 2 沿用同一既有写法，**不新增缺陷
类别**，也不在本票顺手修（越范围）。

## 9. Standards 自查

| 项 | 自查 |
|---|---|
| 语言 | 全中文 doc/注释/报告，跟随文件既有中文 doc 风格（AGENTS.md §语言） |
| TDD | 先失败测试后实现（§7 已录红步错误码） |
| 改动最小化 | 三个文件、无顺手重构；`trading/` 既有 29 个模块一行未改（除 `mod.rs` 注册 + 头注） |
| 不动账本侧 | `retrace_ledger/` 代码零改动（§1 `git diff` 空输出实证），只改登记表 doc 文字 |
| 如实登记 | 判据留白（#680）、名分张力、dead_code 副作用、坐标未对齐、doc 链接既有缺陷——五处均正文声明，不藏 |
| 禁 git mutation | 全程零 `commit`/`push`/`reset`/`rebase`/`stash`（本报告与代码留未提交面） |
| 前台单线程 | 无子代理、无后台进程；cargo 在 `rust/` 前台跑，未设共享 `CARGO_TARGET_DIR` |
| 工位边界 | 只在 `/private/tmp/wt-638` 内改动；主仓 `/Users/silencehan/Projects/NewChanlun` 与 `/private/tmp/kimi-nest-mainline` 零触碰 |
| 命名纪律（#636） | 无「链证书」词汇；新词 = 「三类点成立登记账」，与「中枢死亡证明」明确分家 |
| 编号纪律 | 报告内 `#N` 全指 GitHub Issue |

## 10. 遗留风险 / 未裁项

1. **迟到判据留白 → #680**（blocked_by 本票）：本票只交登记 + 观测，**无任何迟到量化规则**。
   #680 落地前，任何「本账已能识别迟到 pack」的说法都是误读。
2. **跨引擎时钟坐标未对齐**：交易层 bar 坐标 ↔ 账本 `as_of` 知情时无对齐依据；同族地 trading
   ladder ↔ 账本 `RetraceProvenance.level` 亦无映射。⟹ #680（时钟）/ #575 驱动票（级别）。
3. **位置判据受 #637 裁定 2A 约束**：`RetracePoint.price` 是 theta_v0 `Tick`，trading 侧价格是
   原始 `f64`，**禁止跨量纲比价**。本票任何字段只登记不比较，故未触雷；后续票若要拿 pack 的
   位置与 trading 价格比，必须先立量纲转换的权威口径。
4. **驱动链未接（裁定二）**：`RetraceLedger` 无生产实例 ⟹ 本模块生产路径上恒为空账。所有 8 条
   测试是靶向驱动，**不构成「生产上会发生什么」的证据**。
5. **`dead_code` 警告 10 条**：驱动链接上即消失；本票选择不掩盖（§2.2）。
6. **名分第五格未裁**：见 §6，上浮宪法线。
7. **备战档消费形态未裁**：登记表 row 2' 保持「未接线」，裁定八禁区不动。
8. **整测覆盖范围声明**：本轮跑 `cargo test --lib`（全量单测）+ `cargo test --doc`；
   `rust/tests/` 下集成测试（parity/fixture 族）**未跑** ——豁免命题：新模块零入边（无任何既有
   模块 `use` 它），parity 面与 fixture 面无数据/控制流连接；证据：`git diff --stat` 显示除
   `trading/mod.rs` 模块注册行外无既有生产文件改动，且 `cargo test --lib` 内含 parity 单测族
   全绿（2558/0）。按交付纪律「说豁免时同时给出能验真的证据」，此为**部分证据豁免**，不覆盖
   票面若要求集成层验收的情形。

## 11. 修复轮（两轴 + 影子评审聚合，2026-07-29）

一句话：本轮以 doc/报告文字为主 + 新增 1 条测试，**无逻辑重构**，处置两轴 5 判断题
（Standards 2 条 + Spec 3 条）与影子 13 条发现（MEDIUM 5 / LOW 8）。逐条对账见 §11.11；
全量指纹终跑见 §11.12。

### 11.1 影子 MEDIUM-1 处置：`spiral/signal.rs` 补核，候选集补齐为四

首轮候选集只有三个（A=`center_book`、B=新模块、C=`runner` 主循环），票面范围第 1 条明文点名
的 `spiral/signal.rs` 族一字未核。本轮亲核该文件（`rust/src/spiral/signal.rs`，391 行）：

| 事实 | 锚 |
|---|---|
| 自称「信号层是操作层**唯一定义域单元**」 | `signal.rs:3` |
| 消费同一份 legacy 磁带（`BarSig`/`BspEvent`），**非** `theta_v0` 塔链 | `signal.rs:24-27` `use crate::trading::{center_book::CenterBook, tape::BarSig, types::BspEvent}` |
| `SignalState::process`（`signal.rs:173-310`）内含两条 panic 级结构守卫；`prove_chain` 不在 `process` 内，由 `spiral/engine.rs` 逐 bar 调用 | `prove_t53_connection_assoc`（:288-289）/`prove_n5_cascade`（:299-300）在 `process` 内；`prove_chain` 定义于 `signal.rs:317`，调用点 `engine.rs:175/209/302` |
| 持有的 `CenterBook` 是**私有实例**，喂 legacy `BspEvent` 行，与 #637 死亡证明消费面无关 | `signal.rs:157` `book: CenterBook::new()` |
| 与 `theta_v0::classifier::retrace_ledger` **零接触面** | `grep -rn "retrace_ledger\|ThirdPointPack\|StandbyWatch" src/spiral/` 零命中 |
| `process` 的调用方 | `fugue_v3/engine.rs:71`、`spiral/engine.rs:109`（均为逐 bar 热路径） |

**为什么不选它**（候选 D，与候选 C 同族排除）：它自述「操作层唯一定义域单元」——按定义就是
判据侧，方向与候选 C（`runner.rs` 主循环）相同、与 #587「登记观测侧」相反；结构上插入本票的
登记逻辑等于新开一条 `theta_v0 → spiral` 的边，而这条边目前**不存在**（零 import）——比候选 B
（零入边，只挂账本公开产出面）风险高一个数量级。裁量结论维持候选 B（新独立模块），论证现已
闭环（候选 A/B/C/D 全覆盖）。

**落位**：`rust/src/trading/third_point_book.rs` 模块头 §为什么落在这里（新增第 4 条事实项 +
理由四改「理由三」，行 9-45 一带）已补入上述内容，本节为其报告侧对应记录。

### 11.2 影子 MEDIUM-2 处置：`sync_from_ledger` 冲突早退补测试 + doc 补声明

新增测试 `sync_from_ledger_stops_at_first_conflict_but_keeps_prior_admissions`
（`third_point_book.rs:451`）：预登记 `later` 身份的旧内容（`stale`），账本侧对同一身份产出
不同内容的 `later`，`earlier` 身份键序早于 `later`。断言三件事：① `sync_from_ledger` 返回
`Err(ConflictingPack{identity: later.identity})`；② `book.pack(&later.identity)` 仍是
`stale`（在册旧内容不被冲突覆盖）；③ `book.pack(&earlier.identity)` 是 `Some(earlier)`（冲突
前已登记部分保留在册——用 `earlier` 这条合法 pack 验证部分入册）。

doc 补充（`third_point_book.rs:228-235`，`sync_from_ledger` 方法doc）：明文声明
`admitted` 返回清单在 `Err` 路径上随 `?` 整体丢弃，但对 `self.established` 的写入**不回滚**；
调用方拿到 `Err` 后应以 `established()` 重查在册状态，不要依赖本次调用的返回值。

### 11.3 影子 MEDIUM-3 处置：禁区措辞两处反正（改 doc，不改签名）

**(a) `register` 方法 doc**（`third_point_book.rs:190-193`）：
删「具体型入参会让闸门退回注释」，改为——具体型入参 `pack: ThirdPointPack` 本身就是最强的
编译期闸门，闸门强度与泛型无关；泛型在此把 `TradableSignal` 显式写进签名，是**同强度**下把
禁区 trait 变成可 grep 的教义锚，非「更强的闸门」。

**(b) 模块头 §禁区第 2 条**（`third_point_book.rs:88-95`）：
删「也拿不出本入口所需载荷」，改为——也只能用假值凑出载荷；字段表全解构测试保证的是
「字段表一变即编译失败」（活守卫 = **变更探测器**），不是「凑不出载荷」；真正挡住绕路的是
①目前 `TradableSignal` 只有 `ThirdPointPack` 一个实现者、②两型互无 `From`/`Into`，绕路需要
显式新增 impl（可见的破坏性改动）——结构墙让绕路**显式**，不让绕路**不可能**。

禁区在生产段的实际成色（如实收窄）= 类型面 `StandbyWatch` 零出现（票面验收字面，grep 实证）
+ 账本侧既有 `compile_fail` doctest 负向强制；「编译期不可绕」的表述按此收窄，不再声称具体型
更强或字段表阻止伪造。

### 11.4 影子 MEDIUM-4 处置：报告证据指针订正

**首轮指错，如实标注**：本报告 §2.1 表格「`ThirdPointPack` 接线前零生产消费方」一格写作
「仅 `level_view/tests/feature_gating.rs` 一处同名测试函数」——**该指向是错的**，该文件内实测
零命中 `established_pack`/`ThirdPointPack`。真实的同名测试函数是
`retrace_ledger/tests/standby.rs:86`
`fn established_pack_satisfies_tradable_signal_gate_standby_watch_cannot()`——在
`retrace_ledger/` **之内**，不是「之外的例外」。结论本身不受影响（接线前 `ThirdPointPack` 除
`retrace_ledger/` 无任何出现，独立复核成立），仅原报告 §2.1 该格的支撑证据指错文件，本节订正。

### 11.5 影子 MEDIUM-5 处置：迟到测试恒真断言删除

`late_pack_is_registered_verbatim_regardless_of_confirmed_clock`（`third_point_book.rs:495`）
原有 `assert_eq!(late_ledger, ledger_before)` 已删除（连同 `ledger_before` 变量）——该断言在
任何实现下都恒真：`register` 的签名 `fn register<S>(&mut self, signal: S)` 根本不接
`RetraceLedger` 引用，`late_ledger` 与被测代码之间无任何通路，无信息量。改为注释说明「账本侧
零参与」由类型系统保证：`register` 只收 `Copy` 值，`sync_from_ledger` / `register_from_ledger`
只收 `&RetraceLedger`，无内部可变性即无回写通路——比一个恒真运行时断言诚实。保留其余有意义的
断言（迟到者同样 `Registered`、`confirmed_as_of` 原样在册）不动。

### 11.6 影子 LOW-1（两轴 Spec (a)3）处置：`dead_code` 计数按实测

本轮编辑令模块头篇幅增长、行号整体后移，`cargo build --lib` 重新实测（本 worktree，未设共享
`CARGO_TARGET_DIR`）：

```
$ cargo build --lib 2>&1 | grep -c "^warning:"
63   # crate 全局

$ cargo build --lib 2>&1 | grep -A2 "third_point_book.rs"
enum `ThirdPointRegistration` is never used         → :152
enum `ThirdPointConflict` is never used              → :161
struct `ThirdPointBook` is never constructed         → :172
multiple associated items are never used（impl 块）  → :183（含 9 span：
  183 `new` / 203 `register` / 239 `sync_from_ledger` / 256 `register_from_ledger` /
  270 `established` / 275 `pack` / 279 `contains` / 283 `len` / 287 `is_empty`）
```

即：本模块共 **4 条 `dead_code` 警告 / 12 个 span**（3 个单项 + 1 个 9-span 复合项），行号
随本轮 doc 编辑后移但计数不变——与影子实测（4 条/12 span）口径一致；首轮报告 §2.2/§8/§10.5
「10 条」的旧计数按此订正为「4 条 / 12 span」，行号清单不再单列旧行号（已随 doc 篇幅后移作废，
以本表重新实测值为准）。驱动链接上即消失（#575 后续票），本票仍不加 `#[allow(dead_code)]` 掩盖。

### 11.7 两轴 Standards-1 处置：#680 措辞对齐

`third_point_book.rs:54-57`（裁定一段落）与 `retrace_ledger/mod.rs` 消费方登记表 row 2 均已
补澄清：`gh issue view 680` 原文「前置条件」= `RetraceLedger` 接生产驱动、有真实 pack 数据流
（#575 后续票）；GitHub 的 `blocked_by #638` 是依赖排序边——#638 只是缘起 / 登记面提供者，不是
#680 的前置条件。两处 doc 均补一句「勿把 `blocked_by` 读成『#638 是 #680 的前置条件』」，避免
后来者按依赖边字面误读为本票是 #680 的门槛。

### 11.8 两轴 Standards-2 处置：裁定编号统一映射

| 报告内编号 | 内容 | 对齐 |
|---|---|---|
| 裁定一 | 迟到判据：照实登记，不新造判据 | = 用户面 A（2026-07-29 编排者裁定 A） |
| 裁定二 | 接线口径：不立生产驱动挂点 | 同 #637 裁定 3A / #639 裁定 B 先例（票面「生产调用点 ≥1」按字面结口径一致） |

模块头（`third_point_book.rs`）按裁定要求**不动**，此映射只登记在报告侧。

### 11.9 两轴 Spec (a)2 处置：豁免登记保持（编排者已批，不改）

§10.8 的 `rust/tests/` 集成层豁免（命题 = 零入边无数据控制流；证据 = `git diff --stat` 除
`trading/mod.rs` 模块注册行外零既有生产文件改动 + `cargo test --lib` 全绿）**保持原样不改**。
关票声明将引「部分证据豁免，仅豁免默认项」，口径与首轮一致。

### 11.10 影子 LOW-2~8 处置：留档不改

按影子放行建议，LOW-2（`#636` 票面限定出处标注失准）、LOW-3（`:359` 「登记面 ≡ 账本产出面」
注释过强）、LOW-4（三通道互不代劳无结构强制）、LOW-5（迟到测试构造与 #587 语义不同构 + 两本账
拼法非必要）、LOW-6（`registrations`/`idempotent_repeats` 为 `pub` 无封装保护）、LOW-7（登记表
「已接线」人话读感）、LOW-8（#587 迟到子集适用域越域背书）**均留档不改**——影子已判定不影响
行为、仅措辞/文档成色问题，不要求本轮处置。

### 11.11 逐条对账表

**两轴 5 判断题**

| # | 轴 | 内容 | 处置 | 落位 |
|---:|---|---|---|---|
| 1 | Standards-1 | #680 `blocked_by` 措辞对齐 | 改 | §11.7 |
| 2 | Standards-2 | 裁定编号统一映射 | 改（报告侧登记） | §11.8 |
| 3 | Spec (a)2 | 集成层豁免登记保持 | 不改（已批） | §11.9 |
| 4 | Spec (a)3 | dead_code 计数按实测 | 改 | §11.6 |
| 5 | Spec (c) | 迟到测试弱断言处置 | 改 | §11.5 |

**影子 13 条**

| # | 级别 | 内容 | 处置 |
|---:|---|---|---|
| MEDIUM-1 | MEDIUM | `spiral/signal.rs` 族未核 | 改（补核 + 候选集补齐为四）§11.1 |
| MEDIUM-2 | MEDIUM | `sync_from_ledger` 冲突早退零测试 | 改（补测试 + doc 声明）§11.2 |
| MEDIUM-3 | MEDIUM | 禁区证明成色两处反了 | 改（两处 doc 措辞）§11.3 |
| MEDIUM-4 | MEDIUM | 报告证据指错文件 | 订正（如实标注首轮指错）§11.4 |
| MEDIUM-5 | MEDIUM | 「账本侧零参与」断言恒真 | 改（删断言 + 改注释）§11.5 |
| LOW-1 | LOW | dead_code 计数/span 失准 | 改（实测值）§11.6 |
| LOW-2 | LOW | 「#636 票面限定」出处失准 | 留档不改 §11.10 |
| LOW-3 | LOW | `:359` 等式注释过强 | 留档不改 §11.10 |
| LOW-4 | LOW | 「不递」缺结构强制 | 留档不改 §11.10 |
| LOW-5 | LOW | 迟到测试构造不同构 + 两账本非必要 | 留档不改 §11.10 |
| LOW-6 | LOW | 计数器 `pub` 无封装保护 | 留档不改 §11.10 |
| LOW-7 | LOW | 「已接线」人话读感 | 留档不改 §11.10 |
| LOW-8 | LOW | #587 适用域越域背书 | 留档不改 §11.10 |

抽核锚点：MEDIUM-1→`third_point_book.rs:9-45`；MEDIUM-2→`:228-235`/`:451`（测试）；
MEDIUM-3→`:88-95`（模块头）/`:190-193`（`register` doc）；MEDIUM-4→本节 §11.4 文字订正；
MEDIUM-5→`:495` 一带（测试，断言已删）；LOW-1/(a)3→本节 §11.6 实测表；Standards-1→
`:54-57` + `retrace_ledger/mod.rs` row 2。

### 11.12 全量指纹终跑

| 时点 | 命令 | 结果 |
|---|---|---|
| 开工干净基线 | `cargo test --lib` | 2550 passed; 0 failed; 138 ignored |
| 首轮终跑 | `cargo test --lib` | 2558 passed; 0 failed; 138 ignored |
| **本轮终跑** | `cargo test --lib` | **2559 passed; 0 failed; 138 ignored** |
| 本轮终跑 | `cargo test --lib trading::third_point_book` | **9 passed; 0 failed**（8 首轮 + 1 本轮新增） |
| 本轮终跑 | `cargo test --doc` | **3 passed; 0 failed; 3 ignored**（与首轮一致） |
| 本模块 `dead_code` | `cargo build --lib` | **4 条警告 / 12 个 span**（§11.6，行号随 doc 篇幅后移） |
| crate 全局警告 | `cargo build --lib` | 63（订正：本轮尾部评审实测值，62 为旧计数） |

差额对账：2559 − 2558 = **+1 = 本轮新增测试数**
（`sync_from_ledger_stops_at_first_conflict_but_keeps_prior_admissions`），无既有测试增减、
无既有测试改动导致的通过/失败翻转。**0 failed 硬杠达成**（本轮 + 首轮 + 基线三点均 0 failed）。

### 改动文件清单（本轮，累计于首轮之上）

| 文件 | 改动性质 |
|---|---|
| `rust/src/trading/third_point_book.rs` | 模块头 §为什么落在这里 扩写（候选 D 补核）+ §禁区第 2 条措辞订正 + `register`/`sync_from_ledger` doc 订正 + 裁定一 #680 措辞澄清 + 新增 1 条测试 + 删 1 条恒真断言 |
| `rust/src/theta_v0/classifier/retrace_ledger/mod.rs` | 消费方登记表 row 2：#680 `blocked_by` 措辞澄清（仅 doc 文字，账本侧代码仍零改动） |

`rust/src/trading/mod.rs` 本轮零改动（首轮改动保持不变）。全程未 commit，`git status` 三项
（同首轮）无 commit 痕迹。
