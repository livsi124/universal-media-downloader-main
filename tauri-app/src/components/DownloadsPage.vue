<template>
  <div class="downloads-page">
    <!-- Empty State -->
    <div v-if="downloadStore.tasks.length === 0" class="empty-state">
      <div class="empty-card">
        <div class="empty-card-header">
          <span class="empty-icon">🚀</span>
          <span class="empty-title">{{ t('no_downloads_placeholder') }}</span>
        </div>
        <ul class="empty-tips">
          <li>{{ t('empty_tip_dragdrop') }}</li>
          <li>{{ t('empty_tip_paste') }}</li>
          <li>{{ t('empty_tip_support') }}</li>
        </ul>
        <div class="quick-actions">
          <button class="btn btn-secondary" @click="pasteFromClipboard">
            📋 {{ t('paste_from_clipboard') }}
          </button>
          <button class="btn btn-secondary" @click="emit('switch-to-settings')">
            ⚙️ {{ t('open_quality_settings') }}
          </button>
        </div>
        <div class="recent-section" v-if="settingsStore.settings.recent_urls?.length">
          <div class="recent-label">
            {{ t('recent') }}
            <button class="btn btn-icon" style="width:22px;height:22px;font-size:12px;border-radius:4px"
              @click="clearRecent" :title="t('clear_history')">🗑</button>
          </div>
          <div class="recent-chips">
            <div
              v-for="url in settingsStore.settings.recent_urls"
              :key="url"
              class="chip"
              :title="url"
              @click="useRecent(url)"
            >
              {{ truncate(url, 50) }}
            </div>
          </div>
        </div>
        <div style="margin-top:16px; font-size:12px; color:var(--text-muted); text-align:center;">
          {{ t('empty_hint') }}
        </div>
      </div>
    </div>

    <!-- Downloads List -->
    <TransitionGroup v-else name="list" tag="div" class="downloads-scroll">
      <DownloadItem
        v-for="task in downloadStore.tasks"
        :key="task.id"
        :task="task"
        @remove="downloadStore.removeTask(task.id)"
        @retry="downloadStore.retryTask(task)"
        @copy-link="copyLink(task.url)"
        @open-folder="openFolder"
        @save-history="saveHistory"
      />
    </TransitionGroup>
  </div>
</template>

<script setup>
import { inject } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager'
import { useDownloadStore, useSettingsStore } from '../stores/index.js'
import DownloadItem from './DownloadItem.vue'

const emit = defineEmits(['switch-to-settings'])
const { t } = inject('i18n')
const notify = inject('notify')

const downloadStore = useDownloadStore()
const settingsStore = useSettingsStore()

function truncate(str, max) {
  return str.length <= max ? str : str.slice(0, max - 3) + '...'
}

async function pasteFromClipboard() {
  try {
    const text = await readText()
    if (!text) return
    const urls = text.split('\n').map(u => u.trim()).filter(Boolean)
    if (urls.length) {
      downloadStore.addUrls(urls)
      for (const u of urls) settingsStore.addRecentUrl(u)
    }
  } catch (err) {
    notify('Clipboard error: ' + err, 'error')
  }
}

async function clearRecent() {
  await settingsStore.clearRecent()
}

function useRecent(url) {
  downloadStore.addUrls([url])
  settingsStore.addRecentUrl(url)
}

async function copyLink(url) {
  await writeText(url)
  notify(t('copy_link') + ' ✓', 'success', 2000)
}

async function openFolder() {
  let path = settingsStore.settings.save_path
  if (!path) path = await invoke('get_default_download_path')
  await invoke('open_folder', { path })
}

async function saveHistory(task) {
  try {
    await invoke('add_history_entry', {
      url: task.url,
      title: task.title,
      platform: task.platform,
      status: task.status,
      filePath: task.filePath || null,
    })
  } catch (_) {}
}
</script>
