import { useMemo, useState } from "react";
import type { MapMetadata } from "../../lib/wasm";
import { Card } from "../ui/Card";

export function JsonPanel({ data }: { data: MapMetadata }) {
  const text = useMemo(() => {
    const slim = {
      ...data,
      images: (data.images ?? []).map((img) => ({
        filename: img.filename,
        width: img.width,
        height: img.height,
        data_url: `…omitted ${img.data_url?.length ?? 0} chars…`,
      })),
    };
    return JSON.stringify(slim, null, 2);
  }, [data]);

  const downloadUrl = useMemo(
    () => URL.createObjectURL(new Blob([text], { type: "application/json" })),
    [text],
  );

  const [copied, setCopied] = useState(false);

  return (
    <Card>
      <div className="toolbar">
        <h2>Raw JSON</h2>
        <div style={{ display: "flex", gap: "0.5rem" }}>
          <button
            type="button"
            className="btn"
            onClick={async () => {
              await navigator.clipboard.writeText(text);
              setCopied(true);
              window.setTimeout(() => setCopied(false), 1200);
            }}
          >
            {copied ? "Copied" : "Copy"}
          </button>
          <a className="btn btn-ghost" href={downloadUrl} download="war3parser-map.json">
            Download
          </a>
        </div>
      </div>
      <pre className="pre">{text}</pre>
    </Card>
  );
}
