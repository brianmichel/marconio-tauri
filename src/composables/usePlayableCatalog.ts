import { computed, onBeforeUnmount, ref, watch, type Ref } from "vue";
import {
  createNTSClient,
  playableFromChannel,
  playableFromMixtape,
  type MediaPlayable,
} from "../nts";
import {
  calculateChannelRefreshDelay,
  syncChannelPlayableFromLive,
} from "../nts/channelRefresh";

const USER_SLOTS = [3, 4, 5, 6] as const;

type UsePlayableCatalogOptions = {
  assignments: Ref<Record<3 | 4 | 5 | 6, string | null>>;
  currentPlayable: Ref<MediaPlayable | null>;
  isPlaying: Ref<boolean>;
  errorMessage: Ref<string | null>;
  getOpenContextSlot: () => 3 | 4 | 5 | 6 | null;
  closeContextMenu: () => void;
};

export function usePlayableCatalog(options: UsePlayableCatalogOptions) {
  const client = createNTSClient();
  const channels = ref<MediaPlayable[]>([]);
  const mixtapes = ref<MediaPlayable[]>([]);
  const isLoading = ref(false);
  const mixtapeByAlias = computed(() => {
    const map = new Map<string, MediaPlayable>();
    for (const item of mixtapes.value) {
      if (item.source.kind === "mixtape") {
        map.set(item.source.value.mixtapeAlias, item);
      }
    }
    return map;
  });

  let controller: AbortController | null = null;
  let channelRefreshTimer: ReturnType<typeof setTimeout> | null = null;

  function normalizeAssignments() {
    let changed = false;

    for (const slot of USER_SLOTS) {
      const alias = options.assignments.value[slot];
      if (alias && !mixtapeByAlias.value.has(alias)) {
        options.assignments.value[slot] = null;
        changed = true;
      }
    }

    const openSlot = options.getOpenContextSlot();
    if (changed && openSlot && !options.assignments.value[openSlot]) {
      options.closeContextMenu();
    }
  }

  async function loadPlayableMedia() {
    controller?.abort();
    controller = new AbortController();

    isLoading.value = true;
    options.errorMessage.value = null;

    try {
      const [live, mixtapeData] = await Promise.all([
        client.live({ signal: controller.signal }),
        client.mixtapes({ signal: controller.signal }),
      ]);

      channels.value = live.results
        .map(playableFromChannel)
        .filter((item): item is MediaPlayable => item !== null);

      mixtapes.value = mixtapeData.results.map(playableFromMixtape);
      normalizeAssignments();
      options.currentPlayable.value = syncChannelPlayableFromLive(
        options.currentPlayable.value,
        channels.value,
      );
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") {
        return;
      }

      const message = error instanceof Error ? error.message : "Unknown error";
      options.errorMessage.value = `Failed to load NTS streams: ${message}`;
    } finally {
      isLoading.value = false;
    }
  }

  function clearChannelRefreshTimer() {
    if (channelRefreshTimer) {
      clearTimeout(channelRefreshTimer);
      channelRefreshTimer = null;
    }
  }

  function scheduleChannelRefresh() {
    clearChannelRefreshTimer();

    if (!options.isPlaying.value || options.currentPlayable.value?.source.kind !== "channel") {
      return;
    }

    const delay = calculateChannelRefreshDelay(options.currentPlayable.value.source.value);

    channelRefreshTimer = setTimeout(() => {
      void loadPlayableMedia();
    }, delay);
  }

  watch([options.currentPlayable, options.isPlaying], () => {
    scheduleChannelRefresh();
  });

  onBeforeUnmount(() => {
    controller?.abort();
    clearChannelRefreshTimer();
  });

  return {
    channels,
    mixtapes,
    mixtapeByAlias,
    isLoading,
    loadPlayableMedia,
  };
}
