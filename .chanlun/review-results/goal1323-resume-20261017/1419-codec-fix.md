# #1419 codec 接替实施（2026-10-17）

仅修改 resume-1419 的 rust/src/bin/s_session_v3/content_structure.rs；未提交、未推送、未运行 cargo。

## 实际改动

- 用 Value::Bool、NumberToken::parse(number.to_string())、Vec 对象条目和 render_canonical 对齐正式 codec。覆盖 serde_json 可表示的负整数、u64 全域、小数和指数，不再用布尔对象包装或整数专用降级。
- 三处 filter_map(decode_object.ok()) 合为一次 decode_prior Result 收集；任何节点读取/codec/JSON/对象类型错误直接返回，保留 codec 错误文本，不改写 MissingField。
- SecondFoldStore::new(round_budget()) 后恢复同一份游标簿，消除将预算 u32 当 store 及恢复到 clone 的问题。未变更预算值。
- TypedObject 交给现有 Cc004Producer::build_object 构造，形成全部内容边、身份、记录字段；源引用和坐标要求数组，字段缺失/非法 JSON 明确报错。
- 加入三个靶向测试：JSON 类型与数值域往返、真实缺节点在有效记录之后仍传播错误、非对象及非法 payload 拒绝。

## 验证边界

单文件 rustfmt 成功；git diff --check 成功。遵守主控禁令，未编译、未执行新增测试，不能宣称编译或验收通过。

## 仍存在的验收阻塞

E/F 仍用 segment:{generation}:{start_index}、obj-segment-{i} 和位置 i 作为合成身份/来源。本补丁不伪造正式身份替代。旧 tb02c/e/f 输出为 obj-，共享工厂输出为 obj3-，现有 relations 仍引用旧 ID；需要正式跨对象身份迁移合同。当前工厂按旧生产器的 tb02a::PROFILE + crate::RULE_REVISION 构造；与 V3_RULE_REVISION 的版本分歧须主控裁定，不能静默改档冒充兼容。request 仍为共享工厂默认 None，C/D/E/F 知识请求合同未在本补丁闭环。

这些问题意味着该桥接仍不具备生产验收资格。本次交付仅清除已知 codec/类型编译缺陷和吞错，不能关闭 #1419/#1323/#1279。
