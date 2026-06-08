import { useCallback, useEffect, useRef, useState } from 'react'
import { useFileStore } from './hooks/useFileStore'
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
  isReadOnlyFile,
  renameFile,
  restoreFile,
  selectFile,
  updateActiveContent,
} from './fileStore'
import { useJianpuWorker } from './hooks/useJianpuWorker'
import type { EditorHandle } from './types'
import './App.css'

export default function App() {
  const [store, setStore] = useFileStore()
  const source = fileContent(store, store.active)
  const readOnly = isReadOnlyFile(store.active)

  const [disabledParts, setDisabledParts] = useState<Set<string>>(
    () => new Set(),
  )
  const editorRef = useRef<EditorHandle>(null)
  const { parts, partsLoading, svgs, diagnostics, rendering } = useJianpuWorker(
    source,
    disabledParts,
  )

  useEffect(() => {
    const abbreviations = new Set(parts.map((part) => part.abbreviation))
    setDisabledParts((prev) => {
      const next = new Set(
        [...prev].filter((abbreviation) => abbreviations.has(abbreviation)),
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
        <h1>Jianpu</h1>
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
                    onToggle={handlePartToggle}
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
            emptyMessage={
              noPartsSelected ? 'No parts selected.' : 'No preview yet.'
            }
          />
        </section>
      </main>
    </div>
  )
}
