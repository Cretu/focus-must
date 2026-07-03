// Focus Must splash controller. Loaded as a classic script (not a module) so
// it runs before the app bundle is even fetched: overlay windows never flash
// the splash, the tagline is localized before first paint, and the main
// window gets a fade-out API to call once Vue has finished booting.
(function () {
  "use strict";

  // Secondary-monitor overlay windows load this same page. They render their
  // own full-screen mask, so the splash must never appear there. Tauri injects
  // __TAURI_INTERNALS__ before any page script runs, which lets us read the
  // window label synchronously.
  try {
    var internals = window.__TAURI_INTERNALS__;
    var currentWindow =
      internals && internals.metadata && internals.metadata.currentWindow;
    var label = (currentWindow && currentWindow.label) || "main";
    if (label.indexOf("overlay-") === 0) {
      document.documentElement.setAttribute("data-splash-off", "");
    }
  } catch (error) {
    // Detection failing just means the splash shows everywhere — harmless.
  }

  // Localize the tagline (the markup ships with the Chinese default).
  document.addEventListener("DOMContentLoaded", function () {
    var tagline = document.getElementById("splash-tagline");
    if (!tagline) return;
    var lang = (navigator.language || "").toLowerCase();
    if (lang.indexOf("en") === 0) {
      tagline.textContent = "Plan first. Then focus.";
    }
  });

  var hidden = false;
  window.__FOCUS_SPLASH__ = {
    hide: function () {
      if (hidden) return;
      hidden = true;
      var el = document.getElementById("splash");
      if (!el) return;
      var remove = function () {
        if (el.parentNode) el.parentNode.removeChild(el);
      };
      el.style.opacity = "0";
      el.addEventListener("transitionend", remove, { once: true });
      // Fallback in case transitionend never fires (e.g. reduced motion).
      setTimeout(remove, 700);
    },
  };
})();
