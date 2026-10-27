# #1405 DevSkim 增量预登记

本机无法跑 CI 同款扫描（Docker 守护进程拒绝本会话访问，固定镜像也需联网构建），所以这里是对本票增量（`0d4dc644..` 本分支 head）的静态预扫，不冒充 SARIF 结果。实际增量须在根开 PR 后取 Actions 的 `devskim-sarif` 产物，按 #1404 `CI-DELTA.md` 先例逐项对；若决定让零新增 gate 转绿，再照 #1370 先例把核实后的「规则|路径|行」追加进 `.github/devskim-baseline.json` 的 `updates`。

预扫口径：新增行里的 40/64 位十六进制串、URL、凭据关键词。入仓收据已把对象 id、批次 id 等长十六进制截成前 12 位，未截的只有下表 8 处。

| 预计规则 | 位置 | 性质 |
|---|---|---|
| DS173237（密钥规则） | `rust/tests/fixtures/tb02v_same_price/PROVENANCE.md:9`–`:14`（6 处） | 六个见证夹具文件的 SHA-256 证据指纹；误报 |
| DS173237（密钥规则） | `rust/src/theta_v0/parser/stroke_tb02v_tests.rs:580` | 尾 5 万根外部数据的 SHA-256，测试先校验数据身份；误报 |
| DS162092（localhost URL） | `.chanlun/review-results/issue1405-tb02v-20261014/receipts/browser.json:10` | 浏览器槽连接本机 Q 的 loopback 地址，验收收据原值；开发规则提示 |

凭据关键词零命中；生产代码路径无新增 URL。`s_session/tests/tb02v_runtime.py` 只把本机端口写进运行配置，不含 URL 字面量。
