<script setup lang="ts">
const props = defineProps<{
  slot: number;
  label: string;
  active: boolean;
  locked: boolean;
  empty: boolean;
  setButtonRef?: (element: unknown) => void;
}>();

const emit = defineEmits<{
  press: [slot: number];
  openContext: [payload: { event: MouseEvent; slot: number; locked: boolean }];
}>();

function onPress() {
  emit("press", props.slot);
}

function onContextMenu(event: MouseEvent) {
  emit("openContext", {
    event,
    slot: props.slot,
    locked: props.locked,
  });
}
</script>

<template>
  <article
    class="preset-card"
    :class="{
      active: props.active,
      locked: props.locked,
      empty: props.empty,
    }"
  >
    <button
      type="button"
      class="preset-button"
      :ref="props.setButtonRef"
      @click="onPress"
      @contextmenu.prevent="onContextMenu"
    >
      <span class="slot-number">{{ props.slot }}</span>
      <span class="slot-label">
        {{ props.label }}
      </span>
    </button>
  </article>
</template>

<style scoped>
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

.preset-button {
  -webkit-appearance: none;
  appearance: none;
  width: 100%;
  flex: 1;
  min-height: 0;
  border: none;
  border-radius: 4px;
  color: #656e76;
  background: linear-gradient(180deg, #2c3035 0%, #222629 30%, #1a1d20 70%, #141618 100%);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.1),
    inset 1px 0 0 rgba(255, 255, 255, 0.04),
    inset -1px 0 0 rgba(255, 255, 255, 0.04),
    inset 0 -2px 3px rgba(0, 0, 0, 0.25),
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

.preset-button:hover {
  background: linear-gradient(180deg, #333840 0%, #282c30 30%, #1e2124 70%, #181a1d 100%);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.13),
    inset 1px 0 0 rgba(255, 255, 255, 0.05),
    inset -1px 0 0 rgba(255, 255, 255, 0.05),
    inset 0 -2px 3px rgba(0, 0, 0, 0.25),
    0 2px 3px rgba(0, 0, 0, 0.5),
    0 4px 8px rgba(0, 0, 0, 0.25);
}

.preset-button:active {
  transform: translateY(1.5px);
  background: linear-gradient(180deg, #1e2124 0%, #1a1d20 40%, #161819 100%);
  box-shadow:
    inset 0 1px 3px rgba(0, 0, 0, 0.4),
    inset 0 0 6px rgba(0, 0, 0, 0.15),
    0 0 1px rgba(0, 0, 0, 0.4);
}

.preset-button:focus-visible {
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
</style>
