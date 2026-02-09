<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import {
  createNTSClient,
  playableFromChannel,
  playableFromMixtape,
  type MediaPlayable,
} from "./nts";
import LcdDisplay from "./components/receiver/LcdDisplay.vue";
import PresetGrid from "./components/receiver/PresetGrid.vue";
import PresetContextMenu from "./components/receiver/PresetContextMenu.vue";
import AudioFxSegmentedControl from "./components/receiver/AudioFxSegmentedControl.vue";
import { AUDIO_FX_PRESETS, type AudioFxPreset } from "./audio/fxPresets";

const client = createNTSClient();
const STORAGE_KEY = "nts-user-presets-v1";
const LCD_THEME_KEY = "lcd-theme-v1";
const AUDIO_FX_KEY = "audio-fx-preset-v2";
const USER_SLOTS = [3, 4, 5, 6] as const;
const LCD_THEMES = ["amber", "blue", "green", "purpleRed"] as const;

type UserSlot = (typeof USER_SLOTS)[number];
type PresetAssignments = Record<UserSlot, string | null>;
type LcdTheme = (typeof LCD_THEMES)[number];

const channels = ref<MediaPlayable[]>([]);
const mixtapes = ref<MediaPlayable[]>([]);
const isLoading = ref(false);
const errorMessage = ref<string | null>(null);
const currentPlayable = ref<MediaPlayable | null>(null);
const activeSlot = ref<number | null>(null);
const isPlaying = ref(false);
const isLcdThemeAnimating = ref(false);
let lcdThemeAnimationTimer: ReturnType<typeof setTimeout> | null = null;
const BLOCKED_BROWSER_SHORTCUTS = new Set(["a", "r", "+", "=", "-", "0"]);
const IS_DEV = import.meta.env.DEV;
const EDITABLE_TARGET_SELECTOR = [
  "input:not([readonly]):not([disabled])",
  "textarea:not([readonly]):not([disabled])",
  '[contenteditable=""]',
  '[contenteditable="true"]',
  '[role="textbox"]',
].join(", ");

function readLcdTheme(): LcdTheme {
  try {
    const stored = localStorage.getItem(LCD_THEME_KEY);
    if (stored && LCD_THEMES.includes(stored as LcdTheme)) {
      return stored as LcdTheme;
    }
  } catch {
    // Ignore localStorage access errors and keep default theme.
  }

  return "amber";
}

function readAudioFxPreset(): AudioFxPreset {
  try {
    const stored = localStorage.getItem(AUDIO_FX_KEY);
    if (stored && AUDIO_FX_PRESETS.some((preset) => preset.id === stored)) {
      return stored as AudioFxPreset;
    }
  } catch {
    // Ignore localStorage access errors and keep default preset.
  }

  return "clean";
}

function canUseTauriInvoke() {
  return (
    typeof window !== "undefined" &&
    "__TAURI_INTERNALS__" in window &&
    typeof (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ !==
      "undefined"
  );
}

function cycleLcdTheme() {
  const currentIndex = LCD_THEMES.indexOf(lcdTheme.value);
  lcdTheme.value = LCD_THEMES[(currentIndex + 1) % LCD_THEMES.length];
  isLcdThemeAnimating.value = true;

  if (lcdThemeAnimationTimer) {
    clearTimeout(lcdThemeAnimationTimer);
  }

  lcdThemeAnimationTimer = setTimeout(() => {
    isLcdThemeAnimating.value = false;
    lcdThemeAnimationTimer = null;
  }, 450);
}

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
const presetButtonRefs = ref<Record<number, HTMLButtonElement | null>>({});

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
const lcdTheme = ref<LcdTheme>(readLcdTheme());
const audioFxPreset = ref<AudioFxPreset>(readAudioFxPreset());

watch(
  assignments,
  (value) => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
  },
  { deep: true },
);

watch(lcdTheme, (value) => {
  localStorage.setItem(LCD_THEME_KEY, value);
});

watch(
  audioFxPreset,
  (value) => {
    try {
      localStorage.setItem(AUDIO_FX_KEY, value);
    } catch {
      // Ignore storage errors.
    }
    if (!canUseTauriInvoke()) {
      return;
    }
    void invoke("set_audio_fx_preset", { preset: value })
      .catch((error) => {
        console.warn("[audio] Unable to set native preset", error);
      });
  },
  { immediate: true },
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
  return isLoading.value ? "TUNING..." : "NOT PLAYING";
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

  return "";
});

const lcdMeta = computed(() => {
  const play = isPlaying.value ? "PLAY" : "STOP";
  const status = errorMessage.value
    ? "FAULT"
    : isLoading.value
      ? "SYNC"
      : "READY";
  return `${play} ${status}`;
});

function setAudioFxPreset(preset: AudioFxPreset) {
  audioFxPreset.value = preset;
}

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

const contextMenuHasAssignment = computed(() => {
  const slot = contextMenu.value.slot;
  if (!slot) {
    return false;
  }

  return Boolean(assignments.value[slot]);
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

  if (!canUseTauriInvoke()) {
    errorMessage.value = "Native playback requires a Tauri runtime.";
    return;
  }

  try {
    await invoke("start_native_stream", { streamUrl: playable.streamUrl });
    isPlaying.value = true;
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    errorMessage.value = `Unable to start playback: ${message}`;
    isPlaying.value = false;
  }
}

async function stopPlayback() {
  if (!canUseTauriInvoke()) {
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

const modelMenuVisible = ref(false);

function toggleModelMenu() {
  modelMenuVisible.value = !modelMenuVisible.value;
}

function closeModelMenu() {
  modelMenuVisible.value = false;
}

function closeContextMenu() {
  contextMenu.value.visible = false;
  contextMenu.value.slot = null;
}

function setPresetButtonRef(slot: number, element: unknown) {
  presetButtonRefs.value[slot] = element instanceof HTMLButtonElement ? element : null;
}

function openContextMenuAt(slot: UserSlot, x: number, y: number) {
  const menuWidth = 210;
  const menuMaxHeight = 220;
  const padding = 6;

  contextMenu.value.slot = slot;
  contextMenu.value.x = Math.max(
    padding,
    Math.min(x, window.innerWidth - menuWidth - padding),
  );
  contextMenu.value.y = Math.max(
    padding,
    Math.min(y, window.innerHeight - menuMaxHeight - padding),
  );
  contextMenu.value.visible = true;
}

function openContextMenu(event: MouseEvent, slot: UserSlot) {
  openContextMenuAt(slot, event.clientX, event.clientY);
}

function openContextMenuForSlot(slot: UserSlot) {
  const button = presetButtonRefs.value[slot];
  if (button) {
    const rect = button.getBoundingClientRect();
    openContextMenuAt(slot, rect.left + rect.width / 2, rect.bottom + 6);
    return;
  }

  openContextMenuAt(slot, window.innerWidth / 2, window.innerHeight / 2);
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
    void stopPlayback();
    activeSlot.value = null;
    currentPlayable.value = null;
  }
}

function clearContextMenuSlot() {
  const slot = contextMenu.value.slot;
  if (!slot) {
    return;
  }

  clearUserSlot(slot);
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

function onPresetHotkey(slot: number) {
  const card = presetCards.value.find((item) => item.slot === slot);
  if (!card) {
    return;
  }

  if (card.playable) {
    void startPlayback(card.playable, slot);
    return;
  }

  if (!card.locked && slot >= 3 && slot <= 6) {
    openContextMenuForSlot(slot as UserSlot);
  }
}

function isEditableTarget(target: EventTarget | null): boolean {
  const element = target instanceof Element ? target : null;
  if (!element) {
    return false;
  }

  return element.closest(EDITABLE_TARGET_SELECTOR) !== null;
}

function preventBrowserContextMenu(event: MouseEvent) {
  if (IS_DEV) {
    return;
  }

  if (isEditableTarget(event.target)) {
    return;
  }

  event.preventDefault();
}

function preventDocumentDrop(event: DragEvent) {
  event.preventDefault();
}

async function startWindowDrag() {
  try {
    await getCurrentWindow().startDragging();
  } catch {
    // Ignore drag errors (e.g. non-tauri context).
  }
}

onMounted(async () => {
  window.addEventListener("keydown", onGlobalKeyDown);
  window.addEventListener("contextmenu", preventBrowserContextMenu);
  window.addEventListener("dragover", preventDocumentDrop);
  window.addEventListener("drop", preventDocumentDrop);

  if (canUseTauriInvoke()) {
    try {
      await invoke("set_audio_fx_preset", { preset: audioFxPreset.value });
    } catch (error) {
      console.warn("[audio] Unable to initialize native preset", error);
    }
  }

  loadPlayableMedia();
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onGlobalKeyDown);
  window.removeEventListener("contextmenu", preventBrowserContextMenu);
  window.removeEventListener("dragover", preventDocumentDrop);
  window.removeEventListener("drop", preventDocumentDrop);
  controller?.abort();
  if (canUseTauriInvoke()) {
    void invoke("stop_native_stream").catch(() => {
      // Ignore cleanup errors.
    });
  }
  if (lcdThemeAnimationTimer) {
    clearTimeout(lcdThemeAnimationTimer);
    lcdThemeAnimationTimer = null;
  }
});

function onGlobalKeyDown(event: KeyboardEvent) {
  const editable = isEditableTarget(event.target);
  const key = event.key.toLowerCase();
  const hasPrimaryModifier = event.metaKey || event.ctrlKey;
  const shortcutSlot = Number(event.key);

  if (
    !editable &&
    hasPrimaryModifier &&
    Number.isInteger(shortcutSlot) &&
    shortcutSlot >= 1 &&
    shortcutSlot <= 6
  ) {
    event.preventDefault();
    onPresetHotkey(shortcutSlot);
    return;
  }

  if (!editable && hasPrimaryModifier && BLOCKED_BROWSER_SHORTCUTS.has(key)) {
    event.preventDefault();
    return;
  }

  if (!editable && (event.key === "F5" || event.key === "F12")) {
    event.preventDefault();
    return;
  }

  if (event.key === "Escape") {
    closeContextMenu();
    closeModelMenu();
    return;
  }

  if (editable || event.repeat || contextMenu.value.visible || modelMenuVisible.value) {
    return;
  }

  if (event.code === "Space") {
    event.preventDefault();

    if (isPlaying.value) {
      void stopPlayback();
      return;
    }

    if (currentPlayable.value && activeSlot.value !== null) {
      void startPlayback(currentPlayable.value, activeSlot.value);
    }

    return;
  }

  if (event.key >= "1" && event.key <= "6") {
    event.preventDefault();
    onPresetPress(Number(event.key));
  }
}
</script>

<template>
  <main class="scene">
    <div class="unit" :class="`theme--${lcdTheme}`">
      <div
        class="drag-strip"
        data-tauri-drag-region
        role="presentation"
        @mousedown.left="startWindowDrag"
      />
      <header class="unit-header" data-tauri-drag-region>
        <p class="brand">MARCONIO</p>
      </header>

      <LcdDisplay
        :primary-text="lcdPrimary"
        :secondary-text="lcdSecondary"
        :meta-text="lcdMeta"
        :theme-animating="isLcdThemeAnimating"
        @cycle-theme="cycleLcdTheme"
      />

      <PresetGrid
        :cards="presetCards"
        :active-slot="activeSlot"
        :set-button-ref="setPresetButtonRef"
        @press-slot="onPresetPress"
        @open-slot-context="onPresetContextMenu($event.event, $event.slot, $event.locked)"
      />

      <AudioFxSegmentedControl
        class="fx-segmented-row"
        :presets="AUDIO_FX_PRESETS"
        :model-value="audioFxPreset"
        @update:model-value="setAudioFxPreset"
      />

      <footer class="unit-footer" data-tauri-drag-region>
        <p class="tagline">STREAMING RECEIVER</p>
        <div class="model-wrap">
          <button type="button" class="model" @click="toggleModelMenu">MRC-1900</button>
          <div v-if="modelMenuVisible" class="model-backdrop" @mousedown="closeModelMenu" />
          <div v-if="modelMenuVisible" class="model-menu" @mousedown.stop>
            <button
              type="button"
              class="model-menu-item"
              :disabled="isLoading"
              @click="loadPlayableMedia(); closeModelMenu()"
            >
              {{ isLoading ? "SYNCING..." : "REFRESH" }}
            </button>
            <button
              type="button"
              class="model-menu-item"
              :disabled="!isPlaying"
              @click="stopPlayback(); closeModelMenu()"
            >
              STOP
            </button>
          </div>
        </div>
      </footer>

      <PresetContextMenu
        :visible="contextMenu.visible"
        :slot="contextMenu.slot"
        :x="contextMenu.x"
        :y="contextMenu.y"
        :options="mixtapeOptions"
        :has-assignment="contextMenuHasAssignment"
        @close="closeContextMenu"
        @assign="assignSlotFromMenu"
        @clear="clearContextMenuSlot"
      />
    </div>
  </main>
</template>

<style scoped src="./components/receiver/receiver-shell.css"></style>
