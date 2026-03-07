import { T, FONT } from "../tokens";
import {
  AreaChart, Area, XAxis, YAxis, Tooltip, ResponsiveContainer,
} from "recharts";

interface Beta1Point {
  step: number;
  beta1: number;
}

interface Props {
  history: Beta1Point[];
}

export function Beta1Curve({ history }: Props) {
  return (
    <div style={{ height: 100, padding: "8px 0", flexShrink: 0 }}>
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={history} margin={{ top: 4, right: 8, bottom: 0, left: 8 }}>
          <defs>
            <linearGradient id="beta1Grad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={T.accent} stopOpacity={0.3} />
              <stop offset="100%" stopColor={T.accent} stopOpacity={0} />
            </linearGradient>
          </defs>
          <XAxis dataKey="step" hide />
          <YAxis hide domain={["dataMin - 10", "dataMax + 10"]} />
          <Tooltip
            contentStyle={{
              background: T.bgCard,
              border: `1px solid ${T.border}`,
              borderRadius: 4,
              fontFamily: FONT.mono,
              fontSize: 10,
              color: T.text,
            }}
            formatter={(value: number) => [value.toLocaleString(), "β₁"]}
          />
          <Area
            type="monotone"
            dataKey="beta1"
            stroke={T.accent}
            fill="url(#beta1Grad)"
            strokeWidth={1.5}
            dot={false}
            isAnimationActive={false}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
