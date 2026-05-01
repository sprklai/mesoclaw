/**
 * i18n parity tests — assert every locale file contains the same set of keys
 * as the canonical English locale (web/messages/en.json).
 *
 * If this test fails it means a key was added to en.json without being added to
 * one or more locale files.  Fix: add the missing key(s) with an "[EN] ..."
 * placeholder value so the UI doesn't break, then arrange for a proper
 * translation.
 */

import { describe, it, expect } from "vitest";
import { readFileSync } from "fs";
import { resolve } from "path";

const MESSAGES_DIR = resolve(__dirname, "../../messages");

function loadLocale(filename: string): Record<string, string> {
  const raw = readFileSync(resolve(MESSAGES_DIR, filename), "utf-8");
  return JSON.parse(raw) as Record<string, string>;
}

const enRaw = loadLocale("en.json");
const enKeys = new Set(
  Object.keys(enRaw).filter((k) => !k.startsWith("_"))
);

const NON_ENGLISH_LOCALES = [
  "es.json",
  "fr.json",
  "hi.json",
  "ja.json",
  "ko.json",
  "pt-BR.json",
  "zh-CN.json",
];

describe("i18n parity", () => {
  for (const filename of NON_ENGLISH_LOCALES) {
    it(`${filename} has the same key set as en.json`, () => {
      const locale = loadLocale(filename);
      const localeKeys = new Set(
        Object.keys(locale).filter((k) => !k.startsWith("_"))
      );

      const missing = [...enKeys].filter((k) => !localeKeys.has(k));
      const extra = [...localeKeys].filter((k) => !enKeys.has(k));

      expect(
        missing,
        `Keys in en.json missing from ${filename}: ${missing.join(", ")}`
      ).toHaveLength(0);

      expect(
        extra,
        `Keys in ${filename} not present in en.json (remove them): ${extra.join(", ")}`
      ).toHaveLength(0);
    });
  }
});
