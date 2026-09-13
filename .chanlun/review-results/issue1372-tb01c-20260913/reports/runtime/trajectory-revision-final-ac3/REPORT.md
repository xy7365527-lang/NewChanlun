# #1372 最终候选独立AC3运行事实

固定HEAD `3ff8c47ea4ab2aeb4672722e204d9f20b35f9b7f`。**公开Client在原30秒期限内完成跨修订完整续接：cut95→99，gap=null，96..99四批逐cut完整分页后一次原子提交。** 实际用时23.254250秒，113个Snapshot页，cut99含3项withdrawals和3项replaces；完成时真实输入head144，输入仍在持续提交。最终独评另汇合，不自称完整C或GUI通过。

初始load90及准备Watch90→94→95均使用同一正式Client。准备结束真实head95、已安装cursor95；等待真实head99后发送原cursor的Watch。四批全部比较期间已提交状态仍保持95，只有完整同cut分页、六族全字段/顺序、发布证据和cursor均校验完成才安装99。当前候选快照SHA `4e96d9d80fb3882b770b0827eeeebf21e4ec04e7d0efa156fce2cf7d86716f8e`。

保持原160输入、500毫秒节拍、S op95 afterBegin真实SIGKILL和epoch2恢复，原S/Q资源、页31、Watch4、30秒和64MiB不变；事前明确本臂不注入Q故障。不能将它与原Q109故障双跑的全495条生命周期视为同一轨迹。

实际HTTP 278次，全部278次完整，请求+响应原字节29435644字节，所有请求/响应/正文附件SHA及字节数均相符。输入驱动完成160提交、495条核心记录，runtime和consumer退出0，458份绑定源前后SHA一致。仅前494条输入/收据/切面/全表/schema子域与R4-a逐JSON相等；末条明确只有S重启、无Q重启。

采集和源尾核后，已用本臂正式controller精确SIGTERM停止本臂S/Q，完整回执含发信号后的观察警告保存在[POST-RUN-STOP.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-final-ac3/POST-RUN-STOP.json)，最终两个PID均匹配登记且state=stopped。未留下本臂服务给下一臂叠加负载。

原件入口：[FACTS.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-final-ac3/FACTS.json)、[目标命令结果](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-final-ac3/http-consumers/revisions/commands/revision-three-commit-reconnect/RESULT.json)、[完整已提交candidate](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-final-ac3/http-consumers/revisions/commands/revision-three-commit-reconnect/committed-candidate.json)、[事件行号索引](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-final-ac3/EVIDENCE-INDEX.json)。历史r1/r2/r3失败均保留，本次不改写它们的名分。
