<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useAppStore } from "../stores/appStore";

const store = useAppStore();
const { t } = useI18n();

const hoveredTrendIndex = ref<number | null>(null);

const maxDailyFocusSecs = computed(() => {
    const values = store.analyticsData?.daily_trend.map((point) => point.focus_secs) ?? [];
    return Math.max(1, ...values);
});

const maxHourFocusSecs = computed(() => {
    const values =
        store.analyticsData?.focus_hour_distribution.map((point) => point.focus_secs) ?? [];
    return Math.max(1, ...values);
});

// Only surface hours that actually contain focus time — rendering all 24
// buckets (most of them empty) just adds noise.
const activeHourBuckets = computed(
    () =>
        store.analyticsData?.focus_hour_distribution.filter(
            (bucket) => bucket.focus_secs > 0,
        ) ?? [],
);

const DAILY_TREND_PLOT_TOP = 8;
const DAILY_TREND_PLOT_BOTTOM = 92;

type DailyTrendChartPoint = {
    x: number;
    y: number;
    day: string;
    focusSecs: number;
};

const dailyTrendChartPoints = computed(() => {
    const trend = store.analyticsData?.daily_trend ?? [];
    if (trend.length === 0) {
        return [] as DailyTrendChartPoint[];
    }

    return trend.map((point, index) => {
        const x = trend.length === 1 ? 50 : (index / (trend.length - 1)) * 100;
        const normalized = Math.max(0, Math.min(1, point.focus_secs / maxDailyFocusSecs.value));
        const y =
            DAILY_TREND_PLOT_TOP +
            (1 - normalized) * (DAILY_TREND_PLOT_BOTTOM - DAILY_TREND_PLOT_TOP);

        return {
            x,
            y,
            day: point.day,
            focusSecs: point.focus_secs,
        };
    });
});

function buildSmoothPath(points: DailyTrendChartPoint[]): string {
    if (points.length === 0) {
        return "";
    }

    if (points.length === 1) {
        return `M ${points[0].x} ${points[0].y}`;
    }

    let path = `M ${points[0].x} ${points[0].y}`;

    for (let index = 0; index < points.length - 1; index++) {
        const p0 = points[index - 1] ?? points[index];
        const p1 = points[index];
        const p2 = points[index + 1];
        const p3 = points[index + 2] ?? p2;

        const cp1x = p1.x + (p2.x - p0.x) / 6;
        const cp1y = p1.y + (p2.y - p0.y) / 6;
        const cp2x = p2.x - (p3.x - p1.x) / 6;
        const cp2y = p2.y - (p3.y - p1.y) / 6;

        path += ` C ${cp1x} ${cp1y}, ${cp2x} ${cp2y}, ${p2.x} ${p2.y}`;
    }

    return path;
}

const dailyTrendSmoothPath = computed(() => buildSmoothPath(dailyTrendChartPoints.value));

const dailyTrendAreaPath = computed(() => {
    const points = dailyTrendChartPoints.value;
    if (points.length === 0) {
        return "";
    }

    const smoothLinePath = buildSmoothPath(points);
    const first = points[0];
    const last = points[points.length - 1];

    return `${smoothLinePath} L ${last.x} ${DAILY_TREND_PLOT_BOTTOM} L ${first.x} ${DAILY_TREND_PLOT_BOTTOM} Z`;
});

const activeTrendIndex = computed(() => {
    const points = dailyTrendChartPoints.value;
    if (points.length === 0) {
        return null;
    }

    if (hoveredTrendIndex.value === null) {
        return points.length - 1;
    }

    return Math.max(0, Math.min(points.length - 1, hoveredTrendIndex.value));
});

const activeTrendPoint = computed(() => {
    const index = activeTrendIndex.value;
    if (index === null) {
        return null;
    }

    return dailyTrendChartPoints.value[index] ?? null;
});

const dailyTrendLabelPoints = computed(() => {
    const points = dailyTrendChartPoints.value;
    if (points.length <= 6) {
        return points;
    }

    const step = Math.ceil(points.length / 6);
    return points.filter(
        (_point, index) => index % step === 0 || index === points.length - 1,
    );
});

function updateTrendHover(event: MouseEvent) {
    const points = dailyTrendChartPoints.value;
    const target = event.currentTarget as SVGSVGElement | null;

    if (!target || points.length === 0) {
        return;
    }

    const rect = target.getBoundingClientRect();
    if (rect.width <= 0) {
        return;
    }

    const ratio = (event.clientX - rect.left) / rect.width;
    const normalizedX = Math.max(0, Math.min(100, ratio * 100));

    let nearest = 0;
    let nearestDistance = Number.POSITIVE_INFINITY;

    points.forEach((point, index) => {
        const distance = Math.abs(point.x - normalizedX);
        if (distance < nearestDistance) {
            nearest = index;
            nearestDistance = distance;
        }
    });

    hoveredTrendIndex.value = nearest;
}

function clearTrendHover() {
    hoveredTrendIndex.value = null;
}

function formatDurationLabel(totalSeconds: number): string {
    const hours = Math.floor(totalSeconds / 3600);
    const mins = Math.floor((totalSeconds % 3600) / 60);
    if (hours > 0) {
        return t("duration.hoursLabel", { hours, minutes: mins });
    }
    return t("duration.minutesLabel", { minutes: mins });
}

function formatDurationCompact(totalSeconds: number): string {
    const mins = Math.floor(totalSeconds / 60);
    if (mins >= 60) {
        const h = Math.floor(mins / 60);
        const m = mins % 60;
        return t("duration.hoursCompact", {
            hours: h,
            minutes: m > 0 ? t("duration.minutesCompact", { minutes: m }) : "",
        }).trim();
    }
    return t("duration.minutesCompact", { minutes: mins });
}

function formatHourLabel(hour: number): string {
    const startHour = ((hour % 24) + 24) % 24;
    const endHour = (startHour + 1) % 24;
    return `${String(startHour).padStart(2, "0")}:00~${String(endHour).padStart(2, "0")}:00`;
}
</script>

<template>
    <UCard
        class="flex h-[86vh] w-[min(1080px,94vw)] flex-col overflow-hidden"
        :ui="{ body: 'flex-1 min-h-0 overflow-hidden' }"
    >
        <template #header>
            <div class="space-y-1">
                <div class="flex min-w-0 items-center gap-2">
                    <UIcon name="i-lucide-chart-column" class="text-3xl text-primary" />
                    <h1 class="text-xl font-semibold leading-tight">{{ t("app.analyticsTitle") }}</h1>
                </div>
                <p class="text-sm text-muted">{{ t("app.analyticsSubtitle") }}</p>
            </div>
        </template>

        <div class="h-full min-h-0 space-y-3">
            <UAlert
                v-if="store.analyticsLoading"
                color="neutral"
                variant="soft"
                :title="t('app.analyticsLoading')"
            />

            <template v-else-if="store.analyticsData">
                <div class="grid gap-2.5 sm:grid-cols-3">
                    <UCard variant="soft">
                        <div class="analytics-metric-card">
                            <div class="analytics-metric-main">
                                <UIcon name="i-lucide-timer" class="analytics-metric-icon" />
                                <div class="analytics-metric-value-group">
                                    <p class="analytics-metric-label">{{ t("app.totalFocusDuration") }}</p>
                                    <p class="analytics-metric-value">
                                        {{ formatDurationLabel(store.analyticsData.summary.total_focus_secs) }}
                                    </p>
                                </div>
                            </div>
                        </div>
                    </UCard>
                    <UCard variant="soft">
                        <div class="analytics-metric-card">
                            <div class="analytics-metric-main">
                                <UIcon name="i-lucide-coffee" class="analytics-metric-icon" />
                                <div class="analytics-metric-value-group">
                                    <p class="analytics-metric-label">{{ t("app.totalBreakDuration") }}</p>
                                    <p class="analytics-metric-value">
                                        {{ formatDurationLabel(store.analyticsData.summary.total_break_secs) }}
                                    </p>
                                </div>
                            </div>
                        </div>
                    </UCard>
                    <UCard variant="soft">
                        <div class="analytics-metric-card">
                            <div class="analytics-metric-main">
                                <UIcon name="i-lucide-list" class="analytics-metric-icon" />
                                <div class="analytics-metric-value-group">
                                    <p class="analytics-metric-label">{{ t("app.totalSessions") }}</p>
                                    <p class="analytics-metric-value">
                                        {{ store.analyticsData.summary.total_sessions }}
                                    </p>
                                </div>
                            </div>
                        </div>
                    </UCard>
                </div>

                <UCard variant="soft">
                    <template #header>
                        <p class="flex items-center gap-1.5 text-sm font-semibold text-muted">
                            <UIcon name="i-lucide-chart-line" class="text-base" />
                            <span>{{ t("app.dailyTrend") }}</span>
                        </p>
                    </template>
                    <div v-if="dailyTrendChartPoints.length > 0" class="space-y-3">
                        <div class="daily-trend-chart-wrap">
                            <svg
                                class="daily-trend-chart"
                                viewBox="0 0 100 100"
                                preserveAspectRatio="none"
                                role="img"
                                :aria-label="t('app.dailyTrendAria')"
                                @mousemove="updateTrendHover"
                                @mouseleave="clearTrendHover"
                            >
                                <defs>
                                    <linearGradient id="dailyTrendArea" x1="0" y1="0" x2="0" y2="1">
                                        <stop offset="0%" stop-color="rgb(16 185 129 / 0.35)" />
                                        <stop offset="100%" stop-color="rgb(16 185 129 / 0.04)" />
                                    </linearGradient>
                                </defs>
                                <line
                                    v-for="point in dailyTrendChartPoints"
                                    :key="`grid-${point.day}`"
                                    :x1="point.x"
                                    :y1="DAILY_TREND_PLOT_TOP"
                                    :x2="point.x"
                                    :y2="DAILY_TREND_PLOT_BOTTOM"
                                    class="daily-trend-grid-line"
                                />
                                <path :d="dailyTrendAreaPath" fill="url(#dailyTrendArea)" />
                                <path
                                    :d="dailyTrendSmoothPath"
                                    fill="none"
                                    stroke="rgb(16 185 129)"
                                    stroke-width="0.35"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                />
                                <line
                                    v-if="activeTrendPoint"
                                    :x1="activeTrendPoint.x"
                                    :y1="DAILY_TREND_PLOT_TOP"
                                    :x2="activeTrendPoint.x"
                                    :y2="DAILY_TREND_PLOT_BOTTOM"
                                    class="daily-trend-focus-line"
                                />
                            </svg>

                            <div
                                v-if="activeTrendPoint"
                                class="daily-trend-dot"
                                :style="{ left: `${activeTrendPoint.x}%`, top: `${activeTrendPoint.y}%` }"
                            ></div>

                            <div
                                v-if="activeTrendPoint"
                                class="daily-trend-tooltip"
                                :style="{ left: `${activeTrendPoint.x}%` }"
                            >
                                <p class="daily-trend-tooltip-date">{{ activeTrendPoint.day }}</p>
                                <p class="daily-trend-tooltip-value">
                                    {{ formatDurationCompact(activeTrendPoint.focusSecs) }}
                                </p>
                            </div>
                        </div>

                        <div class="flex items-center justify-between gap-2">
                            <div
                                v-for="point in dailyTrendLabelPoints"
                                :key="`label-${point.day}`"
                                class="min-w-0 text-center"
                            >
                                <p class="text-[10px] text-muted">{{ point.day.slice(5) }}</p>
                                <p class="text-[10px] font-medium">{{ formatDurationCompact(point.focusSecs) }}</p>
                            </div>
                        </div>
                    </div>

                    <div v-else class="py-6 text-center text-xs text-muted">
                        {{ t("app.noTrendData") }}
                    </div>
                </UCard>

                <UCard variant="soft">
                    <template #header>
                        <p class="flex items-center gap-1.5 text-sm font-semibold text-muted">
                            <UIcon name="i-lucide-clock-3" class="text-base" />
                            <span>{{ t("app.focusHourDistribution") }}</span>
                        </p>
                    </template>
                    <div
                        v-if="activeHourBuckets.length > 0"
                        class="grid grid-cols-2 gap-x-6 gap-y-1.5"
                    >
                        <div
                            v-for="bucket in activeHourBuckets"
                            :key="bucket.hour"
                            class="grid grid-cols-[100px_1fr_64px] items-center gap-1"
                        >
                            <span class="hour-range-chip">{{ formatHourLabel(bucket.hour) }}</span>
                            <UProgress :model-value="bucket.focus_secs" :max="maxHourFocusSecs" size="xs" />
                            <span class="text-[11px] text-right text-muted whitespace-nowrap tabular-nums">{{
                                formatDurationCompact(bucket.focus_secs)
                            }}</span>
                        </div>
                    </div>

                    <div v-else class="py-6 text-center text-xs text-muted">
                        {{ t("app.noTrendData") }}
                    </div>
                </UCard>
            </template>
        </div>

        <template #footer>
            <div class="flex justify-end">
                <UButton
                    color="neutral"
                    variant="outline"
                    leading-icon="i-lucide-arrow-left"
                    @click="store.currentView = 'planning'"
                >
                    {{ t("app.back") }}
                </UButton>
            </div>
        </template>
    </UCard>
</template>

<style scoped>
.analytics-metric-card {
    display: flex;
    flex-direction: column;
    justify-content: center;
    min-height: 74px;
}

.analytics-metric-label {
    margin: 0;
    font-size: 11px;
    line-height: 1.1;
    color: color-mix(in oklab, currentColor 60%, transparent);
}

.analytics-metric-main {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
}

.analytics-metric-value-group {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 5px;
}

.analytics-metric-value {
    margin: 0;
    font-size: 21px;
    font-weight: 700;
    line-height: 1;
}

.analytics-metric-icon {
    font-size: 40px;
    line-height: 1;
    color: rgb(16 185 129);
}

.hour-range-chip {
    display: inline-flex;
    width: 100%;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    background: color-mix(in oklab, rgb(16 185 129) 14%, transparent);
    padding: 2px 6px;
    font-size: 11px;
    font-weight: 600;
    line-height: 1.2;
    color: color-mix(in oklab, rgb(16 185 129) 72%, currentColor);
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
}

.daily-trend-chart-wrap {
    position: relative;
    height: 136px;
    width: 100%;
    border-radius: 12px;
    padding: 10px;
    border: 1px solid rgba(148, 163, 184, 0.2);
    background: rgba(148, 163, 184, 0.06);
}

.daily-trend-chart {
    width: 100%;
    height: 100%;
    display: block;
}

.daily-trend-grid-line {
    stroke: rgba(148, 163, 184, 0.2);
    stroke-width: 0.35;
}

.daily-trend-focus-line {
    stroke: rgba(16, 185, 129, 0.7);
    stroke-width: 0.45;
    stroke-dasharray: 1.4 1.4;
}

.daily-trend-dot {
    position: absolute;
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: rgb(16 185 129);
    border: 2px solid rgb(255 255 255 / 0.92);
    transform: translate(-50%, -50%);
    box-shadow: 0 0 0 1px rgb(16 185 129 / 0.2);
    pointer-events: none;
}

.daily-trend-tooltip {
    position: absolute;
    top: 8px;
    transform: translateX(-50%);
    border-radius: 10px;
    border: 1px solid rgba(148, 163, 184, 0.28);
    background: rgba(15, 23, 42, 0.86);
    padding: 6px 8px;
    pointer-events: none;
    min-width: 82px;
}

.daily-trend-tooltip-date {
    font-size: 10px;
    color: rgba(226, 232, 240, 0.82);
    line-height: 1.1;
}

.daily-trend-tooltip-value {
    margin-top: 2px;
    font-size: 11px;
    font-weight: 600;
    color: rgba(240, 253, 244, 0.96);
    line-height: 1.1;
}
</style>
