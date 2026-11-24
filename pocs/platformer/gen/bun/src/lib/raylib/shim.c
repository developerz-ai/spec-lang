// The `raylib` library's runtime for TypeScript (libs/raylib/real/ts): the by-value half of the foreign calls,
// which bun:ffi cannot make. Lifted from pocs/platformer/out/bun/native/shim.c.
//
// bun:ffi passes scalars and pointers only: it has no C struct type, by value or as a return.
// Each function here takes the address of the raylib function (found by bun:ffi's own dlopen, so
// `<BIN>_RAYLIB` decides the library) and pointers to the structs, and makes the by-value call.
// Nothing but argument marshalling: no state, no logic, no raylib symbol linked.

#include <stdbool.h>
#include <stddef.h>

#include "raylib.h"

// The layouts src/screen/structs.ts writes and reads from TypeScript, checked against raylib.h at build time.
_Static_assert(sizeof(Vector3) == 12 && sizeof(Color) == 4 && sizeof(BoundingBox) == 24, "value structs");
_Static_assert(sizeof(Camera3D) == 44 && offsetof(Camera3D, fovy) == 36 && offsetof(Camera3D, projection) == 40, "Camera3D");
_Static_assert(sizeof(Shader) == 16, "Shader");
_Static_assert(sizeof(Model) == 120 && offsetof(Model, materialCount) == 68 && offsetof(Model, materials) == 80, "Model");
_Static_assert(sizeof(Material) == 40 && offsetof(Material, shader) == 0, "Material");
_Static_assert(sizeof(RenderTexture2D) == 44 && offsetof(RenderTexture2D, texture) == 4 && sizeof(Texture2D) == 20, "RenderTexture2D");
_Static_assert(sizeof(Image) == 24 && offsetof(Image, data) == 0 && offsetof(Image, width) == 8 && offsetof(Image, height) == 12 && offsetof(Image, format) == 20, "Image");

// Struct returned by value.
void spec_rl_LoadShader(Shader (*fn)(const char *, const char *), Shader *out, const char *vs, const char *fs) { *out = fn(vs, fs); }
void spec_rl_LoadModel(Model (*fn)(const char *), Model *out, const char *path) { *out = fn(path); }
void spec_rl_LoadRenderTexture(RenderTexture2D (*fn)(int, int), RenderTexture2D *out, int width, int height) { *out = fn(width, height); }
void spec_rl_LoadImageFromTexture(Image (*fn)(Texture2D), Image *out, const Texture2D *texture) { *out = fn(*texture); }

// Struct passed by value: resources.
bool spec_rl_IsShaderValid(bool (*fn)(Shader), const Shader *shader) { return fn(*shader); }
int spec_rl_GetShaderLocation(int (*fn)(Shader, const char *), const Shader *shader, const char *name) { return fn(*shader, name); }
void spec_rl_SetShaderValue(void (*fn)(Shader, int, const void *, int), const Shader *shader, int loc, const void *value, int type) { fn(*shader, loc, value, type); }
void spec_rl_BeginShaderMode(void (*fn)(Shader), const Shader *shader) { fn(*shader); }
void spec_rl_UnloadShader(void (*fn)(Shader), const Shader *shader) { fn(*shader); }
bool spec_rl_IsModelValid(bool (*fn)(Model), const Model *model) { return fn(*model); }
void spec_rl_UnloadModel(void (*fn)(Model), const Model *model) { fn(*model); }
void spec_rl_BeginTextureMode(void (*fn)(RenderTexture2D), const RenderTexture2D *target) { fn(*target); }
void spec_rl_UnloadRenderTexture(void (*fn)(RenderTexture2D), const RenderTexture2D *target) { fn(*target); }
bool spec_rl_ExportImage(bool (*fn)(Image, const char *), const Image *image, const char *path) { return fn(*image, path); }
void spec_rl_UnloadImage(void (*fn)(Image), const Image *image) { fn(*image); }

// Struct passed by value: drawing.
void spec_rl_ClearBackground(void (*fn)(Color), const Color *color) { fn(*color); }
void spec_rl_BeginMode3D(void (*fn)(Camera3D), const Camera3D *camera) { fn(*camera); }
void spec_rl_DrawCube(void (*fn)(Vector3, float, float, float, Color), const Vector3 *centre, float w, float h, float d, const Color *color) { fn(*centre, w, h, d, *color); }
void spec_rl_DrawCubeWires(void (*fn)(Vector3, float, float, float, Color), const Vector3 *centre, float w, float h, float d, const Color *color) { fn(*centre, w, h, d, *color); }
void spec_rl_DrawModelEx(void (*fn)(Model, Vector3, Vector3, float, Vector3, Color), const Model *model, const Vector3 *at, const Vector3 *axis, float angle, const Vector3 *scale, const Color *tint) { fn(*model, *at, *axis, angle, *scale, *tint); }
void spec_rl_DrawText(void (*fn)(const char *, int, int, int, Color), const char *text, int x, int y, int size, const Color *color) { fn(text, x, y, size, *color); }

// Struct passed by value: geometry.
bool spec_rl_CheckCollisionBoxes(bool (*fn)(BoundingBox, BoundingBox), const BoundingBox *a, const BoundingBox *b) { return fn(*a, *b); }
