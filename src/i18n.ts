import { createI18n } from "vue-i18n";
import zhCN from "./locales/zh-CN.json";
import enUS from "./locales/en-US.json";

export const LOCALE_STORAGE_KEY = "focus-must-locale";

export const localeMessages = {
    "zh-CN": zhCN,
    "en-US": enUS,
} as const;

export type LocaleCode = keyof typeof localeMessages;
export type PreferredLocale = LocaleCode | "system";

export const localeOptions: Array<{ label: string; value: PreferredLocale }> = [
    { label: "🖥️", value: "system" },
    { label: "🇨🇳", value: "zh-CN" },
    { label: "🇺🇸", value: "en-US" },
];

export const localeOptionsWithText: Array<{ label: string; value: PreferredLocale }> = [
    { label: "🖥️ 跟随系统", value: "system" },
    { label: "🇨🇳 简体中文", value: "zh-CN" },
    { label: "🇺🇸 English", value: "en-US" },
];

export function isSupportedLocale(value: unknown): value is PreferredLocale {
    return value === "system" || value === "zh-CN" || value === "en-US";
}

export const i18n = createI18n({
    legacy: false,
    locale: "zh-CN",
    fallbackLocale: "en-US",
    messages: localeMessages,
});
