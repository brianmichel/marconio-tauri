<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  createNTSClient,
  playableFromChannel,
  playableFromMixtape,
  type MediaPlayable,
} from "./nts";

const client = createNTSClient();
const STORAGE_KEY = "nts-user-presets-v1";
const LCD_THEME_KEY = "lcd-theme-v1";
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
const audioRef = ref<HTMLAudioElement | null>(null);

// LCD character-cell display
// ~ shows all 14 segments in DSEG14 (ghost/background segments)
const MAIN_FONT_SIZE = 20;
const SUB_FONT_SIZE = 10;
const META_FONT_SIZE = 8;
const SCROLL_GAP = 4; // blank cells between end and wrap

const lcdRef = ref<HTMLElement | null>(null);
const mainCols = ref(18);
const subCols = ref(36);
const metaCols = ref(45);
const mainScrollPos = ref(0);
const subScrollPos = ref(0);
const isLcdThemeAnimating = ref(false);
let mainScrollTimer: ReturnType<typeof setTimeout> | null = null;
let subScrollTimer: ReturnType<typeof setTimeout> | null = null;
let lcdThemeAnimationTimer: ReturnType<typeof setTimeout> | null = null;
const BLOCKED_BROWSER_SHORTCUTS = new Set(["a", "r", "+", "=", "-", "0"]);
const EDITABLE_TARGET_SELECTOR = [
  "input:not([readonly]):not([disabled])",
  "textarea:not([readonly]):not([disabled])",
  '[contenteditable=""]',
  '[contenteditable="true"]',
  '[role="textbox"]',
].join(", ");

function measureCols() {
  const el = lcdRef.value;
  if (!el) return;
  const style = getComputedStyle(el);
  const width =
    el.clientWidth -
    parseFloat(style.paddingLeft) -
    parseFloat(style.paddingRight);

  const canvas = document.createElement("canvas");
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  // Measure a run of characters to get accurate average width
  const sample = "~~~~~~~~~~"; // 10 chars
  ctx.font = `700 ${MAIN_FONT_SIZE}px DSEG14, monospace`;
  const mainCharW = ctx.measureText(sample).width / sample.length;
  if (mainCharW > 0) mainCols.value = Math.floor(width / mainCharW);

  ctx.font = `700 ${SUB_FONT_SIZE}px DSEG14, monospace`;
  const subCharW = ctx.measureText(sample).width / sample.length;
  if (subCharW > 0) subCols.value = Math.floor(width / subCharW);

  ctx.font = `700 ${META_FONT_SIZE}px DSEG14, monospace`;
  const metaCharW = ctx.measureText(sample).width / sample.length;
  if (metaCharW > 0) metaCols.value = Math.floor(width / metaCharW);
}

/**
 * Clean text for DSEG14 rendering.
 * Space → ! (full-width blank cell, keeps alignment with ghost ~)
 * Colon → - (colons break the segment illusion)
 */
function dsegClean(text: string): string {
  return text.replace(/ /g, "!").replace(/:/g, "-");
}

/** Extract a cols-wide window from text at the given offset, wrapping around. */
function visibleSlice(text: string, cols: number, offset: number): string {
  const cleaned = dsegClean(text);
  if (!cleaned || cleaned.length <= cols) {
    return (cleaned || "").padEnd(cols, "!");
  }
  const gap = "!".repeat(SCROLL_GAP);
  const looped = cleaned + gap;
  const total = looped.length;
  const pos = offset % total;
  let out = "";
  for (let i = 0; i < cols; i++) {
    out += looped[(pos + i) % total];
  }
  return out;
}

const lcdMainVisible = computed(() =>
  visibleSlice(lcdPrimary.value, mainCols.value, mainScrollPos.value),
);

const lcdSubVisible = computed(() =>
  visibleSlice(lcdSecondary.value, subCols.value, subScrollPos.value),
);

const lcdMetaText = computed(() => {
  const play = isPlaying.value ? "PLAY" : "STOP";
  const status = errorMessage.value
    ? "FAULT"
    : isLoading.value
      ? "SYNC"
      : "READY";
  return dsegClean(play + " " + status).padEnd(metaCols.value, "!");
});

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
  }, 320);
}

function startMainScroll() {
  stopMainScroll();
  if (lcdPrimary.value.length <= mainCols.value) return;
  const total = lcdPrimary.value.length + SCROLL_GAP;
  mainScrollTimer = setTimeout(function tick() {
    mainScrollPos.value++;
    // Pause when a full cycle completes
    mainScrollTimer = setTimeout(tick, mainScrollPos.value % total === 0 ? 2500 : 250);
  }, 2500);
}

function stopMainScroll() {
  if (mainScrollTimer) {
    clearTimeout(mainScrollTimer);
    mainScrollTimer = null;
  }
  mainScrollPos.value = 0;
}

function startSubScroll() {
  stopSubScroll();
  if (lcdSecondary.value.length <= subCols.value) return;
  const total = lcdSecondary.value.length + SCROLL_GAP;
  subScrollTimer = setTimeout(function tick() {
    subScrollPos.value++;
    subScrollTimer = setTimeout(tick, subScrollPos.value % total === 0 ? 2500 : 200);
  }, 2500);
}

function stopSubScroll() {
  if (subScrollTimer) {
    clearTimeout(subScrollTimer);
    subScrollTimer = null;
  }
  subScrollPos.value = 0;
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

watch(lcdPrimary, () => {
  stopMainScroll();
  nextTick(startMainScroll);
});

watch(lcdSecondary, () => {
  stopSubScroll();
  nextTick(startSubScroll);
});

onMounted(async () => {
  window.addEventListener("keydown", onGlobalKeyDown);
  window.addEventListener("contextmenu", preventBrowserContextMenu);
  window.addEventListener("dragover", preventDocumentDrop);
  window.addEventListener("drop", preventDocumentDrop);
  loadPlayableMedia();
  await document.fonts.ready;
  nextTick(() => {
    measureCols();
    startMainScroll();
    startSubScroll();
  });
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onGlobalKeyDown);
  window.removeEventListener("contextmenu", preventBrowserContextMenu);
  window.removeEventListener("dragover", preventDocumentDrop);
  window.removeEventListener("drop", preventDocumentDrop);
  controller?.abort();
  stopMainScroll();
  stopSubScroll();
  const audio = audioRef.value;
  if (audio) {
    audio.pause();
    audio.src = "";
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
    return;
  }

  if (editable || event.repeat || contextMenu.value.visible) {
    return;
  }

  if (event.code === "Space") {
    event.preventDefault();

    if (isPlaying.value) {
      stopPlayback();
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

      <section
        ref="lcdRef"
        class="lcd"
        :class="{ 'lcd--theme-animating': isLcdThemeAnimating }"
        aria-live="polite"
        @click="cycleLcdTheme"
      >
        <div class="lcd-row lcd-row--main" aria-hidden="true">
          <span v-for="i in mainCols" :key="i" class="lcd-cell">
            <span class="lcd-cell-ghost">~</span>
            <span class="lcd-cell-text">{{ lcdMainVisible[i - 1] }}</span>
          </span>
        </div>
        <div class="lcd-row lcd-row--sub" aria-hidden="true">
          <span v-for="i in subCols" :key="i" class="lcd-cell">
            <span class="lcd-cell-ghost">~</span>
            <span class="lcd-cell-text">{{ lcdSubVisible[i - 1] }}</span>
          </span>
        </div>
        <div class="lcd-row lcd-row--meta" aria-hidden="true">
          <span v-for="i in metaCols" :key="i" class="lcd-cell">
            <span class="lcd-cell-ghost">~</span>
            <span class="lcd-cell-text">{{ lcdMetaText[i - 1] }}</span>
          </span>
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
            :ref="(el) => setPresetButtonRef(card.slot, el)"
            @click="onPresetPress(card.slot)"
            @contextmenu.prevent="onPresetContextMenu($event, card.slot, card.locked)"
          >
            <span class="slot-number">{{ card.slot }}</span>
            <span class="slot-label">
              {{ card.locked ? "CH" : card.playable ? card.playable.title.toUpperCase() : "+" }}
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
        <div class="context-scroll">
          <button
            v-for="option in mixtapeOptions"
            :key="option.alias"
            type="button"
            class="context-item"
            @click="assignSlotFromMenu(option.alias)"
          >
            {{ option.title }}
          </button>
        </div>
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
@font-face {
  font-family: "DSEG14";
  src: url("./assets/DSEG14Classic-Bold.ttf") format("truetype");
  font-weight: 700;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: "DSEG14";
  src: url("./assets/DSEG14Classic-Regular.ttf") format("truetype");
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: "DSEG14";
  src: url("./assets/DSEG14Classic-Light.ttf") format("truetype");
  font-weight: 300;
  font-style: normal;
  font-display: swap;
}

:global(html),
:global(body),
:global(#app) {
  width: 100%;
  height: 100%;
  margin: 0;
  overscroll-behavior: none;
  user-select: none;
  -webkit-user-select: none;
  -webkit-touch-callout: none;
}

:global(*),
:global(*::before),
:global(*::after) {
  box-sizing: border-box;
  -webkit-tap-highlight-color: transparent;
  -webkit-user-drag: none;
}

:global(input),
:global(textarea),
:global([contenteditable=""]),
:global([contenteditable="true"]) {
  user-select: text;
  -webkit-user-select: text;
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
  cursor: default;
}

/* ──────────────────────────────────────────
   UNIT BODY — thick molded enclosure
   ────────────────────────────────────────── */
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
  --lcd-font: "DSEG14", monospace;
  --lcd-border: #7a4120;
  --lcd-glow: rgba(255, 240, 210, 0.3);
  --lcd-top: #c87848;
  --lcd-mid: #a35a30;
  --lcd-bottom: #8e4e28;
  --lcd-shadow-strong: rgba(60, 20, 0, 0.3);
  --lcd-shadow-soft: rgba(60, 20, 0, 0.1);
  --lcd-ghost: rgba(55, 28, 10, 0.16);
  --lcd-main-text: #3a1e0a;
  --lcd-main-glow: rgba(40, 15, 0, 0.2);
  --lcd-sub-glow: rgba(40, 15, 0, 0.15);
  --lcd-meta-text: #5a301a;
  --theme-accent-border: #8a5a30;
  --theme-accent-soft: rgba(180, 120, 60, 0.08);
  --theme-accent-outline: rgba(200, 140, 70, 0.25);
  --theme-accent-glow: rgba(180, 120, 60, 0.12);
  --theme-slot-active: #a08060;
  --theme-slot-label-active: #8a7060;
  --theme-focus-outline: #c68452;

  width: 100%;
  height: 100%;
  border: 1px solid #4a4d50;
  background:
    linear-gradient(168deg, rgba(255, 255, 255, 0.07) 0%, transparent 22%),
    linear-gradient(180deg, #2a2e32 0%, #1a1d21 38%, #111417 100%);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.15),
    inset 0 -1px 0 rgba(0, 0, 0, 0.6),
    inset 2px 0 6px rgba(0, 0, 0, 0.2),
    inset -2px 0 6px rgba(0, 0, 0, 0.2),
    inset 0 -12px 28px rgba(0, 0, 0, 0.4);
  padding: 6px 8px 5px;
  color: #f2f2f2;
  position: relative;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  gap: 5px;
  font-family: var(--ui-font);
}

/* subtle scan-line texture */
.unit::before {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  background: repeating-linear-gradient(
    to bottom,
    rgba(255, 255, 255, 0.012) 0,
    rgba(255, 255, 255, 0.012) 1px,
    transparent 1px,
    transparent 3px
  );
  z-index: 1;
}

/* fine noise grain overlay */
.unit::after {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  opacity: 0.035;
  background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
  background-size: 128px 128px;
  z-index: 1;
}

.drag-strip {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 28px;
  z-index: 10;
  cursor: grab;
}

.drag-strip:active {
  cursor: grabbing;
}

/* ──────────────────────────────────────────
   HEADER
   ────────────────────────────────────────── */
.unit-header {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  min-height: 22px;
  padding-left: 76px;
  padding-right: 2px;
  position: relative;
  z-index: 8;
}

.brand {
  margin: 0;
  font-size: 13px;
  font-weight: 800;
  letter-spacing: 0.14em;
  color: #d4d6d8;
  font-family: var(--display-font);
  text-transform: uppercase;
  text-shadow:
    0 1px 0 rgba(0, 0, 0, 0.5),
    0 -1px 0 rgba(255, 255, 255, 0.06);
}

/* ──────────────────────────────────────────
   LCD DISPLAY — warm amber segmented screen
   ────────────────────────────────────────── */
.lcd {
  border-radius: 4px;
  border: 1px solid var(--lcd-border);
  background:
    radial-gradient(120% 100% at 8% -10%, var(--lcd-glow), transparent 50%),
    linear-gradient(180deg, var(--lcd-top) 0%, var(--lcd-mid) 50%, var(--lcd-bottom) 100%);
  padding: 8px 10px 6px;
  box-shadow:
    inset 0 2px 4px rgba(0, 0, 0, 0.15),
    inset 0 -3px 8px var(--lcd-shadow-strong),
    inset 2px 0 6px var(--lcd-shadow-soft),
    inset -2px 0 6px var(--lcd-shadow-soft),
    0 1px 0 rgba(255, 255, 255, 0.06),
    0 3px 12px rgba(0, 0, 0, 0.35);
  position: relative;
  z-index: 2;
  overflow: hidden;
  cursor: pointer;
  transition:
    border-color 280ms ease,
    box-shadow 280ms ease,
    filter 140ms ease;
}

.lcd:hover {
  filter: brightness(1.03);
}

.lcd--theme-animating {
  animation: lcd-theme-swap 320ms cubic-bezier(0.16, 1, 0.3, 1);
}

@keyframes lcd-theme-swap {
  0% {
    transform: scale(1);
    filter: saturate(0.95) brightness(0.98);
  }
  45% {
    transform: scale(0.9975);
    filter: saturate(1.25) brightness(1.08);
  }
  100% {
    transform: scale(1);
    filter: saturate(1) brightness(1);
  }
}

.unit.theme--amber {
  --lcd-border: #7a4120;
  --lcd-glow: rgba(255, 240, 210, 0.3);
  --lcd-top: #c87848;
  --lcd-mid: #a35a30;
  --lcd-bottom: #8e4e28;
  --lcd-shadow-strong: rgba(60, 20, 0, 0.3);
  --lcd-shadow-soft: rgba(60, 20, 0, 0.1);
  --lcd-ghost: rgba(55, 28, 10, 0.16);
  --lcd-main-text: #3a1e0a;
  --lcd-main-glow: rgba(40, 15, 0, 0.2);
  --lcd-sub-glow: rgba(40, 15, 0, 0.15);
  --lcd-meta-text: #5a301a;
  --theme-accent-border: #8a5a30;
  --theme-accent-soft: rgba(180, 120, 60, 0.08);
  --theme-accent-outline: rgba(200, 140, 70, 0.25);
  --theme-accent-glow: rgba(180, 120, 60, 0.12);
  --theme-slot-active: #a08060;
  --theme-slot-label-active: #8a7060;
  --theme-focus-outline: #c68452;
}

.unit.theme--blue {
  --lcd-border: #2a5f89;
  --lcd-glow: rgba(208, 236, 255, 0.3);
  --lcd-top: #6ba6d8;
  --lcd-mid: #4377ad;
  --lcd-bottom: #2b567f;
  --lcd-shadow-strong: rgba(17, 49, 76, 0.35);
  --lcd-shadow-soft: rgba(17, 49, 76, 0.15);
  --lcd-ghost: rgba(20, 45, 71, 0.18);
  --lcd-main-text: #122b44;
  --lcd-main-glow: rgba(8, 30, 55, 0.24);
  --lcd-sub-glow: rgba(8, 30, 55, 0.2);
  --lcd-meta-text: #1d3f62;
  --theme-accent-border: #4a75b4;
  --theme-accent-soft: rgba(77, 130, 194, 0.12);
  --theme-accent-outline: rgba(108, 165, 233, 0.35);
  --theme-accent-glow: rgba(108, 165, 233, 0.2);
  --theme-slot-active: #8fb5e1;
  --theme-slot-label-active: #7da7d8;
  --theme-focus-outline: #7fb4ef;
}

.unit.theme--green {
  --lcd-border: #3f6e2f;
  --lcd-glow: rgba(224, 255, 214, 0.28);
  --lcd-top: #8cca63;
  --lcd-mid: #699f44;
  --lcd-bottom: #4c7932;
  --lcd-shadow-strong: rgba(27, 56, 16, 0.35);
  --lcd-shadow-soft: rgba(27, 56, 16, 0.15);
  --lcd-ghost: rgba(25, 49, 15, 0.18);
  --lcd-main-text: #1e3c12;
  --lcd-main-glow: rgba(15, 35, 8, 0.22);
  --lcd-sub-glow: rgba(15, 35, 8, 0.18);
  --lcd-meta-text: #2f5122;
  --theme-accent-border: #5e8d45;
  --theme-accent-soft: rgba(126, 186, 88, 0.12);
  --theme-accent-outline: rgba(150, 220, 108, 0.33);
  --theme-accent-glow: rgba(133, 201, 94, 0.2);
  --theme-slot-active: #9ecf79;
  --theme-slot-label-active: #8dc266;
  --theme-focus-outline: #a6da7e;
}

.unit.theme--purpleRed {
  --lcd-border: #7c2d57;
  --lcd-glow: rgba(255, 214, 234, 0.3);
  --lcd-top: #cb6897;
  --lcd-mid: #a54778;
  --lcd-bottom: #7a315a;
  --lcd-shadow-strong: rgba(73, 18, 45, 0.35);
  --lcd-shadow-soft: rgba(73, 18, 45, 0.15);
  --lcd-ghost: rgba(63, 18, 39, 0.18);
  --lcd-main-text: #4a1734;
  --lcd-main-glow: rgba(52, 11, 30, 0.24);
  --lcd-sub-glow: rgba(52, 11, 30, 0.2);
  --lcd-meta-text: #662347;
  --theme-accent-border: #a7487a;
  --theme-accent-soft: rgba(193, 86, 142, 0.12);
  --theme-accent-outline: rgba(220, 121, 173, 0.35);
  --theme-accent-glow: rgba(212, 103, 160, 0.2);
  --theme-slot-active: #d191b7;
  --theme-slot-label-active: #c884ad;
  --theme-focus-outline: #d98bb7;
}

/* LCD grain texture */
.lcd::before {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  opacity: 0.06;
  background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='1.2' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
  background-size: 128px 128px;
  z-index: 0;
}

/* LCD row: holds fixed character cells */
.lcd-row {
  overflow: hidden;
  white-space: nowrap;
  font-family: var(--lcd-font);
  text-transform: uppercase;
  letter-spacing: 0;
  line-height: 1;
}

/* Individual character cell — fixed position, never moves */
.lcd-cell {
  display: inline-block;
  position: relative;
  text-align: center;
  vertical-align: top;
}

/* Ghost: the ~ character sets cell width, shows unlit segments */
.lcd-cell-ghost {
  visibility: visible;
  color: var(--lcd-ghost);
  transition: color 280ms ease;
}

/* Text: absolutely positioned on top of ghost, same size */
.lcd-cell-text {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
}

/* Main row: large title */
.lcd-row--main {
  font-size: 20px;
  font-weight: 700;
}

.lcd-row--main .lcd-cell {
  letter-spacing: 0.04em;
}

.lcd-row--main .lcd-cell-text {
  color: var(--lcd-main-text);
  text-shadow: 0 0 3px var(--lcd-main-glow);
  transition:
    color 280ms ease,
    text-shadow 280ms ease;
}

/* Sub row: subtitle / show info */
.lcd-row--sub {
  font-size: 10px;
  font-weight: 700;
  margin-top: 4px;
}

.lcd-row--sub .lcd-cell-text {
  color: var(--lcd-main-text);
  text-shadow: 0 0 2px var(--lcd-sub-glow);
  transition:
    color 280ms ease,
    text-shadow 280ms ease;
}

/* Meta row: status indicators */
.lcd-row--meta {
  font-size: 8px;
  font-weight: 700;
  margin-top: 4px;
}

.lcd-row--meta .lcd-cell-ghost {
  opacity: 0.6;
}

.lcd-row--meta .lcd-cell-text {
  color: var(--lcd-meta-text);
  opacity: 0.7;
  transition: color 280ms ease;
}

/* ──────────────────────────────────────────
   PRESET GRID
   ────────────────────────────────────────── */
.preset-grid {
  flex: 1;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  grid-template-rows: repeat(2, minmax(0, 1fr));
  gap: 6px;
  min-height: 0;
  position: relative;
  z-index: 2;
}

/* ──────────────────────────────────────────
   PRESET CARD — recessed well that holds the button
   ────────────────────────────────────────── */
.preset-card {
  border-radius: 6px;
  border: 1px solid #0a0b0d;
  background: linear-gradient(180deg, #080a0c 0%, #0d0f11 100%);
  box-shadow:
    inset 0 2px 4px rgba(0, 0, 0, 0.6),
    inset 0 -1px 2px rgba(255, 255, 255, 0.03),
    0 1px 0 rgba(255, 255, 255, 0.04);
  padding: 3px;
  display: flex;
  flex-direction: column;
  transition:
    border-color 280ms ease,
    box-shadow 280ms ease;
}

.preset-card.active {
  border-color: var(--theme-accent-border);
  box-shadow:
    inset 0 2px 4px rgba(0, 0, 0, 0.5),
    inset 0 0 8px var(--theme-accent-soft),
    0 0 0 1px var(--theme-accent-outline),
    0 0 12px var(--theme-accent-glow);
}

/* ──────────────────────────────────────────
   PRESET BUTTON — raised 3D physical button
   ────────────────────────────────────────── */
.preset-button {
  -webkit-appearance: none;
  appearance: none;
  width: 100%;
  flex: 1;
  min-height: 0;
  border: none;
  border-radius: 4px;
  color: #656e76;
  background:
    linear-gradient(180deg,
      #2c3035 0%,
      #222629 30%,
      #1a1d20 70%,
      #141618 100%
    );
  box-shadow:
    /* top bevel highlight */
    inset 0 1px 0 rgba(255, 255, 255, 0.1),
    /* left/right edge highlights */
    inset 1px 0 0 rgba(255, 255, 255, 0.04),
    inset -1px 0 0 rgba(255, 255, 255, 0.04),
    /* bottom inner shadow */
    inset 0 -2px 3px rgba(0, 0, 0, 0.25),
    /* raised shadow below button */
    0 2px 3px rgba(0, 0, 0, 0.5),
    0 4px 8px rgba(0, 0, 0, 0.25);
  cursor: pointer;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  padding: 4px 4px 6px;
  transition:
    box-shadow 60ms ease,
    transform 60ms ease,
    background 60ms ease;
  position: relative;
}

/* hover: subtle surface brightening, no movement */
.preset-button:hover {
  background:
    linear-gradient(180deg,
      #333840 0%,
      #282c30 30%,
      #1e2124 70%,
      #181a1d 100%
    );
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.13),
    inset 1px 0 0 rgba(255, 255, 255, 0.05),
    inset -1px 0 0 rgba(255, 255, 255, 0.05),
    inset 0 -2px 3px rgba(0, 0, 0, 0.25),
    0 2px 3px rgba(0, 0, 0, 0.5),
    0 4px 8px rgba(0, 0, 0, 0.25);
}

/* press: button sinks into the recess */
.preset-button:active {
  transform: translateY(1.5px);
  background:
    linear-gradient(180deg,
      #1e2124 0%,
      #1a1d20 40%,
      #161819 100%
    );
  box-shadow:
    inset 0 1px 3px rgba(0, 0, 0, 0.4),
    inset 0 0 6px rgba(0, 0, 0, 0.15),
    0 0 1px rgba(0, 0, 0, 0.4);
}

.preset-button:focus-visible,
.footer-btn:focus-visible,
.context-item:focus-visible {
  outline: 1px solid var(--theme-focus-outline);
  outline-offset: 1px;
  transition: outline-color 280ms ease;
}

.slot-number {
  font-size: 28px;
  line-height: 1;
  font-weight: 800;
  color: #555e66;
  font-family: var(--display-font);
  letter-spacing: -0.01em;
  text-shadow: 0 1px 0 rgba(0, 0, 0, 0.4);
  transition: color 280ms ease;
}

.preset-card.active .slot-number {
  color: var(--theme-slot-active);
}

.slot-label {
  font-weight: 750;
  font-size: 8px;
  line-height: 1;
  letter-spacing: 0.08em;
  color: #555e66;
  text-transform: uppercase;
  font-family: var(--display-font);
  text-shadow: 0 1px 0 rgba(0, 0, 0, 0.3);
  max-width: 90%;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  text-align: center;
  transition: color 280ms ease;
}

.preset-card.active .slot-label {
  color: var(--theme-slot-label-active);
}

.preset-card.empty .slot-number {
  color: #3a4248;
}

.preset-card.empty .slot-label {
  color: #3a4248;
}

/* ──────────────────────────────────────────
   FOOTER — integrated control strip
   ────────────────────────────────────────── */
.unit-footer {
  margin-top: 0;
  padding-top: 4px;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  display: flex;
  justify-content: space-between;
  gap: 8px;
  align-items: center;
  position: relative;
  z-index: 3;
}

.tagline {
  margin: 0;
  color: #9a9080;
  font-style: italic;
  font-weight: 700;
  font-size: 9px;
  letter-spacing: 0.075em;
  font-family: "Times New Roman", Georgia, serif;
  text-transform: uppercase;
  text-shadow: 0 1px 0 rgba(0, 0, 0, 0.4);
}

.footer-controls {
  display: flex;
  align-items: center;
  gap: 4px;
}

.footer-btn {
  -webkit-appearance: none;
  appearance: none;
  border-radius: 3px;
  border: 1px solid #2e3338;
  background:
    linear-gradient(180deg, #272b2f 0%, #1c1f22 100%);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.07),
    0 1px 2px rgba(0, 0, 0, 0.35);
  color: #b0b6bc;
  font-size: 7px;
  font-weight: 760;
  padding: 3px 7px;
  letter-spacing: 0.09em;
  cursor: pointer;
  text-transform: uppercase;
  font-family: var(--display-font);
  transition:
    box-shadow 60ms ease,
    transform 60ms ease;
}

.footer-btn:active {
  transform: translateY(0.5px);
  box-shadow:
    inset 0 1px 2px rgba(0, 0, 0, 0.3),
    0 0 1px rgba(0, 0, 0, 0.3);
}

.footer-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.model {
  margin: 0;
  border: 1px solid #363b40;
  background:
    linear-gradient(180deg, #1e2124 0%, #181b1e 100%);
  box-shadow:
    inset 0 1px 2px rgba(0, 0, 0, 0.2),
    0 1px 0 rgba(255, 255, 255, 0.03);
  color: #a0a8ae;
  border-radius: 3px;
  padding: 2px 7px;
  font-weight: 800;
  letter-spacing: 0.12em;
  font-family: var(--display-font);
  font-size: 10px;
  text-shadow: 0 1px 0 rgba(0, 0, 0, 0.4);
}

audio {
  display: none;
}

/* ──────────────────────────────────────────
   CONTEXT MENU — hardware-style recessed panel
   ────────────────────────────────────────── */
.context-backdrop {
  position: absolute;
  inset: 0;
  z-index: 20;
}

.context-menu {
  position: fixed;
  min-width: 200px;
  max-width: 240px;
  max-height: 220px;
  border-radius: 6px;
  border: 1px solid #2a2e33;
  background:
    linear-gradient(180deg, #1c2024 0%, #141719 100%);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.05),
    0 8px 24px rgba(0, 0, 0, 0.65),
    0 2px 6px rgba(0, 0, 0, 0.4);
  padding: 5px;
  z-index: 24;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.context-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 3px;
  scrollbar-width: thin;
  scrollbar-color: #3a4046 transparent;
}

.context-scroll::-webkit-scrollbar {
  width: 4px;
}

.context-scroll::-webkit-scrollbar-track {
  background: transparent;
}

.context-scroll::-webkit-scrollbar-thumb {
  background: #3a4046;
  border-radius: 2px;
}

.context-title {
  margin: 0;
  padding: 4px 8px 5px;
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: #888e94;
  font-family: var(--display-font);
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

.context-item {
  -webkit-appearance: none;
  appearance: none;
  border: 1px solid #252a2f;
  border-radius: 4px;
  background:
    linear-gradient(180deg, #1a1e22 0%, #141719 100%);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.04),
    0 1px 2px rgba(0, 0, 0, 0.3);
  color: #c8cdd2;
  text-align: left;
  font-size: 11px;
  font-weight: 650;
  padding: 7px 9px;
  cursor: pointer;
  font-family: var(--ui-font);
  transition:
    box-shadow 60ms ease,
    transform 60ms ease;
}

.context-item:active {
  transform: translateY(0.5px);
  box-shadow:
    inset 0 1px 2px rgba(0, 0, 0, 0.3);
}

.context-item.danger {
  color: #e89488;
  border-color: #3a2826;
}

@media (max-width: 420px) {
  .context-menu {
    max-width: 180px;
    min-width: 160px;
  }
}
</style>
