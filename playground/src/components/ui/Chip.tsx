import type { ReactNode } from "react";

type Tone = "default" | "ok" | "warn" | "err" | "brass";

const toneClass: Record<Tone, string> = {
  default: "chip",
  ok: "chip chip-ok",
  warn: "chip chip-warn",
  err: "chip chip-err",
  brass: "chip chip-brass",
};

export function Chip({ children, tone = "default" }: { children: ReactNode; tone?: Tone }) {
  return <span className={toneClass[tone]}>{children}</span>;
}
