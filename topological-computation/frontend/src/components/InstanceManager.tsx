/**
 * InstanceManager.tsx — UI for managing daemon instances.
 *
 * Shows instance list with connection status, add/remove buttons.
 * Users can select which instance to view topology/narrative from.
 * All instances are peers (no primary concept).
 * Compact dark theme, sits in a collapsible panel.
 */

import { useState, useCallback } from "react";
import { T, FONT, INSTANCE_COLORS } from "../tokens";
import type { DaemonInstance } from "../tokens";
import { useStore } from "../hooks/useStore";
import type { InstanceState } from "../hooks/useStore";

export function InstanceManager() {
  const instances = useStore((s) => s.instances);
  const instanceStates = useStore((s) => s.instanceStates);
  const activeInstanceId = useStore((s) => s.activeInstanceId);
  const addInstance = useStore((s) => s.addInstance);
  const removeInstance = useStore((s) => s.removeInstance);
  const setActiveInstanceId = useStore((s) => s.setActiveInstanceId);

  const [expanded, setExpanded] = useState(false);
  const [showAddForm, setShowAddForm] = useState(false);
  const [newName, setNewName] = useState("");
  const [newHttp, setNewHttp] = useState("");
  const [newWs, setNewWs] = useState("");

  const handleAdd = useCallback(() => {
    if (!newName.trim() || !newHttp.trim()) return;

    const id = newName.toLowerCase().replace(/\s+/g, "-") + "-" + Date.now().toString(36);
    const httpBase = newHttp.trim().replace(/\/+$/, "");
    const wsUrl = newWs.trim() || httpBase.replace(/^http/, "ws").replace(/:9765/, ":8765") + "/ws";

    addInstance({ id, name: newName.trim(), httpBase, wsUrl });

    setNewName("");
    setNewHttp("");
    setNewWs("");
    setShowAddForm(false);
  }, [newName, newHttp, newWs, addInstance]);

  return (
    <div style={{
      borderBottom: `1px solid ${T.border}`,
      fontFamily: FONT.mono,
      fontSize: 10,
    }}>
      {/* Header */}
      <div
        onClick={() => setExpanded(!expanded)}
        style={{
          display: "flex", alignItems: "center", gap: 8,
          padding: "4px 12px",
          cursor: "pointer",
          color: T.textDim,
          userSelect: "none",
        }}
      >
        <span style={{ fontSize: 8 }}>{expanded ? "▼" : "▶"}</span>
        <span style={{ letterSpacing: "0.1em" }}>INSTANCES</span>

        {/* Mini dots showing connection status */}
        <div style={{ display: "flex", gap: 3, marginLeft: 4 }}>
          {instances.map((inst, idx) => {
            const state = instanceStates[inst.id];
            const connected = state?.wsConnected || state?.reachable;
            return (
              <div
                key={inst.id}
                title={`${inst.name}: ${connected ? "连接" : "断开"}${inst.id === activeInstanceId ? " (查看中)" : ""}`}
                style={{
                  width: 5, height: 5, borderRadius: "50%",
                  background: connected
                    ? INSTANCE_COLORS[idx % INSTANCE_COLORS.length]
                    : T.textMuted,
                  opacity: connected ? 1 : 0.4,
                  transition: "background 0.3s, opacity 0.3s",
                  border: inst.id === activeInstanceId ? `1px solid ${T.text}` : "none",
                  boxSizing: "border-box",
                }}
              />
            );
          })}
        </div>

        <span style={{ color: T.textMuted, marginLeft: "auto" }}>
          {instances.length}
        </span>
      </div>

      {/* Expanded panel */}
      {expanded && (
        <div style={{ padding: "0 12px 8px" }}>
          {/* Instance list */}
          {instances.map((inst, idx) => {
            const state: InstanceState | undefined = instanceStates[inst.id];
            const connected = state?.wsConnected || state?.reachable;
            const color = INSTANCE_COLORS[idx % INSTANCE_COLORS.length];
            const status = state?.status;
            const isActive = inst.id === activeInstanceId;

            return (
              <div
                key={inst.id}
                style={{
                  display: "flex", alignItems: "center", gap: 6,
                  padding: "3px 0",
                  borderBottom: `1px solid ${T.border}22`,
                }}
              >
                {/* Color dot */}
                <div style={{
                  width: 6, height: 6, borderRadius: "50%",
                  background: connected ? color : T.textMuted,
                  boxShadow: connected ? `0 0 4px ${color}66` : "none",
                  flexShrink: 0,
                }} />

                {/* Name + active badge */}
                <span style={{
                  color: isActive ? T.accent : T.text,
                  fontWeight: isActive ? 600 : 400,
                  minWidth: 40,
                }}>
                  {inst.name}
                </span>
                {isActive && (
                  <span style={{
                    fontSize: 7, color: T.accent,
                    border: `1px solid ${T.accent}44`,
                    borderRadius: 2, padding: "0 3px",
                  }}>
                    查看
                  </span>
                )}

                {/* Position label */}
                {connected && state?.currentPositionLabel && (
                  <span style={{
                    color: color,
                    fontSize: 8,
                    maxWidth: 80,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                    opacity: 0.7,
                  }}
                    title={state.currentPositionLabel}
                  >
                    @{state.currentPositionLabel}
                  </span>
                )}

                {/* Status indicators */}
                <span style={{ color: T.textMuted, fontSize: 8 }}>
                  {connected ? (
                    <>
                      V{status?.vertices ?? "?"} E{status?.edges ?? "?"} s{status?.steps ?? "?"}
                    </>
                  ) : (
                    "断开"
                  )}
                </span>

                <div style={{ flex: 1 }} />

                {/* View source button */}
                {!isActive && (
                  <button
                    onClick={() => setActiveInstanceId(inst.id)}
                    title="查看此实例的拓扑数据"
                    style={{
                      background: "transparent",
                      border: `1px solid ${T.border}`,
                      borderRadius: 2,
                      color: T.textDim,
                      fontSize: 8,
                      padding: "1px 4px",
                      cursor: "pointer",
                    }}
                  >
                    查看
                  </button>
                )}

                {/* Remove button (can't remove last) */}
                {instances.length > 1 && (
                  <button
                    onClick={() => removeInstance(inst.id)}
                    title="删除实例"
                    style={{
                      background: "transparent",
                      border: `1px solid ${T.border}`,
                      borderRadius: 2,
                      color: T.fHigh,
                      fontSize: 8,
                      padding: "1px 4px",
                      cursor: "pointer",
                      opacity: 0.6,
                    }}
                  >
                    ×
                  </button>
                )}
              </div>
            );
          })}

          {/* Add button / form */}
          {showAddForm ? (
            <div style={{
              marginTop: 6, padding: 6,
              background: T.bgCard,
              border: `1px solid ${T.border}`,
              borderRadius: 4,
              display: "flex", flexDirection: "column", gap: 4,
            }}>
              <input
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                placeholder="名称 (如: VPS-2)"
                style={{
                  background: T.bg, color: T.text,
                  border: `1px solid ${T.border}`,
                  borderRadius: 2, padding: "2px 6px",
                  fontFamily: FONT.mono, fontSize: 10,
                  outline: "none",
                }}
              />
              <input
                value={newHttp}
                onChange={(e) => setNewHttp(e.target.value)}
                placeholder="HTTP (如: http://1.2.3.4:9765)"
                style={{
                  background: T.bg, color: T.text,
                  border: `1px solid ${T.border}`,
                  borderRadius: 2, padding: "2px 6px",
                  fontFamily: FONT.mono, fontSize: 10,
                  outline: "none",
                }}
              />
              <input
                value={newWs}
                onChange={(e) => setNewWs(e.target.value)}
                placeholder="WS (留空自动推导)"
                style={{
                  background: T.bg, color: T.text,
                  border: `1px solid ${T.border}`,
                  borderRadius: 2, padding: "2px 6px",
                  fontFamily: FONT.mono, fontSize: 10,
                  outline: "none",
                }}
              />
              <div style={{ display: "flex", gap: 4, justifyContent: "flex-end" }}>
                <button
                  onClick={() => setShowAddForm(false)}
                  style={{
                    background: "transparent", border: `1px solid ${T.border}`,
                    borderRadius: 2, color: T.textDim,
                    fontSize: 9, padding: "2px 8px", cursor: "pointer",
                  }}
                >
                  取消
                </button>
                <button
                  onClick={handleAdd}
                  disabled={!newName.trim() || !newHttp.trim()}
                  style={{
                    background: T.accent + "22", border: `1px solid ${T.accent}44`,
                    borderRadius: 2, color: T.accent,
                    fontSize: 9, padding: "2px 8px", cursor: "pointer",
                    opacity: (!newName.trim() || !newHttp.trim()) ? 0.4 : 1,
                  }}
                >
                  添加
                </button>
              </div>
            </div>
          ) : (
            <button
              onClick={() => setShowAddForm(true)}
              style={{
                marginTop: 4, width: "100%",
                background: "transparent",
                border: `1px dashed ${T.border}`,
                borderRadius: 3,
                color: T.textDim,
                fontSize: 9,
                padding: "3px 0",
                cursor: "pointer",
              }}
            >
              + 添加实例
            </button>
          )}
        </div>
      )}
    </div>
  );
}
