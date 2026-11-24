// Projected by hand from "load before the rule" of @store (docs/cli-vocabulary.md, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

import { readFileSync } from "node:fs";
import { EMPTY_WORLD, type World } from "../board/world.ts";
import { readCanonical } from "./canonical-read.ts";
import { decodeWorld } from "./decode.ts";
import { readJson } from "./json-reader.ts";
import { isSystemError } from "./system-error.ts";

/** `bytes` is what was on disk, or undefined for a missing store, so a write can tell whether it changes anything. */
export type Loaded =
  | { readonly ok: true; readonly world: World; readonly bytes: Uint8Array | undefined }
  | { readonly ok: false; readonly error: "unreadable" };

/** Strict UTF-8; a byte-order mark is kept, so the JSON reader refuses it. */
const utf8 = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true });

/** A missing store is the initial world; anything else that is not a valid store is unreadable. */
export function readStore(path: string): Loaded {
  let bytes: Uint8Array;
  try {
    bytes = readFileSync(path);
  } catch (e) {
    if (!isSystemError(e)) throw e;
    if (e.code === "ENOENT") return { ok: true, world: EMPTY_WORLD, bytes: undefined };
    return { ok: false, error: "unreadable" };
  }
  const parsed = parseStore(bytes);
  return parsed.ok ? { ok: true, world: parsed.world, bytes } : parsed;
}

export function parseStore(
  bytes: Uint8Array,
): { readonly ok: true; readonly world: World } | { readonly ok: false; readonly error: "unreadable" } {
  let text: string;
  try {
    text = utf8.decode(bytes);
  } catch (e) {
    if (!(e instanceof TypeError)) throw e;
    return { ok: false, error: "unreadable" };
  }
  const canonical = readCanonical(text);
  if (canonical !== undefined) return { ok: true, world: canonical };
  const json = readJson(text);
  if (!json.ok) return { ok: false, error: "unreadable" };
  const decoded = decodeWorld(json.value);
  return decoded.ok ? { ok: true, world: decoded.world } : { ok: false, error: "unreadable" };
}
