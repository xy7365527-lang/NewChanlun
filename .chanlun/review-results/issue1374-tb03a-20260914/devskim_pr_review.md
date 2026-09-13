最新提交 `5d5d6236a71b6726078d180fd09b2b32c5de0371` 的 DevSkim 仍为 **FAIL**。实际扫描器与 SARIF 上传成功；门禁按既有基线规则拒绝 26,907 个基线外位置键。随后 3,155 KiB 的完整摘要超过 GitHub 1,024 KiB 限制，另报摘要上传失败。本次分诊没有发现由本 PR 新增内容构成的已确认产品安全漏洞；本结论不批准合入、不令 CI 变绿、不授权扫描例外。

| 实际运行 | 原始 findings | 唯一位置键 | 基线外 |
|---|---:|---:|---:|
| base `65298686` / 34759110350 | 34,663 | 34,589 | 26,402 |
| 首轮 `b5b1ab14` / 34781051428 | 35,122 | 35,048 | 26,861 |
| 最新 `5d5d6236` / 34781472772 | 35,168 | 35,094 | 26,907 |

门禁实际用 `rule|path|startLine` 去重，因此 raw 与 unique 不可混用。最新相对 base 有 512 个新增位置键、7 个消失键；26,395 个相同基线外键全部位于未修改文件。7 增/7 消为完全相同 localhost 原行的平移，实际新增内容对应 505 个键。基线 8,201 个键与五份扫描配置逐字未变，三轮均有 14 个基线键未再出现。

| 本 PR 命中 | 位置 | 数量 | 语境与最小处理 |
|---|---|---:|---|
| DS173237 凭证检测 | `.chanlun/review-results/issue1374-tb03a-20260914/*.json` | 502 | 481 个 64 位摘要/对象标识，21 个 Git commit 引用；已逐位置绑定原始行和 JSON pointer，21 个 Git 引用均核为真实 commit。属于证据数据，不承担鉴权凭证用途。保留原件，不删摘要或改报告来刷绿。 |
| DS173237 凭证检测 | `rust/src/session_protocol.rs:339` | 1 | `cfg(test)` 中独立 Python wire 向量的 `payload_hash`；本工位已用 Python 重算完整规范 payload，值一致。保留固定向量。 |
| DS172411 动态执行检查 | `s_session/browser/tb03a-client.js:150` | 1 | 固定函数闭包 `() => control.abort()`；未拼接字符串执行。超时来自常数或已校验数值，`finally` 清理计时器。不需要功能修改。 |
| DS197836 时间值低熵哈希 | `rust/src/economic_session/service.rs:118:71` | 1 | SARIF 的 228 字符片段从 `raw_sha256` 字段名内 `sha256` 开始，跨多个 JSON 字段直到 `received_time`，不是 hash 调用。实际对完整原件、完整事实和完整请求取 SHA-256。独立 Codex 子代理复核为词法误报。可选的每字段一行格式整理不改协议，但并不能修复其余门禁失败；本次未实施。 |
| DS162092 localhost 检查 | `s_session/launch_s.sh:270–273`；`s_session/s_readonly_server.py:1159`；`s_session/s_service_control.py:80,82` | 7 | 对应 base `264–267`、`1153`、`75,77`；七行逐字相同，只发生行移，仍是有意本地控制/只读地址。不扩大监听地址。 |

最新相对首轮的 145 增/99 消全部位于 `EVIDENCE.json` 和新归档 `fmt_tail.json`，净增 46 个摘要位置键。全部非报告告警与首轮相同。

既有 `DS176209|rust/src/theta_v0/nautilus/strategy.rs|314` 仍由 [#1385 的精确承接记录](https://github.com/xy7365527-lang/NewChanlun/issues/1385#issuecomment-5648592872) 保持 `needs_review`：`entry_tick_for` 恒为 `None` 的 entry 映射待 OE09 核查。当前文件与 base 逐字相同。此处未确认安全漏洞、未改交易行为、未销项；旧批准不是本 head 的批准。

报告范围是三个实际 SARIF、真实日志与本 PR 增量。没有把基线内、全部历史告警或扫描未覆盖面重新作完整安全审计，也没有重新计算所有审查报告的业务证据；摘要的用途按字段和既有冻结来源核实。没有修改 W4、规则、基线、ignore、审查原件或服务，没有发布评论。

扫描契约不变且冻结证据保留时，没有发现能在本 PR 产品范围内使全仓门禁变绿的必要安全修复。摘要超限可在独立门禁改动中限制展示条数并保留完整 SARIF，不能吞掉 FAIL 或删除全量证据。合入与任何扫描治理决定由 root 按当前具体授权处理。

完整逐位置表：`LATEST-NEW-FINDINGS-ASSESSED.json`；精确增减：`exact-base-to-latest-delta.json`；原始 SARIF、日志和下载元数据均保存在本目录。最新 Actions 实际 checkout `65cd98e0` 的树与上述 head 完全一致；first checkout 也逐树核实。`REVIEW.json` 绑定各原件摘要、数量、来源和限制。
