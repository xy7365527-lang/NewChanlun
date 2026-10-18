# #1394 实施与作者自检记录

名分：绑定1394工作草稿。这里只声明实际代码、验证与边界，不签独评或运行批准。

| 范围 | 实施与验证 | 边界 |
|---|---|---|
| 历史来源 | 两commit/tree及14原件精确核验；同b46树补读HTTP输入/Q信封；保留C1唯一financial_reader差异 | 不固定未来producer；仍需统一源语义兼容审 |
| Q协议 | 十一字段外层信封、固定cut、17字段token、逐页身份/offset/末页、全页公开原序投影根 | 不实现Q content/watch/Delta；不猜新G字段 |
| financial | 精确会话argv、ACK后原stdin、全对象basis原序、同anchor/path空/after空串、真实next全闭包 | 不走冷6参；不代造独立原库source |
| 原件 | Capture与Exchange保留body/HTTP/stdout/stderr/exit/时间/命令；失败先保存再判；清单精确bytes/SHA | 流超帽保已观测前缀并明确不完整；没有未收到原件的伪摘要 |
| 资源 | session/reader/Q独立，reader/Q两个具名Job及原staged/store累计；完整HTTP请求帧、全部wire/stderr、每Q页完整投影记录 | 本地账是可观察下界；Native/OS全账由独立read_guard实际供给 |
| 截止 | 原whole/current-stage/所属Job/request最早值，调用前后/解码/guard均检查 | 不新增时间窗，不跨Job借帽，不以完成历史stage截止挡当前阶段 |
| 准入 | 真ExecutionAdmission外部pin/单次receipt，原owner和command，独立ACK与额外READ_ACK | read_guard服务端当前未实现；旧ACK拒绝；产品NOT_RUN |
| 生命周期 | 根冻结初始八维，同grant包装器复用账本，消费不退款、请求不重试、失败全生命周期停止 | 跨进程延续须根冻结真实已有累计；不自动批准重启 |
| 成功义务 | 返回未认证原件及MISSING_DEPENDENCY | 不输出authority/业务PASS/terminal/K/γ；不调用export自签pin |

作者自检按新边界逐项补强：传输只可绑定相同根计划和状态机；发送只消费一次；stdin gate需本次独立监督；未知传输异常也保留尝试请求；Q全部行序在basis前核完整投影根；独立控制帧/截止也须根冻结；真实HTTP输入帧计头；每Q页记录费计完整投影。对应最终38方法详见TEST-SELECTION。

实际执行历程：基础30方法通过；同范围作者自检增加返回后拒绝、不可重试、未知请求原件、完整投影顺序、实际binary变化等锁后36方法通过；发现HTTP帧/全投影计费两项后分别定向通过，最终普通与优化模式38方法均通过。初始新模块不存在，未伪造“修前红灯”或将后续自检输出称为独审。最终完整输出与文件指纹收于TEST-RESULTS。

已读AGENTS/CLAUDE、docs/agents宪法/交付/工作流、CODEX-CHILD及ask-matt。当前没有可调用图工具，代码发现使用已知路径和必要rg；未安装索引。用户明确授权本票实施且禁止其他工蜂，因此本轮直接执行并自检；独立复核由管理者固定快照后完成。

旧Supervisor/captured_cut/ControlBridge/driver/scheduler以及旧报告逐字不动。独审04的R-F5不在本写集，不把本轮新测试当作其修复。只留下新文件差异，没有Git或tracker写操作。
