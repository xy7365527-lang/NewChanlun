# #1419 29Z 历史 base 原件导出检查点

## 运行标识

- **运行 ID**: `export-layout1-5123-29z-20261029`
- **时间戳**: 2026-10-29T05:17:44Z
- **仓库**: xy7365527-lang/NewChanlun
- **隔离分支**: `codex/1419-hist-export-5123-29z`
- **提交**: 70ba62dbb94e8b2583daff59390a2b6c208c86a9
- **base 来源**: 5123c9e2736573dd1c1232311c9c63ba0a1abc80 (public main)

## 输入源确认

1. **tb02b_oracle_rows**: 现有公开测试夹具（rust/src/bin/s_structure_session.rs:5956-5973），14行静态数据
2. **三组 cuts/rule**: 取自既有公开测试 `layout1_generation_roots_frozen_from_base_main`
   - long: 14根，rule "r"
   - revision: 8根（含修订/复原/移动），rule "r"  
   - realrule: 3根（g5/g9/g14），rule V3_RULE_REVISION

3. **无私有内容**: 无真实市场数据、账号、密钥、本机路径、会话记录

## 产物范围

### 代码变更
- `rust/src/bin/s_session_v3/generation.rs`: 新增 141 行测试函数 `export_layout1_historical_roots_5123`（--ignored）
- `.github/workflows/issue1419-hist-export-29z.yml`: 43 行 dispatch-only workflow

### 导出内容（在 CI artifact 中）
- **节点集**: 完整 MemoryNodes 迭代器输出（digest, tag_byte, body_hex）
- **生成态 JSON**: 每个 cut 的 `generated_state` 和 `state_without_axes`
- **25 根比对**: TSV 格式，包含 tag/gen/cut/prefix/rule/expected/actual/match
- **cuts 元数据**: JSON 格式，包含所有 cut 的 ContentRef、nodes 计数

## 执行边界

1. **隔离执行**: 仅在 `codex/1419-hist-export-5123-29z` 分支触发 workflow_dispatch
2. **--ignored --exact**: 测试不在常规 CI 中运行
3. **环境要求**: `NC_EXPORT_DIR` 环境变量指定输出目录
4. **artifact 保留**: 30 天
5. **无修改现有测试**: 不影响任何既有 CI gate

## 25 根预期

### long (14 根)
```
sc1-948366d1a1a96702e595c31a981d44a8cd860d8079cbff9f96ac6cb6c174c4a7
sc1-f66ed7a7823e94b2600747308722749f3d6de9a17b4ef0cc79fa4fe2c968b4a4
sc1-20c92b84d00d51bef243e8151905fc94f05fa0d75777f92e9cf16deb03ffe855
sc1-9373c03393f0b22bc9b7419ca0fe3f0a089083a3ec5c322a0d94e7cef6d655db
sc1-94022dcec4c0d9e81058e584398b2a132bac9cb328481261a09b34a3c8f7da37
sc1-9ed3773460b18c74c8878c6b7e759d64f6443ebe0bcbce27b723dc2f3a9c8c2f
sc1-31cd8bc881d70f7bf698ecb9fd6ca9740310f3e111df6a0d1d742434cb10b883
sc1-77f7e973924aa969081800c6a99e2a0d6d800ae2551d2fcec29cb3e9568bd903
sc1-f944daa8922cbae6e56b8b6f6e43746c59ee0bd2002be3f0b2c6a791f88553dc
sc1-55c40900ebcd79380bf25f8eb7d262f7ad79aba660fafb82d3070c519111966b
sc1-4d3ff43c59eb4d8434adb7dee4842a5271d07cfcfa0ed4e8f8eec62140231b5b
sc1-a4c940d2cc628929764d8a94c4f6774dec6cb8f9b98dfb1253cf117cddd8ac49
sc1-cc91d04f817bd368f7ab43b15320656cd20765c87ce2b9d4269995f863f10290
sc1-1f1883260c44c0de1f736eecfa10b260299e8b37638fb83ea79672e050e64b66
```

### revision (8 根)
```
sc1-948366d1a1a96702e595c31a981d44a8cd860d8079cbff9f96ac6cb6c174c4a7
sc1-f66ed7a7823e94b2600747308722749f3d6de9a17b4ef0cc79fa4fe2c968b4a4
sc1-5f62791ebc8f826d882fcee15ffa09edcdab873bc27b04261e99cc7ed29521f1
sc1-f93b79013d5bea7c79b28f60e64e82115a3e09f838c26c83318e1e7c046cea06
sc1-b8cf282c385cc9878ca8f967ddf4ba8183a8e54be8823a177d04405d589f6db4
sc1-333091400aa357723f809c07d6a923e4755947413d04cbc1beade766072ec854
sc1-236fb13005488b6f7c966188ae62daa7ffa1143eb05fd00cf44dd4433a14b316
sc1-8a20f9cc37496026a94e59b4b45cc7b8f8a500dfd487b19e772a018694d9d063
```

### realrule (3 根)
```
sc1-c8f28f2cec73feaae72bf279e0790497d8d8af64197116e45e096b89992b214f
sc1-5f47222d095c66143e8cd5f52d70080f15b93218c7b8499edaa24948ad38a9ae
sc1-cf5c2f1e68e4a50dddc548abb9a2fd00725f23714e4c69fbb9b033549a369f14
```

## 下一步

1. 手动触发 workflow dispatch: Actions → #1419 29Z 历史 base 原件导出 → Run workflow → 选择 `codex/1419-hist-export-5123-29z` 分支
2. 下载 artifact `issue1419-hist-export-5123-29z`
3. 检查 `compare.tsv` 确认全部 25 根匹配
4. 验证节点集完整性与生成态 JSON 格式

## 安全确认

- ✅ 输入来源: 公开测试夹具 tb02b_oracle_rows
- ✅ base 验证: 5123c9e2 是 origin/main 祖先
- ✅ 无私有内容: 无真实市场数据/账号/密钥/本机路径
- ✅ 隔离分支: 不影响 main 或其他开发分支
- ✅ 仓库授权: 用户已有 push/CI/外发授权
- ✅ 产物公开: 仅导出已在同仓公开的合成 fixture 衍生物
