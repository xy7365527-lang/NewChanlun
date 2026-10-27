# #1488 HashReader 持费返回值接线

2026-10-17，主控独占 `resume-1488/rust/src/bin/s_session_v3/history.rs` 的 `impl CodecRead for HashReader` 和新增测试模块。此为实施切片，不是完整 history 迁移。

`parse_rendered_plain` 改返回既定 PlainJson，调用同一 strict_plain_json_admitted；删除旧解析峰 permit.commit/retain，原许可随真正 PlainJson 析构。非 Owned 的 array/path/optional-field/fields/content-ref 输出接现有 typed Native 接口，数组结果容量使用 native_codec_room，删除与裸数组分离的永久 retain；Owned 分支保留。

新增一项源码锁：严格 JSON 重复键失败不留费用；正常非空解析结果在 reader/scope 析构后仍持费，最终 drop 回基线。所有 Rust 测试 **NOT_RUN**，单文件格式检查通过。

仍未覆盖：history 的 binding/stack/overlay、旧 NodeRead/CheckedFrame 兼容返回、capture/cache 移交、ExactRows 与跨请求池。不能据这段接口修复宣称整树可编译或全域释放完成。待独审核 parser 错误分类、当前 scope、可编译性及生命周期测试。
