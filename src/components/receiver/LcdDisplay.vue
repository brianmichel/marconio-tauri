<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";

const MAIN_FONT_SIZE = 20;
const SUB_FONT_SIZE = 10;
const META_FONT_SIZE = 8;
const SCROLL_GAP = 4;

const props = defineProps<{
  primaryText: string;
  secondaryText: string;
  metaText: string;
  themeAnimating: boolean;
}>();

const emit = defineEmits<{
  cycleTheme: [];
}>();

const lcdRef = ref<HTMLElement | null>(null);
const mainCols = ref(18);
const subCols = ref(36);
const metaCols = ref(45);
const mainScrollPos = ref(0);
const subScrollPos = ref(0);
let mainScrollTimer: ReturnType<typeof setTimeout> | null = null;
let subScrollTimer: ReturnType<typeof setTimeout> | null = null;

function dsegClean(text: string): string {
  return text.replace(/ /g, "!").replace(/:/g, "-");
}

function visibleSlice(text: string, cols: number, offset: number): string {
  const cleaned = dsegClean(text);
  if (!cleaned || cleaned.length <= cols) {
    return (cleaned || "").padEnd(cols, "!");
  }
  const gap = "!".repeat(SCROLL_GAP);
  const looped = cleaned + gap;
  const total = looped.length;
  const pos = offset % total;
  let out = "";
  for (let i = 0; i < cols; i++) {
    out += looped[(pos + i) % total];
  }
  return out;
}

function cleanedLength(text: string): number {
  return dsegClean(text).length;
}

const mainVisible = computed(() =>
  visibleSlice(props.primaryText, mainCols.value, mainScrollPos.value),
);

const subVisible = computed(() =>
  visibleSlice(props.secondaryText, subCols.value, subScrollPos.value),
);

const metaVisible = computed(() => visibleSlice(props.metaText, metaCols.value, 0));

function measureCols() {
  const el = lcdRef.value;
  if (!el) return;

  const style = getComputedStyle(el);
  const width = el.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);

  const canvas = document.createElement("canvas");
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const sample = "~~~~~~~~~~";

  ctx.font = `700 ${MAIN_FONT_SIZE}px DSEG14, monospace`;
  const mainCharWidth = ctx.measureText(sample).width / sample.length;
  if (mainCharWidth > 0) mainCols.value = Math.floor(width / mainCharWidth);

  ctx.font = `700 ${SUB_FONT_SIZE}px DSEG14, monospace`;
  const subCharWidth = ctx.measureText(sample).width / sample.length;
  if (subCharWidth > 0) subCols.value = Math.floor(width / subCharWidth);

  ctx.font = `700 ${META_FONT_SIZE}px DSEG14, monospace`;
  const metaCharWidth = ctx.measureText(sample).width / sample.length;
  if (metaCharWidth > 0) metaCols.value = Math.floor(width / metaCharWidth);
}

function stopMainScroll() {
  if (mainScrollTimer) {
    clearTimeout(mainScrollTimer);
    mainScrollTimer = null;
  }
  mainScrollPos.value = 0;
}

function stopSubScroll() {
  if (subScrollTimer) {
    clearTimeout(subScrollTimer);
    subScrollTimer = null;
  }
  subScrollPos.value = 0;
}

function startMainScroll() {
  stopMainScroll();
  const total = cleanedLength(props.primaryText) + SCROLL_GAP;
  if (total <= mainCols.value + SCROLL_GAP) {
    return;
  }

  mainScrollTimer = setTimeout(function tick() {
    mainScrollPos.value += 1;
    mainScrollTimer = setTimeout(tick, mainScrollPos.value % total === 0 ? 2500 : 250);
  }, 2500);
}

function startSubScroll() {
  stopSubScroll();
  const total = cleanedLength(props.secondaryText) + SCROLL_GAP;
  if (total <= subCols.value + SCROLL_GAP) {
    return;
  }

  subScrollTimer = setTimeout(function tick() {
    subScrollPos.value += 1;
    subScrollTimer = setTimeout(tick, subScrollPos.value % total === 0 ? 2500 : 200);
  }, 2500);
}

function restartScrollAndMeasure() {
  measureCols();
  startMainScroll();
  startSubScroll();
}

function onCycleTheme() {
  emit("cycleTheme");
}

watch(
  () => props.primaryText,
  () => {
    stopMainScroll();
    nextTick(startMainScroll);
  },
);

watch(
  () => props.secondaryText,
  () => {
    stopSubScroll();
    nextTick(startSubScroll);
  },
);

onMounted(async () => {
  window.addEventListener("resize", restartScrollAndMeasure);
  await document.fonts.ready;
  nextTick(restartScrollAndMeasure);
});

onBeforeUnmount(() => {
  window.removeEventListener("resize", restartScrollAndMeasure);
  stopMainScroll();
  stopSubScroll();
});
</script>

<template>
  <section
    ref="lcdRef"
    class="lcd"
    :class="{ 'lcd--theme-animating': props.themeAnimating }"
    aria-live="polite"
    @click="onCycleTheme"
  >
    <div class="lcd-row lcd-row--main" aria-hidden="true">
      <span v-for="i in mainCols" :key="`main-${i}`" class="lcd-cell">
        <span class="lcd-cell-ghost">~</span>
        <span class="lcd-cell-text">{{ mainVisible[i - 1] }}</span>
      </span>
    </div>
    <div class="lcd-row lcd-row--sub" aria-hidden="true">
      <span v-for="i in subCols" :key="`sub-${i}`" class="lcd-cell">
        <span class="lcd-cell-ghost">~</span>
        <span class="lcd-cell-text">{{ subVisible[i - 1] }}</span>
      </span>
    </div>
    <div class="lcd-row lcd-row--meta" aria-hidden="true">
      <span v-for="i in metaCols" :key="`meta-${i}`" class="lcd-cell">
        <span class="lcd-cell-ghost">~</span>
        <span class="lcd-cell-text">{{ metaVisible[i - 1] }}</span>
      </span>
    </div>
  </section>
</template>

<style scoped>
@font-face {
  font-family: "DSEG14";
  src: url("../../assets/DSEG14Classic-Bold.ttf") format("truetype");
  font-weight: 700;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: "DSEG14";
  src: url("../../assets/DSEG14Classic-Regular.ttf") format("truetype");
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: "DSEG14";
  src: url("../../assets/DSEG14Classic-Light.ttf") format("truetype");
  font-weight: 300;
  font-style: normal;
  font-display: swap;
}

.lcd {
  border-radius: 4px;
  border: 1px solid var(--lcd-border);
  background:
    radial-gradient(120% 100% at 8% -10%, var(--lcd-glow), transparent 50%),
    linear-gradient(180deg, var(--lcd-top) 0%, var(--lcd-mid) 50%, var(--lcd-bottom) 100%);
  padding: 8px 10px 6px;
  box-shadow:
    inset 0 2px 4px rgba(0, 0, 0, 0.15),
    inset 0 -3px 8px var(--lcd-shadow-strong),
    inset 2px 0 6px var(--lcd-shadow-soft),
    inset -2px 0 6px var(--lcd-shadow-soft),
    0 1px 0 rgba(255, 255, 255, 0.06),
    0 3px 12px rgba(0, 0, 0, 0.35),
    0 0 18px var(--lcd-outer-glow),
    0 0 48px var(--lcd-outer-glow);
  position: relative;
  z-index: 2;
  overflow: hidden;
  cursor: pointer;
  transition:
    border-color 280ms ease,
    box-shadow 280ms ease,
    filter 140ms ease;
}

.lcd:hover {
  filter: brightness(1.03);
}

.lcd--theme-animating {
  animation: lcd-flicker 450ms linear;
}

@keyframes lcd-flicker {
  0% {
    filter: brightness(1) saturate(1);
  }
  12% {
    filter: brightness(0.82) saturate(0.7);
  }
  22% {
    filter: brightness(1.08) saturate(0.9);
  }
  35% {
    filter: brightness(0.75) saturate(0.6);
  }
  50% {
    filter: brightness(1.18) saturate(1.15);
  }
  65% {
    filter: brightness(1.1) saturate(1.06);
  }
  82% {
    filter: brightness(1.03) saturate(1.01);
  }
  100% {
    filter: brightness(1) saturate(1);
  }
}

.lcd::before {
  content: "";
  position: absolute;
  inset: 0;
  border-radius: inherit;
  pointer-events: none;
  opacity: 0.06;
  background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='1.2' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
  background-size: 128px 128px;
  z-index: 0;
}

.lcd::after {
  content: "";
  position: absolute;
  inset: -30%;
  border-radius: 50%;
  background: radial-gradient(ellipse at center, var(--lcd-glow), transparent 55%);
  opacity: 0.8;
  animation: lcd-glow-wander 11s ease-in-out infinite;
  pointer-events: none;
  mix-blend-mode: soft-light;
  will-change: transform;
  z-index: 1;
}

@keyframes lcd-glow-wander {
  0%,
  100% {
    transform: translate(-10%, -6%);
  }
  33% {
    transform: translate(14%, 10%);
  }
  66% {
    transform: translate(-5%, -12%);
  }
}

.lcd-row {
  overflow: hidden;
  white-space: nowrap;
  font-family: var(--lcd-font);
  text-transform: uppercase;
  letter-spacing: 0;
  line-height: 1;
}

.lcd-cell {
  display: inline-block;
  position: relative;
  text-align: center;
  vertical-align: top;
}

.lcd-cell-ghost {
  visibility: visible;
  color: var(--lcd-ghost);
  transition: color 280ms ease;
}

.lcd-cell-text {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
}

.lcd-row--main {
  font-size: 20px;
  font-weight: 700;
}

.lcd-row--main .lcd-cell {
  letter-spacing: 0.04em;
}

.lcd-row--main .lcd-cell-text {
  color: var(--lcd-main-text);
  text-shadow: 0 0 3px var(--lcd-main-glow);
  transition:
    color 280ms ease,
    text-shadow 280ms ease;
}

.lcd-row--sub {
  font-size: 10px;
  font-weight: 700;
  margin-top: 4px;
}

.lcd-row--sub .lcd-cell-text {
  color: var(--lcd-main-text);
  text-shadow: 0 0 2px var(--lcd-sub-glow);
  transition:
    color 280ms ease,
    text-shadow 280ms ease;
}

.lcd-row--meta {
  font-size: 8px;
  font-weight: 700;
  margin-top: 4px;
}

.lcd-row--meta .lcd-cell-ghost {
  opacity: 0.6;
}

.lcd-row--meta .lcd-cell-text {
  color: var(--lcd-meta-text);
  opacity: 0.7;
  transition: color 280ms ease;
}
</style>
