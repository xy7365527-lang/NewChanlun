# #1672 有限容量修订待批准

主控采纳旧 CAPACITY-PROPOSAL.md 与异家族独审 1672-CAPACITY-REVIEW26BK.md 的受限方案 PASS。26BJ 完整论证不采纳，尤其不采纳必能跑完、cleanup恒满足、十轮越帽及未经测量的增长阶保证。

申请仅将 FIN/2 TestOnly 的 E/三个 B 独立 UDIF/HFS+ 卷从每卷16 MiB改为32 MiB，并实施及完整复跑。总名义容量从64 MiB变为128 MiB。
保留8倍页预算、cleanup/checkpoint预留、reader全部资源帽、WE7全部输入、12 Apply、四域恢复、21次原消息重送、两个macOS ignored测试实跑及完整原件。
复跑逐op记录E/B真实持久图像大小、页数、分配空间及cleanup余量，包括恢复/重送前后；不新增业务调用或时钟窗口，不用构造下一次Apply代替观测。
生产资源、资金、隔离及期限不变。32 MiB再次不足即停并回报，不自动扩卷。不包含main合入、关票、真实资金或场所动作。

已保存读数证明当前候选ccc115dd在B0的m-19失败，页帽507。32 MiB估算页帽1016，只是有限验证规格，不保证足够。旧run40178218217终态cancelled，不计成功验收。
独审原件 /Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1672-CAPACITY-REVIEW26BK.md。
原提案 /Volumes/AgentStorage/issue1672-hfs-resend-repair-20261025/CAPACITY-PROPOSAL.md。
批准前不修改卷尺寸或规格。
