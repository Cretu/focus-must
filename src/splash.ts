// Bridge to the startup splash defined in index.html / public/splash.js.

interface SplashController {
    hide?: () => void;
}

/** Fade out and remove the startup splash (safe to call multiple times). */
export function hideSplash(): void {
    const controller = (
        window as unknown as { __FOCUS_SPLASH__?: SplashController }
    ).__FOCUS_SPLASH__;

    if (controller?.hide) {
        controller.hide();
        return;
    }

    // splash.js failed to load — remove the element outright so the app is
    // never stuck behind the splash.
    document.getElementById("splash")?.remove();
}
