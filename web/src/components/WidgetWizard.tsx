import { h } from "preact";
import { useState } from "preact/hooks";
import { WidgetConfig, WIDGET_VARIANTS } from "../lib/types";
import { recordHotkey, resetHotkeyRecording } from "../lib/api";
import { WidgetContent } from "./WidgetContent";

interface WidgetWizardProps {
  widget: WidgetConfig;
  columns: number;
  onSave: (
    id: string,
    updates: {
      title?: string;
      colSpan?: number;
      settings?: Record<string, any>;
    },
  ) => void;
  onRemove: () => void;
  onClose: () => void;
}

export function WidgetWizard({
  widget,
  columns,
  onSave,
  onRemove,
  onClose,
}: WidgetWizardProps) {
  const [step, setStep] = useState(0);
  const [title, setTitle] = useState(widget.title);
  const [colSpan, setColSpan] = useState(widget.colSpan);
  const [settings, setSettings] = useState({ ...widget.settings });
  const [variant, setVariant] = useState<string>(
    widget.settings.variant || "compact",
  );
  const totalSteps = 4;

  function handleNext() {
    if (step < totalSteps - 1) setStep(step + 1);
  }
  function handleBack() {
    if (step > 0) setStep(step - 1);
  }
  function handleApply() {
    onSave(widget.id, { title, colSpan, settings: { ...settings, variant } });
  }

  return h(
    "div",
    { class: "wizard-overlay", onClick: onClose },
    h(
      "div",
      { class: "wizard-modal", onClick: (e: Event) => e.stopPropagation() },
      h(
        "div",
        { class: "wizard-header" },
        h("div", { class: "wizard-title" }, `Edit: ${widget.type}`),
        h("button", { class: "picker-close", onClick: onClose }, "X"),
      ),
      h(
        "div",
        { class: "wizard-steps" },
        ["General", "Config", "Style", "Apply"].map((label, i) =>
          h(
            "div",
            {
              class: `wizard-step-indicator ${i === step ? "active" : i < step ? "done" : ""}`,
              key: label,
              onClick: () => setStep(i),
            },
            h("div", { class: "wizard-step-circle" }, label[0]),
            h("div", { class: "wizard-step-label" }, label),
          ),
        ),
      ),
      h(
        "div",
        { class: "wizard-body" },
        step === 0 &&
          h(WizardGeneral, {
            title,
            colSpan,
            columns,
            onChangeTitle: setTitle,
            onChangeColSpan: setColSpan,
          }),
        step === 1 &&
          h(WizardConfig, { widget, settings, onChange: setSettings }),
        step === 2 && h(WizardStyle, { widget, variant, onChange: setVariant }),
        step === 3 &&
          h(WizardConfirm, {
            widget,
            title,
            colSpan,
            settings,
            variant,
            onApply: handleApply,
            onRemove,
          }),
      ),
      h(
        "div",
        { class: "wizard-footer" },
        step > 0 &&
          h(
            "button",
            { class: "wizard-btn back", onClick: handleBack },
            "Back",
          ),
        h("div", { class: "wizard-footer-spacer" }),
        step < totalSteps - 1
          ? h(
              "button",
              { class: "wizard-btn next", onClick: handleNext },
              "Next",
            )
          : h(
              "button",
              { class: "wizard-btn apply", onClick: handleApply },
              "Save & Close",
            ),
      ),
    ),
  );
}

function WizardGeneral({
  title,
  colSpan,
  columns,
  onChangeTitle,
  onChangeColSpan,
}: {
  title: string;
  colSpan: number;
  columns: number;
  onChangeTitle: (t: string) => void;
  onChangeColSpan: (c: number) => void;
}) {
  return h(
    "div",
    { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, "General Settings"),
    h(
      "div",
      { class: "wizard-field" },
      h("label", null, "Widget Title"),
      h("input", {
        type: "text",
        value: title,
        onInput: (e: Event) =>
          onChangeTitle((e.target as HTMLInputElement).value),
        placeholder: "Enter widget title...",
      }),
    ),
    h(
      "div",
      { class: "wizard-field" },
      h("label", null, "Column Span"),
      h("input", {
        type: "number",
        min: "1",
        max: String(columns),
        value: String(colSpan),
        onInput: (e: Event) =>
          onChangeColSpan(parseInt((e.target as HTMLInputElement).value) || 1),
      }),
      h("span", { class: "wizard-field-hint" }, `Grid has ${columns} columns`),
    ),
  );
}

function WizardConfig({
  widget,
  settings,
  onChange,
}: {
  widget: WidgetConfig;
  settings: Record<string, any>;
  onChange: (s: Record<string, any>) => void;
}) {
  function set(key: string, value: any) {
    onChange({ ...settings, [key]: value });
  }
  return h(
    "div",
    { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, "Widget Configuration"),
    widget.type === "send-hotkey" &&
      h(HotkeyRecorder, {
        currentKeys: settings.keys || "",
        onChange: (keys) => set("keys", keys),
      }),
    widget.type === "open-url" &&
      h(
        "div",
        { class: "wizard-field" },
        h("label", null, "URL"),
        h("input", {
          type: "text",
          value: settings.url || "",
          placeholder: "https://example.com",
          onInput: (e: Event) =>
            set("url", (e.target as HTMLInputElement).value),
        }),
      ),
    widget.type === "type-text" &&
      h(
        "div",
        { class: "wizard-field" },
        h("label", null, "Text"),
        h("textarea", {
          value: settings.text || "",
          placeholder: "Text to type...",
          onInput: (e: Event) =>
            set("text", (e.target as HTMLTextAreaElement).value),
        }),
      ),
    widget.type === "system-monitor" &&
      h(
        "div",
        { class: "wizard-field" },
        h("label", null, "Refresh Interval (ms)"),
        h("input", {
          type: "number",
          min: "500",
          step: "500",
          value: String(settings.refreshInterval || 2000),
          onInput: (e: Event) =>
            set(
              "refreshInterval",
              parseInt((e.target as HTMLInputElement).value) || 2000,
            ),
        }),
      ),
    widget.type === "volume-master" &&
      h(
        "div",
        { class: "wizard-field" },
        h("label", null, "Refresh Interval (ms)"),
        h("input", {
          type: "number",
          min: "500",
          step: "500",
          value: String(settings.refreshInterval || 2000),
          onInput: (e: Event) =>
            set(
              "refreshInterval",
              parseInt((e.target as HTMLInputElement).value) || 2000,
            ),
        }),
      ),
    widget.type === "volume-apps" &&
      h(
        "div",
        { class: "wizard-field" },
        h("label", null, "Refresh Interval (ms)"),
        h("input", {
          type: "number",
          min: "500",
          step: "500",
          value: String(settings.refreshInterval || 2000),
          onInput: (e: Event) =>
            set(
              "refreshInterval",
              parseInt((e.target as HTMLInputElement).value) || 2000,
            ),
        }),
      ),
    widget.type === "obs-control" &&
      h(
        "div",
        { class: "wizard-field" },
        h("label", null, "OBS Host"),
        h("input", {
          type: "text",
          value: settings.host || "127.0.0.1",
          placeholder: "127.0.0.1",
          onInput: (e: Event) => set("host", (e.target as HTMLInputElement).value),
        }),
        h("label", null, "Port"),
        h("input", {
          type: "number",
          value: String(settings.port || 4455),
          onInput: (e: Event) => set("port", parseInt((e.target as HTMLInputElement).value) || 4455),
        }),
        h("label", null, "Password"),
        h("input", {
          type: "password",
          value: settings.password || "",
          placeholder: "OBS WebSocket password",
          onInput: (e: Event) => set("password", (e.target as HTMLInputElement).value),
        }),
        h("label", null, "Refresh Interval (ms)"),
        h("input", {
          type: "number",
          min: "500",
          step: "500",
          value: String(settings.refreshInterval || 2000),
          onInput: (e: Event) =>
            set("refreshInterval", parseInt((e.target as HTMLInputElement).value) || 2000),
        }),
      ),
    widget.type === "obs-scenes" &&
      h(
        "div",
        { class: "wizard-field" },
        h("label", null, "Refresh Interval (ms)"),
        h("input", {
          type: "number",
          min: "500",
          step: "500",
          value: String(settings.refreshInterval || 2000),
          onInput: (e: Event) =>
            set("refreshInterval", parseInt((e.target as HTMLInputElement).value) || 2000),
        }),
      ),
    widget.type === "obs-inputs" &&
      h(
        "div",
        { class: "wizard-field" },
        h("label", null, "Refresh Interval (ms)"),
        h("input", {
          type: "number",
          min: "500",
          step: "500",
          value: String(settings.refreshInterval || 2000),
          onInput: (e: Event) =>
            set("refreshInterval", parseInt((e.target as HTMLInputElement).value) || 2000),
        }),
      ),
    widget.type === "fetch" &&
      h(
        "div",
        { class: "wizard-step-content" },
        h("h3", { class: "wizard-step-heading" }, "Widget Configuration"),
        h(
          "div",
          { class: "wizard-field" },
          h("label", null, "URL"),
          h("input", {
            type: "text",
            value: settings.url || "",
            placeholder: "https://api.example.com/data",
            onInput: (e: Event) => set("url", (e.target as HTMLInputElement).value),
          }),
        ),
        h(
          "div",
          { class: "wizard-field-row" },
          h(
            "div",
            { class: "wizard-field" },
            h("label", null, "Fetch Mode"),
            h("select", {
              value: settings.mode || "proxy",
              onChange: (e: Event) => set("mode", (e.target as HTMLSelectElement).value),
            },
              h("option", { value: "local" }, "Local (Browser)"),
              h("option", { value: "proxy" }, "Proxy (Backend)")
            ),
          ),
          h(
            "div",
            { class: "wizard-field" },
            h("label", null, "HTTP Method"),
            h("select", {
              value: settings.method || "GET",
              onChange: (e: Event) => set("method", (e.target as HTMLSelectElement).value),
            },
              h("option", { value: "GET" }, "GET"),
              h("option", { value: "POST" }, "POST"),
              h("option", { value: "PUT" }, "PUT"),
              h("option", { value: "DELETE" }, "DELETE"),
              h("option", { value: "PATCH" }, "PATCH")
            ),
          ),
        ),
        h(
          "div",
          { class: "wizard-field" },
          h("label", null, "Refresh Interval (ms)"),
          h("input", {
            type: "number",
            min: "1000",
            step: "1000",
            value: String(settings.refreshInterval || 30000),
            onInput: (e: Event) =>
              set("refreshInterval", parseInt((e.target as HTMLInputElement).value) || 30000),
          }),
        ),
        h("div", { class: "wizard-divider" }),
        h(FetchHeadersEditor, {
          headers: settings.headers || "",
          onChange: (h: string) => set("headers", h),
        }),
        h("div", { class: "wizard-divider" }),
        h(FetchBodyEditor, {
          method: settings.method || "GET",
          body: settings.body || "",
          onChange: (b: string) => set("body", b),
        }),
      ),
  );
}

function HotkeyRecorder({
  currentKeys,
  onChange,
}: {
  currentKeys: string;
  onChange: (keys: string) => void;
}) {
  const [recording, setRecording] = useState(false);
  const [selectedKeys, setSelectedKeys] = useState<string[]>(
    currentKeys ? currentKeys.split("+").filter(Boolean) : []
  );
  const [showPicker, setShowPicker] = useState(false);

  const MODIFIERS = ["ctrl", "shift", "alt", "win"];
  const MODIFIER_LABELS: Record<string, string> = {
    ctrl: "Ctrl",
    shift: "Shift",
    alt: "Alt",
    win: "Win",
  };

  const LETTER_KEYS = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("");
  const NUMBER_KEYS = "0123456789".split("");
  const FUNCTION_KEYS = Array.from({ length: 12 }, (_, i) => `f${i + 1}`);
  const SPECIAL_KEYS = [
    { key: "space", label: "Space" },
    { key: "enter", label: "Enter" },
    { key: "tab", label: "Tab" },
    { key: "escape", label: "Esc" },
    { key: "backspace", label: "Backspace" },
    { key: "delete", label: "Del" },
    { key: "home", label: "Home" },
    { key: "end", label: "End" },
    { key: "pageup", label: "PgUp" },
    { key: "pagedown", label: "PgDn" },
    { key: "up", label: "↑" },
    { key: "down", label: "↓" },
    { key: "left", label: "←" },
    { key: "right", label: "→" },
  ];

  function toggleKey(key: string) {
    const lower = key.toLowerCase();
    setSelectedKeys((prev) => {
      if (prev.includes(lower)) {
        return prev.filter((k) => k !== lower);
      }
      return [...prev, lower];
    });
  }

  function removeKey(key: string) {
    setSelectedKeys((prev) => prev.filter((k) => k !== key));
  }

  function clearAll() {
    setSelectedKeys([]);
  }

  function applySelection() {
    if (selectedKeys.length > 0) {
      onChange(selectedKeys.join("+"));
      setShowPicker(false);
    }
  }

  async function startRecording() {
    setRecording(true);
    try {
      const combo = await recordHotkey(2000);
      if (combo) {
        setSelectedKeys(combo.split("+").filter(Boolean));
      }
    } catch (e) {
      if (e instanceof Error && e.message.includes("Already recording")) {
        await resetHotkeyRecording();
        try {
          const combo = await recordHotkey(2000);
          if (combo) {
            setSelectedKeys(combo.split("+").filter(Boolean));
          }
        } catch {}
      }
    }
    setRecording(false);
  }

  const combo = selectedKeys.join("+");

  return h(
    "div",
    { class: "wizard-field" },
    h("label", null, "Hotkey Combination"),

    h(
      "div",
      { class: "hotkey-display" },
      h(
        "span",
        { class: "hotkey-keys" },
        combo || "Not set"
      ),
      h(
        "button",
        {
          class: "hotkey-record-btn",
          onClick: () => setShowPicker(!showPicker),
        },
        showPicker ? "Close" : "Select"
      ),
      h(
        "button",
        {
          class: `hotkey-record-btn ${recording ? "recording" : ""}`,
          onClick: recording ? () => {} : startRecording,
        },
        recording ? "..." : "Record"
      )
    ),

    showPicker &&
      h(
        "div",
        { class: "key-picker" },

        h("div", { class: "key-picker-section" },
          h("div", { class: "key-picker-label" }, "Selected:"),
          h(
            "div",
            { class: "key-picker-selected" },
            selectedKeys.length === 0
              ? h("span", { class: "key-picker-empty" }, "No keys selected")
              : selectedKeys.map((key) =>
                  h(
                    "span",
                    {
                      class: "key-picker-chip",
                      key,
                      onClick: () => removeKey(key),
                    },
                    key,
                    h("span", { class: "key-picker-chip-x" }, "×")
                  )
                )
          ),
          selectedKeys.length > 0 &&
            h(
              "div",
              { class: "key-picker-actions" },
              h("button", { class: "key-picker-clear", onClick: clearAll }, "Clear"),
              h("button", { class: "key-picker-apply", onClick: applySelection }, "Apply")
            )
        ),

        h("div", { class: "key-picker-section" },
          h("div", { class: "key-picker-label" }, "Modifiers:"),
          h(
            "div",
            { class: "key-picker-modifiers" },
            MODIFIERS.map((key) =>
              h(
                "button",
                {
                  key,
                  class: `key-picker-mod ${selectedKeys.includes(key) ? "active" : ""}`,
                  onClick: () => toggleKey(key),
                },
                MODIFIER_LABELS[key]
              )
            )
          )
        ),

        h("div", { class: "key-picker-section" },
          h("div", { class: "key-picker-label" }, "Letters:"),
          h(
            "div",
            { class: "key-picker-grid key-picker-letters" },
            LETTER_KEYS.map((key) =>
              h(
                "button",
                {
                  key,
                  class: `key-picker-key ${selectedKeys.includes(key.toLowerCase()) ? "active" : ""}`,
                  onClick: () => toggleKey(key),
                },
                key
              )
            )
          )
        ),

        h("div", { class: "key-picker-section" },
          h("div", { class: "key-picker-label" }, "Numbers:"),
          h(
            "div",
            { class: "key-picker-grid key-picker-numbers" },
            NUMBER_KEYS.map((key) =>
              h(
                "button",
                {
                  key,
                  class: `key-picker-key ${selectedKeys.includes(key) ? "active" : ""}`,
                  onClick: () => toggleKey(key),
                },
                key
              )
            )
          )
        ),

        h("div", { class: "key-picker-section" },
          h("div", { class: "key-picker-label" }, "Function Keys:"),
          h(
            "div",
            { class: "key-picker-grid key-picker-functions" },
            FUNCTION_KEYS.map((key) =>
              h(
                "button",
                {
                  key,
                  class: `key-picker-key ${selectedKeys.includes(key) ? "active" : ""}`,
                  onClick: () => toggleKey(key),
                },
                key.toUpperCase()
              )
            )
          )
        ),

        h("div", { class: "key-picker-section" },
          h("div", { class: "key-picker-label" }, "Special Keys:"),
          h(
            "div",
            { class: "key-picker-grid key-picker-special" },
            SPECIAL_KEYS.map(({ key, label }) =>
              h(
                "button",
                {
                  key,
                  class: `key-picker-key ${selectedKeys.includes(key) ? "active" : ""}`,
                  onClick: () => toggleKey(key),
                },
                label
              )
            )
          )
        )
      )
  );
}

function WizardStyle({
  widget,
  variant,
  onChange,
}: {
  widget: WidgetConfig;
  variant: string;
  onChange: (v: string) => void;
}) {
  const entries = WIDGET_VARIANTS.find((e) => e.type === widget.type);
  if (!entries) return null;

  return h(
    "div",
    { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, "Style Variant"),
    h("p", { class: "wizard-step-desc" }, "Choose how this widget displays"),
    h(
      "div",
      { class: "variant-grid" },
      entries.variants.map((v) =>
        h(
          "button",
          {
            class: `variant-card ${variant === v.value ? "selected" : ""}`,
            key: v.value,
            onClick: () => onChange(v.value),
          },
          h(
            "div",
            { class: "variant-card-preview" },
            h(VariantPreview, { type: widget.type, variant: v.value }),
          ),
          h(
            "div",
            { class: "variant-card-info" },
            h("div", { class: "variant-card-label" }, v.label),
            h("div", { class: "variant-card-desc" }, v.description),
          ),
        ),
      ),
    ),
  );
}

function VariantPreview({ type, variant }: { type: string; variant: string }) {
  switch (type) {
    case "system-monitor":
      switch (variant) {
        case "minimal":
          return h(
            "div",
            { class: "variant-preview sysmon-minimal" },
            h("div", null, "42% CPU"),
            h("div", null, "56% RAM"),
          );
        case "compact":
          return h(
            "div",
            { class: "variant-preview sysmon-compact" },
            h(
              "div",
              { class: "mini-bar" },
              h("div", {
                class: "mini-bar-fill",
                style: { width: "42%", background: "#4caf50" },
              }),
            ),
            h(
              "div",
              { class: "mini-bar" },
              h("div", {
                class: "mini-bar-fill",
                style: { width: "56%", background: "#2196f3" },
              }),
            ),
          );
        case "detailed":
          return h(
            "div",
            { class: "variant-preview sysmon-detailed" },
            h(
              "div",
              { class: "mini-grid" },
              h("div", null, "42%"),
              h("div", null, "56%"),
              h("div", null, "1.2"),
              h("div", null, "2d"),
            ),
          );
      }
    case "clock":
      switch (variant) {
        case "simple":
          return h("div", { class: "variant-preview clock-simple" }, "14:30");
        case "digital":
          return h(
            "div",
            { class: "variant-preview clock-digital" },
            "14:30",
            h("div", { class: "mini-sec" }, "15"),
            h("div", { class: "mini-date" }, "Mon"),
          );
        case "detailed":
          return h(
            "div",
            { class: "variant-preview clock-detailed" },
            "14:30:15",
            h("div", { class: "mini-date" }, "Monday, Jun 10"),
          );
      }
    case "volume-master":
      switch (variant) {
        case "minimal":
          return h(
            "div",
            { class: "variant-preview vol-minimal" },
            h("div", null, "75%"),
            h("div", { class: "mini-btn" }, "MUTE"),
          );
        case "compact":
          return h(
            "div",
            { class: "variant-preview vol-compact" },
            h(
              "div",
              { class: "mini-bar" },
              h("div", {
                class: "mini-bar-fill",
                style: { width: "75%", background: "#4caf50" },
              }),
            ),
            h("div", null, "Speaker"),
          );
        case "detailed":
          return h(
            "div",
            { class: "variant-preview vol-detailed" },
            h("div", null, "75%"),
            h(
              "div",
              { class: "mini-bar" },
              h("div", {
                class: "mini-bar-fill",
                style: { width: "75%", background: "#4caf50" },
              }),
            ),
            h("div", { class: "mini-apps" }, "Apps: 2"),
          );
      }
    case "volume-apps":
      switch (variant) {
        case "minimal":
          return h(
            "div",
            { class: "variant-preview volapps-minimal" },
            h("div", null, "3 apps"),
            h("div", { class: "mini-list" }, "Firefox, Spotify"),
          );
        case "compact":
          return h(
            "div",
            { class: "variant-preview volapps-compact" },
            h("div", null, "Firefox"),
            h(
              "div",
              { class: "mini-bar" },
              h("div", {
                class: "mini-bar-fill",
                style: { width: "60%", background: "#4caf50" },
              }),
            ),
          );
        case "detailed":
          return h(
            "div",
            { class: "variant-preview volapps-detailed" },
            h("div", null, "Firefox (PID: 1234)"),
            h(
              "div",
              { class: "mini-bar" },
              h("div", {
                class: "mini-bar-fill",
                style: { width: "60%", background: "#4caf50" },
              }),
            ),
            h("div", null, "60%"),
          );
      }
    case "obs-control":
      switch (variant) {
        case "minimal":
          return h(
            "div",
            { class: "variant-preview obs-minimal" },
            h("div", { class: "mini-row" },
              h("div", { class: "mini-dot green" }),
              h("span", null, "Connected"),
            ),
            h("div", { class: "mini-row" },
              h("div", { class: "mini-dot red" }),
              h("span", null, "Stream"),
            ),
          );
        case "compact":
          return h(
            "div",
            { class: "variant-preview obs-compact" },
            h("div", null, "Scene 1"),
            h("div", { class: "mini-btns" },
              h("div", { class: "mini-btn" }, "STR"),
              h("div", { class: "mini-btn" }, "REC"),
              h("div", { class: "mini-btn" }, "VC"),
            ),
          );
        case "detailed":
          return h(
            "div",
            { class: "variant-preview obs-detailed" },
            h("div", { class: "mini-btns" },
              h("div", { class: "mini-btn active" }, "Stream"),
              h("div", { class: "mini-btn" }, "Record"),
            ),
            h("div", { class: "mini-grid" },
              h("div", null, "CPU"),
              h("div", null, "FPS"),
            ),
          );
      }
    case "obs-scenes":
      switch (variant) {
        case "minimal":
          return h(
            "div",
            { class: "variant-preview obscene-minimal" },
            h("div", null, "Scene 1"),
            h("div", { class: "mini-grid" },
              h("div", { class: "mini-btn active" }, "S1"),
              h("div", { class: "mini-btn" }, "S2"),
            ),
          );
        case "compact":
          return h(
            "div",
            { class: "variant-preview obscene-compact" },
            h("div", { class: "mini-list" },
              h("div", { class: "mini-btn active" }, "Scene 1"),
              h("div", { class: "mini-btn" }, "Scene 2"),
            ),
          );
        case "detailed":
          return h(
            "div",
            { class: "variant-preview obscene-detailed" },
            h("div", { class: "mini-list" },
              h("div", { class: "mini-btn active" }, "Scene 1"),
              h("div", { class: "mini-btn" }, "Scene 2"),
            ),
            h("div", { class: "mini-btns" },
              h("div", { class: "mini-btn" }, "Fade"),
            ),
          );
      }
    case "obs-inputs":
      switch (variant) {
        case "minimal":
          return h(
            "div",
            { class: "variant-preview obsinput-minimal" },
            h("div", null, "3 inputs"),
            h("div", { class: "mini-list" },
              h("div", { class: "mini-row" },
                h("span", null, "Mic"),
                h("div", { class: "mini-btn" }, "M"),
              ),
            ),
          );
        case "compact":
          return h(
            "div",
            { class: "variant-preview obsinput-compact" },
            h("div", null, "Mic"),
            h(
              "div",
              { class: "mini-bar" },
              h("div", {
                class: "mini-bar-fill",
                style: { width: "75%", background: "#4caf50" },
              }),
            ),
          );
        case "detailed":
          return h(
            "div",
            { class: "variant-preview obsinput-detailed" },
            h("div", null, "Mic (audio)"),
            h(
              "div",
              { class: "mini-bar" },
              h("div", {
                class: "mini-bar-fill",
                style: { width: "75%", background: "#4caf50" },
              }),
            ),
            h("div", null, "75%"),
          );
      }
    case "fetch":
      switch (variant) {
        case "minimal":
          return h(
            "div",
            { class: "variant-preview fetch-minimal" },
            h("div", { class: "mini-status ok" }, "200"),
          );
        case "compact":
          return h(
            "div",
            { class: "variant-preview fetch-compact" },
            h("div", { class: "mini-url" }, "api.ex..."),
            h("div", { class: "mini-data" }, '{"id":1...}'),
          );
        case "detailed":
          return h(
            "div",
            { class: "variant-preview fetch-detailed" },
            h("div", { class: "mini-url" }, "https://api.example.com/v1"),
            h("div", { class: "mini-json" }, '{\n  "status": "ok",\n  "data": [...]\n}'),
          );
      }
    default:
      return h(
        "div",
        { class: "variant-preview simple-preview" },
        h(
          "div",
          {
            class: variant === "compact" ? "preview-btn-sm" : "preview-btn-lg",
          },
          "Action",
        ),
      );
  }
}

function WizardConfirm({
  widget,
  title,
  colSpan,
  settings,
  variant,
  onApply,
  onRemove,
}: {
  widget: WidgetConfig;
  title: string;
  colSpan: number;
  settings: Record<string, any>;
  variant: string;
  onApply: () => void;
  onRemove: () => void;
}) {
  return h(
    "div",
    { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, "Confirm Changes"),
    h(
      "p",
      { class: "wizard-step-desc" },
      "Review your widget configuration before saving",
    ),
    h(
      "div",
      { class: "confirm-details" },
      h(
        "div",
        { class: "confirm-row" },
        h("span", null, "Title"),
        h("span", null, title),
      ),
      h(
        "div",
        { class: "confirm-row" },
        h("span", null, "Span"),
        h("span", null, `${colSpan} column${colSpan > 1 ? "s" : ""}`),
      ),
      h(
        "div",
        { class: "confirm-row" },
        h("span", null, "Variant"),
        h("span", null, variant),
      ),
      ...Object.entries(settings)
        .filter(([k]) => k !== "variant")
        .map(([k, v]) =>
          h(
            "div",
            { class: "confirm-row", key: k },
            h("span", null, k),
            h("span", null, String(v).substring(0, 40)),
          ),
        ),
    ),
    h(
      "div",
      { class: "confirm-preview" },
      h(
        "div",
        {
          class: "wizard-step-heading",
          style: "font-size:0.8rem;color:#888;margin-bottom:0.5rem",
        },
        "Preview",
      ),
      h(
        "div",
        { class: "preview-frame" },
        h(WidgetContent, {
          widget: {
            ...widget,
            title,
            colSpan,
            settings: { ...settings, variant },
          },
        }),
      ),
    ),
    h(
      "button",
      { class: "wizard-remove-btn", onClick: onRemove },
      "Delete Widget",
    ),
  );
}

function FetchHeadersEditor({
  headers,
  onChange,
}: {
  headers: string;
  onChange: (v: string) => void;
}) {
  const [error, setError] = useState<string | null>(null);

  function handleChange(value: string) {
    if (!value.trim()) {
      setError(null);
      onChange(value);
      return;
    }
    try {
      JSON.parse(value);
      setError(null);
      onChange(value);
    } catch {
      setError("Invalid JSON");
      onChange(value);
    }
  }

  return h(
    "div",
    { class: "wizard-field" },
    h("label", null, "Request Headers", h("span", { class: "wizard-field-hint" }, " (JSON)")),
    h("textarea", {
      class: `wizard-json-editor ${error ? "has-error" : ""}`,
      value: headers,
      placeholder: '{\n  "Authorization": "Bearer token",\n  "X-Custom": "value"\n}',
      onInput: (e: Event) => handleChange((e.target as HTMLTextAreaElement).value),
      rows: 4,
    }),
    error ? h("span", { class: "wizard-field-error" }, error) : null,
  );
}

function FetchBodyEditor({
  method,
  body,
  onChange,
}: {
  method: string;
  body: string;
  onChange: (v: string) => void;
}) {
  const [error, setError] = useState<string | null>(null);
  const hasBody = ["POST", "PUT", "PATCH"].includes(method);

  if (!hasBody) {
    return null;
  }

  function handleChange(value: string) {
    if (!value.trim()) {
      setError(null);
      onChange(value);
      return;
    }
    try {
      JSON.parse(value);
      setError(null);
      onChange(value);
    } catch {
      setError("Invalid JSON");
      onChange(value);
    }
  }

  return h(
    "div",
    { class: "wizard-field" },
    h("label", null, "Request Body", h("span", { class: "wizard-field-hint" }, " (JSON)")),
    h("textarea", {
      class: `wizard-json-editor ${error ? "has-error" : ""}`,
      value: body,
      placeholder: '{\n  "key": "value"\n}',
      onInput: (e: Event) => handleChange((e.target as HTMLTextAreaElement).value),
      rows: 4,
    }),
    error ? h("span", { class: "wizard-field-error" }, error) : null,
  );
}
