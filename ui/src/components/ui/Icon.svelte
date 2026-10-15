<script lang="ts" context="module">
  /**
   * The whole icon vocabulary, replacing the emoji set (🎯 🌙 ☀ ⏳ ⚠️ 🔗 ⬢ ⊠ ✕).
   * Emoji render differently per OS, read verbosely to screen readers, and made
   * the UI look unfinished. These are 24x24 stroke paths on `currentColor`, so
   * they inherit text colour and theme for free.
   */
  export const ICON_PATHS = {
    close: 'M18 6 6 18M6 6l12 12',
    warning: 'M12 9v4M12 17h.01M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0Z',
    error: 'M12 8v4M12 16h.01M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Z',
    clock: 'M12 6v6l4 2M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Z',
    link: 'M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71',
    sun: 'M12 17a5 5 0 1 0 0-10 5 5 0 0 0 0 10ZM12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42',
    moon: 'M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79Z',
    target: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18ZM12 17a5 5 0 1 0 0-10 5 5 0 0 0 0 10ZM12 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2Z',
    shield: 'M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10Z',
    bolt: 'M13 2 3 14h9l-1 8 10-12h-9l1-8Z',
    collapse: 'M4 14h6v6M20 10h-6V4M14 10l7-7M3 21l7-7',
    expand: 'M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7',
    note: 'M4 4h10l6 6v10a0 0 0 0 1 0 0H4V4Z M14 4v6h6M8 13h8M8 17h5',
  } as const;

  export type IconName = keyof typeof ICON_PATHS;
</script>

<script lang="ts">
  /** Icon name from the shared set. */
  export let name: IconName;
  /** Pixel size; icons are square. */
  export let size: number | string = 16;
  /**
   * Accessible label. Omit for decorative icons sitting next to real text —
   * they are then hidden from assistive tech instead of being read out.
   */
  export let label: string | null = null;

  $: paths = ICON_PATHS[name].split('M').filter(Boolean).map((d) => `M${d}`);
</script>

<svg
  class="icon"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  role={label ? 'img' : 'presentation'}
  aria-label={label}
  aria-hidden={label ? undefined : 'true'}
  focusable="false"
>
  {#each paths as d}
    <path {d} />
  {/each}
</svg>

<style>
  .icon {
    display: inline-block;
    flex: none;
    vertical-align: -0.125em;
  }
</style>
