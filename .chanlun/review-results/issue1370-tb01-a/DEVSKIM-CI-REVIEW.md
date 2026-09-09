# TB-01-A / #1447 DevSkim 实际 CI 审查

**实际 CI 为 FAILURE；相对已审 W38a 新增的 2,614 个精确键已逐项分类为本次受测范围内的误报，未见新增真实代码缺陷。原有 8 键全部保留。** 此结论只覆盖本次 DevSkim 已报位置，不替代全仓安全审计或整票验收。

受审 head：`17e9afb6fbadcc3057da8f6b3b9f9df57e5b7358`；对照 head：`38a6b3f6f7d6118ffeb22c49c4a67a736be1ab92`。审查只写本报告及 P/evidence/PR1447-devskim，未改仓库、GitHub 或服务。

## 实际扫描与数量

[实际 run 34334331420](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34334331420)，job `102410063559`，于 `2026-09-09T09:24:11Z` 完成。镜像构建、扫描和 SARIF 上传均 success，findings gate failure。driver 为 `devskim 1.0.90+fb2d676ce4`。日志确认测试合并提交 `3bfa862c534c104e28cd775a60d9839c89c1fc09`，由指定 head 合入 `644075a6abcbfcda74815daed68e06eff6ddaf63`；最终源码位置均按指定 head 复核。

| 口径 | 数量 |
|---|---:|
| 当前 SARIF 原始结果 / 唯一精确键 | 8,231 / 8,200 |
| W38a SARIF 原始结果 / 唯一精确键 | 5,598 / 5,586 |
| 两份真实 SARIF 直接比较：新增 / 删除 | 2,614 / 0 |
| 原有 baseline / 本次 baseline 外告警 | 5,587 / 2,622 |
| 本票新增 / 原有保留 | 2,614 / 8 |

精确键沿用 gate 的 `ruleId|首 location 的 uri 去 file:// 前缀|startLine`，去重后比较。原 baseline 与 W38a 完全一致。gate 显示的 9 个“Fixed since baseline”在 W38a 已经缺席，本 PR 对比 W38a 删除 0，不能说本 PR 修了这 9 项。

## 新增键的分类

| 规则 | 唯一键 | 核实的内容 |
|---|---:|---|
| DS173237 | 2,564 | 源文件/工件/输入/profile 摘要与 Git 身份。全部匹配片段为引号内的 40 或 64 位十六进制；按字段语义复核。2,020 个源码摘要位置对应的 408 个唯一 Rust 文件已对冻结提交 `76a641606d70` 重算；其余来源绑定由逐键字段及原审查证据定位。 |
| DS117838 | 26 | 20 个 key/pass 文件名加源码摘要的位置已重算；6 个 TestOnly 输入身份、载荷摘要和回执记录。没有把这些业务身份键认成认证凭据。 |
| DS162092 | 24 | 19 个验收档案中的回环 HTTP/浏览器记录，及下面 5 个有意的本地入口位置。 |

其中 **2,608 键在本票验收档案，6 键在运行入口或目录文件**。逐键 `rule|path|line`、完整扫描片段、冻结源码行、源码 SHA256、分类及理由见 [classified-new-findings.json](/tmp/newchanlun-1323-publication-20260909/evidence/PR1447-devskim/classified-new-findings.json)；本报告 JSON 也逐键保留分类和来源指纹。

## 6 个运行入口或目录位置

| 位置 | 数据流与判断 |
|---|---|
| `s_session/launch_s.sh:234` | echo 打印本地浏览器 URL；不发网络请求。 |
| `s_session/launch_s.sh:235` | echo 打印同端口目录 API URL。 |
| `s_session/launch_s.sh:236` | echo 打印同端口快照 API URL。 |
| `s_session/launch_s.sh:237` | echo 打印同端口状态 API URL。 |
| `s_session/s_readonly_server.py:351` | `--host` 默认回环地址经 argparse 到第 355 行 `ThreadingHTTPServer((args.host,args.port), Handler)`。launcher 第 222 行不覆盖 host，故打印地址与默认监听一致；这是本地只读观察入口的约定。 |
| `s_session/catalog/signed-catalog.json:4` | `source_sha256` 为已签 `SPEC-COVERAGE-INPUT.json` 原始字节来源摘要，已重算相等；不是认证数据。 |

## 原有 8 键及承接证据

两路检索均完成：GitHub issue 搜索含 open/closed；本地 `analysis/ docs/ .chanlun/` 多关键词扫描。完整检索与票面/评论快照保存在证据目录；不以关键词未命中证明绝对不存在。

- `analysis/p3_random_gate_control.py` 的 RNG 3 键（197/251/344）是固定种子的研究模拟；第 141 行仍写“由用户实现”，但 172–181 行已有实现。**[#1316](https://github.com/xy7365527-lang/NewChanlun/issues/1316) 已具名承接冻结退役**，要求保留代码、不改逻辑；当前第 30 行冻结声明存在。不能将旧注释解作本轮授权补实现或变更统计语义。#1317 是生产回放接替，不是修改此档案的授权。
- `rust/src/theta_v0/nautilus/strategy.rs` 的 19/104/113 三键分别是“非 TODO”“原 TODO 已兑现”“原接入点已兑现”说明；不能凭词面判断未实现。
- 同文件第 314 键指向**仍然存在的旧入口缺口**：316–318 行 `entry_tick_for` 恒返回 None；207/234 行把它传给 `to_order_intent`，后者 85/94 行写入 `price_tick`，限价 entry 映射未接。**[#1368](https://github.com/xy7365527-lang/NewChanlun/issues/1368) → 具体叶 [#1385](https://github.com/xy7365527-lang/NewChanlun/issues/1385) 明列 OE09 Rust/Nautilus 旧入口的 R2 迁移责任**。这支持职责承接，不支持“该函数已经修复”或“该叶明确要求恢复旧限价映射”的更强说法。#927 / ADR0019:471 明确不覆盖 NT 限价；#1319 选择独立执行器的记录也不能证明此旧入口已修。

8 个原键均未加入候选 baseline；没有替旧票关项、开票或改语义。

## 仅供审阅的候选

[candidate-baseline.json](/tmp/newchanlun-1323-publication-20260909/evidence/PR1447-devskim/candidate-baseline.json) SHA256：`df25ff2ec9a0898de03a4450fc3506444f08100ca0657a32665086991e202595`。

保留原有 5,587 键的顺序，在末尾仅追加已核实的 2,614 键，总数 8,201；原第 860 行逐字保留，避免基线自定位告警漂移。原有元数据及 updates 项保留，只追加本次 run、短 head、范围、数量和中文分类说明。候选未写入仓库。

[候选核对](/tmp/newchanlun-1323-publication-20260909/evidence/PR1447-devskim/candidate-check.json)证明精确新增集一致、无重复、未吸收旧 8。**这是对现有真实 SARIF 的集合比较，尚无候选的新远端扫描结果；按当前集合仍会留下原 8 键。** 配套说明宜避免再次把大量证据 JSON 全量归档，引出新的摘要文字告警。

## 证据绑定

- [当前 SARIF](/tmp/newchanlun-1323-publication-20260909/evidence/PR1447-devskim/artifact/devskim-results.sarif)：`77104d9e22f4c51266f3fc69e1869bfad868d8b493188eaa773b524a9b8951d7`。
- [W38a SARIF](/tmp/newchanlun-1323-publication-20260909/evidence/PR1447-devskim/base38a-artifact/devskim-results.sarif)：`4b8693d103c19661c343b6762aa42612f89d0e59b7a020aed17f5b905e08ccb9`。
- [实际 job 日志](/tmp/newchanlun-1323-publication-20260909/evidence/PR1447-devskim/actual-job.log)：`7802111aafdb909dbe34effb38954526ead2602051d89f933d76ae12fcea5450`。
- [仅新增 2,614 键](/tmp/newchanlun-1323-publication-20260909/evidence/PR1447-devskim/new-vs-reviewed38a-exact-keys.txt)：`949f1edcc533c2192718122a407c2a9847ee890cda356ec10333cc19d71a83c2`。
- [完整证据清单](/tmp/newchanlun-1323-publication-20260909/evidence/PR1447-devskim/evidence-manifest.json)与[机器审查报告](/tmp/newchanlun-1323-publication-20260909/inputs/TB01-A-DEVSKIM-CI-REVIEW.json)。
