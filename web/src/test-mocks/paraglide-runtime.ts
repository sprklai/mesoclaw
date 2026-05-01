// Stub for $lib/paraglide/runtime — no-op in test environments.
export function setLanguageTag(_tag: string) {}
export function languageTag() { return "en"; }
export function onSetLanguageTag(_cb: (tag: string) => void) {}
export const availableLanguageTags = ["en"];
export const sourceLanguageTag = "en";
