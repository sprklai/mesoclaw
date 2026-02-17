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
