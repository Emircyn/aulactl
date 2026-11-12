<script setup lang="ts">
import layout from '~~/layout.json'

const { config } = useDevice()
const cfg = computed(() => config.value!)

const layer = ref(0)
const selected = ref<number | null>(null)
const key = computed(() => layout.keys.find(k => k.index === selected.value))
const codes = computed(() => cfg.value.layers[layer.value]!)
const macroNames = computed(() => cfg.value.macros.map(m => m.name))

const code = computed({
  get: () => selected.value == null ? DISABLED : codes.value[selected.value]!,
  set: (v) => { if (selected.value != null) codes.value[selected.value] = v }
})
const dflt = computed(() => key.value ? defaultCode(key.value.vk) : DISABLED)
const decoded = computed(() => decode(code.value))

const labels = computed(() => {
  const out: Record<number, string> = {}
  for (const k of layout.keys) {
    const c = codes.value[k.index]!
    if (c === defaultCode(k.vk)) continue
    const d = decode(c)
    out[k.index] = d.kind === 'disabled' ? '⦸' : d.kind === 'special' ? t('Özel') : d.kind === 'media' ? mediaShort.get(d.usage) ?? t('Medya') : describe(c, macroNames.value)
  }
  return out
})

// --- Normal tuş formu
const MODS = [{ label: 'Ctrl', bit: 1 }, { label: 'Shift', bit: 2 }, { label: 'Alt', bit: 4 }, { label: 'Win', bit: 8 }]
const mods = ref(0)
const hid = ref<number | undefined>()
const keyItems = HID_KEYS.map(([value, label]) => ({ value, label: t(label) }))

// --- Makro formu
const macroSlot = ref<number | undefined>()
const repeat = ref(1)
const count = ref(1)
const repeatItems = Object.entries(REPEAT_LABEL).map(([v, label]) => ({ value: Number(v), label: t(label) }))

const tab = ref('key')
watch([selected, layer], () => {
  const d = decoded.value
  mods.value = d.kind === 'key' ? (d.mods | d.mods >> 4) & 0xF : 0
  hid.value = d.kind === 'key' && d.key ? d.key : undefined
  if (d.kind === 'macro') {
    macroSlot.value = d.slot
    repeat.value = d.mode
    count.value = d.count
  }
  tab.value = d.kind === 'media' ? 'media' : d.kind === 'macro' ? 'macro' : 'key'
})

function pickMacro(slot: number | undefined) {
  const m = slot == null ? undefined : cfg.value.macros[slot]
  if (!m) return
  repeat.value = m.repeat
  count.value = m.count
}

const capturing = ref(false)
function onKey(e: KeyboardEvent) {
  if (!capturing.value) return
  e.preventDefault()
  const u = codeToHid.get(e.code)
  if (u == null || u >= 0xE0) return // modifier tek başına: bırakınca/diğer tuşla belli olur
  mods.value = (e.ctrlKey ? 1 : 0) | (e.shiftKey ? 2 : 0) | (e.altKey ? 4 : 0) | (e.metaKey ? 8 : 0)
  hid.value = u
  capturing.value = false
}
onMounted(() => window.addEventListener('keydown', onKey))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))

const tabs = [
  { label: t('Tuş'), value: 'key', icon: 'i-lucide-keyboard' },
  { label: t('Medya'), value: 'media', icon: 'i-lucide-music' },
  { label: t('Makro'), value: 'macro', icon: 'i-lucide-list-video' }
]
</script>

<template>
  <div class="flex h-full flex-col">
    <PageHeader
      :title="t('Tuşlar')"
      :sections="['keymap']"
    >
      <UTabs
        v-model="layer"
        :items="[{ label: t('Normal'), value: 0 }, { label: t('FN katmanı'), value: 1 }]"
        :content="false"
        size="sm"
        class="ml-4 w-56"
      />
    </PageHeader>

    <KeyboardView
      :labels="labels"
      :selected="selected"
      @press="selected = $event"
    />

    <div
      v-if="key"
      class="mt-5 flex min-h-0 flex-1 gap-6 rounded-lg border border-default bg-muted p-4"
    >
      <div class="w-56 shrink-0 space-y-3">
        <div>
          <div class="text-xs text-dimmed">
            {{ layer ? 'FN + ' : '' }}{{ t('Tuş') }}
          </div>
          <div class="text-lg font-semibold text-highlighted">
            {{ key.name.startsWith('K75') ? 'RCtrl' : key.name }}
          </div>
        </div>
        <div>
          <div class="text-xs text-dimmed">
            {{ t('Şu an') }}
          </div>
          <div class="text-secondary">
            {{ describe(code, macroNames) }}
          </div>
          <div
            v-if="code !== dflt"
            class="text-xs text-dimmed"
          >
            {{ t('Varsayılan: {key}', { key: describe(dflt) }) }}
          </div>
        </div>
        <div class="flex flex-wrap gap-2">
          <UButton
            size="sm"
            color="neutral"
            variant="soft"
            icon="i-lucide-rotate-ccw"
            :label="t('Varsayılan')"
            :disabled="code === dflt"
            @click="code = dflt"
          />
          <UButton
            size="sm"
            color="error"
            variant="soft"
            icon="i-lucide-ban"
            :label="t('Devre dışı')"
            :disabled="code === DISABLED"
            @click="code = DISABLED"
          />
        </div>
      </div>

      <div
        v-if="decoded.kind === 'special'"
        class="text-sm text-muted"
      >
        {{ t('Bu tuş üretici kodu taşıyor') }} (<span class="font-mono">{{ code }}</span>: {{ t('ışık/Fn/özel işlev') }}).
        {{ t('Düzenlenemez; yalnızca varsayılana döndürülebilir ya da başka bir işlev atanabilir.') }}
        <UButton
          class="mt-3 block"
          size="sm"
          variant="link"
          :label="t('Yine de yeni işlev ata')"
          @click="code = DISABLED"
        />
      </div>

      <div
        v-else
        class="min-w-0 flex-1 space-y-4"
      >
        <UTabs
          v-model="tab"
          :items="tabs"
          :content="false"
          size="sm"
          class="w-80"
        />

        <div
          v-if="tab === 'key'"
          class="flex flex-wrap items-end gap-3"
        >
          <div class="flex gap-1">
            <UButton
              v-for="m in MODS"
              :key="m.bit"
              :label="m.label"
              size="sm"
              :variant="mods & m.bit ? 'solid' : 'outline'"
              :color="mods & m.bit ? 'primary' : 'neutral'"
              @click="mods ^= m.bit"
            />
          </div>
          <span class="pb-1 text-muted">+</span>
          <USelectMenu
            v-model="hid"
            :items="keyItems"
            value-key="value"
            :placeholder="t('Tuş seç')"
            size="sm"
            class="w-44"
          />
          <UButton
            size="sm"
            color="neutral"
            variant="soft"
            :icon="capturing ? 'i-lucide-loader-circle' : 'i-lucide-scan-line'"
            :label="capturing ? t('Bir tuşa bas…') : t('Basarak seç')"
            @click="capturing = !capturing"
          />
          <UButton
            size="sm"
            icon="i-lucide-check"
            :label="t('Ata')"
            :disabled="hid == null && !mods"
            @click="code = encodeKey(mods, hid ?? 0)"
          />
        </div>

        <div
          v-else-if="tab === 'media'"
          class="flex flex-wrap gap-2"
        >
          <UButton
            v-for="[u, name] in MEDIA"
            :key="u"
            size="sm"
            :label="t(name)"
            :variant="decoded.kind === 'media' && decoded.usage === u ? 'solid' : 'soft'"
            :color="decoded.kind === 'media' && decoded.usage === u ? 'primary' : 'neutral'"
            @click="code = encodeMedia(u)"
          />
        </div>

        <div
          v-else
          class="flex flex-wrap items-end gap-3"
        >
          <p
            v-if="!cfg.macros.length"
            class="text-sm text-muted"
          >
            {{ t('Henüz makro yok.') }} <NuxtLink
              to="/makrolar"
              class="text-primary hover:underline"
            >{{ t('Makrolar') }}</NuxtLink> {{ t('sayfasından oluştur.') }}
          </p>
          <template v-else>
            <UFormField :label="t('Makro')">
              <USelect
                v-model="macroSlot"
                :items="cfg.macros.map((m, i) => ({ label: m.name, value: i }))"
                :placeholder="t('Seç')"
                size="sm"
                class="w-44"
                @update:model-value="pickMacro"
              />
            </UFormField>
            <UFormField :label="t('Tekrar')">
              <USelect
                v-model="repeat"
                :items="repeatItems"
                size="sm"
                class="w-52"
              />
            </UFormField>
            <UFormField
              v-if="repeat === 1"
              :label="t('Kez')"
            >
              <UInputNumber
                v-model="count"
                :min="1"
                :max="255"
                size="sm"
                class="w-28"
              />
            </UFormField>
            <UButton
              size="sm"
              icon="i-lucide-check"
              :label="t('Ata')"
              :disabled="macroSlot == null"
              @click="code = encodeMacro(repeat, count, macroSlot!)"
            />
          </template>
        </div>
      </div>
    </div>
    <p
      v-else
      class="mt-5 text-sm text-muted"
    >
      {{ t('Atamak için klavyede bir tuşa tıkla. Varsayılandan farklı tuşlar mor çerçeveyle gösterilir.') }}
    </p>
  </div>
</template>
