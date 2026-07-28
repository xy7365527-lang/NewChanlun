# #69 实装收尾 Spec 轴评审

**结论：FAIL**

## 票体验证协议

- 票体原文“`cargo test` 全绿”：**未达标**。Debug/Release 均 `rc=101`，分别 1980/4、1983/1（`/tmp/wt69-5b-post-test-status.txt:1-2`；`/tmp/wt69-5b-post-debug.log:2584`、`/tmp/wt69-5b-post-release.log:2567`）。
- 票体原文“250k/1M diff=0”：**未能判定（整体 e1e3aa7037..e8d5a47f06）**。两段各自 pre/post 的 dump/stdout 均 `cmp=0`（`/tmp/wt69-5b-250k-cmp.txt:1-2`、`/tmp/wt69-5b-1m-cmp.txt:1-2`）；但 5a 输入为 250001/1000001 bars（`/tmp/wt69-5a-pre-250k/stdout.txt:1`、`/tmp/wt69-5a-pre-1m/stdout.txt:1`），5b 为 250000/1000000 bars（`/tmp/wt69-5b-pre-250k.stdout:1`、`/tmp/wt69-5b-pre-1m.stdout:1`），且 5b-pre 已含 5a，缺同口径 base→final 直连证据。
- 票体原文“`P123_SHADOW=1` 0 mismatch”：**达标**。`shadow_checks=27037 shadow_mismatches=0 shadow_term_mismatches=0`（`/tmp/wt69-5b-shadow-1m.stderr:3`）。
- 票体原文“在线 pending/views 计数一致”：**达标**。pre/post/shadow 在 500k 均为 `pending=2218/4091 views=8198`（`/tmp/wt69-5b-{pre,post,shadow}-1m.stderr:2`）。

## 实装与授权面

卡原文要求 5a“dirty 调用传 resident store；forced 调用传 `None`”、5b“dirty 路径传 `Some`；forced 路径传 `None`”（5a 卡 `:278`；5b 卡 `:325`）。实装吻合 R1–R3：store 在 bin `LevelDerived` 旁（`rust/src/bin/p123_fast_replay.rs:383`），memo 在 `RunEntry` 旁（`:398`），dirty 两个 `Some`、forced 两个 `None`（`:988-1050`）；旧入口委托 `None`（`rust/src/level_view.rs:1814-1820`）。抽核 `p86_anchor_hypothesis.rs:534`、`p124_shard_replay.rs:1047`、`admission.rs:675` 三处旧调用相对基线零 diff。e_src 单式（`rust/src/signal.rs:1134-1141`）、两后继块门（`rust/src/level_view.rs:1038-1049`），未增 TowerCache resident 状态。TDD 红绿日志覆盖 5a T0–T5、5b T0–T6；集成切片未闭合。六个改动文件均属两卡授权面，**未发现越面**。

## 越面／欠面

- **HIGH 欠面**：两卡原文均规定“只有同时满足以下条件才可声明完成”，且“该豁免不覆盖 5a/5b”；全绿硬门及 wave-1+5a/5b 全量双跑封印均未完成（`wave1-plan5a-implementation-card-20260719.md:293-307`；`wave1-plan5b-implementation-card-20260719.md:338-353`；`/tmp/wt-69-dispatch-5b.out:396386-396387`）。
- **MED 证据欠面**：整体两提交缺统一 bars 口径的 250k/1M base→final 对拍。
- **LOW 欠面**：5a 卡原文“记录 Confirmed/TerminalFalse/Scanning 新实测计数”（`wave1-plan5a-implementation-card-20260719.md:288`）未交付；`SparseStats` 无三态计数（`rust/src/bin/p123_fast_replay.rs:431-456`）。
- **LOW 照实漂移**：抽核“基线红”“封印未跑”“5b 最终仅四文件”均属实；但报告引用 `scene-ledger:16`（`/tmp/wt-69-dispatch-5b.out:396386`）已漂移，现行锚为 `.chanlun/scene-ledger.md:18`。“提交门通过”（dispatch-5b `:396228`）不等于票面验收门通过。
