import { useEffect, useRef, useCallback } from "react";
import { WS_THROTTLE_MS } from "../tokens";
import type { InstanceConfig, InstanceState, WsMessage, WsStepMessage } from "../types";

const RECONNECT_MS = 3000;
const TRAVERSAL_HISTORY_CAP = 50;

interface InstanceConnection {
  ws: WebSocket | null;
  buffer: WsMessage[];
  flushTimer: ReturnType<typeof setTimeout> | null;
  reconnectTimer: ReturnType<typeof setTimeout> | null;
  destroyed: boolean;
}

export interface MultiInstanceCallbacks {
  onStateChange: (instanceId: string, state: Partial<InstanceState>) => void;
  onWsBatch: (instanceId: string, msgs: WsMessage[]) => void;
}

/**
 * Manages WebSocket connections to multiple daemon instances.
 * Each instance gets its own connection with independent reconnect logic.
 */
export function useMultiInstanceWS(
  instances: InstanceConfig[],
  callbacks: MultiInstanceCallbacks,
): void {
  const connectionsRef = useRef<Map<string, InstanceConnection>>(new Map());
  const callbacksRef = useRef(callbacks);
  callbacksRef.current = callbacks;

  const connect = useCallback((config: InstanceConfig, conn: InstanceConnection) => {
    if (conn.destroyed) return;

    const ws = new WebSocket(config.wsUrl);
    conn.ws = ws;

    ws.onopen = () => {
      callbacksRef.current.onStateChange(config.id, { connected: true });
    };

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data as string) as WsMessage;
        conn.buffer.push(msg);

        if (conn.flushTimer === null) {
          conn.flushTimer = setTimeout(() => {
            conn.flushTimer = null;
            if (conn.buffer.length === 0) return;
            const batch = conn.buffer.splice(0);
            callbacksRef.current.onWsBatch(config.id, batch);

            // Extract step info for instance state
            const lastStep = findLastStep(batch);
            if (lastStep) {
              callbacksRef.current.onStateChange(config.id, {
                currentPositionLabel: lastStep.position_label,
                steps: lastStep.step,
                beta1: lastStep.beta_1,
              });
            }
          }, WS_THROTTLE_MS);
        }
      } catch {
        // ignore malformed messages
      }
    };

    ws.onclose = () => {
      callbacksRef.current.onStateChange(config.id, { connected: false });
      conn.ws = null;
      if (!conn.destroyed) {
        conn.reconnectTimer = setTimeout(() => connect(config, conn), RECONNECT_MS);
      }
    };

    ws.onerror = () => {
      ws.close();
    };
  }, []);

  useEffect(() => {
    const connections = connectionsRef.current;

    // Determine which instances to add/remove
    const currentIds = new Set(instances.map((i) => i.id));
    const existingIds = new Set(connections.keys());

    // Remove connections for removed instances
    for (const id of existingIds) {
      if (!currentIds.has(id)) {
        const conn = connections.get(id)!;
        destroyConnection(conn);
        connections.delete(id);
      }
    }

    // Add connections for new instances
    for (const config of instances) {
      if (!connections.has(config.id)) {
        const conn: InstanceConnection = {
          ws: null,
          buffer: [],
          flushTimer: null,
          reconnectTimer: null,
          destroyed: false,
        };
        connections.set(config.id, conn);
        connect(config, conn);
      }
    }

    return () => {
      for (const conn of connections.values()) {
        destroyConnection(conn);
      }
      connections.clear();
    };
  }, [instances, connect]);
}

function destroyConnection(conn: InstanceConnection): void {
  conn.destroyed = true;
  if (conn.flushTimer !== null) clearTimeout(conn.flushTimer);
  if (conn.reconnectTimer !== null) clearTimeout(conn.reconnectTimer);
  conn.ws?.close();
}

function findLastStep(msgs: WsMessage[]): WsStepMessage | null {
  for (let i = msgs.length - 1; i >= 0; i--) {
    if (msgs[i].type === "step") return msgs[i] as WsStepMessage;
  }
  return null;
}

/**
 * Builds a traversal history from WS batches.
 * Called by the consumer to maintain position history per instance.
 */
export function appendTraversalHistory(
  history: string[],
  msgs: WsMessage[],
): string[] {
  const newPositions: string[] = [];
  for (const msg of msgs) {
    if (msg.type === "step") {
      newPositions.push((msg as WsStepMessage).position);
    }
  }
  if (newPositions.length === 0) return history;
  return [...history, ...newPositions].slice(-TRAVERSAL_HISTORY_CAP);
}
