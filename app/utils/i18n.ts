// Arayüz dili. Anahtar = Türkçe metin; İngilizce karşılığı app/locales/en.ts içinde, eksikse Türkçe kalır.
// Dil localStorage + Rust prefs.lang (tepsi/bildirimler) içinde; değişince pencere yeniden yüklenir, reaktiflik gerekmez.
import en from '../locales/en.ts'

export type Lang = 'en' | 'tr'

function stored(): Lang {
  try {
    return localStorage.getItem('lang') === 'tr' ? 'tr' : 'en'
  } catch {
    return 'en'
  }
}

export const lang: Lang = stored()

/** t('Pil %{n}', { n: 62 }): dile göre metni seçer, {n} yerine değeri koyar */
export function t(s: string, vars?: Record<string, string | number>): string {
  let r = lang === 'tr' ? s : (en[s] ?? s)
  if (vars) for (const [k, v] of Object.entries(vars)) r = r.replaceAll(`{${k}}`, String(v))
  return r
}

/** Dili kaydeder ve pencereyi yeniden yükler (metinler modül yüklenirken seçiliyor). */
export function setLang(l: Lang) {
  try {
    localStorage.setItem('lang', l)
  } catch { /* yoksa varsayılan en kalır */ }
  location.reload()
}
