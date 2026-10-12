# #1404 完整交付检查与绑定口径

本文件把现有检查接到main基线8619a20至产品230f7cdb及后续纯报告提交的完整差异，不声称所有检查重跑于最终报告提交。

## 必需检查及适用域

| 检查 | 实际执行与结果 | 最终版本适用边界 |
|---|---|---|
| Rust lib编译和尾部13测试 | `TAIL-TEST-EXECUTION.json`记录2026-10-12T03:04:31Z开始，真实cargo退出0；完整输出`TAIL-TB02B.log`，13通过 | 唯一新增生产文件stroke.rs已经当前版本编译、测试；其公开接口未改。旧S二进制相关源码保持不变，原release构建见VALIDATION.build，不称当前release重编。 |
| TB02C八fixture | `TAIL-TB02C-PROVENANCE.json`恢复2026-10-12T02:28:13Z的直接cargo退出0；`TAIL-TB02C.log`八项通过 | 覆盖同一230版本的两方向28前缀seed/FIRST及负见证；未重跑这些已有有效检查。 |
| 原交付定向与正式运行 | `VALIDATION.json`、`DELIVERY.md`分别保留最终Rust10、Python14、Node5、旧版本release构建与R3四arm的实际命令/输出/身份 | 旧32项FINAL-SOURCE文件与230全部同字节，新增stroke尾部由上述检查与独评覆盖；R3仅复用已核冻结输入域。 |
| 完整扫描 | `TAIL-SCAN-RECEIPT.json`记录当前产品230，扫描器exit0、gate exit1 | 本轮41312条对public为0增0减，main增量105均有分诊；旧基线41207条未重新全部裁定。仅为带明确例外的候选就绪，不是全仓安全绿灯。 |

manifest中的`covers`保存所声明检查所绑定的完整最终产品字节集合，用于防止检查之后混入新产品改动；每一检查实际执行的行为范围仍由本表、命令和日志限定。不能从这个摘要集合推导每个脚本、二进制、PNG或历史日志都在某条cargo命令中被运行。旧检查的复用只因其对应产品字节与具名行为域未变，尾部13/8检查不替代全仓测试或重新执行R3。

## 逐产品提交与纯报告尾部

- d8a4dc2c8a9b22dd6360406a447b6d5c3c991cac：本片主体产品，R3前release编译及定向检查见VALIDATION.build与RUNTIME-SOURCE-R3；审阅主体及比较器补审见REVIEW/COMPARATOR-REVIEW。
- ce1ec9d283b05b5939e44f977a30848bd499d6f5：日志规范化文档提交，非新产品逻辑；不重新声明编译。
- 95165b79d7dea25c2f3b45e666596c03edde4c13：Rust/Python尾部修复，准确源摘要与检查见POST-PR-RUST-REVIEW、POST-PR-PYTHON-REVIEW及VALIDATION.post_pr_checks。
- 230f7cdb06feb5710d97a50cffe7cf01b6063041：仅stroke.rs新增尾部，当前lib编译及13/8检查、TAIL-INDEPENDENT-REVIEW对应当前指纹。
- 其后报告提交：仅本目录md/json/log数据实体。最终提交号由仓外manifest记录；纯报告提交豁免新编译，不改变上述真实执行版本。

## CI和合入限制

公开951版本CI run37560904248的test、s-observer、fixture-drift成功；rust-check默认域2946通过、3失败、161忽略，三个失败为m3_partition_synthetic_holds、calibration_sweep_finds_non_collapsed_segments、seam_btc_tick_bars_yield_non_trivial_structure，与main基线已登记失败一致。后续被跳过步骤不称通过。该远端CI属于公开951，不能充当230或纯报告最终提交的已运行CI。

本地封包预检是请求具体版本批准的材料门，不产生main合入权限。DevSkim红灯、尚无最终远端CI等限制须在请求批准时完整披露；合入后才可按交付纪律核origin/main、CI并处理关票。未执行push/merge/关票，第二种终结、一般同价裁定、S7/完整9011、166代、真实交易及父图仍不在本片交付声明内。
