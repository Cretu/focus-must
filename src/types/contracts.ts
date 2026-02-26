import type { PreferredLocale } from "../i18n";

export interface AppInfo {
    name: string;
    bundle_id: string;
    icon_data_url?: string | null;
}

export interface BlockedAppEvent {
    name: string;
    bundle_id: string;
    return_to_bundle_id?: string;
    return_to_name?: string;
}

export interface AppState {
    is_restricted: boolean;
    default_whitelist: string[];
    session_whitelist: string[];
    task_description: string | null;
    focus_started_at: number | null;
    free_activity_started_at: number | null;
    free_activity_end_at: number | null;
    locale: PreferredLocale;
}

export interface SessionRecord {
    session_type: string;
    started_at: number;
    ended_at: number;
    duration_secs: number;
    task?: string;
    whitelist: string[];
}

export interface AnalyticsSummary {
    total_focus_secs: number;
    total_break_secs: number;
    total_sessions: number;
    focus_sessions: number;
    break_sessions: number;
}

export interface DailyTrendPoint {
    day: string;
    focus_secs: number;
    break_secs: number;
}

export interface FocusHourBucket {
    hour: number;
    focus_secs: number;
    sessions: number;
}

export interface AnalyticsData {
    summary: AnalyticsSummary;
    daily_trend: DailyTrendPoint[];
    focus_hour_distribution: FocusHourBucket[];
}

export interface HistoryPage {
    items: SessionRecord[];
    has_more: boolean;
}
