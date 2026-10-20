revision1 正常可写S共享源与只读Q的单bar真实入口验收 PASS_BOUNDED_JOINT_SINGLEBAR。

唯一运行修正是删除首轮harness人为db.chmod(0444)。正式Controller.start_s自然初始化0644的DB/WAL/SHM，直接Controller.start_q读同库；S PID11179、Q PID11194、UID501、argv/nonce独立。Q主DB fd5r，read_only flags与query_only源码锚已冻结；仅S持writer.lock。Q启动前后及post-publication读取前后全DB/WAL/SHM内容、mtime、权限完全不变，各closing(mode=ro)观察器亦未改变物理状态。

原message-1提交后Q由generation0观察到generation1的7条来源记录，Watch旧cursor0→1返回一批delta；旧cut0的content/asof读取仍为空前缀。S receipt/publication与Q generation/cut/index_frontier及原clock10/11/12对应，公共响应头独立校验；1845持久content节点、7个Q片段item根、49个返回部分证明节点及原OHLC内容根绑定通过。完整递归消费者闭包未执行。

Q SIGINT实际exit0、S SIGTERM实际exit-15，均同Popen wait/reap；独占Unixsocket身份清理、0700短目录删除、HTTP58882重绑通过。原件95件射程前后不变，冻结Root/tree/binary/七值/Q21均原值，无产品改动、build或unit重跑。

首轮131文件包及FAILED-VERIFY-1全部原样保留，首轮限于人为0444源；本revision1补足自然可写源验收。没有扩大至全源、Client物理allocation、生产capacity、FIFO、Browser、9011、wholegraph或main/push/tracker。
