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
    const checkEmptyStrings = (obj: Record<string, unknown>, path = ''): string[] => {
      const empty: string[] = [];

      for (const [key, value] of Object.entries(obj)) {
        const currentPath = path ? `${path}.${key}` : key;

        if (typeof value === 'string' && value.trim() === '') {
          empty.push(currentPath);
        } else if (typeof value === 'object' && value !== null) {
          empty.push(...checkEmptyStrings(value as Record<string, unknown>, currentPath));
        }
      }

      return empty;
    };

    const emptyKeys = checkEmptyStrings(commonEn as unknown as Record<string, unknown>);
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
