// pnpm test — arayüzdeki her t('…') anahtarının ve Türkçe harfli her metnin İngilizcesi olmalı, yer tutucular eşleşmeli
import assert from 'node:assert/strict'
import { readFileSync, readdirSync } from 'node:fs'
import en from '../app/locales/en.ts'

const dir = new URL('../app/', import.meta.url)
const files = readdirSync(dir, { recursive: true, encoding: 'utf8' })
  .filter(f => /\.(vue|ts)$/.test(f) && !f.startsWith('locales'))
const missing = new Set<string>()
for (const f of files) {
  const src = readFileSync(new URL(f, dir), 'utf8')
    .replace(/^\s*\/\/.*$/gm, '') // satır yorumları
    .replace(/\/\*[\s\S]*?\*\//g, '') // blok yorumlar
    .replace(/<!--[\s\S]*?-->/g, '')
  // ponytail: yalnız tek tırnaklı literaller; t() dışında kalan Türkçesiz düz metinleri yakalamaz
  for (const m of src.matchAll(/'((?:[^'\\\n]|\\.)*)'/g)) {
    const s = m[1]!.replace(/\\'/g, '\'')
    const isKey = new RegExp(`\\bt\\(\\s*$`).test(src.slice(Math.max(0, m.index! - 8), m.index))
    if ((isKey || /[çğıöşüÇĞİÖŞÜ]/.test(s)) && !(s in en) && s !== 'Türkçe') missing.add(`${f}: ${s}`)
  }
}
assert.deepEqual([...missing], [], 'İngilizcesi eksik metinler')

const holes = (s: string) => [...s.matchAll(/\{\w+\}/g)].map(m => m[0]).sort().join()
for (const [k, v] of Object.entries(en)) assert.equal(holes(v), holes(k), `yer tutucu uyuşmuyor: ${k}`)

console.log(`i18n: ok (${Object.keys(en).length} metin)`)
