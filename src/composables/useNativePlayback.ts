import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { onBeforeUnmount, onMounted, ref, type Ref } from "vue";
import type { MediaPlayable } from "../nts";

type NativeMediaControlAction = "play" | "pause" | "stop" | "toggle";

type NativeMediaControlPayload = {
  action: NativeMediaControlAction;
};

type UseNativePlaybackOptions = {
  canUseTauriInvoke: () => boolean;
  currentPlayable: Ref<MediaPlayable | null>;
  activeSlot: Ref<number | null>;
  errorMessage: Ref<string | null>;
  isLcdTuning: Ref<boolean>;
};

export function useNativePlayback(options: UseNativePlaybackOptions) {
  const isPlaying = ref(false);
  let unlistenNativeMediaControl: (() => void) | null = null;

  function nowPlayingFromPlayable(playable: MediaPlayable) {
    const album = playable.source.kind === "channel"
      ? `NTS ${playable.source.value.channelName}`
      : "NTS Mixtape";

    return {
      title: playable.title,
      artist: playable.subtitle ?? "NTS Radio",
      album,
      artworkUrl: playable.artworkUrl,
    };
  }

  async function startPlayback(playable: MediaPlayable, slot: number) {
    options.isLcdTuning.value = true;
    setTimeout(() => {
      options.isLcdTuning.value = false;
    }, 400);
    options.currentPlayable.value = playable;
    options.activeSlot.value = slot;
    options.errorMessage.value = null;

    if (!options.canUseTauriInvoke()) {
      options.errorMessage.value = "Native playback requires a Tauri runtime.";
      return;
    }

    try {
      await invoke("start_native_stream", {
        streamUrl: playable.streamUrl,
        nowPlaying: nowPlayingFromPlayable(playable),
      });
      isPlaying.value = true;
    } catch (error) {
      const message = error instanceof Error ? error.message : "Unknown error";
      options.errorMessage.value = `Unable to start playback: ${message}`;
      isPlaying.value = false;
    }
  }

  async function stopPlayback() {
    if (!options.canUseTauriInvoke()) {
      isPlaying.value = false;
      return;
    }

    try {
      await invoke("stop_native_stream");
    } catch (error) {
      console.warn("[audio] Unable to stop native playback", error);
    } finally {
      isPlaying.value = false;
    }
  }

  async function handleNativeMediaControl(action: NativeMediaControlAction) {
    if (action === "pause" || action === "stop") {
      if (isPlaying.value) {
        await stopPlayback();
      }
      return;
    }

    if (!options.currentPlayable.value || options.activeSlot.value === null) {
      return;
    }

    if (action === "play") {
      if (!isPlaying.value) {
        await startPlayback(options.currentPlayable.value, options.activeSlot.value);
      }
      return;
    }

    if (action === "toggle") {
      if (isPlaying.value) {
        await stopPlayback();
        return;
      }
      await startPlayback(options.currentPlayable.value, options.activeSlot.value);
    }
  }

  onMounted(async () => {
    if (!options.canUseTauriInvoke()) {
      return;
    }

    try {
      unlistenNativeMediaControl = await listen<NativeMediaControlPayload>(
        "native-media-control",
        (event) => {
          void handleNativeMediaControl(event.payload.action);
        },
      );
    } catch (error) {
      console.warn("[audio] Unable to listen for native media controls", error);
    }
  });

  onBeforeUnmount(() => {
    if (options.canUseTauriInvoke()) {
      void invoke("stop_native_stream").catch(() => {
        // Ignore cleanup errors.
      });
    }
    if (unlistenNativeMediaControl) {
      unlistenNativeMediaControl();
      unlistenNativeMediaControl = null;
    }
  });

  return {
    isPlaying,
    startPlayback,
    stopPlayback,
  };
}
