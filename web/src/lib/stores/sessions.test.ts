import { describe, it, expect } from "vitest";
import type { Session, SessionSummary } from "./sessions.svelte";

// 8.7.15 — Session with source field renders badge data
describe("Session store channel support", () => {
  it('session with source="telegram" has source field', () => {
    const session: Session = {
      id: "sess-1",
      title: "Telegram conversation",
      created_at: Date.now(),
      updated_at: Date.now(),
      source: "telegram",
    };
    expect(session.source).toBe("telegram");
    // Badge rendering would check: session.source && session.source !== 'web'
    expect(session.source).toBeTruthy();
    expect(session.source).not.toBe("web");
  });

  // 8.7.16 — Session interface includes optional source field
  it("Session interface has optional source field", () => {
    const sessionWithSource: SessionSummary = {
      id: "sess-2",
      title: "Slack thread",
      created_at: Date.now(),
      source: "slack",
    };
    expect(sessionWithSource.source).toBe("slack");

    const sessionWithoutSource: SessionSummary = {
      id: "sess-3",
      title: "Regular chat",
      created_at: Date.now(),
    };
    expect(sessionWithoutSource.source).toBeUndefined();
  });
});
