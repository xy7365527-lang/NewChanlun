# 收据 workers/probe-0064/PROBE.md

- 原件：`workers/probe-0064/PROBE.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`6a3bd1930c8aa3d52b16becd0aa4dd21f053671206f4da702c883a84b3f9986b`
- 字节数：10088；行数：96

## 关键行（行号: 原文）

```
1: # #1488 0064 错位探针 PROBE
3: - checkout `accept-s4-checkout`。诊断提交 `540b2141ce`（只在 `generation.rs` 加 `p64_probe`，eprintln），已由 `809710da97` revert；HEAD 与 `ba83f5fc60` 同树，生产代码未改。
4: - 运行：`NC_TARGET=target-p64`，私有 target，串行。S4 锁 `v3::driver::tests::s4_generation_incremental_matches_cold_full_six_stages --nocapture`，release，features `s_session,s_capture_native_testonly`。结果 **PASS**（985s，EXIT=0），说明探针没有扰动对拍。
7: ## 结论：证伪（次因部分成立）
15:    - 结论：全量置换的充分且必要原因是代戳，与插入位置无关。
24: ### 三种差分口径对照（摘自 table.md）
39: ## 证据日志（节选）
59: ## 对 P1 的含义（改写 ARCH ③.3 “P1 高风险白做”）
61: - P1 是必要的，不会被段 B 抵消：段 B 既不去代戳，也不改生成态数组次序。
68: ## 逐阶段判定表（PLAN §2）
78: | prepare_source_audit ① 对象差分 | **段 B 前做**（改成身份键差分） | 本探针主结论；段 B 不消除代戳与分块次序。去代戳本身登记 #1491 |
80: | prepare_source_audit ③ 只核脏对象（P6） | 等裁定；spec 用 object_id 坐标写 | ARCH 建议按出生序坐标写；但这条流在段 B 后仍不是出生序，用身份坐标写才与布局无关 |
92: ## 局限
```
