# #1372 驱动两处边界修复独立复验

结论：**两项已在本轮固定字节闭合**，没有在本限定面发现新阻断。

- DRIVER-FIX-05：原稀疏采样日志逐字复用，三个窗口均因少于两个不同时间点成为 NOT_VERIFIED；原健康日志仍得到有限观测结论。重复同刻样本不能凑齐两点；两点但无进展为 FAIL。源码 `tb01c_load_report.py:35–76` 只取声明闭区间内首末实测点，并记录实际跨度与两端未观测时长。
- DRIVER-FIX-06：原故障模型让空 marker 先可见、50ms 后完整，新的真实 `Run.fault` 方法完成模拟 KILL/恢复。`tb01c_runtime.py:45–64,287–288` 使用原 deadline 等待完整四行；错身份、重复键、超过 64KiB 均拒绝，持续半帧到期超时。

共 9 项离线/自有文件线程断言，exit0；无遗留线程，根源码未漂移。源码、原输入 SHA、完整结果见 SOURCE.json、RESULT.json，机器结论见 REVIEW.json。身份 `/root/r7_codex_product_review`，`codex_native_subagent`。

没有运行根服务、真实信号、产品数据库、GUI 或完整负载；合成日志的 160 条不构成 C 实际运行证明。前轮已闭合四项不重复验证，旧报告及反例保留不改。
