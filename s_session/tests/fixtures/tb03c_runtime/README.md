# #1406 C7a：WE-5 正式进程运行夹具

名分：绑定 #1406 的 TestOnly 验收夹具。种子不包含正式资金参数、FIN/2、真实 S 或 γ 权威；没有浏览器与性能验收。

`SEED.json` 固定既有 WE-5 原 RAW、HAND、manifest 与独立 `tb03c_oracle.py` 的 SHA。原文件不复制、不改写；驱动通过正式 `init/serve/recover` 和 Unix socket 进入 E/B。等待和拒绝不允许新持久收据；同身份重送必须返回原回执并保持十张表不变。

本机允许的轻量检查：

```sh
/usr/bin/python3 -B -m unittest discover -s s_session/tests -p test_tb03c_runtime_contract.py -v
```

这只核驱动契约与变异负控，绝不代表 Rust、真实进程或恢复通过。

远端执行前须从待验精确提交构建 `e_economic_session` 与 `b_chong_session`，保存真实命令、退出码、完整构建日志及两个二进制 SHA。构建证据 JSON 必含 `source_sha`（40 位小写 SHA）、`command`、整数 `exit_code: 0`、`binaries`（两个名称映射 SHA256）。驱动核当前 checkout HEAD 与 source_sha 相等、实际二进制 hash 相等；构建证据本身的来源仍由验收者核。

```sh
python3 -B s_session/tests/tb03c_runtime.py --binaries /absolute/binaries --build-evidence /absolute/build-evidence.json --output /tmp/c7a-run
```

输出目录必须不存在，Unix socket 路径须少于 100 字节。运行会顺序执行六轨：normal 两次、before_commit 两次、after_commit 两次。每轨的实际运行目录相同，结束后连同 DB/WAL/marker 移入各轨证据目录，再从空状态开始；不覆盖既有证据，不按名称批量杀进程。

故障只作用于 WE-5 r6 Resolve 的 `step-014`。marker 的 domain/message/phase/pid/test_only 全字段必须对应本次 Popen 句柄，再 SIGKILL 并 wait。提交前须无残留事务，提交后须原收据恰好一份。E 与 B 在故障后分别实际 recover，使用 CLI 回执 epoch；每 epoch 时钟区间在相应恢复时刻之后。B 再独立恢复并重送费用；最后停止并移走 E DB/socket，再以新 B 进程恢复和读取。

验收比较分三层：

- 每个业务步骤：独立 oracle 全部金额/阶段字段、完整书账键集、原件/费用来源、逐 token 等待、逐事实 Applied/永久收据和开局 E 收据绑定。
- 单次恢复：十表原始行仅允许 `meta.writer_epoch` 与追加的 `epochs` 行改变，其余连 JSON 文本逐字不变。
- 同场景双跑：十表完整行逐字一致；不同故障场景：逐步骤完整经济/因果投影一致。不同场景的 epoch/时钟/根哈希差异不伪称整库相同。

只有六轨、四次实际 Resolve SIGKILL 和所有检查全通过，才写 `RESULT.json: PASS_C7A`。无跳过开关。失败保留原始日志和 FAIL；只读数据库不是预期值来源。输出含启动命令/PID/回收码、完整请求响应、marker、恢复前后表、各步 oracle 与最终十表。

继续开放：FIN/2 正式责任链、真实 S/γ、WE-1/2/7、RF-01 坏重隔离、C4b 未覆盖成功域、C5/C6 与完整 C7。C7a 不构成 #1406 关票依据。
