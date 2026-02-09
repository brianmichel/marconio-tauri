import { mapLiveBroadcastsResponse, mapMixtapesResponse } from "./mappers";
import type { LiveBroadcastsResponse, MixtapesResponse } from "./types";
import { invoke } from "@tauri-apps/api/core";

const DEFAULT_BASE_URL = "https://www.nts.live/api/v2";

export class NTSRequestError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = "NTSRequestError";
    this.status = status;
  }
}

export interface NTSClientOptions {
  baseUrl?: string;
  fetchImpl?: typeof fetch;
  retries?: number;
  timeoutMs?: number;
}

export interface RequestOptions {
  signal?: AbortSignal;
}

export interface NTSClient {
  live(options?: RequestOptions): Promise<LiveBroadcastsResponse>;
  mixtapes(options?: RequestOptions): Promise<MixtapesResponse>;
}

export function createNTSClient(options: NTSClientOptions = {}): NTSClient {
  const baseUrl = (options.baseUrl ?? DEFAULT_BASE_URL).replace(/\/$/, "");
  const fetchImpl = options.fetchImpl ?? fetch;
  const retries = Math.max(0, options.retries ?? 1);
  const timeoutMs = Math.max(1000, options.timeoutMs ?? 12000);
  const inflight = new Map<string, Promise<unknown>>();

  function log(message: string, extra?: unknown): void {
    if (typeof extra === "undefined") {
      console.info(`[nts-client] ${message}`);
      return;
    }
    console.info(`[nts-client] ${message}`, extra);
  }

  async function withTimeout<T>(task: Promise<T>, ms: number, label: string): Promise<T> {
    let timeoutId: ReturnType<typeof setTimeout> | undefined;
    const timeoutPromise = new Promise<T>((_, reject) => {
      timeoutId = setTimeout(() => {
        reject(new Error(`${label} timed out after ${ms}ms`));
      }, ms);
    });

    try {
      return await Promise.race([task, timeoutPromise]);
    } finally {
      if (timeoutId) {
        clearTimeout(timeoutId);
      }
    }
  }

  async function tauriNtsGet(path: string): Promise<unknown> {
    log(`invoke fallback start (${path})`);
    const data = await withTimeout(
      invoke("nts_get", { path }) as Promise<unknown>,
      timeoutMs,
      `invoke nts_get(${path})`,
    );
    log(`invoke fallback success (${path})`);
    return data;
  }

  function canUseTauriInvoke(): boolean {
    return (
      typeof window !== "undefined" &&
      "__TAURI_INTERNALS__" in window &&
      typeof (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ !==
        "undefined"
    );
  }

  async function request<T>(
    path: string,
    mapResponse: (data: unknown) => T,
    requestOptions?: RequestOptions,
  ): Promise<T> {
    const key = path;
    const hasSignal = Boolean(requestOptions?.signal);

    if (!hasSignal && inflight.has(key)) {
      return inflight.get(key) as Promise<T>;
    }

    const promise = (async () => {
      let lastError: unknown;

      for (let attempt = 0; attempt <= retries; attempt += 1) {
        try {
          log(`request start (${path}) attempt ${attempt + 1}/${retries + 1}`);
          let json: unknown;

          try {
            const response = await withTimeout(
              fetchImpl(`${baseUrl}/${path}`, {
                method: "GET",
                cache: "no-store",
                signal: requestOptions?.signal,
                headers: {
                  Accept: "application/json",
                },
              }),
              timeoutMs,
              `fetch ${path}`,
            );

            if (!response.ok) {
              throw new NTSRequestError(
                response.status,
                `NTS request failed (${response.status}) for ${path}`,
              );
            }

            json = (await response.json()) as unknown;
            log(`fetch success (${path})`);
          } catch (error) {
            const shouldTryTauriFallback =
              canUseTauriInvoke() &&
              !(error instanceof NTSRequestError) &&
              !requestOptions?.signal?.aborted;

            log(`fetch failed (${path}), fallback=${shouldTryTauriFallback}`, error);

            if (!shouldTryTauriFallback) {
              throw error;
            }

            json = await tauriNtsGet(path);
          }

          return mapResponse(json);
        } catch (error) {
          log(`request failed (${path})`, error);
          if (requestOptions?.signal?.aborted) {
            throw error;
          }

          lastError = error;

          if (error instanceof NTSRequestError) {
            throw error;
          }

          if (attempt < retries) {
            await new Promise((resolve) => setTimeout(resolve, 250 * (attempt + 1)));
            continue;
          }

          throw error;
        }
      }

      throw lastError;
    })();

    if (!hasSignal) {
      inflight.set(key, promise);
      promise.finally(() => {
        inflight.delete(key);
      });
    }

    return promise;
  }

  return {
    live(requestOptions) {
      return request("live", mapLiveBroadcastsResponse, requestOptions);
    },
    mixtapes(requestOptions) {
      return request("mixtapes", mapMixtapesResponse, requestOptions);
    },
  };
}
