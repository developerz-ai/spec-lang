// The `raylib` library's runtime for C++ (libs/raylib/real/cpp): the only file that includes raylib.h.
// Every `foreign c "raylib#Name"` call ends here, and so does the window the `game` library's loop needs.
// What is loaded when the window opens is read from the app's manifest:
//   a rule bound to `BeginShaderMode`: the shader `<assets>/shaders/lit.vs` and `lit.fs`, uniform `uTime`;
//   a rule bound to `DrawModelEx` whose model is a sum without data: one model per variant, in declared
//   order, from `<assets>/out/<name>.glb`, where <name> is what the program's one public fn from that sum to
//   Text gives, else the variant's name in lower case.
#include <algorithm>
#include <cctype>
#include <cstdlib>
#include <vector>

#include "lib/game/screen.hpp"
#include "lib/raylib/foreign.hpp"
#include "program/machine.hpp"
#include "raylib.h"

namespace lib::raylib {

namespace {

// raylib has one window and one GL context, so what is loaded is one value for the process.
struct Loaded {
  bool shader = false;
  ::Shader lit{};
  int time_loc = 0;
  std::vector<::Model> models;
};
Loaded loaded;

constexpr float metres(long long length) {
  return static_cast<float>(length) / 256.0F;
}
::Vector3 to_c(Vec3 v) {
  return {metres(v.x), metres(v.y), metres(v.z)};
}
::Color to_c(Rgba c) {
  return {c.r, c.g, c.b, c.a};
}

struct CubeArgs {
  ::Vector3 centre;
  float width;
  float height;
  float depth;
};

// centre = (lo + hi) / 512 per axis; the sizes are (hi - lo) / 256.
CubeArgs cube_args(const Box& b) {
  return {
      {static_cast<float>(b.lo.x + b.hi.x) / 512.0F, static_cast<float>(b.lo.y + b.hi.y) / 512.0F,
       static_cast<float>(b.lo.z + b.hi.z) / 512.0F},
      static_cast<float>(b.hi.x - b.lo.x) / 256.0F,
      static_cast<float>(b.hi.y - b.lo.y) / 256.0F,
      static_cast<float>(b.hi.z - b.lo.z) / 256.0F,
  };
}

void unload() {
  // In reverse order of loading. UnloadModel leaves the shared shader alone.
  while (!loaded.models.empty()) {
    ::UnloadModel(loaded.models.back());
    loaded.models.pop_back();
  }
  if (loaded.shader) ::UnloadShader(loaded.lit);
  loaded = {};
}

const rt::meta::Callable* bound_to(const rt::meta::App& app, std::string_view symbol) {
  const auto found = std::ranges::find_if(app.rules, [&](const rt::meta::Callable& c) {
    return c.foreign_library == "raylib" && c.foreign_symbol == symbol;
  });
  return found == app.rules.end() ? nullptr : &*found;
}

// The file names of the models, in load order.
std::vector<std::string> model_names(const rt::meta::App& app) {
  const rt::meta::Callable* draw = bound_to(app, "DrawModelEx");
  if (draw == nullptr || draw->params.empty()) return {};
  const rt::meta::TypeRef type = draw->params.front().type;
  const auto& variants = (*type)().variants;
  const auto namer = std::ranges::find_if(app.fns, [&](const rt::meta::Callable& f) {
    return f.params.size() == 1 && f.params.front().type == type && (*f.result)().kind == rt::meta::Kind::Text;
  });
  std::vector<std::string> names;
  program::Machine machine;
  for (std::size_t i = 0; i < variants.size(); ++i) {
    if (namer != app.fns.end()) {
      names.push_back(namer->call(machine, {rt::Data::fields(i, {})}).text());
    } else {
      std::string name = variants[i].name;
      std::ranges::transform(name, name.begin(), [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
      names.push_back(std::move(name));
    }
  }
  return names;
}

// After the window opens: the shader, then each model with the shader on every material.
std::optional<std::string> load(const rt::meta::App& app, const std::string& dir) {
  if (bound_to(app, "BeginShaderMode") != nullptr) {
    const std::string vertex = dir + "/shaders/lit.vs";
    const std::string fragment = dir + "/shaders/lit.fs";
    loaded.lit = ::LoadShader(vertex.c_str(), fragment.c_str());
    loaded.shader = true;
    if (!::IsShaderValid(loaded.lit)) return "asset " + vertex + ": cannot load";
    loaded.time_loc = ::GetShaderLocation(loaded.lit, "uTime");
  }
  for (const std::string& name : model_names(app)) {
    const std::string path = dir + "/out/" + name + ".glb";
    loaded.models.push_back(::LoadModel(path.c_str()));
    ::Model& model = loaded.models.back();
    if (!::IsModelValid(model)) return "asset " + path + ": cannot load";
    // raylib draws a model with its materials' shader; BeginShaderMode does not reach DrawModelEx.
    for (int i = 0; loaded.shader && i < model.materialCount; ++i) model.materials[i].shader = loaded.lit;
  }
  return std::nullopt;
}

bool has_display() {
  const auto set = [](const char* name) {
    const char* const value = std::getenv(name);
    return value != nullptr && *value != '\0';
  };
  return set("DISPLAY") || set("WAYLAND_DISPLAY");
}

class RaylibScreen final : public lib::game::Screen {
 public:
  std::optional<std::string> open(lib::game::Purpose purpose, const lib::game::Window& window, const rt::meta::App& app,
                                  const std::string& assets_dir) override {
    // raylib 5.5 crashes in InitWindow when there is no display, so that case is refused first.
    if (!has_display()) return "window: cannot open";
    ::SetTraceLogLevel(LOG_NONE);
    ::SetConfigFlags(purpose == lib::game::Purpose::Shot ? FLAG_WINDOW_HIDDEN : FLAG_MSAA_4X_HINT);
    ::InitWindow(window.width, window.height, window.title.c_str());
    if (!::IsWindowReady()) return "window: cannot open";
    if (auto error = load(app, assets_dir)) {
      close();
      return error;
    }
    if (purpose == lib::game::Purpose::Play) ::SetTargetFPS(window.rate);
    return std::nullopt;
  }

  bool should_close() override {
    return ::WindowShouldClose();
  }

  void draw(const std::function<void()>& frame) override {
    ::BeginDrawing();
    frame();
    ::EndDrawing();
  }

  lib::game::Capture capture(int width, int height, const std::function<void()>& frame,
                             const std::optional<std::string>& png) override {
    const ::RenderTexture2D target = ::LoadRenderTexture(width, height);
    ::BeginTextureMode(target);
    frame();
    ::EndTextureMode();
    ::Image image = ::LoadImageFromTexture(target.texture);
    // A render texture reads back bottom row first; flipped, the hash and the PNG are top row first.
    ::ImageFlipVertical(&image);
    lib::game::Capture shot;
    if (image.data != nullptr) {
      const auto* bytes = static_cast<const std::uint8_t*>(image.data);
      shot.rgba.assign(bytes,
                       bytes + static_cast<std::size_t>(image.width) * static_cast<std::size_t>(image.height) * 4);
    }
    if (png) shot.exported = ::ExportImage(image, png->c_str());
    ::UnloadImage(image);
    ::UnloadRenderTexture(target);
    return shot;
  }

  void close() override {
    unload();
    ::CloseWindow();
  }
};

RaylibScreen the_screen;

// Linked with the adapters, so an app that calls raylib has its screen.
const struct Install {
  Install() {
    lib::game::install_screen(&the_screen);
  }
} install;

}  // namespace

namespace edge {

void clear_background(Rgba color) {
  ::ClearBackground(to_c(color));
}

void begin_mode_3d(Vec3 position, Vec3 target) {
  ::BeginMode3D(::Camera3D{to_c(position), to_c(target), {0.0F, 1.0F, 0.0F}, 45.0F, CAMERA_PERSPECTIVE});
}

void end_mode_3d() {
  ::EndMode3D();
}

void begin_shader_mode(long long time) {
  if (!loaded.shader) rt::stop("E-RUNTIME", "BeginShaderMode before the window loaded its shader");
  const float t = static_cast<float>(time);
  ::SetShaderValue(loaded.lit, loaded.time_loc, &t, SHADER_UNIFORM_FLOAT);
  ::BeginShaderMode(loaded.lit);
}

void end_shader_mode() {
  ::EndShaderMode();
}

void draw_cube(const Box& box, Rgba color) {
  const CubeArgs a = cube_args(box);
  ::DrawCube(a.centre, a.width, a.height, a.depth, to_c(color));
}

void draw_cube_wires(const Box& box, Rgba color) {
  const CubeArgs a = cube_args(box);
  ::DrawCubeWires(a.centre, a.width, a.height, a.depth, to_c(color));
}

void draw_model_ex(std::size_t model, Vec3 position, long long yaw) {
  if (model >= loaded.models.size()) rt::stop("E-RUNTIME", "DrawModelEx of a model that was not loaded");
  ::DrawModelEx(loaded.models[model], to_c(position), {0.0F, 1.0F, 0.0F}, static_cast<float>(yaw), {1.0F, 1.0F, 1.0F},
                ::Color{255, 255, 255, 255});
}

void draw_text(const std::string& text, long long x, long long y, long long size, Rgba color) {
  ::DrawText(text.c_str(), static_cast<int>(x), static_cast<int>(y), static_cast<int>(size), to_c(color));
}

bool is_key_down(long long key) {
  return ::IsKeyDown(static_cast<int>(key));
}

bool check_collision_boxes(const Box& a, const Box& b) {
  return ::CheckCollisionBoxes(::BoundingBox{to_c(a.lo), to_c(a.hi)}, ::BoundingBox{to_c(b.lo), to_c(b.hi)});
}

}  // namespace edge

}  // namespace lib::raylib
