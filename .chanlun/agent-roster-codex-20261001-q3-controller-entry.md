# #1394 正式 Q3 Controller 入口工位

- 责任：Codex 根代理，现有 start_q 的显式 v3 接线与必要验证。
- 票：#1394，已批准 C §5.5/5.6.1；#1323 goal active。
- 工位：/Users/silencehan/Projects/NewChanlun-1394-q3-controller-entry-20261001。
- 分支：codex/1394-q3-controller-entry-20261001。
- 基线：08241aa12fa429245ba5132e6b6c2480f40029e4。
- 名分：隔离在制候选；稀疏检出 s_session，未合入 main。
- 范围：已有 query_resource_config 的显式 /3 schema 选择 Rust query-serve；原生命周期合同共用。
- 实测只沿此前具名单 bar Q21 域；不扩 S16/9011/存储范围，不新增页面宿主接口。
- 其他工蜂在独立完整客户端工位工作；不覆盖其产物。
