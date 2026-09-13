# #1372 最终 AC3 跨修订续接独立审阅

**PASS_BOUNDED_AC3_REVISION_CONTINUATION**。固定候选 `3ff8c47ea4ab2aeb4672722e204d9f20b35f9b7f`，本票整体仍 **NOT_VERIFIED**，R5 故障成对轨迹和实际 GUI 另核。

原生 Codex `/root/r7_codex_product_review` 只解析封存原件，复用此前独立索引/历史投影器，没有调用 Q/JS 产品投影、SQL连接、服务或浏览器。原 1917 件（386,111,941 B）全部 SHA 相符；458 份源快照全核，其中22份直接对本候选 Git 对象，其余为冻结构建和本臂材料。原包 manifest SHA `5d2cfe5c9b1c493d4f06a7418f984c1dcec407ff63b0a46773d3031aacea648b`。

公开Client原已安装 cursor95，`http-consumers/events.jsonl:12` 实际head95时停止读取；`:13–15` 等到head99，用同一个原游标重连，结束时head144。实际 Watch 完整返回96、97、98、99四批，99有 **3 withdrawals + 3 replaces**，无Gap。全部 **113页+1Watch**，完成 **23,254.249667 ms**，请求/响应合计 **12,090,709 B**，未改变30s/64MiB、页31、Watch4。最终candidate `4e96d9d80fb3882b770b0827eeeebf21e4ec04e7d0efa156fce2cf7d86716f8e` 的完整state/projection/cursor均与独立投影逐字段、顺序和身份相同。

本臂全部 **278/278** HTTP原请求与响应完整，274页、8375条记录出现、4份完整已安装candidate和9份真实Delta均独核；所有页token原出处、offset、计数、完整摘要、因果回复头及连续游标均闭合。包括准备90→94→95，再95→99。before/after完整状态与正式Client唯一提交回执、已审原子安装代码共同支持提交边界；没有声称做过Chrome逐页状态观察。

输入确为固定160条、16次revision、500ms连续发送。目标Watch期间 **47个新offer、222个frontier观测**，Commit99→144，没有等待消费者ACK再投输入。三个事前10秒窗内实测Commit增加 **20/19/17**；live最大回执 **2570.237042 ms**，最后offer至phase结束 **2571.072292 ms**，均落原上限。原采样窗首末点与未观察边缘精确值见 `runtime-v2/EXAMINATION.json`，不外推生产稳定吞吐。

495条核心原值全核，包含160收据、160独立Delta/batch完整BLOB及15表/schema。前494条与R4-a逐JSON完整相同；末条仅S重启，**不是全495双跑等价**。S op95 afterBegin真实SIGKILL、epoch2恢复原消息、两次Begin/一次Commit和Q存活都由本臂原件核对。事前 `PLAN.json` 明确不注入Q故障，因此不能替代R5原Q109kill链。

原进程和consumer均退出0；正式stop中两个回执PID均匹配其本臂登记且stopped/SIGTERM。发信号后的两条身份观察警告原文保留于 `POST-RUN-STOP.json`，不抹去。首次复用的旧审阅器因本专臂没有旧式 `load-report.json` 停止；失败stderr保留，新审阅器直接计算原events三窗口后通过，没有补造运行汇总或修改源。

详细SHA、路径、行号、每页字段与全部执行边界见 `REVIEW.json`、`HTTP-EXAMINATION.json`、`runtime-v2/EXAMINATION.json`、`SEAL-WINDOW-EXAMINATION.json`。旧失败没有改写；本结论只关闭这个实际跨非空修订续接缺口。
