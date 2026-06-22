import { h } from "preact";
import { useState } from "preact/hooks";
import { WidgetConfig } from "../lib/types";
import { widgetHasConfig } from "./widgetHelpers";
import { WizardGeneral } from "./WidgetWizard/WizardGeneral";
import { WizardConfig } from "./WidgetWizard/WizardConfig";
import { WizardStyle } from "./WidgetWizard/WizardStyle";
import { WizardConfirm } from "./WidgetWizard/WizardConfirm";

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
  const hasConfig = widgetHasConfig(widget.type);
  const stepDefs = [
    { label: "General", render: () => h(WizardGeneral, {
      title, colSpan, columns,
      onChangeTitle: setTitle, onChangeColSpan: setColSpan,
    })},
    ...(hasConfig ? [{ label: "Config", render: () => h(WizardConfig, {
      widget, settings, onChange: setSettings, updateSetting,
    })}] : []),
    { label: "Style", render: () => h(WizardStyle, { widget, variant, onChange: setVariant }) },
    { label: "Apply", render: () => h(WizardConfirm, {
      widget, title, colSpan, settings, variant,
      onApply: handleApply, onRemove,
    }) },
  ];
  const totalSteps = stepDefs.length;

  function handleNext() {
    if (step < totalSteps - 1) setStep(step + 1);
  }
  function handleBack() {
    if (step > 0) setStep(step - 1);
  }
  function handleApply() {
    onSave(widget.id, { title, colSpan, settings: { ...settings, variant } });
  }

  function updateSetting(key: string, value: any) {
    setSettings((prev) => ({ ...prev, [key]: value }));
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
        h("button", { class: "picker-close", onClick: onClose }, "\u2715"),
      ),
      h(
        "div",
        { class: "wizard-steps" },
        stepDefs.map((s, i) =>
          h(
            "div",
            {
              class: `wizard-step-indicator ${i === step ? "active" : i < step ? "done" : ""}`,
              key: s.label,
              onClick: () => setStep(i),
            },
            h("div", { class: "wizard-step-circle" }, s.label[0]),
            h("div", { class: "wizard-step-label" }, s.label),
          ),
        ),
      ),
      h(
        "div",
        { class: "wizard-body" },
        stepDefs[step].render(),
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
