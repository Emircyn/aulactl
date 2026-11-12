<script setup lang="ts">
// 80 tuşu layout.json dikdörtgenleriyle (679×284 tuval) çizer.
import layout from '~~/layout.json'

const props = defineProps<{
  /** matris indeksi → renk (#RRGGBB); verilirse tuş o renkte parlar */
  colors?: (string | undefined)[]
  /** matris indeksi → varsayılandan farklı etiket */
  labels?: Record<number, string>
  selected?: number | null
  /** true: basılı sürükleyerek boyama */
  paint?: boolean
}>()
const emit = defineEmits<{ press: [index: number] }>()

const W = 679
const H = 284
const CAP: Record<string, string> = {
  'Backspace': '⌫', 'Delete': 'Del', 'CapsLock': 'Caps', 'LShift': 'Shift', 'RShift': 'Shift',
  'LCtrl': 'Ctrl', 'LWin': 'Win', 'LAlt': 'Alt', 'Space': '', 'FN': 'Fn', 'Enter': 'Enter',
  'PgUp': 'Pg↑', 'PgDn': 'Pg↓', 'Up': '↑', 'Down': '↓', 'Left': '←', 'Right': '→', '`': '`'
}
const keys = layout.keys.map((k) => {
  const [x1, y1, x2, y2] = k.rect as [number, number, number, number]
  return {
    index: k.index,
    name: k.name,
    cap: k.vk === 0xC4 ? 'Ctrl' : CAP[k.name] ?? k.name,
    style: { left: `${x1 / W * 100}%`, top: `${y1 / H * 100}%`, width: `${(x2 - x1) / W * 100}%`, height: `${(y2 - y1) / H * 100}%` }
  }
})

function keyStyle(k: typeof keys[number]) {
  const c = props.colors?.[k.index]
  if (!c) return k.style
  return {
    ...k.style,
    background: `color-mix(in srgb, ${c} 40%, #000)`,
    borderColor: c,
    boxShadow: `0 0 0.6em color-mix(in srgb, ${c} 45%, transparent)`
  }
}

function enter(e: PointerEvent, i: number) {
  if (props.paint && e.buttons === 1) emit('press', i)
}
</script>

<template>
  <div
    class="relative aspect-[679/284] w-full rounded-[2.5%/6%] bg-muted ring-1 ring-default @container"
    @dragstart.prevent
  >
    <button
      v-for="k in keys"
      :key="k.index"
      :title="labels?.[k.index] ? `${k.name} → ${labels[k.index]}` : k.name"
      :aria-label="k.name"
      :style="keyStyle(k)"
      class="absolute flex flex-col items-center justify-center overflow-hidden rounded-[0.3em] border border-accented bg-elevated/95 text-[max(0.75rem,1.45cqw)] leading-none font-medium tracking-tight text-highlighted transition-[box-shadow,border-color] hover:border-primary"
      :class="{
        'z-10 border-primary ring-2 ring-primary': selected === k.index,
        'border-secondary': labels?.[k.index] && selected !== k.index
      }"
      @click="emit('press', k.index)"
      @pointerenter="enter($event, k.index)"
    >
      <span
        v-if="labels?.[k.index]"
        class="max-w-full overflow-hidden whitespace-nowrap text-secondary"
      >{{ labels[k.index] }}</span>
      <span
        v-else
        class="max-w-full overflow-hidden whitespace-nowrap"
      >{{ k.cap }}</span>
    </button>
  </div>
</template>
