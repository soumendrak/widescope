/* tslint:disable */
/* eslint-disable */

export function compute_flamegraph(): string;

export function compute_timeline(): string;

export function compute_waterfall(): string;

export function get_span_detail(span_id: string): string;

export function init(conventions_json: string): string;

export function parse_trace(raw_input: string): string;

export function search_spans(query: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly compute_flamegraph: (a: number) => void;
    readonly compute_timeline: (a: number) => void;
    readonly compute_waterfall: (a: number) => void;
    readonly get_span_detail: (a: number, b: number, c: number) => void;
    readonly init: (a: number, b: number, c: number) => void;
    readonly parse_trace: (a: number, b: number, c: number) => void;
    readonly search_spans: (a: number, b: number, c: number) => void;
    readonly __wbindgen_export: (a: number, b: number, c: number) => void;
    readonly __wbindgen_export2: (a: number, b: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
