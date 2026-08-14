import {
  controllerLabel,
  playersInForce,
  raceIcon,
  raceName,
  slotColor,
  stripColorCodes,
} from "../../lib/format";
import type { MapMetadata } from "../../lib/wasm";

export function TeamsPanel({ data }: { data: MapMetadata }) {
  const info = data.map_info;
  if (!info) return <div className="empty">No map_info / players</div>;

  const players = [...(info.players ?? [])].sort((a, b) => a.id - b.id);
  const forces = info.forces ?? [];
  const assigned = new Set<number>();

  type P = (typeof players)[number];
  const teams: Array<{ title: string; members: P[]; subtle?: boolean }> = [];

  if (forces.length) {
    forces.forEach((f, i) => {
      const members = playersInForce(f.player_masks >>> 0, players);
      members.forEach((p) => assigned.add(p.id));
      teams.push({
        title: stripColorCodes(f.name) || `Force ${i + 1}`,
        members,
      });
    });
    const orphans = players.filter((p) => !assigned.has(p.id));
    if (orphans.length) teams.push({ title: "Unassigned", members: orphans, subtle: true });
  } else {
    teams.push({ title: "All players", members: players });
  }

  return (
    <div className="panel-stack">
      <div className="teams-board">
        {teams.map((team) => (
          <section
            key={team.title}
            className={team.subtle ? "team-block team-block-subtle" : "team-block"}
          >
            <header className="team-head">
              <h3>{team.title}</h3>
              <span className="team-count">{team.members.length}</span>
            </header>
            {!team.members.length ? (
              <div className="team-empty">No players in this force</div>
            ) : (
              <div className="team-list">
                {team.members.map((p) => {
                  const color = slotColor(p.id);
                  const name = stripColorCodes(p.name) || `Player ${p.id + 1}`;
                  return (
                    <div key={p.id} className="team-row">
                      <div className="team-name" style={{ color }}>
                        <span className="slot-dot" style={{ background: color }} />
                        <span className="team-name-text">{name}</span>
                        <span className="slot-id mono">#{p.id}</span>
                      </div>
                      <div className="team-race">
                        <span className="race-badge" title={raceName(p.race)}>
                          {raceIcon(p.race)}
                        </span>
                        <span>{raceName(p.race)}</span>
                      </div>
                      <div
                        className={`team-ctrl ${p.player_type === 1 ? "ctrl-user" : "ctrl-other"}`}
                      >
                        {controllerLabel(p.player_type)}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </section>
        ))}
      </div>

      <details className="team-details card">
        <summary>
          Slot details <span className="muted">start locations &amp; flags</span>
        </summary>
        <div className="table-wrap">
          <table className="data">
            <thead>
              <tr>
                <th>ID</th>
                <th>Name</th>
                <th>Type</th>
                <th>Race</th>
                <th>Fixed</th>
                <th>Start</th>
              </tr>
            </thead>
            <tbody>
              {players.map((p) => (
                <tr key={p.id}>
                  <td className="mono">{p.id}</td>
                  <td style={{ color: slotColor(p.id) }}>{stripColorCodes(p.name)}</td>
                  <td>{controllerLabel(p.player_type)}</td>
                  <td>{raceName(p.race)}</td>
                  <td>{p.is_fixed_start_position ? "yes" : "no"}</td>
                  <td className="mono">
                    {Number(p.start_location?.[0] ?? 0).toFixed(1)},{" "}
                    {Number(p.start_location?.[1] ?? 0).toFixed(1)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </details>
    </div>
  );
}
