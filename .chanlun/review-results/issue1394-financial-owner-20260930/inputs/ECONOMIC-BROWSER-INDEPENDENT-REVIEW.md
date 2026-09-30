# #1394 经济 Browser 实际运行独立事实验收

结论为 PASS_BOUNDED_ACTUAL_BROWSER_FACTS，未发现本次具名 TestOnly 实跑与原件不一致。此结论只覆盖真实经济 HTTP /2 页面从 current 回看双 Prepared、Watch 返回原 current，并查询全部两 decision 与两 grant 原件的有界路径。

固定工作树为 `/Users/silencehan/Projects/NewChanlun-1394-content-v3`，分支 `codex/1394-content-v3`，HEAD `3b53744be109aa18932bdc50d3590e10dde9b32d`。验收输入 `BROWSER-ACTUAL-MANIFEST.json` SHA256 为 `d7881aa37db27ab206e5a8c8046c916608fde8eb78dc9a3b67783d7d193a5a82`。独立逐项读取并重算清单中的字节数和 SHA256，418 个不同文件全部相符；其中运行原件列表 72 项，新 Chrome profile 324 项，合计 396 个运行文件，其余为来源绑定。没有用 manifest 或 FACT-CHECK 的 PASS 字段替代这些计算。

客户端四文件与固定受审候选 `9f923877e7d2dd9bb9c0a8cfe0c065bbfbda38da` 的 `git show` 字节完全相同。最终独评 `INPUT-FINAL-INDEPENDENT-REVIEW.md` 的 SHA256 `244b5617bb9381919328a820351965c4d68a22b07e85a05c92605aabdc6db5a1`、JSON 的 SHA256 `91805564dc3adc561aa9747d61c2155434b9854c753c3626715ccb7e283b9e03` 均现场重算相符，JSON 的候选号与 Spec、Standards PASS 相符。旧 `ROOT-FINAL-CLIENT-AUDIT.json` 的 PENDING 是整合前历史记录，本次没有改写它。

四个后台 Python 与原 HTTP 封包固定 HEAD `976702750dfd7e89706223216ae8e0e19f2b9c87` 同字节。E/B 二进制的 bytes、path、SHA 与原 `ECONOMIC-HTTP-MANIFEST.json` 完全相同，分别为 `30e66463f55460ff7eef05a7389f2bdca0937503b68465eddf76ebef47d00525`、`6c3cb2b38b83747bcf29a32870413d98c9d0b202230769187657afd57426d048`。Chrome 二进制 SHA 为 `633aa60f1ee2346804e006071d8f6a2114106e9c17abd601fe61bf9754638740`，正式 runner SHA 为 `74eba89e9264a3a859c0dc88957a013f7b58894eeab426aac707b1c4939747ff`，均重算相符。

冻结 owner manifest SHA 是 `bb44978f91d9f3086d244e166d706abcd5a4b4488b1b0cbb1cc6ff73191a0602`。Q19 配置与八项经济配置的 SHA、字节及路径沿用原 case2；八项限额逐项相同。query_source 与 manifest 中同 producer 的 namespace、epoch、producer 精确一致，仅七个读操作 ReadView、Watch、QueryFact、QueryAllocation、QueryVoice、QueryDecision、QueryGrant。clock 为 TestOnly；discovery 明示 TestOnlyRecordedEconomics。原 native Unix oracle SHA `703a0cbbed0ed17bb975ad16555196c604a57f1fed7fc68e0292dea604e78680` 现场相符。

实际命令由 `BROWSER-COMMAND.json`、`browser-run-1/command.json` 和四组 PROCESS-START/READY 原件记录。Node 正式 runner 使用 Chrome `--headless --remote-debugging-pipe` 及该 run 新 profile，打开 `http://127.0.0.1:38794/economic.html`，as-known 参数为 85。CDP 原件有页面导航、三个 asset GET 200 和四次截图。后台 readiness 原件只声明 E/B Unix socket 存在与 Q TCP connect，经济通路有效性由之后实际 discovery、POST 和原 oracle 对拍确立。

12 个 HTTP 回包均为 200，包含 1 discovery 与 11 个 /2 POST，后者为 snapshot 6、watch 1、original 4。逐条根据 CDP requestId 将 requestWillBeSent、responseReceived、getResponseBody 原件关联，响应正文与 http-N 文件逐字一致。对每个 POST 独立重算请求和响应 payload_hash，验证公头精确 11 字段、会话/代际/namespace/epoch/producer、唯一请求 ID、单项 causal_refs，并重算 reply message_id。全部相符。请求和回复均在冻结 1 MiB 帧和 2 MiB 经济回复上限内。这是本次回包的事实，不是全资源上界证明。

重算方法只用 Python 标准库，没有导入生产模块或调用 verifySnapshot。canonical JSON 为 UTF-8、字典键排序、ensure_ascii=False、逗号和冒号之间无空白，SHA256 取其字节。payload_hash 是 payload 的 canonical SHA256；reply ID 是 `reply-` 加 `[request.source_namespace, request.source_epoch, request.message_id, reply.payload_hash]` 的 canonical SHA256；causal_refs 是请求四身份字段组成的单元素数组。逐域 current、Prepared、Watch 变化的 image_hash 也用相同独立方法重算。

| 域 | Prepared cut / commit_ns | 当前 cut / commit_ns | 原件及 Watch 对拍 |
|---|---|---|---|
| B:alpha | 3 / 72 | 4 / 102 | current image、cut、image_hash、commit_ns 与 native ReadView 和 Watch 末变化精确相同 |
| B:beta | 3 / 82 | 4 / 112 | 同上 |
| E | 3 / 65 | 4 / 92 | 同上 |

`as_known_ns=85` 的两条决定均为 Prepared，grant 和 local_commit_receipt 均为空，分别与 native Prepare 原始 record 完整对象相等。native 首次获知时间为 71/81，E Reserve 为 91，B Commit 为 101/111；因此没有把 commit_ns 与 first_known_ns 混为一谈。历史每域 cut 均低于当前。当前两决定为 Committed，锁定额 30/40，execution_eligibility 均 None，实际数量在两时点相同。E 当前责任逐项求和为 90，历史为 20。

四份 `EconomicOriginal.original` 全对象分别与 native `query_decisions` 两项及 `query_grants` 两项相同；保存的 browser reply 又与对应实际 HTTP payload 完全相同。不是只比 ID 或挑字段。四请求在 JSON 报告中列出，其对象为 alpha、beta 和两条 E grant。

人工目视 `prepared-as-known.png` 与 `current.png`，前者显示时间框 85、历史 cut、alpha/beta Prepared、锁定 30/40 及公共责任 20，后者显示当前 cut、两项 Committed 与公共责任 90。中文控制按钮、账簿、实际数量、成本和 Voice 开局记录确有渲染。本次只核渲染事实，未做新 UI 设计或 #1460 产品评审。CDP 没有保存 asset 响应 body，故资产源 SHA 与 GET 200 已绑定，但未另做 asset wire bytes SHA。

owner before/after 中 commit seq/root、commit_ns、receipt 数、published/staging 节点、epoch、manifest hash 逐字段相同。三 DB SHA 的已保存前后观测也精确相同。独立复核没有重新挂载已卸载 DB；不能把这项结论扩成所有 DB/WAL/SHM 字节不变。三原归档 DMG 现场 SHA 与冻结归档和 before/after 相同。三个副本整 DMG 的最终物理 SHA 与 attach 前不同，已照实确认；可写 HFS+ attach/detach 的物理变化不等于经济语义变化。本次没有挂载原归档。

收尾原件记录 Q/E/B PID 44475/44414/44420/44456 全部 SIGTERM 后退出 -15 并 reap；Node runner PID 44476 退出 0 并 reap；Chrome PID 44482 已退出。三副本普通 hdiutil detach 各返回 0。独立 fresh `ps -p` 六 PID 返回 1 且 stdout 空，`lsof -nP -iTCP:38794 -sTCP:LISTEN` 返回 1 且 stdout 空；没有把 TIME_WAIT 当作监听。fresh `hdiutil info -plist` 返回 0，六原/副本路径均未挂载。短 socket 目录 `/tmp/nc1394-eo-31` 已不存在。以上 fresh 状态直接写在 JSON 报告中。

未核边界保持不变：本次不独立证明 FinancialBasis 内容节点闭包、七字段正文展开/绑定、缺少前 root 的顶层经济 root 重算、controller/Q3、全资源或任意部署路径、9011 双跑、#1460、#1323 或整图 joint。没有生产资金、场所调用或交易授权，也没有 main 合入或 push 许可。只写本独立报告 MD/JSON，未重跑服务、浏览器或全套，未改客户节点 API、产品、旧原件、资金、main、push 或评论。

独立离线探针首次把保存请求的展示字段 domain/id 误用于真实 wire payload；实际 payload 字段为 domain_id/object_id。修正审查脚本的映射后所有断言通过，产品和实跑原件未改动。这是验收方法的字段映射修正，不是隐藏的服务失败。

机器报告为 `BROWSER-INDEPENDENT-FACT-REVIEW.json`，SHA256 `dbac5dca3bb7ddca87968ca9089768fd055e51d9f7871023fe305c5bf12de16a`。该文件列出逐条 HTTP 检查、完整原件的 canonical SHA、源资产 SHA、fresh 回收命令与结果，以及本结论的排除范围。
