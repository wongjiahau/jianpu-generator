export const PART_TOGGLES_KEY = 'jianpu:part-toggles:v1'

export interface PartToggleState {
  disabledParts: string[]
  disabledLyrics: string[]
}

type PartToggleCache = Record<string, PartToggleState>

function readCache(): PartToggleCache {
  try {
    const raw = localStorage.getItem(PART_TOGGLES_KEY)
    if (raw != null) {
      const parsed = JSON.parse(raw) as PartToggleCache
      if (parsed && typeof parsed === 'object') return parsed
    }
  } catch {
    // ignore corrupt storage
  }
  return {}
}

export function readPartTogglesForFile(fileId: string): PartToggleState | null {
  return readCache()[fileId] ?? null
}

export function writePartTogglesForFile(
  fileId: string,
  state: PartToggleState,
): void {
  try {
    const cache = readCache()
    cache[fileId] = state
    localStorage.setItem(PART_TOGGLES_KEY, JSON.stringify(cache))
  } catch {
    // ignore quota errors
  }
}
