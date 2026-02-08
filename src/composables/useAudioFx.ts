import { computed, ref, watch, type Ref } from "vue";

const AUDIO_FX_KEY = "audio-fx-preset-v1";

export const AUDIO_FX_PRESETS = [
  { id: "clean", label: "Clean" },
  { id: "cassette", label: "Cassette Deck" },
  { id: "bass", label: "Overdriven Bass" },
  { id: "radio", label: "AM Radio" },
] as const;

export type AudioFxPreset = (typeof AUDIO_FX_PRESETS)[number]["id"];

type AudioFxOptions = {
  storage?: Storage;
  createAudioContext?: () => AudioContext | null;
};

type AudioContextCtor = new () => AudioContext;

function createDefaultAudioContext(): AudioContext | null {
  const globalScope = globalThis as typeof globalThis & {
    AudioContext?: AudioContextCtor;
    webkitAudioContext?: AudioContextCtor;
  };
  const ContextCtor = globalScope.AudioContext ?? globalScope.webkitAudioContext;
  if (!ContextCtor) {
    return null;
  }

  return new ContextCtor();
}

function readAudioFxPreset(storage: Storage): AudioFxPreset {
  try {
    const stored = storage.getItem(AUDIO_FX_KEY);
    if (stored && AUDIO_FX_PRESETS.some((preset) => preset.id === stored)) {
      return stored as AudioFxPreset;
    }
  } catch {
    // Ignore storage access errors and keep default preset.
  }

  return "clean";
}

function createDistortionCurve(amount: number) {
  const samples = 256;
  const curve = new Float32Array(samples);
  const k = typeof amount === "number" ? amount : 0;
  const deg = Math.PI / 180;
  for (let i = 0; i < samples; i += 1) {
    const x = (i * 2) / samples - 1;
    curve[i] = ((3 + k) * x * 20 * deg) / (Math.PI + k * Math.abs(x));
  }
  return curve;
}

export function useAudioFx(audioRef: Ref<HTMLAudioElement | null>, options: AudioFxOptions = {}) {
  const storage = options.storage ?? localStorage;
  const createAudioContext = options.createAudioContext ?? createDefaultAudioContext;

  const audioFxPreset = ref<AudioFxPreset>(readAudioFxPreset(storage));
  const audioContextRef = ref<AudioContext | null>(null);
  const audioSourceRef = ref<MediaElementAudioSourceNode | null>(null);
  const audioNodesRef = ref<AudioNode[]>([]);
  let hasWarnedUnsupportedContext = false;

  const audioFxLabel = computed(() => {
    return AUDIO_FX_PRESETS.find((preset) => preset.id === audioFxPreset.value)?.label ?? "Clean";
  });

  function cycleAudioFxPreset() {
    const currentIndex = AUDIO_FX_PRESETS.findIndex((preset) => preset.id === audioFxPreset.value);
    const nextPreset = AUDIO_FX_PRESETS[(currentIndex + 1) % AUDIO_FX_PRESETS.length];
    audioFxPreset.value = nextPreset.id;
  }

  function ensureAudioContext(): AudioContext | null {
    const audio = audioRef.value;
    if (!audio) {
      return null;
    }

    if (!audioContextRef.value) {
      const context = createAudioContext();
      if (!context) {
        if (!hasWarnedUnsupportedContext) {
          hasWarnedUnsupportedContext = true;
          console.warn("[audio-fx] AudioContext is unavailable in this runtime");
        }
        return null;
      }
      audioContextRef.value = context;
    }

    if (!audioSourceRef.value && audioContextRef.value) {
      try {
        audioSourceRef.value = audioContextRef.value.createMediaElementSource(audio);
      } catch (error) {
        console.warn("[audio-fx] Unable to create media element source node", error);
        return null;
      }
    }

    return audioContextRef.value;
  }

  function clearAudioNodes() {
    for (const node of audioNodesRef.value) {
      try {
        node.disconnect();
      } catch {
        // Ignore disconnect errors.
      }
    }
    audioNodesRef.value = [];
  }

  function refreshAudioFxGraph() {
    const context = ensureAudioContext();
    const source = audioSourceRef.value;
    if (!context || !source) {
      return;
    }

    try {
      source.disconnect();
    } catch {
      // Ignore disconnect errors.
    }
    clearAudioNodes();

    if (audioFxPreset.value === "clean") {
      source.connect(context.destination);
      return;
    }

    const nodes: AudioNode[] = [];
    const lowShelf = context.createBiquadFilter();
    const highPass = context.createBiquadFilter();
    const lowPass = context.createBiquadFilter();
    const mid = context.createBiquadFilter();
    const compressor = context.createDynamicsCompressor();
    const distortion = context.createWaveShaper();
    const makeup = context.createGain();

    switch (audioFxPreset.value) {
      case "cassette": {
        highPass.type = "highpass";
        highPass.frequency.value = 70;
        lowPass.type = "lowpass";
        lowPass.frequency.value = 8500;
        mid.type = "peaking";
        mid.frequency.value = 1200;
        mid.gain.value = -2.5;
        mid.Q.value = 0.8;
        distortion.curve = createDistortionCurve(10);
        distortion.oversample = "2x";
        compressor.threshold.value = -22;
        compressor.ratio.value = 2.6;
        compressor.attack.value = 0.008;
        compressor.release.value = 0.25;
        makeup.gain.value = 1.15;
        nodes.push(highPass, lowPass, mid, distortion, compressor, makeup);
        break;
      }
      case "bass": {
        lowShelf.type = "lowshelf";
        lowShelf.frequency.value = 140;
        lowShelf.gain.value = 8.5;
        mid.type = "peaking";
        mid.frequency.value = 420;
        mid.gain.value = 3.8;
        mid.Q.value = 0.7;
        distortion.curve = createDistortionCurve(22);
        distortion.oversample = "4x";
        compressor.threshold.value = -24;
        compressor.ratio.value = 3.5;
        compressor.attack.value = 0.004;
        compressor.release.value = 0.18;
        makeup.gain.value = 0.95;
        nodes.push(lowShelf, mid, distortion, compressor, makeup);
        break;
      }
      case "radio": {
        highPass.type = "highpass";
        highPass.frequency.value = 180;
        lowPass.type = "lowpass";
        lowPass.frequency.value = 3300;
        mid.type = "peaking";
        mid.frequency.value = 1500;
        mid.gain.value = 2.5;
        mid.Q.value = 1.1;
        distortion.curve = createDistortionCurve(18);
        distortion.oversample = "2x";
        compressor.threshold.value = -28;
        compressor.ratio.value = 4;
        compressor.attack.value = 0.002;
        compressor.release.value = 0.12;
        makeup.gain.value = 0.9;
        nodes.push(highPass, lowPass, mid, distortion, compressor, makeup);
        break;
      }
      default: {
        source.connect(context.destination);
        return;
      }
    }

    if (nodes.length > 0) {
      source.connect(nodes[0]);
      for (let i = 0; i < nodes.length - 1; i += 1) {
        nodes[i]?.connect(nodes[i + 1]);
      }
      nodes[nodes.length - 1]?.connect(context.destination);
      audioNodesRef.value = nodes;
    }
  }

  async function resumeAudioContext() {
    const context = audioContextRef.value;
    if (context && context.state !== "running") {
      await context.resume();
    }
  }

  function teardownAudioFx() {
    clearAudioNodes();
    audioSourceRef.value = null;
    if (audioContextRef.value) {
      void audioContextRef.value.close();
      audioContextRef.value = null;
    }
  }

  watch(audioFxPreset, (value) => {
    try {
      storage.setItem(AUDIO_FX_KEY, value);
    } catch {
      // Ignore storage errors.
    }
    if (audioContextRef.value && audioSourceRef.value) {
      refreshAudioFxGraph();
    }
  });

  return {
    audioFxPreset,
    audioFxLabel,
    audioNodesRef,
    cycleAudioFxPreset,
    ensureAudioContext,
    refreshAudioFxGraph,
    resumeAudioContext,
    teardownAudioFx,
  };
}
