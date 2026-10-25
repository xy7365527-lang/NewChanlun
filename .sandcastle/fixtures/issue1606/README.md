# #1606 调度等价基线

`claim-loop-before.mts` 是提交 `2bd876be4490c340048bbc9f0c63353b46f7f423` 的 `.sandcastle/claim-loop.mts` 原样快照。
该提交是 PR #1724 的堆叠父提交，属于 PR #1701。

`scheduler-config.test.mts` 用 Bun 1.3.13 对快照与现役脚本执行同一 `--once --dry-run`，比较 stdout 和模拟 gh 调用。此快照仅作历史测试输入，不是生产入口。保留原件，不随配置实现更新。

快照随测试交付，因此浅克隆和源码归档无需取得历史 Git 对象。CI 的 `sandcastle-ts` job 显式安装同版 Bun。

原件复核命令（仅维护快照时需要 Git 历史）

```sh
git show 2bd876be4490c340048bbc9f0c63353b46f7f423:.sandcastle/claim-loop.mts | cmp - .sandcastle/fixtures/issue1606/claim-loop-before.mts
```
