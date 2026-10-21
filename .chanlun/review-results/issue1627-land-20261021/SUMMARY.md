# #1627 落地独审汇总

- 第 1 轮（`e5f472f25a`）：BLOCK —— qujiantao §14.1 `selector_keys` 默认值行被误替换成 `tower_base` 内容，§14.3 默认值行未改。
- 修复（`5050a5d4e4`）：从 origin/main 恢复 §14.1 原行，§14.3 默认值行改为指针；§9.6 补 C2 漏列（`ffi.rs:229-232`、测试段）与 C6 行号（`a_zhongshu_v1.py:183`）；三文件「最后更新」改 2026-10-21。
- 第 2 轮（`5050a5d4e4`）：PASS，blocking 为空；6 条非阻断。
- 第 2 轮后处理：C2 补 `stream.rs:205`、`rec_stream.rs:46`，去掉重复列出的 `rec_stream.rs:1270-1273`；C2 改法栏写明承接 #1654；beichi:461 不再括注列值域；qujiantao §9 第 35 行加 #1627 订正指针；zhongshu:138 去重复「保留」。beichi v1.4 changelog 订正注按主控要求（对齐 :1019 同句）保留。
- 草稿验收 1–8 两轮评审各自实跑全过；9–11 为合入/关票后的票状态项。
- 第 3 轮（`1e76abbe28`，增量）：PASS，blocking 为空；非阻断两条（C2 行号排序、证据里的临时 diff 路径）已顺手处理。
