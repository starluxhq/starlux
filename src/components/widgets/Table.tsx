import type { TableWidget } from "../../lib/markdown/widgets/shapes";

export default function Table({ data }: { data: TableWidget }) {
  // A header sitting left over a column of right-aligned figures reads as a
  // misalignment, so the column's contents decide both.
  const numeric = data.columns.map(
    (_, index) => data.rows.length > 0 && data.rows.every((row) => typeof row[index] === "number"),
  );

  return (
    <figure className="my-4 overflow-hidden rounded-lg border border-rule bg-dust/40">
      {data.title ? (
        <figcaption className="border-b border-rule px-3 py-2 font-mono text-[10px] tracking-wide text-muted uppercase">
          {data.title}
        </figcaption>
      ) : null}
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[12.5px]">
          <thead>
            <tr>
              {data.columns.map((column, index) => (
                <th
                  key={column}
                  scope="col"
                  className={`border-b border-rule px-3 py-2 font-medium text-muted ${
                    numeric[index] ? "text-right" : "text-left"
                  }`}
                >
                  {column}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {data.rows.map((row, rowIndex) => (
              <tr key={rowIndex} className="border-b border-rule/40 last:border-0">
                {row.map((cell, cellIndex) => (
                  <td
                    key={cellIndex}
                    className={
                      numeric[cellIndex]
                        ? "px-3 py-2 text-right font-mono text-ink/90 tabular-nums"
                        : "px-3 py-2 text-ink/90"
                    }
                  >
                    {cell}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </figure>
  );
}
