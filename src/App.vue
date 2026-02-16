<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  createNTSClient,
  playableFromChannel,
  playableFromMixtape,
  type MediaPlayable,
} from "./nts";
import {
  calculateChannelRefreshDelay,
  syncChannelPlayableFromLive,
} from "./nts/channelRefresh";
import LcdDisplay from "./components/receiver/LcdDisplay.vue";
import PresetGrid from "./components/receiver/PresetGrid.vue";
import PresetContextMenu from "./components/receiver/PresetContextMenu.vue";
import AudioFxSegmentedControl from "./components/receiver/AudioFxSegmentedControl.vue";
import { AUDIO_FX_PRESETS, type AudioFxPreset } from "./audio/fxPresets";

const client = createNTSClient();
const STORAGE_KEY = "nts-user-presets-v1";
const LCD_THEME_KEY = "lcd-theme-v1";
const AUDIO_FX_KEY = "audio-fx-preset-v2";
const MENU_BAR_ONLY_KEY = "menu-bar-only-v1";
const USER_SLOTS = [3, 4, 5, 6] as const;
const LCD_THEMES = ["amber", "blue", "green", "purpleRed"] as const;

type UserSlot = (typeof USER_SLOTS)[number];
type PresetAssignments = Record<UserSlot, string | null>;
type LcdTheme = (typeof LCD_THEMES)[number];
type NativeMediaControlAction = "play" | "pause" | "stop" | "toggle";
type NativeMediaControlPayload = {
  action: NativeMediaControlAction;
};

const channels = ref<MediaPlayable[]>([]);
const mixtapes = ref<MediaPlayable[]>([]);
const isLoading = ref(false);
const errorMessage = ref<string | null>(null);
const currentPlayable = ref<MediaPlayable | null>(null);
const activeSlot = ref<number | null>(null);
const isPlaying = ref(false);
const isLcdThemeAnimating = ref(false);
let lcdThemeAnimationTimer: ReturnType<typeof setTimeout> | null = null;
const isLcdTuning = ref(false);
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

function readMenuBarOnlyMode() {
  try {
    return localStorage.getItem(MENU_BAR_ONLY_KEY) === "1";
  } catch {
    return false;
  }
}

function canUseTauriInvoke() {
  return (
    typeof window !== "undefined" &&
    "__TAURI_INTERNALS__" in window &&
    typeof (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ !==
      "undefined"
  );
}

function detectMacPlatform() {
  if (typeof navigator === "undefined") {
    return false;
  }

  const nav = navigator as Navigator & { userAgentData?: { platform?: string } };
  const probe = `${nav.userAgentData?.platform ?? ""} ${navigator.platform ?? ""} ${navigator.userAgent ?? ""}`;
  return /mac|darwin/i.test(probe);
}

function detectWindowsPlatform() {
  if (typeof navigator === "undefined") {
    return false;
  }

  const nav = navigator as Navigator & { userAgentData?: { platform?: string } };
  const probe = `${nav.userAgentData?.platform ?? ""} ${navigator.platform ?? ""} ${navigator.userAgent ?? ""}`;
  return /windows|win32|win64|wow64/i.test(probe);
}

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
let unlistenNativeMediaControl: (() => void) | null = null;
let channelRefreshTimer: ReturnType<typeof setTimeout> | null = null;

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
const menuBarOnlyMode = ref(readMenuBarOnlyMode());
const settingsPanelVisible = ref(false);
const settingsPanelRef = ref<HTMLElement | null>(null);
const settingsCloseRef = ref<HTMLElement | null>(null);
let settingsTriggerEl: HTMLElement | null = null;
const isMacPlatform = ref(detectMacPlatform());
const isWindowsPlatform = ref(detectWindowsPlatform());
const isTrayModeSupported = computed(() => isMacPlatform.value || isWindowsPlatform.value);

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

watch(
  menuBarOnlyMode,
  (enabled) => {
    try {
      localStorage.setItem(MENU_BAR_ONLY_KEY, enabled ? "1" : "0");
    } catch {
      // Ignore storage errors.
    }

    if (!isTrayModeSupported.value || !canUseTauriInvoke()) {
      return;
    }

    void invoke("set_menu_bar_mode", { enabled })
      .catch((error) => {
        console.warn("[window] Unable to apply menu bar mode", error);
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
    currentPlayable.value = syncChannelPlayableFromLive(currentPlayable.value, channels.value);
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

function clearChannelRefreshTimer() {
  if (channelRefreshTimer) {
    clearTimeout(channelRefreshTimer);
    channelRefreshTimer = null;
  }
}

function scheduleChannelRefresh() {
  clearChannelRefreshTimer();

  if (!isPlaying.value || currentPlayable.value?.source.kind !== "channel") {
    return;
  }

  const delay = calculateChannelRefreshDelay(currentPlayable.value.source.value);

  channelRefreshTimer = setTimeout(() => {
    void loadPlayableMedia();
  }, delay);
}

async function startPlayback(playable: MediaPlayable, slot: number) {
  isLcdTuning.value = true;
  setTimeout(() => { isLcdTuning.value = false; }, 400);
  currentPlayable.value = playable;
  activeSlot.value = slot;
  errorMessage.value = null;

  if (!canUseTauriInvoke()) {
    errorMessage.value = "Native playback requires a Tauri runtime.";
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

async function handleNativeMediaControl(action: NativeMediaControlAction) {
  if (action === "pause" || action === "stop") {
    if (isPlaying.value) {
      await stopPlayback();
    }
    return;
  }

  if (!currentPlayable.value || activeSlot.value === null) {
    return;
  }

  if (action === "play") {
    if (!isPlaying.value) {
      await startPlayback(currentPlayable.value, activeSlot.value);
    }
    return;
  }

  if (action === "toggle") {
    if (isPlaying.value) {
      await stopPlayback();
      return;
    }
    await startPlayback(currentPlayable.value, activeSlot.value);
  }
}

const modelMenuVisible = ref(false);

function toggleModelMenu() {
  modelMenuVisible.value = !modelMenuVisible.value;
}

function closeModelMenu() {
  modelMenuVisible.value = false;
}

function openSettingsPanel() {
  settingsTriggerEl = document.activeElement as HTMLElement | null;
  settingsPanelVisible.value = true;
  closeModelMenu();
  closeContextMenu();
  void nextTick(() => {
    settingsCloseRef.value?.focus();
  });
}

function closeSettingsPanel() {
  settingsPanelVisible.value = false;
  settingsTriggerEl?.focus();
  settingsTriggerEl = null;
}

function handleSettingsKeydown(event: KeyboardEvent) {
  if (event.key !== "Tab") return;

  const panel = settingsPanelRef.value;
  if (!panel) return;

  const focusable = Array.from(
    panel.querySelectorAll<HTMLElement>(
      'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
    ),
  );
  if (focusable.length === 0) return;

  const first = focusable[0];
  const last = focusable[focusable.length - 1];

  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

function handleRockerKeydown(event: KeyboardEvent) {
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
  event.preventDefault();
  if (!isTrayModeSupported.value) return;

  const next = event.key === "ArrowRight";
  setMenuBarOnlyModeEnabled(next);

  void nextTick(() => {
    const group = (event.target as HTMLElement).closest(".setting-rocker");
    const active = group?.querySelector<HTMLElement>('[aria-checked="true"]');
    active?.focus();
  });
}

function setMenuBarOnlyModeEnabled(enabled: boolean) {
  if (!isTrayModeSupported.value) {
    return;
  }

  menuBarOnlyMode.value = enabled;
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

  if (!card.locked && slot >= 3 && slot <= 6) {
    openContextMenuForSlot(slot as UserSlot);
  }
}

function onPresetContextMenu(event: MouseEvent, slot: number, locked: boolean) {
  if (locked || slot < 3 || slot > 6) {
    return;
  }

  openContextMenu(event, slot as UserSlot);
}

function onPresetContextMenuByKeyboard(slot: number) {
  if (slot < 3 || slot > 6) {
    return;
  }

  openContextMenuForSlot(slot as UserSlot);
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
  }

  loadPlayableMedia();
});

watch([currentPlayable, isPlaying], () => {
  scheduleChannelRefresh();
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
  if (unlistenNativeMediaControl) {
    unlistenNativeMediaControl();
    unlistenNativeMediaControl = null;
  }
  if (lcdThemeAnimationTimer) {
    clearTimeout(lcdThemeAnimationTimer);
    lcdThemeAnimationTimer = null;
  }
  clearChannelRefreshTimer();
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
    closeSettingsPanel();
    closeContextMenu();
    closeModelMenu();
    return;
  }

  if (
    editable ||
    event.repeat ||
    contextMenu.value.visible ||
    modelMenuVisible.value ||
    settingsPanelVisible.value
  ) {
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
        <div class="model-wrap">
          <button
            type="button"
            class="brand-plate"
            aria-haspopup="menu"
            aria-controls="receiver-model-menu"
            :aria-expanded="modelMenuVisible ? 'true' : 'false'"
            @click="toggleModelMenu"
          >
            <span class="brand">MRC-1900</span>
          </button>
          <div v-if="modelMenuVisible" class="model-backdrop" @mousedown="closeModelMenu" />
          <div
            v-if="modelMenuVisible"
            id="receiver-model-menu"
            class="model-menu"
            role="menu"
            aria-label="Receiver controls"
            @mousedown.stop
          >
            <button
              type="button"
              class="model-menu-item"
              role="menuitem"
              :disabled="isLoading"
              @click="loadPlayableMedia(); closeModelMenu()"
            >
              {{ isLoading ? "SYNCING..." : "REFRESH" }}
            </button>
            <button
              type="button"
              class="model-menu-item"
              role="menuitem"
              @click="openSettingsPanel"
            >
              SETTINGS
            </button>
            <button
              type="button"
              class="model-menu-item"
              role="menuitem"
              :disabled="!isPlaying"
              @click="stopPlayback(); closeModelMenu()"
            >
              STOP
            </button>
          </div>
        </div>
      </header>

      <LcdDisplay
        :primary-text="lcdPrimary"
        :secondary-text="lcdSecondary"
        :meta-text="lcdMeta"
        :theme-animating="isLcdThemeAnimating"
        :tuning="isLcdTuning"
        @cycle-theme="cycleLcdTheme"
      />

      <PresetGrid
        :cards="presetCards"
        :active-slot="activeSlot"
        :set-button-ref="setPresetButtonRef"
        @press-slot="onPresetPress"
        @open-slot-context="onPresetContextMenu($event.event, $event.slot, $event.locked)"
        @open-slot-context-by-keyboard="onPresetContextMenuByKeyboard"
      />

      <AudioFxSegmentedControl
        class="fx-segmented-row"
        :presets="AUDIO_FX_PRESETS"
        :model-value="audioFxPreset"
        @update:model-value="setAudioFxPreset"
      />

      <footer class="unit-footer" data-tauri-drag-region>
        <div class="footer-line" />
        <p class="tagline">STREAMING RECEIVER</p>
        <div class="footer-line" />
      </footer>

      <div
        v-if="settingsPanelVisible"
        class="settings-backdrop"
        aria-hidden="true"
        @mousedown="closeSettingsPanel"
      />
      <section
        v-if="settingsPanelVisible"
        ref="settingsPanelRef"
        class="settings-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="receiver-settings-title"
        @mousedown.stop
        @keydown="handleSettingsKeydown"
      >
        <header class="settings-header">
          <div class="settings-header-rule" aria-hidden="true" />
          <p id="receiver-settings-title" class="settings-title">SETTINGS</p>
          <div class="settings-header-rule" aria-hidden="true" />
          <button
            ref="settingsCloseRef"
            type="button"
            class="settings-close-button"
            aria-label="Close settings"
            @click="closeSettingsPanel"
          >
            &times;
          </button>
        </header>

        <div class="settings-list">
          <div class="setting-row">
            <div class="setting-label-row" id="appmode-label">
              <p class="setting-name">APP MODE</p>
              <p class="setting-hint" id="appmode-hint">
                {{
                  isMacPlatform
                    ? "Hides Dock icon when set to tray"
                    : isWindowsPlatform
                      ? "Hides taskbar icon when set to tray"
                      : "Available on macOS and Windows"
                }}
              </p>
            </div>
            <div
              class="setting-rocker"
              role="radiogroup"
              aria-labelledby="appmode-label"
              @keydown="handleRockerKeydown"
            >
              <button
                type="button"
                role="radio"
                class="rocker-key"
                :class="{ 'rocker-key--active': !menuBarOnlyMode }"
                :aria-checked="!menuBarOnlyMode"
                :tabindex="!menuBarOnlyMode ? 0 : -1"
                :disabled="!isTrayModeSupported"
                @click="setMenuBarOnlyModeEnabled(false)"
              >
                DOCK
              </button>
              <button
                type="button"
                role="radio"
                class="rocker-key"
                :class="{ 'rocker-key--active': menuBarOnlyMode }"
                :aria-checked="menuBarOnlyMode"
                :tabindex="menuBarOnlyMode ? 0 : -1"
                :disabled="!isTrayModeSupported"
                @click="setMenuBarOnlyModeEnabled(true)"
              >
                TRAY
              </button>
            </div>
          </div>
        </div>

        <p class="settings-note">
          {{
            isMacPlatform
              ? "Window close keeps Marconio in the menu bar."
              : isWindowsPlatform
                ? "Window close keeps Marconio in the system tray."
              : "Tray mode available on macOS & Windows."
          }}
        </p>
      </section>

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
