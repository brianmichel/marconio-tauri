<script setup lang="ts">
import type { AudioFxPreset, AUDIO_FX_PRESETS } from "../../audio/fxPresets";

defineProps<{
  modelValue: AudioFxPreset;
  presets: typeof AUDIO_FX_PRESETS;
}>();

defineEmits<{
  (event: "update:modelValue", value: AudioFxPreset): void;
}>();
</script>

<template>
  <div class="fx-segmented" role="group" aria-label="Audio effects">
    <button
      v-for="preset in presets"
      :key="preset.id"
      type="button"
      class="fx-segmented__button"
      :class="{ 'fx-segmented__button--active': modelValue === preset.id }"
      :aria-label="`Set audio effect to ${preset.label}`"
      :aria-pressed="modelValue === preset.id ? 'true' : 'false'"
      @click="$emit('update:modelValue', preset.id)"
    >
      <span class="fx-segmented__led" aria-hidden="true" />
      {{ preset.label }}
    </button>
  </div>
</template>

<style scoped>
.fx-segmented {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 5px;
  padding: 2px 0;
}

.fx-segmented__button {
  -webkit-appearance: none;
  appearance: none;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-bottom-color: rgba(0, 0, 0, 0.3);
  border-radius: 3px;
  background:
    linear-gradient(
      180deg,
      rgba(60, 65, 72, 0.9) 0%,
      rgba(36, 40, 46, 0.95) 40%,
      rgba(30, 34, 40, 0.95) 100%
    );
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.1),
    0 2px 4px rgba(0, 0, 0, 0.5),
    0 1px 1px rgba(0, 0, 0, 0.3);
  color: #6e747a;
  font-family: "SF Pro Display", -apple-system, BlinkMacSystemFont, "Helvetica Neue", sans-serif;
  font-size: 7.5px;
  font-weight: 800;
  letter-spacing: 0.1em;
  padding: 8px 4px 6px;
  text-transform: uppercase;
  cursor: pointer;
  transition: all 120ms ease;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 5px;
}

.fx-segmented__led {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: #1a1c1e;
  border: 1px solid rgba(255, 255, 255, 0.05);
  box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.5);
  transition: all 180ms ease;
}

.fx-segmented__button:hover {
  background:
    linear-gradient(
      180deg,
      rgba(70, 76, 84, 0.95) 0%,
      rgba(42, 47, 54, 0.95) 40%,
      rgba(36, 40, 46, 0.95) 100%
    );
  color: #a0a6ac;
}

.fx-segmented__button:active {
  box-shadow:
    inset 0 1px 3px rgba(0, 0, 0, 0.4),
    0 0 1px rgba(0, 0, 0, 0.2);
  transform: translateY(1px);
}

.fx-segmented__button.fx-segmented__button--active {
  color: #c8d0d8;
  border-color: rgba(255, 255, 255, 0.12);
}

.fx-segmented__button.fx-segmented__button--active .fx-segmented__led {
  background: var(--theme-accent-border);
  border-color: transparent;
  box-shadow:
    0 0 4px var(--theme-accent-glow),
    0 0 10px var(--theme-accent-glow),
    inset 0 -1px 1px rgba(0, 0, 0, 0.2);
}

.fx-segmented__button:focus-visible {
  outline: 1px solid var(--theme-focus-outline);
  outline-offset: 1px;
}
</style>
