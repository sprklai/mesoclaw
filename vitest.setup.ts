import "@testing-library/jest-dom";
import { cleanup } from "@testing-library/react";
import { expect, afterEach, vi } from "vitest";

// Cleanup after each test
afterEach(() => {
  cleanup();
});

// Extend Vitest's expect with jest-dom matchers
expect.extend({});

// Mock @lobehub/icons to avoid ESM directory import issues with @lobehub/fluent-emoji
vi.mock("@lobehub/icons", () => {
  const MockIcon = () => null;
  return {
    Anthropic: MockIcon,
    Claude: MockIcon,
    DeepSeek: MockIcon,
    Gemini: MockIcon,
    Google: MockIcon,
    Groq: MockIcon,
    Mistral: MockIcon,
    Ollama: MockIcon,
    OpenAI: MockIcon,
    OpenRouter: MockIcon,
    Together: MockIcon,
    VertexAI: MockIcon,
    Vercel: MockIcon,
    XAI: MockIcon,
  };
});

// Mock react-i18next for component tests
// i18n tests that need real i18next should import from '@/lib/i18n' directly
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      if (params) {
        // Simple interpolation mock: replace {{param}} with value
        let result = key;
        for (const [param, value] of Object.entries(params)) {
          result = result.replace(`{{${param}}}`, String(value));
        }
        return result;
      }
      return key;
    },
    i18n: {
      language: "en",
      changeLanguage: vi.fn(),
    },
  }),
  Trans: ({ children }: { children: React.ReactNode }) => children,
  I18nextProvider: ({ children }: { children: React.ReactNode }) => children,
  initReactI18next: { type: "3rdParty", init: vi.fn() },
}));
