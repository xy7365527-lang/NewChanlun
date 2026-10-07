# PR #1459 DevSkim 固定版本分诊

固定 HEAD：`950a49719bf1cc340ca4d1308df7710bbaff6557`；base：`6f99b36c64e0560febdb08e5d1f8e781d2af9299`。

结论：41 个输入 key 全部为 `not_actionable`，确认的未修产品问题为 0，`needs_review` 为 0。结论只适用于这 41 个 key。DevSkim 失败状态保留，main 合入仍待明确的扫描例外批准。

## 扫描身份与覆盖

- DevSkim run：`34852190257`，主控已核结论为 failure；本轮未重新查询 GitHub。
- Scanner：`devskim / Microsoft DevSkim Command Line Interface / 1.0.90+fb2d676ce4`。
- 原 SARIF SHA-256：`ec83e07420ab175bbc73fb09f8afe2c519cc87d20e20f53a4b756758b19211ea`，本轮对 134729361 字节原件独立复算一致。
- 裁剪输入 SHA-256：`1bd3a3ec9755c9d9f4ccf8e97aeba0f6e4ec9de24fc9ca379d1a9d07a845bce0`。
- 主控已核 CI run `34852190211` 的四个 job SUCCESS；CI 成功不改写 DevSkim 结论。
- 41 个 key 包含 19 个产品/测试路径命中和 22 个报告路径命中，没有 roster 文件命中。19 项进一步为 4 个运行时代码 key 和 15 个测试 key。
- 41 是 rule/path/line key 数，不是原始 SARIF location 数。独立定位得到 49 个原始 location，全部与固定 HEAD 原文匹配，5 个 key 在同一行有多个命中。逐项元数据保存在 TRIAGE.json，每个输入 key 都有独立结论，未删除重复外观条目。
- 变更关系：22 个新报告 key、1 个新运行时错误文案 key、6 个新增测试行 key、1 个修改既有测试行 key、11 个旧代码移位 key。移位不自动等于安全，本轮仍核对其回调/地址上下文。

22 个报告命中均为 Git 版本标识或 SHA-256 证据字段。归档文件的字节数和摘要、旧版五份源文件摘要、当前两份测试绑定摘要均已直接核对。其余命中分别是固定函数计时器、本机 Router 的拒绝文案及隔离测试数据，没有发现扫描器所指的不可信文本执行、实际非 TLS 数据传输或秘密泄露。

三产品字节与前次最终安全审查完全一致。本轮不改工作树、baseline 或扫描状态，只在仓外写两份分诊文件。详细输入归一化、路径、边界判断、反证和原始 location 索引见 [TRIAGE.json](TRIAGE.json)。

## 逐项结论

| 序号 | 完整 key | 分类及变更关系 | 理由 |
|---|---|---|---|
| 1 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/max-baseline/EXECUTE-RESULT.json\|47` | not_actionable；报告摘要；new_report_artifact | EXECUTE-RESULT 的 prompt_sha256 为提示内容的 SHA-256。生成路径是 codex-child.ts:529 的 createHash/update/digest，不是认证材料。 |
| 2 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/max-baseline/EXECUTE-RESULT.json\|31` | not_actionable；报告版本标识；new_report_artifact | EXECUTE-RESULT 的 checkout_after.head 是运行后 Git HEAD，与同记录的受测 fixture 版本字段一致；字段由 Git 快照生成。 |
| 3 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/max-baseline/EXECUTE-RESULT.json\|25` | not_actionable；报告版本标识；new_report_artifact | EXECUTE-RESULT 的 checkout_before.head 是运行前 Git HEAD，来源为预检快照，不用于认证。 |
| 4 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/max-baseline/EXECUTE-RESULT.json\|21` | not_actionable；报告版本标识；new_report_artifact | EXECUTE-RESULT 的 expected_head 为指定 fixture 的版本约束；入口要求完整 Git SHA 并与实际 HEAD 比对，不作为 token 使用。 |
| 5 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/max-baseline/ARCHIVE.json\|19` | not_actionable；报告摘要；new_report_artifact | ARCHIVE 的该 sha256 绑定归档 EXECUTE-RESULT.json；本轮对该 Git 内容重算摘要与字节数均一致。 |
| 6 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/max-baseline/ARCHIVE.json\|13` | not_actionable；报告摘要；new_report_artifact | ARCHIVE 的该 sha256 绑定归档 TESTS.log；本轮对该 Git 内容重算摘要与字节数均一致。 |
| 7 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/max-baseline/ARCHIVE.json\|7` | not_actionable；报告摘要；new_report_artifact | ARCHIVE 的该 sha256 绑定归档 MAX-VERIFIED.json；本轮对该 Git 内容重算摘要与字节数均一致。 |
| 8 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/max-baseline/MAX-VERIFIED.json\|25` | not_actionable；报告摘要；new_report_artifact | MAX-VERIFIED 的 fixture_sha256 记录旧短读写探针的文件校验和；同一摘要作为 execute 结果回读比对目标，没有认证用途。 |
| 9 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/max-baseline/MAX-VERIFIED.json\|23` | not_actionable；报告摘要；new_report_artifact | MAX-VERIFIED 的 runtime.sha256 绑定 execute 结果文件；与 ARCHIVE 对同文件的摘要一致，且已重算文件内容确认。 |
| 10 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/max-baseline/MAX-VERIFIED.json\|13` | not_actionable；报告摘要；new_report_artifact | source_sha256 的该值绑定旧版工位名册文件；本轮对已登记 Git commit 的该文件重算 SHA-256 一致。 |
| 11 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/max-baseline/MAX-VERIFIED.json\|12` | not_actionable；报告摘要；new_report_artifact | source_sha256 的该值绑定旧版 CODEX-CHILD.md；本轮对已登记 Git commit 的该文件重算 SHA-256 一致。 |
| 12 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/max-baseline/MAX-VERIFIED.json\|11` | not_actionable；报告摘要；new_report_artifact | source_sha256 的该值绑定旧版 codex-child.test.ts；本轮对已登记 Git commit 的测试文件重算 SHA-256 一致。 |
| 13 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/max-baseline/MAX-VERIFIED.json\|10` | not_actionable；报告摘要；new_report_artifact | source_sha256 的该值绑定旧版 codex-child.ts；本轮对已登记 Git commit 的产品文件重算 SHA-256 一致。 |
| 14 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/max-baseline/MAX-VERIFIED.json\|9` | not_actionable；报告摘要；new_report_artifact | source_sha256 的该值绑定 codex-router-runtime.ts；本轮对登记的旧 Git 内容重算 SHA-256 一致，当前内容亦相同。 |
| 15 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/max-baseline/MAX-VERIFIED.json\|7` | not_actionable；报告版本标识；new_report_artifact | MAX-VERIFIED 的 commit 是旧路由入口 Git 提交号；本轮 git cat-file 确认该对象类型为 commit，且其源文件摘要逐个匹配。 |
| 16 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/CHECKS.json\|14` | not_actionable；报告摘要；new_report_artifact | CHECKS.sources 中该值是当前 codex-child.test.ts 的文件 SHA-256；本轮对固定 HEAD 内容重算一致。 |
| 17 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/CHECKS.json\|13` | not_actionable；报告摘要；new_report_artifact | CHECKS.sources 中该值是当前 codex-child.ts 的文件 SHA-256；本轮对固定 HEAD 内容重算一致。 |
| 18 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/CHECKS.json\|10` | not_actionable；报告摘要；new_report_artifact | CHECKS.fixture_sha256 是新短只读探针的文件校验和，与相邻 fixture_bytes 及 RUNTIME-RESULT 的摘要检查描述对应，不是凭据。 |
| 19 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/RUNTIME-RESULT.json\|47` | not_actionable；报告摘要；new_report_artifact | RUNTIME-RESULT 的 prompt_sha256 是提示内容 SHA-256，生成路径为 codex-child.ts:529；不保存提示中的认证内容。 |
| 20 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/RUNTIME-RESULT.json\|31` | not_actionable；报告版本标识；new_report_artifact | RUNTIME-RESULT 的 checkout_after.head 是新只读 fixture 运行后 Git HEAD，与前快照和 expected_head 一致。 |
| 21 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/RUNTIME-RESULT.json\|25` | not_actionable；报告版本标识；new_report_artifact | RUNTIME-RESULT 的 checkout_before.head 是新只读 fixture 运行前 Git HEAD，来源为 Git 预检快照。 |
| 22 | `DS173237\|.chanlun/review-results/issue1448-flash-output-20260914/RUNTIME-RESULT.json\|21` | not_actionable；报告版本标识；new_report_artifact | RUNTIME-RESULT 的 expected_head 是新只读 fixture 的版本约束，入口将其与 Git HEAD 校验，不作为认证 token。 |
| 23 | `DS162092\|.sandcastle/codex-router-runtime.ts\|31` | not_actionable；新增校验错误文案；new_runtime_file | 命中位于抛出 Error 的固定说明字符串。上一行拒绝非回环或非法端口输入；该位置没有连接动作或调试后门。本机 Router 是 CODEX-CHILD.md 明确支持的架构。 |
| 24 | `DS172411\|.sandcastle/codex-child.ts\|364` | not_actionable；旧代码移位：固定定时回调；unchanged_line_moved；base 第 351 行 | setTimeout 第一参数是固定箭头函数，只调用 stop 的固定超时消息。可配置时长位于第二参数并已作数值范围校验；外部文本没有进入可执行代码位置。 |
| 25 | `DS172411\|.sandcastle/codex-child.ts\|360` | not_actionable；旧代码移位：固定定时回调；unchanged_line_moved；base 第 347 行 | 空闲计时器第一参数为固定箭头函数，传入 stop 的消息为静态常量；时长只参与毫秒数计算，不被当作代码执行。 |
| 26 | `DS172411\|.sandcastle/codex-child.ts\|349` | not_actionable；旧代码移位：固定定时回调；unchanged_line_moved；base 第 336 行 | killTimer 使用固定函数和固定 1000 毫秒延迟，回调只对已记录的本次子进程组发送 SIGKILL；没有字符串形式的定时代码。 |
| 27 | `DS172411\|.sandcastle/codex-child.test.ts\|525` | not_actionable；旧测试移位：固定定时回调；unchanged_line_moved；base 第 475 行 | 测试的 10 秒兜底用固定箭头函数终止本测试创建的 CLI 子进程；未把外部文本传入 setTimeout。 |
| 28 | `DS172411\|.sandcastle/codex-child.test.ts\|496` | not_actionable；旧测试移位：固定定时回调；unchanged_line_moved；base 第 446 行 | 管道关闭测试的 10 秒兜底是固定箭头函数，只终止同一测试创建的 CLI；不解释不可信字符串。 |
| 29 | `DS172411\|.sandcastle/codex-child.test.ts\|483` | not_actionable；旧测试移位：伪造进程脚本；unchanged_line_moved；base 第 433 行 | f.fake 接受源码中的固定测试程序字面量，延迟回调是固定 session() 函数；该 helper 只写本测试临时目录的假 codex，不是产品输入通道。 |
| 30 | `DS172411\|.sandcastle/codex-child.test.ts\|311` | not_actionable；旧测试移位：伪造进程脚本；unchanged_line_moved；base 第 261 行 | f.fake 的固定脚本通过函数回调延迟写入本测试的退出标记，用于验证监督器确实等到进程退出；没有外部可执行文本输入。 |
| 31 | `DS137138\|.sandcastle/codex-child.test.ts\|214` | not_actionable；旧测试移位：代理占位符；unchanged_line_moved；base 第 168 行 | HTTP 地址仅作为假子进程环境透传与日志不泄漏测试的代理占位值；f.fake 已替换 codex，捕获环境键和生成静态事件，不发出 HTTP 请求。 |
| 32 | `DS137138\|.sandcastle/codex-child.test.ts\|136` | not_actionable；旧测试移位：代理占位符；unchanged_line_moved；base 第 90 行 | 同一行两处 HTTP 字面量是大小写代理环境映射测试数据；workerEnvironment 只复制字段，紧接 deepEqual，不建立网络连接。 |
| 33 | `DS162092\|.sandcastle/codex-child.test.ts\|135` | not_actionable；旧测试移位：NO_PROXY 数据；unchanged_line_moved；base 第 89 行 | 该行 localhost 与回环地址是 NO_PROXY 的预期测试字符串，用来检查环境白名单保留数据；不是遗留调试服务。 |
| 34 | `DS137138\|.sandcastle/codex-child.test.ts\|134` | not_actionable；旧测试移位：代理占位符；unchanged_line_moved；base 第 88 行 | 同一行两处 HTTP 字面量是 HTTP_PROXY/HTTPS_PROXY 测试值，仅参与 workerEnvironment 的纯对象比较，没有 TLS 网络传输。 |
| 35 | `DS137138\|.sandcastle/codex-child.test.ts\|128` | not_actionable；修改既有测试：拒绝不可信环境；modified_existing_test_line；base 第 82 行 | 新 HTTP 值位于 OPENAI_BASE_URL 的反向测试输入；下一行断言返回环境不含此字段。实际白名单也没有该键，所以该地址不会被透传使用。 |
| 36 | `DS162092\|.sandcastle/codex-child.test.ts\|114` | not_actionable；新增测试：本机配置断言；added_test_line | 回环地址是临时配置读取结果的期望值。readRouterConfiguration 只解析文件和检查模型目录，此测试不建立服务连接。 |
| 37 | `DS162092\|.sandcastle/codex-child.test.ts\|102` | not_actionable；新增测试：认证路径拒绝；added_test_line | 地址由固定占位 token 构成，写入隔离的临时 CODEX_HOME；随后断言 runCodexChild 失败、process_pid 为 null、结果不含占位值。生产校验在 spawn 前拒绝它。 |
| 38 | `DS162092\|.sandcastle/codex-child.test.ts\|98` | not_actionable；新增测试：非法地址集合；added_test_line | 同一行五处本机地址属于拒绝用例集合，包含错误协议/端口、认证路径、用户信息和查询参数；每项均要求配置读取抛错，不是部署配置。 |
| 39 | `DS137138\|.sandcastle/codex-child.test.ts\|98` | not_actionable；新增测试：非 TLS 反例；added_test_line | 同一行两处被规则捕获的 HTTP 地址是必须被拒绝的测试输入，含占位用户信息或无效远端；循环断言解析失败，未执行网络连接。 |
| 40 | `DS162092\|.sandcastle/codex-child.test.ts\|83` | not_actionable；新增测试：Router argv 断言；added_test_line | 回环地址处于 command.includes 的固定期望字符串，仅检查构造出的 CLI 配置是否绑定本机 Router，不是未删除的调试调用。 |
| 41 | `DS162092\|.sandcastle/codex-child.test.ts\|31` | not_actionable；新增测试：隔离配置 fixture；added_test_line | 回环地址写入 mkdtemp 创建的临时 CODEX_HOME 配置，与临时模型目录配套供解析和假进程测试使用；测试清理删除整个 fixture，没有改变用户配置。 |

## 例外批准与未覆盖范围

41 个 key 的分诊完成，不把 DevSkim 红灯改成绿灯，也不修改 baseline。`31693` 个未变文件命中和 `8187` 个既有 baseline 条目是裁剪输入给出的排除计数，本轮没有重审其安全性；主控须分别给出同字节继承与既有 main 扫描例外的依据。

本报告没有确认的未修产品问题或待技术查证项。待决事项是明确批准当前版本的 main 扫描例外及合入，不能从本报告或 CI 成功推定已获批准。前次审查保留的宿主读取隔离与模型长任务格式失败边界继续有效。

本轮仅静态读取指定源码、报告、Git 对象、原 SARIF 及适用 SECURITY.md。没有读取凭据内容、启动模型/服务/代理、运行测试/PoC 或执行修复。报告不复制任何疑似秘密字面值。
