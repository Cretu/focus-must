import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { SessionRecord, HistoryPage, AnalyticsData } from "../types/contracts";

const HISTORY_PAGE_SIZE = 20;

export function useHistory() {
    const sessionHistory = ref<SessionRecord[]>([]);
    const historyOffset = ref(0);
    const historyHasMore = ref(true);
    const historyLoading = ref(false);
    const analyticsData = ref<AnalyticsData | null>(null);
    const analyticsLoading = ref(false);

    async function loadHistory(reset = false) {
        if (historyLoading.value) {
            return;
        }

        if (reset) {
            historyOffset.value = 0;
            historyHasMore.value = true;
            sessionHistory.value = [];
        }

        if (!historyHasMore.value) {
            return;
        }

        historyLoading.value = true;

        try {
            const page = await invoke<HistoryPage>("get_history_page", {
                offset: historyOffset.value,
                limit: HISTORY_PAGE_SIZE,
            });

            sessionHistory.value = [...sessionHistory.value, ...page.items];
            historyOffset.value += page.items.length;
            historyHasMore.value = page.has_more;
        } catch (e) {
            console.error("Failed to load history:", e);
        } finally {
            historyLoading.value = false;
        }
    }

    function loadMoreHistory() {
        void loadHistory(false);
    }

    async function loadAnalytics() {
        analyticsLoading.value = true;
        try {
            analyticsData.value = await invoke<AnalyticsData>("get_analytics");
        } catch (e) {
            console.error("Failed to load analytics:", e);
            analyticsData.value = null;
        } finally {
            analyticsLoading.value = false;
        }
    }

    return {
        sessionHistory,
        historyHasMore,
        historyLoading,
        analyticsData,
        analyticsLoading,
        loadHistory,
        loadMoreHistory,
        loadAnalytics,
    };
}
