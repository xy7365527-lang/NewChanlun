/**
 * useMultiDaemon.ts — manages N daemon instance connections.
 *
 * All instances are peers. Each instance:
 *   - Creates a daemonAPI (factory function)
 *   - Connects WS (parameterized)
 *   - Polls /status every 1s
 *
 * The "active" instance (user-selected) provides topology/narrative/gaps/operations
 * to the store-level fields. Other instances update instanceStates only.
 *
 * Output: merged instanceTraversals for TopologyView.
 */

import { useEffect, useRef, useMemo } from "react";
import { STATUS_POLL_MS, INSTANCE_COLORS } from "../tokens";
import type { DaemonInstance } from "../tokens";
import type { WsMessage } from "../types";
import { createDaemonAPI } from "./useDaemonAPI";
import { useStore } from "./useStore";
import type { InstanceTraversal } from "../components/TopologyView";

/**
 * Manages a single WS connection imperatively (not as a hook per-instance,
 * because hooks can't be called in a loop). Returns cleanup function.
 */
function connectInstanceWS(
  wsUrl: string,
  instanceId: string,
  isActive: boolean,
  handleBatch: (msgs: WsMessage[]) => void,
  handleInstanceBatch: (id: string, msgs: WsMessage[]) => void,
  setWsConnected: (v: boolean) => void,
  updateInstanceState: (id: string, partial: { wsConnected: boolean }) => void,
): () => void {
  let destroyed = false;
  let ws: WebSocket | null = null;
  const buffer: WsMessage[] = [];
  let timer: ReturnType<typeof setTimeout> | null = null;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  function flush() {
    if (buffer.length === 0) return;
    const batch = buffer.splice(0);
    if (isActive) {
      handleBatch(batch);
    } else {
      handleInstanceBatch(instanceId, batch);
    }
  }

  function connect() {
    if (destroyed) return;
    ws = new WebSocket(wsUrl);

    ws.onopen = () => {
      if (isActive) setWsConnected(true);
      updateInstanceState(instanceId, { wsConnected: true });
    };

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data as string) as WsMessage;
        buffer.push(msg);
        if (timer === null) {
          timer = setTimeout(() => {
            timer = null;
            flush();
          }, 100);
        }
      } catch { /* ignore */ }
    };

    ws.onclose = () => {
      if (isActive) setWsConnected(false);
      updateInstanceState(instanceId, { wsConnected: false });
      ws = null;
      if (!destroyed) {
        reconnectTimer = setTimeout(connect, 3000);
      }
    };

    ws.onerror = () => { ws?.close(); };
  }

  connect();

  return () => {
    destroyed = true;
    if (timer !== null) { clearTimeout(timer); flush(); }
    if (reconnectTimer !== null) clearTimeout(reconnectTimer);
    ws?.close();
  };
}

export function useMultiDaemon(): void {
  const instances = useStore((s) => s.instances);
  const activeInstanceId = useStore((s) => s.activeInstanceId);
  const setStatus = useStore((s) => s.setStatus);
  const setTopology = useStore((s) => s.setTopology);
  const setNarrative = useStore((s) => s.setNarrative);
  const setGaps = useStore((s) => s.setGaps);
  const setOperations = useStore((s) => s.setOperations);
  const setWsConnected = useStore((s) => s.setWsConnected);
  const setDaemonReachable = useStore((s) => s.setDaemonReachable);
  const handleBatch = useStore((s) => s.handleWsBatch);
  const handleInstanceBatch = useStore((s) => s.handleInstanceWsBatch);
  const updateInstanceState = useStore((s) => s.updateInstanceState);
  const wsConnected = useStore((s) => s.wsConnected);

  // Stable reference to instances for effects
  const instancesKey = instances.map((i) => `${i.id}:${i.httpBase}:${i.wsUrl}`).join("|") + `|active:${activeInstanceId}`;

  // ── WebSocket connections for all instances ──
  useEffect(() => {
    const cleanups: (() => void)[] = [];

    for (const inst of instances) {
      const cleanup = connectInstanceWS(
        inst.wsUrl,
        inst.id,
        inst.id === activeInstanceId,
        handleBatch,
        handleInstanceBatch,
        setWsConnected,
        updateInstanceState,
      );
      cleanups.push(cleanup);
    }

    return () => { cleanups.forEach((fn) => fn()); };
  }, [instancesKey]); // eslint-disable-line react-hooks/exhaustive-deps

  // ── Status poll for ALL instances (every 1s) ──
  useEffect(() => {
    let alive = true;
    const apis = instances.map((inst) => ({
      inst,
      api: createDaemonAPI(inst.httpBase),
    }));

    async function poll() {
      for (const { inst, api } of apis) {
        if (!alive) break;
        try {
          const s = await api.status();
          if (!alive) break;
          if (inst.id === activeInstanceId) {
            setStatus(s);
          }
          updateInstanceState(inst.id, {
            reachable: true,
            status: s,
            currentPositionLabel: s.position_label || "",
            currentPositionId: s.position || "",
          });
        } catch {
          updateInstanceState(inst.id, { reachable: false });
          if (inst.id === activeInstanceId) setDaemonReachable(false);
        }
      }
    }

    poll();
    const id = setInterval(poll, STATUS_POLL_MS);
    return () => { alive = false; clearInterval(id); };
  }, [instancesKey]); // eslint-disable-line react-hooks/exhaustive-deps

  // ── Active instance polls: topology (5s), narrative (3s fallback), gaps (10s), operations (5s) ──
  const activeInst = instances.find((i) => i.id === activeInstanceId);
  const activeHttpBase = activeInst?.httpBase ?? instances[0]?.httpBase ?? "http://localhost:9765";

  useEffect(() => {
    let alive = true;
    const api = createDaemonAPI(activeHttpBase);

    async function fetchTopo() {
      try {
        const t = await api.topology();
        if (alive) setTopology(t);
      } catch { /* ignore */ }
    }
    fetchTopo();
    const id = setInterval(fetchTopo, 5000);
    return () => { alive = false; clearInterval(id); };
  }, [activeHttpBase, setTopology]);

  useEffect(() => {
    let alive = true;
    const api = createDaemonAPI(activeHttpBase);

    async function fetchNarrative() {
      if (wsConnected) return; // WS provides narrative in real-time
      try {
        const events = await api.narrative(30);
        if (alive) setNarrative(events);
      } catch { /* ignore */ }
    }
    fetchNarrative();
    const id = setInterval(fetchNarrative, 3000);
    return () => { alive = false; clearInterval(id); };
  }, [activeHttpBase, wsConnected, setNarrative]);

  useEffect(() => {
    let alive = true;
    const api = createDaemonAPI(activeHttpBase);

    async function fetchGaps() {
      try {
        const g = await api.gaps();
        if (alive) setGaps(g);
      } catch { /* ignore */ }
    }
    fetchGaps();
    const id = setInterval(fetchGaps, 10000);
    return () => { alive = false; clearInterval(id); };
  }, [activeHttpBase, setGaps]);

  useEffect(() => {
    let alive = true;
    const api = createDaemonAPI(activeHttpBase);

    async function fetchOps() {
      try {
        const ops = await api.operations();
        if (alive) setOperations(ops);
      } catch { /* ignore */ }
    }
    fetchOps();
    const id = setInterval(fetchOps, 5000);
    return () => { alive = false; clearInterval(id); };
  }, [activeHttpBase, setOperations]);
}

/**
 * Build merged instanceTraversals from all connected instances.
 * Call this in App.tsx to get the combined traversal positions for TopologyView.
 */
export function useInstanceTraversals(): InstanceTraversal[] {
  const instances = useStore((s) => s.instances);
  const instanceStates = useStore((s) => s.instanceStates);
  const topology = useStore((s) => s.topology);
  const activeInstanceId = useStore((s) => s.activeInstanceId);
  const currentPositionLabel = useStore((s) => s.currentPositionLabel);
  const currentPositionId = useStore((s) => s.currentPositionId);
  const peersPositions = useStore((s) => s.peersPositions);

  return useMemo(() => {
    const result: InstanceTraversal[] = [];

    for (let idx = 0; idx < instances.length; idx++) {
      const inst = instances[idx];
      const state = instanceStates[inst.id];

      // Determine position: active instance uses store-level, others use instanceStates
      const posLabel = inst.id === activeInstanceId
        ? currentPositionLabel
        : (state?.currentPositionLabel || "");
      const posId = inst.id === activeInstanceId
        ? currentPositionId
        : (state?.currentPositionId || "");

      if (!posLabel && !posId) continue;

      // Find matching vertex in topology
      const vertexId = topology?.nodes.find(
        (n) => n.id === posId || n.label === posLabel
      )?.id;

      if (!vertexId) continue;

      result.push({
        instanceId: inst.id,
        instanceName: inst.name,
        position: vertexId,
        color: INSTANCE_COLORS[idx % INSTANCE_COLORS.length],
        history: [],
      });
    }

    // Also include peer positions from SharedLayer (cross-instance sync via WS)
    for (const [peerId, peer] of Object.entries(peersPositions)) {
      if (!peer.online || !peer.posLabel) continue;
      // Skip if already covered by an instance
      if (instances.some((i) => i.id === peerId)) continue;

      const vertexId = topology?.nodes.find(
        (n) => n.label === peer.posLabel
      )?.id;
      if (!vertexId) continue;

      result.push({
        instanceId: peerId,
        instanceName: peerId,
        position: vertexId,
        color: peer.color,
        history: [],
      });
    }

    return result;
  }, [instances, instanceStates, topology, activeInstanceId, currentPositionLabel, currentPositionId, peersPositions]);
}
