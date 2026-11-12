import type { Rgb } from '~/types/device'

export const toHex = (c: Rgb) => '#' + c.map(v => v.toString(16).padStart(2, '0')).join('').toUpperCase()

export function fromHex(hex: string): Rgb {
  const n = Number.parseInt(hex.replace('#', '').slice(0, 6), 16) || 0
  return [n >> 16 & 255, n >> 8 & 255, n & 255]
}

export function hsv(h: number, s = 1, v = 1): Rgb {
  const f = (n: number) => {
    const k = (n + h / 60) % 6
    return Math.round(255 * (v - v * s * Math.max(0, Math.min(k, 4 - k, 1))))
  }
  return [f(5), f(3), f(1)]
}
