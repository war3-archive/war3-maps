import { availabilityName, fourCC, playerMaskLabel, stripColorCodes } from "../../lib/format";
import type { MapMetadata } from "../../lib/wasm";
import { Card } from "../ui/Card";

/** Total entry count used for the tab badge. */
export function techEntryCount(data: MapMetadata): number {
  const info = data.map_info;
  if (!info) return 0;
  return (
    (info.upgrade_availability_changes?.length ?? 0) +
    (info.tech_availability_changes?.length ?? 0) +
    (info.random_unit_tables?.length ?? 0) +
    (info.random_item_tables?.length ?? 0)
  );
}

export function TechPanel({ data }: { data: MapMetadata }) {
  const info = data.map_info;
  if (!info || !techEntryCount(data)) {
    return (
      <div className="empty">
        No upgrade/tech changes or random tables
        {info?.skipped_optional_sections
          ? " — this map skips its optional w3i sections (0xFF marker)"
          : ""}
      </div>
    );
  }

  const upgrades = info.upgrade_availability_changes ?? [];
  const techs = info.tech_availability_changes ?? [];
  const unitTables = info.random_unit_tables ?? [];
  const itemTables = info.random_item_tables ?? [];

  return (
    <div className="panel-stack">
      {upgrades.length ? (
        <Card title="Upgrade availability" badge={String(upgrades.length)}>
          <div className="table-wrap">
            <table className="data">
              <thead>
                <tr>
                  <th>Upgrade</th>
                  <th>Level</th>
                  <th>Availability</th>
                  <th>Players</th>
                </tr>
              </thead>
              <tbody>
                {upgrades.map((u, i) => (
                  <tr key={`${fourCC(u.id)}-${i}`}>
                    <td className="mono">{fourCC(u.id)}</td>
                    <td className="mono">{u.level_affected}</td>
                    <td>{availabilityName(u.availability)}</td>
                    <td>{playerMaskLabel(u.player_flags)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      ) : null}

      {techs.length ? (
        <Card title="Tech restrictions" badge={String(techs.length)}>
          <div className="table-wrap">
            <table className="data">
              <thead>
                <tr>
                  <th>Tech</th>
                  <th>Players</th>
                </tr>
              </thead>
              <tbody>
                {techs.map((t, i) => (
                  <tr key={`${fourCC(t.id)}-${i}`}>
                    <td className="mono">{fourCC(t.id)}</td>
                    <td>{playerMaskLabel(t.player_flags)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      ) : null}

      {unitTables.map((table) => (
        <Card
          key={`unit-${table.id}`}
          title={`Random units — ${stripColorCodes(table.name) || `Table ${table.id}`}`}
          badge={`${table.units?.length ?? 0} rows`}
        >
          <div className="table-wrap">
            <table className="data">
              <thead>
                <tr>
                  <th>Chance</th>
                  {(table.column_types ?? []).map((t, i) => (
                    <th key={i}>{t === 0 ? "Unit" : t === 1 ? "Building" : t === 2 ? "Item" : `Col ${i}`}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {(table.units ?? []).map((row, i) => (
                  <tr key={i}>
                    <td className="mono">{row.chance}%</td>
                    {(row.ids ?? []).map((id, j) => (
                      <td key={j} className="mono">
                        {fourCC(id)}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      ))}

      {itemTables.map((table) => (
        <Card
          key={`item-${table.id}`}
          title={`Random items — ${stripColorCodes(table.name) || `Table ${table.id}`}`}
          badge={`${table.sets?.length ?? 0} sets`}
        >
          <div className="table-wrap">
            <table className="data">
              <thead>
                <tr>
                  <th>Set</th>
                  <th>Items (chance)</th>
                </tr>
              </thead>
              <tbody>
                {(table.sets ?? []).map((set, i) => (
                  <tr key={i}>
                    <td className="mono">#{i + 1}</td>
                    <td className="mono">
                      {(set.items ?? [])
                        .map((it) => `${fourCC(it.id)} (${it.chance}%)`)
                        .join(", ")}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      ))}
    </div>
  );
}
