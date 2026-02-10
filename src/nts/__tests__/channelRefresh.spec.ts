import { describe, expect, it } from "vitest";
import type { Broadcast, BroadcastEmbed, Channel, MediaPlayable, Mixtape } from "../types";
import { calculateChannelRefreshDelay, syncChannelPlayableFromLive } from "../channelRefresh";

function createBroadcast(
  startTimestamp: Date,
  endTimestamp: Date,
): Broadcast {
  const embed: BroadcastEmbed = {
    status: "live",
    updated: new Date(),
    name: "Test Show",
    description: "Test description",
    descriptionHtml: "<p>Test</p>",
    externalLinks: [],
    locationShort: null,
    locationLong: null,
    media: {
      backgroundLarge: null,
      backgroundMediumLarge: null,
      backgroundMedium: null,
      backgroundSmall: null,
      backgroundThumb: null,
      pictureLarge: "large.jpg",
      pictureMediumLarge: "medium-large.jpg",
      pictureMedium: "medium.jpg",
      pictureSmall: "small.jpg",
      pictureThumb: "thumb.jpg",
    },
    episodeAlias: null,
    showAlias: "test-show",
    broadcast: startTimestamp,
    mixcloud: null,
    links: [],
  };

  return {
    broadcastTitle: "Test Broadcast",
    startTimestamp,
    endTimestamp,
    links: [],
    embeds: { details: embed },
  };
}

function createChannel(channelName: string, now: Broadcast, next: Broadcast): Channel {
  return {
    channelName,
    now,
    next,
  };
}

function createChannelPlayable(channel: Channel, id: string): MediaPlayable {
  return {
    id,
    title: `Channel ${channel.channelName}`,
    subtitle: channel.now.broadcastTitle,
    description: "Channel description",
    artworkUrl: "artwork.jpg",
    pageUrl: "https://example.com",
    streamUrl: "https://stream.example.com",
    source: { kind: "channel", value: channel },
  };
}

function createMixtapePlayable(id: string): MediaPlayable {
  const mixtape: Mixtape = {
    mixtapeAlias: "mixtape",
    title: "Mixtape",
    subtitle: "Mixtape subtitle",
    description: "Mixtape description",
    descriptionHtml: "<p>Mixtape</p>",
    audioStreamEndpoint: "https://stream.example.com/mixtape",
    media: {
      backgroundLarge: null,
      backgroundMediumLarge: null,
      backgroundMedium: null,
      backgroundSmall: null,
      backgroundThumb: null,
      pictureLarge: "large.jpg",
      pictureMediumLarge: "medium-large.jpg",
      pictureMedium: "medium.jpg",
      pictureSmall: "small.jpg",
      pictureThumb: "thumb.jpg",
    },
    nowPlayingTopic: "topic",
    links: [],
  };

  return {
    id,
    title: mixtape.title,
    subtitle: mixtape.subtitle,
    description: mixtape.description,
    artworkUrl: mixtape.media.pictureLarge,
    pageUrl: "https://example.com/mixtape",
    streamUrl: mixtape.audioStreamEndpoint,
    source: { kind: "mixtape", value: mixtape },
  };
}

describe("calculateChannelRefreshDelay", () => {
  it("schedules using the earliest transition", () => {
    const now = new Date("2024-06-01T10:00:00Z");
    const current = createBroadcast(
      new Date("2024-06-01T09:00:00Z"),
      new Date("2024-06-01T10:05:00Z"),
    );
    const next = createBroadcast(
      new Date("2024-06-01T10:06:00Z"),
      new Date("2024-06-01T11:00:00Z"),
    );
    const channel = createChannel("1", current, next);

    const delay = calculateChannelRefreshDelay(channel, now.getTime());

    expect(delay).toBe(5 * 60 * 1000 + 5_000);
  });

  it("uses the buffer when transitions are in the past", () => {
    const now = new Date("2024-06-01T10:10:00Z");
    const current = createBroadcast(
      new Date("2024-06-01T09:00:00Z"),
      new Date("2024-06-01T10:00:00Z"),
    );
    const next = createBroadcast(
      new Date("2024-06-01T10:05:00Z"),
      new Date("2024-06-01T11:00:00Z"),
    );
    const channel = createChannel("1", current, next);

    const delay = calculateChannelRefreshDelay(channel, now.getTime());

    expect(delay).toBe(5_000);
  });

  it("falls back when no timestamps are available", () => {
    const now = new Date("2024-06-01T10:00:00Z");
    const current = createBroadcast(
      new Date("2024-06-01T09:00:00Z"),
      new Date("2024-06-01T10:05:00Z"),
    );
    const next = createBroadcast(
      new Date("2024-06-01T10:06:00Z"),
      new Date("2024-06-01T11:00:00Z"),
    );
    const channel = createChannel("1", current, next);
    channel.now.endTimestamp = null as unknown as Date;
    channel.next.startTimestamp = null as unknown as Date;

    const delay = calculateChannelRefreshDelay(channel, now.getTime(), 5_000, 60_000);

    expect(delay).toBe(60_000);
  });
});

describe("syncChannelPlayableFromLive", () => {
  it("replaces the current channel playable when refreshed data exists", () => {
    const now = createBroadcast(new Date(), new Date());
    const next = createBroadcast(new Date(), new Date());
    const channel = createChannel("1", now, next);
    const currentPlayable = createChannelPlayable(channel, "current");
    const refreshedPlayable = createChannelPlayable(channel, "refreshed");

    const result = syncChannelPlayableFromLive(currentPlayable, [refreshedPlayable]);

    expect(result).toBe(refreshedPlayable);
  });

  it("keeps mixtape playables unchanged", () => {
    const mixtapePlayable = createMixtapePlayable("mix-1");

    const result = syncChannelPlayableFromLive(mixtapePlayable, []);

    expect(result).toBe(mixtapePlayable);
  });
});
