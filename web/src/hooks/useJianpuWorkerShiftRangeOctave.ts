import type { RefObject } from 'react'
import { useCallback } from 'react'
import type { EditorSelection } from '../types'
import type { WorkerRequest } from '../worker/jianpu.worker'
import type { RangeOctaveShiftRequestTracker } from './useJianpuWorkerTypes'

interface UseJianpuWorkerShiftRangeOctaveParams {
  workerRef: RefObject<Worker | null>
  sourceRef: RefObject<string>
  shiftRangeOctaveTracker: RangeOctaveShiftRequestTracker
}

/** Sends a "shift selection octave" request for a disjoint set of byte
 * ranges (a Monaco multicursor selection, e.g. every measure a clicked part
 * label's notes span, need not be one contiguous range) to the worker and
 * resolves once it replies with the rewritten `.jianpu` source *and* the
 * shifted notes' own byte ranges in that new source — the range-scoped
 * counterpart of `useJianpuWorkerShiftOctave`. The `ranges` half is what lets
 * the caller restore the editor selection synchronously alongside the new
 * source (see `HANDOFF-octave-toolbar-part-label-selection-bug.md`). */
export function useJianpuWorkerShiftRangeOctave({
  workerRef,
  sourceRef,
  shiftRangeOctaveTracker,
}: UseJianpuWorkerShiftRangeOctaveParams) {
  const shiftRangeOctave = useCallback(
    (ranges: EditorSelection[], delta: number) =>
      new Promise<{ source: string; ranges: EditorSelection[] }>((resolve) => {
        const worker = workerRef.current
        if (!worker) {
          resolve({ source: sourceRef.current, ranges: [] })
          return
        }
        const id = ++shiftRangeOctaveTracker.requestIdRef.current
        shiftRangeOctaveTracker.latestIdRef.current = id
        shiftRangeOctaveTracker.pendingRequestsRef.current.set(id, resolve)
        worker.postMessage({
          type: 'shiftRangeOctave',
          source: sourceRef.current,
          ranges,
          delta,
          id,
        } satisfies WorkerRequest)
      }),
    [workerRef, sourceRef, shiftRangeOctaveTracker],
  )

  return { shiftRangeOctave }
}
