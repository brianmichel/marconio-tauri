import { describe, expect, it } from "vitest";
import { AUDIO_FX_PRESETS } from "../fxPresets";

describe("AUDIO_FX_PRESETS", () => {
  it("exposes the expected ids in order", () => {
    expect(AUDIO_FX_PRESETS.map((preset) => preset.id)).toEqual([
      "clean",
      "cassette",
      "bass",
      "radio",
    ]);
  });

  it("contains labels for each preset", () => {
    for (const preset of AUDIO_FX_PRESETS) {
      expect(typeof preset.label).toBe("string");
      expect(preset.label.length).toBeGreaterThan(0);
    }
  });
});
