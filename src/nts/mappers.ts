import type {
  Broadcast,
  BroadcastEmbed,
  Channel,
  Link,
  LiveBroadcastsResponse,
  Media,
  Mixtape,
  MixtapesResponse,
} from "./types";

interface RawBroadcastEmbed {
  status: string;
  updated: string;
  name: string;
  description: string;
  description_html: string;
  external_links: string[];
  location_short: string | null;
  location_long: string | null;
  media: RawMedia;
  episode_alias: string | null;
  show_alias: string;
  broadcast: string | null;
  mixcloud: string | null;
  links: RawLink[];
}

interface RawBroadcast {
  broadcast_title: string;
  start_timestamp: string;
  end_timestamp: string;
  links: RawLink[];
  embeds: Record<string, RawBroadcastEmbed>;
}

interface RawChannel {
  channel_name: string;
  now: RawBroadcast;
  next: RawBroadcast;
}

interface RawLiveBroadcastsResponse {
  results: RawChannel[];
  links: RawLink[];
}

interface RawMixtape {
  mixtape_alias: string;
  title: string;
  subtitle: string;
  description: string;
  description_html: string;
  audio_stream_endpoint: string;
  media: RawMedia;
  now_playing_topic: string;
  links: RawLink[];
}

interface RawMixtapesResponse {
  results: RawMixtape[];
  links: RawLink[];
}

interface RawMedia {
  background_large: string | null;
  background_medium_large: string | null;
  background_medium: string | null;
  background_small: string | null;
  background_thumb: string | null;
  picture_large: string;
  picture_medium_large: string;
  picture_medium: string;
  picture_small: string;
  picture_thumb: string;
}

interface RawLink {
  href: string;
  rel: string;
  type: string;
}

function toDate(value: string): Date {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    throw new Error(`Invalid date: ${value}`);
  }
  return date;
}

function mapLink(raw: RawLink): Link {
  return {
    href: raw.href,
    rel: raw.rel,
    type: raw.type,
  };
}

function mapMedia(raw: RawMedia): Media {
  return {
    backgroundLarge: raw.background_large,
    backgroundMediumLarge: raw.background_medium_large,
    backgroundMedium: raw.background_medium,
    backgroundSmall: raw.background_small,
    backgroundThumb: raw.background_thumb,
    pictureLarge: raw.picture_large,
    pictureMediumLarge: raw.picture_medium_large,
    pictureMedium: raw.picture_medium,
    pictureSmall: raw.picture_small,
    pictureThumb: raw.picture_thumb,
  };
}

function mapEmbed(raw: RawBroadcastEmbed): BroadcastEmbed {
  return {
    status: raw.status,
    updated: toDate(raw.updated),
    name: raw.name,
    description: raw.description,
    descriptionHtml: raw.description_html,
    externalLinks: raw.external_links,
    locationShort: raw.location_short,
    locationLong: raw.location_long,
    media: mapMedia(raw.media),
    episodeAlias: raw.episode_alias,
    showAlias: raw.show_alias,
    broadcast: raw.broadcast ? toDate(raw.broadcast) : null,
    mixcloud: raw.mixcloud,
    links: raw.links.map(mapLink),
  };
}

function mapBroadcast(raw: RawBroadcast): Broadcast {
  return {
    broadcastTitle: raw.broadcast_title,
    startTimestamp: toDate(raw.start_timestamp),
    endTimestamp: toDate(raw.end_timestamp),
    links: raw.links.map(mapLink),
    embeds: Object.fromEntries(
      Object.entries(raw.embeds).map(([key, value]) => [key, mapEmbed(value)]),
    ),
  };
}

function mapChannel(raw: RawChannel): Channel {
  return {
    channelName: raw.channel_name,
    now: mapBroadcast(raw.now),
    next: mapBroadcast(raw.next),
  };
}

function mapMixtape(raw: RawMixtape): Mixtape {
  return {
    mixtapeAlias: raw.mixtape_alias,
    title: raw.title,
    subtitle: raw.subtitle,
    description: raw.description,
    descriptionHtml: raw.description_html,
    audioStreamEndpoint: raw.audio_stream_endpoint,
    media: mapMedia(raw.media),
    nowPlayingTopic: raw.now_playing_topic,
    links: raw.links.map(mapLink),
  };
}

export function mapLiveBroadcastsResponse(data: unknown): LiveBroadcastsResponse {
  const raw = data as RawLiveBroadcastsResponse;
  return {
    results: raw.results.map(mapChannel),
    links: raw.links.map(mapLink),
  };
}

export function mapMixtapesResponse(data: unknown): MixtapesResponse {
  const raw = data as RawMixtapesResponse;
  return {
    results: raw.results.map(mapMixtape),
    links: raw.links.map(mapLink),
  };
}
