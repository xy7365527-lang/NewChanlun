# #1493 S 正式会话入口复跑收据（HEAD，2026-10-20；私有 target 复跑版）

名分：工作草稿（宪法 §5），活期绑定 #1493，票关后随归档出仓。依据：主控裁定 #1493 issuecomment-6283988878 第 3 条。

## 结论

`PENDING_CASE_ONE → CONFIRMED` 在 HEAD 上经正式会话入口可见：turn_up、turn_down 两例 40 根全部经 launcher 起 S/Q、socket 逐条接纳、
公共 HTTP 查询（live / AsKnown / 重访）采完，手算神谕 PASS（cut 27 起 `PENDING_CASE_ONE`、cut 35 起 `CASE_ONE`），真 Chromium 读
`index.html` 在 cut 27 显示「结果 PENDING_CASE_ONE · 第一种情况候选态 OPEN_AT_PREFIX_END · 窗成员 [5]」、cut 35 显示
「结果 CASE_ONE · 第一种情况候选态 CONFIRMED · 窗成员 [5]」，页面无错误、无阻断框。

**未全过的部分（照实）**：脚本判定为 FAIL，原因都是合入 #1394 后公共读的规模界，与段语义无关；按主控裁定（#1493 评论），读超时并入 #1497，验收按「cut 27/35 页面正确 + 40 cut 离线全量校验 + Q 审计通过」接受，放宽的 Q 调度预算只用于本测试夹具：

1. 两例页面的第三个视图（末 cut 40 的 RecomputedWithRevision live 读）10 分钟内未完成提交，readiness 记录显示快照请求反复被客户端期限取消（复跑 turn_up 失败 65 次、turn_down 59 次；首跑为 74／66）；
2. 冻结 Q 配置下，两例在 live-36 越过生产客户端 30 s 总期限（每页快照约 800 ms）；因此本次用 `--relax-query` 让 turn 用例也用 e2 已有的放宽 Q 调度预算（`verify_deadline_ms` 120000、捕获/缓存 2/4 GiB），此后 40 cut 全部采完；
3. `tb02b_verify` 的全 cut 前端合同校验有 30 s 子进程界，40 cut 实际约 105.5 s（复跑；首跑 106–108 s。离线去界校验：两例 `validated_cuts=40`、退出码 0；Python Q 审计通过）；
4. e2_real_down（52 根真实笔保序映射）的 AsKnown 读超过生产客户端 30 s 总期限，未进页面阶段：首轮（共用 target）在 known-51 用 32.1 s（HTTP 约 5.9 s，其余为客户端校验），私有 target 复跑在 known-42 用 37.2 s（机器同时有他人负载）。合入前（前任 `034124c8aa`，旧客户端）同一用例通过。

## 运行

- **本目录文件取自私有 target 串行复跑**（#1488 通报：12:00–15:38 nc-remote 共用 `CARGO_TARGET_DIR`）：HEAD `9f3b5281eb`，`NC_TARGET=target-1493`（`~/nc-remote/target-1493`，flock 串行），二进制 sha256 `bb0805b1…666d`（与窗口内首跑同一字节）。远端原件 `~/nc-remote/tb02x/20261020-162953-e`，日志 `/tmp/nc-remote/20261020-162946-9f3b5281eb.log`（本机）。读数与窗口内三次首跑一致：两例 40 cut 采完、手算神谕 PASS（cut 27／35）、页面两视图可见、第三视图超时；全 40 cut 前端合同离线去界校验各约 105.5 s 通过。
- 命令：`python3 tb02x_runtime.py prepare --plan $OUT --binary $BIN --python /usr/bin/python3 --port … --relax-query`；`python3 tb02x_runtime.py session --plan $OUT --node /usr/local/bin/node --browser --playwright-module … --case turn_up`（turn_down、e2_real_down 同，串行）。
- 窗口内首跑（对照；已由上条复证）：`~/nc-remote/tb02x/20261020-140354-d`（turn，放宽 Q）、`20261020-134059-c`（冻结 Q 对照 + e2）、`20261020-131953-b`（首跑并发）。

## 本目录

- `HAND-ORACLE.json`：两例逐 cut 手算神谕（`check_case`，期望在 `tb02x_runtime.py` EXPECTED，不调用生产 reducer）。
- `turn_up/`、`turn_down/`：页面文本与截图（`pending-CC-012/013` = cut 27，`confirmed-CC-012/013` = cut 35）、`READINESS-DIAGNOSTICS.json`、`browser.stderr.txt`（第三视图超时）。
- `e2_real_down/`：首轮 known-51 与复跑 known-42 读超时摘要、复跑日志尾。
