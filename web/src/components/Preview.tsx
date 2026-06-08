interface PreviewProps {
  svgs: string[]
  rendering: boolean
  wavUrl?: string | null
  audioAvailable?: boolean
  pdfAvailable?: boolean
  pdfExporting?: boolean
  onExportPdf?: () => void
  splitPdfExporting?: boolean
  onExportSplitPdf?: () => void
  partsCount?: number
  emptyMessage?: string
}

export function Preview({
  svgs,
  rendering,
  wavUrl = null,
  audioAvailable = false,
  pdfAvailable = false,
  pdfExporting = false,
  onExportPdf,
  splitPdfExporting = false,
  onExportSplitPdf,
  partsCount = 0,
  emptyMessage = 'No preview yet.',
}: PreviewProps) {
  const exporting = pdfExporting || splitPdfExporting
  const canExportPdf =
    pdfAvailable && svgs.length > 0 && !rendering && !exporting
  const canExportSplitPdf =
    pdfAvailable && partsCount > 0 && !rendering && !exporting

  return (
    <div className="preview">
      <div className="preview-header">
        <span>Preview</span>
        <div className="preview-header-actions">
          {pdfAvailable ? (
            <button
              type="button"
              className="preview-export-btn"
              disabled={!canExportPdf}
              onClick={onExportPdf}
            >
              {pdfExporting ? 'Exporting PDF…' : 'Export PDF'}
            </button>
          ) : null}
          {pdfAvailable ? (
            <button
              type="button"
              className="preview-export-btn"
              disabled={!canExportSplitPdf}
              onClick={onExportSplitPdf}
            >
              {splitPdfExporting ? 'Exporting parts…' : 'Export parts (ZIP)'}
            </button>
          ) : null}
          {rendering ? (
            <span className="preview-status">Rendering…</span>
          ) : null}
        </div>
      </div>
      {audioAvailable ? (
        <div className="preview-audio">
          {wavUrl ? (
            // biome-ignore lint/a11y/useMediaCaption: synthesized score preview has no captions track
            <audio className="preview-audio-player" controls src={wavUrl} />
          ) : (
            <span className="preview-audio-empty">
              {rendering ? 'Generating audio…' : 'No audio yet.'}
            </span>
          )}
        </div>
      ) : null}
      <div className="preview-pages">
        {svgs.length === 0 && !rendering ? (
          <p className="preview-empty">{emptyMessage}</p>
        ) : null}
        {svgs.map((svg) => (
          <div
            key={svg}
            className="preview-page"
            // biome-ignore lint/security/noDangerouslySetInnerHtml: trusted SVG from local WASM renderer
            dangerouslySetInnerHTML={{ __html: svg }}
          />
        ))}
      </div>
    </div>
  )
}
