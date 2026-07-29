# 影子评审：#603 链（档1 `fa912ba991` + 档2 回退 `0e0d011da2` + #618 小修包 `dc2b7dd48b`）

> 票据：#619（map #597 线）。角色：评审执行层（只读；本 session 未改仓内任何既有文件，只写本报告；
> 全程前台单线程，未派 Task/子代理/后台任务）。
> 基线：`kimi-nest-mainline-20260717` @ `dc2b7dd48b`（HEAD），对照点 `6ebe18eda1`（#613 收口）。
> 独立产物（仓外、不提交）：`/tmp/kimi-nest-target-619{,w}`（本 session 自建 target）、
> `/tmp/rev619-{20k,100k}.{stdout,stderr,dump,p116}`、`/tmp/rev619-20k.stripped`、`/tmp/rev619_nest.diff`、
> `/tmp/rev619-{old,new}.v`。所有读数均由本 session 自跑复现，未采信交付方 `/tmp` 产物的结论
> （交付方产物仅用于「回退态 ↔ 小修包态」的字段级对拍）。

## 0. 结论

**PASS WITH CONDITIONS。**

三个 commit 的**已交付事实全部独立复现，无一处读数造假、无凑数、无静默改写**：字节护栏六面
（stdout 20k/100k、lifecycle dump 2000/20k/100k、P116 20k/100k、m8 三窗 trades/tower_events）我自跑
逐字节/逐哈希核对全绿；覆盖率两口径（77.2% / 78.7%）我用自写探针从头复算逐位吻合；`ForceOvertake`
纠误两例的「谁被误计 / 前身终局 / 认领后终局」我从 dump 逐条追踪与报告表格逐字相同；档2 回退在
`.rs`/`.py`/`.toml` 全仓 grep 零残留，档1 保留面逐 hunk 核对无一行属档2；小修包「只加 `seg_a` 字段、
零行为变化」由「剥字段后与回退态 dump 逐字节相同」在三个窗口上全部成立（20k 那份是我自己剥的）。

**条件**来自两条**潜伏正确性缺陷**（均在保留下来的档1 代码里，本窗数据恰好没触发，故交付方无从发现）
与一条**#533 留痕纪律缺口**：

| # | 级别 | 一句话 |
|---|---|---|
| H1 | **HIGH** | `force_overtake_claimed_count` 用 `superseded_from` 反查认领关联，而该字段会被下一 bar 的 `Supersedes` 桥迁移覆盖 ⟹ 认领方只要多活一个 bar，纠误口径就静默丢数。dump 实证桥迁移是**逐 bar** 发生的。 |
| H2 | **MEDIUM** | `center_upgrade_match` 不看前身死活、取 `BTreeMap` 序首个；留痕形态下终态前身仍在册 ⟹ 第三次 B 升级时死/活两个前身同时匹配，选中谁由 `seg_c_full` 右端大小决定（与死活无关）。文档「同链上至多一只存活」把「存活」当「匹配」，论证不成立。 |
| M3 | **MEDIUM** | #533 golden 第三次重锚（小修包）在仓内**零说明**：commit message 只有一行主题无 body；它引用的 #618 报告 §6 明确自陈「未修改仓内任何既有文件」。前两次重锚各有仓内报告的重锚节，这次没有。 |

H1/H2 都只在**诊断/口径面**，不进判据与证书真值路径（已全仓 grep 自证），故不构成 FAIL；但 H1 污染的
正是票面验收 2 交付的那个数（`35 → 33`），M3 缺的正是护栏体系赖以成立的留痕。建议：H1/H2 各开修复票，
M3 补一份小修包交付报告入库（内容可直接取 `/tmp/dispatch-618-fixpack.out`，本报告 §2/§3 已把其全部
断言复核完毕）。

---

## 1. 票面七项逐条对照

| # | 核验项 | 判定 | 我的独立证据 |
|---|---|---|---|
| 1 | **档1 语义**：迁移/留痕两形态、前身已终态一个 bit 不动 | **PASS** | §2.1 |
| 1 | `force_overtake_claimed_count` 与 `force_overtake` 口径分列 | **PASS**（实现分列；但 H1 影响其取值可靠性） | §2.2 |
| 1 | 跨锚拒绝零实装 | **PASS** | §2.3 |
| 1 | 纠误两例（15984/71448）逐条 | **PASS** | §2.4 |
| 2 | 回退彻底性：档2 零残留 | **PASS** | §3.1 |
| 2 | 档1 保留精确（净增量单档声明） | **PASS**（行数措辞不精确，见 L8） | §3.2 |
| 2 | 复测回落（nonflash 226/88.0、claimed=2 逐位同原交付） | **PASS** | §3.3 |
| 3 | 小修包 seg_a 口径（HIT 实值 / MISS 恒 `none`，「不适用非遗漏」） | **PASS**（结论成立；措辞不精确，见 L6） | §4.1 |
| 3 | 归因脚本完整锚升级 + 3 只转桶（A 20→17） | **PASS** | §4.2 |
| 3 | 旧 dump 向后兼容 | **PASS** | §4.2 |
| 4 | 护栏链 stdout/P116/m8 cmp=0 抽 SHA | **PASS**（P116 面交付侧未复核，我补齐，见 L10） | §5 |
| 4 | #533 golden 重锚只动 dump 面、stdout 面零变化自证 | **PASS（事实）/ FAIL（留痕）** | §5.2 + **M3** |
| 5 | 测试门唯一红 #491 | **PASS** | §6 |
| 5 | 计时 flaky 登记口径 | **照实：仅报告层登记，无票、测试本体无注记** | §6 + L9 |
| 5 | 未提交面污染 | **PASS**（唯一未提交 `.rs` 是 `p100_cert_bsp_recon.rs` +2 行，`--lib`/`--bin p123` 均不编译它） | §6 |
| 6 | impl 报告订正注记在位 | **PASS** | §7 |
| 6 | revert 报告 §6-1(b) 同述误诊段未注 | **确认未注**（照实登记，留编排侧） | §7 + L5 |
| 7 | 永禁清单逐条 + #523/#449 方向 | **PASS** | §8 |

---

## 2. 档1 语义（Spec 轴）

### 2.1 两形态与「前身一个 bit 不动」

`nest_lifecycle.rs:962-1030`（`advance` 第 1 步 `None` 分支）读码核实，两形态如报告所述：

- 前身 `Provisional` ∧ 非倒退 ⟹ **迁移**：`entries.remove(old_key)` → `superseded_from = old_key` →
  `key = new` → `push_revision(CenterUpgraded{from})` → `insert`。五钟随 entry 整体搬走，不重写。
- 否则（**已终态** / 倒退喂入）⟹ **留痕**：新 entry 独立建仓（`Observed`），紧跟一条
  `CenterUpgraded{from}`；**对前身 entry 无任何写操作**——该分支代码路径里根本不持有 `old` 的可变引用。

dump 逐条验证（`/tmp/rev619-100k.dump`，我自跑重放产出）：

| 例 | 前身末条修订 | 认领方修订链 | 前身在认领后是否再被写 |
|---|---|---|---|
| 15984 | `Invalidated{ForceOvertake}` @ 15857（行 21517） | `Observed`→`CenterUpgraded`→`FirstProvable`→`StructureCompleted`→`Confirmed`，**全部 @15984** | **否**（`seg_c_full=(15738,15857) b=14193` 的 REV 行末条即 15857） |
| 71448 | `Invalidated{ForceOvertake}` @ 71403（行 95280） | `Observed`→`CenterUpgraded`→`StructureCompleted`→`Invalidated{NeverConstituted}`，全部 @71448 | **否** |

⟹ 「终态吸收 / 禁复活 / 终态钟只写一次」三条不变量在 E2E §1:83 冲突点上未被破坏。报告 §2.2 把
「不合并」的理由讲成教义必然（合并 = 复活）而非工程折中，与代码一致，**接受**。

顺带证实报告 §3 那句诚实标注属实：认领方 15984/71448 的 `observed_at` **仍是完成时刻**，钟没有被继承
——它们算 covered 靠的是沿认领链上溯前身的 `Observed`，两者不可互相冒充。这一句不是免责话术，是
dump 事实。

### 2.2 口径分列

`LifecycleSettlementStats` 新增独立字段 `force_overtake_claimed_count`（`nest_lifecycle.rs:736`），
累加点在 `InvalidatedReason::ForceOvertake` 分支内部（`:845`）⟹ 结构上保证 `claimed ≤ force_overtake`，
不会出现负的纠误数。dump 行两个字段分列（`force_overtake=40 force_overtake_claimed=2`）。**分列本身
PASS**；取值可靠性见 H1。

### 2.3 跨锚零实装

`bridge_by_center_upgrade`（`:211-217`）把 `old.seg_a == new.seg_a` 写死为合取项，函数体六行、无分支。
全仓 grep 该符号只有 11 处引用：定义 1、文档 3、`assert_invariants` 1、`center_upgrade_match` 1、
测试 5——**不存在任何放开 `seg_a` 的路径**。`issue603_center_upgrade_rejects_cross_anchor_and_backward_center`
正面锁定跨锚 / B 反向 / C 左端不同三种拒绝 + 与 `bridge_identity` 互斥 + 端到端零认领修订。**PASS**。

### 2.4 纠误两例逐条

我从 dump 复算（非采信报告表格）：

| # | 完成信号 | 认领方 | 被误计的前身 | 前身终局 | 认领后终局 | 形态 |
|---|---|---|---|---|---|---|
| 1 | `as_of=15984` | `seg_a=(15191,15441)` `seg_c_full=(15738,15821)` `b=14709` | 同锚同 C 左端、`seg_c_full=(15738,15857)`、`b=14193`；`Observed@15789`→`FirstProvable@15789`→（逐 bar `Supersedes`）→`ForceOvertake@15857` | **ForceOvertake** | **Confirmed@15984** | 留痕 |
| 2 | `as_of=71448` | `seg_a=(71236,71266)` `seg_c_full=(71341,71416)` `b=71033` | 同锚同 C 左端、`seg_c_full=(71341,71403)`、`b=70396`；`Observed@71367`→`FirstProvable@71367`→（逐 bar `Supersedes`）→`ForceOvertake@71403` | **ForceOvertake** | **NeverConstituted@71448** | 留痕 |
| 3 | `as_of=66981` | `seg_a=(66433,66646)` `c=66897` `b=66077` | 同锚、`b=65627`，认领时仍 `Provisional` | 无终局记录（被迁移走） | **Confirmed@66981** | 迁移 |

与报告 §2.3 逐字相同。`force_overtake_claimed=2`（第 3 条走迁移、前身已不在册，不计入）——口径自洽。

---

## 3. 回退彻底性（Spec 轴）

### 3.1 档2 零残留

```
grep -rn "ActiveFrontierQueue|ACTIVE_FRONTIER_QUEUE_DEPTH|frontier_queue|stale_candidate_redivided
        |cand_age|frozen_end|CandidateScan|scan_l1_queue_candidates|scan_runs_for_candidate"
  --include=*.rs --include=*.py --include=*.toml .
→ 零命中
```

`no-patch-mentality` 意义上的**物理删除**（非「留个开关关掉」的兼容性垫片），与报告 §7-5 自称一致。
运行期旁证：我自跑的 100k `P527_L1_LIVE_OUTCOMES` 无 `stale_candidate_redivided` 码，`l1:window=610`
（档2 期为 1008）已回落基线。

### 3.2 档1 保留精确

`git diff --numstat 6ebe18eda1 HEAD` 代码面只有两文件：
`nest_lifecycle.rs +352 / −30`、`p123_fast_replay.rs +17 / −2`（其中 +2/−1 属回退期的
`force_overtake_claimed` 字段，+15/−1 属小修包的 `seg_a`）。

`/tmp/rev619_nest.diff` 全量逐 hunk 核对，10 个 hunk 分别是：`bridge_by_center_upgrade` 判据、
`CenterUpgraded` 枚举、`superseded_from` 文档、`force_overtake_claimed_count` 字段、`claimed` 集合、
累加点、`advance` `None` 分支重写、`assert_invariants` 双码断言、`center_upgrade_match`、3 个测试。
**无一 hunk 属档2**；−30 行即被 `match` 取代的原建仓块。报告 §2「档1 逐位保留」成立。

### 3.3 复测回落

我自跑 `p123_fast_replay`（HEAD、release、BTC 100k）的 `P421_LIFETIME_SUMMARY`：

```
entries=301 first_provable=207 provisional=1 confirmed=135 force_overtake=40
force_overtake_claimed=2 never_constituted=74 identity_vanished_refuted=28
identity_vanished_seam=23 flash_terminal=74 nonflash_count=226 nonflash_median=Some(88.0)
force_lifetime_median=Some(64.5)
```

与回退报告 §3「档1-only」列**逐位相同**；`nonflash 226/88.0` 精确回落 `6ebe18eda1` 基线，
`claimed=2` 与档2 期原交付逐位相同。L2 面 `l2:window=342 / center_not_consolidation=329 /
lower_frontier_not_absorbed=558 / structure_not_locatable=467 / tower_level_absent=520` 逐项与
#601/#613 读数相同 ⟹ **L2 零回归**成立。

覆盖率我用**自写探针**（不复用交付方脚本）复算两口径：

```
L1=202  身份键口径 covered=156 (77.2%)   候选口径 covered=159 (78.7%)
候选口径新增覆盖： [(15984, 15789), (66981, 66923), (71448, 71367)]
```

⟹ 78.7% 与三条来源逐位坐实。**顺带补上交付方自陈的缺口**：回退报告 §6-2 明确写了「未重新跑候选
口径脚本」，把 78.7% 作为派生量推断；本 session 已实测，**推断为真**，该遗留可以结项。

---

## 4. 小修包（Spec 轴）

### 4.1 `seg_a` 口径：「不适用非遗漏」成立

判据是**穷尽** `PanLiveOutcome` 的六个变体在 `provide_active_pan_live_windows`（`:1634-1711`）中的
return 位置：

| 变体 | return 行 | 此时 `structure` 是否存在 |
|---|---|---|
| `FrontierNotAfterConfirmed` | :1667 | 否（`locate_*` 尚未调用） |
| `FrontierAheadOfClock` | :1670 | 否（同上） |
| `NoConfirmedCenterBefore` | :1680 | 否（同上） |
| `CenterNotConsolidation` | :1683 | 否（同上） |
| `StructureNotLocatable` | :1701 | 否（窄锚 + A′ 回退双失败） |
| `Window(w)` | :1703 | 是 ⟹ `w.seg_a` |

`seg_a` 只在 `structure` 里产出 ⟹ 五个 MISS 码下**确实无值可记**，`None` 是「概念不适用」而非「取到了
不记」。调用侧（`p123_fast_replay.rs:1849-1869`）`Some(window) => seg_a: Some(window.seg_a)` /
`None => seg_a: None`，以及五处早退行（`:1687/1706/1734/1749/1765`，`tower_level_absent` 等）恒 `None`
——**与判据一致，PASS**。

字段文档措辞不精确（对前四个码 `locate_*` 是**从未被调用**，不是「未产出」）——见 L6。

### 4.2 归因脚本升级 + 3 只转桶 + 向后兼容

我在**两份 dump** 上自跑 `chanlun/review-results/issue613-attrib-buckets.py`：

| dump | A 桶 | structure_not_locatable | lower_frontier_not_absorbed | 合计 |
|---|---:|---:|---:|---:|
| 回退态（无 `seg_a=` 字段，粗键回退路径） | **20** | 8 | 1 | 38 |
| HEAD 态（含 `seg_a=`，完整锚） | **17** | 9 | 3 | 38 |

- **A 20→17 复现**，L2 完成身份 38 只不破。
- 逐项 diff 三只转桶，与派发回执点名的三只逐字相同：
  `('Short',16724,17018,17399,17581,15191) → structure_not_locatable`、
  `('Short',64944,65096,65516,65618,63188) → lower_frontier_not_absorbed`、
  `('Short',70396,71033,71812,72223,71033) → lower_frontier_not_absorbed`。
- **向后兼容成立**：旧 dump 正常解析、退化为粗键、给出升级前的 20，不报错不拒绝。开关
  `seg_a_field_present = any(key[2] is not None for key in hits)` 在 HIT 行恒携带 `seg_a`（HIT ⟹ `Window`
  ⟹ `Some`）这一事实下取值正确。
- 归因闭合两侧都是 `PASS（零「不知道」）`。

**「只加字段、零行为变化」的字节自证**（三窗全覆盖）：

| 窗 | 剥掉 ` seg_a=…` 后 vs 回退态 dump |
|---|---|
| 2000 | 逐字节相同（交付方 `.stripped`） |
| 20000 | 逐字节相同（**我自己用 `sed` 剥的**，未采信交付方产物） |
| 100000 | 逐字节相同（交付方 `.stripped`） |

---

## 5. 护栏链（Spec 轴）

### 5.1 我自跑的六面

| 面 | 窗 | 结果 |
|---|---|---|
| p123 stdout | 20k | `bd9ac1d655f9d615…` = fixture `stdout` 行 = #527/#601/#613 记录 |
| p123 stdout | 100k | `d8b69c180c23c5e3…` = 同上 |
| p123 lifecycle dump | 20k | `8ee16b36d5f22467…` = fixture `dump` 行 |
| p123 lifecycle dump | 100k | `d46f78e236f4dbf9…` = fixture `dump` 行 |
| p123 lifecycle dump | 2000 | 与仓内 golden 全文逐字节相同 |
| **p123 P116 dump** | 20k / 100k | **与 `6ebe18eda1` 基线产物逐字节相同**（`cmp` 两窗全绿） |
| m8 `p3fold/wf7/wf8` trades + tower_events | — | `cargo test --release --lib …m8_byte_guardrail -- --ignored` **1 passed** |
| #533 门 | 2000 + 20k/100k | `cargo test --release --test issue533_p123_byte_guardrail -- --ignored` **2 passed / 0 failed** |

P116 面交付两侧（回退、小修包）都没列、没跑（见 L10）；我补跑，**实质无缺口**。

### 5.2 golden 三次重锚的性质核对

| commit | 2000 golden | 20k/100k `dump` 行 | 20k/100k `stdout` 行 |
|---|---|---|---|
| `fa912ba991`（档1+档2） | 重锚（档2 诊断字段） | 重锚 | **一个字符未改** |
| `0e0d011da2`（回退） | **与 `6ebe18eda1` 逐字节相同**（我 `diff` 核） | 重锚（档1 独立贡献） | **一个字符未改** |
| `dc2b7dd48b`（小修包） | 重锚；我把 diff 的每一行剥掉 ` seg_a=…` 后**逐行成对抵消，零非配对行** | 重锚 | **一个字符未改** |

⟹ 「只动 dump 面、stdout 面零漂移由 golden 自身自证」在三次重锚上**全部成立**（我逐 commit 取
fixture 首行核对，四个 commit 的两个 `stdout` 哈希完全一致）。

**但**：`rust/tests/issue533_p123_byte_guardrail.rs:44-47` 的纪律要求「更新须在**同一 PR 里说明原因**
（引用相应 issue/report）」。第三次重锚不满足——见 **M3**。

---

## 6. 测试门（Spec 轴）

我自跑（`CARGO_TARGET_DIR=/tmp/kimi-nest-target-619`，release）：

| 命令 | 结果 |
|---|---|
| `cargo test --release --lib` | **2044 passed / 1 failed / 137 ignored**，唯一红 = `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`（**#491**，既线） |
| `cargo test --release --bin p123_fast_replay` | **3 passed / 0 failed**（档2 专属 `lifecycle_window_stem_freezes_right_edge_for_stale_candidate` 已随机制删除） |
| `cargo test --release --test issue533_p123_byte_guardrail -- --ignored` | 2 passed |
| `cargo test --release --lib …m8_byte_guardrail -- --ignored` | 1 passed |
| `cargo build --release --lib --bin p123_fast_replay`（全新 target） | **38 条警告，`nest_lifecycle.rs` / `p123_fast_replay.rs` 零命中** —— 与回退报告 §5 的「38 条 / 两目标文件本身零警告」逐数吻合 |

数目自洽：档2 期 2045（含 `issue603_frontier_queue_depth_behaviour`）→ 回退删 1 → 2044。

- **计时 flaky**：`incremental_tower_scaling_dominates_full_synthetic` 本轮**绿**（未复现）。其
  flaky 性质在仓内可查：`.chanlun/review-results/frontier-had-emitted-window-20260702.md:52` 已定性为
  「含时间比断言 `ratio_at_max < 0.5`，重载机器下偶发 flake，该测试固有属性」，另有
  `shadow-review-389-20260727.md:111`、`treasury-reverify-20260727.md:504` 两次登记。**登记口径照实**：
  只在评审报告层，测试本体无注记、tracker 无票（见 L9）。
- **未提交面污染**：`git status --porcelain -- rust/` 唯一命中 `rust/src/bin/p100_cert_bsp_recon.rs`
  （+2 行，他工位）。`--lib` 与 `--bin p123_fast_replay` 都不编译该 bin ⟹ **本轮读数无污染**。

---

## 7. 文档链（Spec 轴，照实）

- **impl 报告订正注记：在位**。`issue603-bridge-relaxation-impl-20260728.md:3-10` 文首注记锚 #618
  Resolution，点名 §0 表格 17 只与 §7 遗留 2(b) 两处，并明写「下文正文保留原表述、不做静默改写」
  ——与 `no-patch-mentality`「不静默改写历史」一致，**做法正确**。
- **revert 报告 §6-1(b)：确认未注**。`issue603-tier2-revert-20260728.md:133` 仍写「活窗与完成两路径
  centers 投影不同源（17 只，与 #591/#592 根因同族）」，全文无 #618 订正指针。**照实登记，留编排侧**。
- **新发现 M4**：impl 报告的订正注记**只覆盖 #618 误诊，只字未提档2 已被 `0e0d011da2` 回退**。该文件
  §1（档2 语义/深度选择）、§4（entries 304 / vanish_refuted 31 / nonflash 229/95.0）、§5（dump SHA
  `8d7ad3fc…`）、§8-4（下游推论四读数）仍以「档2 在案」的口径发布，且这些数**现在全部作废**。单读
  该文件的下游会取到已作废的读数——注记已经在改了，把这一条一并写上是零成本的。

---

## 8. 永禁清单 / #523 / #449 方向（Spec 轴）

| 条款 | 出处 | 判定 |
|---|---|---|
| L1 frontier 只能由 parser `tail`（`OpenTail.pendingSegment`）+ 未确认笔构造，**禁从 confirmed segments 回放重建** | `nest_lifecycle.rs:1497-1499` | **未触**。档1 不构造任何 frontier（它只比对 `LifecycleKey`）；档2（唯一涉及槽快照留存的机制）已整体删除。 |
| L2 行进中单元只能由 `tower[0]` confirmed + 虚拟追加单元重跑窗口扫描派生，**禁把 `tower[1]` 已产出窗口当行进中单元** | `nest_lifecycle.rs:1761-1766` | **未触**（回退把 `p123_fast_replay.rs` 整体退回 #613 基线，L2 派生逻辑一行未动）。 |
| 禁用 `as_of` 冒充结构点 | `nest_lifecycle.rs:1514` | **未触**（`frozen_end` 随档2 移除；档1 不碰右端——`bridge_by_center_upgrade` 显式不比 C 右端）。 |
| **#449 方向**：nest 产物禁回灌判据 crate / 证书真值路径 | issue #449（CLOSED） | **PASS**。全仓 grep：`force_overtake_claimed_count` 唯一消费点是 `p123_fast_replay.rs:743/749` 的 `eprintln!`（stderr 诊断）；`CenterUpgraded` 只出现在 `nest_lifecycle.rs` 内部与其测试。新 API 面零外溢。 |
| **不可变性**（`coding-style` Immutability） | `.claude/rules/common/coding-style.md` + 2026-07-28 补注 | **适用例外**。`NestLifecycleBook` 正是补注点名的「带修订留痕的单线程状态机/账本」；档1 的 `remove`/`insert`/`push_revision` 在该例外内，且「前身一个 bit 不动」正是补注所保护的「禁静默改写历史」。 |

---

## 9. 新发现（分级 + 锚）

### H1 · **HIGH** · `force_overtake_claimed_count` 的认领关联会被下一 bar 的桥迁移静默覆盖

**锚**：`rust/src/theta_v0/classifier/nest_lifecycle.rs:816-826`（`claimed` 集合构造）
+ `:952`/`:978`/`:1017`（三处 `superseded_from` 写入点）。

`claimed` 集合这样求：

```rust
.filter(|entry| entry.revisions.iter().any(|r| matches!(r.kind, CenterUpgraded { .. })))
.filter_map(|entry| entry.superseded_from)          // ← 问题在这里
```

它假设「有 `CenterUpgraded` 修订的 entry，其 `superseded_from` 就指向被认领的前身」。但
`superseded_from` 是**单个 `Option`，被两个码共用**，且**每次桥迁移都会整体覆盖**（`:952`
`entry.superseded_from = Some(old_key)`）。

**失效场景（具体、可复现的路径）**：留痕认领产出的新身份是 `Provisional`；下一 bar 活窗把 C 右端
延展一格 ⟹ 新 key 与旧 key 只差 `seg_c_full.1` ⟹ `bridge_match` 命中 ⟹ 走 `Supersedes` 迁移 ⟹
`superseded_from` 被改写成「上一 bar 的自己」，指向终态前身的那条唯一可审计关联**从此丢失** ⟹
`claimed` 集合收不到该前身的键 ⟹ `force_overtake_claimed_count` 少计 ⟹ 「35 − claimed = 33」这个
纠误口径给出偏大的反超数。

**这不是理论构造**——dump 实证桥迁移就是**逐 bar** 发生的：`/tmp/rev619-100k.dump` 行 21376–21516，
同一身份从 `as_of=15790` 到 `15857` **每一个 bar 一条 `Supersedes`**（71368–71403 同构）。本窗之所以
没炸，纯属巧合：两例留痕认领的认领方都在**认领的同一 bar** 就走到终局（15984 `Confirmed`、
71448 `NeverConstituted`），一个 bar 都没多活，来不及被桥迁移。换一段数据、或认领方晚一 bar 终结，
这个数就是错的。

**修法（严格形式，非补丁）**：`CenterUpgraded { from }` 修订**自己就带 `from`**，不需要经
`superseded_from` 中转：

```rust
.filter_map(|entry| entry.revisions.iter().find_map(|r| match r.kind {
    LifecycleRevisionKind::CenterUpgraded { from } => Some(from),
    _ => None,
}))
```

修订链是 append-only、永不覆盖，语义上也更对——认领是**事件**，`superseded_from` 是**当下来源指针**，
本就不该拿后者承载前者。迁移形态下 `from` 已被 `remove`、不在 `entries` 里 ⟹ 仍然自动不计入
（`claimed.contains(&entry.key)` 恒 false），与现口径行为一致。

**连带**：`nest_lifecycle.rs:401-405` 那句新加的文档「身份来源链留痕（两码，**逐条对应本 entry 首个非
`Observed` 修订的 kind**）」在同一场景下也不成立——被覆盖后它对应的是**最后一次**迁移。属 090
声明膨胀，随本条一并订正。

---

### H2 · **MEDIUM** · `center_upgrade_match` 不看死活，多候选时按 `seg_c_full` 右端大小盲选

**锚**：`rust/src/theta_v0/classifier/nest_lifecycle.rs:1405-1416`。

```rust
fn center_upgrade_match(&self, key: &LifecycleKey) -> Option<LifecycleKey> {
    self.entries.keys().find(|old| bridge_by_center_upgrade(old, key)).copied()
}
```

文档给的正当性是：「多候选时取 `BTreeMap` 序首个（确定性，无平局歧义）——**同链上至多一只存活**：
迁移形态会 remove 前身，认领留痕形态下前身已终态」。

**该论证不成立**：`find` 匹配的是「所有满足判据的 entry」，不是「所有存活的 entry」。留痕形态**恰恰
把终态前身留在册**（这是它的设计要点），所以留痕之后 book 里同链同时有：终态前身 P1（b 较小）+
新建的 `Provisional` P2（b 居中）。第三次 B 升级到达时，**两个都匹配**。选中谁由 `LifecycleKey`
的 `sort_tuple`（`:134-143`）决定，其排序键在 `b_center_start` **之前**先比 `seg_c_full` ——而
`seg_c_full.1` 是随 `as_of` 漂的右端，**与前身死活完全无关**。

实例佐证这不是小概率：15984 那条链里 P1 右端 15857 > 认领方右端 15821 ⟹ 排序把**认领方**排在前；
换一组右端关系就会把**终态者**排在前。

**后果**：若盲选到终态前身，`migratable` 为 false ⟹ 本该走迁移的那只**存活**前身不被迁移 ⟹ 五钟不继承、
该 entry 沦为孤儿 ⟹ 后续走 `IdentityVanished`。这正是回退报告 §3 用来解释「entries −1 /
identity_vanished_refuted −1」的「迁移合并孤儿记录」机制被反向消掉。

**未在 BTC 100k 触发**：全窗只有 3 条认领，链长都是 1（我 grep `CenterUpgraded` 全 dump 共 3 行）。
属**潜伏**缺陷。

**修法**：匹配时按语义择优而非按字节序盲取——优先返回可迁移（`Provisional` ∧ 非倒退）的那只；
无可迁移者再退回终态者做留痕。同时把文档里「至多一只存活」改成能站住的表述（「至多一只**可迁移**，
但可**匹配**的可以有多只」）。

---

### M3 · **MEDIUM** · #533 golden 第三次重锚在仓内零说明

**锚**：commit `dc2b7dd48b`（message 只有一行主题、无 body）；纪律条文
`rust/tests/issue533_p123_byte_guardrail.rs:44-47`。

该 commit 重写了 `issue533_p123_2000_dump.golden.txt` 全文（873 行）并改了两个 `.sha256` 的 `dump` 行。
纪律要求「更新须在同一 PR 里说明原因（引用相应 issue/report）」。现状：

- commit message 无 body，无重锚说明；
- 它引用的 #618 报告 `issue618-center-projection-divergence-20260728.md:246-249` **§6 影响声明明确自陈**
  「本报告为只读研究产出，**未修改仓内任何既有文件**……未跑 `cargo build`/`cargo test`」——即被引用的
  报告主动否认自己描述了任何改动；
- 前两次重锚各有仓内报告的专门章节（impl §5、revert §1「golden fixture」节）；这次没有。

实质我已代为核验（§4.2/§5.2：纯 `seg_a` 字段追加、剥字段后三窗逐字节相同、stdout 面零变化），**重锚
本身无误**；缺的是留痕。缺留痕的代价是：下一个人 `git blame` 到这次 golden 变更时，仓内没有任何东西
告诉他为什么、以及「已审阅、故意」这个前提被谁在何处满足过。补一份小修包交付报告即可闭合。

---

### M4 · **MEDIUM** · impl 报告的订正注记未提档2 已回退，文件仍在发布作废读数

**锚**：`chanlun/review-results/issue603-bridge-relaxation-impl-20260728.md:3-10`（注记）
vs 同文件 §1 / §4 / §5 / §8-4。详见 §7。

---

### L5 · LOW · revert 报告 §6-1(b) 同述误诊段未注

**锚**：`issue603-tier2-revert-20260728.md:133`。票面已知项，照实登记，留编排侧。

### L6 · LOW · 诊断行 `None` 编码与 `seg_a` 渲染两处不一致

**锚**：`p123_fast_replay.rs:1170-1177`。同一行里 `b_center_start`/`c_start`/`gap_len` 的空值用
`18446744073709551615` 哨兵，新增的 `seg_a` 用字符串 `none` ⟹ 一行内两套空值约定。另外 diag 行把
`seg_a` 渲染成 `(15191,15441)`（无空格），而同一份 dump 的 `COMPLETION_SIGNAL`/`REV` 行用 Rust
`Debug` 渲染成 `(15191, 15441)`（有空格）⟹ 下游若想用一条正则吃 `seg_a`，得同时兼容两种写法
（`issue613-attrib-buckets.py` 现在正是维护着两条不同的正则）。
另：`:553-556` 的字段文档说 MISS 时「`locate_pan_div_structure`/`_front_anchor` 均未产出」——对
`FrontierNotAfterConfirmed`/`FrontierAheadOfClock`/`NoConfirmedCenterBefore`/`CenterNotConsolidation`
四码，这两个函数是**从未被调用**，不是「调用了没产出」。结论（`seg_a` 概念不适用）成立，措辞该收紧。

### L7 · LOW · `seg_a` 渲染每行至少一次堆分配，且 dump 未开启时照样分配

**锚**：`p123_fast_replay.rs:1177`
`row.seg_a.map_or("none".to_string(), |(a, b)| format!("({a},{b})"))`。
`map_or` 的两个分支都在调用前求值 ⟹ 每条 diag 行至少一次 `String` 分配；而
`write_lifecycle_line`（`:1594-1605`）是**进函数之后**才判 `sink` 为 `None` 短路的 ⟹ 没开
`P421_LIFECYCLE_DUMP` 时这些分配照做。100k 窗 diag 行是 10⁵–10⁶ 量级，且这是回放热路径。
其余字段全部用 `usize::MAX` 哨兵、零分配——本字段是唯一破例。
另，闭包参数命名 `|(a, b)|` 里的 `b` 与同一格式串里的 `b_center_start`（域内的「B 中枢」）撞义，读起来
歧义；`|(a_start, a_end)|` 更贴域语言。

### L8 · LOW · 「382 行 / 3 行」把 `--stat` 的变更行数说成「新增 / 净增量」

**锚**：`issue603-tier2-revert-20260728.md:12-14`、`:67`、`:145`。实测
`nest_lifecycle.rs` 为 **+352 / −30**、`p123_fast_replay.rs` 为 **+2 / −1**。382 与 3 是
`git diff --stat` 的合计变更行。结论（保留面精确）不受影响，但「新增 382 行」在 090 意义上是不精确
表述——有 30 行既有代码被替换掉了，这一点在「档1 一个 bit 未动」的语境里值得说准。

### L9 · LOW · 计时 flaky 的登记口径与 #491 不对称

**锚**：`rust/src/theta_v0/classifier/mod.rs:3717-3722`。测试本体的文档注释只讲标度语义，**无任何
时间敏感/flaky 注记**；tracker 无对应票（`gh issue list --search` 零命中）。登记全靠散落在 4 份评审
报告里的自然语言。相较之下唯一红 `#491` 有票号可挂。建议二选一：给测试加注记，或补一张票。

### L10 · LOW · 回退与小修包两侧的护栏表都漏了 P116 面

**锚**：`issue603-tier2-revert-20260728.md:103-110`（表内无 P116 行）；小修包无仓内报告。
impl 报告 §5 是列了 P116 的。`P116_DUMP` 是 `p123_fast_replay.rs:199` 的独立产物面，**不在 #533
护栏测试覆盖范围内**（该测试只 check stdout + `P421_LIFECYCLE_DUMP`）⟹ 漏列 = 真的没人看。
我已补跑两窗，与 `6ebe18eda1` 基线逐字节相同，**实质无缺口**；建议把 P116 行固定进护栏表模板，
或直接纳入 #533 测试。

### L11 · LOW（既有违规加深）· 体量与嵌套

`nest_lifecycle.rs` **5005 行**（仓内标准 `.claude/rules/common/coding-style.md`：800 上限）；
`advance()` 约 **265 行**（标准：<50 行）；档1 新增的 `match (claim, migratable)` 使
`for → if → match → arm → match → arm → if let` 达 **5–6 层**（标准：≤4 层）。三项都是既有违规的
加深而非本次引入，但本次是**在明知超标的位置继续加**。Fowler：Large Class / Long Function /
深条件嵌套。可低风险改善的切口：把 `advance` 第 1 步（建仓/桥/认领）整体提成一个方法。

### L12 · LOW（Fowler: Duplicated Code）· 两处迁移块逐句同构

**锚**：`nest_lifecycle.rs:951-960`（`Supersedes` 迁移）vs `:977-986`（`CenterUpgraded` 迁移）。
两块的语句序列完全一样（`remove` → 写 `superseded_from` → 改 `key` → `push_revision` → `insert` →
`delta.push`），只有 revision kind 一个 token 不同。提成
`fn migrate_entry(&mut self, old: LifecycleKey, new: LifecycleKey, kind: LifecycleRevisionKind, as_of: usize)`
可以消掉重复；而且**顺带把 H1 关掉**——把「写 `superseded_from`」与「写哪种修订」绑成同一个写入点后，
两者的一致性由类型而非纪律保证。

---

## 10. Standards 轴小结

| 维度 | 判定 |
|---|---|
| 命名 | `bridge_by_center_upgrade` / `center_upgrade_match` / `CenterUpgraded` / `force_overtake_claimed_count` 与既有 `bridge_identity` / `bridge_match` / `Supersedes` / `force_overtake_count` 命名族一致，见名知义。**PASS**（唯一 nit：L7 的闭包参数 `b`） |
| 文档 | 密度与仓内风格一致（判据必带教义依据 + 谱系锚）；两处措辞越界：L6（「未产出」vs「未调用」）、H1 连带（「首个非 Observed 修订」）。**PASS with nits** |
| 错误处理 | 新路径无 `unwrap` on 外部输入；两处 `expect` 都在紧邻的 `is_some_and`/`contains_key` 保护下（`:977` `expect("认领键存在")`、`:951` `expect("桥匹配键存在")`），属不变量断言而非吞错。**PASS** |
| 不变量 | `assert_invariants` 同步扩成双码（`bridge_identity ‖ bridge_by_center_upgrade`），新链纳入既有校验。**PASS** |
| 测试 | 3 个档1 测试覆盖迁移 / 留痕 / 拒绝三面，且留痕测试直接断言「前身条目逐位不动」（`assert_eq!(entry, &terminal_before)`）——用值相等钉住「一个 bit 不动」，写法到位。**缺口**：H1/H2 两个多步链场景（同锚 B 连升两次、认领后再桥迁移）无测试，正是它们至今没被发现的原因。 |
| schema 兼容性声明 | 「诊断只写不判、不进真值路径」如实（§8 grep 自证）；「旧 dump 向后兼容」如实（§4.2 实跑）。**PASS**，但仓内**唯一**的 dump 消费者只有 `issue613-attrib-buckets.py` 一个，`/tmp` 探针脚本不在仓内、无版本管理——schema 变更的下游面其实靠人记，这是结构性脆弱点（本次未出事）。 |
| 不可变性 | 见 §8，适用 2026-07-28 例外。**PASS** |
| 体量/嵌套 | **不达标**（L11，既有违规加深） |
| commit 纪律 | 回退与档1 两个 commit 的 message 详尽、归因清楚、`revert(...)` 类型用得对；小修包 message 只有一行主题，与它实际改动的面（含 golden 重锚）不相称（M3）。 |

---

## 11. 结果包六要素

1. **结论**：**PASS WITH CONDITIONS**。#603 链三个 commit 的全部交付事实经本 session 独立复现无误
   （字节护栏八项、测试门四项、覆盖率两口径、纠误两例逐条、转桶三只、档2 零残留、档1 保留精确）；
   条件为 H1（`force_overtake_claimed_count` 认领关联会被逐 bar 桥迁移覆盖，纠误口径潜伏失效）、
   H2（`center_upgrade_match` 盲选，留痕后多候选时可能不迁移存活前身）、M3（#533 第三次重锚仓内零
   说明），另 M4 + L5–L12 共 9 条。H1/H2 均只在诊断/口径面，不进判据与证书真值路径，故不构成 FAIL。
2. **定义依据**：`nest_lifecycle.rs` `bridge_by_center_upgrade`（:211）/ `center_upgrade_match`（:1411）/
   `advance` 第 1 步（:930-1030）/ `settlement_stats` claimed 集合（:816）/ `assert_invariants`（:1305）/
   `LifecycleKey::sort_tuple`（:134）/ `provide_active_pan_live_windows` 六变体 return 位置（:1634-1711）/
   永禁清单（:1497、:1761、:1514）；`p123_fast_replay.rs` `L1LiveDiagRow.seg_a`（:548-554）/ dump 行
   （:1167-1180）/ `write_lifecycle_line`（:1594）；`issue533_p123_byte_guardrail.rs:44-47` golden 纪律；
   `.claude/rules/common/coding-style.md`（体量、不可变性 + 2026-07-28 补注）、`no-patch-mentality`（090
   声明膨胀）、`formalization-validity-domain`（L2 标注）、`result-package.md`；#523 永禁清单、#449 方向、
   #599 裁定、#603 comment-5111127257 裁定、#618 Resolution。
3. **边界条件**（本结论何时翻转）：(a) H1/H2 的判定基于代码路径推导 + dump 实证的**逐 bar 桥迁移事实**，
   若 `superseded_from` 的写入语义或 `bridge_identity` 的 C 右端放开面被改，两条的可达性需重判；
   (b) 本报告全部量化读数为 **L2 = BTC 100k 单窗真实数据**，不外推其它标的/窗口；
   (c) 「档2 零残留」是对 `*.rs`/`*.py`/`*.toml` 的 grep 结论，若档2 概念以别的符号名在别处复现，本判定
   不覆盖；(d) 测试门「唯一红 #491」在计时 flaky 复现时会变成两红——该 flaky 与本链无关（L9），但若
   将来它稳定变红，本判定需重跑确认；(e) 我未复核 `formal/` 面（本链未触及 `formal/`，未触发 fixture
   漂移 gate）。
4. **下游推论**：(a) 当前真值 = 回退报告 §3 的档1-only 列（`entries=301`、`first_provable=207`、
   `identity_vanished_refuted=28`、`flash_terminal=74`、`nonflash 226/88.0`、`force_overtake=40`、
   `force_overtake_claimed=2`），我已逐位复现；impl 报告 §4/§8-4 发布的 304/31/229/95.0 **作废**；
   (b) 覆盖率 78.7%（候选口径）**已由本 session 实测坐实**，回退报告 §6-2 的「未跑候选口径脚本」遗留
   可结项；(c) H1 未修之前，任何引用「纠误后反超数 = force_overtake − claimed」的下游必须同时确认
   认领方是否在认领 bar 即终结——否则该差值偏大；(d) 小修包后 dump schema 多一列 `seg_a`，仓内唯一
   消费者 `issue613-attrib-buckets.py` 已兼容，仓外 `/tmp` 探针（`wt603_final.py` 等）解析仍可用
   （正则未锚行尾）。
5. **谱系引用**：#599（档位裁定与量化来源）；#603（本链主票，含 comment-5111127257 回退裁定）；
   #613/#601（L2 活窗与整窗截断口径，本链保其零回归，我已复核 5 个 L2 读数逐项不变）；#618（误诊
   订正与小修包来源）；#591/#592（两路径时机不对齐的同族刻画）；#523（PanLive provider 永禁清单）；
   #533（字节护栏门与 golden 变更纪律——**M3 即对该纪律的偏离**）；#491（既线唯一红）；#449（判据层
   禁引 nest 产物，本链新 API 面未越界）；`no-patch-mentality`（回退取物理删除而非开关垫片，**正面
   评价**；H1 的建议修法亦按「严格形式」而非补丁给出）；`formalization-validity-domain`（全部读数标
   **L2 = BTC 100k 单窗**，不外推）；`coding-style` 2026-07-28 补注（账本状态机的不可变性例外）。
6. **影响声明**：本 session **未修改仓内任何既有文件**，只新增本报告
   `chanlun/review-results/shadow-603-review-20260728.md`。跑过的写操作全部落在仓外：
   `/tmp/kimi-nest-target-619`、`/tmp/kimi-nest-target-619w`（本 session 自建 target，未碰他工位的
   `-603`/`-603rv`/`-618fix`）、`/tmp/rev619-*`。未 `git add`、未 commit、未改 issue/map/roster、未关票。
   他工位未提交面（`chanlun/agent-roster-*`、`issue571-*`、`treasury-reverify-*`、
   `rust/src/bin/p100_cert_bsp_recon.rs`）未碰、未 stage。未触 `formal/`，未触发 fixture 漂移 gate。
