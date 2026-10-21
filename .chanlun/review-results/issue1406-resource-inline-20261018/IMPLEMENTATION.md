# #1406 resources inline 候选实施报告

名分：#1406 活期工作草稿。基线9718615aed45902feec7244ebab7111a773dd4b2；主控2026-10-18批准诊断方案B，只改资源probe准备/入口及专属测试，不推送或运行产品。

真实run39084502325的53号资源探针在probe取样后至spawn.started前消耗240.089513ms，仅剩9.910487ms活动预算。该区间包含父request写盘/路径解析/调度，原件不能再细分；不是build总截止、编译或owner-clock故障。两次旧FAIL和原件不改。

候选：resources专用`--resource-inline`携带规范JSON，schema固定c6a-resource-probe/1、字节≤16384、groups仅0或1个正32位PID，filesystem/output为各≤4096 UTF-8字节的绝对规范路径。拒重复键、NaN、非规范JSON、多余/缺失字段、非法PID与路径、旧resources文件入口、互斥CLI模式组合。父probe不调用worker_command、save(request)、Path.resolve；完整请求保留于资源helper收据argv。

准备前后都检查同一stage active D=stage D-6.25；准备输入先限为长度≤1的list/tuple，避免无界迭代。准备完成后才取局部probe_end=min(stage D,now+6.25+.25)。该局部绝对取样点相对旧实现后移准备耗时，不能称全部绝对deadline不变。原stage绝对D、.25活动长度、TERM/KILL3/3、所有资源阈值与全部模块常量保持原值；stdio/Popen仍计.25，不从其返回重开窗口。

父线程准备只做有界字符串/列表/JSON计算，没有request文件IO。实际/proc与磁盘观测、probe输出发布继续在受监督helper组执行；worker入口先独立解码验证inline请求，再调用原resources worker。原父stdout/stderr open、Popen及OS调度的不可中断限制仍存在：只能在返回后拒绝超时，不能声称已具备强实时可抢占保证。

实际检查：专属11项纯替身通过，覆盖schema/bytes/domain/非法参数、真实CLI至resources worker派发、无request save/resolve/worker_command、240ms慢准备正常、准备越stage active D不spawn、慢Popen300ms/慢probe400ms仍失败、极限D钳制与及时成功。ClockBroker/owner入口5测通过；driver合同62测通过。未重复未变的完整41步语义套件；本轮业务/driver源码未变。

AST对照：LinuxSupervisor只变probe，run/reap/send/spawn/wait_group/worker_command完全相同；所有顶层赋值（含时限、资源、READY、owner锚、嵌入语义hash）完全相同。新顶层函数仅resource_inline_encode/validate/decode，main新增互斥inline入口。git diff --check通过。

测试命令：

```text
python3 -B .chanlun/review-results/issue1406-resource-inline-20261018/test_resource_inline.py
python3 -B .chanlun/review-results/issue1406-c6a-pagination-20261018/test_owner_clock.py
python3 -B -m unittest discover -s s_session/tests -p test_tb03c_query_runtime_contract.py
```

NOT_RUN：本机产品/cargo/rustc、真实Linux资源与信号回收、远端CI、独立审者最终验收。当前源码绑定常量保持旧值，本提交含专属测试/草稿，不能直接冒充已完成新source/tail绑定或CI准入；由根在独审后处理。XERJ复用19200现有nc1320ref索引，inline subprocess查询只命中弱相关Nautilus配置，未据此创造依据或重装。
