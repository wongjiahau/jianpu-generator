import type { Diagnostic } from '../types'

interface ErrorPanelProps {
  diagnostics: Diagnostic[]
}

export function ErrorPanel({ diagnostics }: ErrorPanelProps) {
  if (diagnostics.length === 0) return null

  const primary = diagnostics[0]

  return (
    <div className="error-panel" role="alert">
      <div className="error-panel-summary">
        <span className="error-panel-message">{primary.message}</span>
        <span className="error-panel-span">
          bytes {primary.span.start}–{primary.span.end}
        </span>
      </div>
      {primary.report ? (
        <pre className="error-panel-report">{primary.report}</pre>
      ) : null}
      {diagnostics.length > 1 ? (
        <ul className="error-panel-more">
          {diagnostics.slice(1).map((d) => (
            <li key={`${d.span.start}-${d.span.end}-${d.message}`}>
              {d.message}{' '}
              <span className="error-panel-span">
                (bytes {d.span.start}–{d.span.end})
              </span>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  )
}
