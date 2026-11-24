# `raylib`

raylib 5.5, as the calls a program may make: [`contract.slg`](contract.slg) declares each as `foreign c "raylib#Name"` with the effect it has (`screen.write` or `keys.read`, from [`game`](../game/README.md), whose two handlers this library offers). `use raylib;` gives them as `raylib.draw_cube(...)`.

| Calls | Effect |
|---|---|
| `clear_background`, `begin_mode_3d`, `end_mode_3d`, `begin_shader_mode`, `end_shader_mode`, `draw_cube`, `draw_cube_wires`, `draw_model_ex`, `draw_text` | `screen.write` |
| `is_key_down` | `keys.read` |
| `check_collision_boxes` | none |

These are the calls the platformer makes. The platformer still declares them itself in `pocs/platformer/src/hop/screen.slg`, over its own types; moving it to `use raylib;` is its owners' choice.

The engine never runs a foreign body: a check that reaches one is blocked (`B-FOREIGN`), so `pretend.slg` is empty and `vocab.slg` only offers `game`'s handlers.

## Limits

- The card has no `f32` and no C struct layout. Lengths here are whole numbers of 1/256 of a raylib unit, as the platformer's edge converts them ([contract](../../pocs/platformer/docs/contract.md) section 5); a runtime must convert the same way.
- `draw_cube` and `draw_model_ex` take the platformer's shape of the call (a box, a model index), not raylib's exact signature.
- No simulator: every check through raylib is blocked, so its laws are `assumed`. A pretend would let tests run; nobody has written one.
