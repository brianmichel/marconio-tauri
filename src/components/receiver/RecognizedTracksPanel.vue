<script setup lang="ts">
import { computed } from "vue";
import type { RecognizedTrack } from "../../shazam/types";

const props = defineProps<{
  visible: boolean;
  tracks: RecognizedTrack[];
  isListening: boolean;
}>();

const emit = defineEmits<{
  close: [];
  clear: [];
}>();

const hasTracks = computed(() => props.tracks.length > 0);

function formatTimestamp(epochSeconds: number) {
  const date = new Date(epochSeconds * 1000);
  if (Number.isNaN(date.valueOf())) {
    return "Unknown time";
  }
  return date.toLocaleString();
}
</script>

<template>
  <div v-if="visible" class="recognized-backdrop" @mousedown="emit('close')" />
  <section
    v-if="visible"
    class="recognized-panel"
    role="dialog"
    aria-modal="true"
    aria-label="Recognized tracks"
    @mousedown.stop
  >
    <header class="recognized-panel-header">
      <div class="recognized-panel-title">
        <h2>Recognized Tracks</h2>
        <p v-if="isListening">Listening for a match...</p>
        <p v-else>{{ tracks.length }} saved</p>
      </div>
      <button
        type="button"
        class="recognized-panel-close"
        aria-label="Close recognized tracks"
        @click="emit('close')"
      >
        CLOSE
      </button>
    </header>

    <div v-if="!hasTracks" class="recognized-empty">
      <p>No recognized tracks yet.</p>
      <p class="hint">Press FIND SONG while audio is playing.</p>
    </div>

    <ul v-else class="recognized-list">
      <li v-for="track in tracks" :key="`${track.shazamId ?? track.title}-${track.recognizedAt}`" class="recognized-item">
        <img
          v-if="track.artworkUrl"
          class="recognized-artwork"
          :src="track.artworkUrl"
          alt=""
          loading="lazy"
        >
        <div class="recognized-item-body">
          <p class="recognized-item-title">{{ track.title }}</p>
          <p class="recognized-item-artist">{{ track.artist ?? "Unknown artist" }}</p>
          <p class="recognized-item-meta">
            {{ formatTimestamp(track.recognizedAt) }}
            <template v-if="track.sourceTitle">
              · {{ track.sourceTitle }}
            </template>
          </p>
          <div class="recognized-item-links">
            <a v-if="track.webUrl" :href="track.webUrl" target="_blank" rel="noopener noreferrer">Shazam</a>
            <a
              v-if="track.appleMusicUrl"
              :href="track.appleMusicUrl"
              target="_blank"
              rel="noopener noreferrer"
            >
              Apple Music
            </a>
          </div>
        </div>
      </li>
    </ul>

    <footer class="recognized-panel-footer">
      <button
        type="button"
        class="recognized-panel-clear"
        :disabled="!hasTracks"
        @click="emit('clear')"
      >
        Clear History
      </button>
    </footer>
  </section>
</template>

<style scoped>
.recognized-backdrop {
  position: absolute;
  inset: 0;
  z-index: 150;
  background: rgba(6, 10, 14, 0.68);
  backdrop-filter: blur(3px);
}

.recognized-panel {
  position: absolute;
  inset: 34px 14px 16px;
  z-index: 160;
  border: 1px solid #3a4046;
  border-radius: 7px;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.05) 0%, transparent 30%),
    linear-gradient(180deg, #252b31 0%, #1b2025 100%);
  box-shadow:
    0 14px 28px rgba(0, 0, 0, 0.5),
    inset 0 1px 0 rgba(255, 255, 255, 0.08);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.recognized-panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 11px 8px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.recognized-panel-title h2 {
  margin: 0;
  font-size: 11px;
  letter-spacing: 0.09em;
  text-transform: uppercase;
  color: #e0e6ed;
  font-family: var(--display-font);
}

.recognized-panel-title p {
  margin: 2px 0 0;
  color: #8f98a3;
  font-size: 10px;
}

.recognized-panel-close {
  -webkit-appearance: none;
  appearance: none;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.04);
  color: #b3bac4;
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.08em;
  padding: 5px 8px;
  cursor: pointer;
  text-transform: uppercase;
}

.recognized-panel-close:hover {
  background: rgba(255, 255, 255, 0.08);
}

.recognized-panel-close:focus-visible {
  outline: 1px solid var(--theme-focus-outline);
  outline-offset: 1px;
}

.recognized-empty {
  margin: auto;
  text-align: center;
  color: #8f98a3;
  font-size: 11px;
}

.recognized-empty .hint {
  margin-top: 4px;
  opacity: 0.8;
}

.recognized-list {
  margin: 0;
  padding: 8px 10px;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow: auto;
}

.recognized-item {
  display: flex;
  gap: 8px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 5px;
  padding: 6px;
  background: rgba(255, 255, 255, 0.03);
}

.recognized-artwork {
  width: 38px;
  height: 38px;
  border-radius: 3px;
  object-fit: cover;
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.recognized-item-body {
  flex: 1;
  min-width: 0;
}

.recognized-item-title {
  margin: 0;
  color: #edf2f7;
  font-size: 11px;
  font-weight: 700;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.recognized-item-artist {
  margin: 1px 0 0;
  color: #a6b0bc;
  font-size: 10px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.recognized-item-meta {
  margin: 2px 0 0;
  color: #7d8793;
  font-size: 9px;
}

.recognized-item-links {
  margin-top: 4px;
  display: flex;
  gap: 8px;
}

.recognized-item-links a {
  color: #8ebaf0;
  font-size: 9px;
  text-decoration: none;
}

.recognized-item-links a:hover {
  text-decoration: underline;
}

.recognized-panel-footer {
  padding: 8px 10px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
}

.recognized-panel-clear {
  -webkit-appearance: none;
  appearance: none;
  width: 100%;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.03);
  color: #c2cad3;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.07em;
  padding: 6px 0;
  cursor: pointer;
  text-transform: uppercase;
}

.recognized-panel-clear:hover:enabled {
  background: rgba(255, 255, 255, 0.08);
}

.recognized-panel-clear:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.recognized-panel-clear:focus-visible {
  outline: 1px solid var(--theme-focus-outline);
  outline-offset: 1px;
}
</style>
