// The `raylib` library's runtime for TypeScript (libs/raylib/real/ts): the `screen` handler, which `game`'s
// runtime drives. A window is raylib's global state, so the foreign calls draw through the one window open here.
// Assets, loaded after the window opens: the shader `<assets>/shaders/lit.{vs,fs}` (its uniform `uTime` is
// `begin_shader_mode`'s time), and one model per variant of the sum `DrawModelEx` takes, in declared order,
// from `<assets>/out/<name>.glb`, `<name>` given by the program's one function from that sum to Text.

import { isAbsolute, resolve } from "node:path";
import * as rt from "../../runtime/index.ts";
import { openRaylib, type Raylib } from "./bind.ts";
import { cString, imagePixels, RENDER_TEXTURE_TEXTURE, SIZE, setMaterialShaders, TEXTURE_SIZE } from "./structs.ts";

const LOG_NONE = 7;
const FLAG_WINDOW_HIDDEN = 0x80;
const FLAG_MSAA_4X_HINT = 0x20;

export interface Window {
  readonly rl: Raylib;
  /** `Shader`, 16 bytes. */
  readonly lit: Uint8Array;
  readonly timeLoc: number;
  /** `Model`, 120 bytes each, by the variant's position. */
  readonly models: readonly Uint8Array[];
}

/** The window open now: what the foreign calls draw on. */
let current: Window | undefined;

export function window(): Window {
  if (current === undefined) return rt.stop("E-RUNTIME", "a raylib call with no window open");
  return current;
}

/** What `open` is asked for. */
export interface WindowSpec {
  /** The executable's name: `<BIN>_RAYLIB` and `<BIN>_ASSETS` are read. */
  readonly bin: string;
  readonly width: number;
  readonly height: number;
  readonly title: string;
  /** Off screen, for a frame read back; else on screen, multisampled. */
  readonly hidden: boolean;
  readonly env: Readonly<Record<string, string | undefined>>;
}

type Env = WindowSpec["env"];

function setting(env: Env, name: string): string | undefined {
  const v = env[name];
  return v === undefined || v === "" ? undefined : v;
}

/** `<BIN>_RAYLIB` first; otherwise `vendor/raylib/lib/libraylib.so` from the working directory. */
function libraryPath(spec: WindowSpec): string {
  const path = setting(spec.env, `${spec.bin.toUpperCase()}_RAYLIB`) ?? "vendor/raylib/lib/libraylib.so";
  // dlopen searches the system for a bare name; a path is always meant here.
  return isAbsolute(path) ? path : resolve(path);
}

/** raylib 5.5 crashes in `InitWindow` when no display is named, so that is asked first. */
const displayNamed = (env: Env): boolean => [env.DISPLAY, env.WAYLAND_DISPLAY].some((d) => d !== undefined && d !== "");

/** The model files, from the program: one per variant of the sum `DrawModelEx` draws. */
function modelNames(program: rt.ProgramView): readonly string[] {
  const draw = program.externs.find((x) => x.library === "raylib" && x.symbol === "DrawModelEx");
  const asset = draw?.params[0]?.type;
  const sum = asset === undefined ? undefined : unalias(asset);
  if (asset === undefined || !(sum instanceof rt.SumType)) return [];
  const namer = program.fns.find(
    (f) => f.params.length === 1 && f.params[0]?.type.key === asset.key && f.result instanceof rt.TextType,
  );
  if (namer === undefined) return rt.stop("E-UNSUPPORTED", `no function names the models of ${asset.key}`);
  return sum.variants.map((v) => namer.call(undefined as never, [sum.make(v.name)]) as string);
}

function unalias(t: rt.Type): rt.Type {
  return t instanceof rt.AliasType ? unalias(t.of) : t;
}

export class Screen implements rt.Handler {
  constructor(private readonly program: rt.ProgramView) {}

  /** Opens the window and loads the assets; the error's message when it cannot. */
  open(spec: WindowSpec): string | undefined {
    const path = libraryPath(spec);
    const opened = openRaylib(path);
    if (!opened.ok) return `raylib ${path}: cannot load`;
    const rl = opened.raylib;
    if (!displayNamed(spec.env)) return "window: cannot open";
    rl.c.SetTraceLogLevel(LOG_NONE);
    rl.c.SetConfigFlags(spec.hidden ? FLAG_WINDOW_HIDDEN : FLAG_MSAA_4X_HINT);
    rl.c.InitWindow(spec.width, spec.height, cString(spec.title));
    if (!rl.c.IsWindowReady()) return "window: cannot open";
    const dir = setting(spec.env, `${spec.bin.toUpperCase()}_ASSETS`) ?? "assets";
    const loaded = load(rl, dir, modelNames(this.program));
    if (!loaded.ok) {
      rl.c.CloseWindow();
      return `asset ${loaded.path}: cannot load`;
    }
    current = loaded.window;
    return undefined;
  }

  /** At most this many frames a second. */
  fps(n: number): void {
    window().rl.c.SetTargetFPS(n);
  }

  /** Esc or the close button. */
  closing(): boolean {
    return window().rl.c.WindowShouldClose();
  }

  /** One frame on the window. */
  frame(draw: () => void): void {
    const { rl } = window();
    rl.c.BeginDrawing();
    draw();
    rl.c.EndDrawing();
  }

  /** One frame of `width` by `height` drawn off screen, its pixels top row first; with `out`, also a PNG. */
  offscreen<T>(
    width: number,
    height: number,
    draw: () => void,
    out: string | null,
    read: (pixels: Uint8Array) => T,
  ): { readonly ok: true; readonly value: T } | { readonly ok: false; readonly error: string } {
    const { rl } = window();
    const target = new Uint8Array(SIZE.renderTexture);
    rl.shim.spec_rl_LoadRenderTexture(rl.fn("LoadRenderTexture"), target, width, height);
    rl.shim.spec_rl_BeginTextureMode(rl.fn("BeginTextureMode"), target);
    draw();
    rl.c.EndTextureMode();
    const image = new Uint8Array(SIZE.image);
    const texture = target.subarray(RENDER_TEXTURE_TEXTURE, RENDER_TEXTURE_TEXTURE + TEXTURE_SIZE);
    rl.shim.spec_rl_LoadImageFromTexture(rl.fn("LoadImageFromTexture"), image, texture);
    // Bottom row first as read from the texture; flipped in place so the top row is first, as a window shows it.
    rl.c.ImageFlipVertical(image);
    const value = read(imagePixels(image));
    const exported = out === null || rl.shim.spec_rl_ExportImage(rl.fn("ExportImage"), image, cString(out));
    rl.shim.spec_rl_UnloadImage(rl.fn("UnloadImage"), image);
    rl.shim.spec_rl_UnloadRenderTexture(rl.fn("UnloadRenderTexture"), target);
    return exported ? { ok: true, value } : { ok: false, error: `image ${out}: cannot write` };
  }

  /** Unloads in reverse order, then `CloseWindow()`. */
  close(): void {
    const w = current;
    if (w === undefined) return;
    unload(w.rl, w.models);
    w.rl.shim.spec_rl_UnloadShader(w.rl.fn("UnloadShader"), w.lit);
    w.rl.c.CloseWindow();
    current = undefined;
  }

  /** The window is outside the world: nothing to take back. */
  save(): unknown {
    return null;
  }

  restore(): void {}
}

function unload(rl: Raylib, models: readonly Uint8Array[]): void {
  for (const model of models.toReversed()) rl.shim.spec_rl_UnloadModel(rl.fn("UnloadModel"), model);
}

type Loaded = { readonly ok: true; readonly window: Window } | { readonly ok: false; readonly path: string };

/** After the window opens. On a failure everything already loaded is unloaded and the path is returned. */
function load(rl: Raylib, dir: string, names: readonly string[]): Loaded {
  const vs = `${dir}/shaders/lit.vs`;
  const lit = new Uint8Array(SIZE.shader);
  rl.shim.spec_rl_LoadShader(rl.fn("LoadShader"), lit, cString(vs), cString(`${dir}/shaders/lit.fs`));
  if (!rl.shim.spec_rl_IsShaderValid(rl.fn("IsShaderValid"), lit)) return { ok: false, path: vs };
  const timeLoc = rl.shim.spec_rl_GetShaderLocation(rl.fn("GetShaderLocation"), lit, cString("uTime"));
  const models: Uint8Array[] = [];
  for (const name of names) {
    const path = `${dir}/out/${name}.glb`;
    const model = new Uint8Array(SIZE.model);
    rl.shim.spec_rl_LoadModel(rl.fn("LoadModel"), model, cString(path));
    if (!rl.shim.spec_rl_IsModelValid(rl.fn("IsModelValid"), model)) {
      rl.shim.spec_rl_UnloadModel(rl.fn("UnloadModel"), model);
      unload(rl, models);
      rl.shim.spec_rl_UnloadShader(rl.fn("UnloadShader"), lit);
      return { ok: false, path };
    }
    // raylib draws a model with its materials' shader; BeginShaderMode does not reach DrawModelEx.
    setMaterialShaders(model, lit);
    models.push(model);
  }
  return { ok: true, window: { rl, lit, timeLoc, models } };
}
