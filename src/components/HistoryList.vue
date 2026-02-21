<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { SessionRecord } from '../types/contracts'

const props = withDefaults(
  defineProps<{
    sessions: SessionRecord[]
    hasMore?: boolean
    isLoading?: boolean
  }>(),
  {
    hasMore: false,
    isLoading: false,
  },
)

const emit = defineEmits<{
  (e: 'load-more'): void
}>()

const historyListRef = ref<HTMLElement | null>(null)
const { t } = useI18n()

type HistoryTab = 'all' | 'focus' | 'break'

const activeTab = ref<HistoryTab>('all')

const tabs = computed(() => [
  { key: 'all', label: t('history.all'), icon: 'i-lucide-list-filter' },
  { key: 'focus', label: t('history.focus'), icon: 'i-lucide-target' },
  { key: 'break', label: t('history.break'), icon: 'i-lucide-coffee' },
])

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

  const parts: string[] = []
  if (hours > 0) parts.push(t('duration.hours', { count: hours }))
  if (mins > 0) parts.push(t('duration.minutes', { count: mins }))
  if (seconds > 0 || parts.length === 0) parts.push(t('duration.seconds', { count: seconds }))

  return parts.join('')
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

function maybeLoadMore() {
  if (!props.hasMore || props.isLoading) {
    return
  }

  const el = historyListRef.value
  if (!el) {
    return
  }

  const preloadDistance = 64
  const isNearBottom = el.scrollTop + el.clientHeight >= el.scrollHeight - preloadDistance
  if (isNearBottom) {
    emit('load-more')
  }
}

watch(
  () => [props.sessions.length, props.hasMore, props.isLoading, activeTab.value],
  () => {
    void nextTick(maybeLoadMore)
  },
)

onMounted(() => {
  void nextTick(maybeLoadMore)
})
</script>

<template>
  <div class="history-container">
    <div class="space-y-3">
      <h3 class="text-sm font-semibold text-muted">{{ t('history.title') }}</h3>
      <UTabs
        v-model="activeTab"
        :items="tabs"
        value-key="key"
        label-key="label"
        size="xs"
        :content="false"
      />
    </div>

    <div v-if="filteredSessions.length === 0" class="empty-state">
      <UAlert color="neutral" variant="soft" :title="t('history.empty')" icon="i-lucide-notebook-pen" />
    </div>

    <div v-else ref="historyListRef" class="history-list space-y-3" @scroll="maybeLoadMore">
      <div v-for="group in groupedSessions" :key="group.date" class="space-y-2">
        <div class="history-day-divider">{{ group.date }}</div>
        <div class="space-y-2">
          <div v-for="(session, index) in group.items" :key="index" class="history-row">
            <div class="space-y-1">
              <div class="flex items-center justify-between gap-2">
                <div class="flex items-center gap-2 text-xs text-muted">
                  <UBadge
                    :icon="session.session_type === 'focus' ? 'i-lucide-target' : 'i-lucide-coffee'"
                    :color="session.session_type === 'focus' ? 'primary' : 'neutral'"
                    :title="session.session_type === 'focus' ? t('history.focus') : t('history.break')"
                    :aria-label="session.session_type === 'focus' ? t('history.focus') : t('history.break')"
                    variant="soft"
                  />
                  <span :class="{ 'whitespace-nowrap': session.session_type === 'break' }">
                    {{ formatTime(session.started_at) }} - {{ formatTime(session.ended_at) }}
                  </span>
                </div>
                <span class="text-xs font-medium">{{ formatDuration(session.duration_secs) }}</span>
              </div>
              <div
                v-if="session.session_type === 'focus' && session.task"
                class="truncate text-sm"
                :title="session.task"
              >
                {{ session.task }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <div v-if="isLoading" class="history-load-state">{{ t('history.loading') }}</div>
      <div v-else-if="!hasMore && sessions.length > 0" class="history-load-state">{{ t('history.loadedAll') }}</div>
    </div>
  </div>
</template>

<style scoped>
.history-container {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.history-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding-right: 4px;
  scrollbar-width: thin;
  scrollbar-color: transparent transparent;
}

.history-list::-webkit-scrollbar {
  width: 4px;
}
.history-list::-webkit-scrollbar-track {
  background: transparent;
  border-radius: 999px;
}
.history-list::-webkit-scrollbar-thumb {
  background: transparent;
  border-radius: 999px;
}

.history-list:hover {
  scrollbar-color: rgba(148, 163, 184, 0.56) transparent;
}

.history-list:hover::-webkit-scrollbar-track {
  background: rgba(148, 163, 184, 0.08);
}

.history-list:hover::-webkit-scrollbar-thumb {
  background: rgba(148, 163, 184, 0.56);
}

.history-list::-webkit-scrollbar-thumb:hover {
  background: rgba(148, 163, 184, 0.72);
}

.empty-state {
  flex: 1;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.history-day-divider {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 11px;
  font-weight: 600;
  color: color-mix(in oklab, currentColor 72%, transparent);
}

.history-day-divider::before,
.history-day-divider::after {
  content: '';
  flex: 1;
  height: 1.5px;
  background: rgba(148, 163, 184, 0.45);
}

.history-row {
  padding: 8px 2px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.26);
}

.history-row:last-child {
  border-bottom: none;
}

.history-load-state {
  padding: 6px 0 2px;
  text-align: center;
  font-size: 11px;
  color: color-mix(in oklab, currentColor 66%, transparent);
}
</style>
