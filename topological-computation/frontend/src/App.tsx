import { useEffect, useState, useCallback, useRef } from "react";
import { T, FONT, STATUS_POLL_MS } from "./tokens";
import { useStore } from "./hooks/useStore";
import { useDaemonWS } from "./hooks/useDaemonWS";
import { daemonAPI } from "./hooks/useDaemonAPI";
import type { TopologyNode, ChatMessage } from "./types";

import { MetricsBar } from "./components/MetricsBar";
import { Beta1Curve } from "./components/Beta1Curve";
import { OperationsSummary } from "./components/OperationsSummary";
import { NarrativeStream } from "./components/NarrativeStream";
import { GapQueue } from "./components/GapQueue";
import { ChatInput } from "./components/ChatInput";
import { TabSwitcher } from "./components/TabSwitcher";
import { CodePanel } from "./components/CodePanel";
import { QueryDetail } from "./components/QueryDetail";
import { TopologyViewSwitcher } from "./views/TopologyViewSwitcher";

const TABS = [
  { id: "chat", label: "对话" },
  { id: "code", label: "代码" },
];

export default function App() {
  // ── Store ────────────────────────────────────────────────
  const status = useStore((s) => s.status);
  const wsConnected = useStore((s) => s.wsConnected);
  const beta1History = useStore((s) => s.beta1History);
  const topology = useStore((s) => s.topology);
  const narrative = useStore((s) => s.narrative);
  const gaps = useStore((s) => s.gaps);
  const operations = useStore((s) => s.operations);
  const messages = useStore((s) => s.messages);
  const queryResult = useStore((s) => s.queryResult);
  const focusConcept = useStore((s) => s.focusConcept);
  const currentPositionLabel = useStore((s) => s.currentPositionLabel);
  const expressionPressure = useStore((s) => s.expressionPressure);

  const setStatus = useStore((s) => s.setStatus);
  const setTopology = useStore((s) => s.setTopology);
  const setNarrative = useStore((s) => s.setNarrative);
  const setGaps = useStore((s) => s.setGaps);
  const setOperations = useStore((s) => s.setOperations);
  const setFocusConcept = useStore((s) => s.setFocusConcept);
  const setQueryResult = useStore((s) => s.setQueryResult);
  const addMessage = useStore((s) => s.addMessage);

  // ── Connect WS ───────────────────────────────────────────────
  useDaemonWS();

  // ── Local UI state ───────────────────────────────────────────
  const [activeTab, setActiveTab] = useState("chat");
  const [pending, setPending] = useState(false);
  const chatBottomRef = useRef<HTMLDivElement>(null);

  // ── Status poll ──────────────────────────────────────────────
  useEffect(() => {
    let alive = true;
    async function poll() {
      try {
        const s = await daemonAPI.status();
        if (alive) setStatus(s);
      } catch {
        // daemon not reachable — skip silently
      }
    }
    poll();
    const id = setInterval(poll, STATUS_POLL_MS);
    return () => { alive = false; clearInterval(id); };
  }, [setStatus]);

  // ── Topology poll (every 5s) ──────────────────────────────────
  useEffect(() => {
    let alive = true;
    async function fetchTopo() {
      try {
        const t = await daemonAPI.topology();
        if (alive) setTopology(t);
      } catch {
        // ignore
      }
    }
    fetchTopo();
    const id = setInterval(fetchTopo, 5000);
    return () => { alive = false; clearInterval(id); };
  }, [setTopology]);

  // ── Narrative poll (every 3s, only if WS unavailable) ────────
  useEffect(() => {
    let alive = true;
    async function fetchNarrative() {
      if (wsConnected) return; // WS keeps narrative fresh
      try {
        const events = await daemonAPI.narrative(30);
        if (alive) setNarrative(events);
      } catch {
        // ignore
      }
    }
    fetchNarrative();
    const id = setInterval(fetchNarrative, 3000);
    return () => { alive = false; clearInterval(id); };
  }, [wsConnected, setNarrative]);

  // ── Gaps poll (every 10s) ─────────────────────────────────────
  useEffect(() => {
    let alive = true;
    async function fetchGaps() {
      try {
        const g = await daemonAPI.gaps();
        if (alive) setGaps(g);
      } catch {
        // ignore
      }
    }
    fetchGaps();
    const id = setInterval(fetchGaps, 10000);
    return () => { alive = false; clearInterval(id); };
  }, [setGaps]);

  // ── Operations poll (every 5s) ────────────────────────────────
  useEffect(() => {
    let alive = true;
    async function fetchOps() {
      try {
        const ops = await daemonAPI.operations();
        if (alive) setOperations(ops);
      } catch {
        // ignore
      }
    }
    fetchOps();
    const id = setInterval(fetchOps, 5000);
    return () => { alive = false; clearInterval(id); };
  }, [setOperations]);

  // ── Auto-scroll chat ─────────────────────────────────────────
  useEffect(() => {
    chatBottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  // ── Node click → still use /query for topology node detail ────
  const handleSelectNode = useCallback(
    async (node: TopologyNode) => {
      setFocusConcept(node.label);
      setActiveTab("chat");
      setQueryResult(null);
      try {
        const result = await daemonAPI.query(node.label);
        setQueryResult(result);
      } catch (e) {
        addMessage({
          role: "daemon",
          text: `查询 "${node.label}" 失败: ${e instanceof Error ? e.message : String(e)}`,
          timestamp: Date.now(),
        });
      }
    },
    [setFocusConcept, setQueryResult, addMessage]
  );

  // ── Concept selected from terrain/3D view ───────────────────
  const handleSelectConcept = useCallback(
    (concept: string) => {
      setFocusConcept(concept);
      setActiveTab("chat");
    },
    [setFocusConcept]
  );

  // ── /present — all user input funnels through here ───────────
  const handleSend = useCallback(
    async (text: string) => {
      const userMsg: ChatMessage = { role: "user", text, timestamp: Date.now() };
      addMessage(userMsg);
      setActiveTab("chat");
      setPending(true);

      try {
        const res = await daemonAPI.present(text);

        if (res.type === "silence") {
          // Silence means the system has nothing specific to share.
          // But since the user is talking, route through language organ fallback
          // instead of showing a template status message.
          addMessage({
            role: "daemon",
            text: res.expression_pressure > 0
              ? `[${res.expression_pressure} 个未报告事件积压中]`
              : "",  // true silence: no message shown
            timestamp: Date.now(),
          });
        } else {
          // dialogue, sharing, co-gaze — render each part
          for (const part of res.parts) {
            addMessage({
              role: "daemon",
              text: part.text,
              timestamp: Date.now(),
            });
          }
        }

        // If concepts were located, also update the focus concept for the topology view
        if (res.concepts_found && res.concepts_found.length > 0) {
          setFocusConcept(res.concepts_found[0]);
        }
      } catch (e) {
        addMessage({
          role: "daemon",
          text: `错误: ${e instanceof Error ? e.message : String(e)}`,
          timestamp: Date.now(),
        });
      } finally {
        setPending(false);
      }
    },
    [addMessage, setFocusConcept]
  );

  // ── Expression pressure click — trigger sharing with empty text ──
  const handlePressureClick = useCallback(async () => {
    if (pending) return;
    setPending(true);
    try {
      const res = await daemonAPI.present("");
      if (res.type === "silence") {
        addMessage({
          role: "daemon",
          text: "系统在稳态中，暂无积压事件。",
          timestamp: Date.now(),
        });
      } else {
        for (const part of res.parts) {
          addMessage({
            role: "daemon",
            text: part.text,
            timestamp: Date.now(),
          });
        }
      }
      setActiveTab("chat");
    } catch (e) {
      addMessage({
        role: "daemon",
        text: `错误: ${e instanceof Error ? e.message : String(e)}`,
        timestamp: Date.now(),
      });
    } finally {
      setPending(false);
    }
  }, [pending, addMessage]);

  // ── Current traversal vertex id from topology ────────────────
  const traversalVertexId = topology?.nodes.find(
    (n) => n.label === currentPositionLabel
  )?.id;

  // ── Render ───────────────────────────────────────────────────
  return (
    <div style={{
      width: "100vw", height: "100vh",
      display: "flex", flexDirection: "column",
      background: T.bg, color: T.text, overflow: "hidden",
    }}>
      {/* Top bar */}
      <MetricsBar
        status={status}
        wsConnected={wsConnected}
        currentPositionLabel={currentPositionLabel}
        expressionPressure={expressionPressure}
        onPressureClick={handlePressureClick}
      />

      {/* Main content */}
      <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>

        {/* Left: topology view switcher + β₁ curve + ops */}
        <div style={{
          flex: 1, display: "flex", flexDirection: "column",
          borderRight: `1px solid ${T.border}`,
          overflow: "hidden",
        }}>
          <div style={{ flex: 1, overflow: "hidden" }}>
            <TopologyViewSwitcher
              data={topology}
              traversalPosition={traversalVertexId}
              focusConcept={focusConcept}
              onSelectNode={handleSelectNode}
              onSelectConcept={handleSelectConcept}
              status={status}
              beta1History={beta1History}
              narrative={narrative}
            />
          </div>
          <Beta1Curve history={beta1History} />
          <OperationsSummary operations={operations} />
        </div>

        {/* Right panel */}
        <div style={{
          width: 440, display: "flex", flexDirection: "column",
          background: T.bgPanel, overflow: "hidden",
        }}>
          <TabSwitcher tabs={TABS} active={activeTab} onChange={setActiveTab} />

          {activeTab === "chat" ? (
            <>
              <div style={{ flex: 1, overflow: "auto", display: "flex", flexDirection: "column" }}>
                {messages.length === 0 ? (
                  /* No messages yet — show narrative stream */
                  <NarrativeStream events={narrative} />
                ) : (
                  <div style={{
                    fontFamily: FONT.mono, fontSize: 11, lineHeight: 1.7,
                    padding: "8px 12px", display: "flex", flexDirection: "column", gap: 0,
                  }}>
                    {/* Pinned narrative (top 3) */}
                    <div style={{
                      color: T.textMuted, fontSize: 9, marginBottom: 8,
                      letterSpacing: "0.1em",
                    }}>
                      psi_L NARRATIVE
                    </div>
                    {narrative.slice(0, 3).map((ev, i) => (
                      <div key={`n-${i}`} style={{
                        padding: "3px 0", opacity: 0.5,
                        color: T.textDim, fontSize: 10,
                      }}>
                        {ev.text}
                      </div>
                    ))}

                    {/* Query detail (if focus concept) */}
                    {queryResult && queryResult.found && (
                      <div style={{
                        margin: "8px 0",
                        background: T.bgCard,
                        border: `1px solid ${T.borderActive}`,
                        borderRadius: 6,
                      }}>
                        <QueryDetail
                          result={queryResult}
                          onSelectNeighbor={(concept) => {
                            handleSend(concept);
                          }}
                        />
                      </div>
                    )}

                    <div style={{ height: 1, background: T.border, margin: "8px 0" }} />

                    {/* Chat messages */}
                    {messages.map((msg, i) => (
                      <div key={i} style={{
                        padding: "8px 0",
                        borderBottom: `1px solid ${T.border}22`,
                      }}>
                        <span style={{
                          color: msg.role === "user" ? T.accent : T.fLow,
                          marginRight: 8, fontSize: 9,
                          letterSpacing: "0.1em",
                        }}>
                          {msg.role === "user" ? "YOU" : "逢亮"}
                        </span>
                        <div style={{
                          color: msg.role === "user" ? T.text : T.textDim,
                          marginTop: 4, wordBreak: "break-word",
                        }}>
                          {msg.text}
                        </div>
                      </div>
                    ))}

                    {pending && (
                      <div style={{ color: T.textMuted, fontSize: 10, padding: "8px 0" }}>
                        逢亮 处理中...
                      </div>
                    )}

                    <div ref={chatBottomRef} />
                  </div>
                )}
              </div>

              <GapQueue gaps={gaps} />
              <ChatInput onSend={handleSend} disabled={pending} />
            </>
          ) : (
            <CodePanel />
          )}
        </div>
      </div>
    </div>
  );
}
