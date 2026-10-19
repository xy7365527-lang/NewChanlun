# C6a 监督器 v2 原审窄复核

名分：绑定 #1406/#1323 的独审工作草稿；2026-10-17。结论：**原 HIGH 与 MEDIUM 具名缺陷均已修复，追加的隐藏文件上传 MEDIUM 也已关闭；监督器返修范围 Approve。不是完整 C6a PASS，不解除 PENDING。**

## 审查快照

| 文件 | SHA256 |
|---|---|
| `.github/scripts/run-1406-c6a.py` | `7f2318dab28d447fb3feea5d4f3e0c99cc4853a3f1c67cad141be346732add5b` |
| `.github/workflows/issue-1406-c6a.yml` | `75a06f24302d861c09630a99579192e815443a2a1c51a0ea4ea31ae919bfeace` |

工作树：`/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1406-c6a`。已读 `c6a-linux-supervisor-repair-v2.md` 并实际读返修两文件；未修改发行件。它们仍为未跟踪新文件，`git diff -- '*.py'` 无输出不代表无候选。

## 原缺陷复核

1. **失败原件上传范围 HIGH 关闭。** driver实际argv的output改成 `artifacts/raw-evidence`；原件从创建起即位于workflow的上传路径。模拟未回收组时真实 `real_run` 返回1，RESULT 为 `FAIL_C6A / UNSEALED_LIVE_OR_UNKNOWN / NOT_VERIFIED_REMOTE_UPLOAD`；不调用archive/inventory，不生成FILES或冒称完整封存。archive helper对source/destination不同直接拒绝，不能再移动仍在写入的原件。

   独立反控：在内存中仅把runtime布局变回 `scratch/runtime`，同一argv驱动替身与上传范围检查立即发现原件不在上传面。没有将旧路径硬编码进正常替身，没有改磁盘源码。

2. **临时名清单竞态 MEDIUM 关闭。** `regular_files` 在 `DirEntry.stat` 之前按相对名字排除；本轮 `.probe-N.json.tmp` 正确被排除。原件目录中的同名文件、旧序号临时名、一般隐藏文件仍列入清单，未广泛吞FileNotFoundError。

   独立反控：虚拟DirEntry的stat一旦被调用即抛FileNotFoundError。新判据没有调用stat；恢复旧判据后同一次遍历重新抛错。另用真实临时普通文件核raw-evidence内同名文件及普通隐藏文件均被哈希。

3. **上传时间不再隐含挤在最后30秒。** workflow实际执行step35分钟、upload step15分钟、job50分钟；脚本1200/1830/1920/2040/2070端点及资源值均未改变。增加的是上传上限，不是产品重启/执行续窗。动作失败/缺文件不能当交付成功。15分钟是预算上限，仍不保证网络完成，也不代替远端artifact实际可取核验。

## 上传边界：追加配置复核已关闭

[MEDIUM，已修复] 清单保留隐藏原件，但旧上传配置使用默认隐藏文件过滤

File: `.github/workflows/issue-1406-c6a.yml:42`

Issue（旧workflow SHA `73eeb3d1361620afb8e362474408ad444b1a5d93c2b52195c9b7ad0cd7a14b28`）: `actions/upload-artifact@v4` 默认不包含隐藏文件，旧配置未设置 `include-hidden-files`。返修清单保留原件目录中的隐藏文件，未回收失败路径不生成tar；隐藏原件可能被上传动作过滤。

Fix复核：主控已新增 `include-hidden-files: true`，位于唯一 `actions/upload-artifact@v4` 步骤的 `with` 下。实际path仍仅为 `${{ runner.temp }}/1406-c6a-artifacts/`，没有新增glob、checkout、凭据目录或整个scratch范围；执行35分钟、上传15分钟、job50分钟以及缺文件error均未改变。runner源码SHA保持不变。该项MEDIUM关闭。

证据边界：追加复核只读取本地workflow与计算哈希；按主控要求不重复32+5测试，没有网络、产品、外盘或真实Action执行。五项既有替身测试仍只证明本地原件集合；启用配置不等于远端已交付，仍须另核实际artifact与清单。

## 实际验证及保留门

- 作者替身套件：32 tests，OK，0.027秒。
- 本次原审独立套件 `test-c6a-supervisor-rereview-v2.py`：5 tests，OK，0.014秒；含两个回退反控、真实原件不误排除及archive禁移动检查。
- 原审v1反例文件保留未改。新套件另存，避免覆盖历史证据。
- 未调用真实进程/信号、/proc探针、产品HTTP、cargo、外盘、浏览器或远端。静态分析器环境仍未提供ruff/mypy/pylint/black，未声称其通过。
- `verify_runtime`仍PENDING。主控告知的语义器 started_at 自锚与 sys.modules 隔离问题属于独立审查线，本次不重复验证、不宣称已修。源码SHA/tree资格、真实Linux回收、完整41步及远端交付验收仍未完成。

本报告只解除两个具名监督器缺陷，不关闭C6a或#1323/#1279。
