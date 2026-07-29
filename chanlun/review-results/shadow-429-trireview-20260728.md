# #429 三审：#421 裁定链收口核验

- 日期：2026-07-28
- 角色：NewChanlun 项目评审执行层
- 固定范围：`a12a1022d9ddd8d1cae867a107a3a33c359358cf..391936a486a3b7404b6992a39cdb9f6594626570`
- 工位：`/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`
- 评审方式：只读核验；除本报告外未改仓内文件
- 裁定准绳：[#421 E 裁定 comment-5106372198](https://github.com/xy7365527-lang/NewChanlun/issues/421#issuecomment-5106372198)

## 1. 结论

**总判：FAIL。**

| 轴 | 判定 | 摘要 |
|---|---|---|
| Spec | **PASS** | E 裁定收窄后的五项口径全部满足；原 FAIL 中“必须有正寿命身份”“必须有生产 ForceOvertake”已按 #523 根因与 E 裁定移出本票，未被复活。 |
| Standards | **FAIL** | 排除已裁定的 S-H1 与“文件超过 800 行”S-H2 后，固定全 diff 仍有 **MED×6、LOW×1** 的未豁免仓内标准违例。 |

总判采用合取门：两轴均通过才可 PASS。这里的 FAIL 由 **Standards 轴硬规则未闭合**导致，不反向否定 Spec 轴已经翻转为 PASS 的账本语义。

## 2. 范围、隔离与证据层级

1. 两端均可解析，目标 diff 非空；固定范围共 31 个文件、`+7336/-2215`。范围内除 #421 外还含 #446、#493、#511–#518 等提交；用户要求“全 diff”，故 Standards 轴照收这些文件，不能只审 #421 Rust 主链。
2. 当前 HEAD 是 `28ac09ce841618e88d928916077f098ee441c529`；`391936a486..HEAD` 仅新增 #523 根因报告、248 行身份表及 roster，不改目标 Rust 实装。本审代码判定固定在 `391936a486`，#523 文件只作后续裁定证据。
3. 开跑前工作树已有 4 个 tracked 修改和 5 个 untracked 报告；tracked Rust 差异全是注释，未含 `#[test]`、断言或行为改动。因此本次 `cargo test --lib` 的测试计数未被并行线新增测试污染。证据：本审前 `git status --short` 与 `git diff -- rust/src`。
4. 认识论等级：确定性单元测试为 **L1**；BTC 单标的、固定 20k/100k 前缀实测为 **L2**。未做多标的/多时段交叉验证，故不是 L3，不外推其他 provider、品种或数据窗。

## 3. Spec 轴：PASS

### 3.1 E 裁定五项逐条

| 项目 | 判定 | 证据锚 |
|---|---|---|
| 1. 测试门 | **满足** | 本审自跑 `CARGO_TARGET_DIR=/tmp/kimi-nest-target-check cargo test --lib`：`1997 passed / 1 failed / 135 ignored`；另取失败名确认唯一红为在册 #491 `extract_signals_bit_exact_digest_guard`。现存 release 终点日志 `/private/tmp/wt421-escape-cargo-test-release-lib-final.log` 同为 `1997/1/135` 且唯一 #491；本审未重跑 release，故 release 结论来自该终点日志抽核。lifecycle 定向复跑为 `30 passed / 0 failed`。 |
| 2. 字节护栏 | **满足** | p123 20k/100k stdout、P116 dump 共 4 对，以及 m8 p3fold/wf7/wf8 trades、tower_events 共 6 对，均 `cmp=0`；本审对 pre/post 文件重新执行 `cmp` 与 SHA-256 抽核，读数与自查档 §9.4 一致。 |
| 3. 结算语义 | **满足** | 生产 loop 每 bar 调用 sidecar：`rust/src/bin/p123_fast_replay.rs:942-954,1198-1240@391936a486`。`ReplayBarFeed` 与 `feed_replay_bar` 在 `rust/src/theta_v0/classifier/nest_lifecycle.rs:1266-1422@391936a486`；活窗批次先构造，完成批次后追加，最终同一 `as_of` 依序 `advance`（`:1331-1417`）。 |
| 4. P-H3 / US15 | **满足** | 100k stderr：`provider_requests=2099`、`provider_reuses=2012`、`provider_reevals=87`，复用率 `2012/2099=95.855169%`；仍复用既有 `LevelDerived/RunEntry`，源码锚 `p123_fast_replay.rs:1402-1486@391936a486`。 |
| 5. 自查档 | **满足** | `issue421-acceptance-selfcheck-20260727.md:347-445` 的计数、分布、哈希和失败归因均与产物相符。§9.2 的“p409 神谕未满足”“结算时序部分满足”是 E 裁定前、按当时逃生门神谕作出的保守结论，不偏高也不偏低；E 后应转译为“反超主导已移出票据”，不能继续当本票失败项。 |

### 3.2 Q1/Q2/Q3 与逃生门语义

| 要求 | 判定 | 证据锚 |
|---|---|---|
| 每 bar 两路喂，同 tick 先观察后完成 | **满足** | `nest_lifecycle.rs:1317-1422`：活窗先入 `live_observations`，完成事件再入 `completion_phase`，`:1415-1417` 才按该顺序合批推进；F10a 锚 `:3350-3388`。 |
| 闪现零寿命完整链、事件零丢弃 | **满足** | completion-only 时在当前 bar 合成观察而不回填（`:1373-1391`）；真实首完成独立登记（`:1393-1397`）；F10c `:3453-3485`。100k 为 `completion_signals=248`、entries=248、闪现=248、非闪现 count=0；20k 为 46/46。 |
| 唯一 completion 分母 | **满足** | `completion_signal_seen/register_completion_signal` 在 `nest_lifecycle.rs:1092-1110`，同桥身份只登记首完成；F9 审计与重发去重 `:3487-3562`。100k `COMPLETION_SIGNAL` 行数=248，与 248 identities 一一对应。 |
| Confirmed/Never 与闪现/非闪现分开 | **满足** | 100k：Confirmed=151、NeverConstituted=97、ForceOvertake=0、IdentityVanished=0、flash=248、nonflash=0；20k：35/11/0/0、flash=46。自查档 `:357-379` 与终点 lifecycle dump 同值。 |
| ForceOvertake 可从 Provisional 直接判 | **满足** | 公共契约登记第二终局路径：`nest_lifecycle.rs:52-63`；实装不以完成为前置：`:845-859`；F10b 跨 bar 合成测试 `:3390-3451`。当前 provider 无生产 Force 实例按 E 裁定不再是 FAIL。 |
| IdentityVanished=0、时钟单调、倒退拒绝=0 | **满足** | 100k dump 1143 条 revision，逐 identity `as_of` 回退=0；`IdentityVanished=0`、`retrograde_rejected=0`。20k 同为 0。守卫源码 `nest_lifecycle.rs:819-831`。 |
| 不可验审计接缝可达 | **满足** | `ForceCheck::Unavailable` 在 `nest_lifecycle.rs:861-883` 留独立 audit，不误判终态；F9 经生产 feed 接缝验证、重发不抬分子/分母（`:3487-3562`）。真实 BTC 本窗发生数为 0，只表示本窗未发生，不表示接缝不可达。 |

### 3.3 #523 永禁清单逐条

权威清单：`chanlun/review-results/issue421-structure-completion-root-cause-20260728.md:222-231`；裁定再次钉死于 comment-5106372198。

| 永禁项 | 判定 | 读码结果 |
|---|---|---|
| completion 延迟到下一 bar/trigger | **未发现** | 完成事件以当前 `feed.as_of` 同批结算，`nest_lifecycle.rs:1344-1417`；无延后队列。 |
| 丢弃同 bar completion | **未发现** | 同 bar 先观察后完成；首见只有完成事件时仍合成当 bar观察并登记首完成，`:1373-1397`。 |
| p409 `structure_completed=false` 接生产 | **未发现** | p123 生产接 `feed_lifecycle_bar → feed_replay_bar`（`p123_fast_replay.rs:1222-1239,1489-1515`），无 p409 probe 接线。活窗相使用 `false` 是两相契约本身，完成事件同 bar 显式转 `true`，不等于把 p409 的“完成后继续持有”反事实接生产。 |
| `observed_at` 回填 | **未发现** | 唯一建仓写入 `observed_at: as_of`，`nest_lifecycle.rs:789-815`；completion-only 也只用当前 bar，`:1373-1391`。 |
| `as_of` 加一/减一凑数 | **未发现** | lifecycle feed/advance 全程透传当前 bar index；在结算时钟路径未见 `as_of ± 1`。 |
| 用 `MoveBlock.status` 冒充完成 | **未发现** | 完成通道来自 `NestCandidateEvent` 并在 `nest_lifecycle.rs:1344-1350` 取 Consolidation 事件；未读取 `MoveBlock.status`。该 provider 边界仍有 #523 已钉根因的能力缺口，但未以永禁手段掩盖。 |

### 3.4 原 FAIL 翻转条件转译

| 原条件 | 三审结果 |
|---|---|
| P-H1：需正寿命身份 | **按 E 翻转，不再是失败门。** 当前 provider 首见即完成是已钉死的 L2 真相；零寿命完整链、事件零丢弃均满足。根因在上游 PanLive provider 接缝，不在账本，见根因报告 `:9-38,121-161` 与 [#523 resolution](https://github.com/xy7365527-lang/NewChanlun/issues/523#issuecomment-5106235814)。 |
| P-H2：需生产 ForceOvertake | **按 E 翻转，不再是失败门。** 公共契约和状态机能力已实现；p409 的 113 个 Force 是 post-completion holding counterfactual，不是生产 oracle。 |
| P-H3：US15 复用 | **满足。** 2099/2012/87，95.855169%。 |
| 审计接缝可达、分母唯一 | **满足。** F9 可达；248 首完成=248 唯一分母，重发去重。 |
| 字节护栏 | **满足。** 10 对 `cmp=0`。 |
| 测试门 | **满足。** debug 本审实跑唯一 #491；release 终点日志同值、未在本审重跑。 |

因此，旧 P-H1/P-H2 不得再用于判 FAIL；本审没有这样做。

## 4. 产物抽核

### 4.1 非 lifecycle 护栏

| 面 | cmp | pre/post SHA-256 |
|---|---:|---|
| p123 20k stdout | 0 | `bd9ac1d655f9d615a5d9b3495b3a92465fb1fae13ce5dadfa28033c47d375b6c` |
| p123 20k P116 dump | 0 | `fcc8016a9a01a1098736b9ee3b348614296bb97aec817a5c172c9a0723427a40` |
| p123 100k stdout | 0 | `d8b69c180c23c5e393bf3c330825d88ae9c989ff38f2b8865e049e1b5eb56da8` |
| p123 100k P116 dump | 0 | `8a7327feb3b9ba29f69fa824af1f1ecc137d9303637b47ad8465b6d638705f84` |
| m8 p3fold trades / tower_events | 0 / 0 | `2da686833581d5358a1806aa41ad7ba16371bc3db190fe2a8ccfe62b9cb46627` / `83f45a36ab422a92d198d5d9289c40db94e95fe2cba7af2322e2d37aa8e718c2` |
| m8 wf7 trades / tower_events | 0 / 0 | `3371f1e62153e8aa216ae6f28530bcaa95423a1e3f9b5341e647ec1a78a3fca8` / `aa96b3e836be5a43514c425d572c5312d15d8c6950d991d094416e4d61af1cbb` |
| m8 wf8 trades / tower_events | 0 / 0 | `006c31f54cd72d8ec9c9461122068faf58377cad2d176f839f6a6e0c5ff601b7` / `1d8dff0d29e8925fd2931e05259fad11b71745354482c413f4138d8123dfb34f` |

### 4.2 lifecycle 修正本体

| 前缀 | 逃生门前 SHA-256 | 逐 bar 最终 SHA-256 | 判定 |
|---|---|---|---|
| 20k | `7a927e9b23d6ed2e0ba06ba7eed8cc96fbda4574c77f56836a06394458bfdda8` | `940a69dbaa96b6ff19ee8dc934d6cae65b378c771fe512d34273d1507727bd1b` | **如实变化** |
| 100k | `7ce0b1b39c4837657525696f695d1acbe1aec31414be8dc8e1392c6f31b8b5d9` | `3df5fbc732514929fcde7c6db6298b40539ca338e8af799ea0de58ac729db71c` | **如实变化** |

100k 从 1206 个 provider trigger 扩为 100000 条连续 `FEED cadence=bar`，活窗观察由 89576 增至 11026776；终局分布不变是 provider 首见即完成所致，不是逐 bar 循环未运行。248/248 满足 `p409.observed_at == COMPLETION_SIGNAL.as_of`，差值 min/max=0/0；身份附表 248 行 SHA-256 为 `81bd478fd00adf0238a7a7e96798eda758c257186dbb6bbe66fd16cb248073d2`。

## 5. Standards 轴：FAIL

以下已排除：S-H1 有 `5022e31df5` 裁定例外；文件超过 800 行由 #454/#497 标准债覆盖。该例外只覆盖文件长度，不覆盖函数 `<50`、测试、错误处理、090 文档照实性等其他规则。

### 5.1 未闭合硬项

| 等级 | 发现 | 证据锚 | 违反标准 |
|---|---|---|---|
| MED | 新增生产函数仍明显超过 50 行：`refresh_lifecycle_cache` 82 行、`feed_lifecycle_bar` 92 行、`feed_replay_bar` 98 行。#454/#497 未覆盖 p123 函数长度。 | `p123_fast_replay.rs:1405-1580`；`nest_lifecycle.rs:1325-1422@391936a486` | `.claude/rules/common/coding-style.md:45-51` |
| MED | `ElementView::replace_overlay` 只做 `debug_assert!`；release 下 base 索引或 overlay 越界会静默不写，调用方仍返回 `existing`。非法索引在生产是否可达 **未能判定**，但静默吞错端点客观存在。 | `rust/src/theta_v0/strategy/coverage/element.rs:120-129`；调用 `held.rs:119-137@391936a486` | `common/coding-style.md:29-35` |
| MED | 新增 Python 测试使用 `unittest`，多数 helper/method 签名无类型注解。 | `scripts/tests/test_check_armR_trades_digest.py:1-67,235-254@391936a486` | `python/testing.md:10-28`；`python/coding-style.md:10-13` |
| MED | #421 落仓的是模块/bin unit；真实 p123/m8 字节对拍只留在 `/tmp` 与报告，未成为仓内可持续 integration/E2E 回归门。80% 覆盖率与完整 RED→GREEN 次序 **未能判定**。 | `nest_lifecycle.rs:3036-3072`；`p123_fast_replay.rs:2138-2160@391936a486`；自查档 `:330-345` | `common/testing.md:3-18` |
| MED | 现行实现报告仍声称 lifecycle “只在重估 trigger 投喂”，还引用不存在的“§7.3”并保留旧 `0/3715`；这与 `391936a486` 已落的逐 bar 能力不一致。 | `chanlun/review-results/v3-lifecycle-rebuild-impl-20260724.md:82-109@391936a486` 对照 `p123_fast_replay.rs:942-954,1198-1240` | 090；`result-package.md:3-12` |
| MED | 自查档对 BTC 实测、p409 反证和适用边界没有显式标注 L0–L3。其数据本身照实，但结果包缺强制认识论等级。 | `issue421-acceptance-selfcheck-20260727.md:347-355,404-437@391936a486` | `formalization-validity-domain.md:16-36` |
| LOW | F6 名称/注释称“事件流逐字节相等”，实际只比较 Copy 后的 `PartialEq` 与同一值的 `Debug` 字符串，证明强度低于生产字节流护栏措辞。 | `nest_lifecycle.rs:3036-3070@391936a486` | 090 声明=能力 |

上述项目均无本审可见的裁定豁免或明确债务 destination，故不能降为 PASS WITH CONDITIONS。

### 5.2 Fowler 12 项判断

Fowler 项只作判断题，不作为硬违例重复计数。

| 气味 | 判断 | 锚 |
|---|---|---|
| Mysterious Name | 未见新增可锚项 | — |
| Duplicated Code | **可能存在**：sidecar 喂入、统计、dump 的相似编排仍集中重复 | `p123_fast_replay.rs:1198-1239,1489-1579` |
| Feature Envy | 未见新增可锚项 | — |
| Data Clumps | **可能存在**：`hist/dif/close_src/as_of` 参数组反复同行 | `p123_fast_replay.rs:1405-1414,1489-1506` |
| Primitive Obsession | 沿用域形状，不升硬项 | `(usize, usize)` run/window 键，`p123_fast_replay.rs:1418-1423` |
| Repeated Switches | 未见新增可锚项 | — |
| Shotgun Surgery | 未见新增可锚项 | — |
| Divergent Change | **可能存在**：p123 同文件同时承载回放、provider cache、lifecycle feed、审计 dump | `p123_fast_replay.rs:942-1240,1352-1580` |
| Speculative Generality | 未见新增可锚项 | — |
| Message Chains | 未见新增可锚项 | — |
| Middle Man | 未见新增可锚项 | — |
| Refused Bequest | 未见新增可锚项 | — |

### 5.3 已闭合与非阻断信息

- 范围内后续提交已消除旧 completion 延迟 workaround，并闭合 #515 base→final 恒验、#493 helper 可见性；不重复报旧项。
- `git diff --check a12a1022d9..391936a486` 返回 2，共 18 条 Markdown trailing-whitespace/EOF 诊断，集中在范围内其他 shadow 报告。因其中多为 Markdown 强制换行且无对应硬规则，本审仅记 INFO，不另计阻断项。

## 6. 边界条件、下游推论与影响声明

### 边界条件

1. Spec PASS 只适用于 E 裁定的“当前 provider 首次可见即完成的闪现主导结算账本”。若 destination 重新要求“完成前真实 PanLive 生命史”，则在 #527 provider 能力落地并以 L2/L3 证据复核前，本结论翻转为未满足。
2. 当前 L2 证据仅覆盖所给 BTC 20k/100k 与 m8 三窗；其他标的、时段、provider 的闪现率与分布 **未能判定**。
3. release 测试本审只核既有终点日志，未现场重跑；若日志来源或二进制对应关系被推翻，测试门的 release 半项需重验。
4. Standards FAIL 的翻转条件是：MED×6/LOW×1 分别修复，或获得逐项、可引用的裁定豁免/债务 destination；S-H1/S-H2 的既有例外不能外推到这些项目。

### 下游推论

- #421 的账本语义可按 E 裁定收口；不得再以“没有正寿命”或“没有生产 ForceOvertake”退回本票。
- #527 承接完成前 PanLive provider；p409 保持 post-completion counterfactual，不能接生产真值。
- 固定全 diff 尚不满足仓内 Standards 合取门；若关 #429 还要求双轴都绿，应先处理或裁定 §5.1，而不是改结算语义凑寿命。

### 谱系引用

- Q1/Q2/Q3：[comment-5102692989](https://github.com/xy7365527-lang/NewChanlun/issues/421#issuecomment-5102692989)
- 逃生门：[comment-5103071677](https://github.com/xy7365527-lang/NewChanlun/issues/421#issuecomment-5103071677)
- 逃生门实测：[comment-5103436732](https://github.com/xy7365527-lang/NewChanlun/issues/421#issuecomment-5103436732)
- #523 根因：[comment-5106235814](https://github.com/xy7365527-lang/NewChanlun/issues/523#issuecomment-5106235814)
- E 收窄裁定：[comment-5106372198](https://github.com/xy7365527-lang/NewChanlun/issues/421#issuecomment-5106372198)

### 影响声明

本审未改生产代码、测试、配置、裁定文档或既有报告；唯一新增文件是本报告 `chanlun/review-results/shadow-429-trireview-20260728.md`。未删除、覆盖或整理并行工位的任何修改与未跟踪文件。

<oai-mem-citation>
<citation_entries>
MEMORY.md:61-75|note=[定位421 Q1Q2Q3和逃生门工作线]
extensions/chronicle/resources/2026-07-28T10-45-00-xIXf-10min-memory-summary.md:55-59|note=[确认421逃生门执行连续性]
</citation_entries>
<rollout_ids>
</rollout_ids>
</oai-mem-citation>
