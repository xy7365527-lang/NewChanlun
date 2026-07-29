# #430 复审：返工后翻转核验（两轴，新上下文，禁自评）

**结论：FAIL**

原 FAIL 的“所有生产身份零寿命”核心数值命题已经翻转：100k 中带
`StructureCompleted` 的 229/229、20k 中 39/39，其
`StructureCompleted.as_of - Observed.as_of` 均大于 0。但这不是整票翻转：
返工把唯一真实 `ForceOvertake` 完成信号直接写成 `Invalidated` 后提前
`continue`，漏写契约要求的 `StructureCompleted` 修订。100k 的 230 个唯一完成信号
只有 229 条 `StructureCompleted`，20k 的 40 个只有 39 条；公共文档又声明该路径应先
写 `StructureCompleted`。这是完整生命史的实装与声明同时失配，Spec 轴仍有 HIGH。

Standards 轴另有独立 HIGH：全 diff 继续扩张多个超过仓内 800 行硬上限的文件，#454
只是债务票，不是规则例外。S-H1 已由 `5022e31df5` 明文裁定例外，本复审不计。

| 轴 | 结论 | 分级 |
|---|---|---|
| Spec（#427 转呈清单 + #421 SPEC + 本票逐条） | **FAIL** | HIGH×1 / MED×1 / LOW×1；未能判定×2 |
| Standards（仓内标准源 + Fowler 12 条） | **FAIL** | HIGH×1 / MED×3 / LOW×3 |
| 总结论 | **FAIL** | 两轴各自已有 HIGH，不跨轴平均或降级 |

## 一、范围、基线与只读边界

- 工位：`/tmp/kimi-nest-mainline`。
- 分支：`kimi-nest-mainline-20260717`。
- 评审 HEAD：`f6639a2a1bb5674ecd735a953ce964f4b205e6e9`。
- 基线：`a12a1022d9ddd8d1cae867a107a3a33c359358cf`。
- 全区间：8 commits、22 files、`+3086/-183`；`git diff --check
  a12a1022d9..f6639a2a1b` 为 exit 0。
- #421 直接提交为 `6e15ceffee`、`ce2038b779`、`f6639a2a1b`；区间还含
  `f838540eff`（#446）、`8d8895c652`（#512）、`ce074711d5`（#513）、
  `4b2b69afc6`（#514）以及例外裁定 `5022e31df5`。本报告对全 diff 做 Standards
  检查，但不把相邻票改动冒充 #421 的 Spec 证据。
- 开工时已有用户/并行态：`chanlun/agent-roster-2026-07-21.md`、
  `scripts/check_armR_trades_digest.py`、`scripts/tests/test_check_armR_trades_digest.py`
  为 modified，另有首轮两份 shadow 报告未跟踪；收口前两份 `scripts/` 修改自行消失。
  本轮未触碰这些文件，其外部变化来源与时点未能判定。
- 本报告首次落盘后的收口 `git status` 又新出现
  `rust/src/theta_v0/backtest/wverify_run.rs` modified 与同名未跟踪目录
  `rust/src/theta_v0/backtest/wverify_run/`。本轮未写二者；其来源未能判定。本文对该文件的
  Standards 结论固定读取 `f6639a2a1b` 提交态（2809 行），不读取并发脏态。
- 本轮除本报告外未改任何仓内文件；Cargo 仅使用票面指定的
  `CARGO_TARGET_DIR=/tmp/kimi-nest-target-check`。

## 二、核心发现翻转：数值成立，但完整修订链仍缺一环

独立解析以 `bridge_identity` 的真实身份字段为键：
`(level, side, kind, seg_a, seg_c_full.0, b_center_start)`，对应
`rust/src/theta_v0/classifier/nest_lifecycle.rs:141-148`。统计结果如下。

| 产物 | Observed | FirstProvable | StructureCompleted | 唯一完成信号 | 正寿命 / 可配对完成 | min / median / max |
|---|---:|---:|---:|---:|---:|---:|
| `/tmp/wt421r-post-100k-lifecycle.dump` | 248 | 149 | 229 | 230 | 229/229（100%） | 5 / 120 / 411 |
| `/tmp/wt421r-post-20k-lifecycle.dump` | 46 | 34 | 39 | 40 | 39/39（100%） | 5 / 96 / 324 |

结论拆开写：

1. **原零寿命核心发现已翻转。** 100k 的 229/229 与 20k 的 39/39 均为正差，
   零差=0、负差=0、缺 Observed=0。100k 最小样例见 dump `:590-594`
   （11098→11103），中位样例见 `:1805-1812`（73048→73168），最大样例见
   `:916-919`（27940→28351）。20k 最大样例见 `:488-492`
   （4479→4803），中位样例见 `:687-693`（17201→17297）。
2. **单位是 bar-index 差，不是 trigger 次数。** 上述数值直接相减的是 `as_of`；
   trigger 是稀疏重估点。自查档 `issue421-acceptance-selfcheck-20260727.md:148`
   把 5/120/411 写成 “trigger”，数值对、单位错。
3. 100k 的按级原始数为 Observed L1/L2/L3=`202/38/8`，
   StructureCompleted=`184/37/8`；20k 为 Observed L1/L2=`38/8`，
   StructureCompleted=`32/7`。这些是从逐行 dump 外算，不是
   `LifecycleReplayStats` 的现成按级指标；该结构只有全局累计字段，见
   `rust/src/bin/p123_fast_replay.rs:424-450`。
4. 100k 终局为 Confirmed=140、NeverConstituted=89、ForceOvertake=1、
   IdentityVanished=18；20k 为 29/10/1/5，另有一只尚未终结。两类真实否证原因
   `NeverConstituted` 与 `ForceOvertake` 均已发生，旧报告“真实数据无
   ForceOvertake”的发现已翻转。
5. 产物 SHA-256 与自查一致：20k
   `6a8877ee6549d50204407cf06dd466bd68874e4bf26b46d35468f699b4b80a52`；
   100k `d45201143c3d2171216849d5acdcfefbb2195c6a889d9f853282dcbde94c6c0e`。

### Spec-H1 — HIGH：唯一 ForceOvertake 完成路径漏写 StructureCompleted

100k/20k 两个截断口径均包含同一条真实路径：

- dump `:721-722`：身份在 `as_of=17600` 写
  `Observed → FirstProvable`；
- dump `:723`：`as_of=17902` 明确计
  `completion_signals=1 / channel_switches=1`；
- dump `:724-725`：只写 `Supersedes → Invalidated(ForceOvertake)`，没有
  `StructureCompleted`。若只以终态计寿命，该身份确实活了 302 个 bar index；问题不是
  又退化成零寿命，而是完成修订链缺一环。

代码原因可直接定位：

- `nest_lifecycle.rs:720-730` 在 `ForceCheck::Verified(false)` 且曾
  `first_provable` 时先 `Invalidated(ForceOvertake)`，随后 `continue`；
- 该 `continue` 跳过 `:761-769` 的 `StructureCompleted` 写入；
- 公共枚举文档 `:234-238` 却声明结构完成信号到达后，力度可验分支产生
  `StructureCompleted`，同 prefix 再结算为 Confirmed 或
  Invalidated(NeverConstituted/ForceOvertake)。

因此：

- 条件在“已有 `StructureCompleted` 的身份”上，229/229 与 39/39 都是真；
- 条件在“所有唯一完成信号”上，覆盖率实际为 229/230=99.565% 与
  39/40=97.5%；
- #421 User Story 12 要求每个假设可重建完整修订链，Testing Decision 2 要求通道切换时
  结构完成信号落账。真实 ForceOvertake 路径同时违反两项；
- 自查把 230 作为唯一完成分母，却用 229 作为生命史分母，恰好排除了唯一
  ForceOvertake。该缩窄没有在“P-H1 满足”结论中披露。

这不是统计尾差或仅文档问题，而是核心生命史在真实可达路径上缺修订，故判 HIGH。原
“36/36、213/213 出生即终结”的旧 HIGH 已消失；本 HIGH 是返工后独立发现，不沿用旧理由。

## 三、T8 矛盾复核

**判定：满足，旧 T8 矛盾已消。**

- 模块头 `nest_lifecycle.rs:54-58`：只在重估 trigger 投喂；非 trigger 间首次可证允许
  晚记到下一 trigger，不能早记；两钟均由首次实际 `advance` 写入。
- T8 `:1762-1766` 使用同一口径：跳过非 trigger prefix 只会晚记，不能构造
  `first_provable_at < observed_at`。
- 实装 `:713-716` 只会在当前 `advance(as_of)` 首写 `first_provable_at`；Observed
  也是首次实体进入时由同一 `advance` 写入。现文件文字与语义自洽。

## 四、#427 条款维持情况

| #427 验收条款 | 判定 | 证据锚 |
|---|---|---|
| feed 契约订正 | **部分满足** | 模块头 `nest_lifecycle.rs:54-58` 与 T8 `:1762-1766` 已统一，旧反向文字消失；但同模块公共修订契约 `:234-238` 与 ForceOvertake 实装 `:720-730` 仍不一致，不能扩大声明成“契约文本与实际行为全部对齐”。 |
| 重建报告收口为单一口径 | **部分满足** | `v3-lifecycle-rebuild-impl-20260724.md:89-96,99-119` 已把 trigger 契约、生产接线、release 核验和 #64 核销集中登记；但 `:107-109` 仍写 20k 的 `0/3715`，分母是旧 raw completion events 口径。当前 20k 唯一完成分母为 40，100k 为 230，故现行报告仍未收成单一口径。 |
| 左端恒等 release 断言 | **满足** | `level_view.rs:786-799` 在唯一汇合写入点使用无条件 `assert_eq!`，不受 debug 门控。 |
| 左端变异红 / 恢复绿 | **满足（产物核验）** | `/tmp/wt421-left-anchor-mutant-red.log:430-450` 在 release 下以 `left=121/right=120` 红；`/tmp/wt421-left-anchor-{restored,postmerge}-green.log:433-436` 为 1/1 绿。`level_view.rs` 在 `ce2038b779..f6639a2a1b` 未变化，故旧变异证据与返工不矛盾。 |
| 通道切换力度跃变注记 | **部分满足** | 比较窗/力度路径的跃变已有注记和合成测试；真实 ForceOvertake 也证明跃变可达。但该跃变分支跳过 StructureCompleted，见 Spec-H1，不能判行为闭环满足。 |
| #64 §2(d) 核销 | **满足** | 原授权在 `chanlun/escalate/r43-lifecycle-ruling-20260721.md:31-33`；现报告 `v3-lifecycle-rebuild-impl-20260724.md:110-116` 明确退役 `seg_c_full` 字段授权且不新增 `c_start_live`，左端由 `level_view.rs:790-799` 护栏。 |
| 订单流/既有输出无行为变化 | **满足（所给隔离对拍）** | 返工的 p123 两档与 m8 三窗共 10 对均 `cmp=0`；lifecycle dump 是本次修复对象，不属于“应不变”的既有输出。exact full-range 同数据集对拍见下述边界。 |
| 全库不新增失败 | **满足** | 本轮指定命令为 1992/1/135，唯一红与 #491 一致，详见第五节。 |

### 订单流字节护栏独立抽核

以下均重新执行 `cmp`，exit 0；SHA-256 的 pre/post 两侧一致：

| 面 | SHA-256 |
|---|---|
| p123 20k stdout | `bd9ac1d655f9d615a5d9b3495b3a92465fb1fae13ce5dadfa28033c47d375b6c` |
| p123 20k P116 replay | `fcc8016a9a01a1098736b9ee3b348614296bb97aec817a5c172c9a0723427a40` |
| p123 100k stdout | `d8b69c180c23c5e393bf3c330825d88ae9c989ff38f2b8865e049e1b5eb56da8` |
| p123 100k P116 replay | `8a7327feb3b9ba29f69fa824af1f1ecc137d9303637b47ad8465b6d638705f84` |
| m8 p3fold trades | `65621e3505253a7fcde046a9eaa78bbc8c19798fe84ed626c88b56d6cb0cc21b` |
| m8 p3fold tower_events | `83f45a36ab422a92d198d5d9289c40db94e95fe2cba7af2322e2d37aa8e718c2` |
| m8 wf7 trades | `db6e14fab9f2e5767f38c832aead1efcf6c06b0831181e6c7c7cdaa3deaffbfb` |
| m8 wf7 tower_events | `aa96b3e836be5a43514c425d572c5312d15d8c6950d991d094416e4d61af1cbb` |
| m8 wf8 trades | `abf8245ffd880c0f4488cd02f8696e950cd8349de15c705b52a2add3d679eaf8` |
| m8 wf8 tower_events | `1d8dff0d29e8925fd2931e05259fad11b71745354482c413f4138d8123dfb34f` |

`/tmp/wt421r-pre-head.txt:1` 指向 `ce074711d5`，所以这组直接证明
`ce074711d5..f6639a2a1b` 的返工隔离对拍。首轮 `/tmp/wt421-*` 也各自在
`a12a1022d9..6e15ceffee` 隔离对拍中为 0；两批输入时段不同，不能拿它们互相比字节，
也没有矛盾。整个 `a12a1022d9..f6639a2a1b` 在同一固定数据集上的直接 pre/post 对拍未
提供，故该更强命题**未能判定**。

另外，`/tmp` 产物没有内嵌 commit manifest。文件内容、相互 `cmp` 与 SHA 可以复核，
但文件名本身不能密码学绑定 exact commit；这一来源绑定**未能判定**，不用于制造或消除
HIGH。

## 五、本轮测试

在 `rust/` 下执行：

```text
CARGO_TARGET_DIR=/tmp/kimi-nest-target-check cargo test --lib
```

结果：

```text
1992 passed; 1 failed; 135 ignored; 0 filtered
```

唯一失败：

```text
theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard
```

panic 锚为 `rust/src/theta_v0/classifier/signal.rs:3382`，实得
`16618955402698307653`，期望 `10432481772907336594`；与在册开放票 #491 的唯一红完全
一致。**本轮未见新增失败。** 本复审没有自跑 release 全库；自查所引 release 结果只作为
现存日志，不冒充本轮重跑。

## 六、返工节照实性复核

对 `issue421-acceptance-selfcheck-20260727.md:137-214` 逐项判定：

| 自查项 | 复审判定 | “满足/部分/未满足”是否照实 |
|---|---|---|
| `:147` P-H1 跨 prefix 存活 | **部分满足** | 229/229 与 39/39 确实跨 prefix；但唯一 ForceOvertake 完成身份缺 StructureCompleted，不能写整体满足。原状态偏高。 |
| `:148` 100k 生产寿命 | **部分满足** | 5/120/411 数值成立；分母只含 229 条 StructureCompleted，漏 1 个完成信号，且单位应为 bar-index 差而非 trigger。原状态偏高。 |
| `:149` 终局与关系行 | **部分满足** | 四类终局计数真实，`live_windows != channel_switches` 也真实；ForceOvertake 的完整修订关系不真实。原状态偏高。 |
| `:150-151` P-H2 主接缝/唯一分母 | **满足** | F9 已经主接缝；`completion_signals` 按桥身份唯一，100k 230、20k 40，raw events 只作诊断。 |
| `:152-153` P-H3 共享 dirty/cache | **满足** | `p123_fast_replay.rs:1121-1148,1195-1346` 复用 `LevelDerived/RunEntry`；100k requests/reevals/reuses=`2099/87/2012`，20k shadow=`179/0 mismatch`。 |
| `:154` T8 | **满足** | 当前模块头与 T8 已一致。 |
| `:155` S-H1 | **不在复审面** | `5022e31df5` 与 `coding-style.md:15-19` 的例外真实，登记准确。 |
| `:198,206` 文件/函数上限归 #454 | **未满足 Standards** | #454 是债务票而非豁免，且只点名 `nest_lifecycle.rs`/`advance`，没有覆盖本 diff 的 `p123_fast_replay.rs`、`level_view.rs`、`wverify_run.rs` 与其他长函数。自查漏项、严重度偏低。 |
| `:209-210` integration/E2E 字节门“部分” | **未满足仓内标准** | 外部对拍与 bin 单测是有效证据积累，但没有把生产 p123 字节门固化成仓内 integration/E2E。若评价“已有进展”可称部分；若评价 `common/testing.md:5-18` 的验收，写“部分”偏高。 |
| `:212` #450 后按级复核 | **未能判定** | dump 可外算按级原始数，但 `LifecycleReplayStats` 无按级面，且未提供 #450 结论后的解释闭环。自查保留未能判定是照实的。 |

补充：上文历史节曾把真实数据“两类否证”写成部分；返工产物已经出现
NeverConstituted 与 ForceOvertake，若只判“原因码是否真实发生”，历史“部分”现已偏低。
但返工节声明以 §7 为准，因此这不另算现行虚假；现行真正偏高的是它没有披露
ForceOvertake 修订链缺环。

## 七、Spec 轴汇总

### HIGH

1. **Spec-H1：真实 ForceOvertake 完成路径缺 StructureCompleted。**
   证据为 `nest_lifecycle.rs:720-730,761-769`、公共契约 `:234-238` 与
   `/tmp/wt421r-post-{20k,100k}-lifecycle.dump:721-725`。这使完整修订链与
   通道切换契约不成立。

### MED

1. **Spec-M1：重建报告仍保留旧发生率分母。**
   `v3-lifecycle-rebuild-impl-20260724.md:107-109` 写 20k `0/3715`；当前唯一完成
   分母是 40，100k 是 230。报告与生产统计未收成单一口径。

### LOW

1. **Spec-L1：寿命单位标错。**
   自查 `:148` 把 `as_of` 的 bar-index 差称为 trigger 数。数值重算一致，但指标标签强于
   实际计算。

### 未能判定

1. `/tmp` 产物与 exact commit 的来源绑定。
2. #450 结论后的按级解释/验收闭环。原始按级数已能外算，但不能替代该结论复核。

## 八、Standards 轴汇总

适用硬标准：

- `.claude/rules/common/coding-style.md:23-27,47-50`：文件 max 800、函数 <50；
- `.claude/rules/common/testing.md:3-18`：unit/integration/E2E 均要求，并要求
  RED→GREEN→IMPROVE；
- `.claude/rules/python/coding-style.md:10-13`：所有函数签名类型注解；
- `.claude/rules/python/testing.md:10-12`：使用 pytest。

S-H1 依 `.claude/rules/common/coding-style.md:15-19` 的账本例外排除，不复审、不计数。

### HIGH

1. **Standards-H2：继续扩张超 800 行文件。**
   当前 `nest_lifecycle.rs`=3140、`p123_fast_replay.rs`=1997、
   `level_view.rs`=1864；全 diff 的相邻 #446 还改动
   `rust/src/theta_v0/backtest/wverify_run.rs`=2809。均超过
   `coding-style.md:23-27,47-50` 的硬界且在本区间继续变化。开放票 #454 只是拆分债，
   明文“登记不等于例外”，并且覆盖面不含上述全部文件，故旧 H2 未翻转。

### MED

1. **Standards-M1：长函数与 Data Clumps 仍在。**
   `NestLifecycleBook::advance`=`nest_lifecycle.rs:619-823`，
   `feed_replay_prefix`=`:1122-1212`，
   `run_targeted_prefix_pass`=`p123_fast_replay.rs:831-1180`，
   `coverage_step_from_buckets_sep`=`strategy/coverage/step.rs:28-377`，
   目标提交中的 `provenance_problems`=
   `f6639a2a1b:scripts/check_armR_trades_digest.py:173-253`，均超过 50 行。
   `p123_fast_replay.rs:1223-1455` 的三个函数还反复携带
   `as_of/hist/dif/close_src`，命中 Fowler Data Clumps 判断项。
2. **Standards-M3：生产字节门仍未仓内化。**
   F6 `nest_lifecycle.rs:2827-2861` 只比较一个 Copy 数组的字段与 Debug 文本；
   P-H3 是 bin 单测，不由本轮 `cargo test --lib` 执行。外部 10 对 `cmp=0` 有证明力，
   但仓内仍无覆盖生产 p123 路径的 integration/E2E 字节门，不满足
   `common/testing.md:5-18`。
3. **Standards-M4（全 diff 相邻 #512-514）：Python 测试不进默认门。**
   `f6639a2a1b:scripts/tests/test_check_armR_trades_digest.py:13-227` 使用
   `unittest`，15 个函数签名无类型注解；`pyproject.toml:40-42` 的默认
   `testpaths` 只有 `tests/`，不会发现 `scripts/tests/`。该项属于票面全 diff，
   但不归因给 #421。

### LOW

1. **Standards-L1：F6 名称强于断言。**
   `f6_feed_exit_is_sidecar_events_byte_identical` 没有比较订单流或序列化字节，只证明传入
   Copy 数组未变；真实字节结论来自外部对拍。
2. **Standards-L2：授权型 Duplicated Code。**
   `nest_lifecycle.rs:1051-1065` 的 `leg_as_segment_copy` 与
   `level_view.rs:436-448` 同型。#421 明示授权复制而非扩大私有函数可见性，因此只登记
   Fowler 异味，不要求本票越权改可见性。
3. **Standards-L3（全 diff 相邻 #446）：Primitive Obsession 判断项。**
   `strategy/coverage/step.rs:81-82,305-315` 以 `&'static str` 编码物化来源；
   该域概念可由小型 enum 承载。此为 Fowler 判断项，不是硬规则。

### Fowler 12 条全量对照

| 臭味 | 本 diff 判定 |
|---|---|
| Mysterious Name | 未见可报告证据 |
| Duplicated Code | 有：`leg_as_segment_copy`，授权型 LOW |
| Feature Envy | 未见可报告证据 |
| Data Clumps | 有：p123 四参数簇，随 Standards-M1 |
| Primitive Obsession | 有：#446 的字符串来源码，LOW |
| Repeated Switches | 未见可报告证据 |
| Shotgun Surgery | 未见可报告证据 |
| Divergent Change | 未见可报告证据 |
| Speculative Generality | 未见可报告证据 |
| Message Chains | 未见可报告证据 |
| Middle Man | 未见可报告证据 |
| Refused Bequest | 未见可报告证据 |

## 九、终局判定

**FAIL。**

翻转成立的部分：

- 229/229 与 39/39 的 `Observed→StructureCompleted` 正寿命；
- 两类真实否证原因均发生；
- P-H2 主接缝与唯一分母；
- P-H3 共享 dirty/cache；
- T8 文字/语义；
- release 左端断言、变异红绿、#64 §2(d) 核销；
- 10 对既有输出 `cmp=0`；
- 本轮全库 1992/1/135，唯一红 #491。

阻止整票翻转的直接证据：

1. Spec-H1：唯一真实 ForceOvertake 完成路径缺 `StructureCompleted`，并与公共契约相反；
2. Standards-H2：多个超 800 行文件继续扩张，且无适用例外。

按 #430 票体“发现 HIGH 即回票”的门槛，两轴都不能给 PASS 或 PASS WITH CONDITIONS。
本轮遵守只读范围，未改 issue 状态、未创建修复票。若进入下一次翻转核验，最低可证条件是：
ForceOvertake 完成路径先留 `StructureCompleted` 再终结、230/230 与 40/40 修订链闭合，
重建报告统一采用唯一完成分母；Standards-H2 则须完成实际拆分或取得覆盖这些文件的明确
规则例外，不能仅引用 #454。

<oai-mem-citation>
<citation_entries>
MEMORY.md:3-4|note=[NewChanlun project and worktree continuity]
MEMORY.md:24-24|note=[issue and review routing handles]
</citation_entries>
<rollout_ids>
</rollout_ids>
</oai-mem-citation>
