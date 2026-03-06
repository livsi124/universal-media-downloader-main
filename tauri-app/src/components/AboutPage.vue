<template>
  <div class="about-page">
    <div class="about-header">
      <img src="/icon.png" alt="Logo" class="about-logo" />
      <div class="about-info">
        <div class="about-title">{{ t('app_title') }}</div>
        <div class="about-version">{{ t('version') }}</div>
        <div class="about-author">{{ t('author') }}</div>
        <div class="about-desc">{{ t('description') }}</div>
      </div>
    </div>

    <div class="about-divider"></div>

    <!-- System Info -->
    <div class="settings-group" style="max-width:400px;">
      <div class="settings-group-header">System</div>
      <div class="settings-row">
        <span class="settings-label">yt-dlp</span>
        <span class="status-text">{{ ytdlpVersion || '...' }}</span>
      </div>
      <div class="settings-row">
        <span class="settings-label">FFmpeg</span>
        <span class="status-text">{{ ffmpegVersion || '...' }}</span>
      </div>
      <div class="settings-row">
        <span class="settings-label">Deno</span>
        <span class="status-text">{{ denoInstalled ? '✅ Installed' : '❌ Not found' }}</span>
      </div>
      <div class="settings-row">
        <span class="settings-label">yt-dlp update</span>
        <div class="settings-value">
          <button class="btn btn-secondary" @click="checkUpdate" :disabled="checking">
            {{ checking ? '...' : '🔄 Check' }}
          </button>
          <button v-if="hasUpdate" class="btn btn-primary" @click="doUpdate" :disabled="updating" style="font-size:12px;height:28px;padding:0 10px;">
            {{ updating ? '...' : t('update_now') }}
          </button>
        </div>
      <div class="settings-row">
        <span class="settings-label">Diagnostics</span>
        <button class="btn btn-secondary" style="font-size:12px; height:28px; padding:0 10px;" @click="showDiagnostics">Show Paths</button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, inject } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const { t } = inject('i18n')
const notify = inject('notify')

const ytdlpVersion = ref('')
const ffmpegVersion = ref('')
const denoInstalled = ref(false)
const checking = ref(false)
const updating = ref(false)
const hasUpdate = ref(false)

async function checkUpdate() {
  checking.value = true
  try {
    const result = await invoke('check_ytdlp_update')
    hasUpdate.value = result.has_update
    if (result.has_update) {
      notify(`yt-dlp: ${result.current} → ${result.latest}`, 'warning', 5000)
    } else {
      notify(t('ytdlp_up_to_date'), 'success')
    }
  } catch (err) {
    notify('Check failed: ' + err, 'error')
  } finally {
    checking.value = false
  }
}

async function doUpdate() {
  updating.value = true
  try {
    await invoke('update_ytdlp')
    notify(t('update_complete'), 'success', 5000)
    hasUpdate.value = false
    // Refresh version
    const v = await invoke('check_ytdlp')
    ytdlpVersion.value = v
  } catch (err) {
    notify(t('update_failed') + ': ' + err, 'error', 5000)
  } finally {
    updating.value = false
  }
}

onMounted(async () => {
  try { ytdlpVersion.value = await invoke('check_ytdlp') } catch (_) { ytdlpVersion.value = 'Not found' }
  try { ffmpegVersion.value = await invoke('check_ffmpeg') } catch (_) { ffmpegVersion.value = 'Not found' }
  try { denoInstalled.value = await invoke('check_deno') } catch (_) {}
})

async function showDiagnostics() {
  try {
    const deps = await invoke('debug_deps')
    alert(deps)
  } catch (err) {
    alert('Failed to get diagnostics: ' + err)
  }
}
</script>
