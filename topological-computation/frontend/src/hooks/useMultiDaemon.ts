/**
 * useMultiDaemon.ts — manages N daemon instance connections.
 *
 * All instances are peers traversing the same shared K_active.
 * There is no "active" or "primary" instance — topology/narrative/gaps/operations
 * come from the shared K_active (fetched from any reachable instance).
 *
 * Each instance:
 *   - Connects WS — all WS messages feed into the shared store
 *   - Polls /status every 1s — updates per-instance state
 *   - Its traversal position is a colored marker on the shared topology
 *
 * Topology/narrative/gaps/operations are fetched from the first reachable instance
 * (they all see the same K_active via IPFS sync).
 */

import { useEffect, useMemo } from "react";
import { STATUS_POLL_MS, INSTANCE_COLORS } from "../tokens";
import type { WsMessage } from "../types";
import { createDaemonAPI } from "./useDaemonAPI";
import { useStore } from "./useStore";
import type { InstanceTraversal } from "../components/TopologyView";

/**
 * Manages a single WS connection imperatively. Returns cleanup function.
 * All instances feed into handleBatch (no distinction between active/non-active).
 */
function connectInstanceWS(
  wsUrl: string,
  instanceId: string,
  handleBatch: (instanceId: string, msgs: WsMessage[]) => void,
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
    handleBatch(instanceId, batch);
  }

  function connect() {
    if (destroyed) return;
    ws = new WebSocket(wsUrl);

    ws.onopen = () => {
      setWsConnected(true);
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
  const setStatus = useStore((s) => s.setStatus);
  const setTopology = useStore((s) => s.setTopology);
  const setNarrative = useStore((s) => s.setNarrative);
  const setGaps = useStore((s) => s.setGaps);
  const setOperations = useStore((s) => s.setOperations);
  const setWsConnected = useStore((s) => s.setWsConnected);
  const setDaemonReachable = useStore((s) => s.setDaemonReachable);
  const handleBatch = useStore((s) => s.handleWsBatch);
  const updateInstanceState = useStore((s) => s.updateInstanceState);

  // Stable reference to instances for effects
  const instancesKey = instances.map((i) => `${i.id}:${i.httpBase}:${i.wsUrl}`).join("|");

  // ── WebSocket connections for all instances ──
  // All WS connections feed into the same handleBatch
  useEffect(() => {
    const cleanups: (() => void)[] = [];

    for (const inst of instances) {
      const cleanup = connectInstanceWS(
        inst.wsUrl,
        inst.id,
        handleBatch,
        setWsConnected,
        updateInstanceState,
      );
      cleanups.push(cleanup);
    }

    return () => { cleanups.forEach((fn) => fn()); };
  }, [instancesKey]); // eslint-disable-line react-hooks/exhaustive-deps

  // ── Status poll for ALL instances (every 1s) ──
  // First reachable instance's status goes to store-level (shared K_active)
  useEffect(() => {
    let alive = true;
    const apis = instances.map((inst) => ({
      inst,
      api: createDaemonAPI(inst.httpBase),
    }));

    async function poll() {
      let sharedStatusSet = false;
      for (const { inst, api } of apis) {
        if (!alive) break;
        try {
          const s = await api.status();
          if (!alive) break;
          // First reachable instance provides the shared status
          if (!sharedStatusSet) {
            setStatus(s);
            setDaemonReachable(true);
            sharedStatusSet = true;
          }
          updateInstanceState(inst.id, {
            reachable: true,
            status: s,
            currentPositionLabel: s.position_label || "",
            currentPositionId: s.position || "",
          });
        } catch {
          updateInstanceState(inst.id, { reachable: false });
        }
      }
      if (!sharedStatusSet) {
        setDaemonReachable(false);
      }
    }

    poll();
    const id = setInterval(poll, STATUS_POLL_MS);
    return () => { alive = false; clearInterval(id); };
  }, [instancesKey]); // eslint-disable-line react-hooks/exhaustive-deps

  // ── Shared K_active polls: topology (5s), narrative (3s fallback), gaps (10s), operations (5s) ──
  // Fetch from any reachable instance (they share the same K_active via IPFS)
  const reachableHttpBase = useStore((s) => s.getReachableHttpBase());
  const wsConnected = useStore((s) => s.wsConnected);

  useEffect(() => {
    let alive = true;
    const api = createDaemonAPI(reachableHttpBase);

    async function fetchTopo() {
      try {
        const t = await api.topology();
        if (!alive) return;

        // Inject traversal position nodes if missing from topology skeleton.
        // The topology cache refreshes every N steps, but status updates every step,
        // so the current traversal position may have moved beyond the cached skeleton.
        const state = useStore.getState();
        const nodeIdSet = new Set(t.nodes.map((n) => n.id));
        const nodeLabelSet = new Set(t.nodes.map((n) => n.label));

        for (const inst of state.instances) {
          const instState = state.instanceStates[inst.id];
          if (!instState) continue;
          const posId = instState.currentPositionId;
          const posLabel = instState.currentPositionLabel;
          if (!posId && !posLabel) continue;
          // Already present by id or label
          if ((posId && nodeIdSet.has(posId)) || (posLabel && nodeLabelSet.has(posLabel))) continue;
          // Synthesize a minimal node for the traversal position
          const syntheticId = posId || `traversal-${inst.id}`;
          t.nodes.push({
            id: syntheticId,
            label: posLabel || posId || "?",
            f_avg: -1,
            degree: 1,
            settled: false,
            type: "text",
          });
          nodeIdSet.add(syntheticId);
        }

        setTopology(t);
      } catch { /* ignore */ }
    }
    fetchTopo();
    const id = setInterval(fetchTopo, 5000);
    return () => { alive = false; clearInterval(id); };
  }, [reachableHttpBase, setTopology]);

  useEffect(() => {
    let alive = true;
    const api = createDaemonAPI(reachableHttpBase);

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
  }, [reachableHttpBase, wsConnected, setNarrative]);

  useEffect(() => {
    let alive = true;
    const api = createDaemonAPI(reachableHttpBase);

    async function fetchGaps() {
      try {
        const g = await api.gaps();
        if (alive) setGaps(g);
      } catch { /* ignore */ }
    }
    fetchGaps();
    const id = setInterval(fetchGaps, 10000);
    return () => { alive = false; clearInterval(id); };
  }, [reachableHttpBase, setGaps]);

  useEffect(() => {
    let alive = true;
    const api = createDaemonAPI(reachableHttpBase);

    async function fetchOps() {
      try {
        const ops = await api.operations();
        if (alive) setOperations(ops);
      } catch { /* ignore */ }
    }
    fetchOps();
    const id = setInterval(fetchOps, 5000);
    return () => { alive = false; clearInterval(id); };
  }, [reachableHttpBase, setOperations]);
}

/**
 * Build merged instanceTraversals from all connected instances.
 * Each instance's current position is a colored marker on the shared topology.
 */
export function useInstanceTraversals(): InstanceTraversal[] {
  const instances = useStore((s) => s.instances);
  const instanceStates = useStore((s) => s.instanceStates);
  const topology = useStore((s) => s.topology);
  const peersPositions = useStore((s) => s.peersPositions);

  return useMemo(() => {
    const result: InstanceTraversal[] = [];

    for (let idx = 0; idx < instances.length; idx++) {
      const inst = instances[idx];
      const state = instanceStates[inst.id];

      const posLabel = state?.currentPositionLabel || "";
      const posId = state?.currentPositionId || "";

      if (!posLabel && !posId) continue;

      // Find matching vertex in shared topology by id or label
      let vertexId = topology?.nodes.find(
        (n) => n.id === posId || n.label === posLabel
      )?.id;

      // Fallback: use the position id directly if it exists in topology node ids
      // (handles case where label matching fails due to truncation)
      if (!vertexId && posId && topology?.nodes.some((n) => n.id === posId)) {
        vertexId = posId;
      }

      // Fallback: use meta.traversal_position if available and matches
      if (!vertexId && topology?.meta?.traversal_position) {
        const metaPos = topology.meta.traversal_position;
        if (metaPos === posId) {
          // The backend guarantees this vertex is in the skeleton
          vertexId = topology.nodes.find((n) => n.id === metaPos)?.id;
        }
      }

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
  }, [instances, instanceStates, topology, peersPositions]);
}
