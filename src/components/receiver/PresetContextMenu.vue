<script setup lang="ts">
interface PresetOption {
  alias: string;
  title: string;
}

const props = defineProps<{
  visible: boolean;
  slot: number | null;
  x: number;
  y: number;
  options: PresetOption[];
  hasAssignment: boolean;
}>();

const emit = defineEmits<{
  close: [];
  assign: [alias: string];
  clear: [];
}>();

function closeMenu() {
  emit("close");
}

function assign(alias: string) {
  emit("assign", alias);
}

function clearSlot() {
  emit("clear");
}
</script>

<template>
  <div
    v-if="props.visible"
    class="context-backdrop"
    @mousedown="closeMenu"
    @contextmenu.prevent="closeMenu"
  />
  <section
    v-if="props.visible && props.slot !== null"
    class="context-menu"
    :style="{ left: `${props.x}px`, top: `${props.y}px` }"
    @mousedown.stop
    @contextmenu.prevent
  >
    <div class="context-header">
      <span class="context-led" aria-hidden="true" />
      <p class="context-title">Preset {{ props.slot }}</p>
    </div>
    <div class="context-scroll">
      <button
        v-for="option in props.options"
        :key="option.alias"
        type="button"
        class="context-item"
        @click="assign(option.alias)"
      >
        {{ option.title }}
      </button>
    </div>
    <button v-if="props.hasAssignment" type="button" class="context-item context-item--danger" @click="clearSlot">
      Clear Preset
    </button>
  </section>
</template>

<style scoped>
.context-backdrop {
  position: absolute;
  inset: 0;
  z-index: 20;
}

.context-menu {
  position: fixed;
  min-width: 190px;
  max-width: 220px;
  max-height: 210px;
  border-radius: 4px;
  border: 1px solid rgba(180, 185, 192, 0.25);
  background:
    linear-gradient(
      180deg,
      rgba(160, 165, 172, 0.1) 0%,
      rgba(100, 105, 112, 0.05) 2px,
      rgba(14, 16, 20, 0.97) 2px,
      rgba(10, 12, 16, 0.98) 100%
    );
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.18),
    inset 0 -1px 0 rgba(0, 0, 0, 0.3),
    0 12px 32px rgba(0, 0, 0, 0.7),
    0 2px 8px rgba(0, 0, 0, 0.5),
    0 0 0 0.5px rgba(0, 0, 0, 0.5);
  padding: 5px;
  z-index: 24;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.context-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 6px 5px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

.context-led {
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: var(--theme-accent-border);
  box-shadow:
    0 0 4px var(--theme-accent-glow),
    0 0 8px var(--theme-accent-glow);
  flex-shrink: 0;
}

.context-title {
  margin: 0;
  font-size: 8px;
  font-weight: 800;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: #8a9098;
  font-family: var(--display-font);
}

.context-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 2px;
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

.context-item {
  -webkit-appearance: none;
  appearance: none;
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-bottom-color: rgba(0, 0, 0, 0.2);
  border-radius: 3px;
  background:
    linear-gradient(
      180deg,
      rgba(42, 46, 54, 0.8) 0%,
      rgba(28, 32, 38, 0.85) 40%,
      rgba(22, 25, 30, 0.9) 100%
    );
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.07),
    0 1px 2px rgba(0, 0, 0, 0.3);
  color: #c0c6cc;
  text-align: left;
  font-size: 9px;
  font-weight: 700;
  padding: 6px 8px;
  cursor: pointer;
  font-family: var(--ui-font);
  transition:
    background 80ms ease,
    box-shadow 60ms ease,
    transform 60ms ease;
}

.context-item:hover {
  background:
    linear-gradient(
      180deg,
      rgba(52, 58, 66, 0.85) 0%,
      rgba(36, 40, 48, 0.9) 40%,
      rgba(28, 32, 38, 0.9) 100%
    );
  color: #e0e4e8;
}

.context-item:active {
  transform: translateY(0.5px);
  background:
    linear-gradient(
      180deg,
      rgba(20, 23, 28, 0.9) 0%,
      rgba(16, 18, 22, 0.95) 100%
    );
  box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.4);
}

.context-item:focus-visible {
  outline: 1px solid var(--theme-focus-outline);
  outline-offset: 1px;
  transition: outline-color 280ms ease;
}

.context-item--danger {
  color: #d4887e;
  border-color: rgba(180, 80, 65, 0.15);
  margin-top: 1px;
  border-top: 1px solid rgba(255, 255, 255, 0.04);
}

.context-item--danger:hover {
  color: #eca090;
  background:
    linear-gradient(
      180deg,
      rgba(60, 36, 32, 0.7) 0%,
      rgba(40, 24, 22, 0.8) 100%
    );
}

@media (max-width: 420px) {
  .context-menu {
    max-width: 180px;
    min-width: 160px;
  }
}
</style>
