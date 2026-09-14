# #1394 S 变化持久化工位

名分：绑定 open #1394 的在跑工位。原生 Codex 子代理监督一次 Sandcastle direct Codex Flash/max 实施；Astra 保留方案和独评决定。

- 工作树：`/Users/silencehan/Projects/NewChanlun-1394-s-delta`；分支 `codex/1394-s-delta`。
- 起点：`21580a1b336b198e04f1ce9fbffae12d95ca19c5`，继承已独评缓存修复。
- 范围：S 新库仅持久化真实变化的 Delta，Rust/Python 审计同步；旧格式仍严格按旧表示核验，禁止失败后换格式重判。
- 保持完整批次、公开对象和批次 Hash、全部历史证据、首次发布元数据与 fresh audit 义务。未知/损坏格式拒绝，不迁移旧库。
- 独立验收前不合入 main、不覆盖原基线数据库或二进制；不修改公共资金规则。
