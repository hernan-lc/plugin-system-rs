import { h } from "preact";
import { useState, useEffect, useCallback } from "preact/hooks";
import { fetchObsInputs, setInputVolume, setInputMute } from "../lib/api";
import { CSSCustomProperties } from "../lib/types";

interface ObsInput {
  name: string;
  kind: string;
  uuid: string;
  muted: boolean;
  volume: number;
}

export function ObsInputsWidget({ settings }: { settings: Record<string, any> }) {
  const [inputs, setInputs] = useState<ObsInput[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const variant = (settings.variant || "compact") as string;

  const fetchInputs = useCallback(async () => {
    try {
      const data = await fetchObsInputs();
      if (data) {
        setInputs(data);
        setError(null);
      } else {
        setError("OBS plugin not available");
      }
    } catch {
      setError("Failed to fetch inputs");
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    fetchInputs();
    const interval = setInterval(fetchInputs, settings.refreshInterval || 2000);
    return () => clearInterval(interval);
  }, [fetchInputs, settings.refreshInterval]);

  async function handleMuteToggle(inputName: string, currentlyMuted: boolean) {
    try {
      await setInputMute(inputName, !currentlyMuted);
      setInputs((prev) =>
        prev.map((inp) =>
          inp.name === inputName ? { ...inp, muted: !currentlyMuted } : inp
        )
      );
    } catch {}
  }

  async function handleVolumeChange(inputName: string, volume: number) {
    try {
      await setInputVolume(inputName, volume);
      setInputs((prev) =>
        prev.map((inp) =>
          inp.name === inputName ? { ...inp, volume } : inp
        )
      );
    } catch {}
  }

  if (loading) return h("div", { class: "obsinput-loading" }, "Loading...");
  if (error) return h("div", { class: "obsinput-error" }, error);

  if (inputs.length === 0) {
    return h("div", { class: "obsinput-empty" },
      h("div", { class: "obsinput-empty-icon" }, "I/O"),
      h("div", null, "No inputs found"),
    );
  }

  if (variant === "minimal") {
    return h("div", { class: "obsinput-variant minimal" },
      h("div", { class: "obsinput-count" }, `${inputs.length} inputs`),
      inputs.slice(0, 4).map((inp) =>
        h("div", { key: inp.name, class: "obsinput-mini-row" },
          h("span", { class: "obsinput-mini-name" }, inp.name.length > 12 ? inp.name.substring(0, 12) + ".." : inp.name),
          h("button", {
            class: `obsinput-mini-mute ${inp.muted ? "muted" : ""}`,
            onClick: () => handleMuteToggle(inp.name, inp.muted),
          }, inp.muted ? "M" : "U"),
        )
      ),
      inputs.length > 4 && h("div", { class: "obsinput-more" }, `+${inputs.length - 4} more`),
    );
  }

  if (variant === "detailed") {
    return h("div", { class: "obsinput-variant detailed" },
      h("div", { class: "obsinput-header" },
        h("span", { class: "obsinput-title" }, "Inputs"),
        h("span", { class: "obsinput-count" }, `${inputs.length}`),
      ),
      h("div", { class: "obsinput-list detailed" },
        inputs.map((inp) => {
          const pct = Math.round(inp.volume * 100);
          return h("div", { key: inp.name, class: "obsinput-item-detailed" },
            h("div", { class: "obsinput-item-info" },
              h("span", { class: "obsinput-item-name" }, inp.name),
              h("span", { class: "obsinput-item-kind" }, inp.kind),
            ),
            h("div", { class: "obsinput-slider-row" },
              h("button", {
                class: `obsinput-mute-btn ${inp.muted ? "muted" : ""}`,
                onClick: () => handleMuteToggle(inp.name, inp.muted),
              }, inp.muted ? "MUTED" : "MUTE"),
              h("input", {
                type: "range",
                class: "obsinput-slider",
                min: "0",
                max: "1",
                step: "0.01",
                value: String(inp.volume),
                onInput: (e: Event) => handleVolumeChange(inp.name, parseFloat((e.target as HTMLInputElement).value)),
                style: { "--vol-pct": `${pct}%` } as CSSCustomProperties,
              }),
              h("span", { class: "obsinput-value" }, `${pct}%`),
            ),
          );
        }),
      ),
    );
  }

  return h("div", { class: "obsinput-variant compact" },
    h("div", { class: "obsinput-header" },
      h("span", { class: "obsinput-title" }, "Inputs"),
      h("span", { class: "obsinput-count" }, `${inputs.length}`),
    ),
    h("div", { class: "obsinput-list" },
      inputs.map((inp) => {
        const pct = Math.round(inp.volume * 100);
        return h("div", { key: inp.name, class: "obsinput-item" },
          h("div", { class: "obsinput-item-header" },
            h("span", { class: "obsinput-item-name" }, inp.name),
            h("button", {
              class: `obsinput-mute-btn-sm ${inp.muted ? "muted" : ""}`,
              onClick: () => handleMuteToggle(inp.name, inp.muted),
            }, inp.muted ? "M" : "U"),
          ),
          h("div", { class: "obsinput-slider-row" },
            h("input", {
              type: "range",
              class: "obsinput-slider",
              min: "0",
              max: "1",
              step: "0.01",
              value: String(inp.volume),
              onInput: (e: Event) => handleVolumeChange(inp.name, parseFloat((e.target as HTMLInputElement).value)),
                style: { "--vol-pct": `${pct}%` } as CSSCustomProperties,
            }),
            h("span", { class: "obsinput-value" }, `${pct}%`),
          ),
        );
      }),
    ),
  );
}
