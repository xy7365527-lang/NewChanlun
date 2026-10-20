# #1394 现有 Controller 的 Q3 实跑

结果：两个全新 Q 实例均通过 `start_process("q", Rust argv)` → `wait_ready("q", query_epoch="1")` → `status` → `stop` → 回收。SIGTERM 与 SIGINT 均退出 0；控制 nonce、PID/启动身份/UID、生产 Ready 原始 HTTP 正文与 Controller 返回一致；停止后无 LISTEN，端口可重新绑定。

使用原 Q21 SHA `50ea5bf865de8a37d8187069d70bee85ca4caac9c759a60ad2e3c717743d4eb1`，未改资源数值。源为已有 S3 单 bar 数据，不生成结构数据；复制前 lsof 无打开者、两次 DB/WAL/SHM 哈希及 mtime 稳定且 WAL 为零，原数据库从未建立 SQLite 连接。每实例使用不同只读 main 副本与 state 目录；两副本 main 字节 SHA 未变、WAL 无结构写，原件三文件完全不变。

配置严格经现 `read_config` 的 `s-launcher/2` 校验，身份按原 INPUTS/input/init，七个 S 资源值来自已有 TB01C runtime-plan；本次未执行 S 门，不能声称 S 执法通过。环境由现 `Controller.runtime_environment` 以具名 Python probe 建立，继承环境值未输出。纯合成观察负控调用现 Controller，验证外来进程身份拒绝发信号、外来 nonce 拒绝 Ready；没有信号发送给外来进程。

二进制 SHA `388288c875f7b3ccfe7411943182602f78d7d3a437d34d9f2df283e2ae629275` 保留 `909ccff13abaed83be3c939ab7459d52ff4afee5` 整体来源绑定；Q3/control 相关三文件对当前 HEAD diff 为空，但不能当作当前 HEAD 全体二进制重建。原 7 页 / 37 nodes 内容案例只引用旧摘要，未重跑。

这是一份现有 typed 控制核心与旧小源域的生命周期验收，不代表正式 `start_q /3`、Browser 完整消费者、9011、全部资源、M1 或 main。所有启动的服务均已停止回收。详细 args、cfg、源文件、二进制、Python、profile/catalog/clock/resource SHA、进程原件及端口/数据库前后证据在单一 `ACTUAL-CONTROLLER-MANIFEST.json` 与 `attempt2/`。

第一尝试在 Popen 前因 scratch 二进制未保留执行权限失败，无 Q PID；失败原件保留于 `attempt1/`。仅将 harness copyfile 改为 copy2 保留权限后新建 attempt2，未改产品。实际执行 handle `32325` 已 terminal exit 0。继承环境值未归档，仅保留实际 runtime_environment 调用、具名 Python 探针及环境键集合指纹，不能声称完整继承环境值已冻结。
