<script setup lang="ts">
import PresetButton from "./PresetButton.vue";

interface PresetGridPlayable {
  title: string;
}

interface PresetGridCard {
  slot: number;
  locked: boolean;
  playable: PresetGridPlayable | null;
}

const props = defineProps<{
  cards: PresetGridCard[];
  activeSlot: number | null;
  setButtonRef?: (slot: number, element: unknown) => void;
}>();

const emit = defineEmits<{
  pressSlot: [slot: number];
  openSlotContext: [payload: { event: MouseEvent; slot: number; locked: boolean }];
}>();

function labelForCard(card: PresetGridCard): string {
  if (card.locked) {
    return "CH";
  }

  if (card.playable) {
    return card.playable.title.toUpperCase();
  }

  return "+";
}

function onPressSlot(slot: number) {
  emit("pressSlot", slot);
}

function onOpenSlotContext(payload: { event: MouseEvent; slot: number; locked: boolean }) {
  emit("openSlotContext", payload);
}
</script>

<template>
  <section class="preset-grid" aria-label="Preset selector">
    <PresetButton
      v-for="card in props.cards"
      :key="card.slot"
      :slot="card.slot"
      :label="labelForCard(card)"
      :active="props.activeSlot === card.slot"
      :locked="card.locked"
      :empty="!card.playable"
      :set-button-ref="(el) => props.setButtonRef?.(card.slot, el)"
      @press="onPressSlot"
      @open-context="onOpenSlotContext"
    />
  </section>
</template>

<style scoped>
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
</style>
