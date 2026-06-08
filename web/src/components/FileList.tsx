import { useEffect, useState } from 'react'
import {
  DEMO_FILE_NAME,
  isReadOnlyFile,
  sortedBinNames,
  sortedFileNames,
  type FileStoreState,
} from '../fileStore'

export interface FileListProps {
  store: FileStoreState
  onSelect: (name: string) => void
  onCreate: () => void
  onDuplicate: () => void
  onRename: (from: string, to: string) => void
  onDelete: (name: string) => void
  onRestore: (name: string) => void
}

function FileNameField({
  name,
  active,
  onSelect,
  onRename,
}: {
  name: string
  active: boolean
  onSelect: (name: string) => void
  onRename: (from: string, to: string) => void
}) {
  const readOnly = isReadOnlyFile(name)
  const [draft, setDraft] = useState(name)

  useEffect(() => {
    setDraft(name)
  }, [name])

  return (
    <input
      type="text"
      className="file-list-name"
      value={draft}
      readOnly={readOnly}
      aria-current={active ? 'true' : undefined}
      onFocus={() => {
        if (!active) onSelect(name)
      }}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={() => {
        if (readOnly) return
        const trimmed = draft.trim()
        if (trimmed && trimmed !== name) {
          onRename(name, trimmed)
        } else {
          setDraft(name)
        }
      }}
      onKeyDown={(e) => {
        if (e.key === 'Enter') {
          e.currentTarget.blur()
        } else if (e.key === 'Escape') {
          setDraft(name)
          e.currentTarget.blur()
        }
      }}
    />
  )
}

export function FileList({
  store,
  onSelect,
  onCreate,
  onDuplicate,
  onRename,
  onDelete,
  onRestore,
}: FileListProps) {
  const names = sortedFileNames(store)
  const binNames = sortedBinNames(store)

  return (
    <aside className="file-list" aria-label="Files">
      <div className="file-list-actions">
        <button type="button" className="file-list-btn" onClick={onCreate}>
          New
        </button>
        <button type="button" className="file-list-btn" onClick={onDuplicate}>
          Duplicate
        </button>
      </div>
      <ul className="file-list-items">
        {names.map((name) => {
          const active = name === store.active
          const readOnly = isReadOnlyFile(name)

          return (
            <li
              key={name}
              className={`file-list-item${active ? ' file-list-item--active' : ''}`}
            >
              <FileNameField
                name={name}
                active={active}
                onSelect={onSelect}
                onRename={onRename}
              />
              {!readOnly ? (
                <button
                  type="button"
                  className="file-list-delete"
                  aria-label={`Move ${name} to bin`}
                  onClick={() => onDelete(name)}
                >
                  ×
                </button>
              ) : null}
            </li>
          )
        })}
      </ul>
      {names.length === 1 && names[0] === DEMO_FILE_NAME ? (
        <p className="file-list-hint">Demo is read-only — duplicate to edit.</p>
      ) : null}
      {binNames.length > 0 ? (
        <section className="file-list-bin" aria-label="Bin">
          <h2 className="file-list-bin-title">Bin</h2>
          <ul className="file-list-bin-items">
            {binNames.map((name) => (
              <li key={name} className="file-list-bin-item">
                <span className="file-list-bin-name">{name}</span>
                <button
                  type="button"
                  className="file-list-restore"
                  aria-label={`Restore ${name}`}
                  onClick={() => onRestore(name)}
                >
                  ↩
                </button>
              </li>
            ))}
          </ul>
        </section>
      ) : null}
    </aside>
  )
}
