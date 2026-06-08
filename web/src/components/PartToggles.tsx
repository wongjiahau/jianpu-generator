import type { PartInfo } from '../types'

interface PartTogglesProps {
  parts: PartInfo[]
  disabledParts: ReadonlySet<string>
  disabledLyrics: ReadonlySet<string>
  onPartToggle: (abbreviation: string, enabled: boolean) => void
  onLyricsToggle: (abbreviation: string, enabled: boolean) => void
  loading?: boolean
}

export function PartToggles({
  parts,
  disabledParts,
  disabledLyrics,
  onPartToggle,
  onLyricsToggle,
  loading = false,
}: PartTogglesProps) {
  if (parts.length === 0) {
    return null
  }

  return (
    <fieldset className="part-toggles">
      <legend className="part-toggles-label">Parts</legend>
      {loading ? <span className="part-toggles-status">Updating…</span> : null}
      <ul className="part-toggles-list">
        {parts.map((part) => {
          const enabled = !disabledParts.has(part.abbreviation)
          const lyricsEnabled = !disabledLyrics.has(part.abbreviation)
          const title =
            part.display_name === part.abbreviation
              ? part.abbreviation
              : `${part.display_name} (${part.abbreviation})`

          return (
            <li key={part.abbreviation} className="part-toggle-group">
              <label className="part-toggle" title={title}>
                <input
                  type="checkbox"
                  checked={enabled}
                  onChange={(event) =>
                    onPartToggle(part.abbreviation, event.target.checked)
                  }
                />
                <span className="part-toggle-label">{part.abbreviation}</span>
              </label>
              {part.has_lyrics ? (
                <label
                  className="part-toggle part-toggle--lyrics"
                  title={`${title} lyrics`}
                >
                  <input
                    type="checkbox"
                    checked={lyricsEnabled}
                    disabled={!enabled}
                    onChange={(event) =>
                      onLyricsToggle(part.abbreviation, event.target.checked)
                    }
                  />
                  <span className="part-toggle-label">lyrics</span>
                </label>
              ) : null}
            </li>
          )
        })}
      </ul>
    </fieldset>
  )
}
