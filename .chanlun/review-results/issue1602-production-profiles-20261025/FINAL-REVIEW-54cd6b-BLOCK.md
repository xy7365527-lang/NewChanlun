# PR #1681 / issue #1602 独立非作者最终复审

复审日期：2026-10-25  
复审身份：独立非作者；只读静态复审，不是实施者。

## 裁决

| 维度 | 结论 |
|---|---|
| 代码 / 合同 | **BLOCK** |
| 当前 head CI | **BLOCK（尚有 1 项运行中，不能判 PASS）** |
| 总体 | **BLOCK** |
| CRITICAL | **0** |
| HIGH | **0** |
| MEDIUM | **1** |
| LOW | **0** |

唯一代码 / 合同阻断项是：v2/v3 的正常输入、幂等及查询回执没有把实际所用记录的 id/version 写到外部输出，未满足 issue #1602 的共同验收。除此之外，本轮没有发现执行成本、风控与声部参数、场所 tick、坏 tick 闸的记录化方向、`AwaitingPolicy`、禁止备用值或生产/诊断分流上的新阻断问题。

## 1. 候选、base 与复审边界

- 当前本地 detached HEAD：`54cd6b2e2b8567e79ba3fe8bb513986ce007347c`。
- 当前 PR API `headRefOid`：`54cd6b2e2b8567e79ba3fe8bb513986ce007347c`，与指定候选一致，本审有效。
- 当前 PR API `baseRefOid` 与本地 `origin/main`：`13ed4d45c8ff85175819ca987690328ab2a247a4`。
- 旧 head：`cc8d45c4d0bd40fba203f17f86b7425fa8f6c2eb`；旧 base：`3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f`。
- 完整新 head diff：31 个文件，2358 insertions / 326 deletions。
- 工作区在复审首尾均保持干净；未改代码、票据或分支，未合 main、未关票，未派子代理，未在本机运行 `cargo`、`rustc` 或 `cargo test`。
- 已读根 `AGENTS.md`、issue/PR、旧 head 尾部独审、ADR 0041、ADR 0040、ADR 0044、ADR 0035、ADR 0028 与 #1586 审计正本。

PR body 中仍能看到旧 head 文字，但 GitHub PR API 的实际 head/base 已单独核实；本审以 API 与本地精确提交为准，不以 body 的旧提交文字代替候选核验。

## 2. rebase 等价性

执行口径：

```text
git range-diff 3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f..cc8d45c4d0bd40fba203f17f86b7425fa8f6c2eb \
               13ed4d45c8ff85175819ca987690328ab2a247a4..54cd6b2e2b8567e79ba3fe8bb513986ce007347c
```

结果 15/15 全部为 `=`：

```text
 1: 45b339c36b =  1: 756cbed66f
 2: 493277a8cf =  2: aa6e0c582a
 3: f79dd38f41 =  3: da2c8e40ae
 4: d46daf030e =  4: 16ebdf78a3
 5: a7af5cad52 =  5: 605bd88436
 6: 841a94391d =  6: ec3c0c319e
 7: d9a04f21fc =  7: b4f549ced6
 8: 17e40660cb =  8: 8d2a6c5872
 9: a0fd07025f =  9: 980e0f04cb
10: 1176d18e2f = 10: d3c6d91196
11: 9e262a85bc = 11: cb7ddd8afa
12: c6ac3a25f9 = 12: d1083c0c06
13: 4d9e2336b8 = 13: d073cfa831
14: 07e3443c51 = 14: 320d52f35e
15: cc8d45c4d0 = 15: 54cd6b2e2b
```

没有冲突适配或本票文件的内容改写需要解释。新旧 head 的快照差异与 old base → new base 的推进一致：新增 #1621/#1622 两份实施记录，并推进 `economic_session` service/store 与 tb03c 测试；没有夹带本票语义变更。

## 3. MEDIUM — v2/v3 外部回执丢失记录版本戳

### 合同

issue #1602「共同验收」明确要求：

> 运行路径只从当前生效的版本化记录读取本票各量……**每份输出写出所用记录 id 与版本号**。

专项验收同时要求，诊断 profile 使用夹具值时，输出必须标明且不参与准入。

### 决定性证据

1. 旧 S 的 `advance_core` 正常/幂等结果已经带 `policy_records`：`rust/src/bin/s_structure_session.rs:3079`、`:3946`。
2. v2 调用了该函数，却丢弃其返回值：`rust/src/bin/s_session_v2/mod.rs:2374-2375`。随后对外返回的是重新查询的 `lookup` 结果：`:2388-2396`。
3. v2 已提交消息的幂等分支更早就直接返回旧 `lookup` 结果：`:2345-2353`；`receipt` 查询也直接返回同一形状：`:2329-2337`。
4. 该共享 `lookup` 的最终 JSON 只有 `ok/kind/original_message_id/receipt_id/accepted_seq/publication`，没有 `policy_records`：`rust/src/bin/s_session_v2/mod.rs:2222-2251`。S3 也共用这一回执形状。
5. S3 只给无输入的 `state_summary` 附加记录戳：`rust/src/bin/s_session_v3/driver.rs:218-227`。有输入的正常推进则直接返回 `receipt.message_receipt`，没有附戳：`rust/src/bin/s_session_v3/driver.rs:485-511`；CLI 又直接输出该值：`rust/src/bin/s_session_v3/writer_cli.rs:642-644`。
6. `GenerationRuntime` 虽新增内部 `Generated.policy_records`（`rust/src/bin/s_session_v3/generation.rs:250-251`、`:885`），但检索到的消费只有生成测试断言 `:2010`；它没有被写入上述外部回执。内部结构体带字段不等于每份外部输出带版本。

### 可复现的失败场景

用诊断夹具，或将来一份字段签齐且可准入的 venue profile，向 v2/v3 提交合法 `ingest`：成功回执会有 `kind=Committed` 与 publication 信息，却没有本次实际使用的 `venue-profile-*`、`theta-risk`、`data-profile` 等记录 id/version。重复同一消息或查询 receipt 仍同样缺戳。因而仅凭输出无法区分两份不同记录版本产生的同形结果，也无法从诊断回执确认它使用的是夹具记录。

这没有直接改变成交数值，所以不定为 HIGH；但它明确未闭合票据的输出溯源验收，故为 **MEDIUM 且阻断本票 PASS**。

### 关闭条件

在 v2/v3 外部响应边界补齐与实际生成结果一致的记录 id/version，覆盖至少正常 `ingest/advance`、幂等 `Committed` 与 `receipt` 查询；若幂等/查询发生在记录改版之后，不能用“当前配置”冒充历史结果实际使用的版本，必要时应随结果持久化溯源。新增外部输出回归测试，不能只断言内部 `Generated.policy_records`。

## 4. 其余重点合同复核

以下项目静态证据成立，不抵消上述 MEDIUM，但也不需要重复返工：

- **记录落盘齐全**：`venue-profile-P0.json` 承载 tick 与四项执行成本；`theta-risk.json` 承载 rho/beta/gamma/kappa/default_lot/chi_z_alpha/cap 开关/max_depth/w_grade；`data-profile.json` 承载 `bad_tick_max_dev`。
- **P0 不伪造未知政策**：`venue-profile-P0.json:11-35` 的 tick、slippage、tax、entry delay 保持 `AwaitingPolicy`；只有已签的 Binance Spot VIP0 taker commission=10bps/side 是数值。P0 当前停止是合同要求，不是实现缺陷。
- **诊断值不冒充生产**：`venue-profile-diagnostic-fixture.json:6-9` 为 `admissible=false`，旧默认常数只存在该显式诊断记录；FFI、backtest/replay 与 Python 入口均要求显式 profile 或显式 fixture，缺少配置时 fail-closed。
- **合法可选值未写死**：`rust/src/theta_v0/config.rs:966-1003` 用 v2 替代记录验证不同成本、风控及坏 tick 数值确实进入配置；`:1010-1022` 验证就地改值会标 `research-override`，不再冒充 Signed。
- **缺失/非法/损坏分型**：`rust/src/theta_v0/run_records.rs:526` 覆盖损坏生产 profile 头为 `InvalidPolicy`；`:584` 覆盖缺记录、缺字段与非法值负控。没有主值失败后回退旧常数的路径。
- **等值与变异体锁**：`run_records.rs:476` 做 v1 记录与旧常数等值对拍；`config.rs:1059` 有源码常数回退防护；`backtest/data.rs:286-299` 实际使用配置阈值，`:603-622` 用 0.25 变异阈值证明 30% 跳空会被拦。
- **批量准入不吞政策失败**：`rust/src/bin/theta_accept.rs:155-156` 将 `has_policy_failure` 纳入整批成功条件，`:565-568` 覆盖“一个成功品种 + 一个政策失败仍失败”。旧尾部独审指出的该项返修保持有效。
- **旧三项返修均保持**：正式 S 默认生产且测试显式 fixture；`theta_accept` 政策失败不被其他品种掩盖；损坏生产 profile 头不误报 `RecordMissing`。

范围边界：`VoiceConfig.depth_weights` 属 C-07/审计判 E，`initial_nav` 属 C-03/#1601，均不作为本票新 finding；显式 TestOnly Native 路径也没有被误算成默认生产旁路。

## 5. CI 与验证证据（独立于代码裁决）

本审没有在本机运行 Rust 工具链；以下均为 GitHub 当前 head 的远端状态或明确标作旧证据，不把旧结果冒充新 head 全绿。

当前 head CI run：`40143224153`。

已通过：

- `test`
- `DevSkim`
- `rust-check`
- `rust-test-default`
- `rust-test-backtest-bin`
- `rust-test-s-session` shard 1、2、3
- `rust-test-s-session-extras`
- `s-observer`
- `fixture-drift`
- `sandcastle-ts`

仍运行中：

- `rust-test-s-session` shard 0（job `121019119780`）。

`macos-session` 为 **SKIPPED**。因此截至本报告刷新时，CI 是 **BLOCK/PENDING**，不是失败，也不是 PASS。

补充：旧 head 的 extras 曾有三项 funding 失败；新 base 已推进 #1621 修复，当前 head 的 `rust-test-s-session-extras` 已实际通过，没有沿用旧“基线失败”措辞。旧 head 的完整 S 与 macOS 结果只作历史参考，不能替代当前候选证据。

## 6. 剩余门

1. 修复并回归验证第 3 节的 v2/v3 外部回执记录版本戳缺口；若产生新 head，需重新核 head 与相应 diff。
2. 等当前精确候选的 shard 0 到终态；全部适用检查通过后才能把 CI 单独改为 PASS。即使 CI 全绿，代码/合同仍因 MEDIUM 保持 BLOCK。
3. 当前没有新 head 的 macOS 实跑证据，`macos-session` 仍为 skipped；不得引用旧 head macOS 结果宣称本候选已覆盖。
4. P0 尚缺 tick 与三项成本签署，继续按 `AwaitingPolicy` 停止；数值签署属于票据声明的后续范围，不要求本票伪填。

**最终结论：CRITICAL 0 / HIGH 0 / MEDIUM 1 / LOW 0；代码与合同 BLOCK，CI 暂 BLOCK。未合 main，未关票。**
