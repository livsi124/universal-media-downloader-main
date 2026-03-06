<template>
  <div class="app-layout" :data-theme="settingsStore.settings.theme">

    <!-- Top Bar -->
    <div class="top-bar">
      <input
        id="url-input"
        class="url-input"
        type="text"
        v-model="urlInput"
        :placeholder="t('enter_link_and_press_add')"
        @keyup.enter="addLink"
        @drop.prevent="onDrop"
        @dragover.prevent
      />
      <button class="btn btn-icon" :title="t('add_link')" @click="addLink">➕</button>
      <button class="btn btn-icon" :title="t('load_from_file')" @click="openFile">📁</button>
    </div>

    <div class="content-wrapper">
      <!-- Sidebar Navigation -->
      <nav class="nav-sidebar">
        <button class="nav-btn" :class="{active: tab === 'downloads'}" @click="tab='downloads'">
          <span class="nav-icon">⬇️</span> {{ t('loader_tab_title') }}
        </button>
        <button class="nav-btn" :class="{active: tab === 'history'}" @click="tab='history'">
          <span class="nav-icon">🕐</span> {{ t('history') }}
        </button>
        <button class="nav-btn" :class="{active: tab === 'settings'}" @click="tab='settings'">
          <span class="nav-icon">⚙️</span> {{ t('settings') }}
        </button>
        <button class="nav-btn" :class="{active: tab === 'about'}" @click="tab='about'">
          <span class="nav-icon">ℹ️</span> {{ t('about') }}
        </button>

        <div class="nav-spacer"></div>

        <!-- Language -->
        <select class="select-native" style="margin-bottom:8px" v-model="settingsStore.settings.language" @change="onLanguageChange">
          <option value="en">English</option>
          <option value="ru">Русский</option>
          <option value="uk">Українська</option>
        </select>

        <!-- Theme -->
        <select class="select-native" v-model="settingsStore.settings.theme" @change="settingsStore.save()">
          <option value="dark">Dark</option>
          <option value="light">Light</option>
        </select>
      </nav>

      <!-- Page Content -->
      <div class="page-area">
        <DownloadsPage v-if="tab === 'downloads'" @switch-to-settings="tab = 'settings'" />
        <HistoryPage v-else-if="tab === 'history'" @redownload="onRedownload" />
        <SettingsPage v-else-if="tab === 'settings'" />
        <AboutPage v-else-if="tab === 'about'" />
      </div>
    </div>

    <!-- Bottom Bar -->
    <div class="bottom-bar">
      <button class="btn btn-primary" @click="downloadAll" :disabled="downloadStore.isDownloading">
        ⬇ {{ t('download_all') }}
      </button>
      <button class="btn btn-stop" @click="stopAll" :disabled="!downloadStore.isDownloading">
        ■ {{ t('stop') }}
      </button>
      <button class="btn btn-secondary" @click="clearCompleted">
        🗑 {{ t('clear_completed') }}
      </button>
      <span class="threads-text" v-if="downloadStore.isDownloading">
        {{ downloadStore.activeProcesses }}/{{ downloadStore.maxProcesses }}
      </span>
      <button class="btn btn-icon" :title="t('open_save_folder')" @click="openSaveFolder">📂</button>
      <button class="btn btn-icon" :title="t('open_logs')" @click="openLogs">🧾</button>
      <div class="spacer"></div>
      <span class="summary-text">{{ downloadStore.summaryText }}</span>
      <span class="status-text">{{ downloadStore.statusMessage || t('waiting') }}</span>
    </div>

    <!-- Notifications -->
    <div class="notifications">
      <div v-for="n in notifications" :key="n.id" class="notification" :class="n.type">
        <span>{{ n.type === 'success' ? '✅' : n.type === 'error' ? '❌' : '⚠️' }}</span>
        <span>{{ n.text }}</span>
      </div>
    </div>

  </div>
</template>

<script setup>
import { ref, onMounted, provide } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { readTextFile } from '@tauri-apps/plugin-fs'
import { useDownloadStore, useSettingsStore } from './stores/index.js'
import { useI18n } from './i18n/index.js'
import DownloadsPage from './components/DownloadsPage.vue'
import HistoryPage from './components/HistoryPage.vue'
import SettingsPage from './components/SettingsPage.vue'
import AboutPage from './components/AboutPage.vue'

const { t, setLocale } = useI18n()
const downloadStore = useDownloadStore()
const settingsStore = useSettingsStore()

provide('i18n', { t, setLocale })
provide('notify', notify)

const tab = ref('downloads')
const urlInput = ref('')
const notifications = ref([])

function notify(text, type = 'info', duration = 3000) {
  const id = Date.now()
  notifications.value.push({ id, text, type })
  setTimeout(() => {
    const idx = notifications.value.findIndex(n => n.id === id)
    if (idx !== -1) notifications.value.splice(idx, 1)
  }, duration)
}

function addLink() {
  const url = urlInput.value.trim()
  if (!url) {
    notify(t('enter_link'), 'warning')
    return
  }
  const urls = url.split('\n').map(u => u.trim()).filter(Boolean)
  downloadStore.addUrls(urls)
  for (const u of urls) settingsStore.addRecentUrl(u)
  urlInput.value = ''
}

async function openFile() {
  try {
    const selected = await openDialog({
      filters: [{ name: 'Text', extensions: ['txt'] }],
      multiple: false,
    })
    if (!selected) return
    const content = await readTextFile(selected)
    const urls = content.split('\n').map(u => u.trim()).filter(Boolean)
    if (urls.length === 0) {
      notify(t('file_empty_or_invalid'), 'warning')
      return
    }
    downloadStore.addUrls(urls)
    for (const u of urls) settingsStore.addRecentUrl(u)
  } catch (err) {
    notify(t('error_reading_file') + ': ' + err, 'error')
  }
}

function onDrop(e) {
  const text = e.dataTransfer.getData('text/plain')
  if (text) {
    const urls = text.split('\n').map(u => u.trim()).filter(Boolean)
    downloadStore.addUrls(urls)
    for (const u of urls) settingsStore.addRecentUrl(u)
  }
}

async function downloadAll() {
  await downloadStore.downloadAll(settingsStore.settings)
}

function stopAll() {
  downloadStore.stopAll()
}

function clearCompleted() {
  downloadStore.clearCompleted()
}

async function openSaveFolder() {
  let path = settingsStore.settings.save_path
  if (!path) {
    path = await invoke('get_default_download_path')
  }
  await invoke('open_folder', { path })
}

async function openLogs() {
  const path = await invoke('get_logs_path')
  await invoke('open_folder', { path })
}

function onLanguageChange() {
  setLocale(settingsStore.settings.language)
  settingsStore.save()
}

function onRedownload(url) {
  downloadStore.addUrls([url])
  settingsStore.addRecentUrl(url)
  tab.value = 'downloads'
}

onMounted(async () => {
  await settingsStore.load()
  await downloadStore.init()
  setLocale(settingsStore.settings.language)

  // Check yt-dlp on startup
  try {
    await invoke('check_ytdlp')
  } catch (_) {
    notify('yt-dlp не найден. Установите его для работы приложения.', 'error', 8000)
  }
})
</script>
