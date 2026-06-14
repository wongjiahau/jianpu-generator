interface PlayMeasureButtonProps {
  disabled: boolean
  loading: boolean
  measureNumber: number | null
  onClick: () => void
}

export function PlayMeasureButton({
  disabled,
  loading,
  measureNumber,
  onClick,
}: PlayMeasureButtonProps) {
  return (
    <button
      type="button"
      className="play-measure-btn"
      disabled={disabled}
      onClick={onClick}
      title={
        measureNumber === null
          ? 'Move cursor into a measure to enable'
          : 'Play current measure'
      }
      aria-label={
        measureNumber !== null ? `Play measure ${measureNumber}` : 'Play current measure'
      }
    >
      {loading ? (
        <span className="play-measure-spinner" aria-hidden="true" />
      ) : measureNumber !== null ? (
        `▶ ${measureNumber}`
      ) : (
        '▶'
      )}
    </button>
  )
}
