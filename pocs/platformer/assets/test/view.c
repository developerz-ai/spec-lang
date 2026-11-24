// Loader and render proof for the Skyhop assets, in raylib itself.
//   view <assets-dir> <out.png> <width> <height> <side|hero>
// Loads shaders/lit.{vs,fs} and the four GLBs from <assets-dir>, checks each model's bounding
// box against the size table and the origin rule, renders a small scene into a render texture,
// checks the frame is not trivial, writes it as a PNG and prints its FNV-1a 64 hash.
// Exit 0: every check passed. Exit 1: a check failed (each failure is printed).

#include <math.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "raylib.h"
#include "rlgl.h"

#define TICK 90.0f // the fixed sim tick the frame is drawn at (1.5 s)

typedef struct {
    const char *name;
    float width, height, depth; // X, Y, Z in the game, metres
    bool centred;               // origin at the centre (gem) instead of the feet
} Spec;

static const Spec SPECS[] = {
    {"pip", 0.5f, 0.75f, 0.5f, false},
    {"crawler", 0.75f, 0.5f, 0.75f, false},
    {"gem", 0.5f, 0.5f, 0.5f, true},
    {"flag", 1.0f, 3.0f, 1.0f, false},
};
enum { PIP, CRAWLER, GEM, FLAG, MODEL_COUNT };

static const Color SKY = {135, 206, 235, 255};
static const Color GRASS = {96, 186, 84, 255};
static const Color STONE = {168, 164, 156, 255};

static int failures = 0;

static void check(bool ok, const char *format, ...)
{
    if (ok) return;
    va_list args;
    va_start(args, format);
    fprintf(stderr, "FAIL: ");
    vfprintf(stderr, format, args);
    fprintf(stderr, "\n");
    va_end(args);
    failures++;
}

static bool within(float value, float want, float fraction)
{
    return fabsf(value - want) <= want*fraction;
}

static void check_bounds(const Spec *spec, Model model)
{
    BoundingBox box = GetModelBoundingBox(model);
    float size[3] = {box.max.x - box.min.x, box.max.y - box.min.y, box.max.z - box.min.z};
    float want[3] = {spec->width, spec->height, spec->depth};
    const char *axis = "XYZ";
    for (int i = 0; i < 3; i++)
        check(within(size[i], want[i], 0.10f), "%s: size %c is %.3f m, want %.3f m within 10%%",
              spec->name, axis[i], size[i], want[i]);
    check(fabsf(box.min.x + box.max.x) <= 0.05f*spec->width, "%s: not centred on X (%.3f..%.3f)",
          spec->name, box.min.x, box.max.x);
    check(fabsf(box.min.z + box.max.z) <= 0.05f*spec->depth, "%s: not centred on Z (%.3f..%.3f)",
          spec->name, box.min.z, box.max.z);
    if (spec->centred)
        check(fabsf(box.min.y + box.max.y) <= 0.05f*spec->height, "%s: not centred on Y (%.3f..%.3f)",
              spec->name, box.min.y, box.max.y);
    else
        check(fabsf(box.min.y) <= 0.01f, "%s: feet at y = %.4f, want 0", spec->name, box.min.y);
    int triangles = 0;
    for (int i = 0; i < model.meshCount; i++) triangles += model.meshes[i].triangleCount;
    printf("%s: box min %.3f %.3f %.3f max %.3f %.3f %.3f, %d mesh(es), %d triangles\n", spec->name,
           box.min.x, box.min.y, box.min.z, box.max.x, box.max.y, box.max.z, model.meshCount, triangles);
}

static void draw_model(Model model, Vector3 position)
{
    // The one draw call the game uses: identity rotation, unit scale.
    DrawModelEx(model, position, (Vector3){0.0f, 1.0f, 0.0f}, 0.0f, (Vector3){1.0f, 1.0f, 1.0f}, WHITE);
}

static void draw_scene(const Model *models)
{
    DrawCube((Vector3){0.0f, -0.5f, 0.0f}, 9.0f, 1.0f, 3.0f, GRASS);
    DrawCube((Vector3){-4.0f, 0.5f, -3.0f}, 3.0f, 3.0f, 3.0f, GRASS);
    DrawCube((Vector3){3.0f, 0.25f, 0.0f}, 2.0f, 0.5f, 2.0f, STONE);
    DrawCube((Vector3){7.0f, 0.5f, -1.0f}, 3.0f, 0.5f, 3.0f, GRASS);
    DrawCubeWires((Vector3){3.0f, 0.25f, 0.0f}, 2.0f, 0.5f, 2.0f, (Color){90, 88, 84, 255});
    DrawSphere((Vector3){-3.6f, 0.3f, -0.9f}, 0.45f, (Color){70, 150, 70, 255});

    draw_model(models[PIP], (Vector3){-1.6f, 0.0f, 0.3f});
    draw_model(models[CRAWLER], (Vector3){0.6f, 0.0f, 0.2f});
    draw_model(models[GEM], (Vector3){-0.5f, 1.1f, 0.3f});
    draw_model(models[GEM], (Vector3){1.9f, 0.9f, 0.3f});
    draw_model(models[FLAG], (Vector3){3.0f, 0.5f, 0.0f});
}

// Read a render texture back as top-down RGBA8.
static Image read_back(RenderTexture2D target)
{
    Image image = LoadImageFromTexture(target.texture);
    ImageFlipVertical(&image);
    ImageFormat(&image, PIXELFORMAT_UNCOMPRESSED_R8G8B8A8);
    return image;
}

// A model drawn alone on the sky must leave coloured pixels: not white, not black, not grey.
static void check_model_pixels(const Spec *spec, Model model, Shader shader, RenderTexture2D target)
{
    float reach = fmaxf(spec->height, spec->width);
    float middle = spec->centred ? 0.0f : spec->height*0.5f;
    Camera3D camera = {{reach*1.2f, middle + reach*0.5f, reach*2.2f}, {0.0f, middle, 0.0f},
                       {0.0f, 1.0f, 0.0f}, 40.0f, CAMERA_PERSPECTIVE};
    BeginTextureMode(target);
    ClearBackground(SKY);
    BeginMode3D(camera);
    BeginShaderMode(shader);
    draw_model(model, (Vector3){0.0f, 0.0f, 0.0f});
    EndShaderMode();
    EndMode3D();
    EndTextureMode();

    Image image = read_back(target);
    const unsigned char *px = image.data;
    long count = 0, extreme = 0;
    double sum[3] = {0.0, 0.0, 0.0};
    for (int i = 0; i < image.width*image.height; i++, px += 4) {
        if (px[0] == SKY.r && px[1] == SKY.g && px[2] == SKY.b) continue;
        count++;
        for (int c = 0; c < 3; c++) sum[c] += px[c];
        bool white = px[0] > 250 && px[1] > 250 && px[2] > 250;
        bool black = px[0] < 5 && px[1] < 5 && px[2] < 5;
        if (white || black) extreme++;
    }
    UnloadImage(image);
    check(count >= image.width*image.height/200, "%s: only %ld pixels drawn", spec->name, count);
    if (count == 0) return;
    double mean[3] = {sum[0]/count, sum[1]/count, sum[2]/count};
    double high = fmax(mean[0], fmax(mean[1], mean[2]));
    double low = fmin(mean[0], fmin(mean[1], mean[2]));
    check(extreme*2 < count, "%s: %ld of %ld pixels are pure white or black", spec->name, extreme, count);
    check(high - low >= 20.0, "%s: mean colour %.0f %.0f %.0f has no hue (colours lost?)", spec->name,
          mean[0], mean[1], mean[2]);
    printf("%s: %ld pixels, mean colour %.0f %.0f %.0f\n", spec->name, count, mean[0], mean[1], mean[2]);
}

static Image render_scene(const Model *models, Shader shader, int timeLocation, RenderTexture2D target,
                          Camera3D camera, float tick)
{
    SetShaderValue(shader, timeLocation, &tick, SHADER_UNIFORM_FLOAT);
    BeginTextureMode(target);
    ClearBackground(SKY);
    BeginMode3D(camera);
    BeginShaderMode(shader);
    draw_scene(models);
    EndShaderMode();
    EndMode3D();
    EndTextureMode();
    return read_back(target);
}

static int count_colours(const Image *image, int limit)
{
    uint32_t *seen = calloc((size_t)limit, sizeof(uint32_t));
    int found = 0;
    const unsigned char *px = image->data;
    for (int i = 0; i < image->width*image->height && found < limit; i++, px += 4) {
        uint32_t key = ((uint32_t)px[0] << 16) | ((uint32_t)px[1] << 8) | px[2];
        int j = 0;
        while (j < found && seen[j] != key) j++;
        if (j == found) seen[found++] = key;
    }
    free(seen);
    return found;
}

static uint64_t fnv1a64(const unsigned char *data, size_t length)
{
    uint64_t hash = 0xcbf29ce484222325ULL;
    for (size_t i = 0; i < length; i++) {
        hash ^= data[i];
        hash *= 0x100000001b3ULL;
    }
    return hash;
}

int main(int argc, char **argv)
{
    if (argc != 6) {
        fprintf(stderr, "usage: view <assets-dir> <out.png> <width> <height> <side|hero>\n");
        return 2;
    }
    const char *assets = argv[1];
    int width = atoi(argv[3]);
    int height = atoi(argv[4]);
    bool hero = strcmp(argv[5], "hero") == 0;
    if (width <= 0 || height <= 0 || (!hero && strcmp(argv[5], "side") != 0)) {
        fprintf(stderr, "view: bad size or camera '%s'\n", argv[5]);
        return 2;
    }

    SetTraceLogLevel(LOG_WARNING);
    SetConfigFlags(FLAG_WINDOW_HIDDEN);
    InitWindow(320, 180, "skyhop assets");
    if (!IsWindowReady()) {
        fprintf(stderr, "view: no window (run under xvfb-run -a)\n");
        return 2;
    }

    Shader shader = LoadShader(TextFormat("%s/shaders/lit.vs", assets), TextFormat("%s/shaders/lit.fs", assets));
    check(IsShaderValid(shader), "shader did not load");
    check(shader.id != rlGetShaderIdDefault(), "shader fell back to raylib's default");
    int timeLocation = GetShaderLocation(shader, "uTime");
    check(timeLocation >= 0, "uTime location is %d", timeLocation);
    float tick = TICK;
    SetShaderValue(shader, timeLocation, &tick, SHADER_UNIFORM_FLOAT); // the game's per-frame call

    Model models[MODEL_COUNT];
    for (int i = 0; i < MODEL_COUNT; i++) {
        models[i] = LoadModel(TextFormat("%s/out/%s.glb", assets, SPECS[i].name));
        check(IsModelValid(models[i]), "%s: model did not load", SPECS[i].name);
        if (!IsModelValid(models[i])) return 1;
        check_bounds(&SPECS[i], models[i]);
        // BeginShaderMode only reaches raylib's batch; a model draws with its materials' shader.
        for (int m = 0; m < models[i].materialCount; m++) models[i].materials[m].shader = shader;
    }

    RenderTexture2D target = LoadRenderTexture(width, height);
    for (int i = 0; i < MODEL_COUNT; i++) check_model_pixels(&SPECS[i], models[i], shader, target);

    Camera3D camera = hero
        ? (Camera3D){{4.4f, 2.6f, 6.2f}, {0.2f, 0.8f, 0.0f}, {0.0f, 1.0f, 0.0f}, 36.0f, CAMERA_PERSPECTIVE}
        : (Camera3D){{0.8f, 1.9f, 9.5f}, {0.8f, 1.1f, 0.0f}, {0.0f, 1.0f, 0.0f}, 34.0f, CAMERA_PERSPECTIVE};
    // The frame at tick 0 and at TICK must differ: uTime moves the grass ripple and the gem shimmer.
    Image still = render_scene(models, shader, timeLocation, target, camera, 0.0f);
    Image frame = render_scene(models, shader, timeLocation, target, camera, TICK);
    size_t bytes = (size_t)frame.width*(size_t)frame.height*4;
    check(memcmp(still.data, frame.data, bytes) != 0, "uTime changes nothing in the frame");
    UnloadImage(still);
    int colours = count_colours(&frame, 4096);
    check(colours >= 64, "frame has only %d distinct colours", colours);
    check(ExportImage(frame, argv[2]), "could not write %s", argv[2]);
    printf("frame: %dx%d, %d%s distinct colours\n", frame.width, frame.height, colours,
           colours == 4096 ? "+" : "");
    printf("hash %016llx\n",
           (unsigned long long)fnv1a64(frame.data, bytes));

    UnloadImage(frame);
    UnloadRenderTexture(target);
    for (int i = 0; i < MODEL_COUNT; i++) UnloadModel(models[i]);
    UnloadShader(shader);
    CloseWindow();
    return failures == 0 ? 0 : 1;
}
