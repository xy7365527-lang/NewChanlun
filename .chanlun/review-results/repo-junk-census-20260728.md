# 仓内杂物逐项盘点（issue #508 数据供给）

- 盘点时间：2026-07-28
- 盘点人：只读盘点 subagent（Kimi）
- 口径：严格只读，未删未改任何对象
- 工作区状态：branch main，128 个 dirty 文件（与本次盘点对象无交集）

| 对象 | 大小 | 文件数 | git 状态 | 最后修改 | 处置建议 |
|---|---|---|---|---|---|
| `G:/` | 4.0K | 1 | 未跟踪（被 `.gitignore:54 tmp/` 间接屏蔽） | 2026-06-27 | 删（零风险） |
| `kimi-export-session_-20260721-173955.md` | 1.1M | 1 | untracked | 2026-07-21 | 挪出仓 |
| `.git.bak-2026-04-27/` | 1.6G | 759 | 未跟踪（`.git/info/exclude` 排除） | 2026-04-26 | 挪出仓或删 |
| `.playwright-mcp/` | 132K | 8 | 2 个旧 log 已跟踪，其余 untracked | 2026-06-04 | 删（需先 `git rm` 2 个跟踪文件） |
| `p7_inputs/` | 23M | 1 | untracked | 2026-07-16 | 留（被 rust bin 引用） |
| `prototypes/` | 152M | 1885 | 25 tracked + 2 untracked | 2026-07-26 | 留（活跃原型，ADR 引用） |
| `.pytest_cache/` | 960K | 6 | 未跟踪（gitignore:38） | 2026-04-23 | 删（零风险） |
| `.rtas/` | 292K | 13 | 全部 tracked | 2026-06-27 | 留（历史评审记录） |
| `.auto-memory-snapshot/` | 204K | 48 | 全部 tracked | 2026-06-20 | 留（记忆快照，continual-learning 体系） |
| `.cache/` | 9.6G | 173 | 未跟踪（gitignore:37） | 2026-06-22 | 留但建议挪出仓（活跃数据缓存） |

---

## 逐项明细

### 1. `G:/` — 删（零风险）
- 内容：单个文件 `G:/NewChanlun/tmp/auto_ceremony.log`，是 `scripts/auto_ceremony.sh:17` 的 bug 产物——脚本写死 Windows 路径 `PROJECT_ROOT="G:/NewChanlun"`，在 macOS 上被当成字面目录名创建，日志里全是 `can't open file .../G:/NewChanlun/G:/NewChanlun/scripts/ceremony_scan.py` 的路径叠加报错。
- git：目录本身不出现在 `git status`（唯一文件被 `tmp/` 规则 ignore）。
- 引用：grep `G:/NewChanlun` 仅命中 `scripts/auto_ceremony.sh`（源 bug）与历史 review 记录（非依赖）。
- **注意**：删目录零风险，但 `scripts/auto_ceremony.sh:17` 的 bug 仍在，下次运行会再生成。处置应连同修脚本（`PROJECT_ROOT` 改为 `$(cd "$(dirname "$0")/.." && pwd)` 之类）。

### 2. `kimi-export-session_-20260721-173955.md` — 挪出仓
- 内容：Kimi 会话导出（138 turns / 342 tool calls / 72 万 token 的聊天记录），属会话档案不是仓内容物。
- git：untracked，出现在 `git status` 脏清单。
- 引用：仓内零引用。

### 3. `.git.bak-2026-04-27/` — 挪出仓或删
- 内容：整个 `.git` 的 1.6G 备份（759 文件），源自 519 号凭证泄露事件的清理备份（`.git/info/exclude` 注释与 `.chanlun/genealogy/settled/519-*.md` 在案）。
- git：被 info/exclude 排除，不跟踪。
- 已过 3 个月，主仓健康；若主仓历史已验证完整，可删；保守做法是挪到仓外冷存储。**注意该备份可能仍含 519 要清除的凭证——删反而是合规动作**（由 Lead 裁定）。

### 4. `.playwright-mcp/` — 删
- 内容：Playwright MCP 的 console log / page yml 调试残留（3 月与 6 月两批）。
- git：2 个 3 月的 console log 被 tracked（历史误提交），其余 6 个 untracked；`.gitignore` 里有 `playwright-mcp` 字样但未覆盖已跟踪文件。
- 引用：仓内零引用（仅 .gitignore 命中）。删除需 `git rm` 那 2 个文件。

### 5. `p7_inputs/` — 留
- 内容：`trades.jsonl`（23M 交易回放数据）。
- 引用：`rust/src/bin/strict_nest_check.rs:1301` 以 `data_root.join("p7_inputs/trades.jsonl")` 引用；多份 STRICT-NEST 结果文档提到。是测试/校验输入数据，不是杂物。
- 可选优化：挪入 `.cache/` 或数据目录统一 ignore 管理，但非本票范围。

### 6. `prototypes/` — 留
- 内容：三个原型工程（no-gate-two-books 的 sim.py；task68-ledger-adopt、task72-gap-connector 两个 Rust 原型，含 target/ 构建产物，故 152M）。
- git：25 个源文件 tracked，2 个 untracked。
- 引用：`docs/adr/0001` 及 p68/gap-connector 实施文档引用。活跃工作面。
- 可选优化：清 `target/`（约大部分体积）可省 ~百 M，但属构建产物常规清理。

### 7. `.pytest_cache/` — 删（零风险）
- 内容：pytest 标准缓存（v/ 目录 + CACHEDIR.TAG），最后修改 2026-04-23，三个月没跑过 pytest。
- git：gitignore:38 已排除。纯再生产物，删后 pytest 自动重建。

### 8. `.rtas/` — 留
- 内容：13 个文件 = 6 月下旬 codex/gemini 影子评审结果（review-results/）+ `.ceremony-in-progress` 标志位。
- git：全部 tracked；`.chanlun/specs/` 三份文档与谱系 551 引用该目录（RTAS 是仪式状态目录）。
- 292K，是蜂群方法论的一部分，非杂物。

### 9. `.auto-memory-snapshot/` — 留
- 内容：48 个记忆快照 markdown（MEMORY.md + feedback_*/project_* 系列），是 continual-learning 体系的持久记忆；AGENTS.md「记忆」一节即源于此体系。
- git：全部 tracked；被 chanlun/review-results 引用。非杂物。

### 10. `.cache/` — 留，建议中期挪出仓
- 内容：9.6G 行情数据缓存——btcb_catalog 8.8G、golden_20k.txt 670M、dbn 90M、BZ/GC parquet 系列。
- git：gitignore:37 排除。
- 引用：`scripts/regression_equiv.py`（golden 文件默认路径）、`scripts/profile_incremental.py`、`rust/src/bin/p119_l5_decompose.rs` 等多个 rust bin 使用。是活跃数据缓存，删了要重新拉数。
- 建议：9.6G 放仓目录下拖慢一切全仓扫描/备份，中期宜挪仓外 + 环境变量指向；短期保留。

---

## 结论

- **零风险可删**：`G:/`（4K，bug 产物）、`.pytest_cache/`（960K，可再生产物）、`.playwright-mcp/`（132K，调试残留，需 git rm 2 文件）。
- **挪出仓**：`kimi-export`（1.1M 会话档案）、`.git.bak`（1.6G 备份，且涉 519 凭证合规，需 Lead 裁定删或冷存）。
- **保留**：`p7_inputs/`、`prototypes/`、`.rtas/`、`.auto-memory-snapshot/`、`.cache/`（均有活跃引用）。
- **连带事项**：`scripts/auto_ceremony.sh:17` 的 `G:/NewChanlun` 硬编码 bug 须修，否则 `G:/` 删了还会再生。
