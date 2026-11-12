// Rust tarafında birebir aynası olmalı (serde, camelCase alan adı yok — hepsi tek kelime).

/** [r, g, b], her biri 0-255 */
export type Rgb = [number, number, number]

/** 4 baytlık tuş kodu, 8 hex hane: "TTAABBCC" = [tip][b1][b2][b3]. Bkz. app/utils/keycode.ts */
export type KeyCode = string

export interface DeviceStatus {
  connected: boolean
  /** bluetooth: yalnız gösterilir, bu modda ayar yazılamaz */
  transport: 'wired' | 'dongle' | 'bluetooth' | null
  /** alıcıda klavye çevrimiçi mi (kabloluda hep true) */
  online: boolean
  fw: string | null
  /** yüzde; bilinmiyorsa null */
  battery: number | null
  charging: boolean | null
}

export interface LedMode {
  /** 0-9 */
  brightness: number
  /** 0-4 */
  speed: number
  /** 0-6 palet indeksi, 7 = rastgele */
  color: number
}

export interface Led {
  /** etkin efektin hw kimliği (layout.json effects[].hw); 0 = Kapalı */
  mode: number
  /** LED bloğu 0x09: true = Self-define (tuş başına renk) etkin, mode yok sayılır */
  custom: boolean
  /** saniye (layout.json sleepSeconds), 0 = uyku kapalı */
  sleep: number
  /** LED bloğu 0x03: tuş sekme önleme, cihaz değeri 0-4 (ekranda +1 kademe) */
  debounce: number
  /** effects sırasıyla (ui 1..18 → indeks 0..17) */
  modes: LedMode[]
}

/** effects sırasıyla, efekt başına 7 renk */
export type Palette = Rgb[][]

export type MacroRepeat = 1 | 2 | 4 // 1 = N kez, 2 = basılı tutuldukça, 4 = başka tuşa kadar

export interface MacroStep {
  action: 'down' | 'up'
  /** HID usage (0xE0-0xE7 = modifier tuşları) */
  key: number
  /** bu adımdan sonra bekleme, ms */
  delay: number
}

export interface Macro {
  name: string
  /** tuşa atanırken varsayılan tekrar modu ve sayısı (kodun b1/b2'si) */
  repeat: MacroRepeat
  count: number
  steps: MacroStep[]
}

export interface DeviceConfig {
  /** 4 katman × 128 kod; 0 = Normal, 1 = FN (arayüz yalnızca bu ikisini düzenler) */
  layers: KeyCode[][]
  led: Led
  palette: Palette
  /** grup başına 126 renk, konum = tuşun matris indeksi */
  perkey: Rgb[][]
  /** makro yuvası = dizideki indeks */
  macros: Macro[]
}

/** "device-event" yükü: alıcının kendiliğinden gönderdiği olaylar (PROTOCOL.md 0x0A). Arayüz yalnızca durumu yeniler. */
export interface DeviceEvent {
  kind: 'battery' | 'link' | 'other'
  a: number
  b: number
}

/** Resmi sürücüde olmayan ek özellik ayarları (prefs.json) */
export interface Prefs {
  /** arayüz, tepsi ve bildirim dili */
  lang: 'en' | 'tr'
  /** Caps Lock açıkken o tuşu caps_color ile yak (klavye Tuş başına moddayken) */
  caps_light: boolean
  caps_color: Rgb
  /** %15 ve %5'te sistem bildirimi */
  battery_notify: boolean
  /** düşük pilde klavye kırmızı yanıp söner (Tuş başına moddayken) */
  battery_flash: boolean
  /** sistem sesine göre spektrum çubukları (anlık ışık, Tuş başına moddayken) */
  music: boolean
  music_gain: number
  /** 0 hızlı düşüş … 9 yumuşak */
  music_smooth: number
  music_color: Rgb
  music_rainbow: boolean
}
