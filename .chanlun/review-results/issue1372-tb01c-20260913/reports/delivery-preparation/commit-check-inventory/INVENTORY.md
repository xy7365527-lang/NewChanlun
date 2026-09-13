逐 commit 交付事实索引已完成。范围固定为 `e46cf3bca6a67da821f9ab046507563b8133f6d9..3ff8c47ea4ab2aeb4672722e204d9f20b35f9b7f`，共18个提交；仅`68198a6cda`为纯文档可编译性豁免。其余提交分别列举真实构建与执行证据，不从末端测试成功推定中间版本功能正确。

本工位未运行cargo或产品测试，仅对Git原件作语法检查并核旧收据。5种Rust源码树与既有构建相同；Python/JS/shell的低成本检查不能代替依赖导入或真实运行。尚未上CI，不能声明远端通过。

| 提交 | 类型 | 可编译性与已有适用证据 | 独评/限制 |
|---|---|---|---|
| `387a4cbc66` | Rust 服务/控制协议/持久投递 | P1 最终 native build exit0；最终目标10测通过。55全套发生在 nonce 最后增补之前，不称最终56全套已运行。 | Rust构建已证；既有P2独评覆盖其累积服务面并发现后续修复项。 |
| `32c907a637` | Rust 新鲜线性历史核 | P2 release build exit0、59全套及13目标通过；16消息全表对拍仅小域。 | Rust构建已证；P2存在4项/5原例完整性缺陷，由73ef75b修复。 |
| `0e83b681da` | Python Q 审计/HTTP/分页/Watch/测试 | 23 Q测试、v2小域通过（v1 R14与P1二进制）；不能称该root整树对P2已整合验证。 | Python语法可编译；Q原数值缺口后由609716c修复，旧launcher与R9必填参数兼容性后来补齐。 |
| `73ef75bcef` | Rust 全控制行/epoch/raw消息覆盖修复 | P3 release build exit0；60全套之后仅新增1条测试，最终1测/60过滤通过。 | Rust构建已证；独立34 CLI产品断言通过，原目录sidecar增量导致保全检查FAIL须保留。 |
| `609716c07b` | Python raw规范signed-i64与v2恢复门 | 24 Q测试、78数值组合(24健康54拒绝)、P2 v2小域通过。 | Python语法可编译；独评原三坏值和九合法值复验通过；非整C。 |
| `0b219fd47c` | Python 已核proof紧凑保留 | 25测试、472完整公开值、6外部坏库；原配置G88可注册缓存。 | Python语法可编译；无此提交独立完整评审收据，后续memo独评含继承依赖。旧v2测试tables访问要到423提交才整合修正。 |
| `8bbaaffd42` | Rust 同源历史memo/资源与响应期限 | P4 release源收据；64通过0失败1忽略；两版S-only160/500ms及15表2860行对拍。 | Rust构建已证；P4独评发现跨期入队缺陷，b924a71修复。 |
| `b924a71fb1` | Rust 每轮入队前核绝对期限 | 65通过0失败1忽略；release build exit0。 | Rust构建已证；独立原函数5臂通过，未冒称全服务调度。 |
| `bf28062adb` | Python 全鲜索引下复用sealed源/压缩完整Delta | 30测试；G88 651/G160 553公开值、160原Delta原文恢复；G96..160每代捕获测时。 | Python语法可编译；独立173值/13坏库通过。实施测时不是HTTP全页或C验收。 |
| `42389dd2a8` | Python控制/driver/比较器；浏览器；shell；冻结fixture | 分组件已有37 Python检查日志、UI30+R12十项；root最终v2小域5ingest/21行/8坏控制。严格源绑定见JSON，不能把零件测试总和当此整树通过。 | Python/JS/shell语法检查；该历史版本真实stop回执与原生fetch随后被打回，分别由a2fc4a9和1a57fb6修复。 |
| `a2fc4a957a` | Python driver单服务stop列表解包/测试 | 后续同字节observer集测试；独立真实CLI对自有sleep替身SIGKILL及六反例，共9项通过。 | Python语法可编译；真实产品故障轨迹仍单独验收。 |
| `1a57fb684e` | 浏览器fetch接收者绑定/回归 | 32 Node测试通过，源两文件SHA均在RESULT；原生Chrome证据另归根。 | JS语法已核；实施者报告不是本工位独立批准。 |
| `2317369c3d` | shell旧launcher显式Q资源/epoch参数 | 三负控拒绝且不创建库；真实legacy启动/三个HTTP端点/停止成功；launcher源SHA具名。 | bash语法已核；独立LEGACY-REVIEW绑定2317369及launcher SHA，有界通过；非R4后新整树验收。 |
| `68198a6cda` | 纯文档README | 仅s_session/README.md，Git差异逐条枚举；不含代码/配置。 | 纯文档可编译性豁免。 |
| `14c15bd44e` | Python完整核验后紧凑Ready与控制器；README/测试 | 24 controller、30 Q测试通过；G160 ready448B、state1144017B，真实start/stop成功。 | Python语法可编译；独立Ready增量报告绑定该commit。 |
| `932de089e9` | Python R9/R10测试入口显式资源/epoch与进程清理 | 真实R9:331 CLI+324 HTTP；R10:199 CLI+392 HTTP；65坏库零写入，88自有Q均回收，两个入口exit0。 | Python语法可编译；这是测试修复作者事实收据，单独独评此尾增量未定位。 |
| `18ee720a02` | Python无损Delta字符串编码memo/测试 | 31 Q测试、v2小域5/21/8；816完整公开值、真实SQLite changedVersion及旧159代坏文拒收。 | Python语法可编译；编码memo独评正在其他工位执行，索引时尚无最终报告。 |
| `3ff8c47ea4` | GitHub Actions CI接入观察器Python/Node测试 | 18ee已有observer命令本地执行exit0；该提交只改CI配置，未发生新的远端CI。 | 非纯文档。YAML结构/命令语法只作低成本证据，GitHub执行环境与远端CI未证明。 |

证据与精确文件归属：`INVENTORY.json`逐项列变更路径、Git blob、SHA、语法结果及既有报告；`EVIDENCE-INDEX.json`列原件SHA及具名源码记录；`RUST-BUILD-BINDINGS.json`核5个构建源的整个Rust树与source manifest；`SYNTAX-CHECKS.json`保留每份唯一Git原件的检查结果。

明确缺证：此清单不宣称每个历史commit的全部入口真实运行通过。`0e83b68`之后旧launcher/R9缺参数、`0b219fd`之后旧v2测试引用已精简tables、`42389dd`的stop回执与原生fetch均由后续提交修复；源码可编译不掩盖这些历史运行回归。`2317369`已有具名LEGACY-REVIEW独评；`932de08`尾增量独评待补；`18ee720`独评正在另一工位运行，未预写通过；`3ff8c47`CI接入实际runner运行未证明。

纯文档豁免依据为Git `diff-tree`只列README.md；CI YAML为可执行配置，不能归纯文档豁免。库存报告包含原P3目录保全检查FAIL及产品修复通过的分别表述，不把两种结论混写。

历史报告均保持原字节。本目录是交付准备事实索引，未入仓，不能据此宣称交付纪律第6条已经满足。

低成本语法结果：71份唯一Git输入中，58份适用语法解析，0失败；另按正式CPython3.11.15对全部36份唯一Python文件编译通过。各提交当时所有s_session Python文件均覆盖，未导入或执行。CI新增两条run命令bash -n通过；不等于GitHub工作流执行。
