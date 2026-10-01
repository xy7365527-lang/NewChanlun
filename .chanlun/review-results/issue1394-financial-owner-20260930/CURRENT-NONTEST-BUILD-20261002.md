# #1394 最新非测试构建与完整源绑定

本文件为open #1394工作草稿，承接已批准C与#1323后端总图；不证明正式全9011或关图。

针对原正式runner必须看到真实非test release编译的门，本轮冻结生产源HEAD abbc8ddbde4b50bf6c9268aec0f69fe315fe132a、Rust子树5da6fc6caec9cb6a80569ff9497a2a5e461e5e12。Sandcastle工蜂准备固定收包脚本，管理者在只读Python独评后执行确定性构建，避免再起第三个LLM工蜂。私有wrapper修正PATH裸名rustc探测、字段资格、证据拒覆盖及信号透明；原失败探测与错误终态均保留，没有真实编译成功前的成功声称。新增测试文件在原基线不存在，明确标记absent/null blob/空字节，不伪造旧Git原件。driver终态修正为必须error is None、cargo0、全部checks、绑定保全及自有target清理成功，失败不会被后续收据掩盖。

真实cargo/rustc退出0，实际52项argv含目标bin/link/opt-level3/default+s_session，无--test或--cfg test，wrapper started/finished同PID/inode/argv且无取消信号；Cargo artifact test=false、opt-level3、fresh=false。Python链接采用已存在3.12路径，仅在子进程指定，没有改变产品源码/RUSTFLAGS或输出环境/凭据。617个rust/s_session源文件逐一核Git blob、当前原字节及归档完整集合，7文件累计变化与新增文件名分保留。Root独立重算源归档、编译事件、产物stat/字节SHA和全部31份artifact manifest。

实际新binary SHA e0054e259ee8427298c56e70e3548cfae5546301fbac69bc8c3b480a24cf25f3，7197360字节、755；它与先前c133不同，只绑定本次真实产物，不宣称两binary等价或偷用旧标签。二进制和重建recipe保全后只清当前唯一自有target，旧589、旧c133/旧失败/pending三件不动。完整单包与大原件仍caller-owned，SHA/路径索引见JSON；必要实体一次冻入current-build-inputs。

全9011原件的source/clock/envelopes/schedule、逐请求身份/hash/顺序及1030字节最大LF帧另经原严格material验证器重核；这只是材料门，不是服务重放。新runner/Root A/B域与committed-source适配仍待冻结和独评。当前本地约4.0GiB，原6GiB启动门未满足，已询问存储事实；不降门、不使用此前失败外盘路径、不接续旧AcceptedPending28。S/Q/Driver、30秒服务域、全9011双跑/crosswalk/完整资源故障/Client实际allocation、C和六Destination保持未完成；main/push与真实场所仍为明确门，#1460前端独立。
