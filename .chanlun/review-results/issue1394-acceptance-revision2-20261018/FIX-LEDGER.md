# #1394 独审01返修账 v2

固定起点 b8c5fd404959ff5fac3e48d4e9449a975d6c0b7e。反例原文和原命令/输出逐字留在references/REVIEW-01-INDEPENDENT{,-PROBES.json}；其中报告文件实际名为 REVIEW-01-INDEPENDENT.md。旧输出未改成新结果。下表新结果均为本轮普通和-O定向运行，只有模拟/控制/内容核验效力。

| 项 | 原反例 | 修复 | 新正/负结果与锁 |
|---|---|---|---|
| F1 HIGH | 保留ce719894…，Requirements与steps同时删至1场景11步骤，清空负域/compare仍模拟完成；另一个输入也去掉compare proof | validate_plan用内置固定pin重读准备合同原字节，重导出Requirements逐字段严格相等。未来正式新合同须显式换pin并重审；不接受任意格式SHA | 原两种删改输入均FAILED且attempted为空；完整30场景510步骤模拟通过。24故障/2normal/3extra/cap80/5负域/10compare不变。test_F1_requirements_and_steps_both_removed_fixed_probe。 |
| F2 HIGH | 首resend第二owner换epoch99、新receipt_key、新request_sha仍成功 | 从business冻结有序请求全集及每owner epoch；仅具名recover更新该owner；resend完整Request对拍，除经登记恢复epoch和当前阶段cut外，hash/key/op/owner/db/ordinal/case/outcome全部保持；未恢复owner禁止换epoch | 原三字段替换、删项、加项、重排、错出现序号、只换epoch、只换hash全部拒绝。完整恢复通过。test_F2_second_owner_original_probe_and_ordered_full_set。 |
| F3 HIGH | grant和local receipt共同伪造外域/外会话/假根、seq2仍通过6态 | 成功Commit必须有独立E持久原件及外部pin；核meta域/会话/代际，从seq0重算完整根链，核完整历史图像及Reserve原key/slot/cut/ns；local两份cut必须等于该独立E cut，保留独算现金expected和E→B时序 | 原伪造cut拒绝；完整真实协议形状的toy E原件+合法cut通过6中间态；缺原件/pin、错会话/代际/域/根/回执/时间/字段拒绝。test_F3_*两个方法及原财务10方法。 |
| F4 HIGH | 最后_evidence将clock推61(work60)仍模拟成功；same_cut第二次parse推3(budget2)仍返回摘要；verify_exchanges末尾缺查 | scheduler在consume、核验、序列化、journal.finish后与收据/结果返回前查同一最早截止；evidence三个返回点补budget检查；journal将step/work最早截止传到每次写/fsync；迟到/失败receipt撤去本次尾段，消费pending不退款 | 最后阶段无成功收据、结果FAILED但cleanup/seal正常；parse/serialize耗时拒绝；verify_exchanges末尾预算拒绝；写fsync晚于stage但未到work时仍拒绝，原持久consume保留且无receipt。test_F4_*五方法；原正例通过。 |
| F5 MEDIUM | receive合法0.125秒内首child出现，先tick导致owners空且child活；另原输入0.2秒同病 | monitor在receive返回后先核具名ACK/后代登记，再采样；ACK沿原完整合同/worker/stage/资源/截止/nonce/owner身份；后代必须带worker与合同且parent已经握手注册。陌生PID完整性门未放宽 | 0.125和0.2原输入首ACK成功、统一回收；后代竞态成功；错worker/parent/start/合同、未知PID、错首ACK拒绝，不收养陌生PID。test_F5_*三个方法；原监督成功锁通过。 |
| F6 MEDIUM | wait返回True但clock=cleanup+1，live和已退出分支都可能CONTROL_REAPED | 每次TERM wait/KILL wait/已退出wait后及汇总检查原cleanup截止；先记录真实reaped再检查，逾期errors+INCOMPLETE；循环继续其余owner | live/dead/kill三分支均保留reaped=true但INCOMPLETE；两owner晚到仍逐一wait；空owner汇总已过期也INCOMPLETE；正常按时回收成功。test_F6_*两个方法及原监督锁。 |

F2夹具订正说明：旧fixture把代表owner的epoch无条件复制给第二个B:beta，等于把并未recover的beta也隐含重启。本轮beta从business冻结为1，目标从recover显式1→2，resend为[2,1]。没有新增业务政策/重启策略；修正了原模拟输入未声明的重启。原27方法全部保留、完整矩阵仍成功；正式原探针文本保留原样，变异三字段输入也按原值99/f…/UNRELATED_NEW_REQUEST固定在新锁。

F3取证说明：旧协议store::cut只有domain_id/commit_seq/root_hash三个字段，会话/代际在root计算中，不能在cut上猜增session字段。oracle.py唯一连线改动是从独立E只读快照传原字节，并从外部已冻结audit_contract取e_original_sha256；不会现算actual摘要当期待pin。测试只用手写原库快照，不调用生产reducer。

F4新增journal修改理由：仅在scheduler返回失败仍不足以防重开日记把迟到成功receipt当作已接受。本轮保持旧消费记录及原hash链；在排他锁中只撤掉本次未获准receipt尾段。发生真实I/O不可中断、回滚截断/fsync本身失败时仍拒绝，不宣称OS原子持久化或断电证明；消费不退款，不自动重试。control.py与driver.py均未动。

原P0两个HIGH（固定command SHA唯一、目录fd/inode归属）未回退。STATIC-CHECKS对driver及原回归源做b8c5指纹相等核验；三项OS限制原测试的manager host成绩只复用为历史，未将它们冒作本轮整个工具包全绿。
