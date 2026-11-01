# #1672 32 MiB 实现与主干集成候选

终版 `0932d3dbcdb18d138ce1e67ed05f971d87e75e3b` 已普通push，远端fmt及回归通过。PR https://github.com/xy7365527-lang/NewChanlun/pull/1702 。分支 `codex/issue-1672-fin2-hfs`。工位 `/Volumes/AgentStorage/issue1672-fin2-hfs-20261025/worktree` 干净。批准登记issuecomment6424282896。未合main、未关票。

## 交付变化

- 四独立UDIF/HFS+ TestOnly卷各32MiB，reader、8倍页帽、cleanup、生产资源与期限未改。
- 原请求后、四域恢复前后、逐次原信封重送前后，读取持久图像BLOB长度、page_count、连接max_page_count、freelist、DB/WAL/SHM长度及st_blocks、容量/空闲量和原公式cleanup余量。成功路径不构造下一Apply。
- 容量复用生产carrier_capacity；connection_max_page_count与carrier_page_limit分列，后者只作诊断预算基准。
- 归档RAW/HAND、manifest、FIN输入、原信封/响应、recover回执、重送请求/响应、空间JSONL、cargo-test.log和最终book/计数。卸载前复制四域实际文件，归档/卸载失败保留根并失败退出。
- 补HFS内非挂载根负控。messages按实际四列k/payload_hash/receipt_key/request_json冻结全行，重送前后完整对拍，不再误调只读json列的store::rows。其它seq/next/receipts/applications/图像/epoch断言全部保留。
- 普通merge main@499841b59265c5395ccdd40b92684b5864802f4f。仅workflow冲突，保留main全部1716/1717作业，净差插入FIN/2及立即always上传，位置在G-1之前。已直接消费main的PR1738 transport同步修复，未另改生产。

## 两次旧运行的实际结论

- df02的run40380252050因Apple statvfs的u32/u64类型错误编译失败，未执行FIN/2。已修复并保留 `1672-32M-evidence27/df02-failure/` 全日志及artifact，不把它当容量失败。
- 1bbd的run40382609382完成6 Capture、3 Resolve、12 Apply及四域恢复，epoch全部为2。真实DB中B数量[2,5,5]、K[204,506,506]。随后在重送前因messages无json列失败，21次重送尚未执行。当前候选已修夹具SQL。
- 各卷实测容量33,513,472 B，页帽1016。E/B1/B2/B3观测最大page_count为209/584/605/622，观测点最小cleanup余量为22,950,272/21,803,392/21,717,376/21,647,744 B，未发生32MiB容量不足。
- 负控测试通过。四卷detach成功，cleanup_failed=0、root_exists=no。完整原件及只读汇总在 `1672-32M-evidence27/1bbd-run/`，汇总为observed-summary.json。
- 旧G-1有transport队列时限测试失败，230通过/1失败/11忽略。该问题已由main的PR1738修复，当前集成通过直接消费修复，不改断言规避。

## 当前验证与验收缺口

nc-remote在1bc593上fmt、FIN/1 allocation16与book7通过，日志 `1672-32M-evidence27/nc-remote.log`。9e777集成首轮fmt发现两处纯排版，未继续回归。0932仅采用该rustfmt输出整理query_map闭包和assert_eq，闭包多一对块括号、返回表达式不变。终版0932的nc-remote fmt通过，allocation16、book8、已修复transport时限测试1全部通过，EXIT=0，日志为 `1672-32M-evidence27/nc-remote-final.log`。

macOS唯一集成运行 https://github.com/xy7365527-lang/NewChanlun/actions/runs/40389540213 绑定9e777。主控明确要求不取消，由它继续产生真实组合证据；独审核9e777到0932确为纯格式后裁决复用，不额外触发macOS。若有语义变化则另判。最终PR CI和DevSkim须绑定0932。完整21次重送、新组合原件、独审和CI都尚未验收。若32MiB再次实际不足即停，不扩卷。

Model the Domain使观测保持只读phase/owner/message_id记录。Prove It Works要求真实macOS实跑及下载原件核读，Linux回归不替代。待办在1672-32M-TODO27.md，轨迹在1672-32M-decisions27.tsv。

终版PR常规CI为40390534105，DevSkim为40390534300，交接时均在途。macOS继续复用既有40389540213，不重复触发。状态原件为 `1672-32M-evidence27/ci-handoff.json`。
