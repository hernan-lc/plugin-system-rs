import { h, Fragment, ComponentChildren } from "preact";
import { useState } from "preact/hooks";

/* ── Field Wrapper ────────────────────────────────────── */

export function FormField({
  label,
  hint,
  children,
  class: cls,
}: {
  label?: string;
  hint?: string;
  children?: ComponentChildren;
  class?: string;
}) {
  return h(
    "div",
    { class: `form-field ${cls || ""}` },
    label
      ? h(
          "label",
          { class: "form-label" },
          label,
          hint ? h("span", { class: "form-hint" }, hint) : null
        )
      : null,
    children
  );
}

/* ── Input ────────────────────────────────────────────── */

export function FormInput({
  value,
  placeholder,
  type = "text",
  onInput,
  class: cls,
}: {
  value: string;
  placeholder?: string;
  type?: string;
  onInput: (v: string) => void;
  class?: string;
}) {
  return h("input", {
    class: `form-input ${cls || ""}`,
    type,
    value,
    placeholder,
    onInput: (e: Event) => onInput((e.target as HTMLInputElement).value),
  });
}

/* ── Textarea ─────────────────────────────────────────── */

export function FormTextarea({
  value,
  placeholder,
  rows = 3,
  onInput,
  class: cls,
}: {
  value: string;
  placeholder?: string;
  rows?: number;
  onInput: (v: string) => void;
  class?: string;
}) {
  return h("textarea", {
    class: `form-textarea ${cls || ""}`,
    value,
    placeholder,
    rows,
    onInput: (e: Event) => onInput((e.target as HTMLTextAreaElement).value),
  });
}

/* ── Select ───────────────────────────────────────────── */

export interface SelectOption {
  value: string;
  label: string;
}

export function FormSelect({
  value,
  options,
  onChange,
  class: cls,
}: {
  value: string;
  options: SelectOption[];
  onChange: (v: string) => void;
  class?: string;
}) {
  return h(
    "select",
    {
      class: `form-select ${cls || ""}`,
      value,
      onChange: (e: Event) => onChange((e.target as HTMLSelectElement).value),
    },
    options.map((opt) =>
      h("option", { key: opt.value, value: opt.value }, opt.label)
    )
  );
}

/* ── Toggle ───────────────────────────────────────────── */

export function FormToggle({
  checked,
  disabled,
  onChange,
}: {
  checked: boolean;
  disabled?: boolean;
  onChange: (v: boolean) => void;
}) {
  return h(
    "label",
    { class: "form-toggle" },
    h("input", {
      type: "checkbox",
      checked,
      disabled,
      onChange: (e: Event) => onChange((e.target as HTMLInputElement).checked),
    }),
    h("span", { class: "form-toggle-slider" })
  );
}

/* ── Key-Value Pair List ──────────────────────────────── */

export interface KeyValueEntry {
  key: string;
  value: string;
  enabled: boolean;
}

function parseKeyValuePairs(raw: string): KeyValueEntry[] {
  if (!raw?.trim()) return [];
  try {
    const obj = JSON.parse(raw);
    if (typeof obj === "object" && obj !== null && !Array.isArray(obj)) {
      return Object.entries(obj).map(([k, v]) => ({
        key: k,
        value: String(v),
        enabled: true,
      }));
    }
  } catch {}
  return [];
}

function serializeKeyValuePairs(entries: KeyValueEntry[]): string {
  const obj: Record<string, string> = {};
  for (const e of entries) {
    if (e.enabled && e.key.trim()) {
      obj[e.key.trim()] = e.value;
    }
  }
  return Object.keys(obj).length > 0 ? JSON.stringify(obj, null, 2) : "";
}

export function KeyValueEditor({
  value,
  placeholder,
  onChange,
}: {
  value: string;
  placeholder?: { key: string; value: string };
  onChange: (v: string) => void;
}) {
  const [entries, setEntries] = useState<KeyValueEntry[]>(() =>
    parseKeyValuePairs(value)
  );
  const [expanded, setExpanded] = useState(entries.length > 0);

  function sync(next: KeyValueEntry[]) {
    setEntries(next);
    onChange(serializeKeyValuePairs(next));
  }

  function addEntry() {
    sync([...entries, { key: "", value: "", enabled: true }]);
    setExpanded(true);
  }

  function removeEntry(index: number) {
    sync(entries.filter((_, i) => i !== index));
  }

  function updateEntry(index: number, field: "key" | "value", val: string) {
    const next = entries.map((e, i) =>
      i === index ? { ...e, [field]: val } : e
    );
    sync(next);
  }

  function toggleEntry(index: number) {
    const next = entries.map((e, i) =>
      i === index ? { ...e, enabled: !e.enabled } : e
    );
    sync(next);
  }

  const activeCount = entries.filter((e) => e.enabled && e.key.trim()).length;

  return h(
    "div",
    { class: "kv-editor" },
    h(
      "div",
      { class: "kv-header" },
      h(
        "button",
        {
          class: "kv-expand-btn",
          type: "button",
          onClick: () => setExpanded(!expanded),
        },
        h("span", { class: `kv-chevron ${expanded ? "open" : ""}` }, "\u25B6"),
        h(
          "span",
          { class: "kv-summary" },
          activeCount > 0
            ? `${activeCount} item${activeCount !== 1 ? "s" : ""}`
            : "None"
        )
      ),
      h(
        "button",
        { class: "kv-add-btn", type: "button", onClick: addEntry },
        "+ Add"
      )
    ),
    expanded &&
      h(
        "div",
        { class: "kv-list" },
        entries.length === 0
          ? h(
              "div",
              { class: "kv-empty" },
              "No entries. Click + Add to create one."
            )
          : entries.map((entry, i) =>
              h(
                "div",
                { class: `kv-row ${entry.enabled ? "" : "disabled"}`, key: i },
                h(FormToggle, {
                  checked: entry.enabled,
                  onChange: () => toggleEntry(i),
                }),
                h("input", {
                  class: "kv-key",
                  type: "text",
                  value: entry.key,
                  placeholder: placeholder?.key || "Key",
                  onInput: (e: Event) =>
                    updateEntry(i, "key", (e.target as HTMLInputElement).value),
                }),
                h("input", {
                  class: "kv-value",
                  type: "text",
                  value: entry.value,
                  placeholder: placeholder?.value || "Value",
                  onInput: (e: Event) =>
                    updateEntry(
                      i,
                      "value",
                      (e.target as HTMLInputElement).value
                    ),
                }),
                h(
                  "button",
                  {
                    class: "kv-remove-btn",
                    type: "button",
                    onClick: () => removeEntry(i),
                  },
                  "\u00D7"
                )
              )
            )
      )
  );
}

/* ── Collapsible Section ──────────────────────────────── */

export function CollapsibleSection({
  title,
  defaultOpen = false,
  children,
}: {
  title: string;
  defaultOpen?: boolean;
  children?: ComponentChildren;
}) {
  const [open, setOpen] = useState(defaultOpen);

  return h(
    "div",
    { class: "collapsible" },
    h(
      "button",
      {
        class: "collapsible-trigger",
        type: "button",
        onClick: () => setOpen(!open),
      },
      h("span", { class: `kv-chevron ${open ? "open" : ""}` }, "\u25B6"),
      title
    ),
    open ? h("div", { class: "collapsible-content" }, children) : null
  );
}

/* ── Tabs ─────────────────────────────────────────────── */

export function FormTabs({
  tabs,
  active,
  onChange,
}: {
  tabs: { value: string; label: string }[];
  active: string;
  onChange: (v: string) => void;
}) {
  return h(
    "div",
    { class: "form-tabs" },
    tabs.map((tab) =>
      h(
        "button",
        {
          key: tab.value,
          class: `form-tab ${active === tab.value ? "active" : ""}`,
          type: "button",
          onClick: () => onChange(tab.value),
        },
        tab.label
      )
    )
  );
}
