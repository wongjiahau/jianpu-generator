import { useEffect, useRef } from 'react'

interface UseKeyboardShortcutsOptions {
  measureAudioPlaying: boolean
  measureAudioGenerating: boolean
  soundfontReady: boolean
  selectedMeasureRange: { start: number; end: number } | null
  selectedSequenceRange: { start: number; end: number } | null
  playSelectedMeasures: () => void
  playFromCurrentMeasure: () => void
  /** True while a note range-select (see `useNoteSelection`) is active; when
   * set, Cmd/Ctrl+Enter plays the selected notes instead of the measure(s)
   * under the cursor. */
  notePlaybackSelectionActive: boolean
  playNoteSelection: () => void
  stopMeasurePlayback: () => void
  forceSave: () => void
}

export function useKeyboardShortcuts({
  measureAudioPlaying,
  measureAudioGenerating,
  soundfontReady,
  selectedMeasureRange,
  selectedSequenceRange,
  playSelectedMeasures,
  playFromCurrentMeasure,
  notePlaybackSelectionActive,
  playNoteSelection,
  stopMeasurePlayback,
  forceSave,
}: UseKeyboardShortcutsOptions) {
  const canPlaySelection =
    !measureAudioGenerating &&
    soundfontReady &&
    (notePlaybackSelectionActive || selectedMeasureRange !== null)
  const canPlayFromCurrentMeasure =
    selectedSequenceRange !== null && !measureAudioGenerating && soundfontReady

  const playMeasureRef = useRef<(() => void) | undefined>(undefined)
  playMeasureRef.current = measureAudioPlaying
    ? stopMeasurePlayback
    : canPlaySelection
      ? notePlaybackSelectionActive
        ? playNoteSelection
        : playSelectedMeasures
      : undefined

  const playFromCurrentMeasureRef = useRef<(() => void) | undefined>(undefined)
  playFromCurrentMeasureRef.current = measureAudioPlaying
    ? stopMeasurePlayback
    : canPlayFromCurrentMeasure
      ? playFromCurrentMeasure
      : undefined

  const forceSaveRef = useRef(forceSave)
  forceSaveRef.current = forceSave

  useEffect(() => {
    const isMac = navigator.platform.startsWith('Mac')
    const onKeyDown = (event: KeyboardEvent) => {
      const modifier = isMac ? event.metaKey : event.ctrlKey
      if (modifier && event.shiftKey && event.key === 'Enter') {
        event.preventDefault()
        playFromCurrentMeasureRef.current?.()
      } else if (modifier && event.key === 'Enter') {
        event.preventDefault()
        playMeasureRef.current?.()
      } else if (modifier && event.key.toLowerCase() === 's') {
        event.preventDefault()
        forceSaveRef.current()
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [])
}
