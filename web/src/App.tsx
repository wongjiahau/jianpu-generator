import { useEffect, useRef, useState } from 'react'
import { Editor } from './components/Editor'
import { ErrorPanel } from './components/ErrorPanel'
import { Preview } from './components/Preview'
import { DEFAULT_SOURCE, STORAGE_KEY } from './defaultSource'
import { useJianpuRender } from './hooks/useJianpuRender'
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
  const editorRef = useRef<EditorHandle>(null)
  const { svgs, diagnostics, rendering } = useJianpuRender(source)

  useEffect(() => {
    saveSource(source)
  }, [source])

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
          />
          <ErrorPanel diagnostics={diagnostics} />
        </section>
        <div className="pane-divider" aria-hidden="true" />
        <section className="pane pane--preview">
          <Preview svgs={svgs} rendering={rendering} />
        </section>
      </main>
    </div>
  )
}
