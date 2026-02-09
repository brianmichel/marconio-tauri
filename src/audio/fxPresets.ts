export const AUDIO_FX_PRESETS = [
  { id: "clean", label: "Clean" },
  { id: "cassette", label: "Cassette Deck" },
  { id: "bass", label: "Overdriven Bass" },
  { id: "radio", label: "AM Radio" },
] as const;

export type AudioFxPreset = (typeof AUDIO_FX_PRESETS)[number]["id"];
