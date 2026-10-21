# #1406 owner 启动独立时钟锚修复工作草稿

名分：#1406 活期工作草稿。基线13ef38967809e9a23992a6ea93c692b546788545；2026-10-18主控已授权证据完整性修复。

根因：采集器owner readiness传10秒启动父窗，独立Replay.start漏传，paged误按无父窗30秒计算。真实run39082749100无独立owner锚，不可重建精确父期限。

决定：扩展外部时钟协议为c6a-clock/2。固定41步既有case/domain对应11项有序锚：work、we5-case、we5-owner:E、we5-owner:B:chong-R1、we5-q、we7-case、we7-owner:E、we7-owner:B:chong-N1/N2/N3、we7-q。op同时绑定case与domain；序号/run_id/全部时序沿用原严格规则。owner与Q启动均从外层issued派生min(issued+10,case,work)，不增加任何常量或预算。

变更清单：s_session/tests/tb03c_query_runtime.py（协议/请求校验、owner_start、Case.start）；.github/scripts/run-1406-c6a.py（协议、ClockBroker.complete、verify_clock_transcript、provenance绑定、嵌入语义）；本目录semantics.py（从独立owner锚传父期限及域/过程时序核验）；test_semantics.py（合成见证和正负控）；新增独立时钟协议靶向测试。LinuxSupervisor、时限常量、资源阈值、CI开关保持不动。

兼容策略：/1旧协议与缺owner锚一律拒绝，不设宽松兼容路径；旧真实run永久FAIL，禁止补写锚或修改原件。新候选离线回归通过不等于真实产品验收；真实运行须独审与主控验收后另行授权，不自动触发。

必要负控：缺锚、错域、错序、重复/重放锚、任意短deadline、超父窗、晚到及逐页续窗拒绝；更紧10秒父窗正常样本通过，并保留完整41步及既有全部负控。
