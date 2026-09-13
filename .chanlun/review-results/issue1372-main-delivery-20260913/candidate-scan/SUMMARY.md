# PR1451 / 041c8e2ef7 DevSkim 实际结果与增量分诊

固定 head `041c8e2ef7916a465d8b352ff9dc1469707f8ac1`，基线 `e46cf3bca6a67da821f9ab046507563b8133f6d9`，对照旧 `acd30cb61e59762114061b2d0b394aa270ed35ec`。[DevSkim run 34755511520](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34755511520) 的 scanner 和 SARIF artifact 上传成功；gate 实际 **FAIL / exit 1**，原因是 **25,691 个基线外键**。完整日志的键集合与排序均和本次 SARIF 差集相同，没有遗漏或截断判据。

远端 gate 步骤耗时从 acd 的 **39 分 02 秒**（11:07:18→11:46:20 UTC）降至本次 **6 秒**（11:54:49→11:54:55 UTC），观察耗时约为原来的 1/390。两次输入不是同字节，runner 硬件未锁；这是两次真实 CI 步骤的观察，不冒充相同输入同机 benchmark。本次原始 findings 反而增加 141。性能修复有效，安全门仍红。

| 计数 | acd | 041 |
|---|---:|---:|
| SARIF 原始 results | 33,811 | 33,952 |
| 唯一 rule/path/line 键 | 33,737 | 33,878 |
| 基线内键 | 8,187 | 8,187 |
| 基线外键 | 25,550 | 25,691 |

新增 **141** 键，消失 **0**，行移 **0**；共同 33,737 键的匹配原文 SHA256 多重集完全相同。新增全部为 **DS173237**，仅在 7 个新报告 JSON：`MATRIX-RESULTS.json` 101、`EVIDENCE-SOURCES.json` 18、`MANIFEST.json` 9、Python `REVIEW.json` 5、`REAL-SARIF-REPLAY.json` 4、gate `REVIEW.json` 3、`OLD-RUN-SNAPSHOT.json` 1。gate 脚本和新 Python 回归文件没有新增扫描命中。

## 141 个值的完整归因

- 99 个具名对拍用例输出摘要：33 × stdout / 规范化 stderr / summary；逐字段定位到 case 与输出类型，核已封存报告原字节，不重新执行或规范化这些用例。
- 32 个具名文件 SHA256：7 报告原件、11 仓外输出/清单/旧分诊原件、9 包清单文件、5 Python 评审源文件。本次逐个重算长度和摘要；7 报告副本也与 Git 逐字相同。
- 8 个 gate/回放绑定摘要：7 个源码、baseline 或完整 stdout 摘要重算；1 个旧 SARIF 摘要复用已冻结 artifact 摘要并核现存长度。
- 2 个可解析的旧 acd Git commit 身份。

全部 141 条都已核 SARIF span 与固定 Git 字节一致，并绑定唯一 JSON Pointer；没有抽样或未分类项。这些值是摘要/对象身份，未发现凭据用途。新增确认产品安全缺陷 **0**，新增 needs_review **0**；实际全仓风险总数仍未知。旧 `DS176209|rust/src/theta_v0/nautilus/strategy.rs|314` 继续由 [#1385](https://github.com/xy7365527-lang/NewChanlun/issues/1385#issuecomment-5648592872) 承接，状态仍是 needs_review。

当前 25,691 = e46 继承 10,888 + 2 个旧基线行移 + C 产品阶段 345 + acd 报告 14,315 + 本次报告 141。旧 B 例外不会自动覆盖 C。

## 提交与原件绑定

独立 Git 核对仅 12 路径变化：gate 脚本、包含 5 项回归的 Python 文件、10 小报告。Rust、s_session、.github 及旧 C 主报告包四棵子树均与 acd 相同。实际 checkout `1098c0cacf74e4a79e3966d4e1a280d6a9a37081`，tree `4ebe9ffe886229634640a0e7c5da57f9d8ecacb8` 与固定 041 head 完全相同，父节点精确为 e46/041。仅 gate SHA 从 `bb5a293ab036624244eb5b29bc2358c352929fc65ea61845e50b53c7f4e2bd51` 变为 `c3154bd748c796ba950d1b4926308cd37a1cf54b0a802f644b6a8877132ce5fc`；baseline、workflow、Dockerfile、entrypoint 原字节均保持。

随后 GITHUB_STEP_SUMMARY 的 3037k 展示内容仍超过 1024k；该附加展示错误在 gate exit 1 之后，不能替代基线外发现导致的 FAIL。

SARIF 原件 130,106,700 B，SHA256 `ff8c5d0f355e19556995cac256a5db187c923fc67550be3fc1f833395aed7d79`，artifact `10317950656`；ZIP 摘要与 GitHub artifact digest 相同。原件、完整分诊、checkout 与日志证据均在本目录，长度/SHA 见 `ORIGINALS-INDEX.json`。本任务未修改仓库、基线或旧证据，也未重扫产品；小摘要只供根代理更新 PR 正文，不因记录 CI 再生成提交。
