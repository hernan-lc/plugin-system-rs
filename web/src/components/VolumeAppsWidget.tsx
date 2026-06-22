import { h } from "preact";
import { useState, useEffect, useCallback } from "preact/hooks";
import { CSSCustomProperties } from "../lib/types";

interface AppVolume {
  name: string;
  volume: number;
  muted: boolean;
  pid: number | null;
}

export function VolumeAppsWidget({ settings }: { settings: Record<string, any> }) {
  const [apps, setApps] = useState<AppVolume[]>([]);
  const [platformSupported, setPlatformSupported] = useState(true);
  const variant = (settings.variant || "compact") as string;

  const loadApps = useCallback(async () => {
    try {
      const res = await fetch("/api/volume");
      const data = await res.json();
      if (data.success && data.data) {
        setPlatformSupported(data.data.state?.per_app_supported ?? false);
        setApps(data.data.apps || []);
      }
    } catch (e) {}
  }, []);

  useEffect(() => {
    loadApps();
    const interval = setInterval(loadApps, settings.refreshInterval || 2000);
    return () => clearInterval(interval);
  }, []);

  const setAppVolume = async (appName: string, vol: number) => {
    try {
      await fetch("/api/volume/app/volume", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ app_name: appName, volume: vol }),
      });
      loadApps();
    } catch (e) {}
  };

  const setAppMute = async (appName: string, muted: boolean) => {
    try {
      await fetch("/api/volume/app/mute", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ app_name: appName, muted }),
      });
      loadApps();
    } catch (e) {}
  };

  if (!platformSupported) {
    return h("div", { class: "vol-unsupported" }, "Per-app volume not supported on this platform");
  }

  if (apps.length === 0) {
    return h("div", { class: "volapps-empty" },
      h("div", { class: "volapps-empty-icon" }, "A"),
      h("div", { class: "volapps-empty-text" }, "No active audio streams"),
    );
  }

  if (variant === "minimal") {
    return h("div", { class: "volapps-variant minimal" },
      h("div", { class: "volapps-count" }, `${apps.length} app${apps.length !== 1 ? "s" : ""}`),
      apps.slice(0, 3).map(app => h("div", { class: "volapps-mini-item", key: app.name },
        h("span", { class: "volapps-mini-name" }, app.name.substring(0, 12)),
        h("span", { class: "volapps-mini-vol", style: { color: app.muted ? "#666" : "#4caf50" } },
          app.muted ? "M" : `${app.volume.toFixed(0)}%`
        ),
      )),
      apps.length > 3 && h("div", { class: "volapps-mini-more" }, `+${apps.length - 3} more`),
    );
  }

  if (variant === "compact") {
    return h("div", { class: "volapps-variant compact" },
      h("div", { class: "volapps-header" },
        h("span", { class: "volapps-title" }, `Audio Apps (${apps.length})`),
      ),
      h("div", { class: "volapps-list" },
        apps.map(app => h("div", { class: "volapps-item", key: app.name },
          h("div", { class: "volapps-item-header" },
            h("span", { class: "volapps-item-name" }, app.name.substring(0, 18)),
            h("button", {
              class: `volapps-mute-btn ${app.muted ? "muted" : ""}`,
              onClick: () => setAppMute(app.name, !app.muted)
            }, app.muted ? "M" : "V"),
          ),
          h("div", { class: "volapps-slider-row" },
            h("input", {
              type: "range",
              min: 0,
              max: 100,
              value: app.volume,
              onInput: (e: Event) => setAppVolume(app.name, parseFloat((e.target as HTMLInputElement).value)),
              class: "volapps-slider",
              style: { "--vol-pct": `${app.volume}%` } as CSSCustomProperties,
            }),
            h("span", { class: "volapps-value" }, `${app.volume.toFixed(0)}%`),
          ),
        ))
      ),
    );
  }

  return h("div", { class: "volapps-variant detailed" },
    h("div", { class: "volapps-header" },
      h("span", { class: "volapps-title" }, `Active Audio Streams (${apps.length})`),
    ),
    h("div", { class: "volapps-list detailed" },
      apps.map(app => h("div", { class: "volapps-item-detailed", key: app.name },
        h("div", { class: "volapps-item-header" },
          h("div", { class: "volapps-item-info" },
            h("span", { class: "volapps-item-name" }, app.name),
            app.pid && h("span", { class: "volapps-item-pid" }, `PID: ${app.pid}`),
          ),
          h("button", {
            class: `volapps-mute-btn-detailed ${app.muted ? "muted" : ""}`,
            onClick: () => setAppMute(app.name, !app.muted)
          }, app.muted ? "UNMUTE" : "MUTE"),
        ),
        h("div", { class: "volapps-slider-row" },
          h("input", {
            type: "range",
            min: 0,
            max: 100,
            value: app.volume,
              onInput: (e: Event) => setAppVolume(app.name, parseFloat((e.target as HTMLInputElement).value)),
              class: "volapps-slider",
              style: { "--vol-pct": `${app.volume}%` } as CSSCustomProperties,
          }),
          h("span", { class: "volapps-value" }, `${app.volume.toFixed(0)}%`),
        ),
      ))
    ),
  );
}
