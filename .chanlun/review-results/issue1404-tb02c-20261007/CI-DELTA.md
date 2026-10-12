# #1404 DevSkim 基线与 PR 增量核验

固定基线 `8619a20` / run `37545808932` 对比 head `ce1ec9d` / run `37558930077`。只读取已下载 SARIF 和既有代码/证据，不运行扫描器，不修改产品，不打开数据库。

基线 41,207 条，head 41,304 条。按 ruleId、level、完整 message、路径及完整 snippet 作多重集比较，忽略行列号和字符偏移。新增 97 条，涉及 87 种键；消失 0 条。

新增包含 94 条 SHA-256 证据指纹、2 条 Git 基线提交指纹，均为 DS173237/error 密钥规则误报；另 1 条 DS162092/note 是本地端到端验收的 loopback URL。未发现新增实际凭据或新的安全风险；此结论仅覆盖这 97 条增量。

| 规则/级别 | 路径 | 新增次数 | 性质 |
|---|---|---:|---|
| DS162092/note | `s_session/tests/tb02c_runtime.py` | 1 | 正式验收脚本仅连接本机Q的loopback浏览器URL；开发代码规则提示 |
| DS173237/error | `.chanlun/review-results/issue1404-tb02c-20261007/COMPARISON-RESULT.json` | 4 | 测试夹具或验收报告中的SHA-256证据指纹；密钥规则误报 |
| DS173237/error | `.chanlun/review-results/issue1404-tb02c-20261007/FILES.json` | 1 | 测试夹具或验收报告中的SHA-256证据指纹；密钥规则误报 |
| DS173237/error | `.chanlun/review-results/issue1404-tb02c-20261007/FINAL-SOURCE.json` | 32 | 测试夹具或验收报告中的SHA-256证据指纹；密钥规则误报 |
| DS173237/error | `.chanlun/review-results/issue1404-tb02c-20261007/FINAL-SOURCE.json` | 1 | 验收报告中的Git基线提交指纹；密钥规则误报 |
| DS173237/error | `.chanlun/review-results/issue1404-tb02c-20261007/RUNTIME-SOURCE-R3.json` | 31 | 测试夹具或验收报告中的SHA-256证据指纹；密钥规则误报 |
| DS173237/error | `.chanlun/review-results/issue1404-tb02c-20261007/VALIDATION.json` | 25 | 测试夹具或验收报告中的SHA-256证据指纹；密钥规则误报 |
| DS173237/error | `.chanlun/review-results/issue1404-tb02c-20261007/VALIDATION.json` | 1 | 验收报告中的Git基线提交指纹；密钥规则误报 |
| DS173237/error | `s_session/tests/fixtures/tb02c/first-cut-logical-slots.json` | 1 | 测试夹具或验收报告中的SHA-256证据指纹；密钥规则误报 |

96 条密钥规则命中均逐项核对 SARIF 片段与固定 head 文件中对应 JSON 字段；94 条为 64 位 SHA-256，2 条为与固定基线 Git 对象一致的 40 位提交指纹。对应源码、R3冻结副本、二进制、20份运行回执、4份完整语义原件、对拍报告及 R2 HTTP 原件的指纹已核。工蜂初版 source_snapshot 指纹与其原始归档报告字段匹配，未把它说成当前源码指纹。

DS162092 指向 `s_session/tests/tb02c_runtime.py` 内启动真实 Chromium 时传入的本机 Q 地址。该文件是验收入口，loopback 是既定隔离范围；本增量不含生产联网地址或调试服务外放。

完整输入是Actions的devskim-sarif产物：基线artifact 11451215878、本轮artifact 11456112310，下载原件保留在外盘issue1404-ci-review-20261007目录。逐项核验工作原件在本票控制目录CI-DELTA.json；消失项为空。此处是入仓结论报告，固定上述head，不冒充后续提交的新增扫描结果。
