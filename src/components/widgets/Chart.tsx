import type { ChartWidget } from "../../lib/markdown/widgets/shapes";

/** Stellar spectral classes, reused so a provider dot and a series read as one
 *  family. Cycled when a chart carries more series than there are classes. */
const SPECTRUM = ["#a9c7ff", "#ffd9a0", "#ff6f5e", "#f4f6fb", "#ffab63"];

const WIDTH = 560;
const HEIGHT = 220;
const PAD = { top: 10, right: 10, bottom: 26, left: 46 };
const PLOT = {
  width: WIDTH - PAD.left - PAD.right,
  height: HEIGHT - PAD.top - PAD.bottom,
};

const compact = new Intl.NumberFormat(undefined, {
  notation: "compact",
  maximumFractionDigits: 1,
});

export default function Chart({ data }: { data: ChartWidget }) {
  const values = data.series.flatMap((entry) => entry.values);
  // Bars are read by area, so they have to start at zero or they overstate the
  // difference between them. A line is read by slope and only needs the range
  // its own values occupy.
  const zeroed = data.chart === "bar";
  const high = Math.max(...values);
  const low = Math.min(...values);
  const headroom = zeroed ? 0 : (high - low) * 0.1;
  const top = zeroed ? Math.max(high, 0) : high + headroom;
  const bottom = zeroed ? Math.min(low, 0) : low - headroom;
  // A flat series would otherwise divide by zero and collapse the plot.
  const span = top - bottom || Math.abs(top) || 1;

  const y = (value: number) => PAD.top + PLOT.height - ((value - bottom) / span) * PLOT.height;
  const ticks = [bottom, bottom + span / 2, top];
  // Bars grow from zero when it is on the axis, and from the floor otherwise,
  // so a series that never reaches zero still has bars rather than slivers.
  const baseline = bottom <= 0 && top >= 0 ? 0 : bottom;
  const columns = Math.max(data.x.length, 1);
  const step = PLOT.width / columns;

  return (
    <figure className="my-4 overflow-hidden rounded-lg border border-rule bg-dust/40">
      {data.title ? (
        <figcaption className="border-b border-rule px-3 py-2 font-mono text-[10px] tracking-wide text-muted uppercase">
          {data.title}
        </figcaption>
      ) : null}

      <div className="px-3 pt-3">
        <svg
          viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
          className="h-auto w-full"
          role="img"
          aria-label={data.title ?? `${data.chart} chart`}
        >
          {ticks.map((tick, index) => (
            <g key={index}>
              <line
                x1={PAD.left}
                x2={WIDTH - PAD.right}
                y1={y(tick)}
                y2={y(tick)}
                stroke="currentColor"
                className="text-rule"
                strokeWidth={1}
              />
              <text
                x={PAD.left - 8}
                y={y(tick) + 3.5}
                textAnchor="end"
                className="fill-faint font-mono text-[9px]"
              >
                {compact.format(tick)}
              </text>
            </g>
          ))}

          {data.chart === "bar"
            ? data.series.map((entry, seriesIndex) => {
                const width = (step * 0.68) / data.series.length;
                return entry.values
                  .slice(0, columns)
                  .map((value, index) => (
                    <rect
                      key={`${seriesIndex}-${index}`}
                      x={PAD.left + index * step + step * 0.16 + seriesIndex * width}
                      y={Math.min(y(value), y(baseline))}
                      width={Math.max(width - 1, 1)}
                      height={Math.max(Math.abs(y(value) - y(baseline)), 1)}
                      fill={SPECTRUM[seriesIndex % SPECTRUM.length]}
                      rx={1.5}
                    />
                  ));
              })
            : data.series.map((entry, seriesIndex) => (
                <polyline
                  key={seriesIndex}
                  fill="none"
                  stroke={SPECTRUM[seriesIndex % SPECTRUM.length]}
                  strokeWidth={1.75}
                  strokeLinejoin="round"
                  strokeLinecap="round"
                  points={entry.values
                    .slice(0, columns)
                    .map((value, index) => `${PAD.left + index * step + step / 2},${y(value)}`)
                    .join(" ")}
                />
              ))}

          {data.x.map((label, index) => (
            <text
              key={index}
              x={PAD.left + index * step + step / 2}
              y={HEIGHT - 8}
              textAnchor="middle"
              className="fill-faint font-mono text-[9px]"
            >
              {label}
            </text>
          ))}
        </svg>
      </div>

      {data.series.length > 1 ? (
        <div className="flex flex-wrap gap-x-4 gap-y-1 px-3 pt-2 pb-3">
          {data.series.map((entry, index) => (
            <span key={index} className="inline-flex items-center gap-1.5 text-[11px] text-muted">
              <span
                className="size-2 rounded-full"
                style={{ background: SPECTRUM[index % SPECTRUM.length] }}
              />
              {entry.label}
            </span>
          ))}
        </div>
      ) : (
        <div className="pb-3" />
      )}
    </figure>
  );
}
