import { h } from "preact";
import { useState, useEffect } from "preact/hooks";

interface FetchWidgetSettings {
  url: string;
  mode: "local" | "proxy";
  method: string;
  headers: string;
  body: string;
  refreshInterval: number;
  variant?: string;
}

export function FetchWidget({ settings }: { settings: Record<string, any> }) {
  const [data, setData] = useState<any>(null);
  const [status, setStatus] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const {
    url,
    mode,
    method,
    headers: rawHeaders,
    body: rawBody,
    refreshInterval,
    variant = "compact",
  } = settings as FetchWidgetSettings;

  function parseHeaders(): Record<string, string> {
    if (!rawHeaders?.trim()) return {};
    try {
      return JSON.parse(rawHeaders);
    } catch {
      return {};
    }
  }

  function parseBody(): any {
    if (!rawBody?.trim()) return undefined;
    try {
      return JSON.parse(rawBody);
    } catch {
      return rawBody;
    }
  }

  useEffect(() => {
    if (!url) return;

    const customHeaders = parseHeaders();
    const parsedBody = parseBody();

    let active = true;
    const fetchData = async () => {
      setLoading(true);
      try {
        if (mode === "proxy") {
          const res = await fetch("/api/proxy", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              url,
              method,
              headers: customHeaders,
              body: ["POST", "PUT", "PATCH"].includes(method) ? parsedBody : undefined,
            }),
          });
          const json = await res.json();
          if (active) {
            if (json.success) {
              setData(json.data.body);
              setStatus(json.data.status);
              setError(null);
            } else {
              setError(json.error || "Proxy error");
            }
          }
        } else {
          const fetchOptions: RequestInit = {
            method,
            headers: customHeaders,
          };
          if (["POST", "PUT", "PATCH"].includes(method) && parsedBody) {
            fetchOptions.body = typeof parsedBody === "string" ? parsedBody : JSON.stringify(parsedBody);
          }
          const res = await fetch(url, fetchOptions);
          const status = res.status;
          const contentType = res.headers.get("content-type");
          let body;
          if (contentType && contentType.includes("application/json")) {
            body = await res.json();
          } else {
            body = await res.text();
          }
          if (active) {
            setData(body);
            setStatus(status);
            setError(null);
          }
        }
      } catch (e: any) {
        if (active) {
          setError(e.message);
          setData(null);
          setStatus(null);
        }
      } finally {
        if (active) setLoading(false);
      }
    };

    fetchData();
    const interval = setInterval(fetchData, refreshInterval || 30000);
    return () => {
      active = false;
      clearInterval(interval);
    };
  }, [url, mode, method, rawHeaders, rawBody, refreshInterval]);

  const renderContent = () => {
    if (!url) return h("div", { class: "fetch-no-url" }, "No URL configured");
    if (error) return h("div", { class: "fetch-error" }, error);
    if (loading && !data) return h("div", { class: "fetch-loading" }, "...");

    const displayData = typeof data === "object" ? JSON.stringify(data).substring(0, 100) : String(data).substring(0, 100);

    if (variant === "minimal") {
      return h("div", { class: "fetch-variant minimal" },
        status ? h("div", { class: `fetch-status ${status < 400 ? "ok" : "err"}` }, status) : "???"
      );
    }

    if (variant === "compact") {
      return h("div", { class: "fetch-variant compact" },
        h("div", { class: "fetch-url-mini" }, url.split("/")[2] || url),
        h("div", { class: "fetch-preview" }, displayData),
        status && h("div", { class: "fetch-status-line" }, `Status: ${status}`)
      );
    }

    return h("div", { class: "fetch-variant detailed" },
      h("div", { class: "fetch-url-line" }, url),
      h("pre", { class: "fetch-full-preview" },
        typeof data === "object" ? JSON.stringify(data, null, 2).substring(0, 500) : String(data).substring(0, 500)
      )
    );
  };

  return h("div", { class: `fetch-widget ${variant}` }, renderContent());
}
