/**
 * Self-contained share links.
 *
 * A trace is compressed and base64url-encoded into the URL hash fragment
 * (`#trace=...`). Fragments are never sent to a server, so a shared link keeps
 * WideScope's local-first, zero-upload promise. View mode and the selected span
 * travel alongside the data so a link restores the exact thing the sender saw.
 *
 * Compression runs in WASM: the trace JSON is DEFLATE-compressed, seeded with
 * a dictionary embedded in the WASM binary (see
 * `crates/widescope-core/src/share.rs`), which gives small traces far shorter
 * links than a standalone gzip pass could. Legacy links created with the
 * earlier gzip scheme are still decoded — they are recognised by the gzip
 * magic byte, which the tagged format never starts with.
 *
 * `?trace=<url>` (a remote URL in the query string) is handled separately by the
 * caller; this module only parses it out.
 */
import type { ViewName } from './types';
import { compressShare, decompressShare, isShareCompressionReady } from './wasm';

const VIEW_NAMES: ReadonlyArray<ViewName> = [
  'flame',
  'timeline',
  'waterfall',
  'graph',
  'agent',
  'diff',
];

/**
 * Largest encoded-blob length (characters) we will put in a share link.
 * Comfortably under every browser's URL limit and still pasteable into chat.
 */
export const MAX_SHARE_DATA_CHARS = 32_000;

export interface PermalinkState {
  /** base64url gzip blob from `#trace=` — still encoded; decode with decodeTrace. */
  traceData: string | null;
  /** Remote trace URL from `?trace=` in the query string. */
  traceUrl: string | null;
  /** Initial view, if a valid one was supplied. */
  view: ViewName | null;
  /** Span id to pre-select. */
  spanId: string | null;
}

export interface ShareUrlResult {
  /** The shareable URL. Still returned when `tooLarge` so the caller can decide. */
  url: string;
  /** Length of the encoded trace blob, in characters. */
  dataChars: number;
  /** True when the blob exceeds {@link MAX_SHARE_DATA_CHARS}. */
  tooLarge: boolean;
}

/** Gzip magic byte — identifies a legacy (pre-dictionary) share blob. */
const GZIP_MAGIC = 0x1f;

/** Whether share links can be built (i.e. the WASM compressor is loaded). */
export function isShareSupported(): boolean {
  return isShareCompressionReady();
}

async function pumpThrough(
  data: Uint8Array,
  transform: CompressionStream | DecompressionStream,
): Promise<Uint8Array> {
  const writer = transform.writable.getWriter();
  // The writable side is typed against ArrayBuffer-backed views; a plain
  // Uint8Array (ArrayBufferLike) is safe to pass at runtime.
  //
  // Feed the input without awaiting here so the readable side can drain
  // concurrently. The `.catch()` is required: when the transform errors
  // (e.g. corrupt gzip), the writer's write/close promises reject too, and
  // without a handler that surfaces as an unhandled promise rejection. The
  // real error is still reported below, where reading `transform.readable`
  // rejects with it.
  void writer
    .write(data as Uint8Array<ArrayBuffer>)
    .then(() => writer.close())
    .catch(() => {
      /* surfaced via transform.readable below */
    });
  return new Uint8Array(await new Response(transform.readable).arrayBuffer());
}

function gunzip(data: Uint8Array): Promise<Uint8Array> {
  return pumpThrough(data, new DecompressionStream('gzip'));
}

function bytesToBase64Url(bytes: Uint8Array): string {
  let binary = '';
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary)
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');
}

function base64UrlToBytes(value: string): Uint8Array {
  const base64 = value.replace(/-/g, '+').replace(/_/g, '/');
  const padding = base64.length % 4 === 0 ? '' : '='.repeat(4 - (base64.length % 4));
  const binary = atob(base64 + padding);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

/** Compress a trace JSON string into a URL-safe blob for `#trace=`. */
export async function encodeTrace(json: string): Promise<string> {
  return bytesToBase64Url(compressShare(json));
}

/**
 * Reverse {@link encodeTrace}.
 *
 * Current blobs carry a leading format tag and are decoded in WASM; legacy
 * gzip blobs are recognised by the gzip magic byte and decoded with the
 * browser's `DecompressionStream`.
 *
 * @throws {Error} If the blob is not valid base64url or cannot be decompressed.
 */
export async function decodeTrace(encoded: string): Promise<string> {
  const bytes = base64UrlToBytes(encoded);
  if (bytes[0] === GZIP_MAGIC) {
    return new TextDecoder().decode(await gunzip(bytes));
  }
  return decompressShare(bytes);
}

function coerceView(value: string | null): ViewName | null {
  return value !== null && VIEW_NAMES.includes(value as ViewName)
    ? (value as ViewName)
    : null;
}

/**
 * Read share state from a URL.
 *
 * Self-contained data is read only from the hash (`#trace=`); a remote URL is
 * read only from the query string (`?trace=`). `view`/`span` are accepted from
 * either, with the hash taking precedence.
 *
 * @param href Defaults to the current document location.
 */
export function parsePermalink(href: string = window.location.href): PermalinkState {
  const url = new URL(href);
  const hash = new URLSearchParams(url.hash.replace(/^#/, ''));
  const query = url.searchParams;
  const pick = (key: string): string | null => hash.get(key) ?? query.get(key);

  return {
    traceData: hash.get('trace'),
    traceUrl: query.get('trace'),
    view: coerceView(pick('view')),
    spanId: pick('span'),
  };
}

function defaultBaseUrl(): string {
  return window.location.origin + window.location.pathname;
}

/**
 * Build a self-contained share URL for the given trace and view state.
 *
 * @param opts.json    Raw trace JSON to embed.
 * @param opts.view    View mode to restore.
 * @param opts.spanId  Span to pre-select, or null.
 * @param opts.baseUrl Base URL to build on; defaults to the current location.
 * @returns The URL plus a `tooLarge` flag the caller should check before sharing.
 */
export async function buildShareUrl(opts: {
  json: string;
  view: ViewName;
  spanId: string | null;
  baseUrl?: string;
}): Promise<ShareUrlResult> {
  const data = await encodeTrace(opts.json);
  const params = new URLSearchParams();
  params.set('trace', data);
  params.set('view', opts.view);
  if (opts.spanId) {
    params.set('span', opts.spanId);
  }
  const base = opts.baseUrl ?? defaultBaseUrl();
  return {
    url: `${base}#${params.toString()}`,
    dataChars: data.length,
    tooLarge: data.length > MAX_SHARE_DATA_CHARS,
  };
}
