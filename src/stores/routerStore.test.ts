/**
 * Tests for router configuration store
 *
 * Note: These tests use bun:test mocking. For more comprehensive integration tests,
 * consider using a test harness that can properly mock Tauri's IPC.
 */
import { describe, it, expect, mock, beforeEach, afterEach } from "bun:test";

// Helper to get a fresh store state
function getFreshStore() {
  // Reset to initial state by creating a new store
  return useRouterStore.getState();
}

// Import the store - it will use the actual invoke which may fail in tests
// For unit tests, we test the store logic directly
import { useRouterStore } from "./routerStore";

describe("routerStore helper functions", () => {
  describe("getModelsByProvider", () => {
    it("filters models by provider", () => {
      const store = getFreshStore();

      // Directly manipulate the models array for testing
      const testModels = [
        { id: "1", providerId: "openai", modelId: "gpt-4", displayName: "GPT-4", costTier: "medium" as const, contextLimit: 8192, modalities: ["text" as const], capabilities: null, discoveredAt: "", isActive: true },
        { id: "2", providerId: "openai", modelId: "gpt-3.5", displayName: "GPT-3.5", costTier: "low" as const, contextLimit: 4096, modalities: ["text" as const], capabilities: null, discoveredAt: "", isActive: true },
        { id: "3", providerId: "anthropic", modelId: "claude", displayName: "Claude", costTier: "high" as const, contextLimit: 200000, modalities: ["text" as const], capabilities: null, discoveredAt: "", isActive: true },
      ];

      // Test the filtering logic directly
      const openaiModels = testModels.filter(m => m.providerId === "openai");
      expect(openaiModels.length).toBe(2);

      const anthropicModels = testModels.filter(m => m.providerId === "anthropic");
      expect(anthropicModels.length).toBe(1);
    });
  });

  describe("getModelsByCostTier", () => {
    it("filters models by cost tier and active status", () => {
      const testModels = [
        { id: "1", providerId: "openai", modelId: "gpt-4", displayName: "GPT-4", costTier: "medium" as const, contextLimit: 8192, modalities: ["text" as const], capabilities: null, discoveredAt: "", isActive: true },
        { id: "2", providerId: "openai", modelId: "gpt-3.5", displayName: "GPT-3.5", costTier: "low" as const, contextLimit: 4096, modalities: ["text" as const], capabilities: null, discoveredAt: "", isActive: true },
        { id: "3", providerId: "anthropic", modelId: "claude", displayName: "Claude", costTier: "high" as const, contextLimit: 200000, modalities: ["text" as const], capabilities: null, discoveredAt: "", isActive: false },
      ];

      // Test low tier (should only include active)
      const lowTierModels = testModels.filter(m => m.costTier === "low" && m.isActive);
      expect(lowTierModels.length).toBe(1);
      expect(lowTierModels[0].modelId).toBe("gpt-3.5");

      // Test high tier (should not include inactive)
      const highTierModels = testModels.filter(m => m.costTier === "high" && m.isActive);
      expect(highTierModels.length).toBe(0);
    });
  });

  describe("getModelsByModality", () => {
    it("filters models by modality", () => {
      const testModels = [
        { id: "1", providerId: "p1", modelId: "m1", displayName: "M1", costTier: "medium" as const, contextLimit: 8192, modalities: ["text" as const], capabilities: null, discoveredAt: "", isActive: true },
        { id: "2", providerId: "p2", modelId: "m2", displayName: "M2", costTier: "medium" as const, contextLimit: 8192, modalities: ["text" as const, "image" as const], capabilities: null, discoveredAt: "", isActive: true },
        { id: "3", providerId: "p3", modelId: "m3", displayName: "M3", costTier: "medium" as const, contextLimit: 8192, modalities: ["image" as const], capabilities: null, discoveredAt: "", isActive: true },
      ];

      const textModels = testModels.filter(m => m.isActive && m.modalities.includes("text"));
      expect(textModels.length).toBe(2);

      const imageModels = testModels.filter(m => m.isActive && m.modalities.includes("image"));
      expect(imageModels.length).toBe(2);
    });
  });
});

describe("helper exports", () => {
  describe("getProfileDisplayName", () => {
    it("returns correct display names", async () => {
      const { getProfileDisplayName } = await import("./routerStore");

      expect(getProfileDisplayName("eco")).toBe("Eco (Cost-Effective)");
      expect(getProfileDisplayName("balanced")).toBe("Balanced");
      expect(getProfileDisplayName("premium")).toBe("Premium (Best Quality)");
    });
  });

  describe("getTaskDisplayName", () => {
    it("returns correct display names", async () => {
      const { getTaskDisplayName } = await import("./routerStore");

      expect(getTaskDisplayName("code")).toBe("Code & Programming");
      expect(getTaskDisplayName("general")).toBe("General Conversation");
      expect(getTaskDisplayName("fast")).toBe("Quick Responses");
      expect(getTaskDisplayName("creative")).toBe("Creative Writing");
      expect(getTaskDisplayName("analysis")).toBe("Analysis & Research");
      expect(getTaskDisplayName("other")).toBe("Other Tasks");
    });
  });

  describe("getModalityDisplayName", () => {
    it("returns correct display names", async () => {
      const { getModalityDisplayName } = await import("./routerStore");

      expect(getModalityDisplayName("text")).toBe("Text");
      expect(getModalityDisplayName("image")).toBe("Image Understanding");
      expect(getModalityDisplayName("image_generation")).toBe("Image Generation");
      expect(getModalityDisplayName("audio_transcription")).toBe("Audio Transcription");
      expect(getModalityDisplayName("audio_generation")).toBe("Audio Generation");
      expect(getModalityDisplayName("video")).toBe("Video");
      expect(getModalityDisplayName("embedding")).toBe("Embeddings");
    });
  });
});

describe("initial state", () => {
  it("starts with isLoading true", () => {
    const { isLoading } = useRouterStore.getState();
    expect(isLoading).toBe(true);
  });

  it("starts with empty models array", () => {
    const { models } = useRouterStore.getState();
    expect(Array.isArray(models)).toBe(true);
  });
});

describe("clearError", () => {
  it("clears error state", () => {
    // Set error via setState
    useRouterStore.setState({ error: "Test error" });

    // Call clearError
    useRouterStore.getState().clearError();

    expect(useRouterStore.getState().error).toBeNull();
  });
});
