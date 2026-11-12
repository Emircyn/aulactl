import { t } from './i18n.ts'

// Tuş kodu: 4 bayt, "TTAABBCC" = [tip][b1][b2][b3] (8 hex hane, büyük harf).
//  00 klavye : b1 modifier bitleri (LCtrl 1, LShift 2, LAlt 4, LWin 8, sağdakiler <<4), b2 ikinci HID, b3 ana HID
//  02 tüketici: b2 usage hi, b3 usage lo
//  03 makro  : b1 mod (1 N kez, 2 basılı, 4 başka tuşa kadar), b2 tekrar, b3 yuva
//  07/08/0D  : özel / ışık / Fn — yalnızca görüntülenir
//  00000000  : devre dışı

export const DISABLED = '00000000'
export const FN_CODE = '0D000000'

export type Decoded
  = | { kind: 'disabled' }
    | { kind: 'key', mods: number, key2: number, key: number }
    | { kind: 'media', usage: number }
    | { kind: 'macro', mode: number, count: number, slot: number }
    | { kind: 'special', type: number }

const hex2 = (n: number) => (n & 0xFF).toString(16).toUpperCase().padStart(2, '0')
const pack = (t: number, a: number, b: number, c: number) => hex2(t) + hex2(a) + hex2(b) + hex2(c)

export function decode(code: string): Decoded {
  const b = [0, 2, 4, 6].map(i => Number.parseInt(code.slice(i, i + 2), 16) || 0)
  const [t, b1, b2, b3] = b as [number, number, number, number]
  if (!t && !b1 && !b2 && !b3) return { kind: 'disabled' }
  if (t === 0x00) return { kind: 'key', mods: b1, key2: b2, key: b3 }
  if (t === 0x02) return { kind: 'media', usage: (b2 << 8) | b3 }
  if (t === 0x03) return { kind: 'macro', mode: b1, count: b2, slot: b3 }
  return { kind: 'special', type: t }
}

/** Modifier HID usage'ı (0xE0-0xE7) verilirse modifier bitine çevrilir: LShift → 00020000. */
export function encodeKey(mods: number, key: number, key2 = 0): string {
  if (key >= 0xE0 && key <= 0xE7) {
    mods |= 1 << (key - 0xE0)
    key = 0
  }
  return pack(0x00, mods, key2, key)
}
export const encodeMedia = (usage: number) => pack(0x02, 0, usage >> 8, usage)
export const encodeMacro = (mode: number, count: number, slot: number) => pack(0x03, mode, count, slot)

// [HID usage, etiket, Windows VK, KeyboardEvent.code]
export const HID_KEYS: [number, string, number, string][] = [
  ...'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('').map((c, i): [number, string, number, string] => [0x04 + i, c, c.charCodeAt(0), `Key${c}`]),
  ...'1234567890'.split('').map((c, i): [number, string, number, string] => [0x1E + i, c, c.charCodeAt(0), `Digit${c}`]),
  [0x28, 'Enter', 0x0D, 'Enter'],
  [0x29, 'Esc', 0x1B, 'Escape'],
  [0x2A, 'Backspace', 0x08, 'Backspace'],
  [0x2B, 'Tab', 0x09, 'Tab'],
  [0x2C, 'Space', 0x20, 'Space'],
  [0x2D, '-', 0xBD, 'Minus'],
  [0x2E, '=', 0xBB, 'Equal'],
  [0x2F, '[', 0xDB, 'BracketLeft'],
  [0x30, ']', 0xDD, 'BracketRight'],
  [0x31, '\\', 0xDC, 'Backslash'],
  [0x33, ';', 0xBA, 'Semicolon'],
  [0x34, '\'', 0xDE, 'Quote'],
  [0x35, '`', 0xC0, 'Backquote'],
  [0x36, ',', 0xBC, 'Comma'],
  [0x37, '.', 0xBE, 'Period'],
  [0x38, '/', 0xBF, 'Slash'],
  [0x39, 'CapsLock', 0x14, 'CapsLock'],
  ...Array.from({ length: 12 }, (_, i): [number, string, number, string] => [0x3A + i, `F${i + 1}`, 0x70 + i, `F${i + 1}`]),
  [0x46, 'PrintScreen', 0x2C, 'PrintScreen'],
  [0x47, 'ScrollLock', 0x91, 'ScrollLock'],
  [0x48, 'Pause', 0x13, 'Pause'],
  [0x49, 'Insert', 0x2D, 'Insert'],
  [0x4A, 'Home', 0x24, 'Home'],
  [0x4B, 'PgUp', 0x21, 'PageUp'],
  [0x4C, 'Delete', 0x2E, 'Delete'],
  [0x4D, 'End', 0x23, 'End'],
  [0x4E, 'PgDn', 0x22, 'PageDown'],
  [0x4F, '→', 0x27, 'ArrowRight'],
  [0x50, '←', 0x25, 'ArrowLeft'],
  [0x51, '↓', 0x28, 'ArrowDown'],
  [0x52, '↑', 0x26, 'ArrowUp'],
  [0x53, 'NumLock', 0x90, 'NumLock'],
  [0x54, 'Num /', 0x6F, 'NumpadDivide'],
  [0x55, 'Num *', 0x6A, 'NumpadMultiply'],
  [0x56, 'Num -', 0x6D, 'NumpadSubtract'],
  [0x57, 'Num +', 0x6B, 'NumpadAdd'],
  [0x58, 'Num Enter', 0, 'NumpadEnter'],
  ...Array.from({ length: 9 }, (_, i): [number, string, number, string] => [0x59 + i, `Num ${i + 1}`, 0x61 + i, `Numpad${i + 1}`]),
  [0x62, 'Num 0', 0x60, 'Numpad0'],
  [0x63, 'Num .', 0x6E, 'NumpadDecimal'],
  [0x64, 'ISO \\', 0xE2, 'IntlBackslash'],
  [0x65, t('Menü'), 0x5D, 'ContextMenu'],
  ...Array.from({ length: 12 }, (_, i): [number, string, number, string] => [0x68 + i, `F${i + 13}`, 0x7C + i, `F${i + 13}`]),
  [0xE0, 'LCtrl', 0xA2, 'ControlLeft'],
  [0xE1, 'LShift', 0xA0, 'ShiftLeft'],
  [0xE2, 'LAlt', 0xA4, 'AltLeft'],
  [0xE3, 'LWin', 0x5B, 'MetaLeft'],
  [0xE4, 'RCtrl', 0xA3, 'ControlRight'],
  [0xE5, 'RShift', 0xA1, 'ShiftRight'],
  [0xE6, 'RAlt', 0xA5, 'AltRight'],
  [0xE7, 'RWin', 0x5C, 'MetaRight']
]

const byHid = new Map(HID_KEYS.map(k => [k[0], k[1]]))
const vkToHid = new Map(HID_KEYS.filter(k => k[2]).map(k => [k[2], k[0]]))
vkToHid.set(0xC4, 0xE4) // layout.json K75 (Fn'in sağı) resmi uygulamada vk 0xC4; klavyede sağ Ctrl
export const codeToHid = new Map(HID_KEYS.map(k => [k[3], k[0]]))

export const hidLabel = (usage: number) => byHid.get(usage) ?? `0x${hex2(usage)}`

/** layout.json'daki tuşun fabrika kodu. Fn (vk 0xFA) → 0D000000. Bilinmeyen vk → devre dışı. */
export function defaultCode(vk: number): string {
  if (vk === 0xFA) return FN_CODE
  const hid = vkToHid.get(vk)
  return hid === undefined ? DISABLED : encodeKey(0, hid)
}

// [usage, ad, tuş üstü kısa etiket]
export const MEDIA: [number, string, string][] = [
  [0xCD, 'Oynat / Duraklat', '⏯'],
  [0xB5, 'Sonraki parça', '⏭'],
  [0xB6, 'Önceki parça', '⏮'],
  [0xB7, 'Durdur', '⏹'],
  [0xE9, 'Ses +', 'Ses+'],
  [0xEA, 'Ses −', 'Ses−'],
  [0xE2, 'Sessiz', 'Sus'],
  [0x6F, 'Ekran parlaklığı +', '☀+'],
  [0x70, 'Ekran parlaklığı −', '☀−'],
  [0x192, 'Hesap makinesi', 'Hesap'],
  [0x18A, 'E-posta', 'Posta'],
  [0x223, 'Tarayıcı ana sayfa', 'Web'],
  [0x194, 'Bilgisayarım', 'PC']
]
const mediaName = new Map(MEDIA.map(m => [m[0], m[1]]))
export const mediaShort = new Map(MEDIA.map(m => [m[0], m[2]]))

const MOD_NAMES = ['Ctrl', 'Shift', 'Alt', 'Win']
export const modNames = (mods: number) => MOD_NAMES.filter((_, i) => mods & (0x11 << i))

export const REPEAT_LABEL: Record<number, string> = { 1: 'N kez', 2: 'Basılı tutuldukça', 4: 'Başka tuşa basılana kadar' }

/** İnsan okunur ad: "Ctrl+Shift+Esc", "Ses +", "Makro 2", "Özel 08000102" */
export function describe(code: string, macroNames: string[] = []): string {
  const d = decode(code)
  switch (d.kind) {
    case 'disabled': return t('Devre dışı')
    case 'media': return t(mediaName.get(d.usage) ?? 'Medya 0x{u}', { u: d.usage.toString(16).toUpperCase() })
    case 'macro': return macroNames[d.slot] ?? t('Makro {n}', { n: d.slot + 1 })
    case 'special': return d.type === 0x0D ? 'Fn' : t('Özel {code}', { code })
    case 'key': {
      if (!d.key && !d.key2) {
        // yalnızca modifier: yanını da söyle
        return HID_KEYS.filter(k => k[0] >= 0xE0 && d.mods & (1 << (k[0] - 0xE0))).map(k => k[1]).join('+')
      }
      return [...modNames(d.mods), d.key2 ? hidLabel(d.key2) : '', hidLabel(d.key)].filter(Boolean).join('+')
    }
  }
}
