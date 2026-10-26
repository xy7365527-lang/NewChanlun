# 收据 workers/accept-s2/REPORT.md

- 原件：`workers/accept-s2/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`ac363bf5c72892af1295d2301b8a5215db84adc2997669955cb414f79da428b0`
- 字节数：15369；行数：139

## 关键行（行号: 原文）

```
3: # #1488 S2：MIN12 oracle 重建报告（D2）
8: ## 0. 结论速览
10: | 项 | 结论 |
12: | 输入固定 | 52 行 `input/messages.jsonl` sha256 `d61371c9…c6cd`，`input/clock.json` `779dcb85…fb36f`，与归档原件一致 |
13: | 二进制 | main `6badaa6049`，`--features s_session`，WSL 构建，sha256 `c944bd6bed40dd7e75332d66851ad050178b7ccb31860c1068d749de2726e040`（放 G:，不入仓） |
16: | oracle | g=1..52 逐代 AsKnown 全量公共切面；每代服务端 `cut_projection_digest` = 本地复算 = 落盘文件 sha256（104/104 一致） |
17: | 双跑 | run-a / run-b 新进程新库：52 个 oracle 文件逐字节相同，`STRUCTURAL.jsonl` 相同，最终库 sha256 相同（`4f185399…dedc`） |
19: | FORMED seed / EXTENDING 中枢 | **main 不产出**：main `6badaa6049` 没有 center-seed/TB-02-E 代码（`tb02e.rs`、`center_scan.rs` 只在 `codex/1419-center-seed-20261013` 等分支和 `48e2386463` 上），g1..52 公共切面中心族对象数为 0。A③「第 51 条起出现 FORMED seed 与 EXTENDING 中枢」**无法在 main 上满足**，这是缺口，需要主控裁定（§4） |
22: ## 1. 输入与旧读数（入仓）
24: | 文件 | sha256 | 说明 |
35: ## 2. oracle 生成
37: ### 2.1 方法
39: - 正式入口：`s_session/launch_s.sh service start`（s-launcher/2，S 常驻写者 + Q 只读）。每跑使用新工作目录、新库、新进程（`~/nc-remote/accept-s2/run-<label>`，ext4），跑完整目录拷到 `G:/AgentStorage/1488-accept/s2/run-<label>/` 并写 `SHA256SUMS`。
41: - 公共查询：摄入完成后，对 g=1..52 逐代 `POST /api/v2/snapshot`（AsKnown、`as_of_generation=g`、CompleteCut、`page_size=31`），用 `snapshot_token` 翻完所有页，分页间切面摘要必须不变。拼成 projection（fixed_cut/catalog/counts/scope/omissions/order_version/rows），用 canonical 方式落盘，本地 sha256 必须等于服务端 `cut_projection_digest`。
43: ### 2.2 产物
47: | `oracle/READINGS.jsonl` | 每代结构读数摘要：笔（起止 raw 锚、价、state）、段（方向、起点、几何终点、result、形成代/终止代）、种子、segment_version 等待理由、中心族清单 |
48: | `runs/run-{a,b}/` | RUN.json、INGEST.jsonl（逐条耗时与回执）、ORACLE-INDEX.jsonl、SHA256SUMS、config.json |
50: | G: `run-a/STRUCTURAL.jsonl` | sha256 `eb75995a4ee9745674ef7216c1a83cdaa2e0b41b35ab968d5a432939f2997761`（17.6 MB，两跑相同） |
61: ### 2.3 双跑一致性
62: - 52 个 oracle 文件逐个 sha256 相同；ORACLE-INDEX 中 digest、bytes、pages、counts、kind_counts、rule_revision、structure_cut 逐代相同（只有 `query_ms` 不同）。
63: - `STRUCTURAL.jsonl`、`READINGS.jsonl` 相同；最终 `session.sqlite` sha256 两跑相同。
66: ### 2.4 结构读数（main，new-bi/2）
73: ### 2.5 回复耗时（只做记录，10 s 帽的判定留给 S6/A③）
79: ## 3. 与旧 known-15 / known-31 逐项对照
101: ## 4. 缺口：FORMED seed / EXTENDING 中枢（A③）
106: - 需要主控裁定（三选一）：
112: ## 5. R3 第 46 条台阶离线预判
122: ## 6. 偏差与未决
128: 5. 独审待办（D2）：§3 对照表与 §4 缺口的结论都需要独立审读。审读入口：`compare/semdiff-known*.json`、`tools/semdiff.py`、`oracle/READINGS.jsonl`。
130: ## 7. 复现
133: # 二进制（WSL）：git checkout 6badaa6049 && cargo build --release --features s_session --bin s_structure_session
134: # 运行（WSL，ref 已 fetch 到 NewChanlun）：
136: # 对照：
```
