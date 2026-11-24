// Projected by hand from triangle, approach (src/hop/geom.slg). Stand-in for spec-lang build output; change the spec, not this file.

/** Back and forth without division: 0 up to n, then down to 0, period 2n. `mod` is floored; `t >= 0` here. */
export function triangle(t: number, n: number): number {
  const period = 2 * n;
  const p = ((t % period) + period) % period;
  return p <= n ? p : period - p;
}

export const approach = (v: number, target: number, step: number): number =>
  v < target ? Math.min(v + step, target) : Math.max(v - step, target);
