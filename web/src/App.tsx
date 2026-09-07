import { AppHeader } from './components/AppHeader'
import { AppOverlays } from './components/AppOverlays'
import { AppWorkspace } from './components/AppWorkspace'
import { AssetLoadingBanner } from './components/AssetLoadingBanner'
import { ExportAudioToast } from './components/ExportAudioToast'
import { PartToggles } from './components/PartToggles'
import { SectionJumpToolbar } from './components/SectionJumpToolbar'
import { SequenceJumpToolbar } from './components/SequenceJumpToolbar'
import { useAppController } from './hooks/useAppController'
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts'
import {
  playFromCurrentMeasureShortcutLabel,
  shortcutLabel,
} from './utils/shortcutLabels'
import './App.css'
import './file-switcher.css'
import './preview.css'

export default function App() {
  const {
    store,
    setStore,
    backend,
    isLoadingGithub,
    saveStatus,
    autosaveDeadline,
    preference,
    switchBackend,
    forceSave,
    refreshSaveStatus,
    editorCollapsed,
    setEditorCollapsed,
    creatingFile,
    deletingFileName,
    duplicatingFile,
    renamingFileName,
    restoringFileName,
    fileOpError,
    setFileOpError,
    handleCreate,
    handleDuplicate,
    handleRename,
    handleDelete,
    handleRestore,
    sharedPreview,
    syncedShareViewerActive,
    source,
    readOnly,
    syncedShare,
    fileId,
    editorRef,
    soundfont,
    fonts,
    wasm,
    soundfontReady,
    pdfFontsReady,
    disabledParts,
    disabledLyrics,
    soloedParts,
    handlePartToggle,
    handleLyricsToggle,
    handleSoloToggle,
    parts,
    partDeclarations,
    documents,
    pendingDownload,
    requestDownload,
    confirmPendingDownload,
    cancelPendingDownload,
    wavUrl,
    wavFilename,
    mp3Url,
    mp3Filename,
    noteTimings,
    audioAvailable,
    pdfAvailable,
    pdfExporting,
    diagnostics,
    diagnosticViewZones,
    rendering,
    audioGenerating,
    exportPdf,
    splitPdfExporting,
    exportSplitPdf,
    midiAvailable,
    midiExporting,
    exportMidi,
    splitMidiExporting,
    exportSplitMidi,
    splitWavExporting,
    exportSplitWav,
    mp3Available,
    mp3Exporting,
    exportMp3,
    splitMp3Exporting,
    exportSplitMp3,
    generateFullAudio,
    selectedMeasureRange,
    measureAudioGenerating,
    measureAudioPlaying,
    measureAudioNoteTimings,
    measureAudioElement,
    measureSpans,
    selectedSequenceRange,
    sequenceJumpToolbarProps,
    notifySelection,
    playSelectedMeasures,
    playFromCurrentMeasure,
    playAll,
    stopMeasurePlayback,
    highlightedDocuments,
    noteSpans,
    lyricSpans,
    previewInstrument,
    previewPercussion,
    stopPreviewInstrument,
    previewAudioPlaying,
    handleSourceChange,
    handleSelect,
    handleFormatScore,
    handleShiftSelectionOctave,
    importingFile,
    handleImportFile,
    editPartsOpen,
    setEditPartsOpen,
    editMetadataOpen,
    setEditMetadataOpen,
    storageSettingsOpen,
    setStorageSettingsOpen,
    binOpen,
    setBinOpen,
    handlePartDeclarationChange,
    handleShiftPartOctave,
    parsedMetadata,
    handleMetadataFieldChange,
    setSelectedLineRange,
    handleSectionJump,
    sectionJumpToolbarProps,
    handleNoteRangeSelect,
    handleEditorSelectionChange,
    selectedNoteRangePlaybackInfo,
    selectedNoteCells,
    handleLyricRangeSelect,
    handleLyricEditorSelectionChange,
    selectedLyricCells,
    handleMeasureRangeSelect,
    handlePlayNoteSelection,
    noPartsSelected,
  } = useAppController()

  useKeyboardShortcuts({
    measureAudioPlaying,
    measureAudioGenerating,
    soundfontReady,
    selectedMeasureRange,
    selectedSequenceRange,
    playSelectedMeasures,
    playFromCurrentMeasure,
    notePlaybackSelectionActive: selectedNoteRangePlaybackInfo !== null,
    playNoteSelection: handlePlayNoteSelection,
    stopMeasurePlayback,
    forceSave,
  })

  return (
    <div className="app">
      <AssetLoadingBanner
        soundfontStatus={soundfont.status}
        soundfontLoadedBytes={soundfont.loadedBytes}
        soundfontTotalBytes={soundfont.totalBytes}
        fontsStatus={fonts.status}
        fontsLoadedBytes={fonts.loadedBytes}
        fontsTotalBytes={fonts.totalBytes}
        wasmStatus={wasm.status}
        wasmLoadedBytes={wasm.loadedBytes}
        wasmTotalBytes={wasm.totalBytes}
      />
      <AppHeader
        audioAvailable={audioAvailable}
        selectedMeasureRange={selectedMeasureRange}
        selectedSequenceRange={selectedSequenceRange}
        measureAudioGenerating={measureAudioGenerating}
        soundfontReady={soundfontReady}
        measureAudioPlaying={measureAudioPlaying}
        playSelectedMeasures={playSelectedMeasures}
        playFromCurrentMeasure={playFromCurrentMeasure}
        playAll={playAll}
        notePlaybackSelectionActive={selectedNoteRangePlaybackInfo !== null}
        playNoteSelection={handlePlayNoteSelection}
        stopMeasurePlayback={stopMeasurePlayback}
        shortcutLabel={shortcutLabel}
        playFromCurrentMeasureShortcutLabel={
          playFromCurrentMeasureShortcutLabel
        }
        store={store}
        onSelect={handleSelect}
        onCreate={handleCreate}
        onDuplicate={handleDuplicate}
        onRename={handleRename}
        onDelete={handleDelete}
        onOpenStorageSettings={() => setStorageSettingsOpen(true)}
        saveStatus={saveStatus}
        autosaveDeadline={autosaveDeadline}
        creatingFile={creatingFile}
        deletingFileName={deletingFileName}
        duplicatingFile={duplicatingFile}
        renamingFileName={renamingFileName}
        isLoadingGithub={isLoadingGithub}
        onOpenBin={() => setBinOpen(true)}
        hasDocuments={documents.length > 0}
        rendering={rendering}
        audioGenerating={audioGenerating}
        wavUrl={wavUrl}
        onGenerateAudio={generateFullAudio}
        pdfAvailable={pdfAvailable}
        pdfFontsReady={pdfFontsReady}
        pdfExporting={pdfExporting}
        onExportPdf={exportPdf}
        splitPdfExporting={splitPdfExporting}
        onExportSplitPdf={exportSplitPdf}
        midiAvailable={midiAvailable}
        midiExporting={midiExporting}
        onExportMidi={exportMidi}
        splitMidiExporting={splitMidiExporting}
        onExportSplitMidi={exportSplitMidi}
        splitWavExporting={splitWavExporting}
        onExportSplitWav={exportSplitWav}
        mp3Available={mp3Available}
        mp3Exporting={mp3Exporting}
        mp3Url={mp3Url}
        onExportMp3={exportMp3}
        splitMp3Exporting={splitMp3Exporting}
        onExportSplitMp3={exportSplitMp3}
        partsCount={parts.length}
        importing={importingFile}
        onImportFile={handleImportFile}
        syncedShare={syncedShare}
      />
      <AppOverlays
        fileOpError={fileOpError}
        setFileOpError={setFileOpError}
        storageSettingsOpen={storageSettingsOpen}
        setStorageSettingsOpen={setStorageSettingsOpen}
        backend={backend}
        isLoadingGithub={isLoadingGithub}
        preference={preference}
        switchBackend={switchBackend}
        store={store}
        setStore={setStore}
        refreshSaveStatus={refreshSaveStatus}
        selectedMeasureRange={selectedMeasureRange}
        binOpen={binOpen}
        setBinOpen={setBinOpen}
        onRestore={handleRestore}
        restoringFileName={restoringFileName}
        pendingDownload={pendingDownload}
        onConfirmDownload={confirmPendingDownload}
        onCancelDownload={cancelPendingDownload}
      />
      <ExportAudioToast
        open={
          audioGenerating ||
          mp3Exporting ||
          splitWavExporting ||
          splitMp3Exporting
        }
        label={
          audioGenerating
            ? 'Generating WAV…'
            : mp3Exporting
              ? 'Generating MP3…'
              : splitWavExporting
                ? 'Exporting WAV (ZIP)…'
                : 'Exporting MP3 (ZIP)…'
        }
      />
      <SectionJumpToolbar {...sectionJumpToolbarProps} />
      <SequenceJumpToolbar {...sequenceJumpToolbarProps} />
      <PartToggles
        parts={parts}
        disabledParts={disabledParts}
        disabledLyrics={disabledLyrics}
        soloedParts={soloedParts}
        onPartToggle={handlePartToggle}
        onLyricsToggle={handleLyricsToggle}
        onSoloToggle={handleSoloToggle}
      />
      <AppWorkspace
        editorCollapsed={editorCollapsed}
        setEditorCollapsed={setEditorCollapsed}
        hideEditor={sharedPreview !== null || syncedShareViewerActive}
        editorRef={editorRef}
        fileId={fileId}
        source={source}
        handleSourceChange={handleSourceChange}
        handleFormatScore={handleFormatScore}
        handleShiftSelectionOctave={handleShiftSelectionOctave}
        readOnly={readOnly}
        diagnostics={diagnostics}
        diagnosticViewZones={diagnosticViewZones}
        measureSpans={measureSpans}
        setSelectedLineRange={setSelectedLineRange}
        notifySelection={notifySelection}
        setEditPartsOpen={setEditPartsOpen}
        setEditMetadataOpen={setEditMetadataOpen}
        forceSave={forceSave}
        measureAudioPlaying={measureAudioPlaying}
        stopMeasurePlayback={stopMeasurePlayback}
        selectedMeasureRange={selectedMeasureRange}
        measureAudioGenerating={measureAudioGenerating}
        soundfontReady={soundfontReady}
        playSelectedMeasures={playSelectedMeasures}
        notePlaybackSelectionActive={selectedNoteRangePlaybackInfo !== null}
        playNoteSelection={handlePlayNoteSelection}
        editPartsOpen={editPartsOpen}
        partDeclarations={partDeclarations}
        parts={parts}
        handlePartDeclarationChange={handlePartDeclarationChange}
        handleShiftPartOctave={handleShiftPartOctave}
        previewInstrument={previewInstrument}
        previewPercussion={previewPercussion}
        stopPreviewInstrument={stopPreviewInstrument}
        previewAudioPlaying={previewAudioPlaying}
        editMetadataOpen={editMetadataOpen}
        parsedMetadata={parsedMetadata}
        handleMetadataFieldChange={handleMetadataFieldChange}
        documents={documents}
        highlightedDocuments={highlightedDocuments}
        rendering={rendering}
        handleSectionJump={handleSectionJump}
        handleNoteRangeSelect={handleNoteRangeSelect}
        handleEditorSelectionChange={handleEditorSelectionChange}
        selectedNoteCells={selectedNoteCells}
        noteSpans={noteSpans}
        handleLyricRangeSelect={handleLyricRangeSelect}
        handleLyricEditorSelectionChange={handleLyricEditorSelectionChange}
        selectedLyricCells={selectedLyricCells}
        lyricSpans={lyricSpans}
        handleMeasureRangeSelect={handleMeasureRangeSelect}
        audioGenerating={audioGenerating}
        wavUrl={wavUrl}
        wavFilename={wavFilename}
        mp3Exporting={mp3Exporting}
        mp3Url={mp3Url}
        mp3Filename={mp3Filename}
        onRequestAudioDownload={(url, filename) =>
          requestDownload(url, filename, false)
        }
        noteTimings={noteTimings}
        measureAudioNoteTimings={measureAudioNoteTimings}
        measureAudioElement={measureAudioElement}
        noPartsSelected={noPartsSelected}
      />
    </div>
  )
}
