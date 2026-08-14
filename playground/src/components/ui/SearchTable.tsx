import { useMemo, useState } from "react";
import { Card } from "./Card";

export function SearchTable({
  title,
  headers,
  rows,
  placeholder,
}: {
  title: string;
  headers: string[];
  rows: string[][];
  placeholder: string;
}) {
  const [q, setQ] = useState("");
  const filtered = useMemo(() => {
    const needle = q.trim().toLowerCase();
    if (!needle) return rows;
    return rows.filter((row) => row.some((c) => c.toLowerCase().includes(needle)));
  }, [q, rows]);

  const shown = filtered.slice(0, 500);

  return (
    <Card>
      <div className="toolbar">
        <h2>{title}</h2>
        <input
          className="search"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder={placeholder}
          aria-label={placeholder}
        />
      </div>
      <div className="table-wrap">
        <table className="data">
          <thead>
            <tr>
              {headers.map((h) => (
                <th key={h}>{h}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {shown.map((row, i) => (
              <tr key={`${row[0]}-${i}`}>
                {row.map((c, j) => (
                  <td key={j} className="mono">
                    {c}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="muted" style={{ marginTop: "0.5rem", fontSize: "0.82rem" }}>
        {filtered.length > 500
          ? `Showing 500 of ${filtered.length} matches`
          : `${filtered.length} row${filtered.length === 1 ? "" : "s"}`}
      </div>
    </Card>
  );
}
