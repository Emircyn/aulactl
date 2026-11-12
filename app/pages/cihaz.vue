<script setup lang="ts">
import type { Section } from '~/composables/useDevice'

const { status, busy, dirty, apply, revert, run, loadConfig } = useDevice()

const lastBackup = ref('')
const restorePath = ref('')
const confirmReset = ref(false)
const toast = useToast()

async function backup() {
  const r = await run<string>('backup')
  if (!r) return
  lastBackup.value = restorePath.value = r.value
  toast.add({ title: t('Yedek alındı'), description: r.value, color: 'success', icon: 'i-lucide-check' })
}
async function restore() {
  if (await run('restore', { path: restorePath.value })) toast.add({ title: t('Yedekten geri yüklendi'), color: 'success', icon: 'i-lucide-check' })
}
async function reset() {
  confirmReset.value = false
  if (await run('factory_reset')) toast.add({ title: t('Fabrika ayarlarına dönüldü'), color: 'success', icon: 'i-lucide-check' })
}

const sections = Object.keys(SECTION_LABEL) as Section[]
const info = computed(() => {
  const s = status.value
  return [
    { label: t('Bağlantı'), value: !s.connected ? t('Bağlı değil') : s.transport === 'wired' ? t('Kablolu (USB)') : t('2.4G alıcı') },
    { label: t('Klavye'), value: s.online ? t('Çevrimiçi') : t('Uykuda / kapalı') },
    { label: t('Pil'), value: s.battery == null ? '—' : t('%{n}', { n: s.battery }) },
    { label: t('Şarj'), value: s.charging == null ? '—' : s.charging ? t('Şarj oluyor') : t('Pilden çalışıyor') },
    { label: 'Firmware', value: s.fw ?? '—' }
  ]
})
</script>

<template>
  <div class="max-w-3xl space-y-6">
    <h1 class="text-xl font-semibold text-highlighted">
      {{ t('Cihaz') }}
    </h1>

    <div class="grid grid-cols-5 gap-3">
      <div
        v-for="i in info"
        :key="i.label"
        class="rounded-lg border border-default bg-muted p-3"
      >
        <div class="text-xs text-dimmed">
          {{ i.label }}
        </div>
        <div class="mt-1 text-highlighted">
          {{ i.value }}
        </div>
      </div>
    </div>

    <section class="space-y-2">
      <h2 class="font-medium text-highlighted">
        {{ t('Klavyeye yazılacak değişiklikler') }}
      </h2>
      <div class="divide-y divide-default rounded-lg border border-default">
        <div
          v-for="s in sections"
          :key="s"
          class="flex items-center gap-3 px-4 py-2 text-sm"
        >
          <span
            class="size-2 rounded-full"
            :class="dirty(s) ? 'bg-warning' : 'bg-success'"
          />
          <span class="flex-1">{{ SECTION_LABEL[s] }}</span>
          <template v-if="dirty(s)">
            <UButton
              size="xs"
              color="neutral"
              variant="ghost"
              :label="t('Geri al')"
              @click="revert(s)"
            />
            <UButton
              size="xs"
              :label="t('Uygula')"
              :loading="busy"
              @click="apply(s)"
            />
          </template>
          <span
            v-else
            class="text-xs text-dimmed"
          >{{ t('Klavyeyle aynı') }}</span>
        </div>
      </div>
      <UButton
        color="neutral"
        variant="ghost"
        icon="i-lucide-refresh-cw"
        :label="t('Klavyeden yeniden oku')"
        :disabled="busy"
        @click="loadConfig"
      />
    </section>

    <section class="space-y-3">
      <h2 class="font-medium text-highlighted">
        {{ t('Yedek') }}
      </h2>
      <p class="text-sm text-muted">
        {{ t('Her yazmadan önce otomatik yedek alınır. Elle de alabilirsin.') }}
      </p>
      <div class="flex items-center gap-2">
        <UButton
          icon="i-lucide-archive"
          :label="t('Yedek al')"
          color="neutral"
          variant="soft"
          :disabled="busy || !status.connected"
          @click="backup"
        />
        <span
          v-if="lastBackup"
          class="truncate font-mono text-xs text-muted select-text"
        >{{ lastBackup }}</span>
      </div>
      <div class="flex items-center gap-2">
        <UInput
          v-model="restorePath"
          placeholder="~/.config/aulactl/backups/….json"
          class="flex-1 font-mono"
        />
        <UButton
          icon="i-lucide-archive-restore"
          :label="t('Yedekten geri yükle')"
          color="neutral"
          variant="soft"
          :disabled="busy || !restorePath || !status.connected"
          @click="restore"
        />
      </div>
    </section>

    <section class="space-y-3">
      <h2 class="font-medium text-error">
        {{ t('Tehlikeli bölge') }}
      </h2>
      <UButton
        icon="i-lucide-rotate-ccw"
        :label="t('Fabrika ayarlarına dön')"
        color="error"
        variant="soft"
        :disabled="busy || !status.connected"
        @click="confirmReset = true"
      />
    </section>

    <UModal
      v-model:open="confirmReset"
      :title="t('Fabrika ayarlarına dönülsün mü?')"
      :description="t('Tüm tuş atamaları, ışık ayarları, renkler ve makrolar silinir. Önce otomatik yedek alınır.')"
    >
      <template #footer>
        <UButton
          color="neutral"
          variant="ghost"
          :label="t('Vazgeç')"
          @click="confirmReset = false"
        />
        <UButton
          color="error"
          :label="t('Fabrika ayarlarına dön')"
          @click="reset"
        />
      </template>
    </UModal>
  </div>
</template>
