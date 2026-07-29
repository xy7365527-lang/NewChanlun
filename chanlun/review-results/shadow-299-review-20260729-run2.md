# 影子评审：#299 P0a NestLifecycleBook 五不变量（第二轮，新上下文）

- **票**：#673（影子评审）／对象票 #299
- **对象**：`/tmp/wt-673-obj`，detached @ `919eb47df5`（= `ticket-299` 尖）
- **基座**：`main`
- **评审人**：claude opus 新上下文，与 #299 实装线（codex gpt-5.6-sol ultra）零瓜葛
- **日期**：2026-07-29
- **只读评审**：未改动 worktree 内任何文件（`git status` 全程 clean）；未碰 `/tmp/wt-299`
- **探针载体**（仓外）：`/tmp/shadow673/Probe.lean`

---

## 判定：**CONDITIONAL**（无 H；5 M / 6 L）

- **Standards 轴：绿。** 146 jobs 绿且另经强制真 elaborate 复核（防缓存假绿）；零 sorry／零 admit／零新公理，公理谱只出现 Lean 内建三条、4 条定理零公理依赖；diff 两格无夹带；fixture 漂移 exit 0；090 有效域与三条降级注释诚实分级、无声明膨胀。
- **Spec 轴：五条不变量实质承重成立，但验收线有一条整项未核、一处臂内空洞。** ①③④⑤ 逐条对回 rust 断言后确认不是「更弱命题顶替」（④⑤ 反而强于 rust 运行时断言）；② 主体成立但 IdentityVanished 臂写成 `True` 且我实测反例 `advance` 真实可达；票体验收线「声明锚互指（rust doc ↔ Lean 定理）」的 rust→Lean 方向完全缺失，编排验收评论也未核这一条。

**放行条件**：M1–M5 核销后可关票。M2 有两种合规出路（补证 / 显式登记为第四条降级注释），M1 有两种（补一行 rust doc 锚 / 按 delivery-discipline 显式降级），选哪条属编排裁量。

---

## 一、独立验证实测（全部本轮亲跑，照实记录）

### 1.1 `lake build`

```
cd /tmp/wt-673-obj/formal && lake build
→ Build completed successfully (146 jobs).
```

- **warning 数 = 1**，且**不在本票 diff 面内**：`Origin/SelfSimilarity.lean:414:5`（`Variable name 'hθ' is not explicitly referenced`，linter.unusedVariables）。该警告在 main 上已存在（票 #299 2026-07-27 评论已在案登记 main 有 2 处 linter 警告）。
- **本票新文件 `Origin/NestLifecycleBook.lean` 自身零 warning、零 error。**
- job 数 146 vs 票 07-27 评论记录的 main 侧 144：+2 与「新增 1 个 root（含其 olean/ilean job）」一致。

### 1.2 防缓存假绿：强制真 elaborate

进入 worktree 时 `formal/.lake/build` 已存在（前序评审已构建过），故上面的 146 绿**可能是热缓存**。补做单文件强制 elaborate：

```
cd /tmp/wt-673-obj/formal && lake env lean Origin/NestLifecycleBook.lean
→ EXIT=0，real 2.674s
→ 15 条 #print axioms 全部输出（下表）
```

exit 0 且 15 条审计输出齐全 ⟹ 该文件本轮**真被重新 elaborate 过**，不是缓存回放。

### 1.3 零 sorry／admit／新公理

```
grep -n "sorry\|admit\|axiom" formal/Origin/NestLifecycleBook.lean
→ 15 命中，全部是 :978-992 的 #print axioms <定理名> 审计命令
```

- 无 `sorry`、无 `admit`、无 `axiom` **声明**（15 命中是审计命令，非声明——已逐行分辨）。
- 公理谱（`lake env lean` 实测输出）：

| 公理依赖 | 定理 |
|---|---|
| 零公理 | `clock_discipline`、`force_overtake_evidence_in_payload`、`never_constituted_evidence_in_payload`、`identity_vanished_has_no_force_evidence` |
| `propext` | `confirmed_invalidated_side_empty`、`invalidated_has_payload` |
| `propext, Quot.sound` | `force_reason_first_complement`、`toClosed_some_iff_confirmed` |
| `propext, Classical.choice, Quot.sound` | `advance_legal`、`confirmed_absorbing`、`invalidated_absorbing`、`first_provable_write_once`、`new_first_provable_at_current`、`history_append_only`、`newly_invalidated_archived` |

全部为 Lean 内建（`Classical.choice` 由 `simp`/`omega`/`by_cases` 引入），**零项目新增公理**。

### 1.4 diff 面

```
git diff main...HEAD --stat
 formal/Origin/NestLifecycleBook.lean | 994 +++++++++++++++++++++++++++++++++++
 formal/lakefile.toml                 |   2 +-
 2 files changed, 995 insertions(+), 1 deletion(-)
```

- **恰为票面两格，无第三处改动，无夹带。**
- `lakefile.toml` 唯一改动 = `roots` 数组**末尾追加** `"Origin.NestLifecycleBook"`；前 89 项顺序与拼写逐字未变（已对过前后两行全文）。
- `git status` 全程 clean（无未提交残留干扰）。

### 1.5 fixture 漂移（CLAUDE.md #263 强制节拍，凡动 `formal/` 收尾必跑）

```
python3 scripts/check_fixture_drift.py  → DRIFT_EXIT=0
[center] theta_v0_center_parity.json ← Origin/CenterConstruct.lean  ✓ 语义一致（0 字段差异）；字节级：一致
[parity] theta_v0_parity.json        ← Origin/ParityFixtureExport.lean ✓ 语义一致（0 字段差异）；字节级：一致
```

### 1.6 探针（4 条命题，`lake env lean /tmp/shadow673/Probe.lean` → **EXIT=0**）

| # | 命题 | 结果 |
|---|---|---|
| P1 | `advance (advance (openEntry 0) 5 (.vanished .hypothesisRefuted)) 9 (.seen .unavailable true)` 的 `clocks.invalidatedAt = some 5` ∧ `lastAsOf = 9`（两式 `rfl`） | **成立** → M2 反例真实可达 |
| P2 | 手构 `InvalidatedSnapshot`（`invalidatedAt = 100`、`lastAsOf = 0`、`identityVanished`）可构造，且 `clock_discipline` 对它成立 | **成立** → M2 该臂零信息量 |
| P3 | 票体 I7「倒退拒绝 ⟹ entry 零改动」：`advance entry current feed = entry`，3 行 tactic（`rcases` + 两个 `simp`） | **一次通过** → L1 |
| P4 | ⑥ 所称模型侧性质「unavailable 不制造失效」：`(advance entry current (.seen .unavailable sc)).state ≠ .invalidated`，约 10 行 | **一次通过** → L2 |

附带事实：`touchProvisional`／`touchConfirmed`／`touchInvalidated` 是 `private`（探针首跑因此报 unknownIdentifier）——封装到位，这是好事，登记为正面观察。

---

## 二、Spec 轴：验收线逐条

票 #299 验收线两条（票体）+ 范围定稿五条 ①–⑤ + 降级三条 ⑥⑦⑧。

### 2.1 定稿 ①　三态转移合法　→ **PASS**（附 M3）

| 子项 | Lean 承重 | 对回 rust |
|---|---|---|
| Provisional→Confirmed/Invalidated | `advance_legal` :551 | `advance` :1083-1213 分支 |
| 终态吸收 | `confirmed_absorbing` :579、`invalidated_absorbing` :598 | `admit` → `TerminalAbsorbed`，ledger_kernel:391-395 |
| 无第四态 | `state_trichotomy` :63 | `NestEventState` 三枚举 |
| Confirmed 臂不碰 Invalidated 字段 | `confirmed_invalidated_side_empty` :619 | `assert_invariants` :1363「终态互斥」 |

- `LegalStateStep`（:542-546）对 `provisional` 源写 `True`：**这是穷尽而非弱化**——三个目标态从 Provisional 全都合法可达，与 rust :1187-1196（第二终局路径，曾可证 ∧ 反超 ⟹ 直接 ForceOvertake，无需先 StructureCompleted）+ :1198-1213（Unavailable 诚实滞留 Provisional）逐条相符。
- 终态吸收只对上 rust 两条吸收路径中的一条 → **M3**。

### 2.2 定稿 ②　first_provable 钟纪律 + 五钟偏序　→ **CONDITIONAL**（M2）

- `first_provable_write_once`（:687）结论是精确等式 `= some first`，因此同时封住后移与前移——与 `ledger_kernel/mod.rs:152-161` `first_write_clock` 的 doc「已写则**既不后移也不前移**」逐字对齐。**真净增量**：票体自述 rust 侧此项仅有 `.is_none()` 守卫、无独立断言、无对抗测试。
- `new_first_provable_at_current`（:732）见证「只在当前喂入点首写 + 禁回填」。
- `clock_discipline`（:670）：五钟分支化偏序。Confirmed 臂（:637-641）逐字就是票体 I1 的全链 `observed ≤ first ≤ structure_end ≤ confirmed ≤ last_as_of`，与 rust :1357-1362 同款。文件明写「五钟偏序是路径敏感的，不是总链」（:629）——诚实且正当（Provisional 没有 structure/confirmed 钟）。
- **缺口**：`.identityVanished _ => True`（:190 结构字段、:665 谓词）→ **M2**。

### 2.3 定稿 ③　原因码互补　→ **PASS**

- `force_reason_first_complement`（:783）是**双向 iff**：ForceOvertake ⟺ ∃ first；NeverConstituted ⟺ first = none。对回 rust :1374-1399 两臂（`expect("反超定义要求曾可证")` / `assert!(first_provable_at.is_none())`）语义同款。
- 有效域用 `IsForcePayload`（:773）显式排除 IdentityVanished，与 rust :1401-1402 注释「身份消失路径 `invalidated_at` 独立于 `first_provable_at`」一致 ⟹ **是诚实开口，不是遗漏**。定稿 ③ 只讲力度两码，限定域与票面逐字相符。

### 2.4 定稿 ④　Closed-only 消费　→ **PASS**（附 L4）

- `toClosed_some_iff_confirmed`（:843）双向 + `provisional_not_closed`（:849）+ `invalidated_not_closed`（:854）。
- 消费入口 `consumeClosed : ClosedEntry Evidence → ConsumedClosed`（:834）在**类型层**只接 Confirmed 快照，**强于** rust 的运行时过滤 `consumable_closed()  = self.ledger.in_state(NestEventState::Confirmed)`（nest_lifecycle.rs:1057-1059）。
- 与裁定 #64 §2(a)「构建放开、消费不放开」方向一致：Invalidated 留档可查但不可消费（`invalidated_not_closed`）。

### 2.5 定稿 ⑤　Invalidated 留档　→ **PASS**（加分项）

- `history_append_only`（:869）、`newly_invalidated_archived`（:881，真案例分析）、`invalidated_has_payload`（:945）、三条证据投影（:959/964/969）。
- 终态禁删由 `invalidated_absorbing` 的 `SettlementFrame` 含 `history` 全等（:576）承载。
- **加分**：`VanishCause` 被绑进 `identityVanished` 构造子（:82），而 `forceEvidence` 投影对该构造子恒 `none`（:104）⟹ rust 的两条运行时断言「`vanish_cause` 有值 ⟺ IdentityVanished」（:1338-1345，#559）与「IdentityVanished 恒无力度证据」（:1408-1411）在 Lean 侧**升级为类型层恒真**，无法构造违例。
- **加分**：`forceOvertake`／`neverConstituted` 都带 `Option Evidence`（:80-81），与 rust「材料缺失时诚实为 None」同款，未偷换成「每个 Invalidated 都有非空力度证据」——文件 :97-99 自证式注释点名了这个偷换风险。

### 2.6 降级注释 ⑥⑦⑧　→ 降级理由**照实成立**（措辞问题见 M4）

| | 降级词 | 理由是否成立 | 实读证据 |
|---|---|---|---|
| ⑥ | 「未证（R 域参数化前提）」 | **成立** | 力度确在模型外求值：rust :1175 `obs.force(material)`；Lean `ForceCheck` :271 只接收三值 |
| ⑦ | 「定义域禁令（不立证）」 | **成立且可检验命题实测为真** | 声称「状态真值、`ClosedEntry` 与消费入口均不含桥参数」；我 grep `Key\|bridge\|Supersede\|migrat` 全文仅 2 命中，且都在 ⑦⑧ 自己的注释里（:33-34）⟹ 模型内零桥参数 |
| ⑧ | 「挂起／未能判定」 | **成立** | rust 模块头 090 登记 1 明写「这是工程桥，不是 E2E §6.1:248 WireV1 EventKey；并轨前不得进入任何证书真值路径」 |

三条用词分级诚实（「未证」/「不立证」/「未能判定」三种不同状态分用，无一条把未证说成已证）。⑧ 的措辞把缺口说小 → **M4**。

### 2.7 票体验收线两条

| 验收项 | 判定 |
|---|---|
| 「选定不变量的 Lean 定理零 sorry 过 lake build」 | **PASS**（§1.1/1.3） |
| 「声明锚互指（rust doc ↔ Lean 定理）」 | **FAIL 半边** → **M1**（rust→Lean 方向零命中；编排验收评论未核此项） |
| 「`check_fixture_drift.py` 绿」 | **PASS**（§1.5） |

---

## 三、发现（H / M / L）

### H — 无

无证明层错误，无假绿，无夹带，无 sorry/admit/新公理，无「更弱命题顶替票面命题」（唯一臂内空洞见 M2，属未证未登记而非顶替）。

### M1｜验收线「声明锚互指」只做了一半，rust→Lean 回指完全缺失

- 票 #299 验收线第 1 条明写「声明锚互指（**rust doc ↔ Lean 定理**）」——双向。
- Lean→rust 方向在场：`NestLifecycleBook.lean:5-15`（权威锚段，列 rust 文件 + 符号名 + 裁定书）。
- rust→Lean 方向**零**：`grep -rn "NestLifecycleBook.lean\|Origin.NestLifecycleBook\|Origin/NestLifecycleBook" rust/ formal/ --include='*.rs' --include='*.lean' --include='*.toml'` 唯一命中是 `formal/lakefile.toml:104` 的 roots 登记。rust 侧无任何 doc 指回本文 15 条定理。
- 编排验收评论（#299 @ 2026-07-29T20:12Z）逐条列了 ①–⑤⑥⑦⑧ + 机械判据，**未核这一条**。
- 与「diff 最小无夹带」存在真张力：补锚需动第三个文件。按 `docs/agents/delivery-discipline.md`「说豁免时，同时给出能验真的证据」，合规解法是**显式降级登记**（例：「AC-锚互指：rust→Lean 方向未落，因保 diff 两格；降级为不可按互指验收，随 #XXX 补」）。补锚与降级**两者都没做**。
- 锚：`formal/Origin/NestLifecycleBook.lean:5-15`；票 #299 body 验收第 1 条。

### M2｜IdentityVanished 臂钟纪律写成 `True`，与 rust 钉死的方向相反，且反例 `advance` 真实可达

- **rust 钉的是相反方向**：`nest_lifecycle.rs:1404-1407` 对 IdentityVanished 断言 `entry.last_as_of <= invalidated`（注释「失效在『本 prefix 不再产出』时结算 ⟹ invalidated ≥ last_as_of」）。另两臂方向相反：ForceOvertake `invalidated <= last_as_of`（:1380）、NeverConstituted 同（:1396-1399）。
- **Lean 该臂两处都是 `True`**：`InvalidatedSnapshot.reason_discipline` 的 `.identityVanished _ => True`（:190）、`ClockDiscipline` 同臂（:665）。既没证 rust 的方向，也没证反方向；`InvalidatedSnapshot` 的通用字段里也**没有**任何关联 `invalidatedAt` 与 `lastAsOf` 的约束（`invalidatedAt ≤ lastAsOf` 只出现在另两臂的 `reason_discipline` 内）。
- **实测反例可达**（探针 P1，两式 `rfl`）：`advance (advance (openEntry 0) 5 (.vanished .hypothesisRefuted)) 9 (.seen .unavailable true)` ⟹ `clocks.invalidatedAt = some 5` ∧ `lastAsOf = 9`。即 `lastAsOf` 反超 `invalidatedAt` 4 个刻度——**rust 的该臂断言在这个状态上会 panic**。
- **类型层同样抓不住**（探针 P2）：可手构 `invalidatedAt = 100`／`lastAsOf = 0`／`identityVanished` 的合法 `InvalidatedSnapshot`，且 `clock_discipline` 对它成立。⟹ 该臂在 ② 里承重为零。
- **根因（我的判读）**：模型忠实复制了 `ledger_kernel/mod.rs:392` 的 `entry.set_last_as_of(as_of)` **先于** :393 的终态判定，故终态吸收会前移门卫钟；而 rust 侧同一语义另有一条门卫钟冻结的吸收路径（见 M3），模型只有前移那条。
- 定稿 ② 是「钟纪律」整项，此臂**既未证也未登记**（⑥⑦⑧ 三条降级均不覆盖它）。
- 锚：`formal/Origin/NestLifecycleBook.lean:190`、`:665`、`:312-347`；`rust/.../nest_lifecycle.rs:1404-1407`、`:1380`、`:1396-1399`；`ledger_kernel/mod.rs:392`。
- 修法二选一：(a) 在该臂补 `invalidatedAt ≤ lastAsOf` 或 rust 方向的约束并重证；(b) **登记为第四条降级注释**，照实写「rust 该臂钉 `last_as_of ≤ invalidated`，本模型因只建模门卫钟前移的吸收路径而不立此式」。

### M3｜终态吸收只对上 rust 两条吸收路径中的一条，且未登记

rust 有**两条**终态吸收路径，门卫钟行为相反：

| 路径 | 位置 | `last_as_of` |
|---|---|---|
| (A) 桥匹配命中终态 | `nest_lifecycle.rs:1107-1109`，`if old_terminal { continue; }`——**在 `admit` 之前**返回 | **不动**。模块注释 :1467-1469 明写「终态吸收使照喂与不喂的 book 逐位相同（零 revision、`last_as_of` 不动…）⟹ 跳过是纯成本优化」 |
| (B) 直接键命中 | `:1166` `self.ledger.admit(&key, as_of)` → `ledger_kernel:392` 先 `set_last_as_of` 再 :393-394 `TerminalAbsorbed` | **前移** |

- Lean `SettlementFrame`（:572-576）= state／clocks／invalidation／history 全等，`lastAsOf` **可前移**（`Clocks` 不含 `lastAsOf`，:106-113 与 :208-211 分列）⟹ **只对上 (B)**。对 (A) 而言模型严格更弱：rust 是「逐位相同」，模型允许门卫钟动。
- 文件注释 :570「门卫钟可前移，但三态、五钟、终态载荷与留档不变」**诚实描述了自己**，但没说这是 rust 两路中的一路；:22-24 的三条「不声称」也未登记这一点。
- 与 M2 同源：正因为只建模了 (B)，M2 的反例才可达。
- 锚：`formal/Origin/NestLifecycleBook.lean:570-576`；`rust/.../nest_lifecycle.rs:1107-1109`、`:1467-1469`、`:1166-1171`；`ledger_kernel/mod.rs:381-397`、`:140-146`。

### M4｜模型完全没有身份键／桥／迁移层，⑧ 的措辞把这个缺口说小了

- ⑧（:33-35）只说「不定义 `LifecycleKey ↔ WireV1 EventKey` 等价或转换定理」。实测：模型内 `Key|bridge|Supersede|migrat` 全文仅 2 命中，且**都在 ⑦⑧ 自己的注释里**（:33-34）⟹ 模型**根本没有身份键概念**。
- ⑦ 的措辞「白名单工程桥只负责工程身份迁移……本文的状态真值、`ClosedEntry` 与消费入口均不含桥参数」读起来像「桥在场但被围栏隔开」；实际是「桥不在场」。两种情形的形式化承重完全不同。
- 后果：rust 侧挂在身份键上的整族不变量**无对应物且未登记进有效域**：
  - `superseded_from` 不自环 + 两码互斥穷尽（`:1429-1437`，#603 档 1：`bridge_identity` ∨ `bridge_by_center_upgrade`）；
  - ObservationSeam 的 `successor_c_start ≠ key.seg_c_full.0`（`:1419-1423`）——**注意**：Lean `VanishCause.observationSeam (successorCStart : Nat)`（:70）**带了这个字段却没有任何约束**，字段在场而不变量缺席，是最容易被误读成「已覆盖」的形态；
  - 完成信号按桥身份唯一（`:1441-1447`）；
  - `revision` 计数 == 留档长度（`:1312-1316`）——Lean `Entry` 无 revision 计数器（:199-201）；
  - 完成钟两分单调 `completed_at ≤ as_of`（`:1457-1461`，#527 物理完成 ≤ 账本收到）。
- 本条**不是**要求本票形式化它们（票面没点）；是 090「声明 = 能力」要求把它们记进有效域清单。现有三条「不声称」（:22-24）只说了「不逐 bit 等价／不证力度／生产未改接」，未点名上述整族。
- 锚：`formal/Origin/NestLifecycleBook.lean:26-35`、`:70`、`:199-201`；`rust/.../nest_lifecycle.rs:1312-1316`、`:1419-1423`、`:1429-1437`、`:1441-1461`。

### M5｜I 编号与票体 I1–I7 交叉错位，拿票体验收线对文件会系统性对错臂

文件 docstring 用 I1–I5 指**定稿 ①–⑤**；票 #299 2026-07-27 首条评论的 I1–I7 是**另一套语义**：

| 票体 I（07-27） | 语义 | 文件的 I |
|---|---|---|
| I1 | 五钟偏序 | **I2**（:668） |
| I2 | 终态互斥 + 终态吸收 | **I1**（:61/:578/:597） |
| I3 | 消费 Closed-only | **I4**（:841） |
| I4 | 留档不删 | **I5**（:867） |
| I5 | first_provable 只写一次 | **I2 的一部分**（:685） |
| I6 | 原因码语义 | **I3**（:778） |
| I7 | 倒退拒绝 ⟹ 零改动 | **文件无**（→ L1） |

- 五处交叉错位（I1↔I2、I3↔I4、I4↔I5、I5→I2、I6→I3）。同一张票里两套 I 编号并存，任何后续用票体 I 号做验收的人都会对错臂。
- 修法：文件头加 3 行映射表（定稿 ①–⑤ ↔ 文件 I1–I5 ↔ 票体 I1–I7），或统一改用 ①–⑤ 符号。
- 严重度评 M 而非 L：不影响证明有效性，但直接影响**可验收性**，而本票的关票门就是逐条对验收线。

### L1｜票体 I7 既未进定稿五条也未登记降级，而 3 行 tactic 即可证

- 代码已实现倒退零改写（:363-364 `if hretro : current < snapshot.lastAsOf then { next := snapshot, delta := [] }`），但无定理。
- 票体 07-27 把 I7 列在「进 P0a」；07-29 定稿的 ①–⑤ 与 ⑥⑦⑧ **都不含它**——静默消失。定稿是编排者裁的最终范围，故不算违约；但按 delivery-discipline「降级要显式」应留一句登记。
- 实测（探针 P3，一次通过）：
  ```lean
  theorem retro_no_change (entry : Entry Evidence) (current : Nat) (feed : Feed Evidence)
      (h : current < entry.lastAsOf) : advance entry current feed = entry := by
    rcases entry with ⟨snapshot, history⟩
    simp [Entry.lastAsOf] at h
    simp [advance, advanceSnapshot, h]
  ```
- 是全表性价比最高的补项（3 行换一条票体点名的不变量）。

### L2｜⑥ 声称的模型侧性质「不可验不制造失效」可证未证；rust 的审计留痕落差未登记

- ⑥（:27-29）末句「`unavailable` 不等于 `verifiedOvertake`，不得由不可验制造失效」是关于**本模型**的命题（不是关于 R 域的），可证而未立定理。实测（探针 P4，一次通过）约 10 行。
- 另：rust 的 Unavailable 臂**有审计留痕**——`completion_force_unavailable_audits` push（`:1198-1210`）+ `entry.force_unavailable_at = Some(as_of)` + 一条 revision（`:1211-1213`）；Lean `.unavailable` 分支只 `touchProvisional`、`delta := []`（:407-408），零留痕。属可接受的范围裁剪，但未登记。

### L3｜`clock_discipline` 与 `history_append_only` 是构造性同义反复：信息量在定义里，不在证明里

- `ClockDiscipline`（:631-665）逐字等于三个快照结构自己的证明字段；`clock_discipline` 的证明就是把字段重新打包（:674 / :676 / :678-682 全是 `exact ⟨结构字段…⟩`）。
- `HistoryPrefix`（:863-864）由 `advance` 的 `history := entry.history ++ result.delta`（:535）直接 `rfl`（:874-875）。
- 真正的承重在 `advanceSnapshot` 内部那些 `omega` 义务（:293-347、:386-523）——**这是好的 by-construction 设计**，不是缺陷；但意味着若结构字段与 `ClockDiscipline` 谓词被同步削弱，定理仍会绿：**无独立冗余校验**。报告如实登记，供后续加一条与结构解耦的钟序陈述时参考。
- 同理 `history_append_only` 的实质保证是「`advance` 是唯一写史点」（:527 注释），这是**结构性/社会性断言**，不是被证明的性质——本文件内确实只有 `advance` 写 `history`（已 grep 确认），但没有定理禁止外部再写。

### L4｜`ClosedEntry` 是可归约别名，能力值未封口

- `abbrev ClosedEntry (Evidence : Type) := ConfirmedSnapshot Evidence`（:816），`ConfirmedSnapshot` 构造子公开 ⟹ `consumeClosed` 可直接吃手构的 `ConfirmedSnapshot`，不必经 `toClosed?` 取得。
- ④ 的类型门确实挡住 Provisional／Invalidated（这已强于 rust 运行时 `in_state` 过滤），但「只消费**经门的**合法命中」不成立。属可接受设计（`ConfirmedSnapshot` 本身带 `clock_order` 义务，手构也得交证明），登记即可。

### L5｜模块头锚无行号、无实读 commit，而漂移已实发

- `:5-15` 只给文件名 + 符号名（`advance`、`consumable_closed`、`assert_invariants`），无行号、无实读 commit。
- 票 #299 自己点名要复用的范式 `formal/Origin/CertGatedExit.lean:17` 写的是「rust 侧被形式化的机器语义（**当前 HEAD `2bf3c71b58` 实读，非转述**）」+ 逐符号行号（`nest.rs:395-399`、`admission.rs:869-995`、`:1327-1359`、`:1048-1086` 等）。
- **漂移已实发，我实测**：票 07-27 评论引的 `assert_invariants:754-757`／`advance:586-589`／`consumable_closed:532-538`，今日在 `:1305`／`:1083`／`:1057`——两天内全部失效。rust 模块头另已增 #559/#527/#603/#619 等语义。
- 顺带：票 07-27 评论给的 rust→Lean 范式指针 `parser/feature_seq.rs:108-109` 今日**路径不存在**（该文件已不在此路径）——票面自己的锚也漂了。记录为票面事实，非本交付缺陷。

### L6｜`state_trichotomy` 标 I1 但信息量为零，不宜作为 ① 的承重项

- `:63-65` `cases s <;> simp`——「无第四态」是 `inductive LifecycleState` 三构造子声明的直接后果，不是被证明的性质。诚实（没冒充别的），但计入 ① 的承重会虚增。

---

## 四、正面观察（加分项，实读确认）

1. **first_provable 两条定理是真净增量**：票体自述 rust 侧仅 `.is_none()` 守卫、无独立断言、无对抗测试；Lean 侧 `first_provable_write_once` 用精确等式封住双向改写，与 `first_write_clock` doc 逐字对齐。
2. **`VanishCause` 绑构造子 ⟹ #559 的两条运行时断言升级为类型层恒真**（见 §2.5）。
3. **第二终局路径未被伪造成完成**：`.verifiedOvertake` + 曾写 first ⟹ 直接 Invalidated，`structureEndAt := none`（:467），与 rust :1185-1196「反超只要求曾可证，结构完成不是前置条件」逐条相符，且不伪造结构完成钟。
4. **三处自证式反偷换注释**：:97-99（防「载荷有证据字段」偷换成「每个 Invalidated 都有非空力度证据」）、:106（`lastAsOf` 不冒充第六个领域钟）、:629（五钟偏序路径敏感，不是总链）。
5. **`touch*` 三函数 `private`**，封装到位（探针首跑因此报错，反向印证）。
6. **④⑤ 两条 Lean 侧强于 rust**：④ 类型门 > 运行时 `in_state` 过滤；⑤ `VanishCause` 类型层 > 运行时 `assert_eq!`。

---

## 五、附带线索（→ 另票，不计入本票判定）

**rust 侧自身存在张力**：`ledger_kernel:392` 的 `set_last_as_of` 无条件前移门卫钟（含终态），与 `nest_lifecycle.rs:1404-1407` 的 IdentityVanished 断言 `last_as_of ≤ invalidated` 相冲。触发条件：某个已 Invalidated(IdentityVanished) 的 key **精确重现**在后续 `observations` 里 ⟹ `ledger.contains(&key)` 为真 ⟹ 跳过第 1 步桥分支 ⟹ 直落 `:1166` `admit` ⟹ `last_as_of` 前移过 `invalidated_at` ⟹ 下次 `assert_invariants` 在 debug 构建下 panic。

当前不可达的理由是**域论证**而非结构保证：「身份消失」的定义就是该 key 不再产出，重现则说明它没消失。这是两条实现路径（桥匹配冻结钟 vs `admit` 前移钟）的耦合巧合，不是不变量。建议编排开 rust 侧质询票。

（此项与本票判定无关——本票对象是 Lean 文件；列此是因为 M2/M3 的根因追到这里。）

---

## 六、090 限度登记（我**没**核什么）

1. **未核 rust ↔ Lean 逐 bit 等价**——文件自己也不声称（:22）。我只核了「Lean 语句 vs rust 断言的**语义是否同款**」，方法是逐条读 rust 断言原文对臂，**非运行对拍**。
2. **未跑 rust 测试**（`cargo test --lib nest_lifecycle`）：本票 diff 不含 rust，票 07-27 评论已在案「11 passed / 0 failed」。我没重跑，故**不为 rust 侧现状作证**。
3. **`lake build` 未分辨冷/热**：进入 worktree 时 `formal/.lake/build` 已存在（前序评审已构建过），146 绿可能来自热缓存。已用 `lake env lean` 单文件强制真 elaborate 补掉本文件这个洞（exit 0，15 条审计输出齐），但**全库 146 job 的冷构建我没做**。
4. **未打开裁定书 `chanlun/escalate/r43-lifecycle-ruling-20260721.md`**：文件把它列为权威锚（:15），我没逐条对 §2(a)(b)/§4 原文，只对了 rust 断言与票面。故「与裁定书一致」这一条**我不作证**（§2(a) 的「构建放开/消费不放开」我是经 rust :1054-1069 的注释转引确认的，非读裁定书原文）。
5. **未核 ADR-0003 与 #415 指定的三词词汇表**。「结构完成」不得称「闭合」这条撞词纪律我只做了 grep 级检查（文件用 `structureCompleted`／`structureEndAt`，未见「闭合」误用），**非逐句语义核**。
6. **未评估与 `CertGatedExit.lean` 的「先查再接、不重复造」**：文件 :811-815 声称沿用其 typed-only 范式，我读了 CertGatedExit 头部确认该范式存在，但**没逐行比对**两者是否有可抽公共层的重复。
7. **探针只覆盖我点名的 4 条命题**，没做穷尽可达状态枚举 ⟹「其余臂无反例」**我不作证**。
8. **未核 `docs/agents/generation-constitution.md` 与 `stat-provenance.md` 全文**：本交付无统计量、无数据来源声明，`stat-provenance` 面判为不触发；`generation-constitution` 我只用了容器两态那条（与本票无关，未开新工位）。这两份的其余条款**未逐条比对**。

---

## 七、评审独立性披露（重要）

**本报告不是零先验的独立第二意见。**

按 #673 票面纪律执行 `gh issue view 673 --comments` 时，读到了 **2026-07-29T21:17Z 已发布的前一轮影子评审结论全文摘要**（同票、同对象 `919eb47df5`、同为 opus 线，其报告已落 `/private/tmp/shadow-299-review-20260729.md`，21752 字节，mtime 17:16）。该内容在我开始核验前即进入上下文，无法回避。可分辨的部分：

- **本轮亲跑亲读**：§1 全部实测（`lake build` / `lake env lean` / 4 组 grep / fixture drift / 4 条探针）、以及所有 rust 对臂（我本轮逐段读了 `nest_lifecycle.rs:1305-1463`、`:1083-1213`、`:1040-1082`、`:1-60`，`ledger_kernel/mod.rs:140-161`、`:360-397`）。
- **本轮自有发现**（前轮结论摘要中我未见对应条目）：**M1**（rust→Lean 回指整项缺失）、**M3**（rust 两条吸收路径只对上一条）、**M4**（模型无身份键层，⑧ 措辞把缺口说小）、**L2**（⑥ 的模型侧命题 + rust 审计留痕落差）、**L3**（两条定理构造性同义反复）、**L6**（`state_trichotomy` 零信息量）。
- **与前轮同向但我独立复核并另给证据锚**：**M2**（我改用 `invalidatedAt` vs `lastAsOf` 口径 + 两条吸收路径重新论证，探针自写）、**M5**、**L1**、**L5**。
- **前轮报告文件我没有打开**（只见其 issue 评论摘要），**也没有覆盖它**——本报告写到新路径 `/tmp/shadow-299-review-20260729-run2.md`，`/tmp/shadow-299-review-20260729.md` 原封未动。

**给编排的建议**：本轮与前轮在判定（CONDITIONAL、无 H）与五条不变量实质承重成立上独立收敛，这一致性有信息量；但因上述污染，两轮**不构成完全独立的双人复核**。若需真正的第二独立意见，应派一条未读 #673 评论的线。
