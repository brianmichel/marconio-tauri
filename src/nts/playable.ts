import type { Channel, MediaPlayable, Mixtape } from "./types";

const NTS_WEB_BASE = "https://nts.live";
const NTS_STREAM_BASE = "https://stream-relay-geo.ntslive.net";

function streamPathForChannel(channelName: string): string | null {
  switch (channelName) {
    case "1":
      return "/stream";
    case "2":
      return "/stream2";
    default:
      return null;
  }
}

function showUrl(showAlias: string, episodeAlias: string | null): string {
  if (episodeAlias) {
    return `${NTS_WEB_BASE}/shows/${showAlias}/episodes/${episodeAlias}/`;
  }
  return `${NTS_WEB_BASE}/shows/${showAlias}/`;
}

export function playableFromChannel(channel: Channel): MediaPlayable | null {
  const streamPath = streamPathForChannel(channel.channelName);
  if (!streamPath) {
    return null;
  }

  const details = channel.now.embeds.details;
  const description = details?.description?.trim()
    ? details.description
    : "Description not provided by NTS or broadcaster.";

  return {
    id: `channel-${channel.channelName}`,
    title: `Channel ${channel.channelName}`,
    subtitle: channel.now.broadcastTitle,
    description,
    artworkUrl:
      details?.media.backgroundLarge ??
      details?.media.pictureLarge ??
      "https://upload.wikimedia.org/wikipedia/commons/9/99/Sample_User_Icon.png",
    pageUrl: details
      ? showUrl(details.showAlias, details.episodeAlias)
      : NTS_WEB_BASE,
    streamUrl: `${NTS_STREAM_BASE}${streamPath}`,
    source: { kind: "channel", value: channel },
  };
}

export function playableFromMixtape(mixtape: Mixtape): MediaPlayable {
  return {
    id: `mixtape-${mixtape.mixtapeAlias}`,
    title: mixtape.title,
    subtitle: mixtape.subtitle,
    description: mixtape.description,
    artworkUrl: mixtape.media.pictureLarge,
    pageUrl: `${NTS_WEB_BASE}/infinite-mixtapes/${mixtape.mixtapeAlias}`,
    streamUrl: mixtape.audioStreamEndpoint,
    source: { kind: "mixtape", value: mixtape },
  };
}
