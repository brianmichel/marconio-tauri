<script setup lang="ts">
defineProps<{
  visible: boolean;
  isLoading: boolean;
  isPlaying: boolean;
}>();

const emit = defineEmits<{
  toggle: [];
  close: [];
  refresh: [];
  settings: [];
  stop: [];
}>();

function onToggle() {
  emit("toggle");
}

function onClose() {
  emit("close");
}

function onRefresh() {
  emit("refresh");
}

function onSettings() {
  emit("settings");
}

function onStop() {
  emit("stop");
}
</script>

<template>
  <div class="model-wrap">
    <button
      type="button"
      class="brand-plate"
      aria-haspopup="menu"
      aria-controls="receiver-model-menu"
      :aria-expanded="visible ? 'true' : 'false'"
      @click="onToggle"
    >
      <span class="brand">MRC-1900</span>
    </button>
    <div v-if="visible" class="model-backdrop" @mousedown="onClose" />
    <div
      v-if="visible"
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
        @click="onRefresh"
      >
        {{ isLoading ? "SYNCING..." : "REFRESH" }}
      </button>
      <button
        type="button"
        class="model-menu-item"
        role="menuitem"
        @click="onSettings"
      >
        SETTINGS
      </button>
      <button
        type="button"
        class="model-menu-item"
        role="menuitem"
        :disabled="!isPlaying"
        @click="onStop"
      >
        STOP
      </button>
    </div>
  </div>
</template>

<style scoped>
.model-wrap {
  position: relative;
  z-index: 12;
}

.brand-plate {
  -webkit-appearance: none;
  appearance: none;
  background: linear-gradient(180deg, #b0b4b8 0%, #7a7e82 40%, #8a8e92 60%, #a0a4a8 100%);
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-bottom-color: rgba(0, 0, 0, 0.2);
  border-radius: 2px;
  padding: 2px 16px;
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.4),
    inset 0 -1px 0 rgba(0, 0, 0, 0.1),
    0 1px 3px rgba(0, 0, 0, 0.4);
  cursor: pointer;
}

.brand-plate:active {
  box-shadow:
    inset 0 1px 2px rgba(0, 0, 0, 0.15),
    0 0 1px rgba(0, 0, 0, 0.2);
}

.brand-plate:focus-visible {
  outline: 1px solid var(--theme-focus-outline);
  outline-offset: 1px;
}

.brand {
  margin: 0;
  font-size: 11px;
  font-weight: 800;
  letter-spacing: 0.2em;
  color: #2a2d30;
  font-family: var(--display-font);
  text-transform: uppercase;
  text-shadow: 0 1px 0 rgba(255, 255, 255, 0.3);
}

.model-menu-item:focus-visible {
  outline: 1px solid var(--theme-focus-outline);
  outline-offset: 1px;
  transition: outline-color 280ms ease;
}

.model-backdrop {
  position: fixed;
  inset: 0;
  z-index: 99;
}

.model-menu {
  position: absolute;
  top: calc(100% + 5px);
  left: 50%;
  transform: translateX(-50%);
  background: linear-gradient(180deg, #2c3035 0%, #222629 100%);
  border: 1px solid #3a3f44;
  border-radius: 4px;
  box-shadow:
    0 4px 16px rgba(0, 0, 0, 0.5),
    inset 0 1px 0 rgba(255, 255, 255, 0.06);
  padding: 3px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  z-index: 140;
  min-width: 90px;
}

.model-menu-item {
  -webkit-appearance: none;
  appearance: none;
  border: none;
  border-radius: 3px;
  background: transparent;
  color: #b0b6bc;
  font-size: 10px;
  font-weight: 700;
  padding: 5px 10px;
  letter-spacing: 0.06em;
  cursor: pointer;
  text-transform: uppercase;
  font-family: var(--display-font);
  text-align: left;
  white-space: nowrap;
}

.model-menu-item:hover {
  background: rgba(255, 255, 255, 0.06);
}

.model-menu-item:active {
  background: rgba(255, 255, 255, 0.1);
}

.model-menu-item:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}
</style>
