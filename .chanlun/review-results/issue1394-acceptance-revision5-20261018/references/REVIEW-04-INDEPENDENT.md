# #1394 独立审阅04（正式BLOCK）

固定提交827ac95f3b9aebc36c0a493fe273aac7699481e0；真实Sandcastle run 1b06e4db-5d84-4aee-af84-cec842cc23ab，external session 01a14d80-495b-7223-8684-f950a2cb7eda，2026-10-18T05:39:09.427Z实际退出，入口/worker均0。执行完成，内容结论BLOCK。

审阅执行完成，内容结论 BLOCK。N2 已闭合；R-F5 原排队误拒已修复，但新增两处发送边界回归，尚未闭合。F1/F2/P0、R-F3/R-F4/R-F6、N1/N3/N4/N5 的原实现与证据绑定保持；监督发送后的截止及采样保护出现回归。本结论仅覆盖工具尾差，不代表整票、P11 或产品通过。

## 正式发现

- [HIGH][R-F5] s_session/tests/issue1394_acceptance/supervisor.py:261-262：批量 ACK 在 :100-105 统一预检后逐条发送，发送下一条前不再核其阶段截止。内存复现：两个合法 owner 101/102，阶段截止分别为相对起点 9 秒和 0.02 秒；队列为 ACK101→ACK102→COMPLETE，每次 receive 推进 0.001 秒；第一条 emit 耗时 0.05 秒。HEAD 在 0.053 秒仍发送已过期的 ACK102，并返回 CONTROL_REAPED；基线拒绝并保留 INCOMPLETE。第一条发送仍处于其自身阶段、work、heartbeat 截止内，因此不依赖违规传输实现。建议在每条 ACK 实际发送前重新核验该阶段及原 work/heartbeat 截止，采样到期时补查，停止发送过期确认。
- [HIGH][R-F5] s_session/tests/issue1394_acceptance/supervisor.py:263-265：最后一条 emit 返回后直接处理终止标记，没有再次核截止或采样；即使修复上一条的逐 ACK 发送前检查，此路径仍存在。单 ACK→COMPLETE 内存复现：work=10 秒、cleanup=12 秒，emit 推进 10.1 秒，HEAD 在 10.102 秒仍以 COMPLETE／CONTROL_REAPED 收尾，failures 为空。另令 emit 推进 0.2 秒，超过 0.125 秒采样间隔，并出现陌生 PID999；仍仅保留发送前采样并宣告成功。双 ACK 变体中 RSS=101 超过帽100也被遗漏。基线对应输入均拒绝。建议发送返回后、接受 COMPLETE/CANCEL 前重新核原截止并完成到期的全集与资源采样；异常保留 INCOMPLETE，继续只清理已核身份。

## 实际验证

- 前后 HEAD 均为 827ac95f3b9aebc36c0a493fe273aac7699481e0，工作区干净；相对 6cb6bbf33eb079f639abae2449ccc9b98d81a11c 的差异检查通过。未写 checkout、历史封包或 Git 状态。
- 已读取规定规约及封包。独审03报告和六条原探针的 SHA-256 分别匹配 3842328bbffc0db6c63c4b715bd4a479f63c0f130b225ff4cc4cbcaf47463816、accd04d4973e3bd80cd2d9ef9915b7c374aebcd0f343c4d5cf7523c831d1666f，并读取实际命令与输出。
- 22 项 manifest 的长度、权限、摘要全部匹配，连同 manifest 自身覆盖全部23个差异文件。26个最终 Python 指纹匹配；147项验证/复用绑定（前后指纹52、静态指纹26、复用67、运行输出2）全部匹配。普通及优化模式均为相同37方法，24旧＋13新；修前4方法原输出确为3 ERROR／1 FAIL。本轮未重跑这些集合。
- 23个旧 Python 文件与基线及 revision3 验证指纹一致；31项来源匹配。原41个 driver、27个 scheduler 方法完整保留；stop 等13个函数及 snapshot 分页块逐字不变。26文件通过普通/优化内存编译。
- N2 原件确为5940和6040字节，后者仅前加100空格，解析值相同且各含一个末尾LF；请求帽分别为5940和6040，原件与 fixture 匹配，共16个 closure 页。代码逐页按原长度核自身请求帽；绑定作者输出覆盖5940/5940通过、6040/5940拒绝、6040/6040通过及全部16页超帽拒绝。financial-basis 对象帽、Job 原件账、snapshot 原帧和捕获身份保持。三份固定协议来源及六处摘录匹配，strict_json 未新增 canonical 编码限制。
- 仅针对发送延迟新疑点运行内存 fake 探针，并将基线源码加载到内存对拍：合法对照两版通过；过期阶段 ACK、心跳/work 耗尽、到期采样遇超帽或陌生后代，均为基线拒绝而 HEAD 放行。单 ACK 终止变体同样复现；未使用真实进程或 OS Probe。
- 未运行37/68/91/82/host123集合、旧ps/socket三项、cargo/rustc、真实S/E/B/Q、Long、DB、网络、浏览器或资金动作；未读取凭据、启动工蜂或执行外部写操作。execution_authorized=false，产品全部NOT_RUN。
- 1488→1419→组合C1→统一新SHA及 financial_reader 语义重审要求保持；真实同域terminal/K/γ/Voice、两ARTIFACTS、逐对象资源与准入、A/B/故障/全历史/资金/恢复/隔离/独评仍欠。Journal candidate 不自动恢复接受资格的代价保留；前端归#1460、K4排除，相关图票与#1279保持开放，未新增全TB02 blocker或资源帽。
