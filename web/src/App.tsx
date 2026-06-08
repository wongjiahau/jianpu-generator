import { useCallback, useEffect, useRef, useState } from 'react'
import { Editor } from './components/Editor'
import { ErrorPanel } from './components/ErrorPanel'
import { FileList } from './components/FileList'
import { PartToggles } from './components/PartToggles'
import { Preview } from './components/Preview'
import {
  createFile,
  deleteFile,
  duplicateFile,
  fileContent,
  fileIdForName,
  isReadOnlyFile,
  renameFile,
  restoreFile,
  selectFile,
  updateActiveContent,
} from './fileStore'
import { useFileStore } from './hooks/useFileStore'
import { useJianpuWorker } from './hooks/useJianpuWorker'
import {
  readPartTogglesForFile,
  writePartTogglesForFile,
} from './partToggleCache'
import type { EditorHandle } from './types'
import './App.css'

export default function App() {
  const [store, setStore] = useFileStore()
  const source = fileContent(store, store.active)
  const readOnly = isReadOnlyFile(store.active)
  const fileId = fileIdForName(store, store.active)

  const [disabledParts, setDisabledParts] = useState<Set<string>>(() => {
    const cached = readPartTogglesForFile(fileId)
    return new Set(cached?.disabledParts ?? [])
  })
  const [disabledLyrics, setDisabledLyrics] = useState<Set<string>>(() => {
    const cached = readPartTogglesForFile(fileId)
    return new Set(cached?.disabledLyrics ?? [])
  })
  const editorRef = useRef<EditorHandle>(null)
  const skipToggleSaveRef = useRef(false)
  const {
    parts,
    partsLoading,
    svgs,
    wavUrl,
    audioAvailable,
    pdfAvailable,
    pdfExporting,
    diagnostics,
    rendering,
    exportPdf,
    splitPdfExporting,
    exportSplitPdf,
  } = useJianpuWorker(source, disabledParts, disabledLyrics, store.active)

  useEffect(() => {
    skipToggleSaveRef.current = true
    const cached = readPartTogglesForFile(fileId)
    setDisabledParts(new Set(cached?.disabledParts ?? []))
    setDisabledLyrics(new Set(cached?.disabledLyrics ?? []))
  }, [fileId])

  useEffect(() => {
    if (skipToggleSaveRef.current) {
      skipToggleSaveRef.current = false
      return
    }
    writePartTogglesForFile(fileId, {
      disabledParts: [...disabledParts],
      disabledLyrics: [...disabledLyrics],
    })
  }, [fileId, disabledParts, disabledLyrics])

  useEffect(() => {
    if (parts.length === 0) return

    const abbreviations = new Set(parts.map((part) => part.abbreviation))
    setDisabledParts((prev) => {
      const next = new Set(
        [...prev].filter((abbreviation) => abbreviations.has(abbreviation)),
      )
      return next.size === prev.size ? prev : next
    })
  }, [parts])

  useEffect(() => {
    if (parts.length === 0) return

    const lyricAbbreviations = new Set(
      parts.filter((part) => part.has_lyrics).map((part) => part.abbreviation),
    )
    setDisabledLyrics((prev) => {
      const next = new Set(
        [...prev].filter((abbreviation) =>
          lyricAbbreviations.has(abbreviation),
        ),
      )
      return next.size === prev.size ? prev : next
    })
  }, [parts])

  const handlePartToggle = useCallback(
    (abbreviation: string, enabled: boolean) => {
      setDisabledParts((prev) => {
        const next = new Set(prev)
        if (enabled) {
          next.delete(abbreviation)
        } else {
          next.add(abbreviation)
        }
        return next
      })
    },
    [],
  )

  const handleLyricsToggle = useCallback(
    (abbreviation: string, enabled: boolean) => {
      setDisabledLyrics((prev) => {
        const next = new Set(prev)
        if (enabled) {
          next.delete(abbreviation)
        } else {
          next.add(abbreviation)
        }
        return next
      })
    },
    [],
  )

  const handleSourceChange = useCallback((value: string) => {
    setStore((prev) => updateActiveContent(prev, value))
  }, [])

  const handleSelect = useCallback((name: string) => {
    setStore((prev) => selectFile(prev, name))
  }, [])

  const handleCreate = useCallback(() => {
    setStore((prev) => createFile(prev))
  }, [])

  const handleDuplicate = useCallback(() => {
    setStore((prev) => duplicateFile(prev))
  }, [])

  const handleRename = useCallback((from: string, to: string) => {
    setStore((prev) => renameFile(prev, from, to))
  }, [])

  const handleDelete = useCallback((name: string) => {
    setStore((prev) => deleteFile(prev, name))
  }, [])

  const handleRestore = useCallback((name: string) => {
    setStore((prev) => restoreFile(prev, name))
  }, [])

  const noPartsSelected =
    parts.length > 0 &&
    parts.every((part) => disabledParts.has(part.abbreviation))

  return (
    <div className="app">
      <header className="app-header">
        <h1>簡譜</h1>
        <span className="app-subtitle">live preview</span>
      </header>
      <main className="workspace">
        <section className="pane pane--editor">
          <div className="editor-layout">
            <FileList
              store={store}
              onSelect={handleSelect}
              onCreate={handleCreate}
              onDuplicate={handleDuplicate}
              onRename={handleRename}
              onDelete={handleDelete}
              onRestore={handleRestore}
            />
            <div className="editor-main">
              <Editor
                ref={editorRef}
                value={source}
                onChange={handleSourceChange}
                readOnly={readOnly}
                diagnostics={diagnostics}
                toolbar={
                  <PartToggles
                    parts={parts}
                    disabledParts={disabledParts}
                    disabledLyrics={disabledLyrics}
                    onPartToggle={handlePartToggle}
                    onLyricsToggle={handleLyricsToggle}
                    loading={partsLoading}
                  />
                }
              />
              <ErrorPanel diagnostics={diagnostics} />
            </div>
          </div>
        </section>
        <div className="pane-divider" aria-hidden="true" />
        <section className="pane pane--preview">
          <Preview
            svgs={svgs}
            rendering={rendering}
            wavUrl={wavUrl}
            audioAvailable={audioAvailable}
            pdfAvailable={pdfAvailable}
            pdfExporting={pdfExporting}
            onExportPdf={exportPdf}
            splitPdfExporting={splitPdfExporting}
            onExportSplitPdf={exportSplitPdf}
            partsCount={parts.length}
            emptyMessage={
              noPartsSelected ? 'No parts selected.' : 'No preview yet.'
            }
          />
        </section>
      </main>
    </div>
  )
}
