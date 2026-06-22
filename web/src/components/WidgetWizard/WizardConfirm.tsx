import { h } from "preact";
import { WidgetConfig } from "../../lib/types";
import { WidgetContent } from "../WidgetContent";

interface WizardConfirmProps {
  widget: WidgetConfig;
  title: string;
  colSpan: number;
  settings: Record<string, any>;
  variant: string;
  onApply: () => void;
  onRemove: () => void;
}

export function WizardConfirm({
  widget,
  title,
  colSpan,
  settings,
  variant,
  onApply,
  onRemove,
}: WizardConfirmProps) {
  return h(
    "div",
    { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, "Confirm Changes"),
    h("p", { class: "wizard-step-desc" }, "Review your widget configuration before saving"),
    h(
      "div",
      { class: "confirm-details" },
      h("div", { class: "confirm-row" }, h("span", null, "Title"), h("span", null, title)),
      h("div", { class: "confirm-row" }, h("span", null, "Span"), h("span", null, `${colSpan} column${colSpan > 1 ? "s" : ""}`)),
      h("div", { class: "confirm-row" }, h("span", null, "Variant"), h("span", null, variant)),
      ...Object.entries(settings)
        .filter(([k]) => k !== "variant")
        .map(([k, v]) =>
          h(
            "div",
            { class: "confirm-row", key: k },
            h("span", null, k),
            h("span", null, String(v).substring(0, 60)),
          ),
        ),
    ),
    h(
      "div",
      { class: "confirm-preview" },
      h("div", { class: "wizard-step-heading", style: "font-size:0.8rem;color:#888;margin-bottom:0.5rem" }, "Preview"),
      h(
        "div",
        { class: "preview-frame" },
        h(WidgetContent, {
          widget: { ...widget, title, colSpan, settings: { ...settings, variant } },
        }),
      ),
    ),
    h("button", { class: "wizard-remove-btn", onClick: onRemove }, "Delete Widget"),
  );
}
