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
    <p class="context-title">Preset {{ props.slot }}</p>
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
    <button v-if="props.hasAssignment" type="button" class="context-item danger" @click="clearSlot">
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
  max-height: 190px;
  border-radius: 5px;
  border: 1px solid #2a2e33;
  background: linear-gradient(180deg, #1c2024 0%, #141719 100%);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.05),
    0 8px 24px rgba(0, 0, 0, 0.65),
    0 2px 6px rgba(0, 0, 0, 0.4);
  padding: 5px;
  z-index: 24;
  display: flex;
  flex-direction: column;
  gap: 2px;
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

.context-title {
  margin: 0;
  padding: 3px 6px 4px;
  font-size: 8px;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: #888e94;
  font-family: var(--display-font);
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.context-item {
  -webkit-appearance: none;
  appearance: none;
  border: 1px solid #252a2f;
  border-radius: 3px;
  background: linear-gradient(180deg, #1a1e22 0%, #141719 100%);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.04),
    0 1px 2px rgba(0, 0, 0, 0.3);
  color: #c8cdd2;
  text-align: left;
  font-size: 9px;
  font-weight: 650;
  padding: 5px 7px;
  cursor: pointer;
  font-family: var(--ui-font);
  transition:
    box-shadow 60ms ease,
    transform 60ms ease;
}

.context-item:active {
  transform: translateY(0.5px);
  box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.3);
}

.context-item:focus-visible {
  outline: 1px solid var(--theme-focus-outline);
  outline-offset: 1px;
  transition: outline-color 280ms ease;
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
