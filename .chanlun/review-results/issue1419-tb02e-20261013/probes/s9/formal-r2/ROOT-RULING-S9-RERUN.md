# 根裁定：#1419 S9 首轮失败后的处置（2026-10-14 11:55）

首轮（tb1419_s9_formal）FAIL：min12_up-a 的 S 摄入、公共查询、回访、5 个冻结切面与 27/43 恢复对拍全过，浏览器步页面取 cut 43 的第 2 个分页请求落到冷 Q worker，5.174 s 后 503 QueryBudgetExceeded（verify_deadline_ms=5000，`tests/fixtures/tb01c/query-resources.json:8`），页面 10 s 提交等待超时；其余三臂未启动。F1a 探针已记冷 worker 首次 verify 4.5 s（余量约 0.5 s）；本轮背景负载 9.6–18.1（28 核），含 Spotlight mds_stores 100–288%（推断在索引写入外盘的 8.8 GB 收据）、CLIProxy 最高 166%（并行代理流量）。

裁定：
1. 帽与期限一律不放宽，harness 与页面脚本不改。
2. 只在受控环境下重跑一次（第二轮）：大件输出写 `/Volumes/AgentStorage/issue1419-s9-formal-r2-20261014.noindex/`（`.noindex` 后缀目录不被 Spotlight 索引）；开跑前确认 mds_stores 已回落、无 cargo/rustc/lake/S/Q/计时探针/浏览器验收进程、根不在运行期间并行派发模型重负载工位；用户自用 Chrome 与 agent-browser 无头 Chrome 不属本图，不动，照实记负载。
3. 第二轮仍在同一冷 verify 点失败，则判为性能缺陷而非环境噪声：Q 冷 verify 在 g=43 已贴 5 s 期限，与 #1488（S 写者构造降阶）/ S0b 的超线性同源。届时 #1419 以首轮已过的 S/Q/恢复/冻结账证据为准，浏览器臂与 a/b 对拍登记为挂 #1488 的残留义务，另报根定是否需要独立的 Q 冷 verify 降阶票，不在 #1419 内放宽。
