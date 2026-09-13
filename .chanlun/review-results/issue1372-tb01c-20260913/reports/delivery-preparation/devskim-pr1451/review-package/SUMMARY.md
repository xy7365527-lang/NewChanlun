PR #1451 固定 `3ff8c47ea4ab2aeb4672722e204d9f20b35f9b7f` 的 DevSkim 门禁为 **FAIL**。本次 C 实际新增的 345 个词法声明没有确认产品安全缺陷：339 个具名摘要全部重算或核为 Git 对象，5 个地址用于有意本机控制/隔离测试，1 个 setTimeout 使用固定函数回调。没有新增待审项；实际安全缺陷总数仍未知，旧 strategy 待办继续保留。此结论不批准合入，不沿用 B 的单次扫描例外，也不适用于之后新增文档的 head。

[真实运行34751431403](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34751431403) 的扫描和普通 SARIF artifact 上传成功。gate 日志 L351 明确基线外11,235键（总19,496 raw），L11587 exit1。随后 L11588 另报步骤摘要1122k超过1024k；它是显示限制，不能解释掉实际差集失败。这里“zero-findings gate”的实际判据是现存 `ruleId|path|startLine` baseline 差集，不是简单要求全仓0命中。

| 计数 | e46主线旧扫描 | PR1451 |
|---|---:|---:|
| raw结果 | 19,151 | 19,496 |
| 唯一位置键 | 19,077 | 19,422 |
| baseline仍出现 | 8,189 | 8,187 |
| baseline未出现 | 12 | 14 |
| baseline外键 | 10,888 | 11,235 |
| baseline外raw | 10,931 | 11,278 |

相对e46新增353键、消失8键，19,069个共通键的匹配文本SHA多重集合以及所在Git blob均相同。8进8出属于回环地址行移，7条整行字节相同，R9连接行仅改变缩进。其中2个旧baseline内launcher地址因行号变化进入差集。故 **11,235 = 10,888继承债务 + 2旧baseline行移 + 345本C新增**；不能把基线外总数称为本C新增漏洞数，也不能把8个消失位置叫作安全修复。

| 新增规则/位置 | 键数 | 具名判断与验证 |
|---|---:|---|
| DS173237；operation-schedule.json | 320 | 160个request.payload_sha256等于具名raw-input.jsonl原行SHA（不含LF）；160个canonical_event_content_sha256_static等于同原行events[0]规范UTF8 JSON的SHA。全部相等。 |
| DS173237；MANIFEST.json | 12 | source_head为现存commit对象；4个source SHA按声明source_head下Git原blob重算，7个fixture文件SHA按固定head原blob重算，长度也相等。 |
| DS173237；RUNTIME-MANIFEST.json | 7 | 7个具名运行输入文件原Git字节的长度/SHA全部相等，含messages.jsonl、clock-plan与资源配置。 |
| DS162092；旧8处地址行移 | 8 | R9三处、readonly默认host一处、launcher四条展示地址；原语句/字面值保持。逐对定位见LINE-MOVES.json。 |
| DS162092；s_service_control.py:75、:77 | 2 | read_health固定连接127.0.0.1与声明端口，GET /api/ready，Host同址；属于独立S/Q控制器的有意本机传输。 |
| DS162092；tb01c_query_v2.py:125、tb01c_query_tests.py:361、test_tb01c_service_control.py:373 | 3 | 隔离测试绑定127.0.0.1、端口0；非外部debug依赖。 |
| DS172411；browser/tb01c-client.js:188 | 1 | 第一参数为源码固定()=>abort.abort()，不是字符串。它关闭同方法的AbortController；资源是正safe integer，deadline由begin产生。无网络内容进入代码求值。 |

DS173237的官方固定版本规则是带引号的30位以上十六进制词法匹配，本来就能命中普通hash字段；本次没有仅凭“看起来像SHA”排除凭据，而是将全部339个命中span对应字段与来源逐项验证。DS172411的官方规则为setTimeout手动审查，其正则并不解析回调类型；此处完整调用确为函数。DS162092只查localhost/127.0.0.1字面值，实际用途如上。三个规则均绑定扫描器 `1.0.90+fb2d676ce4` 对应上游 `fb2d676ce475a47c0338966bebd97a47ae566572` 原件，来源及SHA见RULE-SOURCES.json。353处均核snippet与固定Git原字节相同；产物列坐标实际以0起始截取，原SARIF列保留，gate只使用行号。

继承范围：B/PR1449已分诊9,233键全部保留语义（9,227同键、6处地址行移）；e46后续报告归档额外1,655键均在未变文档中。本次继承旧分类与具名反证，不重新全仓安全审计。[#1385精确承接](https://github.com/xy7365527-lang/NewChanlun/issues/1385#issuecomment-5648592872)仍为 `DS176209|rust/src/theta_v0/nautilus/strategy.rs|314`，文件blob与e46相同，继续needs_review；entry映射的实际影响要在OE09旧策略入口/订单归属范围核实，不能由C改交易语义或销项。当前8,187个baseline内键与扫描未覆盖面未重审，实际安全风险不能全部排除。

当前处置建议：新增345项没有证明应修的产品安全缺陷，不需要为这些已分类词法声明另开业务缺陷票。保持校验摘要、函数取消器及本机边界，不以修改字符串外形规避规则。若要调整误报治理或baseline口径，应另按扫描治理范围明确处理；此分诊不作该裁定。真实红灯仍需根/用户针对实际固定head明确处置，旧B批准不能自动覆盖C。

绑定：实际checkout为 `14071107dae23f041dababc3b6ea78b8e51355ba`，父为e46与3ff8c47；GitHub API checkout树和本地head树同为 `f77ef71a70c64b05b477d660799f360efe16e5d2`。gate、baseline、DevSkim工作流、Dockerfile、entrypoint与e46逐字相同。artifact ID `10315776783` 的zip SHA等于GitHub发布digest，下载来源/有效期/完整SARIF与日志SHA见ORIGINALS-INDEX.json；大SARIF和zip保留仓外，不进入报告包。

全部raw与键差分、353新增逐项字段用途已枚举；仅对所报新增上下文做有界人工核对。旧全量分诊、旧红灯和本次原件均未覆盖。未改产品/基线/忽略项/扫描配置，未启动新scanner或产品测试。本机PATH没有devskim/dotnet，用户dotnet工具目录也无devskim；Docker CLI存在但daemon不可连接，因此没有已验证可用的精确1.0.90本机扫描器，未安装或启动它。后续纯报告head若再次触发扫描，必须按该新run另核，不能说已被本报告扫描。
