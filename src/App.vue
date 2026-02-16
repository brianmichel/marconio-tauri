<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { type MediaPlayable } from "./nts";
import LcdDisplay from "./components/receiver/LcdDisplay.vue";
import PresetGrid from "./components/receiver/PresetGrid.vue";
import PresetContextMenu from "./components/receiver/PresetContextMenu.vue";
import AudioFxSegmentedControl from "./components/receiver/AudioFxSegmentedControl.vue";
import SupportDialog from "./components/receiver/SupportDialog.vue";
import ReceiverSettingsPanel from "./components/receiver/ReceiverSettingsPanel.vue";
import ReceiverModelMenu from "./components/receiver/ReceiverModelMenu.vue";
import { AUDIO_FX_PRESETS, type AudioFxPreset } from "./audio/fxPresets";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useGlobalReceiverHotkeys } from "./composables/useGlobalReceiverHotkeys";
import { useNativePlayback } from "./composables/useNativePlayback";
import { usePlayableCatalog } from "./composables/usePlayableCatalog";
import { usePresetController } from "./composables/usePresetController";

const STORAGE_KEY = "nts-user-presets-v1";
const LCD_THEME_KEY = "lcd-theme-v1";
const AUDIO_FX_KEY = "audio-fx-preset-v2";
const MENU_BAR_ONLY_KEY = "menu-bar-only-v1";
const LCD_THEMES = ["amber", "blue", "green", "purpleRed"] as const;

type UserSlot = 3 | 4 | 5 | 6;
type PresetAssignments = Record<UserSlot, string | null>;
type LcdTheme = (typeof LCD_THEMES)[number];

const errorMessage = ref<string | null>(null);
const currentPlayable = ref<MediaPlayable | null>(null);
const activeSlot = ref<number | null>(null);
const isLcdThemeAnimating = ref(false);
let lcdThemeAnimationTimer: ReturnType<typeof setTimeout> | null = null;
const isLcdTuning = ref(false);
const supportDialogVisible = ref(true);

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

let unlistenTrayOpenSettings: (() => void) | null = null;

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
let settingsTriggerEl: HTMLElement | null = null;
const isMacPlatform = ref(detectMacPlatform());
const isWindowsPlatform = ref(detectWindowsPlatform());
const isTrayModeSupported = computed(() => isMacPlatform.value || isWindowsPlatform.value);
const { isPlaying, startPlayback, stopPlayback } = useNativePlayback({
  canUseTauriInvoke,
  currentPlayable,
  activeSlot,
  errorMessage,
  isLcdTuning,
});
const { channels, mixtapes, mixtapeByAlias, isLoading, loadPlayableMedia } = usePlayableCatalog({
  assignments,
  currentPlayable,
  isPlaying,
  errorMessage,
});

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

watch(
  [activeSlot, isPlaying, currentPlayable],
  ([slot, playing, playable]) => {
    if (!canUseTauriInvoke()) {
      return;
    }
    const slotValue = playing && slot !== null ? slot : null;
    const title = playing && playable ? playable.title : null;
    const subtitle = playing && playable?.subtitle ? playable.subtitle : null;

    void invoke("set_tray_preset", { slot: slotValue }).catch((error) => {
      console.warn("[tray] Unable to update tray icon", error);
    });
    void invoke("update_tray_menu", { title, subtitle }).catch((error) => {
      console.warn("[tray] Unable to update tray menu", error);
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

const {
  contextMenu,
  presetCards,
  contextMenuHasAssignment,
  closeContextMenu,
  setPresetButtonRef,
  assignSlotFromMenu,
  clearContextMenuSlot,
  onPresetPress,
  onPresetContextMenu,
  onPresetContextMenuByKeyboard,
  onPresetHotkey,
} = usePresetController({
  assignments,
  channelOne,
  channelTwo,
  mixtapeByAlias,
  activeSlot,
  currentPlayable,
  startPlayback,
  stopPlayback,
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

const modelMenuVisible = ref(false);

function toggleModelMenu() {
  modelMenuVisible.value = !modelMenuVisible.value;
}

function closeModelMenu() {
  modelMenuVisible.value = false;
}

function dismissSupportDialog() {
  supportDialogVisible.value = false;
}

function openSupportersPage() {
  void openUrl("https://www.nts.live/supporters").catch(() => {
    window.open("https://www.nts.live/supporters", "_blank");
  });
}

function openSettingsPanel() {
  settingsTriggerEl = document.activeElement as HTMLElement | null;
  settingsPanelVisible.value = true;
  closeModelMenu();
  closeContextMenu();
}

function closeSettingsPanel() {
  settingsPanelVisible.value = false;
  settingsTriggerEl?.focus();
  settingsTriggerEl = null;
}

function setMenuBarOnlyMode(enabled: boolean) {
  if (!isTrayModeSupported.value) {
    return;
  }

  menuBarOnlyMode.value = enabled;
}

useGlobalReceiverHotkeys({
  isDev: import.meta.env.DEV,
  supportDialogVisible,
  contextMenuVisible: computed(() => contextMenu.value.visible),
  modelMenuVisible,
  settingsPanelVisible,
  isPlaying,
  activeSlot,
  currentPlayable,
  closeSettingsPanel,
  closeContextMenu,
  closeModelMenu,
  onPresetHotkey,
  onPresetPress,
  startPlayback,
  stopPlayback,
});

async function startWindowDrag() {
  try {
    await getCurrentWindow().startDragging();
  } catch {
    // Ignore drag errors (e.g. non-tauri context).
  }
}

onMounted(async () => {
  if (canUseTauriInvoke()) {
    try {
      await invoke("set_audio_fx_preset", { preset: audioFxPreset.value });
    } catch (error) {
      console.warn("[audio] Unable to initialize native preset", error);
    }

    try {
      unlistenTrayOpenSettings = await listen("tray-open-settings", () => {
        openSettingsPanel();
      });
    } catch (error) {
      console.warn("[tray] Unable to listen for settings event", error);
    }
  }

  loadPlayableMedia();
});

onBeforeUnmount(() => {
  if (unlistenTrayOpenSettings) {
    unlistenTrayOpenSettings();
    unlistenTrayOpenSettings = null;
  }
  if (lcdThemeAnimationTimer) {
    clearTimeout(lcdThemeAnimationTimer);
    lcdThemeAnimationTimer = null;
  }
});
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
        <ReceiverModelMenu
          :visible="modelMenuVisible"
          :is-loading="isLoading"
          :is-playing="isPlaying"
          @toggle="toggleModelMenu"
          @close="closeModelMenu"
          @refresh="loadPlayableMedia(); closeModelMenu()"
          @settings="openSettingsPanel"
          @stop="stopPlayback(); closeModelMenu()"
        />
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

      <SupportDialog
        :visible="supportDialogVisible"
        @dismiss="dismissSupportDialog"
        @learn-more="openSupportersPage(); dismissSupportDialog()"
      />

      <ReceiverSettingsPanel
        :visible="settingsPanelVisible"
        :menu-bar-only-mode="menuBarOnlyMode"
        :is-tray-mode-supported="isTrayModeSupported"
        :is-mac-platform="isMacPlatform"
        :is-windows-platform="isWindowsPlatform"
        @close="closeSettingsPanel"
        @set-menu-bar-only-mode="setMenuBarOnlyMode"
      />

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
