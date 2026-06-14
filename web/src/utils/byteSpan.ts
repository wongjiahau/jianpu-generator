const utf8LenCache = new Map<string, number>()
const utf8Encoder = new TextEncoder()

function utf8ByteLengthOfCodePoint(ch: string): number {
  const cached = utf8LenCache.get(ch)
  if (cached != null) return cached
  const len = utf8Encoder.encode(ch).length
  utf8LenCache.set(ch, len)
  return len
}

/**
 * Map a JS string index (UTF-16 code unit count from Monaco) to a UTF-8 byte offset
 * suitable for passing to the Rust parser.
 */
export function stringIndexToByteOffset(
  source: string,
  charIndex: number,
): number {
  return utf8Encoder.encode(source.slice(0, charIndex)).length
}

/**
 * Map a UTF-8 byte offset (from the Rust parser) to a JS string index (UTF-16).
 */
export function byteOffsetToStringIndex(
  source: string,
  byteOffset: number,
): number {
  let bytePos = 0
  let strIndex = 0

  for (const ch of source) {
    const cpByteLen = utf8ByteLengthOfCodePoint(ch)
    const cpEndByte = bytePos + cpByteLen
    if (cpEndByte <= byteOffset) {
      strIndex += ch.length
      bytePos = cpEndByte
      continue
    }
    break
  }

  return strIndex
}
