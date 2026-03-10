// Follows the best practices established in https://shiki.matsu.io/guide/best-performance
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";
import { createHighlighterCore } from "shiki/core";
import githubLightDefault from "@shikijs/themes/github-light-default";
import githubDarkDefault from "@shikijs/themes/github-dark-default";

const bundledLanguages = {
  bash: () => import("@shikijs/langs/bash"),
  diff: () => import("@shikijs/langs/diff"),
  javascript: () => import("@shikijs/langs/javascript"),
  json: () => import("@shikijs/langs/json"),
  svelte: () => import("@shikijs/langs/svelte"),
  typescript: () => import("@shikijs/langs/typescript"),
  python: () => import("@shikijs/langs/python"),
  tsx: () => import("@shikijs/langs/tsx"),
  jsx: () => import("@shikijs/langs/jsx"),
  css: () => import("@shikijs/langs/css"),
  text: () => import("@shikijs/langs/markdown"),
};

/** The languages configured for the highlighter */
export type SupportedLanguage = keyof typeof bundledLanguages;

/** Shared Shiki theme objects for use by both the Code element and Streamdown. */
export const shikiThemes = {
  "github-light-default": githubLightDefault,
  "github-dark-default": githubDarkDefault,
} as const;

/** A preloaded highlighter instance. */
export const highlighter = createHighlighterCore({
  themes: [
    import("@shikijs/themes/github-light-default"),
    import("@shikijs/themes/github-dark-default"),
    import("@shikijs/themes/vesper"),
  ],
  langs: Object.entries(bundledLanguages).map(([_, lang]) => lang),
  engine: createJavaScriptRegexEngine(),
});
