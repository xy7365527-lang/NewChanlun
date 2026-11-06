# PR #1449 writer epoch 分诊

已确认 B 完整性缺陷（Cloud Codex 评论3996936247），原750f版本不可据旧R12 PASS直接合入。

冻结binary SHA256 `e6fdf9deb087561ad574748ad991cf8f71f1b2a9bf9027f6678e8a8d358b9a59` 与750f后端源码相同。新建隔离库中，将持久writer_epoch改为foo、01、-1、空文本、9223372036854775808，分别传入同值配置：五臂accept/advance均exit0，真实接纳3条事件并发布generation1。合法epoch1对照也通过。另臂持久epoch=-2，recover请求-1成功修改writer_epoch；所以旧recover的规范i64解析也未排除负数。

根因是verify_writer_epoch只做裸文本相等；recover仅做有符号i64解析和大小比较。修复使用共享parse_writer_epoch（规范、非负i64），持久损坏映射StorageUnavailable、配置越域映射InvalidDomain，合法不同epoch仍StaleWriter。允许0及i64上界，不引入浮点。

原始回执：work/PR1449-EPOCH-PROBE/RESULT.json；每臂stdout/stderr与新建库同目录。测试脚本在私有work目录，无旧库写入。

R13新增两项拒绝测试在修复前失败，两项正常边界/换代控制通过；修复后四项全部通过。日志位于r13-build/EPOCH-RED.log与EPOCH-GREEN.log。该结论仅为实现者定点测试，最终候选仍需统一验证和独立增量复审。
