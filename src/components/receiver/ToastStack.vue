<script setup lang="ts">
import type { ToastItem } from "../../shazam/types";

defineProps<{
  toasts: ToastItem[];
}>();

const emit = defineEmits<{
  dismiss: [id: number];
}>();
</script>

<template>
  <div class="toast-stack" aria-live="polite" aria-atomic="true">
    <div
      v-for="toast in toasts"
      :key="toast.id"
      class="toast"
      :class="`toast--${toast.kind}`"
      role="status"
    >
      <p>{{ toast.message }}</p>
      <button
        type="button"
        class="toast-dismiss"
        :aria-label="`Dismiss notification: ${toast.message}`"
        @click="emit('dismiss', toast.id)"
      >
        ×
      </button>
    </div>
  </div>
</template>

<style scoped>
.toast-stack {
  position: absolute;
  right: 10px;
  bottom: 10px;
  z-index: 220;
  display: flex;
  flex-direction: column;
  gap: 6px;
  pointer-events: none;
}

.toast {
  width: min(280px, calc(100vw - 30px));
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 6px;
  background: rgba(19, 24, 30, 0.92);
  color: #d9e1ea;
  box-shadow: 0 8px 18px rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 7px 8px;
  pointer-events: auto;
}

.toast p {
  margin: 0;
  font-size: 10px;
  line-height: 1.3;
}

.toast--success {
  border-color: rgba(119, 204, 142, 0.65);
}

.toast--info {
  border-color: rgba(142, 179, 219, 0.65);
}

.toast--error {
  border-color: rgba(227, 118, 118, 0.72);
}

.toast-dismiss {
  -webkit-appearance: none;
  appearance: none;
  border: none;
  background: transparent;
  color: #9facba;
  font-size: 14px;
  line-height: 1;
  cursor: pointer;
  padding: 0 2px;
}

.toast-dismiss:hover {
  color: #d5dce4;
}

.toast-dismiss:focus-visible {
  outline: 1px solid var(--theme-focus-outline);
  outline-offset: 1px;
}
</style>
