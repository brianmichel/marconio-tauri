<script setup lang="ts">
defineProps<{
  visible: boolean;
}>();

const emit = defineEmits<{
  dismiss: [];
  learnMore: [];
}>();

function onDismiss() {
  emit("dismiss");
}

function onLearnMore() {
  emit("learnMore");
}
</script>

<template>
  <div v-if="visible" class="support-backdrop" aria-hidden="true" />
  <section
    v-if="visible"
    class="support-dialog"
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="support-dialog-title"
    aria-describedby="support-dialog-body"
    @keydown.stop
  >
    <div class="support-dialog-lcd" aria-hidden="true">
      <span class="support-dialog-freq">SUPPORT NTS</span>
    </div>
    <p id="support-dialog-body" class="support-dialog-body">
      Marconio streams from directly from NTS, consider
      becoming a supporter to keep independent radio alive & accessible.
    </p>
    <div class="support-dialog-actions">
      <button
        type="button"
        class="support-dialog-btn support-dialog-btn--primary"
        @click="onLearnMore"
      >
        LEARN MORE
      </button>
      <button
        type="button"
        class="support-dialog-btn support-dialog-btn--secondary"
        @click="onDismiss"
      >
        CONTINUE
      </button>
    </div>
  </section>
</template>

<style scoped>
.support-backdrop {
  position: fixed;
  inset: 0;
  z-index: 200;
  background: rgba(5, 8, 11, 0.7);
  backdrop-filter: blur(4px);
}

.support-dialog {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  z-index: 210;
  width: 260px;
  border: 1px solid #3b4045;
  border-radius: 6px;
  padding: 14px 16px 12px;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.05) 0%, transparent 30%),
    linear-gradient(180deg, #272c31 0%, #1c2024 100%);
  box-shadow:
    0 12px 32px rgba(0, 0, 0, 0.55),
    inset 0 1px 0 rgba(255, 255, 255, 0.09);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  text-align: center;
}

.support-dialog-lcd {
  width: 100%;
  height: 32px;
  border: 1px solid var(--lcd-border);
  border-radius: 3px;
  background: linear-gradient(180deg, var(--lcd-top) 0%, var(--lcd-mid) 50%, var(--lcd-bottom) 100%);
  box-shadow:
    inset 0 1px 2px var(--lcd-shadow-strong),
    0 0 8px var(--lcd-outer-glow);
  display: flex;
  align-items: center;
  justify-content: center;
}

.support-dialog-freq {
  font-family: var(--lcd-font);
  font-size: 16px;
  font-weight: bold;
  color: var(--lcd-main-text);
  text-shadow: 0 0 4px var(--lcd-main-glow);
  letter-spacing: 0.15em;
}

.support-dialog-body {
  margin: 0;
  font-size: 11px;
  line-height: 1.5;
  color: #8e959e;
}

.support-dialog-actions {
  display: flex;
  gap: 8px;
  width: 100%;
  margin-top: 2px;
}

.support-dialog-btn {
  -webkit-appearance: none;
  appearance: none;
  flex: 1;
  border: 1px solid #3e444a;
  border-radius: 4px;
  padding: 7px 0;
  font-family: var(--display-font);
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.1em;
  cursor: pointer;
  text-transform: uppercase;
}

.support-dialog-btn--primary {
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.06) 0%, transparent 50%),
    linear-gradient(180deg, #353c44 0%, #2a3138 100%);
  color: #d8dee5;
  border-color: var(--theme-accent-outline);
  text-shadow: 0 0 6px var(--theme-accent-glow);
}

.support-dialog-btn--primary:hover {
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.09) 0%, transparent 50%),
    linear-gradient(180deg, #3c444d 0%, #303840 100%);
}

.support-dialog-btn--primary:active {
  background: linear-gradient(180deg, #2a3038 0%, #242a30 100%);
}

.support-dialog-btn--primary:focus-visible {
  outline: 1px solid var(--theme-focus-outline);
  outline-offset: 1px;
}

.support-dialog-btn--secondary {
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.03) 0%, transparent 50%);
  color: #6e757e;
}

.support-dialog-btn--secondary:hover {
  color: #9aa1aa;
  background: rgba(255, 255, 255, 0.04);
}

.support-dialog-btn--secondary:active {
  background: rgba(255, 255, 255, 0.06);
}

.support-dialog-btn--secondary:focus-visible {
  outline: 1px solid var(--theme-focus-outline);
  outline-offset: 1px;
}
</style>
