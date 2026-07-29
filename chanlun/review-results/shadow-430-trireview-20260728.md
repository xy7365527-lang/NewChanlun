# #430 三审：裁定链收口核验（新上下文、两轴、禁自评）

**结论：PASS WITH CONDITIONS**

本结论以 #421 [E 裁定 comment-5106372198](https://github.com/xy7365527-lang/NewChanlun/issues/421#issuecomment-5106372198) 为现行唯一准绳。#523 已把“完成前活假设生产化”拆到 #527，并把 p409 的 91.1% 反超读数改定为 `post-completion holding counterfactual`；因此本审不再以“身份必须跨 prefix 存活”或“生产 ForceOvertake 必须发生”判负。

运行语义与 E 裁定要求全部满足，未发现代码级 HIGH，也未命中永禁清单。不能给无条件 PASS 的原因是：#427 要求的现行契约文本尚未全部收口，自查档也没有按 E 裁定重译最终状态；Standards 轴另有 4 项 MED 待登记去向。

| 轴 | 结论 | 分级 |
|---|---|---|
| Spec（#427 转呈清单 + E 收窄口径） | **PASS WITH CONDITIONS** | HIGH 0 / MED 2 |
| Standards（仓内标准源 + Fowler 12 条） | **PASS WITH CONDITIONS** | HIGH 0 / MED 4 / LOW 4 |
| 总结论 | **PASS WITH CONDITIONS** | 无 HIGH；两项 Spec 文档条件未闭合 |

S-H1 已由 `.claude/rules/common/coding-style.md:15-19` 明文裁定例外；S-H2 按本票指令由 #454/#497 覆盖。本审对两项均不复审、不计数。

## 一、范围、权威链与只读边界

- 固定范围：`a12a1022d9ddd8d1cae867a107a3a33c359358cf..391936a486a3b7404b6992a39cdb9f6594626570`。
- 全 diff：15 commits、31 files、`+7336/-2215`、10449 行 diff。#421 直接提交为 `6e15ceffee`、`ce2038b779`、`f6639a2a1b`、`26771ee0d8`、`391936a486`；区间还夹有 #446、#511-#515、#493/#518 等相邻票，不能把其变化冒充 #421 Spec 成败。
- 开工分支为 `kimi-nest-mainline-20260717`，当前 HEAD `28ac09ce841618e88d928916077f098ee441c529`，`391936a486` 是其祖先。端点后的已提交变化只有 roster 与 #523 调查报告/TSV，没有 Rust 变化。
- 开工脏态中的三份 Rust 修改仅改注释，`git diff -- rust` 未新增测试、断言或可执行逻辑；本轮 `cargo test --lib` 的计数没有被未提交测试污染。所有关键行为另从 `git archive` 导出的精确 `a12a1022d9`、`26771ee0d8`、`391936a486` 源树在 `/tmp` 构建，排除了并发工作树污染。
- `git diff --check a12a1022d9..391936a486` 有 18 个诊断，均来自相邻 #446/#493/#511 报告的尾随空格或 EOF 空行；最终 `26771ee0d8..391936a486` 为 clean。
- 本审未改生产代码、测试、issue、map、roster 或既有报告；仓内唯一写入是本文。

权威链：

1. #421 [Q1/Q2/Q3 裁定](https://github.com/xy7365527-lang/NewChanlun/issues/421#issuecomment-5102692989)；
2. #421 [逃生门裁定](https://github.com/xy7365527-lang/NewChanlun/issues/421#issuecomment-5103071677)与[实测回报](https://github.com/xy7365527-lang/NewChanlun/issues/421#issuecomment-5103436732)；
3. #523 [根因调查结论](https://github.com/xy7365527-lang/NewChanlun/issues/523#issuecomment-5106235814)及 `chanlun/review-results/issue421-structure-completion-root-cause-20260728.md:9-29,121-138,218-245`；
4. #421 E 裁定 comment-5106372198。冲突时第 4 项覆盖旧验收神谕。

## 二、旧 FAIL 核心发现按 E 裁定转译

### 2.1 “零寿命 36/36、213/213”不再是 FAIL

#523 已证明零寿命来自上游 PanLive provider 接缝：PanLive 与 completion 使用同一套已完成 substrate，pending/tail 未进入该链；248/248 身份的 `p123.observed_at = p409.observed_at = completion_as_of`。代码链和数值见根因报告 `:9-29,98-138,200-212`。

E 裁定把 #421 destination 收窄为“当前 provider 首次可见即完成的闪现主导结算账本”。现行验收对象是：零寿命完整链、首完成零丢弃、异常为零、终局与闪现率分报，而不是制造跨 bar 寿命。

本审以 bridge identity
`(level, side, kind, seg_a, seg_c_full.0, b_center_start)` 独立解析精确端点 dump：

| BTC 前缀 | FEED 连续性 | entries / 首完成信号 | 完整链 | Confirmed / Never | flash / nonflash | IdentityVanished | retrograde |
|---|---:|---:|---:|---:|---:|---:|---:|
| 20k | 20000，`0..19999`，gap 0 | 46 / 46 | 46 / 46 | 35 / 11 | 46 / 0 | 0 | 0 |
| 100k | 100000，`0..99999`，gap 0 | 248 / 248 | 248 / 248 | 151 / 97 | 248 / 0 | 0 | 0 |

“完整链”要求同一 bridge identity 的 `Observed@t → StructureCompleted@t → terminal@t` 顺序齐全；链中允许按真实事实插入 `FirstProvable@t` 与右端桥迁移 `Supersedes@t`。20k 的 219 条 revision、100k 的 1143 条 revision 均无 `as_of` 回退。首样例见 `/tmp/issue430-exact-post-20k-lifecycle.dump:1694-1701`：`FEED@1693` 后依次记录 `COMPLETION_SIGNAL`、`Observed`、`FirstProvable`、`Supersedes`、`StructureCompleted`、`Confirmed`，全为 `as_of=1693`。

精确端点实测：

- 20k：`completion_events=2653` 是 provider 原始重复观察，唯一首完成分母 `completion_signals=46`；`entries=46`，故首完成覆盖 46/46。
- 100k：`completion_events=125453`，唯一首完成分母 `completion_signals=248`；`entries=248`，故首完成覆盖 248/248。
- 20k/100k lifecycle SHA-256 分别为
  `940a69dbaa96b6ff19ee8dc934d6cae65b378c771fe512d34273d1507727bd1b`、
  `3df5fbc732514929fcde7c6db6298b40539ca338e8af799ea0de58ac729db71c`。
- 代码锚：`rust/src/bin/p123_fast_replay.rs:942-954,1198-1240,1489-1580`；
  `rust/src/theta_v0/classifier/nest_lifecycle.rs:746-953,1317-1422`。

结论：旧报告识别到的零寿命数值事实仍成立，但其“必须有跨 prefix 生命史”的 FAIL 推论已被 #523 + E 裁定撤销。按现行语义，本项 **满足**。

### 2.2 p409 只作反事实，不作生产门

本审同码重跑 `/tmp/issue430-tri-p409-100k.*`：

- identities=248、first_provable=124、ForceOvertake=113、IdentityVanished=20；
- `structure_completed=false`、`feed=every_prefix`；
- `retrograde=0`、`P409_INVARIANTS ok`。

这些结果与根因报告 `:27-29,107-108,222-240` 一致：113 个 Force 是“真实 completion 后仍继续持有”的反事实，不是当前生产账本遗漏。生产 Force=0 按 E 不阻断。

## 三、永禁清单逐条核验

| #523 / E 永禁项 | 判定 | 证据锚 |
|---|---|---|
| 延迟 completion 到下一 bar/trigger | **未命中** | 模块头明确禁止回填或人为延迟：`nest_lifecycle.rs:56-63`；完成在当前 `feed.as_of` 注册并结算：`:1393-1417` |
| 丢弃同 bar completion | **未命中** | `feed_replay_bar` 先留 live，再建 completion phase：`:1331-1417`；20k/100k 首完成覆盖 46/46、248/248 |
| 把 p409 `structure_completed=false` 接生产 | **未命中** | p123 只复用 p409 同构的 stem 发现/延展；真实完成仍来自 `completion_events`：`p123_fast_replay.rs:916-917,1198-1239` |
| 回填 `observed_at` | **未命中** | 新身份只写当前 `as_of`：`nest_lifecycle.rs:790-815`；completion-only 也只在当前 bar 合成：`:1373-1390` |
| 用 `as_of ± 1` 凑数 | **未命中** | exact Rust diff 未见钟位 ±1 改写；所有账本钟直接取 `feed.as_of`：`:1266-1275,1413-1417` |
| 未证语义即以 `MoveBlock.status` 冒充 segment completion | **未命中** | 生产完成输入为 `NestCandidateEvent` completion phase：`:1344-1350`；`MoveStatus` 只见测试夹具，不进该裁决 |

## 四、T8 与 #427 转呈清单

### 4.1 T8 三次订正

模块头现行三项语义互相自洽：

- ForceOvertake 第二终局路径：`nest_lifecycle.rs:52-53`；
- 独立逐 bar、先 live 后 completion：`:56-59`；
- 同 bar 闪现、零寿命照实留链：`:60-63`。

旧报告所指出的反向声明已翻转。T8 现写“禁止回填，因此不能构造 `first_provable_at < observed_at`”，见 `:1977-1982`；实际建仓与首证顺序见 `:790-843`。这一部分 **满足**。

但 T8 同段仍写“每个 trigger”及“跳过非 trigger prefix 只会晚记”，`:1979-1982`；`provide_pan_live_windows` 文档也仍称 p123 在生产重估 trigger 调 `feed_replay_prefix`，`:1135-1141`。现行生产实际每 bar 调 `feed_replay_bar`，见 `p123_fast_replay.rs:1222-1240,1489-1515`。这些话只有在 legacy `ReplayPrefixFeed`/adapter（`nest_lifecycle.rs:1227-1235,1296-1315`）语境下成立，未加限定时不再描述生产世界。因此 T8 契约文字判 **部分满足（MED）**，不是运行错误。

### 4.2 #427 逐条

| #427 条款 | 判定 | 证据锚 |
|---|---|---|
| feed 契约订正 | **部分满足** | 模块头 `nest_lifecycle.rs:52-63` 已正确；T8 与 provider 文档仍残留 trigger-only 话术，见上节 |
| 重建报告收为单一现行口径 | **部分满足** | `v3-lifecycle-rebuild-impl-20260724.md:92,94,101-109` 仍称生产只在重估 trigger 调 `feed_replay_prefix`，并保留旧 OKLO `0/3715` 分母；与逐 bar 实装及 E 的唯一首完成分母不一致 |
| 左端恒等 release 断言 | **满足** | 唯一写入点无条件 `assert_eq!`：`level_view.rs:786-799` |
| 左端恒等双臂测试 | **满足** | 确认/未确认两臂及右端确变：`level_view.rs:1653-1785` |
| 左端变异红、恢复绿 | **满足** | `/tmp/spec430-left-anchor-mutant.log:435-446` 以 `left=121/right=120` 红；`/tmp/spec430-left-anchor-restored.log:436` 为 1/1 绿 |
| 通道切换力度跃变注记/测试 | **满足（机制层）** | F3 与状态机第二终局路径；`nest_lifecycle.rs:845-859,887-920`。E 不要求生产样本必须出现 Force |
| #64 §2(d) 字段核销 | **满足** | 重建报告 `:110-116` 明确不复活 provider `seg_c_full`、不新增 `c_start_live`；release 左端断言承保工程桥 |
| 订单流字节护栏 | **满足（按 #421 贡献隔离）** | 精确构建结果见下节 |

## 五、字节护栏

### 5.1 p123 两档：全区间两端精确对拍

pre 为 `git archive a12a1022d9`，post 为 `git archive 391936a486`，均使用独立 source/target 构建：

| 面 | 规模 | `cmp` | pre = post SHA-256 |
|---|---:|---:|---|
| stdout | 20k | 0 | `bd9ac1d655f9d615a5d9b3495b3a92465fb1fae13ce5dadfa28033c47d375b6c` |
| P116 replay dump | 20k | 0 | `fcc8016a9a01a1098736b9ee3b348614296bb97aec817a5c172c9a0723427a40` |
| stdout | 100k | 0 | `d8b69c180c23c5e393bf3c330825d88ae9c989ff38f2b8865e049e1b5eb56da8` |
| P116 replay dump | 100k | 0 | `8a7327feb3b9ba29f69fa824af1f1ecc137d9303637b47ad8465b6d638705f84` |

### 5.2 m8 三窗：最终逃生门贡献精确对拍

pre 为精确 `26771ee0d8`，post 为精确 `391936a486`；两端均用默认净额执行口径，逐窗运行 ignored m8：

| 窗 | trades `cmp` / SHA-256 | tower_events `cmp` / SHA-256 |
|---|---|---|
| p3fold | 0 / `2da686833581d5358a1806aa41ad7ba16371bc3db190fe2a8ccfe62b9cb46627` | 0 / `83f45a36ab422a92d198d5d9289c40db94e95fe2cba7af2322e2d37aa8e718c2` |
| wf7 | 0 / `3371f1e62153e8aa216ae6f28530bcaa95423a1e3f9b5341e647ec1a78a3fca8` | 0 / `aa96b3e836be5a43514c425d572c5312d15d8c6950d991d094416e4d61af1cbb` |
| wf8 | 0 / `006c31f54cd72d8ec9c9461122068faf58377cad2d176f839f6a6e0c5ff601b7` | 0 / `1d8dff0d29e8925fd2931e05259fad11b71745354482c413f4138d8123dfb34f` |

三窗两端各为 1/1 绿；日志为 `/tmp/issue430-exact-{q,post}-m8-{p3fold,wf7,wf8}.log`。

090 范围注记：若直接拿 full diff 两端 `a12a1022d9` 与 `391936a486` 对拍，m8 `trades.jsonl` 会 `cmp=1`，因为全区间夹有 #446 的活动集重复计数修复等真实交易行为变更；`tower_events` 仍为 `cmp=0`。这不构成 #421 侧车反流。#421 的初装、返工、Q1/Q2/Q3、逃生门各阶段隔离产物分别见 `/tmp/wt421-*`、`/tmp/wt421r-*`、`/tmp/wt421q-*`、`/tmp/wt421e-*`，每一阶段自身 pre/post 的 m8 六面均 `cmp=0`；本审又对最终 `26771ee0d8..391936a486` 做了精确提交重生。不得把相邻 #446 的预期变化掩成 full-range `cmp=0`，也不得反向归罪 #421。

## 六、测试门与工作区污染核验

票面命令本审实跑：

```text
cd /tmp/kimi-nest-mainline/rust
CARGO_TARGET_DIR=/tmp/kimi-nest-target-check cargo test --lib 2>&1 | tail -3

test result: FAILED. 1997 passed; 1 failed; 135 ignored; 0 measured; 0 filtered out
error: test failed, to rerun pass `--lib`
```

完整日志 `/tmp/issue430-tri-cargo-test-lib.log:1571,2573-2584` 证明唯一失败为
`theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`，即在册 #491。未发现第二个失败。

并行未提交 Rust 面仅含 `econ_positive.rs`、`theta_v0/mod.rs`、`nautilus/mod.rs` 的注释订正；未含 `#[test]`、测试函数或执行逻辑。端点后的已提交 Rust diff 为空。因此当前 1997/1/135 可作为端点测试集合证据，不受未提交测试计数污染。release 历史门 `/tmp/wt421-escape-cargo-test-release-lib-final.log` 同为 1997/1/135、唯一 #491；本审的精确 release 构建及六次 m8 也全部成功。

结论：测试门 **满足**。

## 七、自查档照实性逐节核验

自查档采用追加式时间线，并在 §§7、8、9 明说后节覆盖前节。故旧节不能删改为现况，但必须由 E 执行节给出当前状态；现文件缺这一节。

| 自查范围 | 事实核验 | 按 E 的现行判定 |
|---|---|---|
| §§0-6 初装节 | 当时测试、初装护栏与“不能完整验收”的历史方向基本照实；后续章节已明确覆盖。`§3:65-66` 的 trigger 契约/报告“满足”不再是现况 | **历史记录可保留；不得作当前结论** |
| §7 返工节 `:137-214` | P-H1/P-H2/P-H3 的返工阶段读数与对应 `/tmp/wt421r-*` 证据相符；§7.4 把仓内 integration gate 判“部分”是诚实的 | **事实满足；状态为历史**。`:154,208` 所称 T8/声明已完全满足，在逃生门落码后失效 |
| §8 裁定执行节 `:216-313` | Q1/Q2 的 248/248 零丢弃、100% 闪现及 hashes 与重生一致；当时把 p409 oracle 判未满足、Q3 判部分符合 E 前裁定 | **历史事实满足**；`:227,247-248,307-313` 的“部分/未满足”已被 E 移出或覆盖 |
| §9 逃生门实现/统计 `:315-438` | RED/GREEN、逐 bar 数、终局分布、SHA、248/248 join 与根因方向均准确；本审精确重生一致 | **事实满足** |
| §9.2 项 3 `:353` | 数值事实正确，但仍把 p409 正面神谕判“未满足” | **现行偏低**：E 已移出本票，应写“不适用/反事实单列” |
| §9.2 项 4 `:354` | 连续 FEED、零回退、零丢弃、Vanished=0 均准确；仅因生产 Force=0 判“部分” | **现行偏低**：E 的结算时序要求已全部满足 |
| §9.2 项 5 `:355` | 声称“自查档满足” | **现行偏高**：文件没有 E 执行节，不能声称当前裁定已登记 |
| §9.6 `:439-447` | 是 E 前尚未裁定时的诚实遗留 | **已被 #523 + E 覆盖**；“须再改完成语义”不再是 #421 当前遗留 |
| 总结 `:5,219` | “部分满足”在各自落笔时点保守且不冒充通过 | **当前偏低**：E 下运行验收已满足；现文件只能因缺 E 执行节判“部分满足（文档）” |

其他仍照实：

- `:209-210` 把仓内 production integration/E2E 字节门判“部分”，本审 Standards 复核仍成立。
- `:58-59,131-134,212` 的 `CONTEXT.md` 编排项与 #450 按级复核仍没有本审所需闭环；它们不被 E 自动改写，也不构成本票运行 HIGH。
- §§7-9 的历史数据不应回写成 E 后数字；正确动作是追加 E 执行节，明确“历史结论被后裁覆盖”。

自查档总判定：**部分满足**。数据与失败项总体诚实；问题是现行裁定状态滞后，不是产物造假。

## 八、Spec 新发现与条件

### SPEC-MED-1：#427 单一现行 feed 口径仍分裂

行为已是逐 bar，模块头也正确；但以下声明仍停留在 trigger-only 世界：

- `nest_lifecycle.rs:9-10,1135-1141,1977-1982`；
- `v3-lifecycle-rebuild-impl-20260724.md:92,94,101-109`。

其中报告 `:108-109` 还沿用旧 OKLO `0/3715`，既不是当前 BTC 数据，也不是 E 要求的唯一首完成分母。现行 BTC 应写 20k `46/46`、100k `248/248` 的唯一信号覆盖，并将 raw `completion_events` 明确标为重复诊断量。

影响：声明与实际能力不一致，违反 `AGENTS.md:9` 与 `.claude/rules/no-patch-mentality.md:15,22,32,42`。行为不受影响，故 MED。

### SPEC-MED-2：自查缺 E 执行节

自查 §9 的事实正确，但 `:353-355,439-447` 仍给出 E 前状态；总判定 `:5,219` 也未重译。最低修复是追加 E 执行节，保留旧节为历史，逐项把 p409 改列反事实、把 E 结算门改为满足，并登记 #527 上游去向。故 MED。

## 九、Standards 轴

### 9.1 MED 与去向

| 发现 | 分级 | 证据锚 | 去向 |
|---|---|---|---|
| 函数 `<50` 仍多处不满足，并有 `as_of/hist/dif/close_src` Data Clumps | MED | 标准 `.claude/rules/common/coding-style.md:45-50`；`nest_lifecycle.rs:746-953,1325-1422`；`p123_fast_replay.rs:900-1294,1405-1486,1489-1580`；`level_view.rs:722-911` | #454 可承接 nest/advance，#497 可承接 level_view；p123 长函数与参数簇尚无覆盖票，需另行登记，不得借 S-H2 豁免 |
| T8/provider/report 的 trigger 旧话术 | MED | 同 SPEC-MED-1 | #421 文档收口条件 |
| production p123 integration/E2E 字节门未仓内化 | MED | `common/testing.md:5-18`；F6 `nest_lifecycle.rs:3036-3071` 只比较合成 `Copy` events 与 Debug 文本 | 专门回归门票；外部 exact cmp 有验收证明力，但不能冒充仓内常驻门 |
| 相邻 #511 Python 测试不符合项目 Python 标准且默认 pytest 不收集 | MED | `scripts/tests/test_check_armR_trades_digest.py:1-13,48-69,253-254` 使用 unittest 且多处缺注解；`pyproject.toml:40-42` 只收 `tests/`；标准 `python/testing.md:10-12`、`python/coding-style.md:12-14` | 回 #511/CI 测试发现面，不归因 #421 |

### 9.2 LOW 与去向

| 发现 | 分级 | 证据锚 | 去向 |
|---|---|---|---|
| F6 名称 `byte_identical` 强于断言 | LOW | `nest_lifecycle.rs:3036-3068` 未比较生产序列化字节 | 与 production E2E 门同票更名/替换 |
| leg→segment 转换重复 | LOW | `level_view.rs:436-448`、`nest_lifecycle.rs:1199-1213`、`p123_fast_replay.rs:1334-1347` | #426 明确授权复制且禁提升私有可见性；只登记 Fowler Duplicated Code，不要求本票改 |
| #446 用字符串表达有限来源类别 | LOW | `strategy/coverage/step.rs:79-82,120,141,174,255,295-322` | 回 #446 后续类型化；不归因 #421 |
| full diff 有 18 个空白诊断 | LOW | `/tmp/issue430-tri-diff-check.txt`；均在 #446/#493/#511 报告，最终两提交 clean | 回各报告来源票做文档卫生 |

### 9.3 Fowler 12 条

| 臭味 | 本 diff 判定 |
|---|---|
| Mysterious Name | 未独立命中；F6 名称强于断言已列 LOW |
| Duplicated Code | 命中；#426 授权型复制，LOW |
| Feature Envy | 未发现 |
| Data Clumps | 命中；纳入 Standards MED |
| Primitive Obsession | 命中；#446 字符串来源码，LOW |
| Repeated Switches | 未发现 |
| Shotgun Surgery | 未发现 |
| Divergent Change | 未发现可报告项 |
| Speculative Generality | 未发现；`feed_replay_bar` 有生产调用，prefix adapter 有兼容测试契约 |
| Message Chains | 未发现 |
| Middle Man | 未发现 |
| Refused Bequest | 未发现 |

#493 拆分后的 `wverify_run.rs`、`m8.rs`、`report.rs`、`tests.rs` 均未超过 800 行，helper 最终保持 private；该相邻票除报告空白字符外未见新增 Standards 问题。

## 十、终局判定与翻转条件

**PASS WITH CONDITIONS。**

已满足且足以翻转旧 FAIL 的部分：

1. E 口径下，20k/100k 全部首完成身份均有零寿命完整链，46/46、248/248 事件零丢弃；
2. `IdentityVanished=0`、`retrograde_rejected=0`、FEED 无 gap、revision `as_of` 无回退；
3. Confirmed/Never 与 100% 闪现率独立分报；生产 Force=0 不再是验收门；
4. #523 已把缺口钉到上游 provider，当前账本没有伪称完成前生命史；
5. 六项永禁全部未命中；
6. p123 两档四面及最终逃生门 m8 三窗六面均以精确提交构建 `cmp=0`；
7. 当前测试门 1997/1/135，唯一红 #491，未提交面未污染测试计数；
8. release 左端断言、双臂测试、变异红绿与 #64 §2(d) 核销均闭合。

转为无条件 PASS 的最低条件：

1. 把 T8、`provide_pan_live_windows` 文档及重建报告统一到“生产逐 bar 两相 feed；trigger adapter 仅 legacy/测试兼容”的现行口径，并删除/历史化旧 `0/3715` 分母；
2. 在自查档追加 E 执行节，把 p409 标为反事实，把 §9.2 项 4 按 E 改判满足，并登记 #527；
3. 为 Standards MED-1、MED-3、MED-4 建立上述去向；其中生产 integration/E2E 字节门不得继续只靠 `/tmp` 人工产物。

在条件 1、2 完成前，不得宣称“裁定文档链已无条件闭合”；但按 E 收窄后的代码、运行语义与实测门，未发现需要把 #421 再判 FAIL 的 HIGH。
