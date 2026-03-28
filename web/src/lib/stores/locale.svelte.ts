import {
  locales,
  baseLocale,
  setLocale,
  getLocale,
} from "$lib/paraglide/runtime";

const LOCALE_KEY = "zenii_locale";

export type Locale = (typeof locales)[number];

/** Load native labels for all locales from their JSON message files at build time */
const localeModules = import.meta.glob("/messages/*.json", {
  eager: true,
  import: "default",
}) as Record<string, Record<string, string>>;

function getNativeLabels(): Record<string, string> {
  const labels: Record<string, string> = {};
  for (const loc of locales) {
    const key = `/messages/${loc}.json`;
    const mod = localeModules[key];
    labels[loc] = mod?._meta_label ?? loc;
  }
  return labels;
}

const nativeLabels = getNativeLabels();

function matchBrowserLocale(): Locale {
  if (typeof navigator === "undefined") return baseLocale as Locale;
  const browserLangs = navigator.languages ?? [navigator.language];
  for (const lang of browserLangs) {
    if ((locales as readonly string[]).includes(lang)) return lang as Locale;
    const prefix = lang.split("-")[0];
    const match = locales.find(
      (l) => l === prefix || l.startsWith(prefix + "-"),
    );
    if (match) return match;
  }
  return baseLocale as Locale;
}

function getInitialLocale(): Locale {
  if (typeof window === "undefined") return baseLocale as Locale;
  const stored = localStorage.getItem(LOCALE_KEY);
  if (stored && (locales as readonly string[]).includes(stored))
    return stored as Locale;
  return matchBrowserLocale();
}

function createLocaleStore() {
  let current = $state<Locale>(getInitialLocale());

  const initial = getInitialLocale();
  if (getLocale() !== initial) {
    setLocale(initial, { reload: false });
  }

  return {
    get locale(): Locale {
      return current;
    },

    get availableLocales(): readonly string[] {
      return locales;
    },

    set(value: Locale) {
      if (!(locales as readonly string[]).includes(value)) return;
      current = value;
      localStorage.setItem(LOCALE_KEY, value);
      setLocale(value, { reload: false });
    },

    /** Get the native display name for a locale (e.g. "日本語" for "ja") */
    label(locale: string): string {
      return nativeLabels[locale] ?? locale;
    },
  };
}

export const localeStore = createLocaleStore();
