<template>
  <div class="history-page">
    <div class="page-title">{{ t('history_title') }}</div>

    <div class="history-toolbar">
      <input
        class="search-input"
        type="text"
        v-model="searchQuery"
        :placeholder="t('search_history')"
        @input="onSearch"
      />
      <button class="btn btn-secondary" @click="loadHistory">🔄 {{ t('refresh') }}</button>
      <button class="btn btn-danger" @click="confirmClear">🗑 {{ t('clear_all_history') }}</button>
    </div>

    <div class="history-table-wrap">
      <table class="history-table">
        <thead>
          <tr>
            <th>{{ t('history_date') }}</th>
            <th>{{ t('history_title') }}</th>
            <th>{{ t('history_platform') }}</th>
            <th>{{ t('history_status') }}</th>
            <th>{{ t('history_actions') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="entry in filteredEntries" :key="entry.id">
            <td class="muted">{{ formatDate(entry.date) }}</td>
            <td :title="entry.title">{{ truncate(entry.title, 50) }}</td>
            <td class="muted">{{ entry.platform }}</td>
            <td>
              <span class="td-status" :class="entry.status">
                {{ t('status_' + entry.status, entry.status) }}
              </span>
            </td>
            <td>
              <div class="td-actions">
                <button class="item-action-btn" :title="t('redownload')" @click="emit('redownload', entry.url)">🔄</button>
                <button class="item-action-btn" :title="t('copy_link')" @click="copyLink(entry.url)">🔗</button>
                <button v-if="entry.file_path" class="item-action-btn" :title="t('open_file')" @click="openFile(entry.file_path)">📂</button>
                <button class="item-action-btn remove" :title="t('remove_from_history')" @click="removeEntry(entry.id)">✕</button>
              </div>
            </td>
          </tr>
          <tr v-if="filteredEntries.length === 0">
            <td colspan="5" style="text-align:center; color:var(--text-muted); padding:32px;">
              {{ searchQuery ? 'Ничего не найдено' : 'История пуста' }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <div class="history-stats">
      {{ statsText }}
    </div>

    <!-- Confirm Clear Modal -->
    <div v-if="showClearModal" class="modal-overlay" @click.self="showClearModal = false">
      <div class="modal">
        <div class="modal-title">{{ t('confirm') }}</div>
        <div class="modal-body">{{ t('clear_history_confirm') }}</div>
        <div class="modal-actions">
          <button class="btn btn-secondary" @click="showClearModal = false">❌ {{ t('stop') }}</button>
          <button class="btn btn-danger" @click="doClear">🗑 {{ t('clear_all_history') }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, inject } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { writeText } from '@tauri-apps/plugin-clipboard-manager'

const { t } = inject('i18n')
const notify = inject('notify')
const emit = defineEmits(['redownload'])

const entries = ref([])
const searchQuery = ref('')
const showClearModal = ref(false)

const filteredEntries = computed(() => {
  if (!searchQuery.value.trim()) return entries.value
  const q = searchQuery.value.toLowerCase()
  return entries.value.filter(e =>
    (e.title || '').toLowerCase().includes(q) ||
    (e.url || '').toLowerCase().includes(q)
  )
})

const statsText = computed(() => {
  const total = entries.value.length
  const completed = entries.value.filter(e => e.status === 'completed').length
  const errors = entries.value.filter(e => e.status === 'error').length
  return t('history_stats', `Total: ${total} | Completed: ${completed} | Errors: ${errors}`)
    .replace('{total}', total)
    .replace('{completed}', completed)
    .replace('{errors}', errors)
})

function formatDate(dateStr) {
  try {
    const d = new Date(dateStr)
    return d.toLocaleString('ru-RU', { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' })
  } catch { return dateStr?.substring(0, 16) || '' }
}

function truncate(str, max) {
  if (!str) return ''
  return str.length <= max ? str : str.slice(0, max - 3) + '...'
}

async function loadHistory() {
  try {
    entries.value = await invoke('get_history')
  } catch (err) {
    console.error('Failed to load history:', err)
  }
}

function onSearch() { /* reactive via computed */ }

async function copyLink(url) {
  await writeText(url)
  notify(t('copy_link') + ' ✓', 'success', 2000)
}

async function openFile(path) {
  try { await invoke('open_file', { path }) } catch (_) {}
}

async function removeEntry(id) {
  try {
    await invoke('remove_history_entry', { id })
    await loadHistory()
  } catch (_) {}
}

function confirmClear() { showClearModal.value = true }

async function doClear() {
  showClearModal.value = false
  try {
    await invoke('clear_history')
    await loadHistory()
  } catch (_) {}
}

onMounted(loadHistory)
</script>
