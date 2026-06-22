import { h } from "preact";
import { WidgetConfig, WIDGET_VARIANTS } from "../../lib/types";

interface WizardStyleProps {
  widget: WidgetConfig;
  variant: string;
  onChange: (v: string) => void;
}

export function WizardStyle({
  widget,
  variant,
  onChange,
}: WizardStyleProps) {
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

interface VariantPreviewProps {
  type: string;
  variant: string;
}

function VariantPreview({ type, variant }: VariantPreviewProps) {
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
            h("div", { class: "mini-bar" }, h("div", { class: "mini-bar-fill", style: { width: "42%", background: "#4caf50" } })),
            h("div", { class: "mini-bar" }, h("div", { class: "mini-bar-fill", style: { width: "56%", background: "#2196f3" } })),
          );
        case "detailed":
          return h(
            "div",
            { class: "variant-preview sysmon-detailed" },
            h("div", { class: "mini-grid" }, h("div", null, "42%"), h("div", null, "56%"), h("div", null, "1.2"), h("div", null, "2d")),
          );
      }
    case "clock":
      switch (variant) {
        case "simple":
          return h("div", { class: "variant-preview clock-simple" }, "14:30");
        case "digital":
          return h("div", { class: "variant-preview clock-digital" }, "14:30", h("div", { class: "mini-sec" }, "15"), h("div", { class: "mini-date" }, "Mon"));
        case "detailed":
          return h("div", { class: "variant-preview clock-detailed" }, "14:30:15", h("div", { class: "mini-date" }, "Monday, Jun 10"));
      }
    case "volume-master":
      switch (variant) {
        case "minimal":
          return h("div", { class: "variant-preview vol-minimal" }, h("div", null, "75%"), h("div", { class: "mini-btn" }, "MUTE"));
        case "compact":
          return h("div", { class: "variant-preview vol-compact" }, h("div", { class: "mini-bar" }, h("div", { class: "mini-bar-fill", style: { width: "75%", background: "#4caf50" } })), h("div", null, "Speaker"));
        case "detailed":
          return h("div", { class: "variant-preview vol-detailed" }, h("div", null, "75%"), h("div", { class: "mini-bar" }, h("div", { class: "mini-bar-fill", style: { width: "75%", background: "#4caf50" } })), h("div", { class: "mini-apps" }, "Apps: 2"));
      }
    case "volume-apps":
      switch (variant) {
        case "minimal":
          return h("div", { class: "variant-preview volapps-minimal" }, h("div", null, "3 apps"), h("div", { class: "mini-list" }, "Firefox, Spotify"));
        case "compact":
          return h("div", { class: "variant-preview volapps-compact" }, h("div", null, "Firefox"), h("div", { class: "mini-bar" }, h("div", { class: "mini-bar-fill", style: { width: "60%", background: "#4caf50" } })));
        case "detailed":
          return h("div", { class: "variant-preview volapps-detailed" }, h("div", null, "Firefox (PID: 1234)"), h("div", { class: "mini-bar" }, h("div", { class: "mini-bar-fill", style: { width: "60%", background: "#4caf50" } })), h("div", null, "60%"));
      }
    case "obs-control":
      switch (variant) {
        case "minimal":
          return h("div", { class: "variant-preview obs-minimal" }, h("div", { class: "mini-row" }, h("div", { class: "mini-dot green" }), h("span", null, "Connected")), h("div", { class: "mini-row" }, h("div", { class: "mini-dot red" }), h("span", null, "Stream")));
        case "compact":
          return h("div", { class: "variant-preview obs-compact" }, h("div", null, "Scene 1"), h("div", { class: "mini-btns" }, h("div", { class: "mini-btn" }, "STR"), h("div", { class: "mini-btn" }, "REC"), h("div", { class: "mini-btn" }, "VC")));
        case "detailed":
          return h("div", { class: "variant-preview obs-detailed" }, h("div", { class: "mini-btns" }, h("div", { class: "mini-btn active" }, "Stream"), h("div", { class: "mini-btn" }, "Record")), h("div", { class: "mini-grid" }, h("div", null, "CPU"), h("div", null, "FPS")));
      }
    case "obs-scenes":
      switch (variant) {
        case "minimal":
          return h("div", { class: "variant-preview obscene-minimal" }, h("div", null, "Scene 1"), h("div", { class: "mini-grid" }, h("div", { class: "mini-btn active" }, "S1"), h("div", { class: "mini-btn" }, "S2")));
        case "compact":
          return h("div", { class: "variant-preview obscene-compact" }, h("div", { class: "mini-list" }, h("div", { class: "mini-btn active" }, "Scene 1"), h("div", { class: "mini-btn" }, "Scene 2")));
        case "detailed":
          return h("div", { class: "variant-preview obscene-detailed" }, h("div", { class: "mini-list" }, h("div", { class: "mini-btn active" }, "Scene 1"), h("div", { class: "mini-btn" }, "Scene 2")), h("div", { class: "mini-btns" }, h("div", { class: "mini-btn" }, "Fade")));
      }
    case "obs-inputs":
      switch (variant) {
        case "minimal":
          return h("div", { class: "variant-preview obsinput-minimal" }, h("div", null, "3 inputs"), h("div", { class: "mini-list" }, h("div", { class: "mini-row" }, h("span", null, "Mic"), h("div", { class: "mini-btn" }, "M"))));
        case "compact":
          return h("div", { class: "variant-preview obsinput-compact" }, h("div", null, "Mic"), h("div", { class: "mini-bar" }, h("div", { class: "mini-bar-fill", style: { width: "75%", background: "#4caf50" } })));
        case "detailed":
          return h("div", { class: "variant-preview obsinput-detailed" }, h("div", null, "Mic (audio)"), h("div", { class: "mini-bar" }, h("div", { class: "mini-bar-fill", style: { width: "75%", background: "#4caf50" } })), h("div", null, "75%"));
      }
    case "fetch":
      switch (variant) {
        case "minimal":
          return h("div", { class: "variant-preview fetch-minimal" }, h("div", { class: "mini-status ok" }, "200"));
        case "compact":
          return h("div", { class: "variant-preview fetch-compact" }, h("div", { class: "mini-url" }, "api.ex..."), h("div", { class: "mini-data" }, '{"id":1...}'));
        case "detailed":
          return h("div", { class: "variant-preview fetch-detailed" }, h("div", { class: "mini-url" }, "https://api.example.com/v1"), h("div", { class: "mini-json" }, '{\n  "status": "ok",\n  "data": [...]\n}'));
      }
    default:
      return h(
        "div",
        { class: "variant-preview simple-preview" },
        h("div", { class: variant === "compact" ? "preview-btn-sm" : "preview-btn-lg" }, "Action"),
      );
  }
}
