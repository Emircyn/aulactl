// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  modules: [
    '@nuxt/eslint',
    '@nuxt/ui'
  ],

  // Masaüstü uygulaması (Tauri): sunucu yok, statik SPA (frontendDist = .output/public).
  ssr: false,

  devtools: {
    enabled: false
  },

  css: ['~/assets/css/main.css'],

  // AMOLED: yalnızca koyu tema.
  colorMode: {
    preference: 'dark',
    fallback: 'dark'
  },

  compatibilityDate: '2026-06-30',

  eslint: {
    config: {
      stylistic: {
        commaDangle: 'never',
        braceStyle: '1tbs'
      }
    }
  }
})
