<script setup lang="ts">
import layout from '~~/layout.json'
import type { Rgb } from '~/types/device'

const { config } = useDevice()
const cfg = computed(() => config.value!)
const led = computed(() => cfg.value.led)

const TR = ['Sabit', 'Nefes', 'Gökkuşağı', 'Kaçan flaş', 'Yağmur damlaları', 'Gökkuşağı çarkı', 'Dalga ışıltısı',
  'Yıldız ışıltısı', 'Kaybolan gölge', 'Retro yılan', 'Neon akış', 'Tepki', 'Sinüs dalgası', 'Tarama',
  'Dönen yel değirmeni', 'Renkli şelale', 'Çiçeklenme', 'Dönen fırtına']

const items = [
  { id: 'off', label: t('Kapalı'), sub: '', icon: 'i-lucide-power' },
  ...layout.effects.map((e, i) => ({ id: `fx${i}`, label: TR[i] ? t(TR[i]) : e.name, sub: e.name, icon: 'i-lucide-sparkles' })),
  { id: 'custom', label: t('Tuş başına'), sub: 'Self-define', icon: 'i-lucide-paintbrush' }
]

const current = computed(() => {
  if (led.value.custom) return 'custom'
  const i = layout.effects.findIndex(e => e.hw === led.value.mode)
  return i < 0 ? 'off' : `fx${i}`
})
function select(id: string) {
  led.value.custom = id === 'custom'
  if (id === 'off') led.value.mode = 0
  else if (id.startsWith('fx')) led.value.mode = layout.effects[Number(id.slice(2))]!.hw
}

const fxIndex = computed(() => current.value.startsWith('fx') ? Number(current.value.slice(2)) : -1)
const fx = computed(() => layout.effects[fxIndex.value])
const mode = computed(() => led.value.modes[fxIndex.value]!)
const palette = computed(() => cfg.value.palette[fxIndex.value]!)

const selectedHex = computed({
  get: () => toHex(palette.value[mode.value.color] ?? [0, 0, 0]),
  set: (v) => { palette.value[mode.value.color] = fromHex(v) }
})

// Önizleme: seçili renk parlaklıkla karartılır; renksiz/rastgele efektlerde gökkuşağı.
const preview = computed(() => {
  const out: (string | undefined)[] = []
  if (current.value === 'off') return out
  if (led.value.custom) {
    cfg.value.perkey[0]?.forEach((c, i) => (out[i] = toHex(c)))
    return out
  }
  const k = (mode.value.brightness + 1) / 10
  for (const key of layout.keys) {
    const c = !fx.value?.color || mode.value.color === 7 ? hsv(key.rect[0]! / 679 * 300) : palette.value[mode.value.color]!
    out[key.index] = toHex(c.map(v => Math.round(v * k)) as Rgb)
  }
  return out
})
</script>

<template>
  <div class="flex h-full flex-col">
    <PageHeader
      :title="t('Işık')"
      :sections="['led', 'palette']"
    />
    <div class="flex min-h-0 flex-1 gap-6">
      <div class="w-52 shrink-0 space-y-0.5 overflow-y-auto pr-1">
        <UButton
          v-for="it in items"
          :key="it.id"
          :title="it.sub"
          :icon="it.icon"
          :label="it.label"
          :active="current === it.id"
          color="neutral"
          variant="ghost"
          active-color="primary"
          active-variant="soft"
          block
          class="justify-start"
          @click="select(it.id)"
        />
      </div>

      <div class="flex min-w-0 flex-1 flex-col gap-5">
        <div
          v-if="fx"
          class="grid grid-cols-2 gap-x-8 gap-y-5"
        >
          <UFormField :label="t('Parlaklık · {n}', { n: mode.brightness })">
            <USlider
              v-model="mode.brightness"
              :min="0"
              :max="9"
            />
          </UFormField>
          <UFormField
            v-if="fx.speed"
            :label="t('Hız · {n}', { n: mode.speed })"
          >
            <USlider
              v-model="mode.speed"
              :min="0"
              :max="4"
            />
          </UFormField>
          <UFormField
            v-if="fx.color"
            :label="t('Renk')"
            :help="t('Bir renge tıkla; kalemle o rengi bu efektin paletinde değiştir.')"
            class="col-span-2"
          >
            <div class="flex items-center gap-2">
              <button
                v-for="(c, i) in palette"
                :key="i"
                class="size-9 rounded-md border-2"
                :class="mode.color === i ? 'border-inverted' : 'border-transparent'"
                :style="{ background: toHex(c) }"
                :aria-label="t('Renk {n}', { n: i + 1 })"
                @click="mode.color = i"
              />
              <button
                class="h-9 rounded-md border-2 bg-[conic-gradient(red,yellow,lime,cyan,blue,magenta,red)] px-3 text-sm font-semibold text-black"
                :class="mode.color === 7 ? 'border-inverted' : 'border-transparent'"
                @click="mode.color = 7"
              >
                {{ t('Rastgele') }}
              </button>
              <ColorEdit
                v-if="mode.color < 7"
                v-model="selectedHex"
              >
                <UButton
                  icon="i-lucide-pencil"
                  color="neutral"
                  variant="soft"
                  :aria-label="t('Seçili rengi düzenle')"
                />
              </ColorEdit>
            </div>
          </UFormField>
        </div>
        <p
          v-else-if="led.custom"
          class="text-sm text-muted"
        >
          {{ t('Her tuş kendi rengiyle yanar. Renkleri') }}
          <NuxtLink
            to="/renkler"
            class="text-primary hover:underline"
          >{{ t('Tuş renkleri') }}</NuxtLink> {{ t('sayfasında boya.') }}
        </p>
        <p
          v-else
          class="text-sm text-muted"
        >
          {{ t('Klavye ışığı kapalı.') }}
        </p>

        <div class="mt-auto">
          <KeyboardView :colors="preview" />
        </div>
      </div>
    </div>
  </div>
</template>
