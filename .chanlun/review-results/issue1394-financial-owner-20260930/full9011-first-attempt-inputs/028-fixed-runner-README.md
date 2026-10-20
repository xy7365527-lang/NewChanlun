#1394 runner 集中修正工作草稿；本轮状态为 `S_Q_NOT_RUN`。仅执行纯离线测试、9011 材料门和具名 Root 输入门；未启动 S/Q/Controller/HTTP/audit，未打开已有 SQLite。

`runner.py` 绑定原件、预检与离线 verifier；`runtime.py` 保存供 Root 后续显式运行的路径；`evidence.py` 核原类型、clock、receipt、完整 schema/table_xinfo/BLOB/native hash；`test_offline.py` 运行无副作用反例。`contract.schema.json` 只描述 Root 原件派生的内部计划，不要求另造契约、容量证明或批准文件。

在 checkout 根运行纯离线检查：

```sh
python3 -B .scratch/issue1394-full9011-runner-20261001/test_offline.py
python3 -B .scratch/issue1394-full9011-runner-20261001/runner.py prepare --output .scratch/issue1394-full9011-runner-20261001/prepared-final
python3 -B .scratch/issue1394-full9011-runner-20261001/runner.py gate --prepared .scratch/issue1394-full9011-runner-20261001/prepared-final
```

`prepare` 仅写 isolated scratch；`gate` 只读，不建立 Controller/DB/socket。生产 Root HEAD 固定为 `3e5a5ba31dbe5c59beeee03de227fd8408eaac55`，Rust tree 固定为 `7196a77fef56cf87cbe91dc4b135326a142e2395`；私有 helper 提交号单独记录在 binding 中，不作为 binary 的构建 HEAD，也不要求合入 Root。binary/source/patch/非test release rustc 原证据继续核验。Root 正本为 `/Users/silencehan/Projects/NewChanlun-1394-content-v3/.scratch/issue1394-full9011-root-inputs-20261001/ROOT-ACCEPTANCE.json`，SHA `2f1753a4aed09c4fa28163354107a7891241649294813b96bc6ba2d99b0a8de4`；其具名 S7、caller、Controller-A/B、source/clock/schedule/envelopes/raw 与原 Q21 全部按路径、SHA 和完整值绑定，不修改 Root 原件来绿测。

仅接受 Root 的正式 output `.scratch/issue1394-full9011-formal-runtime-20261001`，DB 为其中 `{A,B}/s.sqlite`；Controller 私有 state 为相应 DB 同域的 `controller` 子目录。socket 仅 `/private/tmp/nc1394-f9011-20261001/{A,B}.sock`。已有 output/socket/DB/sidecar/state、symlink、重复域或 owner/inode/目录身份变化均拒绝；本轮不创建这些路径。

冻结 owner 门：启动/每请求30s，每 case 观察1h，cleanup 共用一个绝对30s deadline，其中最后5s预留保全。snapshot、hash、save、fsync 与 STOP/status/stop/reap/socket 清理全部受剩余时间限制，不把每次操作重置为30s。启动前6GiB空闲；自有目录、DB/WAL/SHM、Controller 和证据用真实 `st_blocks` 合算4GiB，写前/写后/每请求检查，超限停止。它们是私有观察/停机门，不是生产量纲、容量保证或9011工期预测。

100ms采样只报告样本下界。严格 RSS/heap/IO/内部阶段峰值为 `UNMEASURED`，不新增峰值框架或预先容量证书硬门。异常 exchange 也保存已有 request/reply wire、exchange 错误和可核持久 receipt；持久读取或 snapshot 做不了则记录 `UNMEASURED`，不补造。清理、采样或保全单项失败不跳过其它保全尝试；所有尝试共用 deadline。

原 native 单次 clock 按 `s_session_v2/mod.rs`、`s_session_v3/control.rs` 的既有语义核验：accept index=-1/ordinal=NULL/generation=n-1；begin/commit index=0/ordinal=n/generation=n；message_key 取原 header 的 namespace/epoch/message_id 规范数组。accepted_seq=n-1、attempt_count=1；每条 protocol 的 next_attempt_ordinal=n+1、frontier=本条 commit；最终 protocol 核最后一条，不以 A/B 同错相等放行。schema/table_xinfo 元信息与 typed rows 完整保存，拒 bool/int/REAL 混淆和 row_count=True。

Root 后续审定后可显式调用 `runner.py run --prepared <prepared>`；本轮没有执行。固定完整9011、原顺序、无重试、A 完整清理后才 B。未来只有双跑全逻辑门及原件/清理门都通过才允许 S 范围 PASS；仍为 `full_resource_capacity=false`，C/整图未完成。`verify --bundle <正式产物目录>` 只读 JSON/wire，不打开 SQLite。运行路径尚未经真实 S 实测。
