<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  createNTSClient,
  playableFromChannel,
  playableFromMixtape,
  type MediaPlayable,
} from "./nts";

const client = createNTSClient();
const STORAGE_KEY = "nts-user-presets-v1";
const USER_SLOTS = [3, 4, 5, 6] as const;

type UserSlot = (typeof USER_SLOTS)[number];
type PresetAssignments = Record<UserSlot, string | null>;

const channels = ref<MediaPlayable[]>([]);
const mixtapes = ref<MediaPlayable[]>([]);
const isLoading = ref(false);
const errorMessage = ref<string | null>(null);
const currentPlayable = ref<MediaPlayable | null>(null);
const activeSlot = ref<number | null>(null);
const isPlaying = ref(false);
const audioRef = ref<HTMLAudioElement | null>(null);

const contextMenu = ref<{
  visible: boolean;
  slot: UserSlot | null;
  x: number;
  y: number;
}>({
  visible: false,
  slot: null,
  x: 0,
  y: 0,
});

let controller: AbortController | null = null;

function readAssignments(): PresetAssignments {
  const defaults: PresetAssignments = {
    3: null,
    4: null,
    5: null,
    6: null,
  };

  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return defaults;
    }

    const parsed = JSON.parse(raw) as Partial<PresetAssignments>;
    return {
      3: typeof parsed[3] === "string" ? parsed[3] : null,
      4: typeof parsed[4] === "string" ? parsed[4] : null,
      5: typeof parsed[5] === "string" ? parsed[5] : null,
      6: typeof parsed[6] === "string" ? parsed[6] : null,
    };
  } catch {
    return defaults;
  }
}

const assignments = ref<PresetAssignments>(readAssignments());

watch(
  assignments,
  (value) => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
  },
  { deep: true },
);

const channelOne = computed(
  () =>
    channels.value.find(
      (item) => item.source.kind === "channel" && item.source.value.channelName === "1",
    ) ?? null,
);

const channelTwo = computed(
  () =>
    channels.value.find(
      (item) => item.source.kind === "channel" && item.source.value.channelName === "2",
    ) ?? null,
);

const mixtapeByAlias = computed(() => {
  const map = new Map<string, MediaPlayable>();
  for (const item of mixtapes.value) {
    if (item.source.kind === "mixtape") {
      map.set(item.source.value.mixtapeAlias, item);
    }
  }
  return map;
});

const mixtapeOptions = computed(() =>
  mixtapes.value
    .filter(
      (item): item is MediaPlayable & { source: { kind: "mixtape"; value: { mixtapeAlias: string } } } =>
        item.source.kind === "mixtape",
    )
    .map((item) => ({
      alias: item.source.value.mixtapeAlias,
      title: item.title,
    })),
);

const lcdPrimary = computed(() => {
  if (currentPlayable.value) {
    return currentPlayable.value.title.toUpperCase();
  }
  return isLoading.value ? "TUNING RECEIVER" : "SELECT A PRESET";
});

const lcdSecondary = computed(() => {
  if (errorMessage.value) {
    return "SIGNAL ERROR";
  }

  if (currentPlayable.value?.subtitle) {
    return currentPlayable.value.subtitle.toUpperCase();
  }

  if (currentPlayable.value) {
    return "LIVE STREAM";
  }

  return "NTS BROADCAST UNIT";
});

const presetCards = computed(() => {
  return [1, 2, 3, 4, 5, 6].map((slot) => {
    if (slot === 1) {
      return {
        slot,
        locked: true,
        playable: channelOne.value,
      };
    }

    if (slot === 2) {
      return {
        slot,
        locked: true,
        playable: channelTwo.value,
      };
    }

    const assignedAlias = assignments.value[slot as UserSlot];
    const playable = assignedAlias ? (mixtapeByAlias.value.get(assignedAlias) ?? null) : null;

    return {
      slot,
      locked: false,
      playable,
    };
  });
});

function normalizeAssignments() {
  let changed = false;

  for (const slot of USER_SLOTS) {
    const alias = assignments.value[slot];
    if (alias && !mixtapeByAlias.value.has(alias)) {
      assignments.value[slot] = null;
      changed = true;
    }
  }

  if (changed && contextMenu.value.slot && !assignments.value[contextMenu.value.slot]) {
    closeContextMenu();
  }
}

async function loadPlayableMedia() {
  controller?.abort();
  controller = new AbortController();

  isLoading.value = true;
  errorMessage.value = null;

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
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") {
      return;
    }

    const message = error instanceof Error ? error.message : "Unknown error";
    errorMessage.value = `Failed to load NTS streams: ${message}`;
  } finally {
    isLoading.value = false;
  }
}

async function startPlayback(playable: MediaPlayable, slot: number) {
  currentPlayable.value = playable;
  activeSlot.value = slot;
  errorMessage.value = null;

  const audio = audioRef.value;
  if (!audio) {
    return;
  }

  audio.src = playable.streamUrl;

  try {
    await audio.play();
    isPlaying.value = true;
  } catch (error) {
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
}

function closeContextMenu() {
  contextMenu.value.visible = false;
  contextMenu.value.slot = null;
}

function openContextMenu(event: MouseEvent, slot: UserSlot) {
  const menuWidth = 196;
  const rowCount = mixtapeOptions.value.length + (assignments.value[slot] ? 1 : 0);
  const menuHeight = Math.max(34, rowCount * 26 + 8);
  const padding = 6;

  contextMenu.value.slot = slot;
  contextMenu.value.x = Math.max(
    padding,
    Math.min(event.clientX, window.innerWidth - menuWidth - padding),
  );
  contextMenu.value.y = Math.max(
    padding,
    Math.min(event.clientY, window.innerHeight - menuHeight - padding),
  );
  contextMenu.value.visible = true;
}

function assignSlotFromMenu(alias: string) {
  const slot = contextMenu.value.slot;
  if (!slot) {
    return;
  }

  assignments.value[slot] = alias;
  const playable = mixtapeByAlias.value.get(alias) ?? null;

  closeContextMenu();

  if (playable) {
    void startPlayback(playable, slot);
  }
}

function clearUserSlot(slot: UserSlot) {
  assignments.value[slot] = null;
  closeContextMenu();
  if (activeSlot.value === slot) {
    stopPlayback();
    activeSlot.value = null;
    currentPlayable.value = null;
  }
}

function onPresetPress(slot: number) {
  const card = presetCards.value.find((item) => item.slot === slot);
  if (!card) {
    return;
  }

  if (card.playable) {
    void startPlayback(card.playable, slot);
    return;
  }
}

function onPresetContextMenu(event: MouseEvent, slot: number, locked: boolean) {
  if (locked || slot < 3 || slot > 6) {
    return;
  }

  openContextMenu(event, slot as UserSlot);
}

async function startWindowDrag() {
  try {
    await getCurrentWindow().startDragging();
  } catch {
    // Ignore drag errors (e.g. non-tauri context).
  }
}

onMounted(() => {
  window.addEventListener("keydown", onEscapeKeyDown);
  loadPlayableMedia();
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onEscapeKeyDown);
  controller?.abort();
  const audio = audioRef.value;
  if (audio) {
    audio.pause();
    audio.src = "";
  }
});

function onEscapeKeyDown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    closeContextMenu();
  }
}
</script>

<template>
  <main class="scene">
    <div class="unit">
      <div
        class="drag-strip"
        data-tauri-drag-region
        role="presentation"
        @mousedown.left="startWindowDrag"
      />
      <header class="unit-header" data-tauri-drag-region>
        <p class="brand">MARCONIO</p>
      </header>

      <section class="lcd" aria-live="polite">
        <p class="lcd-main">{{ lcdPrimary }}</p>
        <p class="lcd-sub">{{ lcdSecondary }}</p>
        <div class="lcd-meta">
          <span>{{ isPlaying ? "PLAY" : "STOP" }}</span>
          <span v-if="errorMessage">FAULT</span>
          <span v-else>{{ isLoading ? "SYNC" : "READY" }}</span>
        </div>
      </section>

      <section class="preset-grid" aria-label="Preset selector">
        <article
          v-for="card in presetCards"
          :key="card.slot"
          class="preset-card"
          :class="{
            active: activeSlot === card.slot,
            locked: card.locked,
            empty: !card.playable,
          }"
        >
          <button
            type="button"
            class="preset-button"
            @click="onPresetPress(card.slot)"
            @contextmenu.prevent="onPresetContextMenu($event, card.slot, card.locked)"
          >
            <span class="slot-number">{{ card.slot }}</span>
            <span class="slot-title">
              {{ card.playable ? card.playable.title : card.locked ? "OFF AIR" : "ASSIGN" }}
            </span>
            <span class="slot-subtitle">
              {{
                card.locked
                  ? "CH"
                  : card.playable
                    ? card.playable.subtitle || "MIXTAPE"
                    : "RIGHT CLICK TO ASSIGN"
              }}
            </span>
          </button>
        </article>
      </section>

      <footer class="unit-footer">
        <p class="tagline">STREAMING RECEIVER</p>
        <div class="footer-controls">
          <button type="button" class="footer-btn" :disabled="isLoading" @click="loadPlayableMedia">
            {{ isLoading ? "SYNCING" : "REFRESH" }}
          </button>
          <button type="button" class="footer-btn" :disabled="!isPlaying" @click="stopPlayback">
            STOP
          </button>
          <p class="model">MRC-1900</p>
        </div>
      </footer>

      <audio ref="audioRef" preload="none" @pause="isPlaying = false" @play="isPlaying = true" />

      <div
        v-if="contextMenu.visible"
        class="context-backdrop"
        @mousedown="closeContextMenu"
        @contextmenu.prevent="closeContextMenu"
      />
      <section
        v-if="contextMenu.visible && contextMenu.slot !== null"
        class="context-menu"
        :style="{ left: `${contextMenu.x}px`, top: `${contextMenu.y}px` }"
        @mousedown.stop
        @contextmenu.prevent
      >
        <p class="context-title">Preset {{ contextMenu.slot }}</p>
        <button
          v-for="option in mixtapeOptions"
          :key="option.alias"
          type="button"
          class="context-item"
          @click="assignSlotFromMenu(option.alias)"
        >
          {{ option.title }}
        </button>
        <button
          v-if="assignments[contextMenu.slot]"
          type="button"
          class="context-item danger"
          @click="clearUserSlot(contextMenu.slot)"
        >
          Clear Preset
        </button>
      </section>
    </div>
  </main>
</template>

<style scoped>
:global(html),
:global(body),
:global(#app) {
  width: 100%;
  height: 100%;
  margin: 0;
}

:global(*),
:global(*::before),
:global(*::after) {
  box-sizing: border-box;
}

:global(body) {
  background: #111315;
  overflow: hidden;
  font-family:
    "SF Pro Text",
    -apple-system,
    BlinkMacSystemFont,
    "Helvetica Neue",
    sans-serif;
}

.scene {
  width: 100vw;
  height: 100vh;
}

.unit {
  --ui-font:
    "SF Pro Text",
    -apple-system,
    BlinkMacSystemFont,
    "Helvetica Neue",
    sans-serif;
  --display-font:
    "SF Pro Display",
    -apple-system,
    BlinkMacSystemFont,
    "Helvetica Neue",
    sans-serif;
  --lcd-font: "SF Mono", Menlo, Monaco, "Courier New", monospace;

  width: 100%;
  height: 100%;
  box-sizing: border-box;
  border-radius: 0;
  border: 1px solid #626668;
  background:
    linear-gradient(170deg, rgba(255, 255, 255, 0.06), transparent 30%),
    linear-gradient(180deg, #23272a 0%, #141619 100%);
  box-shadow:
    0 10px 24px rgba(0, 0, 0, 0.28),
    inset 0 1px 1px rgba(255, 255, 255, 0.2),
    inset 0 -8px 18px rgba(0, 0, 0, 0.35);
  padding: 6px;
  color: #f2f2f2;
  position: relative;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-family: var(--ui-font);
}

.drag-strip {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 28px;
  z-index: 7;
}

.unit::before {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  background: repeating-linear-gradient(
    to bottom,
    rgba(255, 255, 255, 0.016) 0,
    rgba(255, 255, 255, 0.016) 1px,
    transparent 1px,
    transparent 3px
  );
}

.unit-header {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  min-height: 24px;
  padding-left: 76px;
  padding-right: 4px;
  position: relative;
  z-index: 8;
}

.brand {
  margin: 0;
  font-size: 13px;
  font-weight: 800;
  letter-spacing: 0.12em;
  color: #e2e4e6;
  font-family: var(--display-font);
  text-transform: uppercase;
  text-shadow: 0 1px 0 rgba(0, 0, 0, 0.35);
}

.lcd {
  border-radius: 5px;
  border: 1px solid #8f4b22;
  background:
    radial-gradient(140% 120% at 10% -20%, rgba(255, 243, 219, 0.26), transparent 48%),
    linear-gradient(180deg, #cc7b4a 0%, #a15c33 100%);
  padding: 6px 8px;
  box-shadow:
    inset 0 0 14px rgba(38, 18, 5, 0.16),
    inset 0 -5px 13px rgba(46, 19, 3, 0.3),
    0 2px 10px rgba(0, 0, 0, 0.3);
  position: relative;
  z-index: 2;
}

.lcd-main,
.lcd-sub {
  margin: 0;
  color: #2a221b;
  text-shadow: 0 0 2px rgba(0, 0, 0, 0.14), 0 1px 0 rgba(255, 211, 177, 0.35);
  font-family: var(--lcd-font);
  font-weight: 700;
  letter-spacing: 0.085em;
  text-transform: uppercase;
}

.lcd-main {
  font-size: 14px;
  min-height: 1.3em;
  line-height: 1.16;
  font-weight: 800;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.lcd-sub {
  font-size: 8px;
  margin-top: 2px;
  opacity: 0.84;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.lcd-meta {
  margin-top: 3px;
  display: flex;
  gap: 8px;
  font-size: 8px;
  color: #3e3228;
  font-weight: 800;
  letter-spacing: 0.11em;
  text-transform: uppercase;
}

.preset-grid {
  flex: 1;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  grid-template-rows: repeat(2, minmax(0, 1fr));
  gap: 4px;
  min-height: 0;
  position: relative;
  z-index: 2;
}

.preset-card {
  border-radius: 5px;
  border: 1px solid #0d0f11;
  background: linear-gradient(180deg, #0f1012 0%, #0a0b0d 100%);
  box-shadow:
    inset 0 0 0 1px rgba(255, 255, 255, 0.05),
    inset 0 8px 16px rgba(255, 255, 255, 0.02),
    0 3px 10px rgba(0, 0, 0, 0.32);
  padding: 2px;
  display: flex;
  flex-direction: column;
}

.preset-card.active {
  border-color: #b17549;
  box-shadow:
    inset 0 0 0 1px rgba(255, 214, 174, 0.1),
    0 0 0 1px rgba(237, 165, 95, 0.3),
    0 4px 12px rgba(0, 0, 0, 0.42);
}

.preset-button {
  width: 100%;
  border: 0;
  border-radius: 4px;
  min-height: 0;
  height: 58px;
  color: #c7cbce;
  background: linear-gradient(155deg, #272a2d 0%, #141618 64%, #101214 100%);
  box-shadow:
    inset 0 0 0 1px rgba(255, 255, 255, 0.07),
    inset 0 1px 8px rgba(255, 255, 255, 0.03);
  cursor: pointer;
  transition:
    transform 90ms ease,
    filter 90ms ease;
  display: grid;
  place-items: center;
  gap: 1px;
  padding: 3px 3px 4px;
  box-sizing: border-box;
}

.preset-button:hover {
  transform: translateY(-0.5px);
  filter: brightness(1.06);
}

.preset-button:active {
  transform: translateY(0);
  filter: brightness(0.97);
}

.slot-number {
  font-size: 24px;
  line-height: 1;
  font-weight: 800;
  color: #556069;
  font-family: var(--display-font);
  letter-spacing: 0.02em;
}

.slot-title {
  font-weight: 750;
  font-size: 9px;
  line-height: 1.08;
  letter-spacing: 0.07em;
  color: #d8dde0;
  text-transform: uppercase;
  text-align: center;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  width: 100%;
  font-family: var(--display-font);
}

.slot-subtitle {
  font-size: 7px;
  color: #79858f;
  font-weight: 700;
  letter-spacing: 0.085em;
  text-transform: uppercase;
  line-height: 1.05;
  display: -webkit-box;
  -webkit-line-clamp: 1;
  -webkit-box-orient: vertical;
  overflow: hidden;
  text-align: center;
  min-height: 1.2em;
  max-width: 98%;
}

.preset-card.empty .slot-title {
  color: #8f999f;
}

.footer-btn {
  border-radius: 3px;
  border: 1px solid #3a4046;
  background: linear-gradient(180deg, #212529 0%, #1a1d20 100%);
  color: #d8dde1;
  font-size: 7px;
  font-weight: 760;
  padding: 3px 6px;
  letter-spacing: 0.09em;
  cursor: pointer;
  text-transform: uppercase;
  font-family: var(--display-font);
  transition: filter 90ms ease;
}

.footer-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.footer-btn:hover:not(:disabled) {
  filter: brightness(1.08);
}

.unit-footer {
  margin-top: 1px;
  padding-top: 3px;
  border-top: 1px solid rgba(255, 255, 255, 0.1);
  display: flex;
  justify-content: space-between;
  gap: 8px;
  align-items: center;
  position: relative;
  z-index: 3;
}

.tagline {
  margin: 0;
  color: #bbaf98;
  font-style: italic;
  font-weight: 760;
  font-size: 9px;
  letter-spacing: 0.075em;
  font-family: "Times New Roman", Georgia, serif;
  text-transform: uppercase;
}

.footer-controls {
  display: flex;
  align-items: center;
  gap: 3px;
}

.model {
  margin: 0;
  border: 1px solid #40464b;
  background: #1a1d20;
  color: #b2b9be;
  border-radius: 3px;
  padding: 2px 6px;
  font-weight: 800;
  letter-spacing: 0.12em;
  font-family: var(--display-font);
  font-size: 10px;
}

audio {
  display: none;
}

.context-backdrop {
  position: absolute;
  inset: 0;
  z-index: 20;
}

.context-menu {
  position: fixed;
  min-width: 190px;
  max-width: 220px;
  border-radius: 5px;
  border: 1px solid #3f454a;
  background: linear-gradient(180deg, #1b1f23 0%, #15181b 100%);
  box-shadow: 0 14px 26px rgba(0, 0, 0, 0.5);
  padding: 4px;
  z-index: 24;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.context-title {
  margin: 0;
  padding: 4px 6px;
  font-size: 8px;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: #c3c9cd;
  font-family: var(--display-font);
}

.context-item {
  border: 1px solid #33393f;
  border-radius: 3px;
  background: #13161a;
  color: #d8dde1;
  text-align: left;
  font-size: 9px;
  font-weight: 650;
  padding: 5px 6px;
  cursor: pointer;
  font-family: var(--ui-font);
}

.context-item:hover {
  filter: brightness(1.08);
}

.context-item.danger {
  color: #ffb8ad;
  border-color: #4e3a39;
}

@media (max-width: 420px) {
  .context-menu {
    max-width: 180px;
    min-width: 160px;
  }
}
</style>
