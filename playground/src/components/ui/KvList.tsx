export function KvList({ rows }: { rows: Array<[string, string]> }) {
  return (
    <dl className="kv">
      {rows.map(([k, v]) => (
        <div key={k} style={{ display: "contents" }}>
          <dt>{k}</dt>
          <dd className="mono">{v}</dd>
        </div>
      ))}
    </dl>
  );
}
