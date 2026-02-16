import { onBeforeUnmount, onMounted, type Ref } from "vue";
import type { MediaPlayable } from "../nts";

const BLOCKED_BROWSER_SHORTCUTS = new Set(["a", "r", "+", "=", "-", "0"]);
const EDITABLE_TARGET_SELECTOR = [
  "input:not([readonly]):not([disabled])",
  "textarea:not([readonly]):not([disabled])",
  '[contenteditable=""]',
  '[contenteditable="true"]',
  '[role="textbox"]',
].join(", ");

type UseGlobalReceiverHotkeysOptions = {
  isDev: boolean;
  supportDialogVisible: Ref<boolean>;
  contextMenuVisible: Ref<boolean>;
  modelMenuVisible: Ref<boolean>;
  settingsPanelVisible: Ref<boolean>;
  isPlaying: Ref<boolean>;
  activeSlot: Ref<number | null>;
  currentPlayable: Ref<MediaPlayable | null>;
  closeSettingsPanel: () => void;
  closeContextMenu: () => void;
  closeModelMenu: () => void;
  onPresetHotkey: (slot: number) => void;
  onPresetPress: (slot: number) => void;
  startPlayback: (playable: MediaPlayable, slot: number) => void | Promise<void>;
  stopPlayback: () => void | Promise<void>;
};

function isEditableTarget(target: EventTarget | null): boolean {
  const element = target instanceof Element ? target : null;
  if (!element) {
    return false;
  }

  return element.closest(EDITABLE_TARGET_SELECTOR) !== null;
}

export function useGlobalReceiverHotkeys(options: UseGlobalReceiverHotkeysOptions) {
  function onGlobalKeyDown(event: KeyboardEvent) {
    if (options.supportDialogVisible.value) {
      return;
    }

    const editable = isEditableTarget(event.target);
    const key = event.key.toLowerCase();
    const hasPrimaryModifier = event.metaKey || event.ctrlKey;
    const shortcutSlot = Number(event.key);

    if (
      !editable &&
      hasPrimaryModifier &&
      Number.isInteger(shortcutSlot) &&
      shortcutSlot >= 1 &&
      shortcutSlot <= 6
    ) {
      event.preventDefault();
      options.onPresetHotkey(shortcutSlot);
      return;
    }

    if (!editable && hasPrimaryModifier && BLOCKED_BROWSER_SHORTCUTS.has(key)) {
      event.preventDefault();
      return;
    }

    if (!editable && (event.key === "F5" || event.key === "F12")) {
      event.preventDefault();
      return;
    }

    if (event.key === "Escape") {
      options.closeSettingsPanel();
      options.closeContextMenu();
      options.closeModelMenu();
      return;
    }

    if (
      editable ||
      event.repeat ||
      options.contextMenuVisible.value ||
      options.modelMenuVisible.value ||
      options.settingsPanelVisible.value
    ) {
      return;
    }

    if (event.code === "Space") {
      event.preventDefault();

      if (options.isPlaying.value) {
        void options.stopPlayback();
        return;
      }

      const playable = options.currentPlayable.value;
      const slot = options.activeSlot.value;
      if (playable && slot !== null) {
        void options.startPlayback(playable, slot);
      }

      return;
    }

    if (event.key >= "1" && event.key <= "6") {
      event.preventDefault();
      options.onPresetPress(Number(event.key));
    }
  }

  function preventBrowserContextMenu(event: MouseEvent) {
    if (options.isDev) {
      return;
    }

    if (isEditableTarget(event.target)) {
      return;
    }

    event.preventDefault();
  }

  function preventDocumentDrop(event: DragEvent) {
    event.preventDefault();
  }

  onMounted(() => {
    window.addEventListener("keydown", onGlobalKeyDown);
    window.addEventListener("contextmenu", preventBrowserContextMenu);
    window.addEventListener("dragover", preventDocumentDrop);
    window.addEventListener("drop", preventDocumentDrop);
  });

  onBeforeUnmount(() => {
    window.removeEventListener("keydown", onGlobalKeyDown);
    window.removeEventListener("contextmenu", preventBrowserContextMenu);
    window.removeEventListener("dragover", preventDocumentDrop);
    window.removeEventListener("drop", preventDocumentDrop);
  });
}
