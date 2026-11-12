<script setup lang="ts">
// Profil = klavyenin tam görüntüsü (tuşlar, ışık, renkler) + makrolar. Tepsi menüsünden de yüklenebilir.
const { loadConfig, busy } = useDevice()
const toast = useToast()
const profiles = ref<string[]>([])
const name = ref('')

async function run(done: string, f: () => Promise<unknown>) {
  busy.value = true
  try {
    await f()
    toast.add({ title: done, color: 'success', icon: 'i-lucide-check' })
  } catch (e) {
    toast.add({ title: t('İşlem başarısız'), description: String(e), color: 'error', icon: 'i-lucide-triangle-alert' })
  } finally {
    busy.value = false
  }
}

async function refresh() {
  profiles.value = await call<string[]>('profile_list')
}
onMounted(refresh)

async function save() {
  const n = name.value.trim()
  if (!n) return
  await run(t('"{n}" kaydedildi', { n }), () => call('profile_save', { name: n }))
  name.value = ''
  await refresh()
}

async function apply(n: string) {
  await run(t('"{n}" klavyeye yüklendi', { n }), () => call('profile_apply', { name: n }))
  await loadConfig()
}

const toDelete = ref<string | null>(null)
async function remove(n: string) {
  toDelete.value = null
  await run(t('"{n}" silindi', { n }), () => call('profile_delete', { name: n }))
  await refresh()
}
</script>

<template>
  <div class="max-w-2xl">
    <h1 class="mb-5 text-xl font-semibold text-highlighted">
      {{ t('Profiller') }}
    </h1>

    <div class="mb-6 flex gap-2">
      <UInput
        v-model="name"
        :placeholder="t('Profil adı (ör. Oyun)')"
        class="w-64"
        @keydown.enter="save"
      />
      <UButton
        icon="i-lucide-save"
        :label="t('Şu anki ayarları kaydet')"
        :disabled="!name.trim() || busy"
        @click="save"
      />
    </div>

    <p
      v-if="!profiles.length"
      class="text-muted"
    >
      {{ t('Henüz profil yok. Klavyeyi istediğin gibi ayarlayıp bir ad vererek kaydet.') }}
    </p>
    <ul class="divide-y divide-default border-y border-default">
      <li
        v-for="p in profiles"
        :key="p"
        class="flex items-center gap-3 py-2.5"
      >
        <UIcon
          name="i-lucide-bookmark"
          class="size-4 text-primary"
        />
        <span class="flex-1 text-highlighted">{{ p }}</span>
        <UButton
          size="sm"
          icon="i-lucide-upload"
          :label="t('Yükle')"
          :disabled="busy"
          @click="apply(p)"
        />
        <UButton
          size="sm"
          color="error"
          variant="ghost"
          icon="i-lucide-trash-2"
          :disabled="busy"
          :aria-label="t('{p} profilini sil', { p })"
          @click="toDelete = p"
        />
      </li>
    </ul>

    <UModal
      :open="!!toDelete"
      :title="t('“{p}” profili silinsin mi?', { p: toDelete ?? '' })"
      :description="t('Bu kayıt geri alınamaz. Klavyedeki ayarlar değişmez.')"
      @update:open="toDelete = null"
    >
      <template #footer>
        <UButton
          color="neutral"
          variant="ghost"
          :label="t('Vazgeç')"
          @click="toDelete = null"
        />
        <UButton
          color="error"
          :label="t('Sil')"
          @click="remove(toDelete!)"
        />
      </template>
    </UModal>

    <p class="mt-4 text-sm text-muted">
      {{ t('Profil yüklemek klavyenin kalıcı belleğine yazar; sık sık geçiş yapma. Profillere sistem tepsisindeki simgeye sağ tıklayarak da ulaşabilirsin.') }}
    </p>
  </div>
</template>
