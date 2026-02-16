<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";

const props = defineProps<{
  visible: boolean;
  menuBarOnlyMode: boolean;
  isTrayModeSupported: boolean;
  isMacPlatform: boolean;
  isWindowsPlatform: boolean;
}>();

const emit = defineEmits<{
  close: [];
  setMenuBarOnlyMode: [enabled: boolean];
}>();

const panelRef = ref<HTMLElement | null>(null);
const closeButtonRef = ref<HTMLButtonElement | null>(null);

const appModeHint = computed(() => {
  if (props.isMacPlatform) {
    return "Hides Dock icon when set to tray";
  }

  if (props.isWindowsPlatform) {
    return "Hides taskbar icon when set to tray";
  }

  return "Available on macOS and Windows";
});

const settingsNote = computed(() => {
  if (props.isMacPlatform) {
    return "Window close keeps Marconio in the menu bar.";
  }

  if (props.isWindowsPlatform) {
    return "Window close keeps Marconio in the system tray.";
  }

  return "Tray mode available on macOS & Windows.";
});

function closePanel() {
  emit("close");
}

function setMenuBarOnlyModeEnabled(enabled: boolean) {
  emit("setMenuBarOnlyMode", enabled);
}

function handleSettingsKeydown(event: KeyboardEvent) {
  if (event.key !== "Tab") {
    return;
  }

  const panel = panelRef.value;
  if (!panel) {
    return;
  }

  const focusable = Array.from(
    panel.querySelectorAll<HTMLElement>(
      'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
    ),
  );
  if (focusable.length === 0) {
    return;
  }

  const first = focusable[0];
  const last = focusable[focusable.length - 1];

  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
    return;
  }

  if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

function handleRockerKeydown(event: KeyboardEvent) {
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") {
    return;
  }

  event.preventDefault();

  if (!props.isTrayModeSupported) {
    return;
  }

  const next = event.key === "ArrowRight";
  setMenuBarOnlyModeEnabled(next);

  void nextTick(() => {
    const group = (event.target as HTMLElement).closest(".setting-rocker");
    const active = group?.querySelector<HTMLElement>('[aria-checked="true"]');
    active?.focus();
  });
}

watch(
  () => props.visible,
  (visible) => {
    if (!visible) {
      return;
    }

    void nextTick(() => {
      closeButtonRef.value?.focus();
    });
  },
);
</script>

<template>
  <div
    v-if="props.visible"
    class="settings-backdrop"
    aria-hidden="true"
    @mousedown="closePanel"
  />
  <section
    v-if="props.visible"
    ref="panelRef"
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
        ref="closeButtonRef"
        type="button"
        class="settings-close-button"
        aria-label="Close settings"
        @click="closePanel"
      >
        &times;
      </button>
    </header>

    <div class="settings-list">
      <div class="setting-row">
        <div class="setting-label-row" id="appmode-label">
          <p class="setting-name">APP MODE</p>
          <p class="setting-hint" id="appmode-hint">
            {{ appModeHint }}
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
            :class="{ 'rocker-key--active': !props.menuBarOnlyMode }"
            :aria-checked="!props.menuBarOnlyMode"
            :tabindex="!props.menuBarOnlyMode ? 0 : -1"
            :disabled="!props.isTrayModeSupported"
            @click="setMenuBarOnlyModeEnabled(false)"
          >
            DOCK
          </button>
          <button
            type="button"
            role="radio"
            class="rocker-key"
            :class="{ 'rocker-key--active': props.menuBarOnlyMode }"
            :aria-checked="props.menuBarOnlyMode"
            :tabindex="props.menuBarOnlyMode ? 0 : -1"
            :disabled="!props.isTrayModeSupported"
            @click="setMenuBarOnlyModeEnabled(true)"
          >
            TRAY
          </button>
        </div>
      </div>
    </div>

    <p class="settings-note">
      {{ settingsNote }}
    </p>
  </section>
</template>

<style scoped>
.settings-backdrop {
  position: fixed;
  inset: 0;
  z-index: 150;
  background: rgba(5, 8, 11, 0.5);
  backdrop-filter: blur(3px);
}

.settings-panel {
  position: absolute;
  top: 34px;
  left: 10px;
  right: 10px;
  z-index: 160;
  border: 1px solid #3b4045;
  border-radius: 5px;
  padding: 8px 10px 8px;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.04) 0%, transparent 28%),
    linear-gradient(180deg, #252a2f 0%, #1d2125 100%);
  box-shadow:
    0 8px 22px rgba(0, 0, 0, 0.46),
    inset 0 1px 0 rgba(255, 255, 255, 0.08);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.settings-header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.settings-header-rule {
  flex: 1;
  height: 1px;
  background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.1), transparent);
}

.settings-title {
  margin: 0;
  font-family: var(--display-font);
  font-size: 9px;
  letter-spacing: 0.22em;
  font-weight: 700;
  color: #8a9099;
  flex-shrink: 0;
}

.settings-close-button {
  -webkit-appearance: none;
  appearance: none;
  border: none;
  border-radius: 0;
  background: none;
  color: #7e858e;
  font-size: 16px;
  line-height: 1;
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  flex-shrink: 0;
  padding: 0;
}

.settings-close-button:hover {
  color: #c2c8cf;
}

.settings-close-button:active {
  color: #f0f0f0;
}

.settings-close-button:focus-visible {
  outline: 1px solid var(--theme-focus-outline);
  outline-offset: 1px;
}

.settings-list {
  display: flex;
  flex-direction: column;
}

.settings-list::before {
  content: "";
  display: block;
  height: 1px;
  background: rgba(255, 255, 255, 0.05);
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 7px 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.setting-label-row {
  min-width: 0;
  flex: 1;
}

.setting-name {
  margin: 0;
  color: #b8bfc6;
  font-family: var(--display-font);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.1em;
}

.setting-hint {
  margin: 2px 0 0;
  color: #848b94;
  font-size: 9px;
  letter-spacing: 0.02em;
  line-height: 1.3;
}

.setting-rocker {
  display: flex;
  border: 1px solid #3e444a;
  border-radius: 3px;
  overflow: hidden;
  background: #1a1e22;
  box-shadow:
    inset 0 2px 4px rgba(0, 0, 0, 0.35),
    0 1px 0 rgba(255, 255, 255, 0.04);
  flex-shrink: 0;
}

.rocker-key {
  -webkit-appearance: none;
  appearance: none;
  border: none;
  background: transparent;
  color: #7e858e;
  font-family: var(--display-font);
  font-size: 9px;
  letter-spacing: 0.08em;
  font-weight: 700;
  padding: 5px 11px;
  cursor: pointer;
  text-transform: uppercase;
  position: relative;
}

.rocker-key + .rocker-key {
  border-left: 1px solid #2e3338;
}

.rocker-key:focus-visible {
  outline: 1px solid var(--theme-focus-outline);
  outline-offset: -1px;
}

.rocker-key:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.rocker-key:not(:disabled):hover {
  color: #8a9099;
}

.rocker-key:not(:disabled):active {
  background: rgba(255, 255, 255, 0.03);
}

.rocker-key--active {
  color: #d8dee5;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.06) 0%, transparent 60%),
    linear-gradient(180deg, #303840 0%, #282e35 100%);
  text-shadow: 0 0 8px var(--theme-accent-glow);
  box-shadow: inset 0 0 0 1px var(--theme-accent-outline);
}

.rocker-key--active::after {
  content: "";
  position: absolute;
  bottom: 0;
  left: 20%;
  right: 20%;
  height: 1px;
  background: var(--theme-accent-border);
  box-shadow: 0 0 4px var(--theme-accent-glow);
}

.settings-note {
  margin: 0;
  color: #7e858e;
  font-size: 9px;
  letter-spacing: 0.02em;
  line-height: 1.4;
}
</style>
