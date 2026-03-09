/**
 * TopologyFilterBar.tsx — Unified filter bar for topology views.
 *
 * Compact dark-theme filter strip that sits above the topology view.
 * Combines four filter dimensions:
 *   1. Domain filter (philosophy, code, chanlun, etc.)
 *   2. Activity/degree threshold slider
 *   3. Settlement toggle (all / settled / unsettled)
 *   4. Instance visibility toggles (per-instance color dots)
 *
 * Filter state is persisted in zustand store (localStorage).
 */

import { useMemo } from "react";
import { T, FONT, INSTANCE_COLORS } from "../tokens";
import type { DaemonInstance } from "../tokens";
import { DOMAIN_FILTERS, type DomainKey } from "../utils/classifyNode";
import { useStore } from "../hooks/useStore";
import type { InstanceState } from "../hooks/useStore";

interface Props {
  instances: DaemonInstance[];
  instanceStates: Record<string, InstanceState>;
  /** Max degree in current topology data (for slider range) */
  maxDegree: number;
}

export function TopologyFilterBar({ instances, instanceStates, maxDegree }: Props) {
  const filters = useStore((s) => s.filters);
  const toggleFilterDomain = useStore((s) => s.toggleFilterDomain);
  const setFilterMinDegree = useStore((s) => s.setFilterMinDegree);
  const setFilterSettlement = useStore((s) => s.setFilterSettlement);
  const toggleFilterInstance = useStore((s) => s.toggleFilterInstance);

  const domainAll = filters.enabledDomains.size === 0;
  const instanceAll = filters.visibleInstances.size === 0;

  const sliderMax = useMemo(() => Math.max(maxDegree, 1), [maxDegree]);

  return (
    <div style={{
      display: "flex",
      alignItems: "center",
      gap: 2,
      padding: "3px 8px",
      borderBottom: `1px solid ${T.border}`,
      background: T.bgPanel,
      flexShrink: 0,
      overflowX: "auto",
      flexWrap: "nowrap",
      fontFamily: FONT.mono,
      fontSize: 9,
    }}>
      {/* ── Domain filters ── */}
      <SectionLabel text="域" />
      <PillButton
        label="全部"
        color="#9ca3af"
        active={domainAll}
        onClick={() => {
          if (!domainAll) {
            // Clear all domain filters (show all)
            useStore.getState().setFilterDomains(new Set());
          }
        }}
      />
      {DOMAIN_FILTERS.map((meta) => {
        const active = domainAll || filters.enabledDomains.has(meta.key);
        return (
          <PillButton
            key={meta.key}
            label={meta.label}
            color={meta.color}
            active={active}
            dimmed={!domainAll && !active}
            onClick={() => {
              if (domainAll) {
                // First click from "all": enable only this domain
                const allExcept = new Set<string>(
                  DOMAIN_FILTERS.map((d) => d.key).filter((k) => k !== meta.key)
                );
                // We want "only this domain visible" = enabledDomains contains only this
                useStore.getState().setFilterDomains(new Set([meta.key]));
              } else {
                toggleFilterDomain(meta.key);
              }
            }}
          />
        );
      })}

      <Separator />

      {/* ── Activity (degree threshold) ── */}
      <SectionLabel text="活跃度" />
      <span style={{ color: T.textMuted, fontSize: 8, marginRight: 2 }}>
        deg≥{filters.minDegree}
      </span>
      <input
        type="range"
        min={0}
        max={sliderMax}
        value={filters.minDegree}
        onChange={(e) => setFilterMinDegree(Number(e.target.value))}
        title={`最小度数: ${filters.minDegree}`}
        style={{
          width: 60,
          height: 12,
          accentColor: T.accent,
          cursor: "pointer",
        }}
      />

      <Separator />

      {/* ── Settlement filter ── */}
      <SectionLabel text="settled" />
      {(["all", "settled", "unsettled"] as const).map((v) => (
        <PillButton
          key={v}
          label={v === "all" ? "全部" : v === "settled" ? "已结晶" : "未结晶"}
          color={v === "settled" ? T.settled : v === "unsettled" ? T.fMid : "#9ca3af"}
          active={filters.settlementFilter === v}
          onClick={() => setFilterSettlement(v)}
        />
      ))}

      {/* ── Instance filter (only if multiple instances) ── */}
      {instances.length > 1 && (
        <>
          <Separator />
          <SectionLabel text="实例" />
          {instances.map((inst, idx) => {
            const color = INSTANCE_COLORS[idx % INSTANCE_COLORS.length];
            const state = instanceStates[inst.id];
            const connected = state?.wsConnected || state?.reachable;
            const visible = instanceAll || filters.visibleInstances.has(inst.id);

            return (
              <PillButton
                key={inst.id}
                label={inst.name}
                color={color}
                active={visible}
                dimmed={!visible}
                onClick={() => {
                  if (instanceAll) {
                    // From "show all" → show only this instance
                    useStore.getState().setFilterVisibleInstances(new Set([inst.id]));
                  } else {
                    toggleFilterInstance(inst.id);
                  }
                }}
                suffix={!connected ? " ○" : undefined}
              />
            );
          })}
          <PillButton
            label="全部"
            color="#9ca3af"
            active={instanceAll}
            onClick={() => {
              useStore.getState().setFilterVisibleInstances(new Set());
            }}
          />
        </>
      )}
    </div>
  );
}

// ── Subcomponents ──────────────────────────────────────────────

function SectionLabel({ text }: { text: string }) {
  return (
    <span style={{
      color: T.textMuted,
      fontSize: 8,
      letterSpacing: "0.08em",
      textTransform: "uppercase",
      marginRight: 2,
      marginLeft: 4,
      flexShrink: 0,
    }}>
      {text}
    </span>
  );
}

function Separator() {
  return (
    <div style={{
      width: 1, height: 14,
      background: T.border,
      marginInline: 4,
      flexShrink: 0,
    }} />
  );
}

interface PillButtonProps {
  label: string;
  color: string;
  active: boolean;
  dimmed?: boolean;
  onClick: () => void;
  suffix?: string;
}

function PillButton({ label, color, active, dimmed = false, onClick, suffix }: PillButtonProps) {
  return (
    <button
      onClick={onClick}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 3,
        padding: "1px 5px",
        borderRadius: 3,
        border: active
          ? `1px solid ${color}66`
          : `1px solid ${T.border}`,
        background: active
          ? `${color}14`
          : "transparent",
        cursor: "pointer",
        fontFamily: FONT.mono,
        fontSize: 9,
        color: active ? color : dimmed ? T.textMuted : T.textDim,
        opacity: dimmed ? 0.4 : 1,
        transition: "all 0.1s",
        whiteSpace: "nowrap",
        flexShrink: 0,
      }}
    >
      <span style={{
        width: 5, height: 5,
        borderRadius: "50%",
        background: color,
        opacity: active ? 1 : 0.35,
        flexShrink: 0,
      }} />
      {label}{suffix ?? ""}
    </button>
  );
}
