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
      @click="$emit('update:modelValue', preset.id)"
    >
      {{ preset.label }}
    </button>
  </div>
</template>

<style scoped>
.fx-segmented {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 3px;
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 5px;
  padding: 3px;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.06) 0%, rgba(255, 255, 255, 0.01) 100%),
    rgba(5, 7, 10, 0.55);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.08),
    inset 0 -1px 0 rgba(0, 0, 0, 0.45);
}

.fx-segmented__button {
  -webkit-appearance: none;
  appearance: none;
  border: 1px solid transparent;
  border-radius: 3px;
  background: rgba(34, 39, 46, 0.65);
  color: #9fa6ae;
  font-family: "SF Pro Display", -apple-system, BlinkMacSystemFont, "Helvetica Neue", sans-serif;
  font-size: 8px;
  font-weight: 800;
  letter-spacing: 0.12em;
  padding: 5px 4px;
  text-transform: uppercase;
  cursor: pointer;
  transition: background-color 140ms ease, color 140ms ease, border-color 140ms ease;
}

.fx-segmented__button:hover {
  background: rgba(52, 61, 72, 0.82);
  color: #d6dbe1;
}

.fx-segmented__button--active {
  background: rgba(158, 125, 95, 0.24);
  border-color: var(--theme-accent-border);
  color: #e9d3bb;
  text-shadow: 0 0 6px rgba(233, 211, 187, 0.2);
}

.fx-segmented__button:focus-visible {
  outline: 1px solid var(--theme-focus-outline);
  outline-offset: 1px;
}
</style>
