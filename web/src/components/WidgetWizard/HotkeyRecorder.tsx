import { h } from "preact";
import { useState } from "preact/hooks";
import { recordHotkey, resetHotkeyRecording } from "../../lib/api";

interface HotkeyRecorderProps {
  currentKeys: string;
  onChange: (keys: string) => void;
}

export function HotkeyRecorder({
  currentKeys,
  onChange,
}: HotkeyRecorderProps) {
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
    { key: "up", label: "\u2191" },
    { key: "down", label: "\u2193" },
    { key: "left", label: "\u2190" },
    { key: "right", label: "\u2192" },
  ];

  function toggleKey(key: string) {
    const lower = key.toLowerCase();
    setSelectedKeys((prev) =>
      prev.includes(lower)
        ? prev.filter((k) => k !== lower)
        : [...prev, lower]
    );
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
    h("label", { class: "form-label" }, "Hotkey Combination"),
    h(
      "div",
      { class: "hotkey-display" },
      h("span", { class: "hotkey-keys" }, combo || "Not set"),
      h(
        "button",
        { class: "hotkey-record-btn", onClick: () => setShowPicker(!showPicker) },
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
        h(
          "div",
          { class: "key-picker-section" },
          h("div", { class: "key-picker-label" }, "Selected:"),
          h(
            "div",
            { class: "key-picker-selected" },
            selectedKeys.length === 0
              ? h("span", { class: "key-picker-empty" }, "No keys selected")
              : selectedKeys.map((key) =>
                  h(
                    "span",
                    { class: "key-picker-chip", key, onClick: () => removeKey(key) },
                    key,
                    h("span", { class: "key-picker-chip-x" }, "\u00D7")
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
        h(
          "div",
          { class: "key-picker-section" },
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
        h(
          "div",
          { class: "key-picker-section" },
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
        h(
          "div",
          { class: "key-picker-section" },
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
        h(
          "div",
          { class: "key-picker-section" },
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
        h(
          "div",
          { class: "key-picker-section" },
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
