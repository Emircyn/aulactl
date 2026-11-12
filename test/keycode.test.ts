// pnpm test — tuş kodu kodlama/çözme ve VK→HID varsayılanları
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { decode, defaultCode, describe, encodeKey, encodeMacro, encodeMedia } from '../app/utils/keycode.ts'

assert.equal(defaultCode(0x1B), '00000029') // Esc
assert.equal(encodeKey(0x04, 0x2B), '0004002B') // Alt+Tab
assert.equal(defaultCode(0xA0), '00020000') // LShift tek başına
assert.equal(defaultCode(0xFA), '0D000000') // Fn
assert.equal(encodeMedia(0xCD), '020000CD')
assert.equal(encodeMedia(0x192), '02000192')
assert.equal(encodeMacro(1, 3, 2), '03010302')
assert.deepEqual(decode('0004002B'), { kind: 'key', mods: 4, key2: 0, key: 0x2B })
assert.deepEqual(decode('020000E9'), { kind: 'media', usage: 0xE9 })
assert.deepEqual(decode('00000000'), { kind: 'disabled' })
assert.equal(describe('0004002B'), 'Alt+Tab')
assert.equal(describe('00020000'), 'LShift')
assert.equal(describe('020000EA'), 'Volume −') // node: localStorage yok → en
assert.equal(describe('08000102'), 'Special 08000102')
assert.equal(describe('03010300', ['Kopyala']), 'Kopyala')

// layout.json'daki her tuşun bir varsayılan kodu olmalı
const layout = JSON.parse(readFileSync(new URL('../layout.json', import.meta.url), 'utf8'))
for (const k of layout.keys) assert.notEqual(defaultCode(k.vk), '00000000', `${k.name} vk ${k.vk}`)
assert.equal(new Set(layout.keys.map((k: { vk: number }) => defaultCode(k.vk))).size, layout.keys.length)

console.log('keycode: ok')
