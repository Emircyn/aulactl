<script setup lang="ts">
// Renk kutusu + açılır renk seçici (hex). v-model: "#RRGGBB"
const model = defineModel<string>({ required: true })
const hex = computed({
  get: () => model.value,
  set: v => /^#[0-9a-f]{6}$/i.test(v) && (model.value = v.toUpperCase())
})
</script>

<template>
  <UPopover>
    <slot>
      <button
        class="size-9 rounded-md border border-accented"
        :style="{ background: model }"
        :aria-label="t('Renk {c}', { c: model })"
      />
    </slot>
    <template #content>
      <div class="space-y-3 p-3">
        <UColorPicker v-model="hex" />
        <UInput
          :model-value="model"
          size="sm"
          class="w-full font-mono"
          @update:model-value="hex = String($event)"
        />
      </div>
    </template>
  </UPopover>
</template>
