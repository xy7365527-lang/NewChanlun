# ADR 0029：共同资金争用——原子 slot 等待、版本事件重算、只取空闲、人工具名 cap

**日期**：2026-10-13
**裁定人**：代理依用户 2026-10-13 授权裁定（分析：claude-fable-5-1；根逐条复核并补充），走 [[grilling] 共同资金争用、cap 与公用资金进出 policy #1349](https://github.com/xy7365527-lang/NewChanlun/issues/1349)（#1323 / SPEC #1340，FU-03/02/04）
**状态**：已采纳。本 ADR 只定规则与记录形状，不填 cap、Q、投入额、系数或期限。

## 背景

[ADR 0024](0024-total-ledger-convention.md) 下游票第 2 条把总账户 cap 的参数源留给后续票；SPEC-COVERAGE FU-03 把争用分配、公用资金进出、实际 cap 列为 `REAL_POLICY_OR_PERMISSION_GAP_NO_DEFAULT`。#1394 在制代码以 `#1349/unresolved-common-cap-allocation` 占位，行为是「同 slot 全部申请合计超 cap 则整 slot 等待、后来者不插队」（FIN `rust/src/economic_session/finance.rs:917`、`:1023-1024`）。缠师原文全为单账户口径：`040-第40课.md:20`【正文】「每一重都对应着一定的资金与筹码」、`:22`【正文】「每一层次的操作都是独立又在一个整体的操作中」；`031-第31课.md:26`【正文】「这股票该买多少，该占总体资金多少，一开始就应该研究好，投入以后就不能再增加」、`:30`【正文】「资金，就可以不断增加参与的股票种类」；多重同点争用原文留白。

## 裁定

### 一、同点争用默认整 slot 原子等待（政策 id `#1349/atomic-slot-wait/v1`）

在默认 await_slot 下，同一 slot 全部申请者的毛锁额（数量 × 限价 + 费用尾）加既有责任 ≤ cap 才整 slot 授信，否则整 slot 进入 `AwaitingAllocationPolicy`。不排序、不缩量、不拆单；默认 await_slot 不拒绝。理由：[ADR 0014](0014-intra-chong-unidirectional-inter-chong-no-arbitration.md) 裁定二无仲裁人；[ADR 0016](0016-chong-quantity-layer-capacity-frequency-shift.md)/[ADR 0017](0017-chong-quantity-numerator-shortdiff-amplitude.md) 的 w(d)×Q 不动；与在制行为同构。

- 等待无时钟时限。失效只来自三类事实：任一申请者在后续稳定点重新 Prepare 或撤回（剩余者构成新 slot 重判）；政策版本到期（`PolicyExpired`）；注入或释放使全体放得下（裁定二）。
- 后来 slot 不越过等待中的 slot。这是算术推论：等待成立时既有加等待者全额已 > cap，任何追加都放不下；允许小 slot 用余量通过，等于按大小选人，即被否的 B/D。
- **跨 slot 次序 = S 稳定点的结构顺序。这是本裁定的显式具名选择**（根复核确认），不是把 FIFO 当默认（RESIDUAL FU-03 所禁）。理由：ADR 0014 下没有仲裁人，任何其他次序都要另造一个来源。

> #1599（用户 2026-10-26 批准）：字段进入所属具名版本，一次运行只读一个已签版本，改值即新版本并重推导；不得因失败换值放行。

`on_contention={await_slot,reject}`，默认 await_slot；reject 是合法政策动作，但会增加重试。`withdraw_rule={partial_up_to_free,deny_if_liability}`，默认 partial_up_to_free；deny_if_liability 保留为有责任即禁取的选择，代价是降低流动性。两者都不改变 `Σ责任≤cap` 真值判据，也不允许选择赢家、缩放 w(d)×Q 或运行中失败换值。

### 二、注入与释放 = 政策版本事件，落地即重算

公用资金注入产生新政策版本（cap、version、authorizer、authorized_at 随之变化）；责任释放（[ADR 0030](0030-exit-authorization-and-liability-retirement.md) 裁定四的退休、撤回）同样是触发源。事件落地即对等待中的 slot 按结构顺序逐个重算；授信记录写 `policy_version`。注入不划给具名重作新 Q（那是 FU-02/#1357）。

### 三、取出默认只限空闲部分

在默认 partial_up_to_free 下，可取 = cap − Σ既有责任；等待中的 slot 不占用。超出部分当场拒绝并写原因，不排队。这保住 Σ责任 ≤ cap（ADR 0024 裁定一/五），也不让资金政策内部产生减仓压力——退出来源只认 ADR 0030 的三类。原文边界：`026-第26课.md:26`【正文】「一旦钱是有限期的，那么等于自动设置了一个停止交易的时限」。

### 四、cap 来源 = 人工具名数额（不填默认金额）

记录形状 ⟨policy_id, version, cap, currency, valid_through, authorizer, authorized_at, profile_id⟩。注入/取出/改额都是新版本；过期后新 Reserve 拒绝，既有授信不受影响。

**cap 数值不在本 ADR 给出（根补充）**：随具体能力验证或生产激活，由用户签署一版政策记录；本 ADR 不填任何默认金额，票面「不填未经确认的 cap」仍成立。账户权益快照不作 cap 来源（[ADR 0028](0028-first-capability-verification-profile.md) 的 P1 是测试资金）；在有真实账户的 profile 上，它只作 TB-08 偿付性核验，与 cap 判的不是同一件事。

## 被否方案

B 具名优先序（需发明来源、引入仲裁人）；C 按比例（改 w(d)×Q、取整残量）；D 覆盖最大（隐式 B、需平局规则）；I2 只对新申请生效（会永久等待）；I3 划给具名重（FU-02）；O2 允许 cap < 责任（破坏不变量、隐含退出来源）；C2/C3（P1 权益无经营意义，系数未填）。

## 手算对账

W1：既有 20、cap 80、{30, 40} 等待；后到 10 不越过；40 撤回后 {30, 10} 构成新 slot，20+30+10 = 60 ≤ 80，一起授信。W3：注入 20 → cap 100，20+70 = 90 ≤ 100，授信并记 v2。W4：cap 100、责任 90、取 30 → 取出 10、拒绝 20、cap 90，不减仓。

## 受影响代码清单

见 [`.chanlun/review-results/issue1323-grilling-20261013/1349-AFFECTED-CODE.md`](../../.chanlun/review-results/issue1323-grilling-20261013/1349-AFFECTED-CODE.md)。改动集中在 FIN 线：政策 id 具名化（`finance.rs:225`）、等待文案与失效/重组（`:917`）、触发源扩展（`:1015-1024`）、政策字段加版本（`:204-213`），及对应测试与客户端校验。main 侧 `theta_v0` 比值帽是旧实现，不是 R2 共同资金 owner，不改。

## 下游票

[#1394](https://github.com/xy7365527-lang/NewChanlun/issues/1394)（具名化与版本字段）、[#1408](https://github.com/xy7365527-lang/NewChanlun/issues/1408) TB-04-I、[#1445](https://github.com/xy7365527-lang/NewChanlun/issues/1445) TB-06-E、[#1412](https://github.com/xy7365527-lang/NewChanlun/issues/1412) TB-08-C；[#1355](https://github.com/xy7365527-lang/NewChanlun/issues/1355) 不受影响；ADR 0030 的释放触发与「权益跌破责任只冻结不减仓」与本 ADR 对齐。

## 溯源标签

四问全部 `[新缠论:选择]`；原文只给边界。

## #1599 批复与受影响清单

[用户批复](https://github.com/xy7365527-lang/NewChanlun/issues/1599#issuecomment-6403278294)；[受影响清单](../../.chanlun/review-results/issue1599-policy-20261026/affected-code.md)含固定基线、Rust/Python/Lean 接缝及下游未承接项。本次为政策文书，不声称实现或关票；新增自由度为 `[新缠论:选择]`，既裁不变量不变。
