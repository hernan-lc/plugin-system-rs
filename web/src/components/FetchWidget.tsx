import { h } from "preact";
import { useState, useEffect } from "preact/hooks";

interface FetchWidgetSettings {
  url: string;
  mode: "local" | "proxy";
  method: string;
  fetchMode: "once" | "auto";
  intervalSec: number;
  headers: string;
  body: string;
  bodyType: "json" | "raw" | "form";
  variant?: string;
}

function parseHeaders(raw: string): Record<string, string> {
  if (!raw?.trim()) return {};
  try {
    const obj = JSON.parse(raw);
    if (typeof obj === "object" && obj !== null && !Array.isArray(obj)) {
      return obj;
    }
  } catch {}
  return {};
}

function parseBody(raw: string, bodyType: string): any {
  if (!raw?.trim()) return undefined;
  if (bodyType === "form") {
    try {
      const obj = JSON.parse(raw);
      if (typeof obj === "object" && obj !== null) {
        const params = new URLSearchParams();
        for (const [k, v] of Object.entries(obj)) {
          params.append(k, String(v));
        }
        return params.toString();
      }
    } catch {}
    return raw;
  }
  if (bodyType === "json") {
    try {
      JSON.parse(raw);
      return raw;
    } catch {
      return raw;
    }
  }
  return raw;
}

export function FetchWidget({ settings }: { settings: Record<string, any> }) {
  const [data, setData] = useState<any>(null);
  const [status, setStatus] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [lastFetched, setLastFetched] = useState<number | null>(null);

  const {
    url,
    mode,
    method,
    fetchMode = "auto",
    intervalSec = 30,
    headers: rawHeaders,
    body: rawBody,
    bodyType = "json",
    variant = "compact",
  } = settings as FetchWidgetSettings;

  const hasBody = ["POST", "PUT", "PATCH"].includes(method);

  async function fetchData() {
    if (!url) return;
    setLoading(true);
    setError(null);
    try {
      const customHeaders = parseHeaders(rawHeaders);
      const parsedBody = hasBody ? parseBody(rawBody, bodyType) : undefined;

      if (mode === "proxy") {
        const res = await fetch("/api/proxy", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            url,
            method,
            headers: customHeaders,
            body: parsedBody,
          }),
        });
        const json = await res.json();
        if (json.success) {
          setData(json.data.body);
          setStatus(json.data.status);
          setError(null);
        } else {
          setError(json.error || "Proxy error");
        }
      } else {
        const fetchOptions: RequestInit = {
          method,
          headers: {
            ...customHeaders,
            ...(bodyType === "json" && parsedBody
              ? { "Content-Type": "application/json" }
              : {}),
          },
        };
        if (parsedBody) {
          fetchOptions.body = parsedBody;
        }
        const res = await fetch(url, fetchOptions);
        const st = res.status;
        const contentType = res.headers.get("content-type");
        let body;
        if (contentType && contentType.includes("application/json")) {
          body = await res.json();
        } else {
          body = await res.text();
        }
        setData(body);
        setStatus(st);
        setError(null);
      }
      setLastFetched(Date.now());
    } catch (e: any) {
      setError(e.message);
      setData(null);
      setStatus(null);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (!url) return;
    fetchData();

    if (fetchMode === "auto") {
      const interval = setInterval(fetchData, (intervalSec || 30) * 1000);
      return () => clearInterval(interval);
    }
  }, [url, mode, method, fetchMode, intervalSec, rawHeaders, rawBody, bodyType]);

  const renderContent = () => {
    if (!url) return h("div", { class: "fetch-no-url" }, "No URL configured");
    if (error) return h("div", { class: "fetch-error" }, error);
    if (loading && !data) return h("div", { class: "fetch-loading" }, "...");

    const displayData =
      typeof data === "object"
        ? JSON.stringify(data).substring(0, 100)
        : String(data).substring(0, 100);

    if (variant === "minimal") {
      return h(
        "div",
        { class: "fetch-variant minimal" },
        status
          ? h("div", { class: `fetch-status ${status < 400 ? "ok" : "err"}` }, status)
          : "???"
      );
    }

    if (variant === "compact") {
      return h(
        "div",
        { class: "fetch-variant compact" },
        h("div", { class: "fetch-url-mini" }, url.split("/")[2] || url),
        h("div", { class: "fetch-preview" }, displayData),
        status && h("div", { class: "fetch-status-line" }, `Status: ${status}`),
        fetchMode === "once" &&
          h(
            "button",
            { class: "fetch-refresh-btn", onClick: fetchData, disabled: loading },
            loading ? "..." : "\u21BB"
          )
      );
    }

    return h(
      "div",
      { class: "fetch-variant detailed" },
      h("div", { class: "fetch-url-line" }, url),
      h(
        "pre",
        { class: "fetch-full-preview" },
        typeof data === "object"
          ? JSON.stringify(data, null, 2).substring(0, 500)
          : String(data).substring(0, 500)
      ),
      h(
        "div",
        { class: "fetch-footer" },
        status && h("span", { class: "fetch-status-badge" }, `HTTP ${status}`),
        fetchMode === "once" &&
          h(
            "button",
            { class: "fetch-refresh-btn", onClick: fetchData, disabled: loading },
            loading ? "Refreshing..." : "Refresh"
          )
      )
    );
  };

  return h("div", { class: `fetch-widget ${variant}` }, renderContent());
}
