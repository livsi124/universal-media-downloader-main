<template>
  <div class="settings-page">

    <!-- General Settings -->
    <div class="settings-group">
      <div class="settings-group-header">{{ t('general_settings') }}</div>

      <div class="settings-row">
        <span class="settings-label">{{ t('select_theme') }}</span>
        <div class="settings-value">
          <select class="select-native" v-model="settings.theme" @change="onThemeChange" style="width:120px">
            <option value="dark">Dark</option>
            <option value="light">Light</option>
          </select>
        </div>
      </div>

      <div class="settings-row">
        <span class="settings-label">{{ t('parallel_downloads') }}</span>
        <div class="settings-value">
          <input type="number" class="number-input" v-model.number="settings.parallel_downloads" min="1" max="10" @change="saveSettings" />
        </div>
      </div>
    </div>

    <!-- Download Settings -->
    <div class="settings-group">
      <div class="settings-group-header">{{ t('download_settings') }}</div>

      <div class="settings-row">
        <span class="settings-label">{{ t('select_save_folder') }}</span>
        <div class="settings-value">
          <button class="btn btn-secondary" @click="selectSaveFolder">📁 {{ t('select_save_folder') }}</button>
          <span class="path-display" :title="settings.save_path || t('folder_not_selected')">
            {{ settings.save_path || t('folder_not_selected') }}
          </span>
        </div>
      </div>

      <div class="settings-row">
        <span class="settings-label">{{ t('download_subtitles') }}</span>
        <label class="toggle-switch">
          <input type="checkbox" v-model="settings.subtitles_enabled" @change="saveSettings" />
          <span class="toggle-slider"></span>
        </label>
      </div>

      <div class="settings-row" style="flex-direction:column; align-items:flex-start; gap:10px;">
        <div style="display:flex; align-items:center; justify-content:space-between; width:100%;">
          <span class="settings-label">{{ t('use_cookies') }}</span>
          <label class="toggle-switch">
            <input type="checkbox" v-model="settings.use_cookies" @change="saveSettings" />
            <span class="toggle-slider"></span>
          </label>
        </div>

        <div v-if="settings.use_cookies" class="cookie-options">
          <label class="radio-row">
            <input type="radio" name="cookie-type" value="file" v-model="settings.cookie_source_type" @change="saveSettings" />
            {{ t('cookie_file') }}
          </label>
          <div v-if="settings.cookie_source_type === 'file'" class="sub-row">
            <button class="btn btn-secondary" @click="selectCookieFile">{{ t('select_cookies_file') }}</button>
            <span class="path-display" :title="settings.cookies_path || t('file_not_selected')">
              {{ settings.cookies_path || t('file_not_selected') }}
            </span>
          </div>

          <label class="radio-row">
            <input type="radio" name="cookie-type" value="browser" v-model="settings.cookie_source_type" @change="saveSettings" />
            {{ t('cookie_browser') }}
          </label>
          <div v-if="settings.cookie_source_type === 'browser'" class="sub-row">
            <select class="select-native" style="width:140px" v-model="settings.cookie_browser" @change="saveSettings">
              <option v-for="b in browsers" :key="b" :value="b">{{ b === 'none' ? 'None' : capitalize(b) }}</option>
            </select>
          </div>
        </div>
      </div>
    </div>

    <!-- Quality Settings -->
    <div class="settings-group">
      <div class="settings-group-header">{{ t('quality_settings') }}</div>
      <div class="quality-grid">
        <div v-for="p in platforms" :key="p.key" class="quality-item">
          <div class="quality-platform-label">
            <img
              v-if="p.logo"
              :src="p.logo"
              class="platform-logo"
              :alt="p.name"
              @error="(e) => e.target.style.display='none'"
            />
            <span>{{ p.name }}</span>
          </div>
          <select class="select-native" v-model="settings.quality_settings[p.key]" @change="saveSettings">
            <template v-if="p.name === 'YouTube'">
              <option value="bestvideo+bestaudio/best">{{ t('video_best_quality') }}</option>
              <option value="bestaudio/best">{{ t('audio_only') }}</option>
              <option value="bestvideo[height<=144]+bestaudio/best">144p</option>
              <option value="bestvideo[height<=240]+bestaudio/best">240p</option>
              <option value="bestvideo[height<=360]+bestaudio/best">360p</option>
              <option value="bestvideo[height<=480]+bestaudio/best">480p</option>
              <option value="bestvideo[height<=720]+bestaudio/best">720p (HD)</option>
              <option value="bestvideo[height<=1080]+bestaudio/best">1080p (Full HD)</option>
              <option value="bestvideo[height<=1440]+bestaudio/best">1440p (2K)</option>
              <option value="bestvideo[height<=2160]+bestaudio/best">2160p (4K)</option>
            </template>
            <template v-else>
              <option value="best">{{ t('best_quality') }}</option>
              <option value="bestaudio/best">{{ t('audio_only') }}</option>
              <option value="video_only_stripped">{{ t('video_only') }}</option>
              <option value="worst">{{ t('worst_quality') }}</option>
            </template>
          </select>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, ref, onMounted, inject } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { useSettingsStore } from '../stores/index.js'

import youtubeImg from '../assets/logos/youtube.png'
import rutubeImg from '../assets/logos/rutube.png'
import tiktokImg from '../assets/logos/tiktok.png'
import instagramImg from '../assets/logos/instagram.png'
import vkImg from '../assets/logos/vk.png'
import pornhubImg from '../assets/logos/pornhub.png'
import facebookImg from '../assets/logos/facebook.png'
import xImg from '../assets/logos/x_(twitter).png'
import kinopoiskImg from '../assets/logos/kinopoisk.png'
import twitchImg from '../assets/logos/twitch.png'
import kickImg from '../assets/logos/kick.png'

const { t } = inject('i18n')
const settingsStore = useSettingsStore()
const settings = computed(() => settingsStore.settings)

const browsers = ref(['none'])

const platforms = [
  { name: 'YouTube', key: 'youtube', logo: youtubeImg },
  { name: 'RuTube', key: 'rutube', logo: rutubeImg },
  { name: 'TikTok', key: 'tiktok', logo: tiktokImg },
  { name: 'Instagram', key: 'instagram', logo: instagramImg },
  { name: 'VK', key: 'vk', logo: vkImg },
  { name: 'PornHub', key: 'pornhub', logo: pornhubImg },
  { name: 'Facebook', key: 'facebook', logo: facebookImg },
  { name: 'X (Twitter)', key: 'x_twitter', logo: xImg },
  { name: 'Kinopoisk', key: 'kinopoisk', logo: kinopoiskImg },
  { name: 'Twitch', key: 'twitch', logo: twitchImg },
  { name: 'Kick', key: 'kick', logo: kickImg },
]

function capitalize(s) { return s.charAt(0).toUpperCase() + s.slice(1) }

function onThemeChange() {
  saveSettings()
}

async function saveSettings() {
  await settingsStore.save()
}

async function selectSaveFolder() {
  try {
    const selected = await openDialog({ directory: true })
    if (selected) {
      settings.value.save_path = selected
      await saveSettings()
    }
  } catch (_) {}
}

async function selectCookieFile() {
  try {
    const selected = await openDialog({
      filters: [{ name: 'Cookies', extensions: ['txt', 'cookies'] }],
    })
    if (selected) {
      settings.value.cookies_path = selected
      await saveSettings()
    }
  } catch (_) {}
}

onMounted(async () => {
  try {
    browsers.value = await invoke('detect_browsers')
  } catch (_) {}

  // Ensure quality settings are initialized
  for (const p of platforms) {
    if (!settings.value.quality_settings[p.key]) {
      settings.value.quality_settings[p.key] = p.name === 'YouTube' ? 'bestvideo+bestaudio/best' : 'best'
    }
  }
})
</script>
