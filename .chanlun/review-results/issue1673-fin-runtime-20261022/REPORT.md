# #1673 TB-03-C/S5 正式会话入口合链运行与 Q 公共查询核对 — 运行报告

状态：**PASS**（covered 78 项断言、uncovered 13 项断言，cleanup_errors 为空）。

## 运行方式

- 驱动：`s_session/tests/tb03c_fin_runtime.py`（仅调度；不引用、不修改冻结驱动 `tb03c_query_runtime.py`）。
- 契约单测：`s_session/tests/test_tb03c_fin_runtime_contract.py`（锁 8 个输入 hash、断言集合 sha256、手算 oracle 复算、文档构造、时钟序、信任边界声明；CI 步骤 “TB-03-C formal-session chain driver contract (#1673)”）。
- 实跑经 nc-remote（WSL，`cargo build --locked --features s_session --bin s_structure_session --bin e_economic_session --bin b_chong_session`），本地未调用 cargo/rustc：

```
NC_REPO="$(git rev-parse --show-toplevel)" NC_TARGET=sandcastle-issue-1673 ~/.local/bin/nc-remote sandcastle/issue-1673 rust \
  bash -c 'python3 -B ../s_session/tests/tb03c_fin_runtime.py --binaries $CARGO_TARGET_DIR/debug --output /tmp/f1673-out-$$/run --emit-archive'
```

- 出处：`run/provenance.json`（源 commit 51ca01653b 为证据入库前的同一驱动；`driver_sha256` 与入库驱动逐字节一致，二进制 sha256 同记）。

## 链路与核对（covered）

真实启动 `s_structure_session`（s-launcher/2）、`e_economic_session`、`b_chong_session`；manifest 携 `tb03b-finance/1` + `book_revision: tb03c-book/1`；全部写入经 Unix socket：

Prepare → Reserve → Commit → Capture → Resolve → Apply（含缺口、冲突、他证据、错 hash、估值负例）。

| 量 | 手算 oracle | 实测（Q 公共 HTTP） |
|---|---|---|
| 锁定额 | 30 | 30 |
| 已保留责任总额 | 50 | 50 |
| 最坏数量 | 6 | 6 |
| 分配 units/acquisition_cash/commission | 2/26/2 | 2/26/2 |
| K_before → K_after | 80000 → 80028 | 80000 → 80028 |
| units_after | 6 | 6 |
| cost_remaining（含估值后） | 80028 | 80028 |

- Commit / Capture / Resolve / Apply 收据与 B 像经 Q `/api/economic/read`（economic-read-pages/1 分页 snapshot/watch）与 `/api/economic/original` 逐值等于 owner 实读与 S1 终态；watch 5 段首尾相接覆盖 E 7 / B 5 个持久提交，每个 change 与 store `commits` 行逐值一致。
- 重送（Capture、Resolve、Apply、Commit）只回原收据；关停 → 正式 `recover` → 重开后公共查询逐值相同（仅剔除合法变化的 `writer_epoch` 与 `runtime.pid`），Q 重启 snapshot 同 cut 同像，无新增提交。
- 计划 `s_ref` 绑定本次 S financial-basis 实读对象（hash 复核）。

## 未覆盖事实（uncovered）

Resolve 经已保留责任 `covered_facts` 核对：未覆盖事实被拒（`MissingDependency…未被已核责任界覆盖`），E 持久表逐表不变（`resolve.uncovered_unchanged`；Resolve 只写 E，本场景未另比 B 表）。

## 信任边界（如实声明）

- **Commit↔成交对应仍由夹具构造**（成交预列于冻结 FIN baseline、`command_id` 夹具填 `plan-alpha`）；**产品绑定 F1/F2 归 TB-04-F #1425，未认定存在**。
- 未签政策保持 AwaitingPolicy/NotEvaluable：`principal`、`growth_enabled` 观测为 NotEvaluable；无默认数量/费用/资金（ADR 0041 裁定二）。
- 声音为单位信任夹具，非真实 γ 独评。
- S 输入为 tb01c 冻结前缀 op-0000..op-0002（首个结构对象出现处）：每条 B 决策/历史项内嵌完整 S financial-basis 快照，95 op 全量会使 B ReadView 超 16MB 帧。
- Q 公共 HTTP 单回复硬帽 1MB，B 像约 0.9MB，故完整文档走正式分页端点（聚合 ≤4MB）。
- 浏览器不在本票（#1460）。
- 冻结件（tb03c-book/1、Legacy 原样重放、既有 tb03c `FROZEN-MANIFEST.json`/`RAW.json`/`HAND-ORACLE.json`）未改；新夹具在 `s_session/tests/fixtures/tb03c_fin_runtime/`。

## 证据清单

- `run/`：完整归档中 <100KB 的文件（RESULT.json、provenance.json、commands.jsonl、各请求/响应、manifest、FIN 输入、S 启动/basis）；回环地址已脱敏为 `<loopback-v4>`。
- `ARCHIVE-SHA256SUMS`：完整归档（144 个文件，约 19MB）全部文件的 sha256（路径相对 `run/`：`cd run && shasum -a 256 -c ../ARCHIVE-SHA256SUMS`，入库的 116 个文件应全部 OK）；未入库的大件为 `covered/messages.jsonl`（约 9.9MB）、分页页响应（约 350KB/页）与分页汇总文档（0.6–1MB），可按上述命令复现。
