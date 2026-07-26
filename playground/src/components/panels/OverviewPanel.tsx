import {
  bgraToCss,
  decodeFlags,
  formatBuild,
  formatBytes,
  gameDataSetName,
  gameDataVersionName,
  graphicsModeName,
  scriptModeName,
  stripColorCodes,
  tilesetName,
  w3iEraName,
  weatherName,
} from "../../lib/format";
import { buildOverlayIcons, countByType, isMinimapImage } from "../../lib/minimap";
import type { MapMetadata } from "../../lib/wasm";
import { Card } from "../ui/Card";
import { Chip } from "../ui/Chip";
import { KvList } from "../ui/KvList";
import { MinimapCanvas } from "../ui/MinimapCanvas";

export function OverviewPanel({
  data,
  file,
}: {
  data: MapMetadata;
  file: { name: string; size: number };
}) {
  const info = data.map_info;
  const name = stripColorCodes(info?.name) || stripColorCodes(data.header.name) || file.name;
  const icons = buildOverlayIcons(data);
  const counts = countByType(icons);
  const minimap = (data.images ?? []).find((img) => isMinimapImage(img.filename));
  const flags = decodeFlags(info?.flags ?? undefined);

  return (
    <div className="panel-stack">
      <section className="hero">
        <div>
          <h2 className="hero-name">{name}</h2>
          <div className="chip-row">
            <Chip tone="brass">
              w3i v{info?.version ?? "?"}
              {info ? ` · ${w3iEraName(info.version)}` : ""}
            </Chip>
            <Chip>{formatBytes(file.size)}</Chip>
            <Chip>{Math.round(data.parse_ms ?? 0)} ms</Chip>
            <Chip tone={data.header.has_hm3w ? "ok" : "warn"}>
              {data.header.has_hm3w ? "HM3W header" : "No HM3W (pure MPQ)"}
            </Chip>
            {info?.skipped_optional_sections ? (
              <Chip tone="warn">Optional sections skipped (0xFF)</Chip>
            ) : null}
          </div>
          {info?.description ? (
            <p className="hero-desc">{stripColorCodes(info.description)}</p>
          ) : null}
        </div>
        {minimap ? (
          <div className="cover-minimap">
            <div className="cover-frame">
              <MinimapCanvas
                imageUrl={minimap.data_url}
                icons={icons}
                size={280}
                label="Minimap with player starts and resources"
              />
            </div>
            <div className="cover-legend">
              {Object.entries(counts).map(([label, n]) => (
                <span key={label} className="legend-item">
                  <span
                    className={`legend-swatch legend-${label.toLowerCase().replace(/\s+/g, "-")}`}
                  />
                  {label} · {n}
                </span>
              ))}
            </div>
          </div>
        ) : null}
      </section>

      <div className="grid-2">
        <Card title="Map info">
          <KvList
            rows={[
              ["Author", stripColorCodes(info?.author) || "—"],
              ["Recommended", stripColorCodes(info?.recommended_players) || "—"],
              ["Tileset", tilesetName(info?.tileset)],
              [
                "Playable size",
                info?.playable_size
                  ? `${info.playable_size[0]} × ${info.playable_size[1]}`
                  : "—",
              ],
              ["Saves", String(info?.saves ?? "—")],
              ["Editor ver", String(info?.editor_version ?? "—")],
              ["Build", formatBuild(info?.build_version)],
              ["Script", scriptModeName(info?.script_mode)],
              ["Graphics", graphicsModeName(info?.graphics_mode)],
              ["Game data", gameDataVersionName(info?.game_data_version)],
              ...(info?.default_camera_zoom != null
                ? ([
                    [
                      "Camera zoom",
                      `${info.min_camera_zoom ?? "—"} / ${info.default_camera_zoom} / ${info.max_camera_zoom ?? "—"} (min/default/max)`,
                    ],
                  ] as Array<[string, string]>)
                : []),
            ]}
          />
        </Card>
        <Card title="Container">
          <KvList
            rows={[
              ["File", file.name],
              ["HM3W name", stripColorCodes(data.header.name) || "—"],
              ["Max players", String(data.header.max_players ?? "—")],
              [
                "Header flags",
                data.header.flags != null ? `0x${data.header.flags.toString(16)}` : "—",
              ],
              ["Players", String(info?.players?.length ?? 0)],
              ["Forces", String(info?.forces?.length ?? 0)],
              ["Minimap icons", String(icons.length)],
              ["Gold mines", String(counts["Gold mine"] ?? 0)],
              ["Buildings", String(counts.Building ?? 0)],
              ["Start locs", String(counts["Player start"] ?? 0)],
              ["Strings", String(data.strings?.length ?? 0)],
              ["Imports", String(data.imports?.length ?? 0)],
              ["Listfile", data.files ? `${data.files.length} files` : "missing"],
            ]}
          />
        </Card>
      </div>

      <Card
        title="Map flags"
        badge={info?.flags != null ? `0x${(info.flags >>> 0).toString(16)}` : undefined}
      >
        {flags.length ? (
          <div className="chip-row">
            {flags.map((f) => (
              <Chip key={f}>{f}</Chip>
            ))}
          </div>
        ) : (
          <div className="empty">No flags decoded</div>
        )}
      </Card>

      {info?.fog_style != null ? (
        <Card title="Environment">
          <div className="grid-2">
            <KvList
              rows={[
                ["Fog style", String(info.fog_style)],
                ["Fog density", info.fog_density != null ? info.fog_density.toFixed(3) : "—"],
                [
                  "Fog height",
                  info.fog_height ? `${info.fog_height[0]} → ${info.fog_height[1]}` : "—",
                ],
                ["Weather", weatherName(info.global_weather)],
              ]}
            />
            <KvList
              rows={[
                ["Sound env", info.sound_environment || "—"],
                [
                  "Light env",
                  info.light_environment_tileset
                    ? tilesetName(info.light_environment_tileset)
                    : "—",
                ],
              ]}
            />
          </div>
          <div className="chip-row" style={{ marginTop: "0.5rem" }}>
            {bgraToCss(info.fog_color) ? (
              <span className="legend-item">
                <span className="legend-swatch" style={{ background: bgraToCss(info.fog_color)! }} />
                Fog {bgraToCss(info.fog_color)}
              </span>
            ) : null}
            {bgraToCss(info.water_vertex_color) ? (
              <span className="legend-item">
                <span
                  className="legend-swatch"
                  style={{ background: bgraToCss(info.water_vertex_color)! }}
                />
                Water {bgraToCss(info.water_vertex_color)}
              </span>
            ) : null}
          </div>
        </Card>
      ) : null}

      {info && (info.loading_screen_title || info.prologue_screen_title) ? (
        <div className="grid-2">
          <Card title="Loading screen">
            <KvList
              rows={[
                ["Title", stripColorCodes(info.loading_screen_title) || "—"],
                ["Subtitle", stripColorCodes(info.loading_screen_subtitle) || "—"],
                ["Text", stripColorCodes(info.loading_screen_text) || "—"],
                ["Model", info.loading_screen_model || "—"],
                [
                  "Background",
                  String(info.loading_screen_background ?? info.campaign_background ?? "—"),
                ],
                ["Game data set", gameDataSetName(info.game_data_set)],
              ]}
            />
          </Card>
          <Card title="Prologue">
            <KvList
              rows={[
                ["Title", stripColorCodes(info.prologue_screen_title) || "—"],
                ["Subtitle", stripColorCodes(info.prologue_screen_subtitle) || "—"],
                ["Text", stripColorCodes(info.prologue_screen_text) || "—"],
                ["Model", info.prologue_screen_model || "—"],
              ]}
            />
          </Card>
        </div>
      ) : null}
    </div>
  );
}
