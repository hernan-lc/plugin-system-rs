import { h } from "preact";
import { FormField, FormInput } from "../FormComponents";

interface WizardGeneralProps {
  title: string;
  colSpan: number;
  columns: number;
  onChangeTitle: (t: string) => void;
  onChangeColSpan: (c: number) => void;
}

export function WizardGeneral({
  title,
  colSpan,
  columns,
  onChangeTitle,
  onChangeColSpan,
}: WizardGeneralProps) {
  return h(
    "div",
    { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, "General Settings"),
    h(
      FormField,
      { label: "Widget Title" },
      h(FormInput, {
        value: title,
        placeholder: "Enter widget title...",
        onInput: onChangeTitle,
      }),
    ),
    h(
      FormField,
      { label: "Column Span", hint: `Grid has ${columns} columns` },
      h(FormInput, {
        type: "number",
        value: String(colSpan),
        onInput: (v) => onChangeColSpan(parseInt(v) || 1),
      }),
    ),
  );
}
