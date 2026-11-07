# agent/cos-units-null

## 1c-01h: values read and printed in a scene

Built on 1c-01b's `null_value` (Null `2c66b92`). This replaces my first 1c-01b (`b71dda3`), which the board had given to two workers; its word tables are dropped, and only its scene part is ported.

**What changed.**

- `Scene` holds the three floats a conversion depends on beyond its word: `scale_length`, `fps` and `fps_base`. The default is unit scale 1 at 24 fps.
- `read_in` reads a word in a scene. A value written with a unit is divided by the unit scale to its dimension's power, as `BKE_unit_value_scale` does:
  - lengths, velocities and accelerations by s;
  - areas and powers by s²;
  - volumes and masses by s³.
- A bare number is already in the internal unit and is never scaled.
- `Frames`, a new dimension, takes the words of seconds and multiplies them by `fps` / `fps_base`. `1.5s` is 36 frames at 24 fps.
- Each float is taken at its exact binary value and the whole product rounded once. Where the scene changes nothing (scale 1, or a dimension it never scales), the plain conversions of 1c-01b serve unchanged.
- `print_in_scene` is the inverse. It is used for the shortest decimal in a unit, and checked by reading back.
- `read` and `print_in` are `read_in` and `print_in_scene` in the default scene.
- The checker learns `Frames`' words. RNA's `time` still maps to a bare number: passing the project's scene is 1c-01c (frames) and 1c-01d (the unit scale).

**Measured.** Four property tests over every unit but degrees, in scenes drawn from:
- scales 1, 0.01, 0.3048, 1000, 10^-30, 3 × 10^30, and anything from 10^-3 to 10^3;
- frame rates from 1 to 239;
- `fps_base` 1, 1.001, and anything from 0.5 to 2.

The four tests:
- each read is the exact rational rounded once. The test computes the expected value apart from the reader, from the scene's floats, rounds it to 130 significant digits, and lets the standard library parse it;
- every f32 and f64 printed in the unit reads back bit for bit;
- a short value prints back exactly as written;
- at scale 1, the scene's arithmetic, forced, gives the plain path's bits.

Each passed 20,000 cases in release. The examples:
- at scale 0.01, `1m`, `1m2`, `1kg`, `1W` and `1m/s` are the exact quotients by the float 0.01, rounded once;
- degrees, lens millimetres, seconds, wavelengths and bare numbers are unscaled;
- `1.5s`, `36` and `1min` are frames at 24 fps;
- `1001s` at 30/1.001 fps is just under 30,000, because of float32's 1.001.
