<script setup lang="ts">
const { status, config, anyDirty, busy, refreshStatus, loadConfig } = useDevice()
const mocked = !isTauri()

const link = computed(() => {
  const s = status.value
  if (!s.connected) return { icon: 'i-lucide-unplug', text: t('Bağlı değil'), cls: 'text-error' }
  if (s.transport === 'wired') return { icon: 'i-lucide-cable', text: t('Kablolu bağlı'), cls: 'text-primary' }
  if (s.transport === 'bluetooth') return { icon: 'i-lucide-bluetooth', text: t('Bluetooth ile bağlı'), cls: 'text-primary' }
  return s.online
    ? { icon: 'i-lucide-radio', text: t('2.4G alıcı ile bağlı'), cls: 'text-primary' }
    : { icon: 'i-lucide-radio', text: t('2.4G alıcı takılı · klavye kapalı ya da uykuda'), cls: 'text-warning' }
})

const battery = computed(() => {
  const { battery: b, charging } = status.value
  if (charging) return 'i-lucide-battery-charging'
  if (b == null) return 'i-lucide-battery'
  return b > 66 ? 'i-lucide-battery-full' : b > 33 ? 'i-lucide-battery-medium' : b > 10 ? 'i-lucide-battery-low' : 'i-lucide-battery-warning'
})

async function cycle() {
  mockCycleTransport()
  await refreshStatus()
  if (status.value.connected && !config.value) await loadConfig()
}
</script>

<template>
  <footer class="flex h-8 shrink-0 items-center gap-5 border-t border-default bg-default px-4 text-xs text-muted">
    <span
      class="flex items-center gap-1.5"
      :class="link.cls"
    >
      <UIcon
        :name="link.icon"
        class="size-4"
      />{{ link.text }}
    </span>
    <span
      v-if="status.connected"
      class="flex items-center gap-1.5"
      :title="status.charging ? t('Şarj oluyor') : t('Pil')"
    >
      <UIcon
        :name="battery"
        class="size-4"
        :class="{ 'text-success': status.charging, 'text-error': (status.battery ?? 100) <= 10 }"
      />
      {{ status.battery != null ? t('%{n}', { n: status.battery }) : status.charging ? t('Şarjda') : '—' }}
    </span>
    <span v-if="status.fw">Firmware {{ status.fw }}</span>
    <UIcon
      v-if="busy"
      name="i-lucide-loader-circle"
      class="size-4 animate-spin text-primary"
    />
    <span class="flex-1" />
    <NuxtLink
      v-if="anyDirty.length"
      to="/cihaz"
      class="flex items-center gap-1.5 text-warning"
    >
      <span class="size-2 rounded-full bg-warning" />{{ t('{n} bölüm yazılmadı', { n: anyDirty.length }) }}
    </NuxtLink>
    <UButton
      v-if="mocked"
      size="xs"
      variant="soft"
      color="secondary"
      icon="i-lucide-flask-conical"
      :label="t('Sahte cihaz')"
      :title="t('Tauri dışında çalışıyor. Tıkla: bağlantı türünü değiştir')"
      @click="cycle"
    />
  </footer>
</template>
