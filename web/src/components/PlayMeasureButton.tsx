import { PauseIcon, PlayIcon } from './icons/PlaybackIcons'

interface PlayMeasureButtonProps {
  disabled: boolean
  loading: boolean
  playing: boolean
  measureRange: { start: number; end: number } | null
  /** True while a note range-select (see `useNoteSelection`) is active. When
   * set, the button plays only the selected notes instead of the measure(s)
   * under the cursor, and its label/tooltip reflect that. */
  noteSelectionActive: boolean
  onClick: () => void
  onPause: () => void
  shortcutLabel: string
}

function measureLabel(range: { start: number; end: number }): string {
  if (range.start === range.end) {
    return `Measure ${range.start + 1}`
  }
  return `Measures ${range.start + 1}–${range.end + 1}`
}

function ShortcutKeys({ label }: { label: string }) {
  const keys = label.includes('+') ? label.split('+') : [...label]
  return (
    <span className="play-measure-shortcut-keys">
      {keys.map((key, index) => (
        <span key={key}>
          {index > 0 && <span className="play-measure-shortcut-sep">+</span>}
          <kbd className="play-measure-kbd">{key}</kbd>
        </span>
      ))}
    </span>
  )
}

function Tooltip({
  shortcutLabel,
  text,
}: {
  shortcutLabel: string
  text: string
}) {
  return (
    <div className="play-measure-tooltip" role="tooltip">
      <span className="play-measure-tooltip-text">{text}</span>
      <ShortcutKeys label={shortcutLabel} />
    </div>
  )
}

export function PlayMeasureButton({
  disabled,
  loading,
  playing,
  measureRange,
  noteSelectionActive,
  onClick,
  onPause,
  shortcutLabel,
}: PlayMeasureButtonProps) {
  const label = noteSelectionActive
    ? 'Selection'
    : measureRange !== null
      ? measureLabel(measureRange)
      : null

  if (playing) {
    return (
      <div className="play-measure-wrapper">
        <button
          type="button"
          className="play-measure-btn play-measure-btn--playing"
          data-testid="play-measure-button"
          onClick={onPause}
          aria-label={label ? `Pause ${label}` : 'Pause playback'}
        >
          <PauseIcon className="play-btn-icon" />
          {label ? ` ${label}` : null}
        </button>
        <Tooltip shortcutLabel={shortcutLabel} text="Pause playback" />
      </div>
    )
  }

  return (
    <div className="play-measure-wrapper">
      <button
        type="button"
        className="play-measure-btn"
        data-testid="play-measure-button"
        disabled={disabled}
        onClick={onClick}
        aria-label={
          label
            ? `Play ${label}`
            : noteSelectionActive
              ? 'Play selection'
              : 'Play selected measure(s)'
        }
      >
        {loading ? (
          <span className="play-measure-spinner" aria-hidden="true" />
        ) : (
          <>
            <PlayIcon className="play-btn-icon" />
            {label !== null ? ` ${label}` : null}
          </>
        )}
      </button>
      <Tooltip
        shortcutLabel={shortcutLabel}
        text={
          noteSelectionActive
            ? 'Play selected notes'
            : measureRange === null
              ? 'Move cursor into a measure to enable'
              : 'Play selected measure(s)'
        }
      />
    </div>
  )
}
