<script setup lang="ts">
// Özel başlık çubuğu (Tauri decorations=false). Tarayıcıda pencere butonları sessizce hiçbir şey yapmaz.
const toast = useToast()

async function run(action: 'minimize' | 'toggleMaximize' | 'close') {
  if (!isTauri()) return
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    await getCurrentWindow()[action]()
  } catch (e) {
    toast.add({ title: t('Pencere: {action} başarısız', { action }), description: String(e), color: 'error' })
  }
}

const version = ref('')
onMounted(async () => {
  if (isTauri()) version.value = await (await import('@tauri-apps/api/app')).getVersion()
})

const btn = 'grid h-full w-11 place-items-center text-muted hover:bg-accented hover:text-highlighted'
</script>

<template>
  <div
    data-tauri-drag-region
    class="flex h-10 shrink-0 items-center gap-2 border-b border-default bg-default ps-3 select-none"
  >
    <div
      data-tauri-drag-region
      class="pointer-events-none flex flex-1 items-center gap-2"
    >
      <img
        src="/logo.svg"
        alt=""
        class="size-5"
      >
      <span class="text-sm font-black tracking-wider text-highlighted">AULA<span class="text-primary">CTL</span></span>
      <span class="text-xs text-dimmed">F75{{ version && ` · v${version}` }}</span>
    </div>
    <div class="flex h-full">
      <button
        :class="btn"
        :aria-label="t('Küçült')"
        @click="run('minimize')"
      >
        <UIcon
          name="i-lucide-minus"
          class="size-4"
        />
      </button>
      <button
        :class="btn"
        :aria-label="t('Büyüt')"
        @click="run('toggleMaximize')"
      >
        <UIcon
          name="i-lucide-square"
          class="size-3.5"
        />
      </button>
      <button
        class="grid h-full w-11 place-items-center text-muted hover:bg-error hover:text-white"
        :aria-label="t('Kapat')"
        @click="run('close')"
      >
        <UIcon
          name="i-lucide-x"
          class="size-4"
        />
      </button>
    </div>
  </div>
</template>
