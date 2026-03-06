<template>
  <div class="download-item" :class="`status-${task.status}`">

    <!-- Thumbnail -->
    <img
      v-if="task.thumbnail && !thumbError"
      :src="task.thumbnail"
      class="item-thumb"
      :alt="task.title"
      @error="thumbError = true"
    />
    <div v-else class="item-thumb-placeholder">
      <span v-if="task.status === 'fetching_info'" class="spinner"></span>
      <template v-else>{{ platformEmoji(task.platform) }}</template>
    </div>

    <!-- Info -->
    <div class="item-info">
      <div class="item-title" :title="task.title">{{ task.title }}</div>
      <div class="item-url" :title="task.url">{{ task.url }}</div>

      <div class="item-status-row">
        <span class="item-platform">{{ task.platform }}</span>
        <span class="status-badge" :class="task.status">
          {{ statusLabel }}
        </span>
        <span v-if="task.status === 'downloading' && task.speed" class="status-text" style="font-size:11px">
          {{ task.speed }} • ETA {{ task.eta }}
        </span>
      </div>

      <!-- Progress -->
      <div v-if="showProgress" class="item-progress">
        <div class="progress-bar-wrap">
          <div
            class="progress-bar-fill"
            :class="task.status"
            :style="{ width: task.progress + '%' }"
          ></div>
        </div>
        <div class="progress-text" v-if="task.status === 'downloading'">
          {{ Math.round(task.progress) }}%
        </div>
        <div class="progress-text error" v-if="task.status === 'error'" style="color:var(--error)">
          {{ task.errorMessage }}
        </div>
      </div>
    </div>

    <!-- Actions -->
    <div class="item-actions">
      <button
        v-if="task.status === 'error' || task.status === 'stopped'"
        class="item-action-btn"
        :title="t('retry')"
        @click="emit('retry')"
      >🔄</button>

      <button
        v-if="task.status === 'completed'"
        class="item-action-btn"
        :title="t('open_save_folder')"
        @click="emit('open-folder')"
      >📂</button>

      <button
        class="item-action-btn"
        :title="t('copy_link')"
        @click="emit('copy-link')"
      >🔗</button>

      <button
        class="item-action-btn remove"
        :title="t('remove_from_list')"
        @click="emit('remove')"
      >✕</button>
    </div>
  </div>
</template>

<script setup>
import { computed, watch, inject, ref } from 'vue'
const { t } = inject('i18n')

const props = defineProps({
  task: { type: Object, required: true }
})

const emit = defineEmits(['remove', 'retry', 'copy-link', 'open-folder', 'save-history'])

const thumbError = ref(false)

const statusLabels = {
  fetching_info: () => t('status_fetching_info'),
  pending: () => t('status_pending'),
  downloading: () => t('status_downloading'),
  processing: () => t('status_processing'),
  completed: () => t('status_completed'),
  error: () => t('status_error'),
  stopped: () => t('status_stopped'),
}

const statusLabel = computed(() => (statusLabels[props.task.status] || (() => props.task.status))())

const showProgress = computed(() =>
  ['downloading', 'processing', 'completed', 'error', 'stopped'].includes(props.task.status)
)

function platformEmoji(platform) {
  const map = {
    YouTube: '▶', TikTok: '🎵', Instagram: '📷', VK: '🔷',
    RuTube: '🎬', Twitch: '🟣', Kick: '🟢', Facebook: '💙',
    Twitter: '🐦', X: '✖'
  }
  return map[platform] || '🎥'
}

// Save to history when completed / errored
watch(() => props.task.status, (newStatus) => {
  if (['completed', 'error', 'stopped'].includes(newStatus)) {
    emit('save-history', props.task)
  }
})
</script>
