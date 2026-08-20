import { useEffect } from "react";
import { DEFAULT_INSTANCES, WS_THROTTLE_MS } from "../tokens";
import { describeDaemonConnectionError } from "../daemonConfig";
import {
  connectDaemonWebSocket,
  DEFAULT_WS_RECONNECT_MS,
  resolveDaemonWebSocketTarget,
} from "./daemonWebSocket";
import { useStore } from "./useStore";

/**
 * Connects to a daemon WebSocket and feeds throttled messages into the store.
 * 100ms throttle: D3 doesn't need every single step.
 *
 * All instances are peers — every WS connection feeds into handleWsBatch(instanceId, msgs).
 *
 * @param wsUrl - Optional explicit WebSocket URL; defaults to the first configured instance
 * @param instanceId - Optional explicit instance ID; defaults to the same configured instance
 */
export function useDaemonWS(
  wsUrl?: string,
  instanceId?: string,
): void {
  const handleBatch = useStore((s) => s.handleWsBatch);
  const updateInstanceState = useStore((s) => s.updateInstanceState);
  const target = resolveDaemonWebSocketTarget(DEFAULT_INSTANCES, { wsUrl, instanceId });

  useEffect(() => {
    updateInstanceState(target.instanceId, { wsConnected: false });
    return connectDaemonWebSocket({
      wsUrl: target.wsUrl,
      instanceId: target.instanceId,
      throttleMs: WS_THROTTLE_MS,
      reconnectMs: DEFAULT_WS_RECONNECT_MS,
      describeError: describeDaemonConnectionError,
      onBatch: handleBatch,
      onState: updateInstanceState,
    });
  }, [target.wsUrl, target.instanceId, handleBatch, updateInstanceState]);
}
