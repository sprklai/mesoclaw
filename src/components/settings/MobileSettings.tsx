/**
 * MobileSettings — mobile-specific user preferences panel.
 *
 * Shown in the "Mobile" tab of the Settings page.  All preferences are stored
 * client-side in localStorage via `mobileSettingsStore`; no backend IPC needed.
 *
 * Settings included (Phase 7.4.7):
 * - Haptic feedback toggle
 * - Push notification opt-in
 * - Battery-optimisation acknowledgement (Android)
 * - Background refresh (iOS)
 */

import { useHaptic } from "@/hooks/useHaptic";
import { useMobileSettingsStore } from "@/stores/mobileSettingsStore";

// ─── Primitive row helpers ────────────────────────────────────────────────────

interface ToggleRowProps {
  label: string;
  description: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

function ToggleRow({ label, description, checked, onChange, disabled = false }: ToggleRowProps) {
  return (
    <div className="flex items-center justify-between gap-4 py-3">
      <div className="min-w-0 flex-1">
        <p className="text-sm font-medium">{label}</p>
        <p className="text-xs text-muted-foreground">{description}</p>
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={[
          "relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent",
          "transition-colors duration-200 ease-in-out focus-visible:outline-none focus-visible:ring-2",
          "focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
          disabled && "cursor-not-allowed opacity-50",
          checked ? "bg-primary" : "bg-input",
        ]
          .filter(Boolean)
          .join(" ")}
      >
        <span
          className={[
            "pointer-events-none inline-block h-5 w-5 rounded-full bg-background shadow-lg ring-0",
            "transition duration-200 ease-in-out",
            checked ? "translate-x-5" : "translate-x-0",
          ].join(" ")}
        />
      </button>
    </div>
  );
}

interface InfoBannerProps {
  message: string;
  onDismiss?: () => void;
}

function InfoBanner({ message, onDismiss }: InfoBannerProps) {
  return (
    <div className="flex items-start gap-3 rounded-md border border-yellow-500/30 bg-yellow-500/10 px-3 py-2 text-sm text-yellow-700 dark:text-yellow-400">
      <span className="mt-0.5 shrink-0">⚠</span>
      <span className="flex-1">{message}</span>
      {onDismiss && (
        <button
          type="button"
          onClick={onDismiss}
          className="shrink-0 text-xs underline opacity-70 hover:opacity-100"
        >
          Dismiss
        </button>
      )}
    </div>
  );
}

// ─── MobileSettings ───────────────────────────────────────────────────────────

export function MobileSettings() {
  const {
    hapticEnabled,
    pushNotificationsEnabled,
    batteryOptimisationAcknowledged,
    backgroundRefreshEnabled,
    setHapticEnabled,
    setPushNotificationsEnabled,
    acknowledgeBatteryOptimisation,
    setBackgroundRefreshEnabled,
  } = useMobileSettingsStore();

  const { isSupported: hapticSupported, haptic } = useHaptic();

  const handleHapticToggle = (enabled: boolean) => {
    setHapticEnabled(enabled);
    // Give immediate tactile confirmation when enabling.
    if (enabled) {
      haptic("light");
    }
  };

  // Detect rough platform hints from the user-agent (best-effort).
  const ua = typeof navigator !== "undefined" ? navigator.userAgent : "";
  const isAndroid = /android/i.test(ua);
  const isIOS = /iphone|ipad|ipod/i.test(ua);
  const isMobile = isAndroid || isIOS;

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-base font-semibold">Mobile Settings</h2>
        <p className="text-sm text-muted-foreground">
          Adjust preferences specific to phone and tablet usage.
        </p>
      </div>

      {/* ── Haptic Feedback ── */}
      <section className="space-y-1">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Haptic Feedback
        </h3>
        <div className="rounded-lg border border-border divide-y divide-border">
          <div className="px-4">
            <ToggleRow
              label="Enable Haptics"
              description={
                hapticSupported
                  ? "Vibrate briefly on button presses and confirmations."
                  : "Haptic feedback is not supported on this device."
              }
              checked={hapticEnabled}
              onChange={handleHapticToggle}
              disabled={!hapticSupported}
            />
          </div>
        </div>
      </section>

      {/* ── Push Notifications ── */}
      <section className="space-y-1">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Notifications
        </h3>
        <div className="rounded-lg border border-border divide-y divide-border">
          <div className="px-4">
            <ToggleRow
              label="Push Notifications"
              description={
                /* ## TODO: wire to tauri-plugin-notification when APNs/FCM is configured */
                "Receive alerts for agent actions, approvals, and reminders. Requires notification permissions."
              }
              checked={pushNotificationsEnabled}
              onChange={setPushNotificationsEnabled}
            />
          </div>
        </div>
        {pushNotificationsEnabled && (
          <p className="px-1 text-xs text-muted-foreground">
            {/* ## TODO: request OS permission via tauri-plugin-notification */}
            Notification delivery requires APNs (iOS) or FCM (Android) to be
            configured in the app build. This toggle saves your preference for
            when that support is available.
          </p>
        )}
      </section>

      {/* ── Android: Battery Optimisation ── */}
      {(isAndroid || !isMobile) && !batteryOptimisationAcknowledged && (
        <section className="space-y-2">
          <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Android
          </h3>
          <InfoBanner
            message="Android may kill background processes to save battery. For reliable agent operation, disable battery optimisation for MesoClaw in System Settings → Apps → MesoClaw → Battery."
            onDismiss={acknowledgeBatteryOptimisation}
          />
        </section>
      )}

      {/* ── iOS: Background Refresh ── */}
      {(isIOS || !isMobile) && (
        <section className="space-y-1">
          <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            iOS
          </h3>
          <div className="rounded-lg border border-border divide-y divide-border">
            <div className="px-4">
              <ToggleRow
                label="Background Refresh"
                description="Allow the app to refresh content in the background when on Wi-Fi. Enable this in iOS Settings → MesoClaw → Background App Refresh for best results."
                checked={backgroundRefreshEnabled}
                onChange={setBackgroundRefreshEnabled}
              />
            </div>
          </div>
          {!backgroundRefreshEnabled && (
            <p className="px-1 text-xs text-muted-foreground">
              Without background refresh, agent responses will only appear when
              you open the app.
            </p>
          )}
        </section>
      )}

      {/* ── Non-mobile hint ── */}
      {!isMobile && (
        <p className="text-xs text-muted-foreground">
          These settings take effect on phone and tablet builds of MesoClaw.
          The desktop app is unaffected.
        </p>
      )}
    </div>
  );
}
