import { useEffect, useRef } from "react";
import { paintMinimapCover, type OverlayIcon } from "../../lib/minimap";

export function MinimapCanvas({
  imageUrl,
  icons,
  size,
  className = "",
  label,
}: {
  imageUrl: string;
  icons: OverlayIcon[];
  size: number;
  className?: string;
  label?: string;
}) {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    let cancelled = false;
    void paintMinimapCover(canvas, imageUrl, icons, size).catch((err) => {
      if (!cancelled) console.warn(err);
    });
    return () => {
      cancelled = true;
    };
  }, [imageUrl, icons, size]);

  return (
    <canvas
      ref={ref}
      className={`cover-canvas ${className}`.trim()}
      role="img"
      aria-label={label ?? "Minimap"}
    />
  );
}
