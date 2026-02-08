import { nextTick, ref } from "vue";
import { describe, expect, it, vi } from "vitest";
import { useAudioFx } from "../useAudioFx";

class MemoryStorage implements Storage {
  private store = new Map<string, string>();

  get length() {
    return this.store.size;
  }

  clear() {
    this.store.clear();
  }

  getItem(key: string) {
    return this.store.get(key) ?? null;
  }

  key(index: number) {
    return Array.from(this.store.keys())[index] ?? null;
  }

  removeItem(key: string) {
    this.store.delete(key);
  }

  setItem(key: string, value: string) {
    this.store.set(key, value);
  }
}

class FakeNode {
  connections: FakeNode[] = [];

  connect(node: FakeNode) {
    this.connections.push(node);
  }

  disconnect() {
    this.connections = [];
  }
}

class FakeFilterNode extends FakeNode {
  type = "lowpass";
  frequency = { value: 0 };
  gain = { value: 0 };
  Q = { value: 0 };
}

class FakeDynamicsCompressorNode extends FakeNode {
  threshold = { value: 0 };
  ratio = { value: 0 };
  attack = { value: 0 };
  release = { value: 0 };
}

class FakeWaveShaperNode extends FakeNode {
  curve: Float32Array | null = null;
  oversample: OverSampleType = "none";
}

class FakeGainNode extends FakeNode {
  gain = { value: 0 };
}

class FakeAudioContext {
  state: AudioContextState = "suspended";
  destination = new FakeNode();
  lastSource: FakeNode | null = null;

  createBiquadFilter() {
    return new FakeFilterNode() as unknown as BiquadFilterNode;
  }

  createDynamicsCompressor() {
    return new FakeDynamicsCompressorNode() as unknown as DynamicsCompressorNode;
  }

  createWaveShaper() {
    return new FakeWaveShaperNode() as unknown as WaveShaperNode;
  }

  createGain() {
    return new FakeGainNode() as unknown as GainNode;
  }

  createMediaElementSource() {
    const source = new FakeNode();
    this.lastSource = source;
    return source as unknown as MediaElementAudioSourceNode;
  }

  resume() {
    this.state = "running";
    return Promise.resolve();
  }

  close() {
    this.state = "closed";
    return Promise.resolve();
  }
}

class ThrowingMediaSourceAudioContext extends FakeAudioContext {
  override createMediaElementSource(): MediaElementAudioSourceNode {
    throw new DOMException("Not allowed", "InvalidStateError");
  }
}

describe("useAudioFx", () => {
  it("reads and cycles presets with storage updates", async () => {
    const storage = new MemoryStorage();
    storage.setItem("audio-fx-preset-v1", "radio");
    const audioRef = ref<HTMLAudioElement | null>(document.createElement("audio"));
    const context = new FakeAudioContext();

    const { audioFxPreset, audioFxLabel, cycleAudioFxPreset } = useAudioFx(audioRef, {
      storage,
      createAudioContext: () => context as unknown as AudioContext,
    });

    expect(audioFxPreset.value).toBe("radio");
    expect(audioFxLabel.value).toBe("AM Radio");

    cycleAudioFxPreset();
    await nextTick();
    expect(audioFxPreset.value).toBe("clean");
    expect(storage.getItem("audio-fx-preset-v1")).toBe("clean");
  });

  it("builds a non-clean audio graph and connects output", () => {
    const storage = new MemoryStorage();
    storage.setItem("audio-fx-preset-v1", "cassette");
    const audioRef = ref<HTMLAudioElement | null>(document.createElement("audio"));
    const context = new FakeAudioContext();

    const { audioNodesRef, ensureAudioContext, refreshAudioFxGraph } = useAudioFx(audioRef, {
      storage,
      createAudioContext: () => context as unknown as AudioContext,
    });

    ensureAudioContext();
    refreshAudioFxGraph();

    expect(audioNodesRef.value.length).toBeGreaterThan(0);
    expect(context.lastSource?.connections.length).toBeGreaterThan(0);
    expect(context.lastSource?.connections[0]).toStrictEqual(audioNodesRef.value[0]);
  });

  it("passes through clean audio without extra nodes", () => {
    const storage = new MemoryStorage();
    const audioRef = ref<HTMLAudioElement | null>(document.createElement("audio"));
    const context = new FakeAudioContext();

    const { audioFxPreset, audioNodesRef, ensureAudioContext, refreshAudioFxGraph } = useAudioFx(
      audioRef,
      {
        storage,
        createAudioContext: () => context as unknown as AudioContext,
      },
    );

    audioFxPreset.value = "clean";
    ensureAudioContext();
    refreshAudioFxGraph();

    expect(audioNodesRef.value.length).toBe(0);
    expect(context.lastSource?.connections[0]).toBe(context.destination);
  });

  it("falls back to webkitAudioContext when AudioContext is unavailable", () => {
    const storage = new MemoryStorage();
    const audioRef = ref<HTMLAudioElement | null>(document.createElement("audio"));
    const globalScope = globalThis as Record<string, unknown>;
    const originalAudioContext = globalScope.AudioContext;
    const originalWebkitAudioContext = globalScope.webkitAudioContext;

    class FakeWebkitAudioContext extends FakeAudioContext {}

    try {
      globalScope.AudioContext = undefined;
      globalScope.webkitAudioContext = FakeWebkitAudioContext as unknown;

      const { ensureAudioContext } = useAudioFx(audioRef, { storage });
      const context = ensureAudioContext();

      expect(context).not.toBeNull();
      expect(context).toBeInstanceOf(FakeWebkitAudioContext);
    } finally {
      globalScope.AudioContext = originalAudioContext;
      globalScope.webkitAudioContext = originalWebkitAudioContext;
    }
  });

  it("gracefully handles media source creation errors", () => {
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const storage = new MemoryStorage();
    const audioRef = ref<HTMLAudioElement | null>(document.createElement("audio"));
    const context = new ThrowingMediaSourceAudioContext();

    const { ensureAudioContext, refreshAudioFxGraph, audioNodesRef } = useAudioFx(audioRef, {
      storage,
      createAudioContext: () => context as unknown as AudioContext,
    });

    expect(ensureAudioContext()).toBeNull();
    expect(() => refreshAudioFxGraph()).not.toThrow();
    expect(audioNodesRef.value.length).toBe(0);
    expect(warnSpy).toHaveBeenCalled();

    warnSpy.mockRestore();
  });
});
