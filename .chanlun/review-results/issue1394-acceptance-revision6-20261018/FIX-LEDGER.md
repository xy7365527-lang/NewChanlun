# #1394 返修06身份矩阵

名分：绑定 #1394 的活期工作草稿。固定基线 `4b3a84ebba8178e03aa04b2bd2a6a4138d9d438a`。独审05原件/7条探针在 references；本次只修其新增 R-F5 MEDIUM。作者自检不替代管理者固定版本后的独审。

| 身份 | 已有结论 | 本次证据与边界 |
|---|---|---|
| F1 / F2 / P0 | 已过；F1/F2独审03闭合 | driver/scheduler及原41/27方法逐字保留，精确SHA复用；不重跑全路径或产品准入。 |
| R-F3 | 独审03闭合 | financial_oracle及其锁未改，按SHA复用。 |
| R-F4 | 独审03闭合 | journal/scheduler未改；candidate隔离、消费不退款、重开不自动恢复的代价保持。 |
| R-F6 | 独审03闭合 | stop/register/live_identity等8个Core方法逐字保持；回收只作用于登记owner。 |
| N1 | 独审03闭合 | 首after、空/null和后页顺序原判据未改，按当前Q序封包匹配字节复用。 |
| N2 | 独审04闭合 | 当前captured/Q序原件保持；不重跑，不用canonical替代原wire长度。 |
| N3 / N4 / N5 | 独审03闭合 | 对象/Job、4Q/17financial、Voice及独立资源计账、当前阶段最早截止规则未改；不借帽、不增帽。 |
| R-F5 原排队误拒 | 独审05确认闭合 | 保留HEARTBEAT→ACK、多个REGISTER、ACK内部登记、有限队列/终止边界，所选旧方法通过。 |
| R-F5 发送前 HIGH | 独审05已闭合 | `test_R_F5_formal_second_stage_expires_between_sends` 及work/heartbeat等原方法通过；第一emit0.05后拒绝B0.02，不再发第二ACK。 |
| R-F5 发送返回后 HIGH | 独审05已闭合 | 最后一ACK返回后work/heartbeat耗尽、RSS超帽、陌生PID、采样耗尽截止仍拒绝，COMPLETE/CANCEL不绕过检查。 |
| R-F5 实际采样边界 MEDIUM | 独审05新BLOCK项 | 本次唯一待独审的修复：tick统一决定到期并进入有限排队或已确认终止边界。两条连续2微秒clock反例修前红灯、精确576a基线成功、修后成功；合法后代实际采样并回收。仅作者自检通过。 |

新增5方法＋旧24方法＝29个不同方法，普通/-O各29通过。计数不替代上表逐项身份。新测试锁同时约束未到期时的身份/截止检查，采样、资源与未知PID判据保持。TEST-SELECTION列具体方法；FORMAL-COMPARISON列正式两反例、控制与历史基线输出。首版一次报错文字回归及输出透明保留，旧测试没有改判据。

execution_authorized=false、产品NOT_RUN。统一source/两ARTIFACTS、真实terminal/K/γ/Voice与独评、OS/资源/根准入、fresh A/B/24故障/3额外域/5负域/10compare/全历史/资金/隔离/恢复仍欠。read_guard候选不升spec/authority；Journal candidate恢复代价不变。前端#1460、K4排除。不勾P11、不关#1394/#1361/#1340/#1323/#1279。
