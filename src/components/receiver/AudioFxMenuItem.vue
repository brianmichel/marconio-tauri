<script setup lang="ts">
defineProps<{ label: string }>();
defineEmits<{ (event: "cycle"): void }>();
</script>

<template>
  <button type="button" class="model-menu-item" @click="$emit('cycle')">
    SOUND: {{ label.toUpperCase() }}
  </button>
</template>
