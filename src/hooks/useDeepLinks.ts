/**
 * Deep Links Hook
 *
 * Listens for deep link events from Tauri and handles navigation.
 *
 * Supported URL schemes:
 * - mesoclaw://session/{id} - Resume a chat session
 * - mesoclaw://channel/{id} - Navigate to a channel
 * - mesoclaw://approval/{id} - Review an approval request
 * - mesoclaw://scheduler/{id} - View a scheduled job
 * - mesoclaw://settings/{tab} - Open settings tab
 */
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useNavigate } from "@tanstack/react-router";

export function useDeepLinks() {
  const navigate = useNavigate();

  useEffect(() => {
    const unlisten = listen<string>("deep-link", (event) => {
      const url = event.payload;
      handleDeepLink(url);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  function handleDeepLink(url: string) {
    // Parse the URL: mesoclaw://<resource>/<id>
    const parsed = url.match(/^mesoclaw:\/\/([^/]+)(?:\/(.+))?$/);
    if (!parsed) {
      console.warn("Invalid deep link format:", url);
      return;
    }

    const [, resource, id] = parsed;

    switch (resource) {
      case "session":
        handleSessionLink(id);
        break;
      case "channel":
        handleChannelLink(id);
        break;
      case "approval":
        handleApprovalLink(id);
        break;
      case "scheduler":
        handleSchedulerLink(id);
        break;
      case "settings":
        handleSettingsLink(id);
        break;
      case "oauth":
        handleOAuthCallback(url);
        break;
      default:
        console.warn("Unknown deep link resource:", resource);
    }
  }

  function handleSessionLink(sessionId: string | undefined) {
    if (sessionId) {
      // Navigate to chat with session ID
      navigate({ to: "/chat" });
      console.log("Deep link: Navigate to session", sessionId);
    } else {
      navigate({ to: "/chat" });
    }
  }

  function handleChannelLink(channelId: string | undefined) {
    if (channelId) {
      navigate({ to: "/channels" });
      console.log("Deep link: Navigate to channel", channelId);
    } else {
      navigate({ to: "/channels" });
    }
  }

  function handleApprovalLink(approvalId: string | undefined) {
    // Navigate to settings approvals tab
    navigate({ to: "/settings", search: { tab: "approvals" } });
    console.log("Deep link: Approval", approvalId);
  }

  function handleSchedulerLink(jobId: string | undefined) {
    // Navigate to settings scheduler tab
    navigate({ to: "/settings", search: { tab: "scheduler" } });
    console.log("Deep link: Scheduler job", jobId);
  }

  function handleSettingsLink(tab: string | undefined) {
    const validTabs = ["ai", "channels", "identity", "skills", "scheduler", "modules", "logs", "approvals"];
    const safeTab = tab && validTabs.includes(tab) ? tab : "ai";
    navigate({ to: "/settings", search: { tab: safeTab } });
  }

  function handleOAuthCallback(url: string) {
    // Parse OAuth callback: mesoclaw://oauth/callback?code=...&state=...
    try {
      const urlObj = new URL(url);
      const code = urlObj.searchParams.get("code");
      const state = urlObj.searchParams.get("state");

      if (code) {
        // Emit OAuth callback event for any listeners
        console.log("OAuth callback received:", { code, state });
        // Could navigate to a success page or close the window
      }
    } catch (e) {
      console.error("Failed to parse OAuth callback URL:", e);
    }
  }
}
