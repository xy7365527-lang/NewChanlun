# DevSkim 1.0.90 维护镜像

CI 仍调用原 `entrypoint.sh`，保留原规则、排除项和零新增发现门。镜像内的
`/usr/local/share/devskim/maintenance.json` 与 OCI 标签标明 `1.0.90-nc-perf1`；
SARIF 的 `1.0.90+fb2d676ce4` 表示规则/上游基线，不表示未修改的官方二进制。

## 为什么需要维护补丁

#1419 / PR1855 的 q3 原件是 91.9 MB 单行 JSON。大量 hex 命中导致规则条件
和 suppression 重复扫描同一行；随后 SARIF 全量 DOM 的多份副本触发 OOM。
补丁只缓存同次扫描中的重复计算，并流式补齐原有 SARIF 默认 level；不删发现。

- `engine-cache.patch`：固定 ApplicationInspector 1.9.50 提交内的 same-line 缓存。
- `devskim-performance.patch`：固定 DevSkim 1.0.90 提交内的 suppression 缓存及流式写出。
- `locks/*.json`：四个源码项目的 NuGet 内容锁；构建使用 `--locked-mode`。

Dockerfile 固定原 SDK digest、两个完整源码提交和上游 CLI 包版本，只替换三个
已审程序集。RulesEngine 是 `SignAssembly=false` 的本地维护构建，不具有微软
强名称签名；不对外冒充官方 NuGet 包。原包内规则和其余运行依赖保持不变。

## 构建与回归

```sh
docker build -t devskim-scanner:1.0.90 -f .github/devskim/Dockerfile .github/devskim
```

构建内的 `regression/` 同时运行原包与实际维护安装目录，检查同一行多命中、
same-line 否定、有效/过期 suppression、默认 level，并比较完整 SARIF 对象。
没有通过回归的镜像不会产生最终构建层。此回归不替代当前 PR 的完整扫描和
零发现门；大型 q3 的 ARM64 性能证据也不替代 Linux x86_64 CI 的 30 分钟终态。

上游变更时，更新源码提交、重新生成并审阅锁文件与补丁，再做等价测试。
禁止为更新依赖在正式构建中关闭 locked restore。
