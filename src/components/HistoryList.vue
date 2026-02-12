<script setup lang="ts">
import { computed } from 'vue'

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

function formatTime(ts: number) {
  return new Date(ts * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function formatDuration(secs: number) {
  const mins = Math.floor(secs / 60)
  return mins < 1 ? '< 1m' : `${mins}m`
}

const groupedSessions = computed(() => {
  const groups: Record<string, SessionRecord[]> = {}
  for (const session of props.sessions) {
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
    
    <div v-if="sessions.length === 0" class="empty-state">
      <div class="empty-icon">📝</div>
      <p>暂无专注记录</p>
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
            <div v-else-if="session.session_type === 'break'" class="history-task" style="opacity: 0.6">休息</div>
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
  border-left: 1px solid rgba(255, 255, 255, 0.1);
  padding-left: 20px;
}

.history-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 16px;
  flex-shrink: 0;
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
