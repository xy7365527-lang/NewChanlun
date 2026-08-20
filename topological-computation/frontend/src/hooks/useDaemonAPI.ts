import { DAEMON_HTTP } from "../tokens";
import { createDaemonAPI } from "./daemonApi";

export { createDaemonAPI } from "./daemonApi";
export type { DaemonAPI } from "./daemonApi";

/** Backward-compatible default instance using DAEMON_HTTP from tokens */
export const daemonAPI = createDaemonAPI(DAEMON_HTTP);
