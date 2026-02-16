import { computed, ref, type Ref } from "vue";
import type { MediaPlayable } from "../nts";

type UserSlot = 3 | 4 | 5 | 6;
type PresetAssignments = Record<UserSlot, string | null>;

type UsePresetControllerOptions = {
  assignments: Ref<PresetAssignments>;
  channelOne: Ref<MediaPlayable | null>;
  channelTwo: Ref<MediaPlayable | null>;
  mixtapeByAlias: Ref<Map<string, MediaPlayable>>;
  activeSlot: Ref<number | null>;
  currentPlayable: Ref<MediaPlayable | null>;
  startPlayback: (playable: MediaPlayable, slot: number) => void | Promise<void>;
  stopPlayback: () => void | Promise<void>;
};

function isUserSlot(slot: number): slot is UserSlot {
  return slot >= 3 && slot <= 6;
}

export function usePresetController(options: UsePresetControllerOptions) {
  const contextMenu = ref<{
    visible: boolean;
    slot: UserSlot | null;
    x: number;
    y: number;
  }>({
    visible: false,
    slot: null,
    x: 0,
    y: 0,
  });
  const presetButtonRefs = ref<Record<number, HTMLButtonElement | null>>({});

  const presetCards = computed(() => {
    return [1, 2, 3, 4, 5, 6].map((slot) => {
      if (slot === 1) {
        return {
          slot,
          locked: true,
          playable: options.channelOne.value,
        };
      }

      if (slot === 2) {
        return {
          slot,
          locked: true,
          playable: options.channelTwo.value,
        };
      }

      const assignedAlias = options.assignments.value[slot as UserSlot];
      const playable = assignedAlias
        ? (options.mixtapeByAlias.value.get(assignedAlias) ?? null)
        : null;

      return {
        slot,
        locked: false,
        playable,
      };
    });
  });

  const contextMenuHasAssignment = computed(() => {
    const slot = contextMenu.value.slot;
    if (!slot) {
      return false;
    }

    return Boolean(options.assignments.value[slot]);
  });

  function closeContextMenu() {
    contextMenu.value.visible = false;
    contextMenu.value.slot = null;
  }

  function setPresetButtonRef(slot: number, element: unknown) {
    presetButtonRefs.value[slot] = element instanceof HTMLButtonElement ? element : null;
  }

  function openContextMenuAt(slot: UserSlot, x: number, y: number) {
    const menuWidth = 210;
    const menuMaxHeight = 220;
    const padding = 6;

    contextMenu.value.slot = slot;
    contextMenu.value.x = Math.max(
      padding,
      Math.min(x, window.innerWidth - menuWidth - padding),
    );
    contextMenu.value.y = Math.max(
      padding,
      Math.min(y, window.innerHeight - menuMaxHeight - padding),
    );
    contextMenu.value.visible = true;
  }

  function openContextMenu(event: MouseEvent, slot: UserSlot) {
    openContextMenuAt(slot, event.clientX, event.clientY);
  }

  function openContextMenuForSlot(slot: UserSlot) {
    const button = presetButtonRefs.value[slot];
    if (button) {
      const rect = button.getBoundingClientRect();
      openContextMenuAt(slot, rect.left + rect.width / 2, rect.bottom + 6);
      return;
    }

    openContextMenuAt(slot, window.innerWidth / 2, window.innerHeight / 2);
  }

  function assignSlotFromMenu(alias: string) {
    const slot = contextMenu.value.slot;
    if (!slot) {
      return;
    }

    options.assignments.value[slot] = alias;
    const playable = options.mixtapeByAlias.value.get(alias) ?? null;

    closeContextMenu();

    if (playable) {
      void options.startPlayback(playable, slot);
    }
  }

  function clearUserSlot(slot: UserSlot) {
    options.assignments.value[slot] = null;
    closeContextMenu();
    if (options.activeSlot.value === slot) {
      void options.stopPlayback();
      options.activeSlot.value = null;
      options.currentPlayable.value = null;
    }
  }

  function clearContextMenuSlot() {
    const slot = contextMenu.value.slot;
    if (!slot) {
      return;
    }

    clearUserSlot(slot);
  }

  function activatePresetSlot(slot: number) {
    const card = presetCards.value.find((item) => item.slot === slot);
    if (!card) {
      return;
    }

    if (card.playable) {
      void options.startPlayback(card.playable, slot);
      return;
    }

    if (!card.locked && isUserSlot(slot)) {
      openContextMenuForSlot(slot);
    }
  }

  function onPresetPress(slot: number) {
    activatePresetSlot(slot);
  }

  function onPresetContextMenu(event: MouseEvent, slot: number, locked: boolean) {
    if (locked || !isUserSlot(slot)) {
      return;
    }

    openContextMenu(event, slot);
  }

  function onPresetContextMenuByKeyboard(slot: number) {
    if (!isUserSlot(slot)) {
      return;
    }

    openContextMenuForSlot(slot);
  }

  function onPresetHotkey(slot: number) {
    activatePresetSlot(slot);
  }

  return {
    contextMenu,
    presetCards,
    contextMenuHasAssignment,
    closeContextMenu,
    setPresetButtonRef,
    assignSlotFromMenu,
    clearContextMenuSlot,
    onPresetPress,
    onPresetContextMenu,
    onPresetContextMenuByKeyboard,
    onPresetHotkey,
  };
}
