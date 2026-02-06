<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import {
  createNTSClient,
  playableFromChannel,
  playableFromMixtape,
  type MediaPlayable,
} from "./nts";

const client = createNTSClient();
const channels = ref<MediaPlayable[]>([]);
const mixtapes = ref<MediaPlayable[]>([]);
const isLoading = ref(false);
const errorMessage = ref<string | null>(null);
const currentPlayable = ref<MediaPlayable | null>(null);
const isPlaying = ref(false);
const audioRef = ref<HTMLAudioElement | null>(null);

let controller: AbortController | null = null;

function log(message: string, extra?: unknown) {
  if (typeof extra === "undefined") {
    console.info(`[app] ${message}`);
    return;
  }
  console.info(`[app] ${message}`, extra);
}

async function loadPlayableMedia() {
  log("loadPlayableMedia start");
  controller?.abort();
  controller = new AbortController();

  isLoading.value = true;
  errorMessage.value = null;

  try {
    const [live, mixtapeData] = await Promise.all([
      client.live({ signal: controller.signal }),
      client.mixtapes({ signal: controller.signal }),
    ]);
    log("API requests completed", {
      liveResults: live.results.length,
      mixtapeResults: mixtapeData.results.length,
    });

    channels.value = live.results
      .map(playableFromChannel)
      .filter((item): item is MediaPlayable => item !== null);

    mixtapes.value = mixtapeData.results.map(playableFromMixtape);
    log("mapped playables", {
      channels: channels.value.length,
      mixtapes: mixtapes.value.length,
    });
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") {
      log("load aborted");
      return;
    }

    log("loadPlayableMedia failed", error);
    const message = error instanceof Error ? error.message : "Unknown error";
    errorMessage.value = `Failed to load NTS streams: ${message}`;
  } finally {
    isLoading.value = false;
    log("loadPlayableMedia end", { isLoading: isLoading.value });
  }
}

async function startPlayback(playable: MediaPlayable) {
  log("startPlayback", {
    id: playable.id,
    streamUrl: playable.streamUrl,
  });
  currentPlayable.value = playable;

  const audio = audioRef.value;
  if (!audio) {
    return;
  }

  audio.src = playable.streamUrl;

  try {
    await audio.play();
    isPlaying.value = true;
    log("playback started");
  } catch (error) {
    log("playback failed", error);
    const message = error instanceof Error ? error.message : "Unknown error";
    errorMessage.value = `Unable to start playback: ${message}`;
    isPlaying.value = false;
  }
}

function stopPlayback() {
  const audio = audioRef.value;
  if (!audio) {
    return;
  }

  audio.pause();
  isPlaying.value = false;
  log("playback stopped");
}

onMounted(() => {
  log("mounted");
  loadPlayableMedia();
});

onBeforeUnmount(() => {
  log("before unmount");
  controller?.abort();
  const audio = audioRef.value;
  if (audio) {
    audio.pause();
    audio.src = "";
  }
});
</script>

<template>
  <main class="app">
    <header class="app-header">
      <h1>NTS.live Player</h1>
      <button type="button" :disabled="isLoading" @click="loadPlayableMedia">
        {{ isLoading ? "Loading..." : "Refresh" }}
      </button>
    </header>

    <p v-if="errorMessage" class="error">{{ errorMessage }}</p>

    <section class="now-playing" v-if="currentPlayable">
      <img :src="currentPlayable.artworkUrl" :alt="currentPlayable.title" />
      <div>
        <h2>{{ currentPlayable.title }}</h2>
        <p>{{ currentPlayable.subtitle }}</p>
        <a :href="currentPlayable.pageUrl" target="_blank" rel="noreferrer">
          Open on NTS.live
        </a>
      </div>
      <button type="button" @click="stopPlayback" :disabled="!isPlaying">Stop</button>
    </section>

    <section>
      <h2>Live Channels</h2>
      <ul>
        <li v-for="item in channels" :key="item.id">
          <div>
            <strong>{{ item.title }}</strong>
            <p>{{ item.subtitle }}</p>
          </div>
          <button type="button" @click="startPlayback(item)">Play</button>
        </li>
      </ul>
    </section>

    <section>
      <h2>Mixtapes</h2>
      <ul>
        <li v-for="item in mixtapes" :key="item.id">
          <div>
            <strong>{{ item.title }}</strong>
            <p>{{ item.subtitle }}</p>
          </div>
          <button type="button" @click="startPlayback(item)">Play</button>
        </li>
      </ul>
    </section>

    <audio
      ref="audioRef"
      controls
      preload="none"
      @pause="isPlaying = false"
      @play="isPlaying = true"
    />
  </main>
</template>

<style scoped>
.app {
  max-width: 960px;
  margin: 0 auto;
  padding: 1.5rem;
  color: #1f2937;
  font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI",
    sans-serif;
}

.app-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.error {
  color: #b91c1c;
}

.now-playing {
  display: grid;
  grid-template-columns: 140px 1fr auto;
  gap: 1rem;
  align-items: center;
  border: 1px solid #d1d5db;
  border-radius: 12px;
  padding: 1rem;
  margin: 1rem 0;
}

.now-playing img {
  width: 140px;
  height: 140px;
  object-fit: cover;
  border-radius: 10px;
}

ul {
  list-style: none;
  padding: 0;
  margin: 0;
}

li {
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid #e5e7eb;
  padding: 0.75rem 0;
}

button {
  border: 1px solid #cbd5e1;
  background: white;
  border-radius: 8px;
  padding: 0.5rem 0.85rem;
  cursor: pointer;
}

button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

audio {
  width: 100%;
  margin-top: 1rem;
}

@media (max-width: 720px) {
  .now-playing {
    grid-template-columns: 1fr;
  }
}
</style>
