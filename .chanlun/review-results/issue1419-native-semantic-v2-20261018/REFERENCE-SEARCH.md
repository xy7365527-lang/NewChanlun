# #1419 本轮参考与来源

工作草稿。按本轮授权直接复用 run01 查证，不重做三方矩阵，不重下 donor、交接或三锁原件。本轮未调用在线 XERJ；前轮 `127.0.0.1:19200` 连接 exit 7 的原命令、原输出仍在 run01 `XERJ-ATTEMPTS.json`，不能称为本轮在线命中。没有重装、启动或重新索引服务。

本运行时工具清单没有 search_graph / trace_path / get_code_snippet / query_graph / get_architecture，因此用已知交接路径、`git show <fixed-rev>:<path>`、限定文件 `rg` 和类型声明核对。来源是当前源码和可追溯 donor，不以普通文本冲突或索引不可达停止实施。

复用文件的完整路径、字节数和 SHA256 见 `REUSED-SOURCES.json`，包括 ADAPTATION-MATRIX、DEPENDENCY-SOURCES、SOURCES-THREE-WAY、REFERENCE-SEARCH、XERJ-ATTEMPTS、REFERENCE-HITS。新源码的 baseline/donor/current blob 与 SHA256 见 `SOURCE-MANIFEST.json`。

实际参考核：

- current `s_content/capture_allocation/{work,native_owned,scratch}.rs`：CodecVec 的 Native 准入、有序插入参数、NativeValue 持费、WorkScope 错误与释放。
- current `s_session_v3/{audit_store,prepared,fixed_radix,source_contributions}.rs`：借用 reader、两段租约、Map/Tree retained 账及失败清理。
- donor `parser/{stroke,second_fold,segment,feature_seq,second_kind}.rs`：/2 真事实与 K1；`classifier/{center_scan,center,recursive_tower}.rs`：中心/五叶同核。
- donor `content_structure`：C/D/E/F 对象、精确请求、全 raw 版本来源；`tb02e::units_from_segment_rows`：成员极值、连续性、方向、L0 对照合同。
- donor `source_structure/source_audit/source_pending`：版本×kind、payload/links/full 与具名请求身份。当前 Native 合同接入后仍保留这些判据。

这些属于本作者静态核对，不是独立评审；未调用本机 cargo/rustc、产品、真实库、历史跑批或浏览器。
