# #1394 返修04逐身份矩阵

基线 `6cb6bbf33eb079f639abae2449ccc9b98d81a11c`。独审03正式BLOCK原件及6条完整探针已核SHA并逐字保存在references。只修R-F5、N2；下表不以合计数代替各项身份。**本轮新字节仅作者自检，待独审。**

## 两处残余

| 身份 | 基线实际问题 | 修复与定向锁 | 当前结论 |
|---|---|---|---|
| R-F5 MEDIUM | HEARTBEAT后ACK仍排队就采样；已登记101后只处理REGISTER102便误拒仍排队的103；ACK内部也会提前强制采样 | supervisor.py 的 monitor 建立受原截止与sample_interval约束的批次，按序核验具名登记；空队列/采样边界/结束消息强制采样后才发ACK。直接ack共用_sample_and_ack。test_review03中10个R-F5方法覆盖两正式复现、ACK内部路径、无排队、陌生PID、伪登记、截止、持续流、COMPLETE和未知资源 | 作者自检通过，待独审；真实OS未运行 |
| N2 HIGH | 原回复5940、请求帽5940为正例；前加100空格为6040且重绑摘要，仍按canonical后5940误放行 | captured_cut.py 保存每页原reply长度，按各自请求帽实核；test_review03中3个N2方法覆盖5940正例、6040负例、抬至真实6040帽的合法JSON、全部16页以及basis原帧/Job独立帽。4份toy原件留存，不靠session/reader/Job帽兜底 | 作者自检通过，待独审；不认证产品数据或语义 |

## 旧已闭合项逐项保持

| 身份 | 本轮证据与状态 |
|---|---|
| F1 HIGH | 独审03已闭合。固定Requirements量词校验及原F1测试未改；scheduler/driver/测试原件摘要匹配。复用revision3普通/-O输出，不重跑510阶段路径。 |
| F2 HIGH | 独审03已闭合。逐owner原请求有序全集、epoch、key/hash校验及原F2测试未改；复用原输出与独审03。 |
| R-F3 HIGH | 独审03已闭合。financial_oracle、对应fixture及原方法逐字不变；三处B时间绑定后对独立E时间的判据保持。未重开或改动。 |
| R-F4 HIGH | 独审03已闭合。journal/scheduler/相关fixture逐字不变；candidate隔离、consume不退款、解锁后接受规则保持。重开不自动恢复接受资格的保守代价保留。 |
| R-F6 MEDIUM | 独审03已闭合。SupervisorCore.stop、register、live_identity及原回收锁精确源码不变，统一TERM→共享绝对期限wait/KILL/reap保持。复用原多owner超期证据；本轮相关fake回收路径通过，不认证真实OS。 |
| N1 HIGH | 独审03已闭合。首after空串、null拒绝、后页顺序规则不变；旧test_N1_empty_cursor_legal_null_and_broken_order_refused普通/-O重验通过。 |
| N3 MEDIUM | 独审03已闭合。对象/Job显式绑定、4Q与17financial分别记账保持；三个test_N3旧方法普通/-O重验通过。 |
| N4 MEDIUM | 独审03已闭合。charge_capture和Voice模块逐字不变；test_N4_size_observation_not_Voice_64_128_caps重验通过，未引入Voice借帽或新运行资源帽。 |
| N5 MEDIUM | 独审03已闭合。resources未改；test_N5_historical_stage_expired_current_valid重验通过，历史stage不参与当前最早截止。 |

方法完整身份、选择理由见TEST-SELECTION.json；指纹、历史输出与来源绑定见TEST-REUSE.json；不变函数/41driver/27scheduler静态核见STATIC-CHECKS.json。execution_authorized=false，产品全NOT_RUN；本表不批准后续动作。
