// The `raylib` library's runtime for TypeScript (libs/raylib/real/ts).

/** A shared library imported `with { type: "file" }`: its path, inside the executable once compiled. */
declare module "*.so" {
  const path: string;
  export default path;
}
