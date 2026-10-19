# #1394 监督独立审阅06（PASS，仅监督尾差）

审阅者：/root/owner_1394（接续负责人；非 93d1c9404d 作者会话）。subject `93d1c9404d`，基线 `4b3a84ebba`，日期 2026-10-19。

结论：PASS。REVIEW-05 的 MEDIUM（外层采样判断与 tick 内重读时钟分叉，跨采样点时绕过有限排队、误拒合法登记）已闭合。本结论只覆盖监督尾差，不代表 P11、产品或整票通过；execution_authorized=false。

## 审阅要点

- `supervisor.py` tick 新增 `sample_boundary`：只在 tick 自己一次读钟判定实际到期后才交给边界；send_boundary 不再自行读钟判断，两处分叉消除。未终止时边界先 drain 排队消息再由 `_sample_and_ack` 强制采样；已终止时 `tick(force=True)`，终止后的登记不追认。
- 工作截止、心跳截止、身份检查仍在边界委托之前执行；receive_deadline 不被心跳续期；资源帽与未知 PID 拒绝未改。
- 无新发现。观察（非缺陷）：拒绝路径 failures 记两条同因（tick 与 monitor 各一），与修前同型，owner 全部回收。

## 实际验证

- HEAD：test_review05/test_review04/test_supervisor 普通与 -O 各 31 项，30 通过；1 项 `test_real_short_toy_child_cancel_wait_reap_with_explicit_fake_identity` 因本审阅禁用 subprocess.Popen 而报错（该项启动真实 toy 子进程，本轮按约束不运行），不在本尾差范围。
- 基线 4b3a84ebba 配 HEAD 的 test_review05：5 项中 2 项失败（两条 REVIEW-05 反例，"存在未登记后代"），与 REVIEW-05 一致。
- 自写探针 REVIEW-06-SUPERVISOR-PROBE.py：count∈{1,2}、delay 0.124900–0.125048 步长 2.5µs 共 120 例合法排队，HEAD 0 误拒，基线 4 误拒（含 REVIEW-05 未列出的 count=2 delay=.1249725）；未登记后代 999 跨边界 4 例，HEAD 与基线均拒绝、未收养。普通与 -O 结果相同。
- 未运行产品、cargo/rustc、网络、浏览器、数据库、真实进程；未改源码。
