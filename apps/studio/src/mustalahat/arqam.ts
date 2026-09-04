/**
 * Floating-point values as they arrive from Rust.
 *
 * `f32` and `f64` cross the boundary as `number | null`, and the `null` is not
 * optionality — it is the JSON form of `NaN` and `±Infinity`, which have no
 * literal in the format. `JSON.stringify(NaN)` is `"null"` and serde does the
 * same, so the generated bindings are telling the truth: a float field can
 * arrive as null.
 *
 * In this product it does not. Every ratio is produced by a helper that returns
 * a defined value for a zero denominator, and every opacity is validated into
 * `0..=1` before it is stored. So `null` here means a guarantee was broken
 * somewhere upstream, and the interface's job is to keep rendering rather than
 * to throw inside a sort comparator or a formatter.
 *
 * Hand-written. `awamir.ts` beside this file is generated output and nothing
 * edits it; this is where the decisions the generator cannot express live.
 */

/**
 * A float off the wire, with the neutral value to fall back to.
 *
 * The caller names the neutral because it differs by measurement: zero for a
 * count or a ratio, one for a scale, the middle for a bounded setting.
 *
 * @param qeema the value as the bindings type it
 * @param badil what to use when the value did not survive serialization
 */
export function kasr(qeema: number | null | undefined, badil = 0): number {
  return qeema ?? badil;
}
