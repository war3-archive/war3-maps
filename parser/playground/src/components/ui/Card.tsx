import type { ReactNode } from "react";
import { Chip } from "./Chip";

export function Card({
  title,
  badge,
  children,
  className = "",
}: {
  title?: string;
  badge?: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={`card ${className}`.trim()}>
      {title ? (
        <div className="card-title">
          <h2>{title}</h2>
          {badge ? <Chip>{badge}</Chip> : null}
        </div>
      ) : null}
      {children}
    </section>
  );
}
