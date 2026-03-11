import { describe, it, expect } from "vitest";
import type { EmbeddingStatus, EmbedTestResult } from "./embeddings.svelte";

// 18.25 — Embeddings store status type
describe("Embeddings store", () => {
  it("EmbeddingStatus has correct shape", () => {
    const status: EmbeddingStatus = {
      provider: "none",
      model: "bge-small-en-v1.5",
      dimensions: 384,
      model_available: true,
    };
    expect(status.provider).toBe("none");
    expect(status.model).toBe("bge-small-en-v1.5");
    expect(status.dimensions).toBe(384);
  });

  // 18.26 — Embeddings store handles different provider states
  it("supports all provider types", () => {
    const none: EmbeddingStatus = {
      provider: "none",
      model: "",
      dimensions: 0,
      model_available: true,
    };
    const local: EmbeddingStatus = {
      provider: "local",
      model: "bge-small-en-v1.5",
      dimensions: 384,
      model_available: true,
    };
    const openai: EmbeddingStatus = {
      provider: "openai",
      model: "text-embedding-3-small",
      dimensions: 384,
      model_available: true,
    };

    expect(none.provider).toBe("none");
    expect(local.provider).toBe("local");
    expect(openai.provider).toBe("openai");
  });

  it("EmbedTestResult success shape", () => {
    const result: EmbedTestResult = {
      success: true,
      dimensions: 384,
      latency_ms: 42,
    };
    expect(result.success).toBe(true);
    expect(result.dimensions).toBe(384);
  });

  it("EmbedTestResult failure shape", () => {
    const result: EmbedTestResult = {
      success: false,
      latency_ms: 0,
      error: "No provider configured",
    };
    expect(result.success).toBe(false);
    expect(result.error).toBeTruthy();
  });
});
