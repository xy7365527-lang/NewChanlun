# 收据 workers/accept-s4/chain-2.out

- 原件：`workers/accept-s4/chain-2.out`（仓外）
- sha256：`782ce6e5f3b20bb5da39bf0513a8fc1f4806c710cbb0ba0c58c35856acb30f97`
- 字节数：2599；行数：27
- 头行：`== n2 06c025e942 rc=101 test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 997 filtered out; finished in 1.49s`

## 关键行（行号: 原文）

```
1: == n2 06c025e942 rc=101 test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 997 filtered out; finished in 1.49s
2: 2167:thread 'v3::driver::tests::s4_generation_incremental_matches_cold_full_six_stages' (23516) panicked at src/bin/s_session_v3/driver.rs:5270:33:
4: error: test failed, to rerun pass `--bin s_structure_session`
5: EXIT=101 SECONDS=102
6: == n3d f4935d63f9 rc=0 test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 997 filtered out; finished in 1010.19s
7: == n4 d8f41f7691 rc=101 test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 997 filtered out; finished in 0.69s
8: 2112:thread 'v3::driver::tests::s4_generation_incremental_matches_cold_full_six_stages' (29081) panicked at src/bin/s_session_v3/driver.rs:5270:33:
10: error: test failed, to rerun pass `--bin s_structure_session`
11: EXIT=101 SECONDS=64
12: == n5 7926215e9c rc=101 test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 997 filtered out; finished in 0.21s
13: 2107:thread 'v3::driver::tests::s4_generation_incremental_matches_cold_full_six_stages' (29548) panicked at src/bin/s_session_v3/driver.rs:5270:33:
15: error: test failed, to rerun pass `--bin s_structure_session`
16: EXIT=101 SECONDS=69
17: == n6 e0d636ccc5 rc=101 test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 997 filtered out; finished in 0.21s
18: 2107:thread 'v3::driver::tests::s4_generation_incremental_matches_cold_full_six_stages' (29992) panicked at src/bin/s_session_v3/driver.rs:5270:33:
20: error: test failed, to rerun pass `--bin s_structure_session`
21: EXIT=101 SECONDS=68
22: == n3a 62fad470f7 rc=0 test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 997 filtered out; finished in 1002.60s
23: == n1 0bd73fb2ca rc=101 test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 997 filtered out; finished in 325.16s
24: 2170:thread 'v3::driver::tests::s4_generation_incremental_matches_cold_full_six_stages' (9506) panicked at src/bin/s_session_v3/driver.rs:5270:33:
26: error: test failed, to rerun pass `--bin s_structure_session`
27: EXIT=101 SECONDS=1458
```
