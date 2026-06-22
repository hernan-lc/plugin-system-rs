import { h } from "preact";
import {
  FormField,
  FormInput,
  FormSelect,
  FormTextarea,
  KeyValueEditor,
  CollapsibleSection,
} from "../FormComponents";

interface FetchConfigProps {
  settings: Record<string, any>;
  updateSetting: (key: string, value: any) => void;
}

export function FetchConfig({
  settings,
  updateSetting,
}: FetchConfigProps) {
  const method = settings.method || "GET";
  const hasBody = ["POST", "PUT", "PATCH"].includes(method);

  return h(
    "div",
    { class: "wizard-step-content" },

    h(
      FormField,
      { label: "URL" },
      h(FormInput, {
        value: settings.url || "",
        placeholder: "https://api.example.com/data",
        onInput: (v) => updateSetting("url", v),
      }),
    ),

    h(
      "div",
      { class: "wizard-field-row" },
      h(
        FormField,
        { label: "Fetch Mode" },
        h(FormSelect, {
          value: settings.mode || "proxy",
          options: [
            { value: "local", label: "Local (Browser)" },
            { value: "proxy", label: "Proxy (Backend)" },
          ],
          onChange: (v) => updateSetting("mode", v),
        }),
      ),
      h(
        FormField,
        { label: "HTTP Method" },
        h(FormSelect, {
          value: method,
          options: [
            { value: "GET", label: "GET" },
            { value: "POST", label: "POST" },
            { value: "PUT", label: "PUT" },
            { value: "DELETE", label: "DELETE" },
            { value: "PATCH", label: "PATCH" },
          ],
          onChange: (v) => updateSetting("method", v),
        }),
      ),
    ),

    h(
      "div",
      { class: "wizard-field-row" },
      h(
        FormField,
        { label: "Fetch Mode" },
        h(FormSelect, {
          value: settings.fetchMode || "auto",
          options: [
            { value: "once", label: "Once (manual refresh)" },
            { value: "auto", label: "Auto (interval)" },
          ],
          onChange: (v) => updateSetting("fetchMode", v),
        }),
      ),
      settings.fetchMode !== "once" &&
        h(
          FormField,
          { label: "Interval", hint: "seconds" },
          h(FormInput, {
            type: "number",
            value: String(settings.intervalSec || 30),
            onInput: (v) => updateSetting("intervalSec", parseInt(v) || 30),
          }),
        ),
    ),

    h(
      CollapsibleSection,
      { title: "Headers", defaultOpen: !!(settings.headers && settings.headers.trim()) },
      h(KeyValueEditor, {
        value: settings.headers || "",
        placeholder: { key: "Authorization", value: "Bearer token" },
        onChange: (v) => updateSetting("headers", v),
      }),
    ),

    hasBody &&
      h(
        CollapsibleSection,
        { title: "Request Body", defaultOpen: true },
        h(
          "div",
          { class: "wizard-step-content" },
          h(
            FormField,
            { label: "Body Type" },
            h(FormSelect, {
              value: settings.bodyType || "json",
              options: [
                { value: "json", label: "JSON" },
                { value: "raw", label: "Raw Text" },
                { value: "form", label: "Form Data" },
              ],
              onChange: (v) => updateSetting("bodyType", v),
            }),
          ),
          settings.bodyType === "json" &&
            h(
              FormField,
              { label: "JSON Body" },
              h(FormTextarea, {
                value: settings.body || "",
                placeholder: '{\n  "key": "value"\n}',
                rows: 5,
                onInput: (v) => updateSetting("body", v),
              }),
            ),
          settings.bodyType === "raw" &&
            h(
              FormField,
              { label: "Raw Body" },
              h(FormTextarea, {
                value: settings.body || "",
                placeholder: "Raw request body...",
                rows: 5,
                onInput: (v) => updateSetting("body", v),
              }),
            ),
          settings.bodyType === "form" &&
            h(
              FormField,
              { label: "Form Fields" },
              h(KeyValueEditor, {
                value: settings.body || "",
                placeholder: { key: "field", value: "value" },
                onChange: (v) => updateSetting("body", v),
              }),
            ),
        ),
      ),
  );
}
