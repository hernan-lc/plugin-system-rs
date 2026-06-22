type TranslationMap = { [key: string]: string | TranslationMap };

let currentLocale: string = "en";
let translations: TranslationMap = {};
const localeModules: Record<string, () => Promise<TranslationMap>> = {
  en: () => import("./locales/en.json").then((m) => m.default),
  es: () => import("./locales/es.json").then((m) => m.default),
  pt: () => import("./locales/pt.json").then((m) => m.default),
};

function getNestedValue(obj: TranslationMap, path: string): string | undefined {
  const parts = path.split(".");
  let current: any = obj;
  for (const part of parts) {
    if (current === null || current === undefined) {
      return undefined;
    }
    current = current[part];
  }
  return typeof current === "string" ? current : undefined;
}

export function t(key: string, params?: Record<string, string | number>): string {
  let value = getNestedValue(translations, key);
  if (value === undefined) {
    value = key;
  }
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      value = value.replace(new RegExp(`\\{${k}\\}`, "g"), String(v));
    }
  }
  return value;
}

export async function setLocale(locale: string): Promise<void> {
  const loader = localeModules[locale];
  if (!loader) {
    console.warn(`Locale "${locale}" not available`);
    return;
  }
  try {
    translations = await loader();
    currentLocale = locale;
    localStorage.setItem("sd-locale", locale);
    document.documentElement.setAttribute("lang", locale);
  } catch (e) {
    console.error(`Failed to load locale "${locale}":`, e);
  }
}

export function getLocale(): string {
  return currentLocale;
}

export function getAvailableLocales(): string[] {
  return Object.keys(localeModules);
}

export async function initI18n(): Promise<void> {
  const saved = localStorage.getItem("sd-locale");
  const browserLang = navigator.language.split("-")[0];
  const locale = saved || (localeModules[browserLang] ? browserLang : "en");
  await setLocale(locale);
}
