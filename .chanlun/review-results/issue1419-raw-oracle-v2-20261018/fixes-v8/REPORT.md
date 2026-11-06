# #1419 raw oracle v8：seq_range.to 必须等于 input_frontier（R7-1）

工作草稿，活期绑定 #1419。基线 `22157e9220`（v7）。作者 /root/owner_1419b（接续负责人），修复 raw-v7 独立审查（`/Volumes/AgentStorage/issue1419-sandcastle-manager-20261018/raw-v7-review/`）的 R7-1（HIGH）。

- `driver/fixed_cut_driver.py`：`_check_cut` 增加 `seq_range.to == input_frontier`，对应 `s_session/browser/q3-consumer.js` fixed 校验（1969）；所有公共入口（snapshot/content/call/bind/watch 观测头与接纳切面）共用此方法。watch 循环对每个 delta 同样校验，对应 q3 `validateDelta`（1566-1569）。
- `fixes-v6/test_v6.py` 新测试 `test_v8_seq_range_to_equals_input_frontier`：delta、观测头、接纳切面三处同步篡改为 `999` 与 `-1` 均拒且状态不动；仅篡改 delta 由 watch 循环内校验拒；追平空页的观测头单独篡改由 `_check_cut` 拒、watch 状态不动。
- 旧测试中只改 `input_frontier` 的夹具（同代冲突、历史 138 代、观测头/接纳切面前沿锁）同步改 `seq_range.to`，使它们继续命中原意图的判据（同代冲突、`watch与接纳切面不符`），而不是被新判据提前拦下。`seq_range` 身份锁变异改为 `{8,139}`，继续命中 `watch与接纳切面不符:seq_range`。

验证：`fixes-v6/run_checks.py v8` 纯 Python 离线 24 tests / 0 failures / 0 errors / 178 observations。复跑审查者探针 `probe_v7.py`（副本 `fixes-v8/probe_v7.reference.py`）：P1/P1b 由 ACCEPTED 变为 REJECTED，P2–P5 结果不变（`fixes-v8/probe_v8.out`）。

R7-2（LOW，q3 对空批重取快照比对 projection，driver 未做）登记为后续，仍属 Delta/Snapshot UNVERIFIED 范围。产品、Rust、正式 S3/Q、浏览器全部 NOT_RUN。包级 MANIFEST.json / SHA256SUMS 未重封，以本节哈希为准：

| 文件 | SHA256 |
|---|---|
| driver/fixed_cut_driver.py | 3447351701da23f5b0af5864fbefd5c97906802bbd959c484cf3600334b9903f |
| fixes-v6/test_v6.py | 6856e04d99970564aea6a4f462253ab0e716c15dce86c475c27d861eaffb101f |
| fixes-v6/tests.v8.stdout | 516b951eec1d7f4df02262252a2186fb3e85dae8285cd80d78ba6796f14cfad6 |
| fixes-v6/v8-observations.json | cd6b634733426e389c4c920bfba9275465e05f412ce8e7af5faeaa1e3b42fcff |
| fixes-v8/probe_v8.out | b9cb72633b742dc9585539061978ff8018d9a4e55b39b28061e999eca89972f8 |

需要独立复审后才可接纳；作者不自审。
