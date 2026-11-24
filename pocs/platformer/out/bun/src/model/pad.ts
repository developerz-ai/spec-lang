// Projected by hand from Pad (src/hop/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

/** One frame of controls, sampled once per tick. */
export type Pad = {
  readonly left: boolean;
  readonly right: boolean;
  readonly forward: boolean;
  readonly back: boolean;
  readonly jump: boolean;
};

export const noPad: Pad = Object.freeze({ left: false, right: false, forward: false, back: false, jump: false });

export const pad = (held: Partial<Pad> = {}): Pad => ({ ...noPad, ...held });
