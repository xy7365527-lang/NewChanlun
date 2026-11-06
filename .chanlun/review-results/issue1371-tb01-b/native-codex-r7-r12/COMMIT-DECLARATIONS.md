# #1371 R12 逐提交可编译性声明

本声明固定基线 `2212eba31e0b7419907e622e79f02f7c2cc37706` 至代码提交 `e5b03629adde165610161b4df3e2d62d22163568`，共 **21 个提交：12 个含产品代码、1 个仅测试代码、8 个纯报告文档**。13 个非文档提交按下列具名证据声明可编译，8 个纯报告提交明确编译豁免。

这份文件是本地交付材料。编译声明覆盖历史报告所用的 `s_session / s_structure_session` 靶向 Rust 构建和本片改动脚本/HTML；不扩成全仓全部 target 构建、功能全域通过或已合入 main。编译与测试均未在本工位新执行。

规则：[delivery-discipline.md:14](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/docs/agents/delivery-discipline.md:14) 要求逐个声明可编译、纯文档明确豁免；并不要求重建 21 次。复用依据：[此前交付门材料](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R7-CODEX-POST-REVIEW-GATES.md)。

## 逐提交声明

| 序号 | 完整提交SHA | 文件性质 | 声明及依据 |
|---|---|---|---|
| 1 | `8c868f968c5f937870a4305dca05a0e0162b5131` | 含产品代码 | **可编译（复用该提交历史检查声明）**。该提交自身版本的 IMPLEMENTATION-REPORT.json#/checks/cargo_check=0；登记产品文件SHA256与该提交Git字节全部一致。声明限报告的 s_session/s_structure_session 靶向构建面，不推为全仓全部target构建。 |
| 2 | `6039b83f6a1730daeb0d557f3bb712d8c31c92bd` | 纯报告文档 | **纯文档豁免**。本提交全部变更仅为该票报告Markdown/JSON，无产品、测试代码、构建输入；按交付纪律第14行明确豁免编译。 |
| 3 | `0298cf429804c611d6ee5f691a1f36ee2a720fba` | 含产品代码 | **可编译（复用该提交历史检查声明）**。该提交自身版本的 IMPLEMENTATION-REPORT.json#/checks/cargo_check=0；登记产品文件SHA256与该提交Git字节全部一致。声明限报告的 s_session/s_structure_session 靶向构建面，不推为全仓全部target构建。 |
| 4 | `6e1b355b1454c9a20afd50a372058da11df86a30` | 纯报告文档 | **纯文档豁免**。本提交全部变更仅为该票报告Markdown/JSON，无产品、测试代码、构建输入；按交付纪律第14行明确豁免编译。 |
| 5 | `a9acf50ac3a700450108040ca1b2c019675df3c9` | 含产品代码 | **可编译（复用该提交历史检查声明）**。该提交自身版本的 IMPLEMENTATION-REPORT.json#/checks/cargo_check=0；登记产品文件SHA256与该提交Git字节全部一致。声明限报告的 s_session/s_structure_session 靶向构建面，不推为全仓全部target构建。 |
| 6 | `58dab231e2210e1ba1e9634c936807d8492e218b` | 含产品代码 | **可编译（复用该提交历史检查声明）**。该提交自身版本的 IMPLEMENTATION-REPORT.json#/checks/cargo_check=0；登记产品文件SHA256与该提交Git字节全部一致。声明限报告的 s_session/s_structure_session 靶向构建面，不推为全仓全部target构建。 |
| 7 | `8b726a57f98974c451ad0490066150afb9fe8e38` | 含产品代码 | **可编译（复用该提交历史检查声明）**。该提交自身版本的 IMPLEMENTATION-REPORT.json#/checks/cargo_check=0；登记产品文件SHA256与该提交Git字节全部一致。声明限报告的 s_session/s_structure_session 靶向构建面，不推为全仓全部target构建。 |
| 8 | `6eeed76e3e12f1ffc3fdc62e23e60f2a3084c37b` | 含产品代码 | **可编译（复用该提交历史检查声明）**。该提交自身版本的 IMPLEMENTATION-REPORT.json#/checks/cargo_check=0；登记产品文件SHA256与该提交Git字节全部一致。声明限报告的 s_session/s_structure_session 靶向构建面，不推为全仓全部target构建。 |
| 9 | `4a3da3787bdd6e783f3c7eee63dec544b7dc3e68` | 含产品代码 | **可编译（复用该提交历史检查声明）**。该提交自身版本的 IMPLEMENTATION-REPORT.json#/checks/cargo_check=0；登记产品文件SHA256与该提交Git字节全部一致。声明限报告的 s_session/s_structure_session 靶向构建面，不推为全仓全部target构建。 |
| 10 | `65fb5e72bbee1de6656ba4b902dd506ae526195f` | 含产品代码 | **可编译（复用该提交历史检查声明）**。该提交自身版本的 IMPLEMENTATION-REPORT.json#/checks/cargo_check=0；登记产品文件SHA256与该提交Git字节全部一致。声明限报告的 s_session/s_structure_session 靶向构建面，不推为全仓全部target构建。 |
| 11 | `257452c5fd02e0b28b385b7bca4c327b76ea6aa0` | 纯报告文档 | **纯文档豁免**。本提交全部变更仅为该票报告Markdown/JSON，无产品、测试代码、构建输入；按交付纪律第14行明确豁免编译。 |
| 12 | `381f5e1c670cf97b6a1491a3ec3c03c30df406d0` | 含产品代码 | **可编译（复用R9历史构建收据）**。R9 /source_commit 精确绑定；/checks/build、/checks/check exit=0；commands /34、/36 已纯解码核验，/62保留该代码提交回执；6份源码登记SHA与Git一致。Python AST /52、launcher语法 /53、HTML消费者实际执行 /59均exit=0。 |
| 13 | `667ce698eacd8c8f50ec5846f24c779b492010a8` | 纯报告文档 | **纯文档豁免**。本提交全部变更仅为该票报告Markdown/JSON，无产品、测试代码、构建输入；按交付纪律第14行明确豁免编译。 |
| 14 | `41ed7163e8bc15bc7cc5af1fb06af22d98a6589c` | 纯报告文档 | **纯文档豁免**。本提交全部变更仅为该票报告Markdown/JSON，无产品、测试代码、构建输入；按交付纪律第14行明确豁免编译。 |
| 15 | `a0b6f17a3def8f27f2ff936980c97e34c9da1c0e` | 含产品代码 | **可编译（复用R10首个代码提交前检查收据）**。R10 commands /9构建、/11检查、/14 Python回归、/15 HTML函数消费者、/20 Python AST、/21 launcher语法均exit=0，/27记录a0b6f17a3d提交。后续50f22只有Python测试更改；产品和Node测试同字节，R10最终/source_commit虽为50f22，不能误写成a0b。 |
| 16 | `50f22cac47b25685a8a24088cd3b64b3a82e2d4f` | 仅测试代码 | **可编译（仅测试代码，构建输入同字节复用；不是文档豁免）**。R10 /source_commit直接绑定50f22；Rust完整子树、产品和Node测试与a0b相同，复用/9构建和/11检查。改动Python脚本另有/28实际回归、/29消费者、/30 AST exit=0，/32精确记录50f22提交。 |
| 17 | `3ddb5c2b7f9f250861640d40d779d1a6a5fd8955` | 纯报告文档 | **纯文档豁免**。本提交全部变更仅为该票报告Markdown/JSON，无产品、测试代码、构建输入；按交付纪律第14行明确豁免编译。 |
| 18 | `a60fdd57e5ebb0e7d6a15b7e1f30994720269ed8` | 纯报告文档 | **纯文档豁免**。本提交全部变更仅为该票报告Markdown/JSON，无产品、测试代码、构建输入；按交付纪律第14行明确豁免编译。 |
| 19 | `6a4e69e60ffd1baca490f870622c098a14fe7b41` | 含产品代码 | **可编译（HTML/Node检查已执行，未变后端复用）**。R11 /source_commit直接绑定；/checks/source-build对应commands /0 exit=0；Node测试语法/4及实际HTML提取script语法/5 exit=0，/6保留28项历史绿灯入口；6份源码hash匹配。 |
| 20 | `4922543038fbcfef1a4d711a306708ace3f62e88` | 纯报告文档 | **纯文档豁免**。本提交全部变更仅为该票报告Markdown/JSON，无产品、测试代码、构建输入；按交付纪律第14行明确豁免编译。 |
| 21 | `e5b03629adde165610161b4df3e2d62d22163568` | 含产品代码 | **可编译（R12实际HTML/Node执行通过，后端构建同输入复用）**。R12仅改HTML、新Node永久测试/JSON fixture和工位文档；GREEN.json十项全passed且failed=[]、EXECUTION.json green exit=0，源码/fixture与本提交Git字节绑定。既有R11 28项用原HTTP字节重放exit=0。R12完整Rust子树与a0b相同，Rust入口、Python服务、launcher、Cargo.toml/lock同字节，复用R10/R11已执行后端构建。 |

## 文件清单与哈希

伴随 JSON 的 `commits[*].files` 逐文件保存完整路径、性质、Git blob、字节数和 SHA256；以下列出各提交实际变更文件，均来自固定 Git 对象，不受他人随后工作树编辑影响。

### 1. 8c868f968c5f937870a4305dca05a0e0162b5131

feat(structure): #1371——修订撤回与持久历史恢复

- `A` [.chanlun/agent-roster-2026-09-10.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/agent-roster-2026-09-10.md)：报告或工位文档；SHA256 `78f54decd70c901b629e5d8c53eab577bd197914ea0caa93b798ca15ea750ebe`。
- `A` [.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json)：报告或工位文档；SHA256 `e9d8e68622b5de95fa3a4d9b96d33577a0ff9a864a500e1fc40907488534eb2b`。
- `A` [.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.md)：报告或工位文档；SHA256 `e23aa5fe406badb4a56ce546549877f08a7482dffbda580cbeb753d759010a4c`。
- `M` [rust/src/bin/s_structure_session.rs](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/rust/src/bin/s_structure_session.rs)：产品代码；SHA256 `0c110dbea358cbbfaafcc3759cd2b5d9931ae6b2e045b88d070ef18812b638b7`。
- `M` [s_session/browser/index.html](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/browser/index.html)：产品代码；SHA256 `5c311db8a9f3577379a7719289b68d34cddc72aa365014fe5573afa3a9bf42f0`。
- `M` [s_session/launch_s.sh](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/launch_s.sh)：产品代码；SHA256 `65cb1552594dea36f1368d952c0bbc839418d06cab107b0ca8ce594e3a5b7d1d`。
- `M` [s_session/s_readonly_server.py](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/s_readonly_server.py)：产品代码；SHA256 `db8998936c5719539e672a453ac2c1b942e84a46baa87bf7d889edf675b147f5`。

声明：**可编译（复用该提交历史检查声明）**。证据强度：历史作者检查声明 + 本輪Git源码字节绑定；未从此早期报告取得独立原始cargo receipt，不冒称本轮新编译。

应读该提交的 `.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json`（报告Git blob `4e9aa59fefef1ba55ef510f46c76421e09a5a4d5`，报告SHA256 `e9d8e68622b5de95fa3a4d9b96d33577a0ff9a864a500e1fc40907488534eb2b`），不能用最终同名报告代替历史版本。字段 `/checks/cargo_check=0`、`/checks/bin_tests=18 passed / 0 failed`。登记的四份产品文件逐项SHA匹配；完整登记值与JSON pointer见伴随JSON该提交 `evidence[0].source_hash_checks`。

### 2. 6039b83f6a1730daeb0d557f3bb712d8c31c92bd

docs: #1371 TB-01-B 独立评审报告（review.md + review.json）

- `A` [.chanlun/review-results/issue1371-tb01-b/review.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/review.json)：报告或工位文档；SHA256 `78237dcb496c4a718e6bcab2b6713029b33b4ef7b8e5d330a0268d542dfd5d49`。
- `A` [.chanlun/review-results/issue1371-tb01-b/review.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/review.md)：报告或工位文档；SHA256 `2a4726c9ace53d01b35e598df75d20c5510c490a441890e2e4ddce48ced04958`。

声明：**纯文档豁免**。证据强度：本轮只读Git逐文件分类。

### 3. 0298cf429804c611d6ee5f691a1f36ee2a720fba

fix(structure): #1371——历史cut不可变/可达根写前件/幂等advance与原身份Query

- `M` [.chanlun/agent-roster-2026-09-10.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/agent-roster-2026-09-10.md)：报告或工位文档；SHA256 `21aa19c7718e6aad6230c7693b447ad79fa3795f4227a2c7b42777cbc42eec68`。
- `M` [.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json)：报告或工位文档；SHA256 `c86f348647f05e5508c4e0e1d2e6a8b9d18108ab23963e7ba793846d9fb60114`。
- `M` [.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.md)：报告或工位文档；SHA256 `536b493ef5a683440c0d3c73f23fdaa7bf0f95e81996e32d84f35d1d85b2faf1`。
- `M` [rust/src/bin/s_structure_session.rs](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/rust/src/bin/s_structure_session.rs)：产品代码；SHA256 `2e06b83eb220934aba6f88f3be31ea876fe21b63984ce0a78dc162549dfdeb9e`。
- `M` [s_session/browser/index.html](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/browser/index.html)：产品代码；SHA256 `bbc86d0158bfebc5a9f5e73070f99a19f89532c4a35f28f477d076c7ca0c5084`。
- `M` [s_session/s_readonly_server.py](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/s_readonly_server.py)：产品代码；SHA256 `8024b01ca38215a23357afd8354c76b392565c895bac472269617fb531addef1`。

声明：**可编译（复用该提交历史检查声明）**。证据强度：历史作者检查声明 + 本輪Git源码字节绑定；未从此早期报告取得独立原始cargo receipt，不冒称本轮新编译。

应读该提交的 `.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json`（报告Git blob `2a760331f71145ba8f39fc9087454d2efd4fb271`，报告SHA256 `c86f348647f05e5508c4e0e1d2e6a8b9d18108ab23963e7ba793846d9fb60114`），不能用最终同名报告代替历史版本。字段 `/checks/cargo_check=0`、`/checks/bin_tests=22 passed / 0 failed`。登记的四份产品文件逐项SHA匹配；完整登记值与JSON pointer见伴随JSON该提交 `evidence[0].source_hash_checks`。

### 4. 6e1b355b1454c9a20afd50a372058da11df86a30

docs: #1371 TB-01-B 独立评审·增量复审（review-incremental.md + review-incremental.json）

- `A` [.chanlun/review-results/issue1371-tb01-b/review-incremental.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/review-incremental.json)：报告或工位文档；SHA256 `68dc5813917c5449dc65cdae9974f12578ea27d0d0c1b9485be282511b56a72a`。
- `A` [.chanlun/review-results/issue1371-tb01-b/review-incremental.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/review-incremental.md)：报告或工位文档；SHA256 `e0091f663f13bf3f45c56dcad7ed9d15cc13ca8c91b32804a4abb863ae1a4567`。

声明：**纯文档豁免**。证据强度：本轮只读Git逐文件分类。

### 5. a9acf50ac3a700450108040ca1b2c019675df3c9

fix(structure): #1371——可达根完整性与reader/浏览器同cut身份原子提交

- `M` [.chanlun/agent-roster-2026-09-10.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/agent-roster-2026-09-10.md)：报告或工位文档；SHA256 `9799d3dcdd750f3c5aaf4b8e9dc4f7820fc18b8ee87a329e242f3b00653570d8`。
- `M` [.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json)：报告或工位文档；SHA256 `0da435dff6321ab1f7443fc2b8a54ca3d7a5a9984fec746bc9c8c6f792a91dac`。
- `M` [.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.md)：报告或工位文档；SHA256 `6661c4ed98b05bafe6f747d03c1d6389f9730e4c77c311fff07368442b077d8d`。
- `M` [rust/src/bin/s_structure_session.rs](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/rust/src/bin/s_structure_session.rs)：产品代码；SHA256 `1d19627dfed365b0782cc62651773ece7a9afd2a2df9905c8428dbc3786516da`。
- `M` [s_session/browser/index.html](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/browser/index.html)：产品代码；SHA256 `ce4732b38a04c034cea336ba13e78db1f184220f333d2470674a937d2fb97d77`。
- `M` [s_session/s_readonly_server.py](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/s_readonly_server.py)：产品代码；SHA256 `f01988cf118b874b9f4b2c8221b4a00f2dd53a5df6f0f80c1675f39522405578`。

声明：**可编译（复用该提交历史检查声明）**。证据强度：历史作者检查声明 + 本輪Git源码字节绑定；未从此早期报告取得独立原始cargo receipt，不冒称本轮新编译。

应读该提交的 `.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json`（报告Git blob `fc1a9752bb3fa4a563fe3ea405ed9bd86dce9879`，报告SHA256 `0da435dff6321ab1f7443fc2b8a54ca3d7a5a9984fec746bc9c8c6f792a91dac`），不能用最终同名报告代替历史版本。字段 `/checks/cargo_check=0`、`/checks/bin_tests=23 passed / 0 failed`。登记的四份产品文件逐项SHA匹配；完整登记值与JSON pointer见伴随JSON该提交 `evidence[0].source_hash_checks`。

### 6. 58dab231e2210e1ba1e9634c936807d8492e218b

fix(structure): #1371——必需meta完整代链生命周期域与读写统一校验

- `M` [.chanlun/agent-roster-2026-09-10.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/agent-roster-2026-09-10.md)：报告或工位文档；SHA256 `337650a245f38e96c254a25b072575c8035f698709252cd43af8fe9b1b8e2598`。
- `M` [.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json)：报告或工位文档；SHA256 `bb20cc87b7316e0e8ef101c93e7e757f04a71c2b045762be1964b2b926dcd607`。
- `M` [.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.md)：报告或工位文档；SHA256 `f3e131203c9e15f75ba9055df4002bca19dd29dd661409cf5c693c05b042e16e`。
- `M` [rust/src/bin/s_structure_session.rs](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/rust/src/bin/s_structure_session.rs)：产品代码；SHA256 `07a0b061a6d7b884af368929005d21e50fdc02a34897e3e18f78ef56c40b096f`。
- `M` [s_session/browser/index.html](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/browser/index.html)：产品代码；SHA256 `13402a4011034e624de34b4137696a3dd8d3de2e1b35ec854e3a2331ca1591ac`。
- `M` [s_session/s_readonly_server.py](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/s_readonly_server.py)：产品代码；SHA256 `9a3f648cc49ea5622f3ab821bd66b0027f17adb447f4ede9835fffd236ba50d6`。

声明：**可编译（复用该提交历史检查声明）**。证据强度：历史作者检查声明 + 本輪Git源码字节绑定；未从此早期报告取得独立原始cargo receipt，不冒称本轮新编译。

应读该提交的 `.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json`（报告Git blob `03ae456f6fe540aeb0b5dd7de16e5928043b5bac`，报告SHA256 `bb20cc87b7316e0e8ef101c93e7e757f04a71c2b045762be1964b2b926dcd607`），不能用最终同名报告代替历史版本。字段 `/checks/cargo_check=0`、`/checks/bin_tests=26 passed / 0 failed`。登记的四份产品文件逐项SHA匹配；完整登记值与JSON pointer见伴随JSON该提交 `evidence[0].source_hash_checks`。

### 7. 8b726a57f98974c451ad0490066150afb9fe8e38

fix(structure): #1371——同代根元组完整一致与空输入frontier合法

- `M` [.chanlun/agent-roster-2026-09-10.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/agent-roster-2026-09-10.md)：报告或工位文档；SHA256 `0d83ffc1d3ce13b354301ed3840e4eba0fe2aa952183649bd1481061df781c50`。
- `M` [.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json)：报告或工位文档；SHA256 `a922bc2b330400125a093ca91e9d28803a28271fbe54932489df2b962247a237`。
- `M` [.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.md)：报告或工位文档；SHA256 `6e5151c2dcd3359241e7ae3c0c82e2379e68732d362a54f5639ee4c96747f199`。
- `M` [rust/src/bin/s_structure_session.rs](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/rust/src/bin/s_structure_session.rs)：产品代码；SHA256 `423e847812f7de2f75e7706a1a3eefcaa155bd16c7f44b7e1c9989fe3dfebc7a`。
- `M` [s_session/s_readonly_server.py](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/s_readonly_server.py)：产品代码；SHA256 `03718ffd3b6e2aaa26e56fd9185328e79ee7fa4f7c7a4092fe2b99fee09bd4ac`。

声明：**可编译（复用该提交历史检查声明）**。证据强度：历史作者检查声明 + 本輪Git源码字节绑定；未从此早期报告取得独立原始cargo receipt，不冒称本轮新编译。

应读该提交的 `.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json`（报告Git blob `26884ced98ecee9fa7aa3aad8afef825d09c4d57`，报告SHA256 `a922bc2b330400125a093ca91e9d28803a28271fbe54932489df2b962247a237`），不能用最终同名报告代替历史版本。字段 `/checks/cargo_check=0`、`/checks/bin_tests=28 passed / 0 failed`。登记的四份产品文件逐项SHA匹配；完整登记值与JSON pointer见伴随JSON该提交 `evidence[0].source_hash_checks`。

### 8. 6eeed76e3e12f1ffc3fdc62e23e60f2a3084c37b

fix(structure): #1371——Delta内层payload与外层11列同字段深等对应

- `M` [.chanlun/agent-roster-2026-09-10.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/agent-roster-2026-09-10.md)：报告或工位文档；SHA256 `f356aff6ac7a50ac7557d4740b68d6f511248730431c3a81aeed14b8dc15ddde`。
- `M` [.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json)：报告或工位文档；SHA256 `1494c1f0d9b792b688f56c951df4350bfb3d7bf1716b7b8d3888169ae3ee894a`。
- `M` [.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.md)：报告或工位文档；SHA256 `32945b648013af1a71cc3ec57b728fced964f999ff2acbb6f47ef84578ed547a`。
- `M` [rust/src/bin/s_structure_session.rs](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/rust/src/bin/s_structure_session.rs)：产品代码；SHA256 `addb492ac2c1888904f2ab2d9789ae9a7ebbfc8e0037e4d822f73b67be333be8`。
- `M` [s_session/s_readonly_server.py](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/s_readonly_server.py)：产品代码；SHA256 `526b9672dd00d51e3cf2b5ac4263243b53d139c7228f91c82b53a726a457c8cd`。

声明：**可编译（复用该提交历史检查声明）**。证据强度：历史作者检查声明 + 本輪Git源码字节绑定；未从此早期报告取得独立原始cargo receipt，不冒称本轮新编译。

应读该提交的 `.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json`（报告Git blob `1508961848c9141d71bc1bf5d5eb286fe2025234`，报告SHA256 `1494c1f0d9b792b688f56c951df4350bfb3d7bf1716b7b8d3888169ae3ee894a`），不能用最终同名报告代替历史版本。字段 `/checks/cargo_check=0`、`/checks/bin_tests=29 passed / 0 failed`。登记的四份产品文件逐项SHA匹配；完整登记值与JSON pointer见伴随JSON该提交 `evidence[0].source_hash_checks`。

### 9. 4a3da3787bdd6e783f3c7eee63dec544b7dc3e68

fix(structure): #1371——input_frontier与supersedes/证据计数的精确整数wire

- `M` [.chanlun/agent-roster-2026-09-10.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/agent-roster-2026-09-10.md)：报告或工位文档；SHA256 `265b6a2ea64eeaf76293864859ddc0958dff3c008afdaddf71ce882910c62d91`。
- `M` [.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json)：报告或工位文档；SHA256 `d02c06f8676fd7d88bccfa30b9da17880a12434433b8c5102d5d159d4ed9da7f`。
- `M` [.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.md)：报告或工位文档；SHA256 `5dffb1057dd7459f90bd1eaaf7163e36479cfc60afe597b990d3b4bd8e1f0a49`。
- `M` [rust/src/bin/s_structure_session.rs](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/rust/src/bin/s_structure_session.rs)：产品代码；SHA256 `c7813827bc3e43a253bf5d66fbf3ae0456baee1abbbc6868f51f2c77506da861`。
- `M` [s_session/s_readonly_server.py](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/s_readonly_server.py)：产品代码；SHA256 `a9475590c77c3a48e65de6b54b386c6f29085c830060de104f265e89abc892f1`。

声明：**可编译（复用该提交历史检查声明）**。证据强度：历史作者检查声明 + 本輪Git源码字节绑定；未从此早期报告取得独立原始cargo receipt，不冒称本轮新编译。

应读该提交的 `.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json`（报告Git blob `4b43bfc15fb9240e2563a42575efa53342e9cc7c`，报告SHA256 `d02c06f8676fd7d88bccfa30b9da17880a12434433b8c5102d5d159d4ed9da7f`），不能用最终同名报告代替历史版本。字段 `/checks/cargo_check=0`、`/checks/bin_tests=30 passed / 0 failed`。登记的四份产品文件逐项SHA匹配；完整登记值与JSON pointer见伴随JSON该提交 `evidence[0].source_hash_checks`。

### 10. 65fb5e72bbee1de6656ba4b902dd506ae526195f

fix(structure): #1371——最终wire边界安全递归整数转十进制文本投影

- `M` [.chanlun/agent-roster-2026-09-10.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/agent-roster-2026-09-10.md)：报告或工位文档；SHA256 `5a97ba281fff60b0576502c013a8d44043cbc10b8a32c6bdd65bc4c605b56117`。
- `M` [.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json)：报告或工位文档；SHA256 `43153824e4eca056d5840b884a05f1735393c95c5714223622f1d097498aaea7`。
- `M` [.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.md)：报告或工位文档；SHA256 `6d210bf5205f973b365abda980e313ec23ac9d1c92b7bd7f2310b653d88ae25d`。
- `M` [rust/src/bin/s_structure_session.rs](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/rust/src/bin/s_structure_session.rs)：产品代码；SHA256 `0718729cf3535b128ff995d1f6aad75b5385c9042140f5a053db970a75f3dfb2`。
- `M` [s_session/s_readonly_server.py](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/s_readonly_server.py)：产品代码；SHA256 `ee29bf801dfec475f603ab9c2aa98b616fdbceb33f3d3a573d18e9d05146e4b6`。

声明：**可编译（复用该提交历史检查声明）**。证据强度：历史作者检查声明 + 本輪Git源码字节绑定；未从此早期报告取得独立原始cargo receipt，不冒称本轮新编译。

应读该提交的 `.chanlun/review-results/issue1371-tb01-b/IMPLEMENTATION-REPORT.json`（报告Git blob `05b8bdc3b438430e37fd3808ae3cb7463f3cdd04`，报告SHA256 `43153824e4eca056d5840b884a05f1735393c95c5714223622f1d097498aaea7`），不能用最终同名报告代替历史版本。字段 `/checks/cargo_check=0`、`/checks/bin_tests=31 passed / 0 failed`。登记的四份产品文件逐项SHA匹配；完整登记值与JSON pointer见伴随JSON该提交 `evidence[0].source_hash_checks`。

### 11. 257452c5fd02e0b28b385b7bca4c327b76ea6aa0

docs(review): #1371——R3独立复审与可复核反例（FAIL）

- `A` [.chanlun/review-results/issue1371-tb01-b/review-r3.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/review-r3.json)：报告或工位文档；SHA256 `51da189de900b8527739487de0a54d6c048e5399ec9dff568cd08b98f883e5fe`。
- `A` [.chanlun/review-results/issue1371-tb01-b/review-r3.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/review-r3.md)：报告或工位文档；SHA256 `006cb682f7f885c9d966a4cd7ce7d05870e127f1e5f26efdffb28bff3174f326`。

声明：**纯文档豁免**。证据强度：本轮只读Git逐文件分类。

### 12. 381f5e1c670cf97b6a1491a3ec3c03c30df406d0

fix(structure): #1371——统一发布根和完整载荷校验并保留观察首版

- `M` [rust/src/bin/s_structure_session.rs](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/rust/src/bin/s_structure_session.rs)：产品代码；SHA256 `e26d69cb72832204168feb2c19b226f8c7c323e65c81012a57aff75d587a2f85`。
- `M` [s_session/browser/index.html](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/browser/index.html)：产品代码；SHA256 `b4577c16a21ea6ce11f7e0a03d93eecb3042a400cced483662d6ecf73e9919a4`。
- `M` [s_session/s_readonly_server.py](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/s_readonly_server.py)：产品代码；SHA256 `b705c0a7bcde38613f9e6acd7dc04cede440b56fa5aa40b892906d8baf6a59c7`。
- `A` [s_session/tests/r9_consumer.cjs](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/tests/r9_consumer.cjs)：测试代码；SHA256 `c73c252360ad417e724fbc1ca9c9b42febf26f6418deb61836c63f38d6b93c78`。
- `A` [s_session/tests/r9_contract.py](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/tests/r9_contract.py)：测试代码；SHA256 `39f0e9cee3d581a1981f882ed057f08f6c057d213dd4d2b6e393bc048f8faf11`。

声明：**可编译（复用R9历史构建收据）**。证据强度：历史实际receipt + 本轮附件完整性及源码绑定核对。

### 13. 667ce698eacd8c8f50ec5846f24c779b492010a8

docs(structure): #1371——归档R9修复证据与原作者交棒报告

- `A` [.chanlun/review-results/issue1371-tb01-b/implementation-r9.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/implementation-r9.json)：报告或工位文档；SHA256 `94f3a8e9024b1683a7a909f0e811d9fb51088b5f0b389734a985b77721e934ef`。
- `A` [.chanlun/review-results/issue1371-tb01-b/implementation-r9.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/implementation-r9.md)：报告或工位文档；SHA256 `87ff8cc1961573fa1bb3b109553966ac753dc4d353f86341746f5616c68038fe`。

声明：**纯文档豁免**。证据强度：本轮只读Git逐文件分类。

### 14. 41ed7163e8bc15bc7cc5af1fb06af22d98a6589c

docs(review): #1371——R4复审与profile和初始历史反例（FAIL）

- `A` [.chanlun/review-results/issue1371-tb01-b/review-r4.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/review-r4.json)：报告或工位文档；SHA256 `c8d16ecd415c8e74d90273e99eb86f9e66b2aac38d6183fdcfa3db2b50983fbc`。
- `A` [.chanlun/review-results/issue1371-tb01-b/review-r4.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/review-r4.md)：报告或工位文档；SHA256 `85b5158a56051ab59468917b0fc0c64a6603906f79fa6ebb0a4fa2a4c45468e7`。

声明：**纯文档豁免**。证据强度：本轮只读Git逐文件分类。

### 15. a0b6f17a3def8f27f2ff936980c97e34c9da1c0e

fix(structure): #1371——封闭profile绑定并稳定cut0历史投影

- `M` [rust/src/bin/s_structure_session.rs](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/rust/src/bin/s_structure_session.rs)：产品代码；SHA256 `9450f2a9642be70dc5a18f7044fa07312d81d126e25d512e50921105a1f25901`。
- `M` [s_session/browser/index.html](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/browser/index.html)：产品代码；SHA256 `9c3d2c3703b9e765f3d65e2cd76759ff5716e83781fd49c6ec5457aad8565498`。
- `M` [s_session/s_readonly_server.py](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/s_readonly_server.py)：产品代码；SHA256 `9dad379ff1aaa1ff6f9315a877249f4b9003aa39c7d48fdba8016e534c27c65c`。
- `M` [s_session/tests/r9_consumer.cjs](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/tests/r9_consumer.cjs)：测试代码；SHA256 `8d8a467640e6a98b2c42bacc3f1c70f2ac778bf68ea307329ab00299e727557d`。
- `M` [s_session/tests/r9_contract.py](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/tests/r9_contract.py)：测试代码；SHA256 `1480198f9c9e248bf45b9c4dca54ab2430d889dda26f251a5fcabf6adee51a82`。

声明：**可编译（复用R10首个代码提交前检查收据）**。证据强度：历史原执行/提交回执对应 + 本轮产品源码等同性核对。

### 16. 50f22cac47b25685a8a24088cd3b64b3a82e2d4f

test(structure): #1371——为profile故障逐副本补健康HTTP对照

- `M` [s_session/tests/r9_contract.py](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/tests/r9_contract.py)：测试代码；SHA256 `89eeaff7430a098c4abaf239b71f0d684da6dd1f53038dba9d2f84667cdadc2f`。

声明：**可编译（仅测试代码，构建输入同字节复用；不是文档豁免）**。证据强度：历史实际receipt + 本轮源码绑定/同字节核对。

### 17. 3ddb5c2b7f9f250861640d40d779d1a6a5fd8955

docs(structure): #1371——归档R10绑定与初始历史修复证据

- `A` [.chanlun/review-results/issue1371-tb01-b/implementation-r10.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/implementation-r10.json)：报告或工位文档；SHA256 `1ff78159548dff927dec21fcbb7a07ee5fa59d992caefac8a69d01e9eb85378f`。
- `A` [.chanlun/review-results/issue1371-tb01-b/implementation-r10.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/implementation-r10.md)：报告或工位文档；SHA256 `438ac54a1e884a22f45a6fa26f51706a6b8b4d306fab5a8dedc3180c81fc107d`。

声明：**纯文档豁免**。证据强度：本轮只读Git逐文件分类。

### 18. a60fdd57e5ebb0e7d6a15b7e1f30994720269ed8

docs(review): #1371——R5续审与请求切面模式绑定反例（FAIL）

- `A` [.chanlun/review-results/issue1371-tb01-b/review-r5.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/review-r5.json)：报告或工位文档；SHA256 `58d99aece9208a525f3eb1963db6775d71fb67e119e5fcc1235d13547d893f7b`。
- `A` [.chanlun/review-results/issue1371-tb01-b/review-r5.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/review-r5.md)：报告或工位文档；SHA256 `96834d61582bfaf3d1ef1366383c8e3a68c9c714f5fedcf0fb00f6285e6e2dce`。

声明：**纯文档豁免**。证据强度：本轮只读Git逐文件分类。

### 19. 6a4e69e60ffd1baca490f870622c098a14fe7b41

fix(observation): #1371——校验本次请求的模式与指定cut后再提交

- `M` [s_session/browser/index.html](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/browser/index.html)：产品代码；SHA256 `a04ddb1c67e26207cbc9ec4bd72f7cb36fcac30accf4c3f1a49e5ee10e5de94b`。
- `M` [s_session/tests/r9_consumer.cjs](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/tests/r9_consumer.cjs)：测试代码；SHA256 `c375406cf09a092d451c7b09050fa3eda378b3087ee96d8c033edcc1842d119d`。

声明：**可编译（HTML/Node检查已执行，未变后端复用）**。证据强度：历史实际receipt + 本轮附件完整性及源码绑定核对。

### 20. 4922543038fbcfef1a4d711a306708ace3f62e88

docs(observation): #1371——归档R11请求绑定验证与原作者交棒

- `A` [.chanlun/review-results/issue1371-tb01-b/implementation-r11.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/implementation-r11.json)：报告或工位文档；SHA256 `63de32660497813d0830eda838775d563428e0b121d0b86d26b0ac110a2fd991`。
- `A` [.chanlun/review-results/issue1371-tb01-b/implementation-r11.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/implementation-r11.md)：报告或工位文档；SHA256 `49e9202a4ed48fdd70d01aed58865d284647d6477bf580542d6d55c45456a4cb`。

声明：**纯文档豁免**。证据强度：本轮只读Git逐文件分类。

### 21. e5b03629adde165610161b4df3e2d62d22163568

fix(observation): #1371 校验替代端点并绑定 Gap 重建会话

- `A` [.chanlun/agent-roster-2026-09-12.md](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/agent-roster-2026-09-12.md)：报告或工位文档；SHA256 `83a8be0eca729e8a821f01d1130cdc348888e12fe3e0e3433aa76aacb3ac0378`。
- `M` [s_session/browser/index.html](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/browser/index.html)：产品代码；SHA256 `dbd5465fb74bd45f5df71b9c7b824124f91130606392d89c2a8a752e42e20afe`。
- `A` [s_session/tests/fixtures/r12_consumer.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/tests/fixtures/r12_consumer.json)：测试数据；SHA256 `1b0020a69a897da000056666907ef606a6d0b2ed35bac97ec65d625131e25745`。
- `A` [s_session/tests/r12_consumer.cjs](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/s_session/tests/r12_consumer.cjs)：测试代码；SHA256 `228a9cb06b385a124b1cc4bbccaefbb85f252868777381f99281224b29c99e39`。

声明：**可编译（R12实际HTML/Node执行通过，后端构建同输入复用）**。证据强度：本轮根已执行的R12函数检查 + 本工位只读字节核验；后端使用历史构建复用，不声称新native/build。

## 精确证据入口与复用边界

### R9

报告：[implementation-r9.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/implementation-r9.json)；`/source_commit=381f5e1c670cf97b6a1491a3ec3c03c30df406d0`。报告SHA256 `94f3a8e9024b1683a7a909f0e811d9fb51088b5f0b389734a985b77721e934ef`。

嵌入字段 `/evidence_artifacts/r9-commands.json`，编码 `xz+base64`；本次纯解码后 **900867 bytes**，SHA256 `58df8155c49a8936fa3451aacfbbfdca925dadf037d083d7c37eb9bcd29d0972`，与登记完全一致。下列是历史执行记录，本次没有运行这些命令。

| 解码后记录pointer | 实际命令 | exit |
|---|---|---|
| `/34` | `cargo build --locked --features s_session --bin s_structure_session --jobs 1` | 0 |
| `/35` | `cargo test --locked --features s_session --bin s_structure_session --jobs 1` | 0 |
| `/36` | `cargo check --locked --features s_session --bin s_structure_session --jobs 1` | 0 |
| `/52` | `/home/agent/.local/share/uv/python/cpython-3.11.16-linux-aarch64-gnu/bin/python3.11 -c import ast,pathlib,sys; [ast.parse(pathlib.Path(p).read_text()) for p in sys.argv[1:]] /home/agent/workspace/s_session/s_readonly_server.py /home/agent/workspace/s_session/tests/r9_contract.py` | 0 |
| `/53` | `bash -n /home/agent/workspace/s_session/launch_s.sh` | 0 |
| `/59` | `node /home/agent/workspace/s_session/tests/r9_consumer.cjs /tmp/issue1371-r9-082xvfgs/delivery /tmp/1371-r9-r3-ir8sef7t /home/agent/workspace` | 0 |
| `/62` | `git commit（原输出绑定 381f5e1c67）` | 0 |

报告 `/binding/source_sha256` 的六项与所指源码提交全部吻合，精确字段和值见伴随 JSON `source_reports.R9.source_hash_checks`。

### R10

报告：[implementation-r10.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/implementation-r10.json)；`/source_commit=50f22cac47b25685a8a24088cd3b64b3a82e2d4f`。报告SHA256 `1ff78159548dff927dec21fcbb7a07ee5fa59d992caefac8a69d01e9eb85378f`。

嵌入字段 `/evidence_artifacts/commands.json`，编码 `xz+base64`；本次纯解码后 **338120 bytes**，SHA256 `657849c5136cec4342a1439b4ac8c2a042cb8b51781b2cfffe92e2020c71eb67`，与登记完全一致。下列是历史执行记录，本次没有运行这些命令。

| 解码后记录pointer | 实际命令 | exit |
|---|---|---|
| `/9` | `cargo build --locked --features s_session --bin s_structure_session --jobs 1` | 0 |
| `/10` | `cargo test --locked --features s_session --bin s_structure_session --jobs 1` | 0 |
| `/11` | `cargo check --locked --features s_session --bin s_structure_session --jobs 1` | 0 |
| `/14` | `/home/agent/.local/share/uv/python/cpython-3.11.16-linux-aarch64-gnu/bin/python3.11 /home/agent/workspace/s_session/tests/r9_contract.py /tmp/issue1371-r10-wwkstffx/final --r10` | 0 |
| `/15` | `/home/agent/.local/share/uv/python/cpython-3.11.16-linux-aarch64-gnu/bin/python3.11 /tmp/issue1371-r10-wwkstffx/node-driver.py /home/agent/workspace/s_session/tests/r9_contract.py /tmp/issue1371-r10-wwkstffx/final` | 0 |
| `/20` | `/home/agent/.local/share/uv/python/cpython-3.11.16-linux-aarch64-gnu/bin/python3.11 -c import ast,pathlib,sys;[ast.parse(pathlib.Path(p).read_text()) for p in sys.argv[1:]] /home/agent/workspace/s_session/s_readonly_server.py /home/agent/workspace/s_session/tests/r9_contract.py` | 0 |
| `/21` | `bash -n /home/agent/workspace/s_session/launch_s.sh` | 0 |
| `/27` | `git commit（原输出绑定 a0b6f17a3d）` | 0 |
| `/28` | `/home/agent/.local/share/uv/python/cpython-3.11.16-linux-aarch64-gnu/bin/python3.11 /home/agent/workspace/s_session/tests/r9_contract.py /tmp/issue1371-r10-wwkstffx/final-v2 --r10` | 0 |
| `/29` | `/home/agent/.local/share/uv/python/cpython-3.11.16-linux-aarch64-gnu/bin/python3.11 /tmp/issue1371-r10-wwkstffx/node-driver.py /home/agent/workspace/s_session/tests/r9_contract.py /tmp/issue1371-r10-wwkstffx/final-v2` | 0 |
| `/30` | `/home/agent/.local/share/uv/python/cpython-3.11.16-linux-aarch64-gnu/bin/python3.11 -c import ast,pathlib,sys;ast.parse(pathlib.Path(sys.argv[1]).read_text()) /home/agent/workspace/s_session/tests/r9_contract.py` | 0 |
| `/32` | `git commit（原输出绑定 50f22cac47）` | 0 |

报告 `/binding/source_sha256` 的六项与所指源码提交全部吻合，精确字段和值见伴随 JSON `source_reports.R10.source_hash_checks`。

### R11

报告：[implementation-r11.json](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/implementation-r11.json)；`/source_commit=6a4e69e60ffd1baca490f870622c098a14fe7b41`。报告SHA256 `63de32660497813d0830eda838775d563428e0b121d0b86d26b0ac110a2fd991`。

嵌入字段 `/evidence_artifacts/commands.json`，编码 `xz+base64`；本次纯解码后 **28268 bytes**，SHA256 `e40c9546961d41560d622e03be5d1862648e75006f5bdfc436914bd35c4cac4c`，与登记完全一致。下列是历史执行记录，本次没有运行这些命令。

| 解码后记录pointer | 实际命令 | exit |
|---|---|---|
| `/0` | `cargo build --locked --features s_session --bin s_structure_session --jobs 1` | 0 |
| `/4` | `node --check /home/agent/workspace/s_session/tests/r9_consumer.cjs` | 0 |
| `/5` | `node --check /tmp/issue1371-r11-nbsi3u4h/page-script.js` | 0 |
| `/6` | `/home/agent/.local/share/uv/python/cpython-3.11.16-linux-aarch64-gnu/bin/python3.11 /tmp/issue1371-r11-nbsi3u4h/driver.py /tmp/issue1371-r11-nbsi3u4h/source green-final` | 0 |
| `/7` | `node /home/agent/workspace/s_session/tests/r9_consumer.cjs /tmp/issue1371-r11-nbsi3u4h/source /tmp/issue1371-r11-nbsi3u4h/r3 /home/agent/workspace` | 0 |
| `/8` | `/home/agent/.local/share/uv/python/cpython-3.11.16-linux-aarch64-gnu/bin/python3.11 /tmp/issue1371-r11-nbsi3u4h/r10-driver.py /tmp/issue1371-r11-nbsi3u4h/source r10` | 0 |

报告 `/binding/source_sha256` 的六项与所指源码提交全部吻合，精确字段和值见伴随 JSON `source_reports.R11.source_hash_checks`。

R10 特别说明：最终报告 `/source_commit` 为 `50f22cac47b25685a8a24088cd3b64b3a82e2d4f`。`commands.json#/27` 原提交回执对应 `a0b6f17a3d`，其前面 `/9`、`/11`、`/14`、`/20` 等已完成构建/检查/脚本验证；`/32` 对应后一个测试提交，之前 `/28`、`/29`、`/30` 另验最终 Python 脚本。两提交的产品及 Node 测试逐字相同；Python 测试不同，不能把最终脚本 hash 倒贴给前一个提交。此区别与[作者复用说明](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1371-tb01-b/implementation-r10.md:98)一致。

### R12 本轮根执行与后端复用

- [EXECUTION.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R7-CONSUMER-PROBES/EXECUTION.json)：`/green/exit_code=0`；`/r11_captured_response_replay/exit_code=0`。
- [GREEN.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R7-CONSUMER-PROBES/GREEN.json)：`/cases` 共10项全部 `passed=true`，`/failed=[]`。
- [VALIDATION.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R7-CONSUMER-PROBES/VALIDATION.json)：`/candidate=e5b03629adde165610161b4df3e2d62d22163568`，`/legacy_cases_pass=28`；`/files` 每份原件 bytes/SHA本次已逐项核对，三份R12产品/测试文件与Git字节一致。
- [REPLAY-RECEIPT.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R7-CONSUMER-PROBES/r11-replay/REPLAY-RECEIPT.json)：HTML与旧HTTP字节来源绑定；28项是归档响应重放，不是新的实际HTTP或native运行。
- R12 HTML SHA256：`dbd5465fb74bd45f5df71b9c7b824124f91130606392d89c2a8a752e42e20afe`；Node永久测试：`228a9cb06b385a124b1cc4bbccaefbb85f252868777381f99281224b29c99e39`；fixture：`1b0020a69a897da000056666907ef606a6d0b2ed35bac97ec65d625131e25745`。
- 完整 `rust` 子树 Git OID `e53add416786d0a99e2eff943b87671a48b639ec` 与 `a0b6f17a3def8f27f2ff936980c97e34c9da1c0e` 完全相同。Rust入口、Python服务、launcher、Cargo.toml/lock各文件亦同字节，复用R10/R11后端构建收据；详细哈希在伴随JSON最后一项 `backend_build_input_equivalence`。

本工位只做Git对象读取、JSON/压缩附件纯解析及字节哈希核对，没有执行构建、测试、服务、浏览器或外部写入。VALIDATION 的 GUI=PENDING 是该文件形成时状态，本声明不据此判断后续GUI是否完成；GUI结论由根的独立材料承担。

## 后续纯报告提交追加规则

根后续每增加一个纯报告提交：记录全SHA、父SHA、实际变更文件清单并逐文件确认仅报告/文档；追加“纯文档豁免”，用Git确认产品、测试代码/fixture和构建输入仍与e5b03629adde165610161b4df3e2d62d22163568一致，更新最终head与总数，不回写本21项为新构建。若含任何产品/测试/构建输入变化，不适用此规则，应追加对应可编译依据并按最终代码评审范围处理。

## 证据等级与尚未宣称事项

- 前8产品提交依据各自历史作者cargo_check声明和源码哈希，未取得额外独立原始cargo回执；不提高该证据等级。
- R9/R10/R11是归档历史命令实际记录；本次只解码，不新启动任何构建或测试。
- R12十项/二十八项由根本轮执行；本工位未独立重跑。
- 本声明不替代GUI或独立产品审查，不改变main/push/GitHub发布授权，不声称合入、CI通过或关票。
- 引用D下原件为本地交付材料；未来入仓发布时须同步落可达报告实体/必要附件，不能只以此绝对路径满足关票入仓规则。

机器清单：[R12-COMMIT-DECLARATIONS.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-COMMIT-DECLARATIONS.json)。
