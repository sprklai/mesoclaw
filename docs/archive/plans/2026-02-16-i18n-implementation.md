# Frontend i18n Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement internationalization (i18n) for MesoClaw's React frontend with auto-detection, Settings override, and full locale formatting.

**Architecture:** Use react-i18next with namespace-based translation organization. Initialize i18n before React mounts, auto-detect OS language, persist choice to localStorage + Tauri settings, and provide formatting utilities using native Intl API.

**Tech Stack:** react-i18next, i18next-browser-languagedetector, i18next-icu, TypeScript, Intl API

**Related Design:** `docs/plans/2026-02-16-i18n-design.md`

---

## Task 1: Install Dependencies

**Files:**
- Modify: `package.json`

**Step 1: Install i18n dependencies**

```bash
bun add i18next react-i18next i18next-browser-languagedetector i18next-icu
```

Expected: Dependencies added to package.json

**Step 2: Verify installation**

```bash
bun install
```

Expected: Lock file updated, node_modules contains new packages

**Step 3: Commit**

```bash
git add package.json bun.lockb
git commit -m "deps: Add i18next and react-i18next dependencies

Add i18next ecosystem for frontend internationalization:
- i18next: Core i18n framework
- react-i18next: React bindings and hooks
- i18next-browser-languagedetector: OS language detection
- i18next-icu: Date/number/currency formatting via Intl API

Total bundle impact: ~14kb gzipped"
```

---

## Task 2: Create Translation Directory Structure

**Files:**
- Create: `src/locales/en/common.json`
- Create: `src/locales/en/settings.json`
- Create: `src/locales/en/errors.json`
- Create: `src/locales/en/notifications.json`
- Create: `src/locales/en/chat.json`
- Create: `src/locales/en/ai.json`
- Create: `src/locales/en/validation.json`
- Create: `src/locales/index.ts`

**Step 1: Create English common translations**

File: `src/locales/en/common.json`

```json
{
  "actions": {
    "save": "Save",
    "cancel": "Cancel",
    "delete": "Delete",
    "edit": "Edit",
    "create": "Create",
    "copy": "Copy",
    "close": "Close",
    "back": "Back",
    "next": "Next",
    "finish": "Finish",
    "confirm": "Confirm"
  },
  "navigation": {
    "home": "Home",
    "settings": "Settings",
    "chat": "Chat"
  },
  "status": {
    "loading": "Loading...",
    "saving": "Saving...",
    "noResults": "No results found",
    "error": "An error occurred"
  },
  "tableCount_one": "{{count}} table",
  "tableCount_other": "{{count}} tables",
  "itemCount_one": "{{count}} item",
  "itemCount_other": "{{count}} items"
}
```

**Step 2: Create English settings translations**

File: `src/locales/en/settings.json`

```json
{
  "title": "Settings",
  "tabs": {
    "app": "App Settings",
    "ai": "AI Settings",
    "skills": "Skills",
    "advanced": "Advanced",
    "privacy": "Privacy"
  },
  "appearance": {
    "title": "Appearance",
    "description": "Customize how the application looks",
    "theme": {
      "label": "Theme",
      "description": "Select your preferred color scheme",
      "light": "Light",
      "dark": "Dark",
      "system": "System"
    },
    "language": {
      "label": "Language",
      "description": "Choose your preferred language"
    },
    "sidebarExpanded": {
      "label": "Sidebar Expanded",
      "description": "Keep the sidebar expanded by default"
    }
  },
  "behavior": {
    "title": "Behavior",
    "description": "Control how the application behaves",
    "showInTray": {
      "label": "Show in System Tray",
      "description": "Show the application icon in the system tray"
    },
    "launchAtLogin": {
      "label": "Launch at Login",
      "description": "Automatically start when you log in"
    }
  },
  "notifications": {
    "title": "Notifications",
    "description": "Configure notification preferences",
    "enable": {
      "label": "Enable Notifications",
      "description": "Allow the app to send you notifications"
    }
  },
  "developer": {
    "title": "Developer",
    "description": "Advanced settings for debugging and development",
    "enableLogging": {
      "label": "Enable Logging",
      "description": "Enable detailed application logging"
    },
    "logLevel": {
      "label": "Log Level",
      "description": "Set the minimum log level to record"
    }
  }
}
```

**Step 3: Create English errors translations**

File: `src/locales/en/errors.json`

```json
{
  "connection": {
    "failed": "Connection failed",
    "timeout": "Connection timeout",
    "refused": "Connection refused"
  },
  "validation": {
    "required": "This field is required",
    "invalid": "Invalid value",
    "tooShort": "Value is too short",
    "tooLong": "Value is too long"
  },
  "api": {
    "generalError": "An error occurred while communicating with the server",
    "unauthorized": "Unauthorized access",
    "forbidden": "Access forbidden",
    "notFound": "Resource not found"
  },
  "file": {
    "notFound": "File not found",
    "readError": "Error reading file",
    "writeError": "Error writing file"
  }
}
```

**Step 4: Create English notifications translations**

File: `src/locales/en/notifications.json`

```json
{
  "workspace": {
    "created": "Workspace '{{name}}' created successfully",
    "updated": "Workspace updated successfully",
    "deleted": "Workspace deleted successfully"
  },
  "settings": {
    "saved": "Settings saved successfully",
    "error": "Failed to save settings"
  },
  "ai": {
    "providerSet": "AI provider set to {{provider}}",
    "keyUpdated": "API key updated successfully"
  },
  "general": {
    "success": "Operation completed successfully",
    "error": "Operation failed"
  }
}
```

**Step 5: Create English chat translations**

File: `src/locales/en/chat.json`

```json
{
  "placeholder": "Ask anything about your database...",
  "emptyState": {
    "title": "Start a conversation",
    "description": "Ask questions about your database schema, relationships, or queries"
  },
  "actions": {
    "send": "Send",
    "clear": "Clear conversation",
    "copy": "Copy message",
    "regenerate": "Regenerate response"
  },
  "status": {
    "thinking": "Thinking...",
    "streaming": "Generating response...",
    "error": "Failed to get response"
  }
}
```

**Step 6: Create English AI translations**

File: `src/locales/en/ai.json`

```json
{
  "providers": {
    "openai": "OpenAI",
    "anthropic": "Anthropic",
    "google": "Google AI",
    "groq": "Groq",
    "vercel": "Vercel AI Gateway",
    "ollama": "Ollama (Local)"
  },
  "skills": {
    "title": "Skills",
    "description": "Custom AI capabilities",
    "create": "Create Skill",
    "edit": "Edit Skill",
    "delete": "Delete Skill",
    "execute": "Execute Skill"
  },
  "models": {
    "select": "Select Model",
    "custom": "Custom Model"
  }
}
```

**Step 7: Create English validation translations**

File: `src/locales/en/validation.json`

```json
{
  "required": "{{field}} is required",
  "minLength": "{{field}} must be at least {{min}} characters",
  "maxLength": "{{field}} must be at most {{max}} characters",
  "email": "Invalid email address",
  "url": "Invalid URL",
  "number": "Must be a valid number",
  "positive": "Must be a positive number",
  "integer": "Must be an integer"
}
```

**Step 8: Create index file**

File: `src/locales/index.ts`

```typescript
// Export all translation namespaces for type safety
export { default as commonEn } from './en/common.json';
export { default as settingsEn } from './en/settings.json';
export { default as errorsEn } from './en/errors.json';
export { default as notificationsEn } from './en/notifications.json';
export { default as chatEn } from './en/chat.json';
export { default as aiEn } from './en/ai.json';
export { default as validationEn } from './en/validation.json';
```

**Step 9: Verify JSON structure**

```bash
bun run check
```

Expected: TypeScript compilation succeeds

**Step 10: Commit**

```bash
git add src/locales/
git commit -m "feat(i18n): Add English translation files for all namespaces

Create 7 translation namespaces:
- common: UI actions, navigation, status messages
- settings: Settings page content
- errors: Error messages by category
- notifications: Toast/success messages
- chat: Chat interface text
- ai: AI provider and skill text
- validation: Form validation messages

All files use hierarchical key structure for organization."
```

---

## Task 3: Create i18n Configuration

**Files:**
- Create: `src/lib/i18n.ts`

**Step 1: Create i18n initialization file**

File: `src/lib/i18n.ts`

```typescript
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';
import ICU from 'i18next-icu';

// Import English translations
import {
  commonEn,
  settingsEn,
  errorsEn,
  notificationsEn,
  chatEn,
  aiEn,
  validationEn,
} from '@/locales';

i18n
  .use(ICU) // Date/number formatting via Intl API
  .use(LanguageDetector) // Auto-detect OS language
  .use(initReactI18next) // React bindings
  .init({
    resources: {
      en: {
        common: commonEn,
        settings: settingsEn,
        errors: errorsEn,
        notifications: notificationsEn,
        chat: chatEn,
        ai: aiEn,
        validation: validationEn,
      },
    },
    fallbackLng: 'en',
    defaultNS: 'common',
    ns: [
      'common',
      'settings',
      'errors',
      'notifications',
      'chat',
      'ai',
      'validation',
    ],

    interpolation: {
      escapeValue: false, // React already escapes
    },

    detection: {
      // Order of language detection methods
      order: ['localStorage', 'navigator'],
      // Persist language choice
      caches: ['localStorage'],
      lookupLocalStorage: 'i18nextLng',
    },
  });

export default i18n;
```

**Step 2: Verify i18n configuration**

```bash
bun run check
```

Expected: TypeScript compilation succeeds

**Step 3: Commit**

```bash
git add src/lib/i18n.ts
git commit -m "feat(i18n): Configure i18next with auto-detection and namespaces

Initialize i18next with:
- ICU plugin for date/number formatting
- LanguageDetector for OS language auto-detection
- 7 translation namespaces
- localStorage persistence
- English as default/fallback language

Detection order: localStorage → navigator → fallback to 'en'"
```

---

## Task 4: Add TypeScript Type Definitions

**Files:**
- Create: `src/types/i18next.d.ts`

**Step 1: Create TypeScript declarations**

File: `src/types/i18next.d.ts`

```typescript
import 'react-i18next';
import type {
  commonEn,
  settingsEn,
  errorsEn,
  notificationsEn,
  chatEn,
  aiEn,
  validationEn,
} from '@/locales';

declare module 'react-i18next' {
  interface CustomTypeOptions {
    defaultNS: 'common';
    resources: {
      common: typeof commonEn;
      settings: typeof settingsEn;
      errors: typeof errorsEn;
      notifications: typeof notificationsEn;
      chat: typeof chatEn;
      ai: typeof aiEn;
      validation: typeof validationEn;
    };
  }
}
```

**Step 2: Verify type safety**

```bash
bun run check
```

Expected: TypeScript compilation succeeds with type-safe translation keys

**Step 3: Commit**

```bash
git add src/types/i18next.d.ts
git commit -m "feat(i18n): Add TypeScript type definitions for translation keys

Extend react-i18next module with CustomTypeOptions to provide:
- Autocomplete for translation keys
- Type errors for invalid keys
- Namespace-aware type checking

Enables t('settings:appearance.title') with full type safety."
```

---

## Task 5: Initialize i18n in App Entry Point

**Files:**
- Modify: `src/main.tsx`

**Step 1: Import i18n before React**

File: `src/main.tsx` - Add import at top of file (before App import)

```typescript
import './lib/i18n'; // ← Add this line
```

Expected location: After CSS imports, before App component import

**Step 2: Verify initialization**

```bash
bun run dev
```

Expected: App starts, no console errors, i18n initializes

**Step 3: Check browser console**

Open DevTools → Console
Expected: No i18next errors, language detected

**Step 4: Commit**

```bash
git add src/main.tsx
git commit -m "feat(i18n): Initialize i18next before React mounts

Import i18n configuration at app entry point to ensure
i18next initializes before any components render.

Language detection runs on app start via LanguageDetector plugin."
```

---

## Task 6: Create Formatting Utilities

**Files:**
- Create: `src/lib/format.ts`

**Step 1: Create formatting hook**

File: `src/lib/format.ts`

```typescript
import { useTranslation } from 'react-i18next';

/**
 * Formatting hook providing locale-aware date, number, and currency formatters
 * Uses native Intl API for zero-dependency formatting
 */
export function useFormatters() {
  const { i18n } = useTranslation();
  const locale = i18n.language;

  return {
    /**
     * Format date with medium style (e.g., "Feb 16, 2026" or "16 févr. 2026")
     */
    formatDate: (date: Date | string, options?: Intl.DateTimeFormatOptions) => {
      const dateObj = typeof date === 'string' ? new Date(date) : date;
      return new Intl.DateTimeFormat(locale, {
        dateStyle: 'medium',
        ...options,
      }).format(dateObj);
    },

    /**
     * Format date with short style (e.g., "2/16/26" or "16/02/26")
     */
    formatDateShort: (date: Date | string) => {
      const dateObj = typeof date === 'string' ? new Date(date) : date;
      return new Intl.DateTimeFormat(locale, {
        dateStyle: 'short',
      }).format(dateObj);
    },

    /**
     * Format date with long style (e.g., "February 16, 2026" or "16 février 2026")
     */
    formatDateLong: (date: Date | string) => {
      const dateObj = typeof date === 'string' ? new Date(date) : date;
      return new Intl.DateTimeFormat(locale, {
        dateStyle: 'long',
      }).format(dateObj);
    },

    /**
     * Format time (e.g., "2:30 PM" or "14:30")
     */
    formatTime: (date: Date | string) => {
      const dateObj = typeof date === 'string' ? new Date(date) : date;
      return new Intl.DateTimeFormat(locale, {
        timeStyle: 'short',
      }).format(dateObj);
    },

    /**
     * Format date and time together
     */
    formatDateTime: (date: Date | string) => {
      const dateObj = typeof date === 'string' ? new Date(date) : date;
      return new Intl.DateTimeFormat(locale, {
        dateStyle: 'medium',
        timeStyle: 'short',
      }).format(dateObj);
    },

    /**
     * Format relative time (e.g., "2 hours ago" or "hace 2 horas")
     */
    formatRelativeTime: (date: Date | string) => {
      const dateObj = typeof date === 'string' ? new Date(date) : date;
      const now = new Date();
      const diffMs = now.getTime() - dateObj.getTime();
      const diffSec = Math.floor(diffMs / 1000);
      const diffMin = Math.floor(diffSec / 60);
      const diffHour = Math.floor(diffMin / 60);
      const diffDay = Math.floor(diffHour / 24);

      const rtf = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' });

      if (diffDay > 0) return rtf.format(-diffDay, 'day');
      if (diffHour > 0) return rtf.format(-diffHour, 'hour');
      if (diffMin > 0) return rtf.format(-diffMin, 'minute');
      return rtf.format(-diffSec, 'second');
    },

    /**
     * Format number with locale-specific separators (e.g., "1,234.56" or "1.234,56")
     */
    formatNumber: (num: number, options?: Intl.NumberFormatOptions) => {
      return new Intl.NumberFormat(locale, options).format(num);
    },

    /**
     * Format currency (e.g., "$1,234.56" or "1.234,56 €")
     */
    formatCurrency: (
      amount: number,
      currency = 'USD',
      options?: Intl.NumberFormatOptions
    ) => {
      return new Intl.NumberFormat(locale, {
        style: 'currency',
        currency,
        ...options,
      }).format(amount);
    },

    /**
     * Format percentage (e.g., "45%" or "45 %")
     */
    formatPercent: (value: number, options?: Intl.NumberFormatOptions) => {
      return new Intl.NumberFormat(locale, {
        style: 'percent',
        ...options,
      }).format(value);
    },

    /**
     * Format file size with locale-aware number formatting
     */
    formatFileSize: (bytes: number) => {
      const units = ['B', 'KB', 'MB', 'GB', 'TB'];
      let size = bytes;
      let unitIndex = 0;

      while (size >= 1024 && unitIndex < units.length - 1) {
        size /= 1024;
        unitIndex++;
      }

      return `${new Intl.NumberFormat(locale, {
        maximumFractionDigits: 2,
      }).format(size)} ${units[unitIndex]}`;
    },
  };
}
```

**Step 2: Verify formatting utilities**

```bash
bun run check
```

Expected: TypeScript compilation succeeds

**Step 3: Commit**

```bash
git add src/lib/format.ts
git commit -m "feat(i18n): Add locale-aware formatting utilities

Create useFormatters hook providing:
- Date formatting (short, medium, long, time, datetime)
- Relative time formatting (\"2 hours ago\")
- Number formatting with locale separators
- Currency formatting with locale symbols
- Percentage formatting
- File size formatting

Uses native Intl API for zero-dependency formatting.
All formatters adapt to user's selected language."
```

---

## Task 7: Write Tests for i18n Infrastructure

**Files:**
- Create: `src/__tests__/i18n/translations.test.ts`
- Create: `src/__tests__/i18n/format.test.ts`

**Step 1: Write translation file tests**

File: `src/__tests__/i18n/translations.test.ts`

```typescript
import { describe, it, expect } from 'vitest';
import {
  commonEn,
  settingsEn,
  errorsEn,
  notificationsEn,
  chatEn,
  aiEn,
  validationEn,
} from '@/locales';

describe('Translation Files', () => {
  it('should have valid JSON structure for all namespaces', () => {
    expect(commonEn).toBeDefined();
    expect(settingsEn).toBeDefined();
    expect(errorsEn).toBeDefined();
    expect(notificationsEn).toBeDefined();
    expect(chatEn).toBeDefined();
    expect(aiEn).toBeDefined();
    expect(validationEn).toBeDefined();
  });

  it('should not have empty strings in common namespace', () => {
    const checkEmptyStrings = (obj: any, path = ''): string[] => {
      const empty: string[] = [];

      for (const [key, value] of Object.entries(obj)) {
        const currentPath = path ? `${path}.${key}` : key;

        if (typeof value === 'string' && value.trim() === '') {
          empty.push(currentPath);
        } else if (typeof value === 'object' && value !== null) {
          empty.push(...checkEmptyStrings(value, currentPath));
        }
      }

      return empty;
    };

    const emptyKeys = checkEmptyStrings(commonEn);
    expect(emptyKeys).toHaveLength(0);
  });

  it('should have required action keys in common namespace', () => {
    expect(commonEn.actions).toBeDefined();
    expect(commonEn.actions.save).toBe('Save');
    expect(commonEn.actions.cancel).toBe('Cancel');
    expect(commonEn.actions.delete).toBe('Delete');
  });

  it('should have required navigation keys in common namespace', () => {
    expect(commonEn.navigation).toBeDefined();
    expect(commonEn.navigation.home).toBe('Home');
    expect(commonEn.navigation.settings).toBe('Settings');
  });

  it('should have settings title', () => {
    expect(settingsEn.title).toBe('Settings');
  });

  it('should have appearance settings structure', () => {
    expect(settingsEn.appearance).toBeDefined();
    expect(settingsEn.appearance.title).toBe('Appearance');
    expect(settingsEn.appearance.theme).toBeDefined();
    expect(settingsEn.appearance.language).toBeDefined();
  });
});
```

**Step 2: Write formatting tests**

File: `src/__tests__/i18n/format.test.ts`

```typescript
import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook } from '@testing-library/react';
import { I18nextProvider } from 'react-i18next';
import { useFormatters } from '@/lib/format';
import i18n from '@/lib/i18n';

const wrapper = ({ children }: { children: React.ReactNode }) => (
  <I18nextProvider i18n={i18n}>{children}</I18nextProvider>
);

describe('useFormatters', () => {
  beforeEach(() => {
    i18n.changeLanguage('en-US');
  });

  it('should format dates according to locale', () => {
    const { result } = renderHook(() => useFormatters(), { wrapper });

    const date = new Date('2026-02-16T14:30:00');
    const formatted = result.current.formatDate(date);

    expect(formatted).toMatch(/Feb.*16.*2026/);
  });

  it('should format short dates', () => {
    const { result } = renderHook(() => useFormatters(), { wrapper });

    const date = new Date('2026-02-16');
    const formatted = result.current.formatDateShort(date);

    // US format: M/D/YY
    expect(formatted).toMatch(/2\/16\/26/);
  });

  it('should format numbers with locale separators', () => {
    const { result } = renderHook(() => useFormatters(), { wrapper });

    const formatted = result.current.formatNumber(1234.56);

    // US format: 1,234.56
    expect(formatted).toBe('1,234.56');
  });

  it('should format currency with locale symbols', () => {
    const { result } = renderHook(() => useFormatters(), { wrapper });

    const formatted = result.current.formatCurrency(1234.56, 'USD');

    expect(formatted).toBe('$1,234.56');
  });

  it('should format percentages', () => {
    const { result } = renderHook(() => useFormatters(), { wrapper });

    const formatted = result.current.formatPercent(0.45);

    expect(formatted).toBe('45%');
  });

  it('should format file sizes', () => {
    const { result } = renderHook(() => useFormatters(), { wrapper });

    const formatted = result.current.formatFileSize(1536); // 1.5 KB

    expect(formatted).toBe('1.5 KB');
  });

  it('should format relative time', () => {
    const { result } = renderHook(() => useFormatters(), { wrapper });

    const twoHoursAgo = new Date(Date.now() - 2 * 60 * 60 * 1000);
    const formatted = result.current.formatRelativeTime(twoHoursAgo);

    expect(formatted).toMatch(/2 hours ago/);
  });
});
```

**Step 3: Run tests**

```bash
bun run test src/__tests__/i18n
```

Expected: All tests pass

**Step 4: Commit**

```bash
git add src/__tests__/i18n/
git commit -m "test(i18n): Add tests for translation files and formatters

Add comprehensive test coverage for:
- Translation file structure validation
- Empty string detection
- Required translation keys
- Date/number/currency formatting
- Relative time formatting
- File size formatting

All tests verify English locale behavior."
```

---

## Task 8: Migrate Settings Page to Use i18n

**Files:**
- Modify: `src/components/settings/AppSettingsTab.tsx`
- Modify: `src/constants/settings.ts`

**Step 1: Update AppSettingsTab to use translations**

File: `src/components/settings/AppSettingsTab.tsx`

Find and replace the hardcoded strings with translation calls:

```typescript
import { useTranslation } from 'react-i18next';

// Add at top of component function:
const { t } = useTranslation(['settings', 'common']);

// Replace hardcoded strings:
// "Appearance" → {t('settings:appearance.title')}
// "Customize how the application looks" → {t('settings:appearance.description')}
// "Theme" → {t('settings:appearance.theme.label')}
// "Select your preferred color scheme" → {t('settings:appearance.theme.description')}
// etc.
```

**Step 2: Create language selector options helper**

File: `src/constants/settings.ts` - Add new constant:

```typescript
export const LANGUAGE_OPTIONS = [
  { value: 'en', label: 'English' },
  // Future languages will be added here
] as const;
```

**Step 3: Add language selector to AppSettingsTab**

Add after theme selector in AppSettingsTab:

```typescript
<SettingRow
  label={t('settings:appearance.language.label')}
  description={t('settings:appearance.language.description')}
>
  <Select
    value={i18n.language}
    onValueChange={handleLanguageChange}
    options={LANGUAGE_OPTIONS}
    disabled={isSaving}
    className="w-full sm:w-40"
  />
</SettingRow>
```

Add language change handler:

```typescript
const handleLanguageChange = async (newLanguage: string) => {
  await i18n.changeLanguage(newLanguage);

  // Persist to Tauri settings (backup storage)
  try {
    await invoke('update_setting_command', {
      key: 'language',
      value: newLanguage,
    });
  } catch (error) {
    console.error('Failed to persist language to Tauri settings:', error);
  }
};
```

**Step 4: Test Settings page**

```bash
bun run dev
```

- Navigate to Settings
- Verify all text displays from translations
- Verify language selector appears
- Check browser console for errors

Expected: No errors, all text rendered

**Step 5: Commit**

```bash
git add src/components/settings/AppSettingsTab.tsx src/constants/settings.ts
git commit -m "feat(i18n): Migrate Settings page to use translations

Replace hardcoded English strings with translation keys:
- Appearance section (title, description, theme, language)
- Behavior section (tray, autostart)
- Notifications section
- Developer section

Add language selector with:
- Auto-sync to i18n.changeLanguage()
- Persistence to Tauri settings store
- Disabled state during save

Language selector shows in Appearance section below Theme."
```

---

## Task 9: Migrate Home Page to Use i18n

**Files:**
- Modify: `src/routes/index.tsx`

**Step 1: Update HomePage to use translations**

File: `src/routes/index.tsx`

Add translation hook and update all hardcoded text:

```typescript
import { useTranslation } from 'react-i18next';

function HomePage() {
  const { t } = useTranslation('common');

  return (
    <div className="flex h-full items-center justify-center">
      <div className="max-w-3xl text-center">
        <h1 className="text-4xl font-bold">
          {t('navigation.home')} to {APP_IDENTITY.productName}
        </h1>
        {/* Continue replacing all hardcoded strings */}
      </div>
    </div>
  );
}
```

**Note:** Home page has minimal text currently, most content is in Settings.

**Step 2: Test home page**

```bash
bun run dev
```

Navigate to home page, verify text displays correctly

**Step 3: Commit**

```bash
git add src/routes/index.tsx
git commit -m "feat(i18n): Migrate Home page to use translations

Replace hardcoded strings with translation keys.
Uses 'common' namespace for navigation and actions."
```

---

## Task 10: Add i18n Mock for Component Tests

**Files:**
- Create: `src/__tests__/setup.ts` (or modify if exists)
- Modify: `vitest.config.ts`

**Step 1: Create test setup with i18n mock**

File: `src/__tests__/setup.ts`

```typescript
import { vi } from 'vitest';

// Mock react-i18next for component tests
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, any>) => {
      if (params) {
        // Simple interpolation mock
        let result = key;
        for (const [param, value] of Object.entries(params)) {
          result = result.replace(`{{${param}}}`, String(value));
        }
        return result;
      }
      return key;
    },
    i18n: {
      language: 'en',
      changeLanguage: vi.fn(),
    },
  }),
  Trans: ({ children }: { children: React.ReactNode }) => children,
}));
```

**Step 2: Update vitest config to use setup file**

File: `vitest.config.ts` - Add setup file:

```typescript
export default defineConfig({
  test: {
    setupFiles: ['./src/__tests__/setup.ts'],
    // ... existing config
  },
});
```

**Step 3: Run existing tests to verify mock works**

```bash
bun run test
```

Expected: All tests pass with i18n mocked

**Step 4: Commit**

```bash
git add src/__tests__/setup.ts vitest.config.ts
git commit -m "test(i18n): Add i18n mock for component tests

Mock react-i18next in test setup to:
- Return translation keys as-is for verification
- Support simple interpolation
- Provide mock i18n.changeLanguage function

Component tests can now verify translation keys are used."
```

---

## Task 11: Add Documentation Index

**Files:**
- Create: `docs/index.md`

**Step 1: Create comprehensive documentation index**

File: `docs/index.md`

```markdown
# MesoClaw Documentation Index

This index organizes all documentation for the MesoClaw project, providing a structured guide to architecture, planning, features, and implementation.

## Quick Start

- [README](../README.md) - Project overview and quick start
- [CLAUDE.md](../CLAUDE.md) - High-level project orientation for Claude Code
- [.claude/CLAUDE.md](../.claude/CLAUDE.md) - Comprehensive project standards

## Architecture Documentation

### Core Architecture
- [Architecture Diagram](./architecture-diagram.md) - Complete system architecture overview
- [Frontend Database-Agnostic Design](./architecture/frontend-database-agnostic-design.md) - Frontend abstraction layer design
- [AI Multi-Provider Design](./architecture/AI_MULTI_PROVIDER_DESIGN.md) - AI provider integration architecture
- [Tauri Plugin Baseline](./architecture/tauri-plugin-baseline.md) - Tauri plugin architecture

### Ecosystem Analysis
- [Claw Ecosystem Analysis](./claw-ecosystem-analysis.md) - Analysis of Claw family products
- [Moltis/MicroClaw Analysis](./moltis-microclaw-analysis.md) - Comparison with related products
- [MesoClaw Gap Analysis](./mesoclaw-gap-analysis.md) - Feature gap analysis

## Implementation Plans

### Active Plans
- **[i18n Implementation Plan](./plans/2026-02-16-i18n-implementation.md)** - Frontend internationalization (Current)
- [i18n Design](./plans/2026-02-16-i18n-design.md) - i18n architecture and design decisions

### Architecture & Design Plans
- [CLI + Gateway + Agents Design](./plans/2026-02-16-cli-gateway-agents-design.md) - CLI-first architecture
- [Sidecar Modularity Design](./plans/2026-02-16-sidecar-modularity-design.md) - Sidecar architecture for modularity
- [MesoClaw Refactoring Design](./plans/2026-02-16-mesoclaw-refactoring-design.md) - Documentation and code refactoring
- [MesoClaw Refactoring Plan](./plans/2026-02-16-mesoclaw-refactoring-plan.md) - Bite-sized refactoring tasks
- [Doc Reconciliation Draft](./plans/2026-02-16-doc-reconciliation-draft.md) - Documentation cleanup plan

### Historical Plans
- [Implementation Plan](./implementation-plan.md) - Original phase-based implementation roadmap
- [Test Plan](./test-plan.md) - Comprehensive testing strategy

## Product & Requirements

- [Product Requirements](./product-requirements.md) - Complete product requirements document
- [User Journey](./user-journey.md) - User experience flow and scenarios

## Features

### AI & Chat
- [Chat Functionality](./features/CHAT_FUNCTIONALITY.md) - Chat interface implementation
- [Skill System](./features/SKILL_SYSTEM.md) - AI skill system architecture

### Accessibility
- [Accessibility Improvements](./features/accessibility/ACCESSIBILITY_IMPROVEMENTS.md) - Accessibility enhancements
- [Keyboard Navigation](./features/accessibility/KEYBOARD_NAVIGATION.md) - Keyboard shortcuts and navigation

## Security

- [Secure Storage](./security/SECURE_STORAGE.md) - Secure credential storage design
- [Secure Storage Quickstart](./security/SECURE_STORAGE_QUICKSTART.md) - Quick reference for secure storage
- [Keychain Migration](./security/KEYCHAIN_MIGRATION.md) - Migration guide for keychain storage

## UI/UX

- [UI/UX Improvements](./ux/UI_UX_IMPROVEMENTS.md) - Interface improvements and enhancements
- [Splash Screen Fix](./ui-fixes/SPLASH_SCREEN_FIX.md) - Splash screen implementation
- [Splash Screen Position Fix](./ui-fixes/SPLASH_SCREEN_POSITION_FIX.md) - Splash screen positioning

## Build & Optimization

- [Build Optimizations](./BUILD_OPTIMIZATIONS.md) - Build performance improvements
- [Generated Diagrams](./generated-diagrams.md) - Auto-generated architecture diagrams

## Implementation Sequence

For new contributors or when implementing features, follow this sequence:

1. **Read Foundation Docs**
   - [README](../README.md) → [CLAUDE.md](../CLAUDE.md) → [Architecture Diagram](./architecture-diagram.md)

2. **Understand Requirements**
   - [Product Requirements](./product-requirements.md) → [User Journey](./user-journey.md)

3. **Review Architecture**
   - [Frontend Database-Agnostic Design](./architecture/frontend-database-agnostic-design.md)
   - [AI Multi-Provider Design](./architecture/AI_MULTI_PROVIDER_DESIGN.md)

4. **Check Current Plans**
   - Review [Active Plans](#active-plans) for ongoing work
   - Check [Implementation Plan](./implementation-plan.md) for phase status

5. **Implement Features**
   - Follow TDD approach from [Test Plan](./test-plan.md)
   - Use bite-sized tasks from relevant implementation plans
   - Ensure accessibility per [Accessibility docs](./features/accessibility/)

6. **Security & Testing**
   - Follow [Secure Storage](./security/SECURE_STORAGE.md) for credentials
   - Run tests per [Test Plan](./test-plan.md)
   - Check [Build Optimizations](./BUILD_OPTIMIZATIONS.md)

## Testing Sequence

1. **Unit Tests** - Test individual components and utilities
2. **Integration Tests** - Test feature integration
3. **E2E Tests** - Test user workflows per [User Journey](./user-journey.md)
4. **Accessibility Tests** - Verify keyboard navigation and screen readers
5. **Security Tests** - Verify secure storage and API key handling

## Documentation Standards

- **Architecture docs** - High-level design, diagrams, trade-offs
- **Implementation plans** - Bite-sized tasks, exact file paths, TDD approach
- **Feature docs** - User-facing functionality, usage examples
- **Security docs** - Threat models, mitigation strategies

---

**Last Updated:** 2026-02-16
**Maintained By:** MesoClaw Development Team
```

**Step 2: Verify markdown formatting**

```bash
bun run check
```

**Step 3: Commit**

```bash
git add docs/index.md
git commit -m "docs: Add comprehensive documentation index

Create centralized documentation index organizing:
- Architecture documentation (core + ecosystem analysis)
- Implementation plans (active + historical)
- Product requirements and user journey
- Feature documentation (AI, chat, accessibility)
- Security documentation
- UI/UX improvements
- Build and optimization guides

Includes implementation sequence guide for new contributors
and testing sequence for QA workflows."
```

---

## Task 12: Final Verification & Testing

**Files:**
- None (verification only)

**Step 1: Run full test suite**

```bash
bun run test
```

Expected: All tests pass including new i18n tests

**Step 2: Run type checking**

```bash
bun run check
```

Expected: No TypeScript errors

**Step 3: Run linting**

```bash
bun run lint
```

Expected: No linting errors

**Step 4: Start dev server**

```bash
bun run dev
```

**Step 5: Manual verification checklist**

- [ ] Settings page displays with translations
- [ ] Language selector appears in Settings → App → Appearance
- [ ] Changing language persists after refresh
- [ ] Browser DevTools shows no console errors
- [ ] Translation keys display correctly (not showing "key.path.here")
- [ ] Home page uses translations
- [ ] Date/number formatting works (check via useFormatters in a test component)

**Step 6: Check localStorage**

Open DevTools → Application → Local Storage
Verify: `i18nextLng` key exists with value "en"

**Step 7: Commit verification**

```bash
git add -A
git commit -m "test(i18n): Verify i18n implementation works end-to-end

Manual verification completed:
✅ Settings page fully translated
✅ Language selector functional
✅ Language persistence works
✅ No console errors
✅ Type safety verified
✅ All tests passing
✅ Documentation index created

i18n infrastructure ready for future language additions."
```

---

## Success Criteria

- ✅ All dependencies installed
- ✅ Translation files created for 7 namespaces
- ✅ i18n initialized before React
- ✅ TypeScript type definitions provide autocomplete
- ✅ Formatting utilities work with Intl API
- ✅ Settings page fully migrated to translations
- ✅ Language selector functional in Settings
- ✅ Language choice persists to localStorage + Tauri settings
- ✅ All tests passing
- ✅ No TypeScript errors
- ✅ No console errors in browser
- ✅ Documentation index created

## Future Tasks

**Adding new languages:**
1. Create `src/locales/{lang}/` directory with all 7 JSON files
2. Import in `src/lib/i18n.ts` resources object
3. Add to `LANGUAGE_OPTIONS` in `src/constants/settings.ts`
4. Test with language selector

**Migrating more components:**
1. Extract hardcoded strings to appropriate namespace JSON
2. Replace with `t('namespace:key')` calls
3. Add tests for component rendering
4. Verify no missing translation keys

**Translation management:**
1. Consider translation management UI
2. Consider AI-powered translation via OpenAI API
3. Consider i18next-parser for automated extraction

---

**Plan Status:** Ready for execution
**Estimated Time:** 3-4 hours for full implementation
**Prerequisites:** None (all changes are frontend-only)
