import type {
  GenerateMp3Response,
  GenerateWavResponse,
  NoteTimingOut,
  NoteTimingsResponse,
} from '../jianpuWasm'
import { computeNoteSelectionTrimWindow } from '../utils/noteSelectionTrim'
import type { WorkerRequest, WorkerResponse } from './jianpu.worker'

function binaryBufferFromResult(
  bytes: Uint8Array | ArrayBuffer | ArrayLike<number>,
): ArrayBuffer {
  if (bytes instanceof ArrayBuffer) {
    return bytes.slice(0)
  }
  const view = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes)
  return view.slice().buffer
}

function noteTimingsFromSource(
  listNoteTimings:
    | ((
        source: string,
        visibleTracks?: string[],
        enabledTracks?: string[],
      ) => NoteTimingsResponse)
    | null,
  source: string,
  visibleTracks: string[] | undefined,
  enabledTracks: string[] | undefined,
): NoteTimingOut[] {
  if (!listNoteTimings) return []
  const result = listNoteTimings(source, visibleTracks, enabledTracks)
  return result.status === 'ok' ? result.timings : []
}

function noteTimingsForRangeFromSource(
  listNoteTimingsForRange:
    | ((
        source: string,
        startIndex: number,
        endIndex: number,
        extendToLastOccurrence: boolean,
        respectSequence: boolean,
        sequenceEntryStartIndex: number | undefined,
        sequenceEntryEndIndex: number | undefined,
        visibleTracks?: string[],
        enabledTracks?: string[],
      ) => NoteTimingsResponse)
    | null,
  source: string,
  startMeasureIndex: number,
  endMeasureIndex: number,
  extendToLastOccurrence: boolean,
  respectSequence: boolean,
  sequenceEntryStartIndex: number | undefined,
  sequenceEntryEndIndex: number | undefined,
  visibleTracks: string[] | undefined,
  enabledTracks: string[] | undefined,
): NoteTimingOut[] {
  if (!listNoteTimingsForRange) return []
  const result = listNoteTimingsForRange(
    source,
    startMeasureIndex,
    endMeasureIndex,
    extendToLastOccurrence,
    respectSequence,
    sequenceEntryStartIndex,
    sequenceEntryEndIndex,
    visibleTracks,
    enabledTracks,
  )
  return result.status === 'ok' ? result.timings : []
}

type GenerateWavFn =
  | ((
      source: string,
      enabledTracks: string[] | undefined,
      soundfont: Uint8Array,
    ) => GenerateWavResponse)
  | null

type ListNoteTimingsFn =
  | ((
      source: string,
      visibleTracks?: string[],
      enabledTracks?: string[],
    ) => NoteTimingsResponse)
  | null

export function handleGenerateAudio(
  msg: Extract<WorkerRequest, { type: 'generateAudio' }>,
  generateWav: GenerateWavFn,
  listNoteTimings: ListNoteTimingsFn,
  loadedSoundfont: Uint8Array | null,
): void {
  if (!generateWav || !loadedSoundfont) {
    postMessage({ type: 'audioErr', id: msg.id } satisfies WorkerResponse)
    return
  }
  const wavResult = generateWav(msg.source, msg.enabledTracks, loadedSoundfont)
  if (wavResult.status === 'ok' && wavResult.wav != null) {
    const wavBuffer = binaryBufferFromResult(wavResult.wav)
    // `msg.enabledTracks` here is always the part-visibility toggle's state
    // (never a playback-only mute override — this handler has no such
    // concept), so it doubles as `visibleTracks`. Passing it as Rust's own
    // `visible_tracks` makes `source_part_index` already agree with the
    // rendered SVG's `data-part-index` (including a `MultiMeasureRest` run
    // only created once a hidden sibling part's notes are removed) — no
    // further client-side remap needed.
    const noteTimings = noteTimingsFromSource(
      listNoteTimings,
      msg.source,
      msg.enabledTracks,
      undefined,
    )
    postMessage(
      {
        type: 'audio',
        id: msg.id,
        wav: wavBuffer,
        noteTimings,
      } satisfies WorkerResponse,
      { transfer: [wavBuffer] },
    )
    return
  }
  postMessage({ type: 'audioErr', id: msg.id } satisfies WorkerResponse)
}

type GenerateWavForMeasureRangeFn =
  | ((
      source: string,
      startIndex: number,
      endIndex: number,
      extendToLastOccurrence: boolean,
      respectSequence: boolean,
      sequenceEntryStartIndex: number | undefined,
      sequenceEntryEndIndex: number | undefined,
      enabledTracks: string[] | undefined,
      trimStartS: number | undefined,
      trimEndS: number | undefined,
      trimNextNoteStartS: number | undefined,
      soundfont: Uint8Array,
    ) => GenerateWavResponse)
  | null

type ListNoteTimingsForRangeFn =
  | ((
      source: string,
      startIndex: number,
      endIndex: number,
      extendToLastOccurrence: boolean,
      respectSequence: boolean,
      sequenceEntryStartIndex: number | undefined,
      sequenceEntryEndIndex: number | undefined,
      visibleTracks?: string[],
      enabledTracks?: string[],
    ) => NoteTimingsResponse)
  | null

/** Shifts every timing's `start_s`/`end_s` back by `trimStartS`, so they
 * stay relative to the start of a clip that Rust has sample-accurately
 * trimmed down to `[trimStartS, trimEndS]` (see `crate::wav::TrimWindow`)
 * instead of the full, untrimmed measure-range clip they were originally
 * computed against. */
function shiftNoteTimings(
  timings: NoteTimingOut[],
  trimStartS: number,
): NoteTimingOut[] {
  return timings.map((t) => ({
    ...t,
    start_s: t.start_s - trimStartS,
    end_s: t.end_s - trimStartS,
  }))
}

export function handleGenerateMeasureRangeAudio(
  msg: Extract<WorkerRequest, { type: 'generateMeasureRangeAudio' }>,
  generateWavForMeasureRange: GenerateWavForMeasureRangeFn,
  listNoteTimingsForRange: ListNoteTimingsForRangeFn,
  loadedSoundfont: Uint8Array | null,
): void {
  if (!generateWavForMeasureRange || !loadedSoundfont) {
    postMessage({
      type: 'measureRangeAudioErr',
      id: msg.id,
    } satisfies WorkerResponse)
    return
  }
  // `msg.visibleTracks` (the part-visibility toggle's own state) is passed
  // as Rust's own `visible_tracks`, so `source_part_index`/block structure
  // (including a `MultiMeasureRest` run only created once a hidden sibling
  // part's notes are removed) already agree with the currently rendered
  // SVG's `data-part-index`/`data-note-id` — no client-side remap needed.
  // `msg.enabledTracks` is this clip's own, possibly narrower, playback mute
  // (e.g. "play selection"), applied on top without affecting either.
  const fullRangeNoteTimings = noteTimingsForRangeFromSource(
    listNoteTimingsForRange,
    msg.source,
    msg.startMeasureIndex,
    msg.endMeasureIndex,
    msg.extendToLastOccurrence,
    msg.respectSequence,
    msg.sequenceEntryStartIndex,
    msg.sequenceEntryEndIndex,
    msg.visibleTracks,
    msg.enabledTracks,
  )
  // "Play selection": narrow the clip Rust synthesizes down to exactly the
  // range-selected notes' elapsed-seconds span (sample-accurate trim/fade —
  // see `crate::wav::TrimWindow`), derived from the full range's note
  // timings fetched above. `undefined` for a plain measure-range play
  // (every other caller), which always plays the range in full.
  const trim = msg.trimToSelectedNoteCells
    ? computeNoteSelectionTrimWindow(
        msg.trimToSelectedNoteCells,
        fullRangeNoteTimings,
      )
    : null
  const wavResult = generateWavForMeasureRange(
    msg.source,
    msg.startMeasureIndex,
    msg.endMeasureIndex,
    msg.extendToLastOccurrence,
    msg.respectSequence,
    msg.sequenceEntryStartIndex,
    msg.sequenceEntryEndIndex,
    msg.enabledTracks,
    trim?.start,
    trim?.end,
    trim?.nextNoteStart,
    loadedSoundfont,
  )
  if (wavResult.status === 'ok' && wavResult.wav != null) {
    const wavBuffer = binaryBufferFromResult(wavResult.wav)
    postMessage(
      {
        type: 'measureRangeAudio',
        id: msg.id,
        wav: wavBuffer,
        noteTimings: trim
          ? shiftNoteTimings(fullRangeNoteTimings, trim.start)
          : fullRangeNoteTimings,
      } satisfies WorkerResponse,
      { transfer: [wavBuffer] },
    )
    return
  }
  postMessage({
    type: 'measureRangeAudioErr',
    id: msg.id,
  } satisfies WorkerResponse)
}

type GenerateMp3Fn =
  | ((
      source: string,
      enabledTracks: string[] | undefined,
      soundfont: Uint8Array,
    ) => GenerateMp3Response)
  | null

/**
 * One-shot MP3 export — like [`handleGenerateAudio`], this also produces the
 * WAV preview's interactive playback cursor: MP3 gets the same inline
 * player, so it needs the same note timings. `listNoteTimings` runs off the
 * source alone (see `noteTimingsFromSource`), independent of the audio
 * codec, so this reuses it exactly as the WAV path does.
 */
export function handleGenerateMp3(
  msg: Extract<WorkerRequest, { type: 'generateMp3' }>,
  generateMp3: GenerateMp3Fn,
  listNoteTimings: ListNoteTimingsFn,
  loadedSoundfont: Uint8Array | null,
): void {
  if (!generateMp3 || !loadedSoundfont) {
    postMessage({
      type: 'mp3Err',
      id: msg.id,
      diagnostics: [
        {
          severity: 'error',
          message: loadedSoundfont
            ? 'MP3 export is not available in this build.'
            : 'Soundfont is not yet loaded.',
          span: { start: 0, end: 0 },
        },
      ],
    } satisfies WorkerResponse)
    return
  }
  const result = generateMp3(msg.source, msg.enabledTracks, loadedSoundfont)
  if (result.status === 'ok') {
    const mp3Buffer = binaryBufferFromResult(result.mp3)
    const noteTimings = noteTimingsFromSource(
      listNoteTimings,
      msg.source,
      msg.enabledTracks,
      undefined,
    )
    postMessage(
      {
        type: 'mp3',
        id: msg.id,
        mp3: mp3Buffer,
        noteTimings,
      } satisfies WorkerResponse,
      { transfer: [mp3Buffer] },
    )
    return
  }
  postMessage({
    type: 'mp3Err',
    id: msg.id,
    diagnostics: result.diagnostics,
  } satisfies WorkerResponse)
}
