import init, {
  parse_map,
  version as wasmVersion,
  type War3MapMetadata,
} from "@wesleyel/war3parser";

let ready: Promise<void> | null = null;

export type MapMetadata = War3MapMetadata;

export async function ensureWasm(): Promise<string> {
  if (!ready) {
    ready = init().then(() => undefined);
  }
  await ready;
  return wasmVersion();
}

export function parseMap(bytes: Uint8Array): MapMetadata | undefined {
  return parse_map(bytes) ?? undefined;
}
