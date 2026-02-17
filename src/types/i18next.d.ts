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
