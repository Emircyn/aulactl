<script setup lang="ts">
import type { Macro, MacroRepeat } from '~/types/device'

const { config } = useDevice()
const cfg = computed(() => config.value!)
const macros = computed(() => cfg.value.macros)

const sel = ref(0)
const macro = computed(() => macros.value[sel.value])

const keyItems = HID_KEYS.map(([value, label]) => ({ value, label }))
const actionItems = [{ label: t('Bas'), value: 'down' }, { label: t('Bırak'), value: 'up' }]
const repeatItems = Object.entries(REPEAT_LABEL).map(([v, label]) => ({ value: Number(v) as MacroRepeat, label: t(label) }))

function add() {
  macros.value.push({ name: t('Makro {n}', { n: macros.value.length + 1 }), repeat: 1, count: 1, steps: [] } satisfies Macro)
  sel.value = macros.value.length - 1
}

// Makro yuvası = dizideki indeks: silince bu yuvayı kullanan tuşlar devre dışı, sonrakiler bir kayar.
function remove(i: number) {
  macros.value.splice(i, 1)
  for (const codes of cfg.value.layers) {
    codes.forEach((c, k) => {
      const d = decode(c)
      if (d.kind !== 'macro' || d.slot < i) return
      codes[k] = d.slot === i ? DISABLED : encodeMacro(d.mode, d.count, d.slot - 1)
    })
  }
  sel.value = Math.max(0, Math.min(sel.value, macros.value.length - 1))
}
const usedBy = computed(() => cfg.value.layers.slice(0, 2).flat().filter((c) => {
  const d = decode(c)
  return d.kind === 'macro' && d.slot === sel.value
}).length)
const confirmDelete = ref(false)

// --- kayıt: penceredeki gerçek tuş basışları
const recording = ref(false)
let last = 0
function rec(e: KeyboardEvent) {
  if (!recording.value || !macro.value || e.repeat) return
  e.preventDefault()
  const key = codeToHid.get(e.code)
  if (key == null) return
  const now = performance.now()
  const steps = macro.value.steps
  if (steps.length) steps[steps.length - 1]!.delay = Math.min(65535, Math.round(now - last))
  last = now
  steps.push({ action: e.type === 'keydown' ? 'down' : 'up', key, delay: 0 })
}
function toggleRec() {
  recording.value = !recording.value
  last = performance.now()
}
onMounted(() => {
  window.addEventListener('keydown', rec)
  window.addEventListener('keyup', rec)
})
onBeforeUnmount(() => {
  window.removeEventListener('keydown', rec)
  window.removeEventListener('keyup', rec)
})
watch(sel, () => (recording.value = false))
</script>

<template>
  <div class="flex h-full flex-col">
    <PageHeader
      :title="t('Makrolar')"
      :sections="['macros', 'keymap']"
    />
    <div class="flex min-h-0 flex-1 gap-6">
      <div class="flex w-56 shrink-0 flex-col gap-1">
        <div class="min-h-0 flex-1 space-y-0.5 overflow-y-auto">
          <UButton
            v-for="(m, i) in macros"
            :key="i"
            :label="m.name"
            :active="sel === i"
            color="neutral"
            variant="ghost"
            active-color="primary"
            active-variant="soft"
            block
            class="justify-start"
            @click="sel = i"
          >
            <template #leading>
              <span class="w-5 text-xs text-dimmed">{{ i + 1 }}</span>
            </template>
            <template #trailing>
              <span class="ms-auto text-xs text-dimmed">{{ m.steps.length }}</span>
            </template>
          </UButton>
        </div>
        <UButton
          icon="i-lucide-plus"
          :label="t('Yeni makro')"
          color="neutral"
          variant="soft"
          block
          @click="add"
        />
      </div>

      <div
        v-if="macro"
        class="flex min-w-0 flex-1 flex-col gap-4"
      >
        <div class="flex items-end gap-3">
          <UFormField
            :label="t('Ad')"
            class="flex-1"
          >
            <UInput
              v-model="macro.name"
              class="w-full"
            />
          </UFormField>
          <UFormField :label="t('Tekrar (tuşa atarken varsayılan)')">
            <USelect
              v-model="macro.repeat"
              :items="repeatItems"
              class="w-52"
            />
          </UFormField>
          <UFormField
            v-if="macro.repeat === 1"
            :label="t('Kez')"
          >
            <UInputNumber
              v-model="macro.count"
              :min="1"
              :max="255"
              class="w-28"
            />
          </UFormField>
          <UButton
            color="error"
            variant="soft"
            icon="i-lucide-trash-2"
            :aria-label="t('Makroyu sil')"
            @click="confirmDelete = true"
          />
        </div>

        <div class="flex items-center gap-2">
          <UButton
            :color="recording ? 'error' : 'neutral'"
            :variant="recording ? 'solid' : 'soft'"
            :icon="recording ? 'i-lucide-square' : 'i-lucide-circle'"
            :label="recording ? t('Kaydı durdur') : t('Tuşlardan kaydet')"
            @click="toggleRec"
          />
          <UButton
            color="neutral"
            variant="soft"
            icon="i-lucide-plus"
            :label="t('Adım ekle')"
            @click="macro.steps.push({ action: 'down', key: 0x04, delay: 20 })"
          />
          <UButton
            color="neutral"
            variant="ghost"
            icon="i-lucide-list-x"
            :label="t('Adımları temizle')"
            :disabled="!macro.steps.length"
            @click="macro.steps.splice(0)"
          />
          <span
            v-if="recording"
            class="text-sm text-error"
          >{{ t('Kayıt sürüyor: bu pencerede bastığın tuşlar ve aralarındaki süre ekleniyor.') }}</span>
        </div>

        <div class="min-h-0 flex-1 overflow-y-auto rounded-lg border border-default">
          <p
            v-if="!macro.steps.length"
            class="p-6 text-center text-sm text-muted"
          >
            {{ t('Adım yok.') }}
          </p>
          <div
            v-for="(s, i) in macro.steps"
            :key="i"
            class="flex items-center gap-3 border-b border-default px-3 py-1.5 last:border-0"
          >
            <span class="w-6 text-xs text-dimmed">{{ i + 1 }}</span>
            <USelect
              v-model="s.action"
              :items="actionItems"
              size="sm"
              class="w-24"
            />
            <USelectMenu
              v-model="s.key"
              :items="keyItems"
              value-key="value"
              size="sm"
              class="w-40"
            />
            <UInputNumber
              v-model="s.delay"
              :min="0"
              :max="65535"
              size="sm"
              class="w-32"
            />
            <span class="text-xs text-dimmed">{{ t('ms bekle') }}</span>
            <span class="flex-1" />
            <UButton
              size="xs"
              color="neutral"
              variant="ghost"
              icon="i-lucide-x"
              :aria-label="t('Adımı sil')"
              @click="macro.steps.splice(i, 1)"
            />
          </div>
        </div>
      </div>
      <p
        v-else
        class="text-sm text-muted"
      >
        {{ t('Makro yok. “Yeni makro” ile başla.') }}
      </p>
    </div>

    <UModal
      v-model:open="confirmDelete"
      :title="t('Makro silinsin mi?')"
      :description="usedBy ? t('{n} tuş bu makroya atanmış; bu tuşlar devre dışı kalır.', { n: usedBy }) : t('Bu makro hiçbir tuşa atanmamış.')"
    >
      <template #footer>
        <UButton
          color="neutral"
          variant="ghost"
          :label="t('Vazgeç')"
          @click="confirmDelete = false"
        />
        <UButton
          color="error"
          :label="t('Sil')"
          @click="remove(sel); confirmDelete = false"
        />
      </template>
    </UModal>
  </div>
</template>
