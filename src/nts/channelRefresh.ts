import type { Channel, MediaPlayable } from "./types";

const DEFAULT_REFRESH_BUFFER_MS = 5_000;
const DEFAULT_REFRESH_FALLBACK_MS = 60_000;

function toTimestampMs(value: Date | null | undefined): number | null {
  if (value instanceof Date) {
    return value.getTime();
  }
  return null;
}

export function calculateChannelRefreshDelay(
  channel: Channel,
  nowMs: number = Date.now(),
  bufferMs: number = DEFAULT_REFRESH_BUFFER_MS,
  fallbackMs: number = DEFAULT_REFRESH_FALLBACK_MS,
): number {
  const candidateTimes = [channel.now.endTimestamp, channel.next.startTimestamp]
    .map(toTimestampMs)
    .filter((timestamp): timestamp is number => typeof timestamp === "number");

  if (candidateTimes.length === 0) {
    return fallbackMs;
  }

  const nextTransition = Math.min(...candidateTimes);
  return Math.max(nextTransition - nowMs + bufferMs, bufferMs);
}

export function syncChannelPlayableFromLive(
  currentPlayable: MediaPlayable | null,
  channels: MediaPlayable[],
): MediaPlayable | null {
  if (!currentPlayable || currentPlayable.source.kind !== "channel") {
    return currentPlayable;
  }

  const channelName = currentPlayable.source.value.channelName;
  const refreshed = channels.find(
    (item) => item.source.kind === "channel" && item.source.value.channelName === channelName,
  );

  return refreshed ?? currentPlayable;
}
