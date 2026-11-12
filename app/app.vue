<script setup lang="ts">
import { en as uiEn, tr as uiTr } from '@nuxt/ui/locale'

useHead({
  meta: [{ name: 'viewport', content: 'width=device-width, initial-scale=1' }],
  link: [{ rel: 'icon', href: '/favicon.ico' }],
  htmlAttrs: { lang }
})
useSeoMeta({ title: 'aulactl', description: t('AULA F75 için hafif ayar uygulaması') })

const { status, config, busy, loadConfig } = useDevice()
onMounted(initDevice)

const nav = [
  { label: t('Işık'), icon: 'i-lucide-sun', to: '/' },
  { label: t('Tuşlar'), icon: 'i-lucide-keyboard', to: '/tuslar' },
  { label: t('Tuş renkleri'), icon: 'i-lucide-paintbrush', to: '/renkler' },
  { label: t('Makrolar'), icon: 'i-lucide-list-video', to: '/makrolar' },
  { label: t('Profiller'), icon: 'i-lucide-bookmark', to: '/profiller' },
  { label: t('Ayarlar'), icon: 'i-lucide-settings', to: '/ayarlar' },
  { label: t('Cihaz'), icon: 'i-lucide-cpu', to: '/cihaz' }
]
</script>

<template>
  <UApp :locale="lang === 'tr' ? uiTr : uiEn">
    <div
      class="flex h-screen flex-col"
      @contextmenu.prevent
    >
      <TitleBar />
      <div class="flex min-h-0 flex-1">
        <nav class="flex w-52 shrink-0 flex-col gap-1 border-r border-default p-3">
          <UNavigationMenu
            :items="nav"
            orientation="vertical"
            :ui="{ link: 'py-2 text-sm' }"
          />
        </nav>
        <main class="relative min-w-0 flex-1 overflow-y-auto p-6">
          <NuxtPage v-if="config" />
          <div
            v-else
            class="grid h-full place-items-center text-center"
          >
            <div class="space-y-3">
              <template v-if="status.transport === 'bluetooth'">
                <UIcon
                  name="i-lucide-bluetooth"
                  class="size-10 text-dimmed"
                />
                <p class="text-muted">
                  {{ t('Klavye Bluetooth ile bağlı. Bu modda ayarlar değiştirilemiyor; ayar için kabloyla ya da 2.4G alıcıyla bağlayın.') }}
                </p>
              </template>
              <template v-else-if="status.connected && !busy">
                <UIcon
                  name="i-lucide-moon"
                  class="size-10 text-dimmed"
                />
                <p class="text-muted">
                  {{ t('Klavye yanıt vermedi; uykuda olabilir. Bir tuşa bas, uyanınca ayarlar kendiliğinden okunur.') }}
                </p>
                <UButton
                  icon="i-lucide-refresh-cw"
                  :label="t('Tekrar dene')"
                  variant="soft"
                  @click="loadConfig"
                />
              </template>
              <template v-else>
                <UIcon
                  :name="status.connected ? 'i-lucide-loader-circle' : 'i-lucide-unplug'"
                  class="size-10 text-dimmed"
                  :class="{ 'animate-spin': status.connected }"
                />
                <p class="text-muted">
                  {{ status.connected ? t('Ayarlar klavyeden okunuyor…') : t('Klavye bağlı değil. Kabloyu ya da 2.4G alıcıyı takın.') }}
                </p>
              </template>
            </div>
          </div>
        </main>
      </div>
      <StatusBar />
    </div>
  </UApp>
</template>
