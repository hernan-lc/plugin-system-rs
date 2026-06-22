import { h } from "preact";
import { useState } from "preact/hooks";
import { WidgetConfig } from "../lib/types";
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
          h(WizardConfig, {
            widget,
            settings,
            onChange: setSettings,
            updateSetting,
          }),
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
