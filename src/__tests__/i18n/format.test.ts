import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook } from '@testing-library/react';
import { I18nextProvider } from 'react-i18next';
import React from 'react';
import { useFormatters } from '@/lib/format';
import i18n from '@/lib/i18n';

const wrapper = ({ children }: { children: React.ReactNode }) =>
  React.createElement(I18nextProvider, { i18n }, children);

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

    // Use noon local time to avoid UTC day-boundary timezone shift
    const date = new Date(2026, 1, 16, 12, 0, 0); // Feb 16, 2026 noon local
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
