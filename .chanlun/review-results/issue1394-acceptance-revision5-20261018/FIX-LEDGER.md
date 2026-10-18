# #1394 返修05身份矩阵

固定基线 `576a13470566a5ca4625eae3d4e89abcb950e346`。独审04原件及五条实际探针完整保存在references；其旧版本BLOCK不改写。本轮只修R-F5两个发送回归，作者自检不替代独审。

| 身份 | 既有结论 | 本轮变化、证据与结论 |
|---|---|---|
| F1 HIGH | 已过，独审03闭合 | 固定Requirements量词、原测试及scheduler/driver逐字保持，按精确SHA复用；不重跑510阶段路径。 |
| F2 HIGH | 已过，独审03闭合 | 逐owner原请求有序全集、epoch/key/hash及原方法保持，复用既有输出。 |
| P0 | 已过 | driver/P0原件与独审及探针长度/SHA绑定复核，41个driver方法逐字保留，不重跑产品准入。 |
| R-F3 HIGH | 独审03闭合 | financial_oracle、独立E时间判据及测试不动，精确SHA复用。 |
| R-F4 HIGH | 独审03闭合 | journal/scheduler字节不动，candidate隔离与消费不退款规则保持；不把重开升级为自动恢复接受资格。 |
| R-F6 MEDIUM | 独审03闭合 | stop/register/live_identity等原类方法逐字保持，TERM共享绝对期限、KILL及reap规则不变；本轮fake异常仍只回收登记owner。 |
| N1 HIGH | 独审03闭合 | 首after空串、null拒绝及后页顺序代码/测试不动，复用旧证据。 |
| N2 HIGH | 独审04闭合 | captured_cut及原16页请求帽、5940/6040原件锁逐字不动；不重跑，不以新canonical判据替代原wire长度。 |
| N3 MEDIUM | 独审03闭合 | 对象/Job显式绑定与4Q/17financial分别计账保持，按SHA复用。 |
| N4 MEDIUM | 独审03闭合 | Voice/charge_capture逐字不动，不借帽，不新增运行资源帽。 |
| N5 MEDIUM | 独审03闭合 | resources及当前阶段最早截止规则不动，不以历史阶段截止阻挡当前阶段。 |
| R-F5 原排队误拒 | 独审04确认原误拒已修 | 原HEARTBEAT→ACK、多个REGISTER、ACK内部路径等24个旧监督方法重验；发送期间新增同型排队正例也通过。 |
| R-F5 发送前 HIGH | 独审04新增回归，旧版本待修 | 每条emit前重查原截止及到期采样、再查该ACK阶段。正式独立E9秒/B0.02秒反例修前在0.053秒发B，修后不发B且INCOMPLETE；作者自检通过，待独审。 |
| R-F5 发送返回后 HIGH | 独审04新增回归，旧版本待修 | 最后一emit返回也核原work/heartbeat及应有全集/资源采样。单ACK终止、双ACK最后RSS超帽、陌生PID及采样耗尽截止均拒绝；作者自检通过，待独审。 |

受影响方法逐项见TEST-SELECTION.json；普通/-O各40方法（24旧+16新），不是以总数替代上表身份。修前16方法中12方法红灯、4方法原保护通过；实际25个失败断言及原输出可查。正式发送探针重放与历史源码对拍见FORMAL-COMPARISON.json。源码只改supervisor.py；全部既有product_capture字节与报告保留，其独审仍待管理者处理。

execution_authorized=false，产品NOT_RUN；不勾P11、不关整票或相关图票/#1279。真实统一源/两ARTIFACTS、同域terminal/K/γ/Voice及独评、全资源/准入/OS、fresh A/B/24故障/3额外/5负域/10compare/完整历史/资金/隔离/恢复义务全部保留。前端#1460、K4排除、Journal保守candidate恢复代价不变。
