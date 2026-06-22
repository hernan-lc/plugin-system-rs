import { h } from "preact";
import { useState, useEffect, useCallback } from "preact/hooks";
import { CSSCustomProperties } from "../lib/types";

interface VolumeState {
  master_volume: number;
  muted: boolean;
  default_device_name: string;
  platform_supported: boolean;
}

export function VolumeWidget({ settings }: { settings: Record<string, any> }) {
  const [state, setState] = useState<VolumeState | null>(null);
  const variant = (settings.variant || "compact") as string;

  const loadVolume = useCallback(async () => {
    try {
      const res = await fetch("/api/volume");
      const data = await res.json();
      if (data.success && data.data) {
        const d = data.data;
        setState({
          master_volume: d.state?.master_volume ?? 0,
          muted: d.state?.muted ?? false,
          default_device_name: d.state?.default_device_name ?? "",
          platform_supported: d.state?.platform_supported ?? false,
        });
      }
    } catch (e) {}
  }, []);

  useEffect(() => {
    loadVolume();
    const interval = setInterval(() => {
      loadVolume();
    }, settings.refreshInterval || 2000);
    return () => clearInterval(interval);
  }, []);

  const setVolume = async (vol: number) => {
    try {
      await fetch("/api/volume/master", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ volume: vol }),
      });
      loadVolume();
    } catch (e) {}
  };

  const setMute = async (muted: boolean) => {
    try {
      await fetch("/api/volume/mute", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ muted }),
      });
      loadVolume();
    } catch (e) {}
  };

  if (!state) return h("div", { class: "vol-loading" }, "Loading...");

  if (!state.platform_supported) {
    return h("div", { class: "vol-unsupported" }, "Volume control not supported on this platform");
  }

  const volColor = state.master_volume < 50 ? "#4caf50" : state.master_volume < 80 ? "#ff9800" : "#f44336";

  if (variant === "minimal") {
    return h("div", { class: "vol-variant minimal" },
      h("div", { class: "vol-big", style: { color: state.muted ? "#666" : volColor } },
        state.muted ? "MUTED" : `${state.master_volume.toFixed(0)}%`
      ),
      h("button", {
        class: "vol-mute-btn",
        onClick: () => setMute(!state.muted)
      }, state.muted ? "UNMUTE" : "MUTE"),
    );
  }

  if (variant === "compact") {
    return h("div", { class: "vol-variant compact" },
      h("div", { class: "vol-header" },
        h("span", { class: "vol-device" }, state.default_device_name.substring(0, 25)),
        h("button", {
          class: "vol-icon-btn",
          onClick: () => setMute(!state.muted)
        }, state.muted ? "M" : "V"),
      ),
      h("div", { class: "vol-slider-row" },
        h("input", {
          type: "range",
          min: 0,
          max: 100,
          value: state.master_volume,
          onInput: (e: Event) => setVolume(parseFloat((e.target as HTMLInputElement).value)),
          class: "vol-slider",
          style: { "--vol-pct": `${state.master_volume}%`, "--vol-color": volColor } as CSSCustomProperties,
        }),
        h("span", { class: "vol-value", style: { color: volColor } }, `${state.master_volume.toFixed(0)}%`),
      ),
    );
  }

  return h("div", { class: "vol-variant detailed" },
    h("div", { class: "vol-detail-header" },
      h("div", { class: "vol-device-full" }, state.default_device_name),
      h("button", {
        class: "vol-mute-btn-large",
        onClick: () => setMute(!state.muted),
        style: { background: state.muted ? "#f44336" : "#4caf50" }
      }, state.muted ? "UNMUTE" : "MUTE"),
    ),
    h("div", { class: "vol-big-row" },
      h("div", { class: "vol-big", style: { color: state.muted ? "#666" : volColor } },
        `${state.master_volume.toFixed(1)}%`
      ),
    ),
    h("div", { class: "vol-slider-row" },
      h("input", {
        type: "range",
        min: 0,
        max: 100,
        value: state.master_volume,
          onInput: (e: Event) => setVolume(parseFloat((e.target as HTMLInputElement).value)),
          class: "vol-slider",
          style: { "--vol-pct": `${state.master_volume}%`, "--vol-color": volColor } as CSSCustomProperties,
      }),
    ),
  );
}
