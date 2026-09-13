# #1372 stop 回执形状独立复验

结论：**PASS_BOUNDED_REAL_CLI_STOP_ENVELOPE**。固定 driver SHA `0a7ad225773c05c0f16dcd883ac3be134ab4f72354ca6d1b824d31301017480a`。

真实 controller main 对 stop-s/stop-q 始终输出单元素 result 列表（s_service_control.py:525–533）；Run.control:165–173 现仅对这两个动作要求“列表、长度1、元素dict”后解包，其它动作原样返回。CLI stdout 在解包前完整保存；后续 require_kill_receipt 仍严格核 service、pid、stopped 和 SIGKILL。

独立执行使用原字节复制的正式 launcher/controller/driver，实际启动两个本工位 `/bin/sleep 30` 替身，分别登记为 s/q。真实 Run.control→launcher→Controller.stop 对自有 PID 发 SIGKILL，两子进程均退出 -9；原 CLI stdout 为 list，驱动返回 dict，严格故障回执门通过。status 原双元素列表保持不变。另六个离线反例（空/多元素/非列表/非dict、错service、缺signal）均拒绝。共9检查，exit0，无遗留子进程。完整 stdout 与RESULT原件已保存。

旧模型直接 stub Run.control 返回 dict，没有覆盖 CLI 结果形状；此前限域模型通过不能证明这条实际接线。本轮明确补齐这一缺口，不改旧报告。

这是接口与自有替身进程停止证明，不是产品 S/Q、完整故障恢复或 C 成功证据。未访问正式 r3-a 服务或运行库。精确来源、行号与执行边界见 REVIEW.json/SOURCE.json；源码未漂。身份 `/root/r7_codex_product_review`，`codex_native_subagent`。
