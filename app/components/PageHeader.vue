<script setup lang="ts">
import type { Section } from '~/composables/useDevice'
// Sayfa başlığı + o sayfanın bölümleri için "yazılmadı" göstergesi, Geri al ve Uygula.
const props = defineProps<{ title: string, sections: Section[] }>()
const { dirty, apply, revert, busy } = useDevice()
const pending = computed(() => props.sections.filter(dirty))

async function applyAll() {
  for (const s of pending.value) await apply(s)
}
</script>

<template>
  <div class="mb-5 flex items-center gap-3">
    <h1 class="text-xl font-semibold text-highlighted">
      {{ title }}
    </h1>
    <slot />
    <span class="flex-1" />
    <template v-if="pending.length">
      <span class="flex items-center gap-1.5 text-sm text-warning">
        <span class="size-2 rounded-full bg-warning" />{{ t('Klavyeye yazılmadı') }}
      </span>
      <UButton
        color="neutral"
        variant="ghost"
        icon="i-lucide-undo-2"
        :label="t('Geri al')"
        :disabled="busy"
        @click="pending.forEach(revert)"
      />
    </template>
    <UButton
      icon="i-lucide-upload"
      :label="t('Uygula')"
      :disabled="!pending.length"
      :loading="busy"
      @click="applyAll"
    />
  </div>
</template>
