import * as Tooltip from '@radix-ui/react-tooltip'
import { AlignLeft, ChevronDown, ChevronUp } from 'lucide-react'
import { useEffect, useState } from 'react'
import { MOBILE_BREAKPOINT_QUERY, useMediaQuery } from '../hooks/useMediaQuery'
import type { EditorSelection } from '../types'
import type { AppWorkspaceProps } from './AppWorkspace.types'
import { EditMetadataModal } from './EditMetadataModal'
import { Editor } from './Editor'
import { EditorToolbarButton } from './EditorToolbarButton'
import { EditPartsModal } from './EditPartsModal'
import { Preview } from './Preview'

export function AppWorkspace({
  editorCollapsed,
  setEditorCollapsed,
  hideEditor,
  editorRef,
  fileId,
  source,
  handleSourceChange,
  handleFormatScore,
  handleShiftSelectionOctave,
  readOnly,
  diagnostics,
  diagnosticViewZones,
  measureSpans,
  setSelectedLineRange,
  notifySelection,
  setEditPartsOpen,
  setEditMetadataOpen,
  forceSave,
  measureAudioPlaying,
  stopMeasurePlayback,
  selectedMeasureRange,
  measureAudioGenerating,
  soundfontReady,
  playSelectedMeasures,
  notePlaybackSelectionActive,
  playNoteSelection,
  editPartsOpen,
  partDeclarations,
  parts,
  handlePartDeclarationChange,
  handleShiftPartOctave,
  previewInstrument,
  previewPercussion,
  stopPreviewInstrument,
  previewAudioPlaying,
  editMetadataOpen,
  parsedMetadata,
  handleMetadataFieldChange,
  documents,
  highlightedDocuments,
  rendering,
  handleSectionJump,
  handleNoteRangeSelect,
  handleEditorSelectionChange,
  selectedNoteCells,
  noteSpans,
  handleLyricRangeSelect,
  handleLyricEditorSelectionChange,
  selectedLyricCells,
  lyricSpans,
  handleMeasureRangeSelect,
  audioGenerating,
  wavUrl,
  wavFilename,
  mp3Exporting,
  mp3Url,
  mp3Filename,
  onRequestAudioDownload,
  noteTimings,
  measureAudioNoteTimings,
  measureAudioElement,
  noPartsSelected,
}: AppWorkspaceProps) {
  const [editorPaneEl, setEditorPaneEl] = useState<HTMLDivElement | null>(null)
  // The editor's current selection, in byte offsets — `null` for a plain
  // caret (nothing to shift), tracked purely to enable/disable the "Octave
  // up"/"Octave down" toolbar buttons and supply their byte ranges on click.
  // A disjoint list, not one min/max range: a Monaco multicursor selection
  // (e.g. clicking a part label, which selects that part's notes across
  // every measure in its system) is generally disjoint, so collapsing it to
  // a single span would sweep in unrelated notes/parts sitting between the
  // disjoint pieces.
  const [selectionByteRanges, setSelectionByteRanges] = useState<
    EditorSelection[] | null
  >(null)
  const isMobile = useMediaQuery(MOBILE_BREAKPOINT_QUERY)
  // Below the mobile breakpoint only one pane is visible at a time, so
  // showing the editor must collapse the preview instead of sitting beside it.
  const previewCollapsed = isMobile && !hideEditor && !editorCollapsed
  // On mobile the divider sits between a stacked editor (above) and preview
  // (below), so the chevron points up/down instead of left/right.
  const toggleIconRotationDeg = isMobile
    ? editorCollapsed
      ? -90
      : 90
    : editorCollapsed
      ? 180
      : 0

  // Default to showing the preview on mobile, where only one pane fits.
  useEffect(() => {
    if (window.matchMedia(MOBILE_BREAKPOINT_QUERY).matches) {
      setEditorCollapsed(() => true)
    }
  }, [setEditorCollapsed])

  return (
    <main className="workspace">
      <section
        className={[
          'pane',
          'pane--editor',
          'pane--collapsible',
          editorCollapsed ? 'pane--editor-collapsed' : '',
        ]
          .filter(Boolean)
          .join(' ')}
      >
        <div className="editor-layout">
          <div className="editor-main" ref={setEditorPaneEl}>
            {hideEditor ? null : (
              <Editor
                ref={editorRef}
                path={fileId}
                value={source}
                onChange={handleSourceChange}
                toolbar={
                  <Tooltip.Provider delayDuration={0}>
                    <EditorToolbarButton
                      label="Format score"
                      icon={<AlignLeft size={14} aria-hidden="true" />}
                      onClick={handleFormatScore}
                    />
                    <EditorToolbarButton
                      label="Octave up"
                      icon={<ChevronUp size={14} aria-hidden="true" />}
                      disabled={selectionByteRanges === null}
                      onClick={() => {
                        if (selectionByteRanges === null) return
                        handleShiftSelectionOctave(selectionByteRanges, 1)
                      }}
                    />
                    <EditorToolbarButton
                      label="Octave down"
                      icon={<ChevronDown size={14} aria-hidden="true" />}
                      disabled={selectionByteRanges === null}
                      onClick={() => {
                        if (selectionByteRanges === null) return
                        handleShiftSelectionOctave(selectionByteRanges, -1)
                      }}
                    />
                  </Tooltip.Provider>
                }
                readOnly={readOnly}
                diagnostics={diagnostics}
                diagnosticViewZones={diagnosticViewZones}
                measureSpans={measureSpans}
                onSelectionChange={(firstLine, lastLine, isEmpty) => {
                  setSelectedLineRange(null)
                  notifySelection(firstLine, lastLine, isEmpty)
                }}
                onSelectionOffsetChange={(ranges, isEmpty) => {
                  // Every disjoint range is passed through, not just the
                  // primary one — a multicursor selection (e.g. a clicked
                  // part label's per-measure ranges, or the "shift
                  // selection octave" toolbar action's remapped ranges)
                  // must keep every range's notes/lyrics highlighted, not
                  // just whichever one Monaco calls the anchor.
                  handleEditorSelectionChange(ranges)
                  handleLyricEditorSelectionChange(ranges)
                  setSelectionByteRanges(isEmpty ? null : ranges)
                }}
                onEditPartsClick={() => setEditPartsOpen(true)}
                onEditMetadataClick={() => setEditMetadataOpen(true)}
                onForceSave={forceSave}
                onPlayMeasure={
                  measureAudioPlaying
                    ? stopMeasurePlayback
                    : !measureAudioGenerating && soundfontReady
                      ? notePlaybackSelectionActive
                        ? playNoteSelection
                        : selectedMeasureRange !== null
                          ? playSelectedMeasures
                          : undefined
                      : undefined
                }
              />
            )}
            <EditPartsModal
              open={editPartsOpen}
              onOpenChange={setEditPartsOpen}
              partDeclarations={partDeclarations}
              allParts={parts}
              onPartDeclarationChange={handlePartDeclarationChange}
              onShiftPartOctave={handleShiftPartOctave}
              previewInstrument={previewInstrument}
              previewPercussion={previewPercussion}
              stopPreviewInstrument={stopPreviewInstrument}
              previewAudioPlaying={previewAudioPlaying}
            />
            <EditMetadataModal
              open={editMetadataOpen}
              onOpenChange={setEditMetadataOpen}
              metadata={parsedMetadata}
              onFieldChange={handleMetadataFieldChange}
              container={editorPaneEl}
            />
          </div>
        </div>
      </section>
      <div className="pane-divider">
        {hideEditor ? null : (
          <div className="pane-divider-toggles">
            <button
              type="button"
              className="pane-divider-toggle"
              onClick={() => setEditorCollapsed((collapsed) => !collapsed)}
              title={editorCollapsed ? 'Show editor' : 'Hide editor'}
              aria-label={editorCollapsed ? 'Show editor' : 'Hide editor'}
            >
              <span
                className="pane-divider-toggle-icon"
                style={{
                  transform: `rotate(${toggleIconRotationDeg}deg)`,
                }}
                aria-hidden="true"
              >
                ‹
              </span>
            </button>
          </div>
        )}
      </div>
      <section
        className={[
          'pane',
          'pane--preview',
          'pane--collapsible',
          previewCollapsed ? 'pane--preview-collapsed' : '',
        ]
          .filter(Boolean)
          .join(' ')}
      >
        <Preview
          documents={documents}
          highlightedDocuments={highlightedDocuments}
          rendering={rendering}
          onSectionLabelClick={handleSectionJump}
          onNoteRangeSelect={handleNoteRangeSelect}
          selectedNoteCells={selectedNoteCells}
          noteSpans={noteSpans}
          onLyricRangeSelect={handleLyricRangeSelect}
          selectedLyricCells={selectedLyricCells}
          lyricSpans={lyricSpans}
          onMeasureRangeSelect={handleMeasureRangeSelect}
          selectedMeasureRange={selectedMeasureRange}
          audioGenerating={audioGenerating}
          wavUrl={wavUrl}
          wavFilename={wavFilename}
          mp3Exporting={mp3Exporting}
          mp3Url={mp3Url}
          mp3Filename={mp3Filename}
          onRequestAudioDownload={onRequestAudioDownload}
          noteTimings={noteTimings}
          measureAudioNoteTimings={measureAudioNoteTimings}
          measureAudioElement={measureAudioElement}
          emptyMessage={
            noPartsSelected ? 'No parts selected.' : 'No preview yet.'
          }
        />
      </section>
    </main>
  )
}
