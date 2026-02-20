# spec/theorems/

存放缠论形式化定理和证明。

## 用途

- 收录从 `.chanlun/definitions/` 和 `.chanlun/genealogy/settled/` 中提取的所有形式化定理、公理、关键定律
- 每个定理一个文件，格式统一（陈述 + 状态 + 来源 + 证明或 @proof-required）
- 文件变更自动触发 Gemini 数学验证（049号路由表）

## 标签

- `@proof-required` — 定理陈述已形式化，但证明尚未完成或需要外部验证
- `verified` — 定理已通过形式化验证或原文推导链完整
- `unverified` — 定理已记录但尚未验证

## 文件命名

`[编号]-[英文短名].md`，编号三位数，按发现/提取顺序递增。

## 来源映射

| 定理 | 来源定义/谱系 |
|------|-------------|
| 001-segment-decomposition | xianduan.md (第65课) |
| 002-divergence-buysellpoint | beichi.md (第24课) |
| 003-interval-nesting | beichi.md (第27课) |
| 004-trend-completion | zoushi.md (第17课) |
| 005-trend-decomposition-1 | zoushi.md (第17课) |
| 006-trend-decomposition-2 | zoushi.md (第17课) |
| 007-center-theorem-1 | zhongshu.md (第20课) |
| 008-center-theorem-2 | zhongshu.md (第20课) |
| 009-buysellpoint-completeness | maimai.md (第21课) |
| 010-updown-completeness | maimai.md (第21课) |
| 011-prohibition-theorem | 谱系 005a (第17-21课推导) |
| 012-conservation-constraint | liuzhuan.md [新缠论] |
