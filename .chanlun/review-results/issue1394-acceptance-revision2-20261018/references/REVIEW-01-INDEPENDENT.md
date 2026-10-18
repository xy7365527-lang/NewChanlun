# #1394 工具集成独立审阅01（正式结果）

固定审查提交：b8c5fd404959ff5fac3e48d4e9449a975d6c0b7e；基线3315ed1d59ba31ffbad3f0b959702eaa92f4ddbd。
真实Sandcastle run_id 76bb657c-1ef8-4509-ab74-0cba970f3c03，external session 01a14d38-919a-78c1-b462-4249a487bd81；04:22:09Z真实退出，worker/入口均0，审阅结论BLOCK，不能将执行completed当审查PASS。

审阅完成，结论：阻断本次工具集成通过，确认6项问题。P0原F1/F2已针对正式反例补锁；新问题涉及合同完整性、恢复身份、独立核账及截止/监督逻辑。所有反例均限离线内存模拟，未绕过产品执行门；不构成P11、AC4或产品验收。

## 正式发现

- [HIGH] s_session/tests/issue1394_acceptance/scheduler.py:274、285：只检查preparation_sha格式，随后信任调用者提供的Requirements，未核其与冻结合同一致。独立探针保留ce719894…摘要，却删至1场景/11步骤、0负域、0compare，仍返回SIMULATED_COMPLETE_NOT_PRODUCT_ACCEPTANCE。静态完整性门因此可漏掉24故障组、B、extra及cap80。必修：从匹配摘要的合同原字节重新导出并逐项核对Requirements，补同时删改requirements与steps的拒绝锁。
- [HIGH] s_session/tests/issue1394_acceptance/scheduler.py:320、358：恢复绑定仅约束Step.binding；requests中的各owner未逐项绑定恢复epoch和原请求。探针将首个resend的第二owner改为epoch=99、新receipt_key及新request_sha，完整模拟仍成功。必修：按各owner维护恢复前后的epoch及有序原请求全集，核重送身份、原字节和出现序号；不得只核阶段代表项。
- [HIGH] s_session/tests/issue1394_acceptance/financial_oracle.py:128：E cut只核两份actual彼此相等及commit_seq，未独立绑定E持久历史。探针将grant和local_commit_receipt共同改为外域B:outsider、OTHER_SESSION、NOT_IN_E_HISTORY根，保留seq=2，audit仍接受全部6个中间态。必修：从独立E原库取得对应完整cut，核域、会话、代际、根及提交身份；缺字段或缺原件必须拒绝，补双方同时伪造cut的反例。
- [HIGH] s_session/tests/issue1394_acceptance/scheduler.py:426、433及evidence.py:69：截止检查未覆盖核验后的返回/封口。fake-clock探针在最后一次_evidence结束时推进至61（work截止60），仍封为模拟完成；same_cut第二次真实JSON解析后推进至3（预算截止2），仍返回摘要。verify_exchanges:38之后也缺最终预算检查。必修：核验、序列化和持久化后，以及成功收据/结果返回前，复查同一个最早截止；超时不得生成成功阶段收据。
- [MEDIUM] s_session/tests/issue1394_acceptance/supervisor.py:197：收到ACK_REQUEST或REGISTER_DESCENDANT后先tick，再处理登记。独立fake Probe在合法receive超时窗0.125秒内返回首个ACK，随后的采样先将该child判为未登记，监控退出；owners为空，Probe仍报告child存活。必修：为具名待登记进程建立明确握手顺序，在采样完整性判定前核验登记消息，同时继续拒绝陌生PID；补通过monitor入口的首ACK及后代登记竞态锁。
- [MEDIUM] s_session/tests/issue1394_acceptance/supervisor.py:162、177：wait返回后没有复查cleanup截止，已退出分支也直接接受wait结果。独立fake-clock探针让wait返回True时超过cleanup_deadline一秒，结果仍为CONTROL_REAPED且errors为空。必修：每次wait后及汇总前核同一截止，保留实际reap事实但将逾期结果标INCOMPLETE，并继续处理其余登记owner。

## 实际验证

- git rev-parse HEAD：入口及收尾均为b8c5fd404959ff5fac3e48d4e9449a975d6c0b7e；git status --porcelain为空。比较3315ed1d59ba31ffbad3f0b959702eaa92f4ddbd至HEAD，共54个新增文件，限工具与报告目录。
- python3 -B -c内联只读摘要核验：DELIVERY-FILES的47项、MANAGER-DELIVERY-FILES的53项全部匹配；后者覆盖全部差异，仅按设计排除清单自身。20个Python文件AST解析成功。
- git diff --check 3315ed1d59ba31ffbad3f0b959702eaa92f4ddbd HEAD -- s_session/tests/issue1394_acceptance：退出0。未清洗报告中的原始diff或失败输出。
- 复用MANAGER-RECEIPT、HOST-TEST-RESULTS及原日志：host普通/-O各123项通过；内层原三项受阻记录完整保留，显示1失败、3错误（含子例），离线普通/-O各120项通过。本轮未重跑这些套件。
- 实际执行python3 -B -c内存定向探针，完整脚本及输出见本轮工具记录：确认合同删减、恢复身份替换、假E cut、末阶段核验超时、same_cut解析超时、ACK登记竞态、回收超时接受路径。监督身份、资源及时间使用显式fake，无真实业务进程或数据库。
- P0 F1/F2按正式报告、源码及5个新增回归方法复核；冻结SHA不再被观测覆盖，既有/替换目录拒写及合法实例失败回收均有行为锁。SQLite共享ReadBudget与显式close路径保留；typed closure的独立遍历、原字节比较及模板拒绝边界保留。
- 来源角色与后续路线披露正确：3315工具宿主、b46旧协议快照、99e819 C1静审分列；未来1488→1419→根组合C1→新SHA，不将工具HEAD或b46冒作未来已审执行源。
- 当前只读沙箱连/tmp临时写入也拒绝，首次here-document未执行；后续探针均不落盘。未修改checkout或原封包，未启动其他工蜂，未读取凭据，未执行网络、cargo/rustc、S/E/B/Q、Long、真实资金或浏览器动作。正式同域S/Voice、两份ARTIFACTS、完整资源及OS监督接线仍缺；P11、AC1..5及相关关票门保持未完成。
