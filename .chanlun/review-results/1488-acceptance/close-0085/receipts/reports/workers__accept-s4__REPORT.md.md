# 收据 workers/accept-s4/REPORT.md

- 原件：`workers/accept-s4/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`abae8269edca4827c662aa389dafee2ac138723cec642c8681eab2ea5cc169bc`
- 字节数：16129；行数：183

## 关键行（行号: 原文）

```
1: # S4 REPORT：阶段读量/Δ_R 计数器 + 整代 增量≡冷全量 对拍锁（#1488，主控 D4/D6，#799/#804）
3: - 分支 `codex/1488-accept-s4-20261020`，起点合并面 `c2bdf9b662`，交付 HEAD 见文末。只追加、未 push、未合 main。
4: - 证据口径：本报告**所有结论性运行**（锁、全部负控、冒烟）都在 15:38 新规之后用私有 target `NC_TARGET=target-s4` 取得（日志首行 `target=/home/hanju/nc-remote/target-s4`），每次都确认编译了本 SHA（`Compiling newchan_rust`）。15:38 前在共享 target 的并发窗口里拿到的结果（旧锁 a2942581cb、旧 N1/N2/N3a/N3b/N3c）只作历史参考，标注为“并发窗口取得，待私有 target 复跑”；除 N3c 探针外，都已在私有 target 上复跑，结论一致（见 §
6: ## 1. testonly 计数器（D4/D6）
25: ## 2. 整代 增量≡冷全量 对拍锁
49: - **结果（私有 target）**：`a89996a487` PASS，28 代，1032.7 s（日志 `/tmp/nc-remote/20261020-172553-a89996a487.log`，JSON `/mnt/g/AgentStorage/1488-accept/s4/lock-a89996a487.json`）。
54: ## 3. 每阶段变异负控（在私有 target 上复跑）
56: 每个变异都是单独 commit，紧跟一个 revert commit，HEAD 与锁 SHA 同树。
67: ### 来源审计（prepare_source_audit）的覆盖面
80: ### 3.1 N3h 逐代来源审计 trace（`12c0dc8a87`，revert `ba83f5fc60`；eprintln，不改判定，`--nocapture`）
95: 结论：
106: ## 4. D6 数据：增量臂读量随 N 线性增长，不随 Δ_R 增长
127: - **冒烟**：测量测试 `s4_stage_measurement_from_bars`，60 根随机 bars，每代 1 根，`S4_COLD=1`，PASS，2462 s，JSON `/mnt/g/AgentStorage/1488-accept/s4/smoke-a89996a487.json`。Δ_R 约 0.9k–1.4k，几乎是常数，但增量读量从 39k 线性涨到 532k，delta/full ≈ 0.49。和上面的结论一致。
130: ## 5. #799 审定：delta=None 完整路径参与准入，不只是离线对照臂
132: 结论：**完整重算（delta=None）路径参与生产准入和发布，而且是生产构建里唯一的生成路径。只作 testonly 对照臂的，是增量（delta）路径。** 生产行为没有改动，交负责人。
146: ## 6. 写集偏离（如实报告）
150: - 负控和探针临时改过 `source_contributions.rs`、`generation.rs`、`aux_index`、`object_index`、`s_session_v2/content_manifest.rs`，**每一处都有对应的 revert commit**。HEAD 与锁 SHA 同树，相对起点只有 §1/§2 的测试与计数改动。
153: ## 7. S5 用法
156: # 锁 + 逐代 JSON（release）：
160: # 任意 bars 测量（ignored）：S4_BARS=<[[o,h,l,c],...] JSON> [S4_GENERATIONS=n] [S4_COLD=1] [S4_CACHE_BYTES=...]
167: ## 8. 交付
169: - **HEAD `ba83f5fc60`**，与锁 PASS 的 SHA `a89996a487` 同树。相对起点 `c2bdf9b662` 的净改动：
```
