import type { TransparentRectRoleOut } from './jianpuWasm'

/**
 * Every `data-variant` attribute value written onto rendered preview SVG
 * elements — the kebab-case wire format for `TransparentRectRoleOut`
 * (`crates/jianpu-wasm/src/svg_types.rs:20-30`), plus `playbackCursorRect`
 * (a distinct `SvgKindOut::PlaybackCursorRect` element, not one of the
 * `TransparentRectRoleOut` variants, but a `data-variant` value in its own
 * right).
 *
 * Single source of truth for every place that used to re-type these strings
 * independently: `PreviewSvgRenderer.tsx`'s `transparentRectRoleToDataVariant`
 * switch and `playbackCursorRect` case, `usePlaybackCursor.ts`,
 * `previewRangeHighlights.ts`, and `previewLabelRangeHighlights.ts`'s
 * `querySelector`/`closest` selectors — mirroring how `data-tag` is already
 * dispatched off a typed value in `PreviewSvgRenderer.tsx`'s
 * `groupAttrsForTag` rather than re-typed per consumer. `preview.css` and
 * `index.css` are plain CSS and can't import this constant, so
 * `dataVariant.test.ts` instead asserts every `data-variant` literal they
 * contain is still a value produced here — a rename here that isn't mirrored
 * in the CSS fails that test instead of the CSS rule silently matching
 * nothing.
 */
export const DATA_VARIANT = {
  measureClickTarget: 'measure-click-target-rect',
  barNumberClickTarget: 'bar-number-click-target-rect',
  sectionLabelBackground: 'section-label-bg',
  sectionLabelClickTarget: 'section-label-click-target-rect',
  noteClickTarget: 'note-click-target-rect',
  partLabelClickTarget: 'part-label-click-target-rect',
  lyricClickTarget: 'lyric-click-target-rect',
  lyricLabelClickTarget: 'lyric-label-click-target-rect',
  barLineClickTarget: 'bar-line-click-target-rect',
  playbackCursorRect: 'playback-cursor-rect',
} as const satisfies Record<
  TransparentRectRoleOut | 'playbackCursorRect',
  string
>
