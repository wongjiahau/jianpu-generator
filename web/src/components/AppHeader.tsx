import type { FileStoreState } from '../fileStore'
import { sortedBinNames } from '../fileStore'
import type { DisplaySaveStatus } from '../hooks/useStorageBackend'
import type { SyncedShareViewerStatus } from '../hooks/useSyncedShareViewer'
import type { SharePayload } from '../shareUrl'
import { ExportControls } from './ExportControls'
import { FileSwitcher } from './FileSwitcher'
import { PlayAllButton } from './PlayAllButton'
import { PlayFromCurrentMeasureButton } from './PlayFromCurrentMeasureButton'
import { PlayMeasureButton } from './PlayMeasureButton'
import { SharedPreviewBanner } from './SharedPreviewBanner'
import { SyncedShareBanner } from './SyncedShareBanner'
import { SyncedShareButton } from './SyncedShareButton'

interface MeasureRange {
  start: number
  end: number
}

interface SyncedShareHeaderProps {
  sharedPreview: SharePayload | null
  onImportShared: () => void
  onDismissShared: () => void
  /** Non-null while a `#synced=` link is being viewed. Takes a back seat to
   * `sharedPreview` if both are somehow present at once (a documented edge
   * case, not expected in practice). */
  viewerActive: boolean
  viewerStatus: SyncedShareViewerStatus
  viewerFilename: string | null
  onImportSyncedShare: () => void
  isSynced: boolean
  syncedShareLink: string | null
  onStartSync: () => string
  onStopSync: () => void
}

interface AppHeaderProps {
  audioAvailable?: boolean
  selectedMeasureRange: MeasureRange | null
  selectedSequenceRange: MeasureRange | null
  measureAudioGenerating: boolean
  soundfontReady: boolean
  measureAudioPlaying: boolean
  playSelectedMeasures: () => void
  playFromCurrentMeasure: () => void
  playAll: () => void
  /** True while a note range-select (see `useNoteSelection`) is active; when
   * set, `PlayMeasureButton` plays only the selected parts, muted elsewhere,
   * over the selection's measure range instead of the measure(s) under the
   * cursor. */
  notePlaybackSelectionActive: boolean
  playNoteSelection: () => void
  stopMeasurePlayback: () => void
  shortcutLabel: string
  playFromCurrentMeasureShortcutLabel: string
  store: FileStoreState
  onSelect: (name: string) => void
  onCreate: () => void
  onDuplicate: () => void
  onRename: (from: string, to: string) => void
  onDelete: (name: string) => void
  onOpenStorageSettings: () => void
  saveStatus: DisplaySaveStatus
  autosaveDeadline: number | null
  creatingFile?: boolean
  deletingFileName?: string | null
  duplicatingFile?: boolean
  renamingFileName?: string | null
  isLoadingGithub?: boolean
  onOpenBin: () => void
  hasDocuments: boolean
  rendering: boolean
  audioGenerating?: boolean
  wavUrl?: string | null
  onGenerateAudio?: () => void
  pdfAvailable?: boolean
  pdfFontsReady?: boolean
  pdfExporting?: boolean
  onExportPdf?: () => void
  splitPdfExporting?: boolean
  onExportSplitPdf?: () => void
  midiAvailable?: boolean
  midiExporting?: boolean
  onExportMidi?: () => void
  splitMidiExporting?: boolean
  onExportSplitMidi?: () => void
  splitWavExporting?: boolean
  onExportSplitWav?: () => void
  mp3Available?: boolean
  mp3Exporting?: boolean
  mp3Url?: string | null
  onExportMp3?: () => void
  splitMp3Exporting?: boolean
  onExportSplitMp3?: () => void
  partsCount?: number
  importing?: boolean
  onImportFile?: (file: File) => void
  syncedShare: SyncedShareHeaderProps
}

export function AppHeader({
  audioAvailable,
  selectedMeasureRange,
  selectedSequenceRange,
  measureAudioGenerating,
  soundfontReady,
  measureAudioPlaying,
  playSelectedMeasures,
  playFromCurrentMeasure,
  playAll,
  notePlaybackSelectionActive,
  playNoteSelection,
  stopMeasurePlayback,
  shortcutLabel,
  playFromCurrentMeasureShortcutLabel,
  store,
  onSelect,
  onCreate,
  onDuplicate,
  onRename,
  onDelete,
  onOpenStorageSettings,
  saveStatus,
  autosaveDeadline,
  creatingFile,
  deletingFileName,
  duplicatingFile,
  renamingFileName,
  isLoadingGithub,
  onOpenBin,
  hasDocuments,
  rendering,
  audioGenerating,
  wavUrl,
  onGenerateAudio,
  pdfAvailable,
  pdfFontsReady,
  pdfExporting,
  onExportPdf,
  splitPdfExporting,
  onExportSplitPdf,
  midiAvailable,
  midiExporting,
  onExportMidi,
  splitMidiExporting,
  onExportSplitMidi,
  splitWavExporting,
  onExportSplitWav,
  mp3Available,
  mp3Exporting,
  mp3Url,
  onExportMp3,
  splitMp3Exporting,
  onExportSplitMp3,
  partsCount,
  importing,
  onImportFile,
  syncedShare,
}: AppHeaderProps) {
  const { sharedPreview, viewerActive: syncedShareViewerActive } = syncedShare
  return (
    <header className="app-header">
      {sharedPreview ? (
        <SharedPreviewBanner
          onImport={syncedShare.onImportShared}
          onDiscard={syncedShare.onDismissShared}
        />
      ) : (
        syncedShareViewerActive && (
          <SyncedShareBanner
            status={syncedShare.viewerStatus}
            filename={syncedShare.viewerFilename}
            onImport={syncedShare.onImportSyncedShare}
          />
        )
      )}
      {audioAvailable && (
        <PlayMeasureButton
          disabled={
            (notePlaybackSelectionActive
              ? false
              : selectedMeasureRange === null) ||
            measureAudioGenerating ||
            !soundfontReady
          }
          loading={measureAudioGenerating}
          playing={measureAudioPlaying}
          measureRange={selectedMeasureRange}
          noteSelectionActive={notePlaybackSelectionActive}
          onClick={
            notePlaybackSelectionActive
              ? playNoteSelection
              : playSelectedMeasures
          }
          onPause={stopMeasurePlayback}
          shortcutLabel={shortcutLabel}
        />
      )}
      {audioAvailable && selectedSequenceRange !== null && (
        <PlayFromCurrentMeasureButton
          disabled={
            selectedSequenceRange === null ||
            measureAudioGenerating ||
            !soundfontReady
          }
          loading={measureAudioGenerating}
          playing={measureAudioPlaying}
          currentMeasure={selectedSequenceRange?.start ?? null}
          onClick={playFromCurrentMeasure}
          onPause={stopMeasurePlayback}
          shortcutLabel={playFromCurrentMeasureShortcutLabel}
        />
      )}
      {audioAvailable && (
        <PlayAllButton
          disabled={measureAudioGenerating || !soundfontReady}
          loading={measureAudioGenerating}
          playing={measureAudioPlaying}
          onClick={playAll}
          onPause={stopMeasurePlayback}
        />
      )}
      <div className="app-header-actions">
        {!sharedPreview && !syncedShareViewerActive && (
          <FileSwitcher
            store={store}
            triggerLabel={store.active}
            onSelect={onSelect}
            onCreate={onCreate}
            onDuplicate={onDuplicate}
            onRename={onRename}
            onDelete={onDelete}
            onOpenStorageSettings={onOpenStorageSettings}
            saveStatus={saveStatus}
            autosaveDeadline={autosaveDeadline}
            creating={creatingFile}
            deletingName={deletingFileName}
            duplicating={duplicatingFile}
            renamingName={renamingFileName}
            isLoadingGithub={isLoadingGithub}
            importing={importing}
            onImportFile={onImportFile}
            binNames={sortedBinNames(store)}
            onOpenBin={onOpenBin}
          />
        )}
        {!sharedPreview && !syncedShareViewerActive && (
          <SyncedShareButton
            isSynced={syncedShare.isSynced}
            syncedShareLink={syncedShare.syncedShareLink}
            onStartSync={syncedShare.onStartSync}
            onStopSync={syncedShare.onStopSync}
          />
        )}
        <ExportControls
          hasDocuments={hasDocuments}
          rendering={rendering}
          audioGenerating={audioGenerating}
          wavUrl={wavUrl}
          soundfontReady={soundfontReady}
          onGenerateAudio={onGenerateAudio}
          pdfAvailable={pdfAvailable}
          pdfFontsReady={pdfFontsReady}
          pdfExporting={pdfExporting}
          onExportPdf={onExportPdf}
          splitPdfExporting={splitPdfExporting}
          onExportSplitPdf={onExportSplitPdf}
          midiAvailable={midiAvailable}
          midiExporting={midiExporting}
          onExportMidi={onExportMidi}
          splitMidiExporting={splitMidiExporting}
          onExportSplitMidi={onExportSplitMidi}
          audioAvailable={audioAvailable}
          splitWavExporting={splitWavExporting}
          onExportSplitWav={onExportSplitWav}
          mp3Available={mp3Available}
          mp3Exporting={mp3Exporting}
          mp3Url={mp3Url}
          onExportMp3={onExportMp3}
          splitMp3Exporting={splitMp3Exporting}
          onExportSplitMp3={onExportSplitMp3}
          partsCount={partsCount}
          isLoadingGithub={isLoadingGithub}
        />
      </div>
    </header>
  )
}
