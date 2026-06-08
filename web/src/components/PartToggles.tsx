import type { PartInfo } from '../types'

interface PartTogglesProps {
  parts: PartInfo[]
  disabledParts: ReadonlySet<string>
  onToggle: (abbreviation: string, enabled: boolean) => void
  loading?: boolean
}

export function PartToggles({
  parts,
  disabledParts,
  onToggle,
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
          const title =
            part.display_name === part.abbreviation
              ? part.abbreviation
              : `${part.display_name} (${part.abbreviation})`

          return (
            <li key={part.abbreviation}>
              <label className="part-toggle" title={title}>
                <input
                  type="checkbox"
                  checked={enabled}
                  onChange={(event) =>
                    onToggle(part.abbreviation, event.target.checked)
                  }
                />
                <span className="part-toggle-label">{part.abbreviation}</span>
              </label>
            </li>
          )
        })}
      </ul>
    </fieldset>
  )
}
