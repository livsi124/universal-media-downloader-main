import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

let progressUnlisten = null

export const useDownloadStore = defineStore('downloads', {
    state: () => ({
        tasks: [],
        isDownloading: false,
        statusMessage: '',
        activeProcesses: 0,
        maxProcesses: 2,
    }),

    getters: {
        completedTasks: (state) => state.tasks.filter(t => ['completed', 'error', 'stopped'].includes(t.status)),
        pendingTasks: (state) => state.tasks.filter(t => t.status === 'pending'),
        summaryText: (state) => {
            const total = state.tasks.length
            const done = state.tasks.filter(t => t.status === 'completed').length
            const errs = state.tasks.filter(t => t.status === 'error').length
            return total > 0 ? `${done}/${total} ✓ • ${errs} ⚠` : ''
        }
    },

    actions: {
        async init() {
            // Listen to download progress events from Rust
            if (progressUnlisten) {
                progressUnlisten()
            }
            progressUnlisten = await listen('download-progress', (event) => {
                this.onProgress(event.payload)
            })
        },

        onProgress(payload) {
            const idx = this.tasks.findIndex(t => t.id === payload.task_id)
            if (idx === -1) return

            // Force Vue array reactivity
            this.tasks.splice(idx, 1, {
                ...this.tasks[idx],
                status: payload.status,
                progress: payload.percent,
                speed: payload.speed || '',
                eta: payload.eta || ''
            });

            if (payload.status === 'completed') {
                this.onTaskCompleted(this.tasks[idx])
            } else if (payload.status === 'error') {
                this.tasks.splice(idx, 1, {
                    ...this.tasks[idx],
                    errorMessage: payload.message || 'Download failed'
                });
                this.onTaskCompleted(this.tasks[idx])
            }
        },

        onTaskCompleted(task) {
            this.activeProcesses = Math.max(0, this.activeProcesses - 1)
            // Start next pending task
            const next = this.tasks.find(t => t.status === 'pending' && !t._started)
            if (next) {
                this.startTask(next)
            } else if (this.activeProcesses === 0) {
                this.isDownloading = false
            }
        },

        normalizeUrl(url) {
            const kickMatch = url.match(/https?:\/\/(?:www\.)?kick\.com\/[^/]+\/videos\/([0-9a-fA-F-]{6,})/)
            if (kickMatch) return `https://kick.com/video/${kickMatch[1]}`
            return url.trim()
        },

        generateId() {
            return Date.now().toString(36) + Math.random().toString(36).substr(2)
        },

        addUrls(urls) {
            const newTasks = []
            for (const rawUrl of urls) {
                const url = this.normalizeUrl(rawUrl)
                if (!url) continue
                const task = {
                    id: this.generateId(),
                    url,
                    title: '...',
                    platform: 'Unknown',
                    thumbnail: null,
                    status: 'fetching_info',
                    progress: 0,
                    speed: '',
                    eta: '',
                    errorMessage: '',
                    _started: false,
                }
                this.tasks.push(task)
                newTasks.push(task)
                this.fetchInfo(task.id)
            }
            return newTasks
        },

        async fetchInfo(taskId) {
            const initialTask = this.tasks.find(t => t.id === taskId);
            if (!initialTask) return;
            const url = initialTask.url;

            try {
                const info = await invoke('fetch_video_info', { url: url, taskId: taskId })
                const idx = this.tasks.findIndex(t => t.id === taskId);
                if (idx !== -1) {
                    this.tasks.splice(idx, 1, {
                        ...this.tasks[idx],
                        title: info.title,
                        platform: info.platform,
                        thumbnail: info.thumbnail,
                        status: 'pending'
                    });
                }
            } catch (err) {
                const idx = this.tasks.findIndex(t => t.id === taskId);
                if (idx !== -1) {
                    this.tasks.splice(idx, 1, {
                        ...this.tasks[idx],
                        status: 'error',
                        errorMessage: String(err)
                    });
                }
            }
        },

        async downloadAll(settings) {
            if (this.isDownloading) return
            const pending = this.tasks.filter(t => t.status === 'pending')
            if (pending.length === 0) return

            this.isDownloading = true
            this.maxProcesses = settings.parallel_downloads || 2
            this.activeProcesses = 0

            // Start up to maxProcesses tasks
            const toStart = pending.slice(0, this.maxProcesses)
            for (const task of toStart) {
                await this.startTask(task)
            }
        },

        async startTask(task, settings) {
            if (task._started && task.status === 'downloading') return

            task._started = true
            task.status = 'downloading'
            task.progress = 0
            this.activeProcesses++

            const store = useSettingsStore()
            const platform = task.platform.toLowerCase().replace(/\s/g, '_').replace(/[()]/g, '')
            const format = store.settings.quality_settings?.[platform] || 'bestvideo+bestaudio/best'

            try {
                await invoke('start_download', {
                    taskId: task.id,
                    url: task.url,
                    format,
                })
            } catch (err) {
                task.status = 'error'
                task.errorMessage = String(err)
                this.onTaskCompleted(task)
            }
        },

        async retryTask(task) {
            task.status = 'pending'
            task._started = false
            task.progress = 0
            task.errorMessage = ''
            if (!this.isDownloading) {
                await this.downloadAll(useSettingsStore().settings)
            } else {
                await this.startTask(task)
            }
        },

        stopAll() {
            for (const task of this.tasks) {
                if (['downloading', 'fetching_info', 'pending', 'processing'].includes(task.status)) {
                    task.status = 'stopped'
                    task._started = false
                }
            }
            this.isDownloading = false
            this.activeProcesses = 0
        },

        removeTask(taskId) {
            const idx = this.tasks.findIndex(t => t.id === taskId)
            if (idx !== -1) {
                this.tasks.splice(idx, 1)
            }
        },

        clearCompleted() {
            this.tasks = this.tasks.filter(t => !['completed', 'error', 'stopped'].includes(t.status))
        },
    }
})

export const useSettingsStore = defineStore('settings', {
    state: () => ({
        settings: {
            theme: 'dark',
            language: 'ru',
            save_path: null,
            parallel_downloads: 2,
            subtitles_enabled: false,
            use_cookies: false,
            cookie_source_type: 'file',
            cookies_path: null,
            cookie_browser: null,
            quality_settings: {},
            recent_urls: [],
            skipped_ytdlp_version: null,
        },
        loaded: false,
    }),

    actions: {
        async load() {
            try {
                this.settings = await invoke('get_settings')
                this.loaded = true
            } catch (err) {
                console.error('Failed to load settings:', err)
                this.loaded = true
            }
        },

        async save() {
            try {
                await invoke('save_settings', { settings: this.settings })
            } catch (err) {
                console.error('Failed to save settings:', err)
            }
        },

        async addRecentUrl(url) {
            const recent = this.settings.recent_urls.filter(u => u !== url)
            recent.unshift(url)
            this.settings.recent_urls = recent.slice(0, 5)
            await this.save()
        },

        async clearRecent() {
            this.settings.recent_urls = []
            await this.save()
        }
    }
})
