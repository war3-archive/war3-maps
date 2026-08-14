import { useMemo } from "react";
import {
  buildOverlayIcons,
  iconTypeLabel,
  isMinimapImage,
} from "../../lib/minimap";
import type { MapMetadata } from "../../lib/wasm";
import { Card } from "../ui/Card";
import { MinimapCanvas } from "../ui/MinimapCanvas";

export function ImagesPanel({ data }: { data: MapMetadata }) {
  const images = data.images ?? [];
  const icons = useMemo(() => buildOverlayIcons(data), [data]);

  if (!images.length) {
    return <div className="empty">No minimap/preview images found</div>;
  }

  return (
    <div className="panel-stack">
      <Card title="Images" badge={String(images.length)}>
        <div className="image-grid">
          {images.map((img) => {
            const isMap = isMinimapImage(img.filename);
            return (
              <article key={`${img.filename}-${img.width}`} className="image-card">
                {isMap && icons.length ? (
                  <MinimapCanvas
                    imageUrl={img.data_url}
                    icons={icons}
                    size={360}
                    className="cover-canvas-lg"
                    label={img.filename}
                  />
                ) : (
                  <img src={img.data_url} alt={img.filename} />
                )}
                <div className="image-meta">
                  <span className="mono">{img.filename}</span>
                  <span>
                    {img.width}×{img.height}
                    {isMap ? " · annotated" : ""}
                  </span>
                </div>
                <div>
                  <a
                    className="btn btn-ghost"
                    href={img.data_url}
                    download={`${img.filename.replace(/[\\/]/g, "_")}.png`}
                  >
                    Download PNG
                  </a>
                </div>
              </article>
            );
          })}
        </div>
      </Card>

      {icons.length ? (
        <Card title="Minimap icons (war3map.mmp)" badge={String(icons.length)}>
          <div className="table-wrap">
            <table className="data">
              <thead>
                <tr>
                  <th>Type</th>
                  <th>X</th>
                  <th>Y</th>
                  <th>Color</th>
                </tr>
              </thead>
              <tbody>
                {icons.slice(0, 200).map((ic, i) => (
                  <tr key={`${ic.icon_type}-${ic.x}-${ic.y}-${i}`}>
                    <td>{iconTypeLabel(ic.icon_type)}</td>
                    <td className="mono">{Math.round(ic.x)}</td>
                    <td className="mono">{Math.round(ic.y)}</td>
                    <td>
                      <span className="swatch" style={{ background: ic.color }} />
                      <span className="mono">{ic.color}</span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      ) : null}
    </div>
  );
}
