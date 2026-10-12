# #1404 封包证据数据绑定

本机械核验只负责既有14份txt/PNG证据数据的原件类型、版本和完整字节绑定，不是新增产品代码评审。代码与测试源的独立评审仍由REVIEW.md及其对应补审、TAIL-INDEPENDENT-REVIEW.md承担。

已将public951版本、当前230 Git版本和工作树文件逐字节核对，14项完全相同。txt均UTF8可读，实际测试/构建footer保存在BUNDLE-EVIDENCE-BINDING.json；developing.png保留原始PNG签名和完整摘要，没有生成新截图。ui-red.txt是历史预期失败，不列为当前成功检查。其他历史日志的源版本、命令及适用域以VALIDATION.json与DELIVERY.md为准，不将工蜂初版历史通过升级为新尾部通过。

| 证据文件 | 类型 | SHA-256 |
|---|---|---|
| `developing.png` | 历史PNG证据图 | `280cfa57ef5fae9bc15fb29df0a9c0578c2552f1d49222ea3cd13167009d6a91` |
| `final-javascript.txt` | 历史原始日志 | `adfd2f54716cfa85ef0a5209a3a7b0c0502cac963ebdcbf2c24f3eebe53da86b` |
| `final-member-source.txt` | 历史原始日志 | `041d78cf011dda8e0d11837939cd6024f50e9aca82d42c305b986711f58c0be7` |
| `final-oracle.txt` | 历史原始日志 | `66c72505a2bcc3168f10088791e2271836c4efb830548f2d05cef0e9f104e04d` |
| `final-python.txt` | 历史原始日志 | `5dc3a6fa0695f525a243e109e06bc3242c52f4eed2e1b025c5062478792a7292` |
| `final-release-build.txt` | 历史原始日志 | `ab0239f4579784e4c230cc14b9c7fc2e62a8f1767ade33d5f52691ba9044f9e1` |
| `final-rust-parser.txt` | 历史原始日志 | `dc2531abc321f94bb2b57481135d86aaf23763f2eca2db3718b67dc6d9ae70ec` |
| `javascript-observer.txt` | 历史原始日志 | `8f706f67bc61b1330997a935574b1737d86502c1396d862782e09855bca75235` |
| `python-contract.txt` | 历史原始日志 | `698d16d7b93c4bd9707d3cfacda6fb1e0fea99ed0cda6e69bedc42772940540a` |
| `rust-feature-regression.txt` | 历史原始日志 | `898a8fc27ce2502ea169326253a44456d61f4d150a2269995fffe9b8809df373` |
| `rust-parser.txt` | 历史原始日志 | `25ef2d9914827557691f424246e1348a3cd529ba7c6df0022ef5eeaf5e05ce9d` |
| `rust-persistence.txt` | 历史原始日志 | `3e2773ccb370296eb4c9d90d19eee4652cde0bdb44389ffa3b19630a2a7185f3` |
| `rust-segment-regression.txt` | 历史原始日志 | `b57c4354b652a4ccd2a5aea64ed9bd0b85475a14709a3dddfa6e5b925fd13589` |
| `ui-red.txt` | 历史原始日志 | `41406f370942ac17a7a4436fd9c9a144429c3f3868c1494e1a4e62a4215044cd` |

manifest为满足保守分类如实把这些txt/PNG计入productDiff；本报告仅提供这14项数据证据的审阅覆盖，不把它们声明为可执行代码或新增编译对象。
