<script setup lang="ts">
import type { Prefs } from '~/types/device'
import layout from '~~/layout.json'

// Resmi uygulamanın ayar paneli (OemDrv FUN_0045e6b0): uyku → LED bloğu 0x18, debounce → 0x03.
const { config } = useDevice()
const led = computed(() => config.value!.led)

const sleepItems = [
  { label: t('Kapalı'), value: 0 },
  ...layout.sleepSeconds.map(s => ({ label: t('{n} dk', { n: (s / 60).toLocaleString(lang === 'tr' ? 'tr-TR' : 'en-US') }), value: s }))
]
// KB.ini ShowDebounce=0x04000101 → cihaz değeri 0-4; resmi uygulama değer 1 için "2 Stage" gösteriyor
const debounceItems = [0, 1, 2, 3, 4].map(v => ({ label: t('{n}. kademe', { n: v + 1 }), value: v }))

// Ek özellikler (resmi sürücüde yok): uygulamanın kendi ayarı, değişince hemen kaydedilir; klavyeye yazılmaz.
const toast = useToast()
const prefs = ref<Prefs | null>(null)
onMounted(async () => (prefs.value = await call<Prefs>('get_prefs')))
watch(prefs, p => p && call('set_prefs', { prefs: p }), { deep: true })
const capsHex = computed({
  get: () => toHex(prefs.value?.caps_color ?? [255, 255, 255]),
  set: v => prefs.value && (prefs.value.caps_color = fromHex(v))
})
// firmware: sitedeki resmi güncelleyicinin sürümü klavyedekinden yeniyse onu açar (önyükleyicinin yazma hakkı sınırlı)
const fw = ref<{ current: number | null, latest: number, newer: boolean } | null>(null)
const fwBusy = ref(false)
// durum çubuğundaki gibi onaltılık haneler: 0x1100 → V1.1.0.0
const ver = (v: number | null) => v == null ? t('bilinmiyor') : `V${[...v.toString(16).toUpperCase()].join('.')}`
async function fwCall(cmd: 'firmware_check' | 'firmware_update', silent = false) {
  fwBusy.value = true
  try {
    if (cmd === 'firmware_check') fw.value = await call('firmware_check')
    else {
      await call(cmd)
      toast.add({ title: t('Resmi güncelleyici açıldı'), description: t('Klavye kabloyla bağlı olmalı; bitene kadar kabloyu çıkarma.'), color: 'success', icon: 'i-lucide-check' })
    }
  } catch (e) {
    if (!silent) toast.add({ title: 'Firmware', description: String(e), color: 'error', icon: 'i-lucide-x' })
  } finally {
    fwBusy.value = false
  }
}

// sayfa her açıldığında site/ağ hatası toast olmasın; elle "Denetle" hatayı gösterir
onMounted(() => fwCall('firmware_check', true))

// Windows açılışında tepside başlama (kayıt defteri Run anahtarı; ilk açılışta açılır)
const autostart = ref(false)
onMounted(async () => (autostart.value = await call<boolean>('autostart_get')))
async function setAutostart(on: boolean) {
  try {
    await call('autostart_set', { on })
    autostart.value = on
  } catch (e) {
    toast.add({ title: t('Açılışta başlatma'), description: String(e), color: 'error', icon: 'i-lucide-x' })
  }
}

// dil: arayüz localStorage'dan, tepsi/bildirimler prefs.lang'den okur → ikisi de yazılır, sonra pencere yenilenir
async function changeLang(l: Lang) {
  const p = prefs.value ?? await call<Prefs>('get_prefs')
  await call('set_prefs', { prefs: { ...p, lang: l } })
  setLang(l)
}
const musicHex = computed({
  get: () => toHex(prefs.value?.music_color ?? [0, 255, 0]),
  set: v => prefs.value && (prefs.value.music_color = fromHex(v))
})
</script>

<template>
  <div class="flex h-full flex-col">
    <PageHeader
      :title="t('Ayarlar')"
      :sections="['led']"
    />
    <div class="max-w-xl space-y-6">
      <UFormField :label="t('Dil')">
        <USelect
          :model-value="lang"
          :items="[{ label: 'English', value: 'en' }, { label: 'Türkçe', value: 'tr' }]"
          class="w-48"
          @update:model-value="changeLang($event as Lang)"
        />
      </UFormField>

      <UFormField
        :label="t('Uyku süresi')"
        :help="t('Kablosuz modda bu süre boyunca tuşa basılmazsa klavye uykuya geçer.')"
      >
        <USelect
          v-model="led.sleep"
          :items="sleepItems"
          class="w-48"
        />
      </UFormField>

      <UFormField
        :label="t('Tuş sekme önleme (debounce)')"
        :help="t('Bir tuş basışının kaç kez algılanacağını süzer. Yüksek kademe çift basmayı azaltır, tepkiyi biraz geciktirir.')"
      >
        <URadioGroup
          v-model="led.debounce"
          :items="debounceItems"
          orientation="horizontal"
          variant="card"
        />
      </UFormField>

      <div
        v-if="prefs"
        class="space-y-4 border-t border-default pt-6"
      >
        <h2 class="font-semibold text-highlighted">
          {{ t('Bildirimler') }}
        </h2>
        <USwitch
          :model-value="autostart"
          :label="t('Bilgisayar açılınca başlat')"
          :description="t('Uygulama sistem tepsisinde, pencere açılmadan başlar; bildirim ve ışıklar hemen çalışır.')"
          @update:model-value="setAutostart"
        />
        <USwitch
          v-model="prefs.battery_notify"
          :label="t('Pil azalınca bildirim göster')"
          :description="t('Pil %15 ve %5\'e düşünce sistem bildirimi.')"
        />
        <USwitch
          v-model="prefs.battery_flash"
          :label="t('Pil azalınca klavye kırmızı yanıp sönsün')"
        />
        <div class="flex items-center gap-3">
          <USwitch
            v-model="prefs.caps_light"
            :label="t('Caps Lock açıkken o tuşu yak')"
          />
          <ColorEdit v-model="capsHex" />
        </div>
        <h2 class="pt-2 font-semibold text-highlighted">
          {{ t('Müziğe tepki') }}
        </h2>
        <USwitch
          v-model="prefs.music"
          :label="t('Bilgisayarın sesine göre tuşlar yansın')"
          :description="t('Çalan sesin frekansları tuş sütunlarında çubuk olarak görünür (Windows).')"
        />
        <div
          v-if="prefs.music"
          class="grid grid-cols-2 gap-x-8 gap-y-4"
        >
          <UFormField :label="t('Kazanç · {v}', { v: prefs.music_gain.toFixed(1) })">
            <USlider
              v-model="prefs.music_gain"
              :min="0.3"
              :max="3"
              :step="0.1"
            />
          </UFormField>
          <UFormField :label="t('Yumuşaklık · {v}', { v: prefs.music_smooth })">
            <USlider
              v-model="prefs.music_smooth"
              :min="0"
              :max="9"
            />
          </UFormField>
          <div class="col-span-2 flex items-center gap-3">
            <USwitch
              v-model="prefs.music_rainbow"
              :label="t('Gökkuşağı')"
            />
            <ColorEdit
              v-if="!prefs.music_rainbow"
              v-model="musicHex"
            />
          </div>
        </div>

        <p class="text-sm text-muted">
          {{ t('Bildirim ve müzik ışıkları klavye') }} <NuxtLink
            to="/"
            class="text-primary hover:underline"
          >{{ t('Işık') }}</NuxtLink> {{ t('sayfasında "Tuş başına" moddayken çalışır; klavyenin kalıcı belleğine yazmaz.') }}
          {{ t('Uygulama penceresini kapatınca sistem tepsisinde çalışmaya devam eder.') }}
        </p>
      </div>

      <div class="space-y-3 border-t border-default pt-6">
        <h2 class="font-semibold text-highlighted">
          {{ t('Firmware güncellemesi') }}
        </h2>
        <p class="text-sm text-muted">
          {{ t('Uygulama günde bir kez AULA\'nın sitesini kontrol eder, yeni sürüm çıkınca bildirim gösterir.') }}
          {{ t('Klavyenin güncelleme hakkı sınırlı olduğu için yalnız daha yeni bir sürüm varsa güncellenir.') }}
        </p>
        <div class="flex items-center gap-3">
          <UButton
            icon="i-lucide-search"
            :label="t('Şimdi kontrol et')"
            color="neutral"
            variant="soft"
            :loading="fwBusy"
            @click="fwCall('firmware_check')"
          />
          <span
            v-if="fwBusy && !fw"
            class="text-sm text-muted"
          >{{ t('Güncelleme denetleniyor…') }}</span>
          <span
            v-if="fw"
            class="text-sm"
          >
            {{ t('Klavyede {a} · sitede {b} ·', { a: ver(fw.current), b: ver(fw.latest) }) }}
            <span :class="fw.newer ? 'text-warning' : 'text-success'">{{ fw.newer ? t('yeni sürüm var') : fw.current == null ? t('karşılaştırmak için klavyeyi bir kez kabloyla bağla') : t('güncel') }}</span>
          </span>
          <UButton
            v-if="fw?.newer"
            icon="i-lucide-download"
            :label="t('Güncelleyiciyi aç')"
            :loading="fwBusy"
            @click="fwCall('firmware_update')"
          />
        </div>
      </div>

      <p class="text-sm text-muted">
        {{ t('Fabrika ayarına dönme ve yedekleme') }} <NuxtLink
          to="/cihaz"
          class="text-primary hover:underline"
        >{{ t('Cihaz') }}</NuxtLink> {{ t('sayfasında.') }}
      </p>
    </div>
  </div>
</template>
