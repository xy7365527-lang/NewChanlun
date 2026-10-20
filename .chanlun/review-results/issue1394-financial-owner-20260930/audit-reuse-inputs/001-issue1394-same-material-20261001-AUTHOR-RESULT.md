# #1394 audit-only 同字节素材候选：作者结果

本地隔离候选 commit `c6255d308fc69d94178916a1dc5534546fb77591`，Rust tree `7196a77fef56cf87cbe91dc4b135326a142e2395`；基线 `818e024fdcfacb94564bd97d0be92890f7ab649b`，branch `codex/1394-same-material-20261001`。仅改五个 owned Rust 文件，没有 main/push/tracker/预算/教义动作。独立 Spec/Standards 尚待根任务组织，本作者不代替独评。

当前源码的真实 release 非 test 构建 exit0，实际 rustc 有 `opt-level=3` 且没有 `--test`。24 个不同的靶向测试均实际运行且零失败：content_store 12、暖素材 history 2、Template/Public/JsonInner 1、旧 metered 门序 1、旧 unbudgeted 4、state 空初态/query-only/冷重开损坏 3、库级精确重绑定 1。完整命令、真实 PID、同一 Popen wait/reap、stdout/stderr 和空间采样在 PASS2-BUILD-RESULTS.json/PLAN 中。第1轮两项错误 filter 运行零测试，原日志保留，不计为通过；第2轮已用正确名字补跑。

实际反例包括同事务 SQL UPDATE body/tag、独立连接短读间提交、DELETE、DROP TABLE、超 u8 tag、返回 Vec 改写、两 reader 各自首次 F、容量满/超大和坏当前素材、请求/root/header/SEQ_LEAF角色变化、Template→Public、Value→JsonInner、暖素材预算双 F/prepare顺序/前后 precheck 与暖素材期限前后门。相同 reader 的两次真实 SELECT，诊断计数 SQL=2、真实 F=1；默认 reader 两读仍 F=2，source.read 即使启用也仍每次 F=1。预算前门拒绝 SQL=0；读中期限超时 SQL=1，F 命中为0仍拒。

生产容量资格为 4 MiB / 65,536 entries；私有小资格测试观测 retained capacity 0/1/2 bytes 与 entries 0/1，变化旧素材回减至0、坏新素材不入。Map 元数据、实际满规模 runtime heap/RSS、首次复制瞬时峰值均 UNMEASURED；构建控制器的 cargo RSS 不是 runtime heap，不用来冒充。没有收益 pilot，也没有同源 S16/fullDB全字段对拍，walltime收益和 full9011 均未完成，留给独评通过后的独立工作。

稀疏 checkout 的 rust约26MiB/scripts约4MiB，使用 APFS `cp -cR` 复制既有 target；原575文件逐个核 SHA/inode/mtime前后完全相同，副本逐个独立inode。最终构建最小 sampled free 为286,777,344 bytes，未触250MiB门；所有自有进程已回收。根原五个文件当前 SHA 均匹配基线 git blob，候选源码编译前冻结后再次核 SHA 相同。

SOURCE-BINARY-BINDING.json 绑定五源码 SHA/git blob、commit/tree、patch与真实非test binary。binary SHA `c098bf76bb7870443b7564714ba2f9110f2c5593a3180ca13f9888c01ee16d17`，大小7,180,704 bytes。原target、基线文件、两轮全部日志和controller保留于此owned scratch。
