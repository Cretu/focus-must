<script setup lang="ts">
import { computed, ref } from 'vue'

export interface SessionRecord {
  session_type: string
  started_at: number
  ended_at: number
  duration_secs: number
  task?: string
  whitelist: string[]
}

const props = defineProps<{
  sessions: SessionRecord[]
}>()

type HistoryTab = 'all' | 'focus' | 'break'

const activeTab = ref<HistoryTab>('all')

const tabs: { key: HistoryTab; label: string }[] = [
  { key: 'all', label: '全部' },
  { key: 'focus', label: '专注' },
  { key: 'break', label: '休息' },
]

function pad(value: number) {
  return String(value).padStart(2, '0')
}

function formatTime(ts: number) {
  const d = new Date(ts * 1000)
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

function formatDuration(secs: number) {
  const hours = Math.floor(secs / 3600)
  const mins = Math.floor((secs % 3600) / 60)
  const seconds = secs % 60
  return `${pad(hours)}:${pad(mins)}:${pad(seconds)}`
}

const filteredSessions = computed(() => {
  if (activeTab.value === 'all') {
    return props.sessions
  }
  return props.sessions.filter((session) => session.session_type === activeTab.value)
})

const groupedSessions = computed(() => {
  const groups: Record<string, SessionRecord[]> = {}
  for (const session of filteredSessions.value) {
    const d = new Date(session.started_at * 1000)
    const key = d.toLocaleDateString()
    if (!groups[key]) groups[key] = []
    groups[key].push(session)
  }
  return Object.entries(groups).map(([date, items]) => ({ date, items }))
})
</script>

<template>
  <div class="history-container">
    <h3 class="history-title">最近记录</h3>

    <div class="history-tabs" role="tablist" aria-label="历史记录筛选">
      <button
        v-for="tab in tabs"
        :key="tab.key"
        class="history-tab"
        :class="{ active: activeTab === tab.key }"
        role="tab"
        type="button"
        @click="activeTab = tab.key"
      >
        {{ tab.label }}
      </button>
    </div>
    
    <div v-if="filteredSessions.length === 0" class="empty-state">
      <div class="empty-icon">📝</div>
      <p>暂无记录</p>
    </div>

    <div v-else class="history-list">
      <div v-for="group in groupedSessions" :key="group.date" class="history-group">
        <div class="date-header">{{ group.date }}</div>
        <div v-for="(session, index) in group.items" :key="index" class="history-item">
          <div class="history-icon" :class="session.session_type">
            {{ session.session_type === 'focus' ? '🎯' : '☕️' }}
          </div>
          <div class="history-content">
            <div class="history-header">
              <span class="history-time">
                {{ formatTime(session.started_at) }} - {{ formatTime(session.ended_at) }}
              </span>
              <span class="history-duration">{{ formatDuration(session.duration_secs) }}</span>
            </div>
            <div v-if="session.task" class="history-task" :title="session.task">{{ session.task }}</div>
            <div v-else-if="session.session_type === 'break'" class="history-task history-task-muted">休息</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.history-container {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.history-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 12px;
  flex-shrink: 0;
}

.history-tabs {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 6px;
  margin-bottom: 14px;
  flex-shrink: 0;
}

.history-tab {
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(255, 255, 255, 0.04);
  color: var(--text-secondary);
  border-radius: 8px;
  font-size: 12px;
  padding: 6px 0;
  cursor: pointer;
  transition: all 0.2s ease;
}

.history-tab:hover {
  background: rgba(255, 255, 255, 0.08);
  color: var(--text-primary);
}

.history-tab.active {
  background: rgba(0, 212, 170, 0.15);
  border-color: rgba(0, 212, 170, 0.45);
  color: var(--text-primary);
}

.history-list {
  flex: 1;
  overflow-y: auto;
  padding-right: 4px;
}

.history-list::-webkit-scrollbar {
  width: 4px;
}
.history-list::-webkit-scrollbar-track {
  background: transparent;
}
.history-list::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 2px;
}

.history-group {
  margin-bottom: 16px;
}

.date-header {
  font-size: 11px;
  color: var(--text-secondary);
  margin-bottom: 8px;
  opacity: 0.6;
}

.history-item {
  display: flex;
  gap: 10px;
  margin-bottom: 12px;
  padding: 8px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.03);
  transition: background 0.2s;
}

.history-item:hover {
  background: rgba(255, 255, 255, 0.06);
}

.history-icon {
  font-size: 16px;
  padding-top: 2px;
}

.history-content {
  flex: 1;
  overflow: hidden;
}

.history-header {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: var(--text-secondary);
  margin-bottom: 4px;
}

.history-task {
  font-size: 13px;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.history-task-muted {
  opacity: 0.6;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-secondary);
  font-size: 13px;
  opacity: 0.6;
}
.empty-icon {
  font-size: 32px;
  margin-bottom: 8px;
}
</style>
