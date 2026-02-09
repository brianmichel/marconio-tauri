export const AUDIO_FX_PRESETS = [
  { id: "clean", label: "Clean" },
  { id: "cassette", label: "Mag" },
  { id: "bass", label: "Bass+" },
  { id: "radio", label: "Radio" },
] as const;

export type AudioFxPreset = (typeof AUDIO_FX_PRESETS)[number]["id"];
