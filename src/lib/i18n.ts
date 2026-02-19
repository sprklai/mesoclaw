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
  channelsEn,
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
        channels: channelsEn,
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
      'channels',
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
