import { h } from "preact";
import { FormField, FormInput } from "../FormComponents";

interface IntervalFieldProps {
  value: number;
  min?: number;
  onChange: (v: number) => void;
}

export function IntervalField({
  value,
  min = 500,
  onChange,
}: IntervalFieldProps) {
  return h(
    FormField,
    { label: "Refresh Interval", hint: "ms" },
    h(FormInput, {
      type: "number",
      value: String(value),
      onInput: (v) => onChange(parseInt(v) || min),
    }),
  );
}
