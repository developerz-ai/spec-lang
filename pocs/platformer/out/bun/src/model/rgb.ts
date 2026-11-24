// Projected by hand from Rgb (src/hop/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

/** Each channel is `0..=255`. */
export type Rgb = { readonly r: number; readonly g: number; readonly b: number };

export const rgb = (r: number, g: number, b: number): Rgb => ({ r, g, b });
