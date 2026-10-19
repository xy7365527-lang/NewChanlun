# #1394 十六代 Q 冷读期限修复进展

本件是绑定 open #1394 的工作草稿。固定产品 `c15eeb351f66687225bef80301aadea261e4220c`，来源与证据见 [EVIDENCE.json](EVIDENCE.json)，独立两轴 [INDEPENDENT-REVIEW.md](INDEPENDENT-REVIEW.md)。没有关闭本叶或整图，没有合main或push。

已核帧持有同次读返回的既有Vec，唯一合法构造入口当场核F；字段私有，无未核/反序列化/可变body入口。NodeSource默认checked_read兼容原read实现，消费侧每次核proof摘要是否等于请求摘要。计费读门沿同一调用栈传帧与用量，避免Budget与消费读门重复F，不跨read/job/事务记忆body。三包装的门和首次起钟恢复原次序，写前F与物理ReadStore F保留。

Sandcastle DeepSeek execute工蜂实际输出空闲超时失败，部分两文件差异由原生管理者接管。第一固定版本e0a独评抓到1 HIGH期限/计费非等价；c15新增安全accept_frame入口及伪钟对拍后，独评Standards/Spec PASS，HIGH关闭。失败原件和旧Q观测均保留，未写成完整工蜂交付或一次全部绿。

冻结DB和测试资源分别仍为SHA `1704c9d9ef50a822653206e0bf1136b37280814322bbb44398761a38322c154a`、`50ea5bf865de8a37d8187069d70bee85ca4caac9c759a60ad2e3c717743d4eb1`；verify_deadline_ms仍5000。新c15一次Q16实际cold 4.570188209秒、warm 0.000374292秒，两次capture_digest相同、captures1/cache_hits1、DB前后不变。原件 [Q16-RESULT.json](Q16-RESULT.json) 与构建绑定 [Q16-META.json](Q16-META.json)。未改变捕获配方或增加资源，不重试至绿；单次通过不是稳定耗时或任意规模保证。

主控合入本票特性分支后实际串行S3 169通过/0失败/2诊断默认ignored；codec单元22、六内容组10/8/18/11/12/9通过。共259通过不代替整图条目。初次默认并发S3有5项期限失败，逐项单线程复测通过；最终主控整组串行通过，全部原失败记录保留。clippy退出0、有既有告警；全仓fmt检查有基线范围外漂移，未批量格式化。

只解决这份十六代冻结样本的实际查询失败。峰值资源、具名部署资源档、正式S/Q启停/恢复、非OHLC、客户端完整安装、E/B新域消费/真实财务故障矩阵、9011双跑及主线交付仍未验；产品前端仍独立#1460，六Destination保持。不得以本记录批准资金/语义政策或实场所外效。
