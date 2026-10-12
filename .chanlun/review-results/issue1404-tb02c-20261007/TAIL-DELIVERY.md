# #1404 / TB-02-C 尾部交付补件

工作草稿，活期绑定 #1404。基线 `8619a20f2800095e6bb9ec20ba2607a744740f91`；既有公开 PR #1473 的产品版本为 `95165b79d7dea25c2f3b45e666596c03edde4c13`；本次产品尾部为 `230f7cdb06feb5710d97a50cffe7cf01b6063041`。本报告不授予合入权限，不关闭父 TB、SPEC #1340 或整图 #1323。

## 实际变化与版本绑定

公开版本到本次产品尾部只有 `rust/src/theta_v0/parser/stroke.rs` 一处产品差异（184 行新增、10 行删除）。尾部把 waiting 限于当前判断实际消费的依赖：端点根只取中心组对应顶/底；同型严格价格比较保留获选端点的 waiting；首笔前严格替换保留多根、不选择 raw 身份；成笔合取已含 false 时继续扫描，不让无关未知冻结后续候选。包括四项新增定向测试及一项原测试语义订正。

该源文件本次 SHA-256 为 `057c1fedc52587c9488bdb27dc6f3e1ddf9da0591a3d7ce265ca36bff3e3c429`。`TAIL-SOURCE-BINDINGS.json` 保存实际 base/public/tail 差异及 mode/摘要；既有 `FINAL-SOURCE.json` 的32个文件逐项核验仍完全相同，但该旧清单不含本次新增改动的 stroke.rs，不能直接当作整个尾部指纹。

## 实际检查与原始日志

- 本轮为补齐原进程退出码，执行固定工位的 `CARGO_TARGET_DIR=/tmp/goal1323-tb02c-tail-target cargo test --manifest-path rust/Cargo.toml --locked --offline --lib theta_v0::parser::stroke::tests::tb02b_ -j 2 -- --nocapture`。退出0；13 passed、0 failed、0 ignored、3101 filtered out，测试体16.04秒。开始/结束UTC、源摘要及工作树前后无变化，见 `TAIL-TEST-EXECUTION.json`；完整输出见 `TAIL-TB02B.log`。历史同测日志虽有13通过，但执行包装未开pipefail，故其包装退出码不当作cargo的直接退出码。
- 同版本既有 `cargo test --manifest-path rust/Cargo.toml --locked --offline --lib tb02c_ -j 2 -- --nocapture` 的原日志为8 passed、0 failed、0 ignored、3106 filtered out，测试体0.03秒。直接工具调用退出0可恢复，见 `TAIL-TB02C-PROVENANCE.json`；原日志字节见 `TAIL-TB02C.log`。没有再次运行这8项。
- 两组测试均含既有编译警告，不能写“无编译警告”。历史 `/tmp/TB02C-TAIL-TEST-RESULT.md` 的大多数测试名、时间戳与无警告说法错误，不采纳该报告。实际测试名以本报告列出的两个日志为准。
- 历史rustfmt日志是空文件，空文件本身不作通过证据；本次没有为此重跑格式检查。

本次没有重跑四arm R3、完整9011、166代或release构建；不声称当前release/服务/浏览器重新验收。

## R3 可复用行为域

冻结输入 `s_session/tests/fixtures/tb02c/raw-ledger.json` 与公开版本字节相同，SHA-256 `32d00df0b3e349411b79e34e284242bce5b63dce97a03b6cb2eb17a8dec27aa4`。`TAIL-R3-DOMAIN.json` 逐项检查 first_up 与 first_down：各28 raw，27对相邻raw高低严格同向；每个前缀无包含、raw坐标唯一。因此各包含组始终是单raw，两种极值根均唯一。首对已给出方向；能形成端点的前缀不处于初始方向未知。

对该具名输入域，旧端点逻辑遍历三组时也不会产生 raw tie，新逻辑只检查中心对应根仍为空等待；严格同型比较的等待并集与获选端点等待均为空；首笔前“仅多根等待”的新增放行不产生额外行为；异型pair的raw位置已知，不会出现本次改动针对的“必假且另一条件未知”。增量与全量都走相同判断。故本尾部不改变这56个前缀的笔输出及其事实；下游段/S/Q/观察器代码字节未改，可复用原R3对此输入域的发布、恢复、oracle、Chromium与完整字节对拍证据。

该复用是有限域的源码依赖论证并有同版本8项fixture回归支撑；不是尾部release重新执行，也不是一般同价域证明。旧R3二进制身份始终保留在 `VALIDATION.json`。历史R3到公开head的几何失败提示修复等既有差异，仍依原 `DELIVERY.md` 与对应回归分域说明，不由本报告抹平。

本轮只对四arm各5份小回执进行读取及SHA核对，20份全部匹配 `VALIDATION.json`，详见 `TAIL-R3-RECEIPTS.json`。没有打开、复制或重新全哈希数据库/大core原件。470,987,732与475,548,429字节的原始对拍数字仅沿用已归档 `COMPARISON-RESULT.json`，不称本轮重新测得。

## 独立审查与扫描状态

旧尾部评审曾写PASS，但存在不存在的源码片段、行号和逻辑表述错误，保留在 `TAIL-REVIEW-SUPERSEDED.md` 并取消其验收效力。另一GPT独立会话已针对固定 `95165b79d7..230f7cdb06` 完成重审，结论PASS、无阻断发现，源文件摘要与本轮测试一致。报告原件已复制到待归档材料 `TAIL-INDEPENDENT-REVIEW.md`，尚须随本封包入Git；本报告不将仓外路径称为已归档。新报告逐项核对同价未知、false合取、严格替换及全量/增量，并独立验证冻结R3窄域。

旧DevSkim运行 `37560904358` 只覆盖公开head：基线41,207条、公开head41,312条，新增105条的分诊保留为历史域。本次已完成覆盖230版本的新扫描，扫描器退出0、零告警gate退出1，完整41312条；相对public完整多重集新增0、移除0，相对main新增105条全部复用已绑定来源的既有分诊，无未核项。实际出口、镜像身份和逐项分诊见 `TAIL-SCAN-RECEIPT.json` / `TAIL-SCAN-TRIAGE.md` / `TAIL-SCAN-TRIAGE.json`。本地没有上传Actions artifact，零告警门也未修改，红灯例外尚未批准。扫描后的纯报告提交按实际逐文件摘要披露，不因归档文档循环重扫。

## 封包门与下一步

独立重审已完成PASS，新扫描和完整多重集分诊已完成；本套实体仍待根任务按白名单归档并提交。归档后的纯报告提交不得改变任何产品字节。仓外manifest必须绑定实际最终提交，并包含新增实体和所需旧实体；用 `.sandcastle/delivery-bundle.mts` 在干净工位执行预检。0退出仅表示可请求具体版本批准，仍不代表已合入或例外获批。

最终全片manifest应以main基线8619a20比较；公开head951到230的尾部诊断manifest只检查本补件，不能冒充整个PR封包。既有schema将png/txt等保守归入产品面，生成全片清单时应如实列出其字节覆盖，不可擅改schema或省略报告实体。
