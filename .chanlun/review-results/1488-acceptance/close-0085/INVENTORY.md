# #1488 收据清单（INVENTORY，0085 / D10）

收据分三层：仓内文本收据、分支驻留收据、仓外大件。仓外大件只登记哈希和位置，不入仓。

## 1. 仓内（squash 面上随提交带入）

| 位置 | 来源 | 规模 | 说明 |
|---|---|---|---|
| `.chanlun/review-results/1488-acceptance/receipts/` | 合并面已有 | 78 文件，476K | 各轮作者与独审收据 |
| `.chanlun/review-results/1488-acceptance/degrade-0065/` | 合并面已有 | 28 文件，596K | 0065 降级证据 |
| `.chanlun/review-results/1488-acceptance/close-0085/receipts/reports/` | 0085 新增 | 140 份 | 工位 REPORT/REVIEW/RESULT/MAP 等。保留关键行，附整文件 sha256 |
| `…/close-0085/receipts/logs/` | 0085 新增 | 188 份 | nc-remote 日志摘录。另有 55 个 run-id 已在 receipts/ 或 accept 分支上，不重复收录；本机原件缺失 1 个（`20261021-161555-56ddd8d86f`，被 premerge-0086 REPORT 引用，原件只在 WSL `~/nc-remote/logs/`），INDEX 末尾单列 |
| `…/close-0085/receipts/ci/` | 0085 新增 | 40 份 | uo-ci 与 runner 的 CI 运行文件清单（含 squash 面三轮 run 39609217753、39640543315、39654630073）、locks.tsv 统计、非 PASS 行 |
| `…/close-0085/receipts/outputs/` | 0085 新增 | 59 份 | accept-s4 的 `*.out` 和 attempts.log，merge2-0083 的 ci-*.txt，close-0085 的 uo/wide/fin-main/s86 输出 |
| `…/close-0085/{REVIEW-CHAIN,CLOSE-DECLARATION,P3-STATUS,INVENTORY}.md` | 0085 | 4 | 关票文档 |

`close-0085/receipts/` 共 431 文件（含 4 个 INDEX.md），2.6M。每个子目录有 INDEX.md，生成器是 `workers/close-0085/make_receipts85.py`（仓外）。

## 2. 分支驻留收据（squash 时并入）

| 分支 | head | 目录 | 文件数 |
|---|---|---|---|
| `codex/1488-accept-s1-*` | `874b2cfbba` | `1488-acceptance/s5/`、`1488-acceptance/s86/` | 95 |
| `codex/1488-accept-s2-*` | `34c3251910` | `1488-acceptance/a5s6/`、`1488-acceptance/min12/` | 125 |
| `codex/1488-accept-s4-20261020` | `809710da97` | **已并入**（按 (i)，0086 合并面；0089 改 release-only）。S4 锁代码随合并面进 squash，工位证据收在 `receipts/outputs/` | — |

两条分支相对 `6badaa6049` 都只新增 `1488-acceptance/` 下的文件，与合并面不冲突。并入方式：`git checkout <head> -- <dir>`。

## 3. 仓外大件（只登记）

位置：`windows-5090` WSL `/mnt/g/AgentStorage/1488-accept/`。

### 3.1 目录

| 目录 | 大小 | 文件数 | 用途 |
|---|---|---|---|
| `baselines/` | 2.8G | 18403 | 输入、基线库、min12/s86/p9011 |
| `d65/` | 945M | 490 | 0065 降级臂（min12 arms） |
| `s2/` | 1.1G | 139 | S2 两轮 run-a/run-b |
| `s5/` | 633M | 185 | S5 p1/d1/d2 与 compare_s86.py |
| `a5s6/` | 409M | 220 | A5/S6 final1/2、main1/2、smoke1、pack |
| `s1/` | 322M | 26 | S1 r8 |
| `merge2-review-0084/` | 74M | 29 | 0084 复跑日志 |
| `r2p/` | 23M | 38 | r2 拟合 |
| `s2-fix-0056/` | 12M | 5 | 0056 churn 摘要 |
| `merge2-0083/` | 836K | 11 | 0083 合并面校验日志 |
| `s4/` | 508K | 4 | S4 锁工作站证据 |

### 3.2 二进制

| sha256 | 字节 | 路径 |
|---|---|---|
| `c944bd6bed40dd7e75332d66851ad050178b7ccb31860c1068d749de2726e040` | 11883624 | `s1/bin/s_structure_session-6badaa6049`（与 `s2/bin/` 同件，main 基线） |
| `d2f96f044e2e4d765375d12c79d67cb480888550ddcf58b8cf68052e6333c22e` | 12754072 | `s5/bin/s_structure_session-prod-3ee05a2713`（与 `a5s6/bin/s_structure_session-3ee05a2713` 同件） |
| `4d141d8fc4c9621efd2591c04d6b5a9de497fb1dfa6f6aa090f1d5bf41049882` | 15686608 | `s5/bin/s_structure_session-testonly-3ee05a2713` |
| `65eae4632fbf25375e9f06e740caa85cd8cacf365b8164713d0732a33a1708bd` | 15690488 | `s5/bin/s_structure_session-diag2-c081bfdeee` |

### 3.3 清单与关键 JSON（sha256 前 16 位）

| sha256 | 路径 | 行数/说明 |
|---|---|---|
| `f0811a5a04d67e1a` | `a5s6/final1/SHA256SUMS`（`pack/` 同件） | 22 |
| `c243850ae6e418d0` | `a5s6/final2/SHA256SUMS`（同上） | 26 |
| `2ce7c37564ed5f42` | `a5s6/main1/SHA256SUMS`（同上） | 26 |
| `c211e80967348da1` | `a5s6/main2/SHA256SUMS`（同上） | 23 |
| `d596001d53b4c585` | `a5s6/smoke1/SHA256SUMS`（同上） | 22 |
| `f52362e317af3304` | `s2/run-a/SHA256SUMS` | 68 |
| `b0082668d835ca7e` | `s2/run-b/SHA256SUMS` | 68 |
| `c7bc872b0a6cad89` | `baselines/inputs/p9011/SHA256SUMS` | 6 |
| `f61b747f09946fbe` | `s4/lock-a89996a487.json` | S4 锁 PASS，28 代 |
| `d12e49b8a77463c4` | `s4/smoke-a89996a487.json` | |
| `2365db028e78975c` | `s4/lock-a2942581cb.json` | 前一次尝试 |
| `31c0e9624f32a664` | `s4/bars-smoke.json` | |
| `95a837cd476fec5e` | `s5/compare_s86.py` | 比较脚本（0068 曾独立复跑） |
| `78780ddab55bf2d3` | `merge2-0083/all-026e3e97b0.log` | 宽 parity 全量 |
| `64e7ea43d05a5270` | `merge2-0083/all-failed-026e3e97b0.txt` | 失败集 |
| `0556c5eb7fed12e5` | `merge2-0083/locks-026e3e97b0.log` | 全锁 |
| `c1e8790e0685102a` | `merge2-0083/bsp-026e3e97b0.log` | |

### 3.4 squash 面验证产物（`close-0085/`，577M，108 文件）

squash 面的验证产物在 `/mnt/g/AgentStorage/1488-accept/close-0085/`。第三轮验证提交为 `336b53db8d`，结论见 CLOSE-DECLARATION §8。哈希采自 log `20261022-063022-336b53db8d`。

| sha256（前 16 位） | 字节 | 路径 | 说明 |
|---|---|---|---|
| `2b82e38686e81b0c` | 10600736 | `rel-nobid-0fb96c00ae` | fmt 前（合并面）生产 nobid |
| `e4f7ee3b7792c3d4` | 10600736 | `rel-nobid-squash3` | fmt 后（squash）生产 nobid |
| `182176940deb56cf` | 12902144 | `rel-full-0fb96c00ae` | fmt 前生产全件 |
| `417d3bc313499b30` | 12902024 | `rel-full-squash3` | squash 生产全件（S86 p3 所用，同件拷至 `~/nc-remote/1488-accept/close-0085/bin/s_structure_session-prod-336b53db8d`） |
| `032ba48352b4ca5a` | 146969 | `fmt2.patch` | P58 全量 fmt 补丁（29 文件 +1082/−336） |
| `e5bed7e3869fdf47` | 4541 | `nobid_classify.py` | nobid 差异分类器 |
| `9dfc552f3b2978f0` | 135820 | `all-336b53db8d.log` | 宽对拍全量（log `20261022-034338-336b53db8d`） |
| `4c4fbcb9887a7583` | 97554 | `all-results-336b53db8d.txt` | 逐测试结果 |
| `3f7454da80251228` | 1777 | `all-failed-336b53db8d.txt` | 失败集 22 条（与 `fin-failed-336b53db8d.txt` 同件） |
| `e6902f2ea2154ffc` | 75245 | `wide-336b53db8d.txt` | 宽子集 753 条 |
| `19efb3392be82bad` | 13572 | `fin-336b53db8d.log` | squash financial 组 |
| `c428c7013bfc424c` | 13572 | `fin-main-66c409e7e3.log` | main financial 对照（log `20261022-044955-66c409e7e3`） |
| `0f167731cb0e6d17` | 5387 | `fin-pairs-336b53db8d.tsv` | 与 `fin-pairs-main-66c409e7e3.tsv` 同件 |
| `bc2759a1322f7eb2` | 13252 | `p3/cmp-s1-r8a.json` | S86 对 main r8a（log `20261022-045545-336b53db8d`） |
| `3c9db71ea641789d` | 11760 | `p3/cmp-s5-p1.json` | S86 对 s5/p1 |

`p3/` 是 S86 第三轮运行目录的副本（20 文件），运行原件在 `~/nc-remote/1488-accept/close-0085/p3`。CI 运行 39654630073（第三轮）、39640543315（第二轮）的原件在仓外 `workers/uo-ci-runs/`，39609217753（第一轮）在 `workers/close-0085/ci-runs/`；摘要都收在 §1 `receipts/ci/`。

前两轮的同类产物仍在同一目录（`*-dbfb13d90b`/`*-d5ae676bf9`/`p1/`，`*-5417eea488`/`*-5b5e264276`/`p2/`），结论已被第三轮取代，只作历史保留。第一轮哈希表如下（采自 log `20261022-001652-d5ae676bf9`）：

| sha256（前 16 位） | 字节 | 路径 | 说明 |
|---|---|---|---|
| `4a04cf72d6dd0eb0` | 10604000 | `rel-nobid-dbfb13d90b` | fmt 前生产 nobid |
| `4d92fc29796a1df4` | 10604000 | `rel-nobid-squash` | fmt 后（squash）生产 nobid |
| `f00a1f7a05a8e9ed` | 12906792 | `rel-full-dbfb13d90b` | fmt 前生产全件 |
| `30a2827314638ab7` | 12906760 | `rel-full-squash` | squash 生产全件（S86 所用，同件拷至 `~/nc-remote/1488-accept/close-0085/bin/`） |
| `6d53bd3a340deb6b` | 99666 | `fmt.patch` | P58 全量 fmt 补丁（15 文件 +699/-261） |
| `e5bed7e3869fdf47` | 4541 | `nobid_classify.py` | nobid 差异分类器 |
| `b6c31efcdf224164` | 190056 | `all-d5ae676bf9.log` | 宽对拍全量（log `20261021-220317-d5ae676bf9`） |
| `22593d8d470b4948` | 97475 | `all-results-d5ae676bf9.txt` | 逐测试结果 |
| `3f7454da80251228` | 1777 | `all-failed-d5ae676bf9.txt` | 失败集 22 条（与 `fin-failed-*` 同件） |
| `40ac80166980491c` | 75245 | `wide-d5ae676bf9.txt` | 宽子集 753 条 |
| `5c52f8f230fb9ea6` | 87371 | `fin-d5ae676bf9.log` | squash financial 组 |
| `afc4da0c7008505f` | 73482 | `fin-main-2df3eebec7.log` | main financial 对照（log `20261021-225132-2df3eebec7`） |
| `0f167731cb0e6d17` | 5387 | `fin-pairs-d5ae676bf9.tsv` | 与 `fin-pairs-main-2df3eebec7.tsv` 同件 |
| `7537191b55aaaf39` | — | `p1/cmp-s1-r8a.json` | S86 对 main r8a（log `20261021-225711-d5ae676bf9`） |
| `010cd2a5c6102eaf` | — | `p1/cmp-s5-p1.json` | S86 对 s5/p1 |


## 4. 本地 nc-remote 日志

本机 `/tmp/nc-remote/` 是易失目录。§1 `logs/` 已对 188 个 run 保留摘录和原件 sha256；原件不入仓。
