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
  openSlotContextByKeyboard: [slot: number];
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

function accessibleLabelForCard(card: PresetGridCard): string {
  if (card.locked) {
    const source = card.playable ? card.playable.title : `Channel ${card.slot}`;
    return `Channel ${card.slot}. ${source}. Press to play.`;
  }

  if (card.playable) {
    return `Preset ${card.slot}. ${card.playable.title}. Press to play.`;
  }

  return `Preset ${card.slot} is unassigned. Press to choose a mixtape.`;
}

function onPressSlot(slot: number) {
  emit("pressSlot", slot);
}

function onOpenSlotContext(payload: { event: MouseEvent; slot: number; locked: boolean }) {
  emit("openSlotContext", payload);
}

function onOpenSlotContextByKeyboard(slot: number) {
  emit("openSlotContextByKeyboard", slot);
}
</script>

<template>
  <section class="preset-grid" aria-label="Preset selector">
    <PresetButton
      v-for="card in props.cards"
      :key="card.slot"
      :slot="card.slot"
      :label="labelForCard(card)"
      :accessible-label="accessibleLabelForCard(card)"
      :active="props.activeSlot === card.slot"
      :locked="card.locked"
      :empty="!card.playable"
      :set-button-ref="(el) => props.setButtonRef?.(card.slot, el)"
      @press="onPressSlot"
      @open-context="onOpenSlotContext"
      @open-context-by-keyboard="onOpenSlotContextByKeyboard"
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
