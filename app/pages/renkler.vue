<script setup lang="ts">
import layout from '~~/layout.json'

const { config } = useDevice()
const cfg = computed(() => config.value!)
// ponytail: yalnızca 0. grup düzenleniyor; grupların anlamı (profil/katman?) çözülünce seçici eklenir
const colors = computed(() => cfg.value.perkey[0]!)
const view = computed(() => colors.value.map(c => (c[0] || c[1] || c[2] ? toHex(c) : undefined)))

const brush = ref('#00E5FF')
const swatches = ['#FF0000', '#FF7A00', '#FFE600', '#00FF6A', '#00E5FF', '#0050FF', '#B000FF', '#FF00AA', '#FFFFFF']

function paint(i: number) {
  colors.value[i] = fromHex(brush.value)
}
function fill(hex: string) {
  const c = fromHex(hex)
  for (const k of layout.keys) colors.value[k.index] = [...c]
}
</script>

<template>
  <div class="flex h-full flex-col">
    <PageHeader
      :title="t('Tuş renkleri')"
      :sections="['perkey', 'led']"
    />

    <UAlert
      v-if="!cfg.led.custom"
      class="mb-4"
      color="warning"
      variant="subtle"
      icon="i-lucide-info"
      :title="t('Bu renkler yalnızca “Tuş başına” ışık modunda görünür.')"
      :actions="[{ label: t('Tuş başına moda geç'), color: 'warning', variant: 'solid', onClick: () => { cfg.led.custom = true } }]"
    />

    <div class="mb-4 flex flex-wrap items-center gap-2">
      <ColorEdit v-model="brush" />
      <span class="mr-2 font-mono text-sm text-muted">{{ brush }}</span>
      <button
        v-for="s in swatches"
        :key="s"
        class="size-7 rounded-md border-2"
        :class="brush === s ? 'border-inverted' : 'border-transparent'"
        :style="{ background: s }"
        :aria-label="t('Fırça rengi {c}', { c: s })"
        @click="brush = s"
      />
      <span class="flex-1" />
      <UButton
        color="neutral"
        variant="soft"
        icon="i-lucide-paint-bucket"
        :label="t('Tümünü boya')"
        @click="fill(brush)"
      />
      <UButton
        color="neutral"
        variant="soft"
        icon="i-lucide-eraser"
        :label="t('Temizle')"
        @click="fill('#000000')"
      />
    </div>

    <KeyboardView
      :colors="view"
      paint
      @press="paint"
    />
    <p class="mt-4 text-sm text-muted">
      {{ t('Tuşa tıkla ya da basılı tutup sürükleyerek boya. Siyah = kapalı.') }}
    </p>
  </div>
</template>
