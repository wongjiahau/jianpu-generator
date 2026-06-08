import { useCallback, useEffect, useRef, useState } from 'react'
import { Editor } from './components/Editor'
import { ErrorPanel } from './components/ErrorPanel'
import { PartToggles } from './components/PartToggles'
import { Preview } from './components/Preview'
import { DEFAULT_SOURCE, STORAGE_KEY } from './defaultSource'
import { useJianpuWorker } from './hooks/useJianpuWorker'
import type { EditorHandle } from './types'
import './App.css'

function loadSource(): string {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored != null) return stored
  } catch {
    // private browsing / disabled storage
  }
  return DEFAULT_SOURCE
}

function saveSource(source: string) {
  try {
    localStorage.setItem(STORAGE_KEY, source)
  } catch {
    // ignore
  }
}

export default function App() {
  const [source, setSource] = useState(loadSource)
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

  useEffect(() => {
    saveSource(source)
  }, [source])

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
          <Editor
            ref={editorRef}
            value={source}
            onChange={setSource}
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
