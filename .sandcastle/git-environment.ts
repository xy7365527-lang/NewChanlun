/** #1588：本地只读 git 子进程只拿定位与本地化所需的白名单变量。CLIProxy 密钥等凭据不进 git 及其 hook、credential helper、fsmonitor；
 * 宿主的 GIT_DIR/GIT_INDEX_FILE 等 GIT_* 也不透传，免得把调用方引到 -C 指定之外的仓库或索引。
 * 不含任何推送/取回凭据；需要访问远端的调用方不得复用本函数。 */
export function gitEnvironment(source: NodeJS.ProcessEnv = process.env): Record<string, string> {
  const result: Record<string, string> = {};
  for (const key of ["PATH", "HOME", "USER", "LOGNAME", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE", "LC_MESSAGES", "XDG_CONFIG_HOME"]) {
    if (source[key]) result[key] = source[key]!;
  }
  result.GIT_OPTIONAL_LOCKS = "0";
  return result;
}
