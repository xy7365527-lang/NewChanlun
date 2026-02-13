import { useState, useCallback, type CSSProperties } from "react";
import { useReplay } from "../hooks/useReplay";

// ── styles ──

const barStyle: CSSProperties = {
  height: 48,
  background: "#1a1e2e",
  display: "flex",
  flexDirection: "row",
  alignItems: "center",
  gap: 8,
  padding: "0 16px",
  color: "#d1d4dc",
  fontSize: 13,
  fontFamily: "'Segoe UI', system-ui, sans-serif",
  borderTop: "1px solid #2a2e39",
  flexShrink: 0,
};

const btnBase: CSSProperties = {
  padding: "4px 12px",
  borderRadius: 4,
  cursor: "pointer",
  border: "1px solid #333",
  background: "transparent",
  color: "#d1d4dc",
  fontSize: 13,
  lineHeight: 1.4,
};

const btnHighlight: CSSProperties = {
  ...btnBase,
  background: "#2962ff",
  borderColor: "#2962ff",
  color: "#fff",
};

const speedBtnStyle = (active: boolean): CSSProperties => ({
  ...btnBase,
  padding: "2px 8px",
  fontSize: 12,
  background: active ? "#2962ff" : "transparent",
  borderColor: active ? "#2962ff" : "#333",
  color: active ? "#fff" : "#888",
});

const sliderStyle: CSSProperties = {
  flex: 1,
  minWidth: 80,
  maxWidth: 300,
  height: 4,
  appearance: "auto",
  accentColor: "#2962ff",
  cursor: "pointer",
};

const statusTextStyle: CSSProperties = {
  color: "#888",
  fontSize: 12,
  whiteSpace: "nowrap",
  marginLeft: "auto",
};

// ── component ──

const SPEED_OPTIONS = [0.5, 1, 2, 5];

export function ReplayBar() {
  const { startReplay, step, seek, play, pause, stop, isReplaying, status } =
    useReplay();
  const [selectedSpeed, setSelectedSpeed] = useState(1);

  const handlePlayPause = useCallback(async () => {
    if (status.mode === "playing") {
      await pause();
    } else {
      await play(selectedSpeed);
    }
  }, [status.mode, pause, play, selectedSpeed]);

  const handleSpeedChange = useCallback(
    async (speed: number) => {
      setSelectedSpeed(speed);
      if (status.mode === "playing") {
        await play(speed);
      }
    },
    [status.mode, play],
  );

  const handleSeek = useCallback(
    async (e: React.ChangeEvent<HTMLInputElement>) => {
      const barIdx = parseInt(e.target.value, 10);
      await seek(barIdx);
    },
    [seek],
  );

  const modeLabel: Record<string, string> = {
    idle: "空闲",
    playing: "播放中",
    paused: "已暂停",
    done: "已完成",
  };

  return (
    <div style={barStyle}>
      {/* 开始 / 停止 */}
      {!isReplaying ? (
        <button style={btnHighlight} onClick={startReplay} title="开始回放">
          ▶ 回放
        </button>
      ) : (
        <>
          {/* Step */}
          <button
            style={btnBase}
            onClick={() => step(1)}
            disabled={status.mode === "playing"}
            title="单步前进"
          >
            ⏭ Step
          </button>

          {/* Play / Pause */}
          <button style={btnHighlight} onClick={handlePlayPause}>
            {status.mode === "playing" ? "⏸ 暂停" : "▶ 播放"}
          </button>

          {/* Speed */}
          <div style={{ display: "flex", gap: 2, alignItems: "center" }}>
            {SPEED_OPTIONS.map((sp) => (
              <button
                key={sp}
                style={speedBtnStyle(selectedSpeed === sp)}
                onClick={() => handleSpeedChange(sp)}
              >
                {sp}x
              </button>
            ))}
          </div>

          {/* Progress slider */}
          <input
            type="range"
            min={0}
            max={status.totalBars || 1}
            value={status.currentIdx}
            onChange={handleSeek}
            style={sliderStyle}
            title={`Bar ${status.currentIdx} / ${status.totalBars}`}
          />

          <span style={{ fontSize: 12, color: "#aaa", whiteSpace: "nowrap" }}>
            {status.currentIdx} / {status.totalBars}
          </span>

          {/* Stop */}
          <button
            style={{ ...btnBase, color: "#ef5350", borderColor: "#ef5350" }}
            onClick={stop}
            title="停止回放"
          >
            ■ 停止
          </button>
        </>
      )}

      {/* Status */}
      <span style={statusTextStyle}>
        {isReplaying ? modeLabel[status.mode] ?? status.mode : "就绪"}
      </span>
    </div>
  );
}
