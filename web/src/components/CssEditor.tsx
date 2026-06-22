import { h } from "preact";
import { useState } from "preact/hooks";
import { t } from "../lib/i18n";

interface CssEditorProps {
  value: string;
  onChange: (value: string) => void;
  onClose: () => void;
}

export function CssEditor({ value, onChange, onClose }: CssEditorProps) {
  const [css, setCss] = useState(value);

  function handleSave() {
    onChange(css);
    onClose();
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
        h("h3", { class: "wizard-title" }, "Custom CSS"),
        h(
          "button",
          { class: "wizard-close", onClick: onClose },
          "\u00D7"
        )
      ),
      h(
        "div",
        { class: "wizard-body" },
        h(
          "p",
          { class: "css-editor-hint" },
          "Add custom CSS to style your dashboard. Use CSS variables like --bg-card, --accent, --text."
        ),
        h("textarea", {
          class: "css-editor-textarea",
          value: css,
          onInput: (e: Event) => setCss((e.target as HTMLTextAreaElement).value),
          placeholder: "/* Example:\n.dashboard-widget {\n  border-radius: 12px;\n}\n*/",
          spellcheck: false,
        })
      ),
      h(
        "div",
        { class: "wizard-footer" },
        h(
          "button",
          { class: "wizard-cancel-btn", onClick: onClose },
          t("widget.wizard.cancel")
        ),
        h(
          "button",
          { class: "wizard-apply-btn", onClick: handleSave },
          t("widget.wizard.save")
        )
      )
    )
  );
}
