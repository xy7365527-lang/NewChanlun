# #1845 / PR1846 精确head行为验证

候选 12fa0dc8bfa1bad74d233375edb3c895bfe9069e。GitHub run 41357398565 的 head_sha 已核，记录 run.json。job 125000394545 已 completed/success，完整日志 production-session.log。

两次构建均真实执行：production_session_testonly 22 passed / 0 failed；production_session 24 passed / 0 failed。已逐名核声明匹配监听、缺文件/损坏文件/缺库声明/损坏库声明/声明不一致拒绝、读取字节计入启动帽、两个 standalone 保持原合同、launcher 三项拒绝映射、init和serve共享声明全部ok。此前失败的 s_serve_explicit_source_rejects_mismatch_with_both_canonical_declarations 两构建均ok。

已回查#1845当前票面验收1–9。45X完整源审和45Z增量源审覆盖1–8，本次精确head相关job逐名日志满足第9项的生产/TestOnly新旧入口要求。其余必要CI仍运行，不据此提前merge或关票。不将该单票结果计作整个Destination完成。

合入前继续核必要CI及当前集成；#1781同文件需按实际差异协调，不额外建依赖边。
