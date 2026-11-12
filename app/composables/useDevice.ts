// Ön yüz ↔ cihaz köprüsü. Tauri içinde invoke/listen; tarayıcıda (pnpm dev) sahte cihaz.
import type { DeviceConfig, DeviceStatus, Macro, Prefs, Rgb } from '~/types/device'
import layout from '~~/layout.json'

export type Section = 'keymap' | 'led' | 'palette' | 'perkey' | 'macros'
export const SECTION_LABEL: Record<Section, string> = {
  keymap: t('Tuş atamaları'),
  led: t('Işık ayarları'),
  palette: t('Efekt paletleri'),
  perkey: t('Tuş renkleri'),
  macros: t('Makrolar')
}

export const isTauri = () => import.meta.client && '__TAURI_INTERNALS__' in window

const OFFLINE: DeviceStatus = { connected: false, transport: null, online: false, fw: null, battery: null, charging: null }

// ---------------- sahte cihaz ----------------
const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v)) // reactive proxy de klonlanır
const PALETTE: Rgb[] = [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 0], [160, 0, 255], [0, 255, 255], [255, 255, 255]]

function mockConfig(): DeviceConfig {
  const base = Array<string>(layout.matrixLen).fill(DISABLED)
  for (const k of layout.keys) base[k.index] = defaultCode(k.vk)
  const fn = [...base]
  const fnRow: Record<string, string> = {
    F1: '08010000', F2: '08020000', F3: '08030000', F4: '08040000',
    F5: encodeMedia(0xB6), F6: encodeMedia(0xCD), F7: encodeMedia(0xB5), F8: encodeMedia(0xE2),
    F9: encodeMedia(0xEA), F10: encodeMedia(0xE9), F11: '07000001', F12: '07000002'
  }
  for (const k of layout.keys) if (fnRow[k.name]) fn[k.index] = fnRow[k.name]!
  const perkey = Array.from({ length: 126 }, (): Rgb => [0, 0, 0])
  for (const k of layout.keys) perkey[k.index] = hsv((k.rect[0]! / 679) * 300)
  const macros: Macro[] = [
    { name: 'Kopyala', repeat: 1, count: 1, steps: [
      { action: 'down', key: 0xE0, delay: 20 }, { action: 'down', key: 0x06, delay: 20 },
      { action: 'up', key: 0x06, delay: 20 }, { action: 'up', key: 0xE0, delay: 0 }
    ] },
    { name: 'gg yaz', repeat: 1, count: 1, steps: [
      { action: 'down', key: 0x0A, delay: 30 }, { action: 'up', key: 0x0A, delay: 30 },
      { action: 'down', key: 0x0A, delay: 30 }, { action: 'up', key: 0x0A, delay: 30 },
      { action: 'down', key: 0x28, delay: 30 }, { action: 'up', key: 0x28, delay: 0 }
    ] }
  ]
  return {
    layers: [base, fn, [...base], [...base]],
    led: {
      mode: 13,
      custom: false,
      sleep: 300,
      debounce: 1,
      modes: layout.effects.map((e, i) => ({ brightness: 9, speed: e.speed ? 2 + (i % 3) : 0, color: e.color ? 7 : 0 }))
    },
    palette: layout.effects.map(() => PALETTE.map(c => [...c] as Rgb)),
    perkey: [perkey],
    macros
  }
}

const mock = {
  status: { connected: true, transport: 'dongle', online: true, fw: '1.0.4', battery: 62, charging: false } as DeviceStatus,
  config: null as DeviceConfig | null,
  prefs: { lang, caps_light: false, caps_color: [255, 255, 255], battery_notify: true, battery_flash: true, music: false, music_gain: 1, music_smooth: 4, music_color: [0, 255, 0], music_rainbow: true } as Prefs,
  profiles: new Map<string, DeviceConfig>()
}

async function mockCall(cmd: string, args: Record<string, unknown> = {}): Promise<unknown> {
  await new Promise(r => setTimeout(r, 150))
  if (cmd === 'get_prefs') return clone(mock.prefs)
  if (cmd === 'set_prefs') return void (mock.prefs = clone(args.prefs as Prefs))
  if (cmd === 'autostart_get') return true
  if (cmd === 'autostart_set') return
  if (cmd === 'firmware_check') return { current: 0x1100, latest: 0x1100, newer: false }
  if (cmd === 'firmware_update') throw new Error('Keyboard is already on the latest firmware')
  if (cmd === 'profile_list') return [...mock.profiles.keys()].sort()
  if (cmd === 'profile_delete') return void mock.profiles.delete(args.name as string)
  mock.config ??= mockConfig()
  const c = mock.config
  if (!mock.status.connected && cmd !== 'device_status') throw new Error('Keyboard not connected')
  if (cmd === 'device_status') return clone(mock.status)
  if (cmd === 'read_config') return clone(c)
  if (cmd === 'profile_save') return void mock.profiles.set(args.name as string, clone(c))
  if (cmd === 'profile_apply') return void (mock.config = clone(mock.profiles.get(args.name as string)!))
  if (cmd === 'backup') return `~/.config/aulactl/backups/${new Date().toISOString().replace(/[:.]/g, '-')}.json`
  if (cmd === 'write_keymap') c.layers[args.layer as number] = clone(args.codes as string[])
  else if (cmd === 'write_perkey') c.perkey[args.group as number] = clone(args.colors as Rgb[])
  else if (['write_led', 'write_palette', 'write_macros'].includes(cmd)) Object.assign(c, clone(args))
  else if (cmd === 'factory_reset') mock.config = mockConfig()
  else if (cmd !== 'restore') throw new Error(`Unknown command: ${cmd}`)
  return null
}

/** Sahte cihazda bağlantıyı döndürür: 2.4G → Kablolu → Bluetooth → bağlı değil (durum çubuğundan). */
export function mockCycleTransport() {
  const s = mock.status
  if (s.transport === 'dongle') Object.assign(s, { connected: true, transport: 'wired', online: true, battery: 64, charging: true })
  else if (s.transport === 'wired') Object.assign(s, { ...OFFLINE, connected: true, transport: 'bluetooth', online: true })
  else if (s.transport === 'bluetooth') Object.assign(s, OFFLINE)
  else Object.assign(s, { connected: true, transport: 'dongle', online: true, fw: '1.0.4', battery: 62, charging: false })
}

// ---------------- köprü ----------------
export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) return mockCall(cmd, args) as Promise<T>
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<T>(cmd, args)
}

function pick(c: DeviceConfig, s: Section) {
  return s === 'keymap' ? c.layers : c[s]
}

export function useDevice() {
  const status = useState<DeviceStatus>('dev-status', () => ({ ...OFFLINE }))
  const config = useState<DeviceConfig | null>('dev-config', () => null)
  const saved = useState<Partial<Record<Section, string>>>('dev-saved', () => ({}))
  const busy = useState<boolean>('dev-busy', () => false)
  const toast = useToast()

  const dirty = (s: Section) => !!config.value && JSON.stringify(pick(config.value, s)) !== saved.value[s]
  const anyDirty = computed(() => (Object.keys(SECTION_LABEL) as Section[]).filter(dirty))

  function fail(title: string, e: unknown) {
    toast.add({ title, description: String(e), color: 'error', icon: 'i-lucide-triangle-alert' })
  }

  async function refreshStatus() {
    try {
      status.value = await call<DeviceStatus>('device_status')
    } catch (e) {
      status.value = { ...OFFLINE }
      fail(t('Durum okunamadı'), e)
    }
  }

  async function loadConfig() {
    if (!status.value.connected || status.value.transport === 'bluetooth') {
      config.value = null
      return
    }
    busy.value = true
    try {
      const c = await call<DeviceConfig>('read_config')
      config.value = c
      saved.value = Object.fromEntries((Object.keys(SECTION_LABEL) as Section[]).map(s => [s, JSON.stringify(pick(c, s))]))
    } catch (e) {
      fail(t('Ayarlar okunamadı'), e)
    } finally {
      busy.value = false
    }
  }

  async function apply(s: Section) {
    const c = config.value
    if (!c) return
    busy.value = true
    try {
      if (s === 'keymap') {
        const old = JSON.parse(saved.value.keymap ?? '[]') as string[][]
        for (const [layer, codes] of c.layers.entries()) {
          if (JSON.stringify(codes) !== JSON.stringify(old[layer])) await call('write_keymap', { layer, codes })
        }
      } else if (s === 'perkey') {
        const old = JSON.parse(saved.value.perkey ?? '[]') as Rgb[][]
        for (const [group, colors] of c.perkey.entries()) {
          if (JSON.stringify(colors) !== JSON.stringify(old[group])) await call('write_perkey', { group, colors })
        }
      } else {
        await call(`write_${s}`, { [s]: c[s] })
      }
      saved.value = { ...saved.value, [s]: JSON.stringify(pick(c, s)) }
      toast.add({ title: t('{section} klavyeye yazıldı', { section: SECTION_LABEL[s] }), color: 'success', icon: 'i-lucide-check' })
    } catch (e) {
      fail(t('{section} yazılamadı', { section: SECTION_LABEL[s] }), e)
    } finally {
      busy.value = false
    }
  }

  function revert(s: Section) {
    const c = config.value
    const v = saved.value[s]
    if (!c || !v) return
    if (s === 'keymap') c.layers = JSON.parse(v)
    else (c as unknown as Record<string, unknown>)[s] = JSON.parse(v)
  }

  /** backup/restore/factory_reset: hata toast olur (undefined döner), başarıda { value }. */
  async function run<T>(cmd: 'backup' | 'restore' | 'factory_reset', args?: Record<string, unknown>) {
    busy.value = true
    try {
      const r = await call<T>(cmd, args)
      if (cmd !== 'backup') await loadConfig()
      return { value: r }
    } catch (e) {
      fail(t('İşlem başarısız'), e)
    } finally {
      busy.value = false
    }
  }

  return { status, config, busy, dirty, anyDirty, refreshStatus, loadConfig, apply, revert, run }
}

/** Uygulama açılışında bir kez: durum + ayarlar + olay dinleyicileri. */
export async function initDevice() {
  const { status, config, busy, refreshStatus, loadConfig } = useDevice()
  await refreshStatus()
  await loadConfig()
  if (!isTauri()) return
  const { listen } = await import('@tauri-apps/api/event')
  // yük yalnızca takılı/çıkarıldı bilgisi; tam durum ve ayarlar yeniden okunur
  await listen('device-changed', async () => {
    await refreshStatus()
    if (status.value?.connected) await loadConfig()
  })
  // online/pil/FN olayları: yükü yorumlamak yerine durumu yeniden oku
  // uykudaki klavye uyanınca (pil/bağlantı olayı gelir) okunamamış ayarları yeniden dene
  const tick = async () => {
    await refreshStatus()
    if (!config.value && !busy.value && status.value.connected) await loadConfig()
  }
  await listen('device-event', tick)
  // klavye pil olayını her zaman göndermiyor (cihazda görüldü) → pil ve okunamamış ayarlar için dakikada bir
  setInterval(() => {
    if (!busy.value) tick()
  }, 60_000)
}
