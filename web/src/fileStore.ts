import { DEFAULT_SOURCE, DEMO_FILE_NAME } from './defaultSource'

export { DEMO_FILE_NAME }

export const FILE_STORE_KEY = 'jianpu:files:v1'
const STORAGE_KEY = 'jianpu:source:v5'

const DEFAULT_FILE_STORE: FileStoreState = {
  active: DEMO_FILE_NAME,
  userFiles: {},
  bin: {},
}

const NEW_FILE_TEMPLATE = `[metadata]
title = "Untitled"

[parts]

[score]
`

export interface FileStoreState {
  active: string
  userFiles: Record<string, string>
  bin: Record<string, string>
}

function isDemoFile(name: string): boolean {
  return name === DEMO_FILE_NAME
}

export function fileContent(state: FileStoreState, name: string): string {
  if (isDemoFile(name)) return DEFAULT_SOURCE
  return state.userFiles[name] ?? ''
}

export function sortedFileNames(state: FileStoreState): string[] {
  const names = new Set<string>([DEMO_FILE_NAME, ...Object.keys(state.userFiles)])
  return [...names].sort((a, b) => a.localeCompare(b))
}

export function sortedBinNames(state: FileStoreState): string[] {
  return Object.keys(state.bin).sort((a, b) => a.localeCompare(b))
}

function reservedNames(state: FileStoreState): Set<string> {
  return new Set([
    DEMO_FILE_NAME,
    ...Object.keys(state.userFiles),
    ...Object.keys(state.bin),
  ])
}

function uniqueName(base: string, taken: Set<string>): string {
  if (!taken.has(base)) return base
  const dot = base.lastIndexOf('.')
  const stem = dot > 0 ? base.slice(0, dot) : base
  const ext = dot > 0 ? base.slice(dot) : '.jianpu'
  let n = 2
  while (taken.has(`${stem} ${n}${ext}`)) n++
  return `${stem} ${n}${ext}`
}

function sanitizeFileName(raw: string): string {
  const trimmed = raw.trim()
  if (!trimmed) return 'untitled.jianpu'
  return trimmed.endsWith('.jianpu') ? trimmed : `${trimmed}.jianpu`
}

function normalizeState(parsed: Partial<FileStoreState>): FileStoreState {
  const userFiles = { ...parsed.userFiles }
  delete userFiles[DEMO_FILE_NAME]
  const bin = { ...parsed.bin }
  delete bin[DEMO_FILE_NAME]

  const state: FileStoreState = {
    active: parsed.active ?? DEMO_FILE_NAME,
    userFiles,
    bin,
  }
  const names = sortedFileNames(state)
  return {
    ...state,
    active: names.includes(state.active) ? state.active : DEMO_FILE_NAME,
  }
}

function parseStoredFileStore(raw: string): FileStoreState | null {
  try {
    const parsed = JSON.parse(raw) as Partial<FileStoreState>
    if (parsed && typeof parsed.active === 'string' && parsed.userFiles) {
      return normalizeState({
        ...parsed,
        bin: parsed.bin ?? {},
      })
    }
  } catch {
    // ignore corrupt storage
  }
  return null
}

function readLegacyFileStore(): FileStoreState | null {
  try {
    const legacy = localStorage.getItem(STORAGE_KEY)
    if (legacy != null) {
      return {
        active: 'untitled.jianpu',
        userFiles: { 'untitled.jianpu': legacy },
        bin: {},
      }
    }
  } catch {
    // ignore
  }
  return null
}

export function readInitialFileStore(): FileStoreState {
  try {
    const raw = localStorage.getItem(FILE_STORE_KEY)
    if (raw != null) {
      const parsed = parseStoredFileStore(raw)
      if (parsed) return parsed
    }
  } catch {
    // ignore
  }

  return readLegacyFileStore() ?? DEFAULT_FILE_STORE
}

export function deserializeFileStore(raw: string): FileStoreState {
  return parseStoredFileStore(raw) ?? readLegacyFileStore() ?? DEFAULT_FILE_STORE
}

export function updateActiveContent(
  state: FileStoreState,
  content: string,
): FileStoreState {
  if (isDemoFile(state.active)) return state
  return {
    ...state,
    userFiles: { ...state.userFiles, [state.active]: content },
  }
}

export function selectFile(state: FileStoreState, name: string): FileStoreState {
  const names = sortedFileNames(state)
  if (!names.includes(name)) return state
  return { ...state, active: name }
}

export function createFile(state: FileStoreState): FileStoreState {
  const taken = reservedNames(state)
  const name = uniqueName('untitled.jianpu', taken)
  return {
    ...state,
    active: name,
    userFiles: { ...state.userFiles, [name]: NEW_FILE_TEMPLATE },
  }
}

export function duplicateFile(state: FileStoreState): FileStoreState {
  const source = state.active
  const taken = reservedNames(state)
  const name = uniqueName(source, taken)
  const content = fileContent(state, source)
  return {
    ...state,
    active: name,
    userFiles: { ...state.userFiles, [name]: content },
  }
}

export function renameFile(
  state: FileStoreState,
  from: string,
  toRaw: string,
): FileStoreState {
  if (isDemoFile(from)) return state
  const to = sanitizeFileName(toRaw)
  if (to === from) return state
  const names = sortedFileNames(state)
  if (!names.includes(from) || isDemoFile(to)) return state
  if (reservedNames(state).has(to) && to !== from) return state

  const { [from]: content, ...rest } = state.userFiles
  if (content === undefined) return state

  return {
    ...state,
    active: state.active === from ? to : state.active,
    userFiles: { ...rest, [to]: content },
  }
}

export function deleteFile(state: FileStoreState, name: string): FileStoreState {
  if (isDemoFile(name)) return state
  const content = state.userFiles[name]
  if (content === undefined) return state

  const { [name]: _, ...rest } = state.userFiles
  const remaining = sortedFileNames({ ...state, userFiles: rest })
  const nextActive =
    state.active === name ? (remaining[0] ?? DEMO_FILE_NAME) : state.active

  return {
    active: nextActive,
    userFiles: rest,
    bin: { ...state.bin, [name]: content },
  }
}

export function restoreFile(state: FileStoreState, name: string): FileStoreState {
  const content = state.bin[name]
  if (content === undefined) return state

  const activeNames = new Set(sortedFileNames(state))
  const restoreName = activeNames.has(name)
    ? uniqueName(name, activeNames)
    : name

  const { [name]: _, ...restBin } = state.bin
  return {
    active: restoreName,
    userFiles: { ...state.userFiles, [restoreName]: content },
    bin: restBin,
  }
}

export function isReadOnlyFile(name: string): boolean {
  return isDemoFile(name)
}
