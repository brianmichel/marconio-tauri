export type RecognizedTrack = {
  shazamId?: string | null;
  title: string;
  artist?: string | null;
  artworkUrl?: string | null;
  appleMusicUrl?: string | null;
  webUrl?: string | null;
  recognizedAt: number;
  sourceTitle?: string | null;
  sourceArtist?: string | null;
};

export type ShazamStatusPayload = {
  status: "idle" | "listening" | string;
};

export type ShazamResultPayload = {
  kind: "match" | "noMatch" | "error" | string;
  message: string;
  track?: RecognizedTrack | null;
};

export type ShazamHistoryPayload = {
  history: RecognizedTrack[];
};

export type ToastItem = {
  id: number;
  kind: "success" | "info" | "error";
  message: string;
};
