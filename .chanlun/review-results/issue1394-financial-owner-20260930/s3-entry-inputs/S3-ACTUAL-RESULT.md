46d 候选的 TestOnly S3 真实 Controller 生命周期已验证，两次验收脚本失败完整保留。

A/B 使用原clock，从read_config/start_s/init/serve到真实Ready；原输入Committed逐字段等冻结CLI原结果，原receipt和幂等重投同持久结果。A SIGTERM(-15)、B SIGINT(-2)，均通过同一Popen wait/reap和原socket身份清理。

C 使用批准的新clock，初次原ingest确已提交；跨clock内容根误比较失败后，经根明确允许从同一已停/reap库继续recover2。原message/attempt1不变，clock10/11/12→recover13；旧writer启动和原epoch1输入均拒绝且不改库，原身份receipt逐字段等本C第一次Committed。epoch2 SIGTERM(-15)/reap，所有短socket目录已清理。C原epoch1 -15为失败善后实际退出，未改记为0。

真实CLI wrong64sha、非法字符串、非UTF8声明三个负例均exit1，精确资源hash诊断；child前后DB/WAL/SHM完整bytes/SHA/mtime/inode相同，未创建writer lock或socket。

实际资源门：frame4096允许、4097拒绝，无完整LF读期限约1.005秒，第二连接拒绝。所有实际完整回复逐字核canonical+LF；private原始资源SHA与canonicalSHA分开记录。queue/write/response仅既有源及动态锁证据；单bar无写backpressure/full资源峰值验收，不声称RSS为零。95旧live原件全部逐字与mtime/inode未变，574当前源文件和冻结binary再核一致。

未运行Q/Browser、条件FIFO、AcceptedPending故障矩阵、9011/S16、正式容量、main/push/tracker。FIRST/SECOND失败状态保留于FAILED-RUN-1、FAILED-RUN-2以及C/CASE-FAILURE.json；最终续验收另记C/CONTINUATION/RESULT.json。
