export interface Link {
  href: string;
  rel: string;
  type: string;
}

export interface Media {
  backgroundLarge: string | null;
  backgroundMediumLarge: string | null;
  backgroundMedium: string | null;
  backgroundSmall: string | null;
  backgroundThumb: string | null;
  pictureLarge: string;
  pictureMediumLarge: string;
  pictureMedium: string;
  pictureSmall: string;
  pictureThumb: string;
}

export interface BroadcastEmbed {
  status: string;
  updated: Date;
  name: string;
  description: string;
  descriptionHtml: string;
  externalLinks: string[];
  locationShort: string | null;
  locationLong: string | null;
  media: Media;
  episodeAlias: string | null;
  showAlias: string;
  broadcast: Date | null;
  mixcloud: string | null;
  links: Link[];
}

export interface Broadcast {
  broadcastTitle: string;
  startTimestamp: Date;
  endTimestamp: Date;
  links: Link[];
  embeds: Record<string, BroadcastEmbed>;
}

export interface Channel {
  channelName: string;
  now: Broadcast;
  next: Broadcast;
}

export interface LiveBroadcastsResponse {
  results: Channel[];
  links: Link[];
}

export interface Mixtape {
  mixtapeAlias: string;
  title: string;
  subtitle: string;
  description: string;
  descriptionHtml: string;
  audioStreamEndpoint: string;
  media: Media;
  nowPlayingTopic: string;
  links: Link[];
}

export interface MixtapesResponse {
  results: Mixtape[];
  links: Link[];
}

export interface MediaPlayable {
  id: string;
  title: string;
  subtitle: string | null;
  description: string;
  artworkUrl: string;
  pageUrl: string;
  streamUrl: string;
  source:
    | { kind: "channel"; value: Channel }
    | { kind: "mixtape"; value: Mixtape };
}
