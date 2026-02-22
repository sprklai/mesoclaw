/**
 * Tests for task classification utilities
 */
import { describe, it, expect } from "vitest";
import {
  classifyTask,
  getClassificationReason,
  TASK_DISPLAY_NAMES,
  TASK_ICONS,
} from "./taskClassifier";

describe("classifyTask", () => {
  describe("code classification", () => {
    it("classifies 'debug this error' as code", () => {
      expect(classifyTask("debug this error")).toBe("code");
    });

    it("classifies 'implement a function' as code", () => {
      expect(classifyTask("implement a function")).toBe("code");
    });

    it("classifies 'fix the bug' as code", () => {
      expect(classifyTask("fix the bug")).toBe("code");
    });

    it("classifies 'refactor this class' as code", () => {
      expect(classifyTask("refactor this class")).toBe("code");
    });

    it("classifies 'write a script' as code", () => {
      expect(classifyTask("write a script")).toBe("code");
    });

    it("classifies 'review the API' as code", () => {
      expect(classifyTask("review the API")).toBe("code");
    });

    it("is case-insensitive", () => {
      expect(classifyTask("DEBUG THIS")).toBe("code");
      expect(classifyTask("FIX The BUG")).toBe("code");
    });
  });

  describe("analysis classification", () => {
    it("classifies 'analyze the data' as analysis", () => {
      expect(classifyTask("analyze the data")).toBe("analysis");
    });

    it("classifies 'summarize this text' as analysis", () => {
      expect(classifyTask("summarize this text")).toBe("analysis");
    });

    it("classifies 'why did this happen' as analysis", () => {
      expect(classifyTask("why did this happen")).toBe("analysis");
    });

    it("classifies 'explain how it works' as analysis", () => {
      expect(classifyTask("explain how it works")).toBe("analysis");
    });

    it("classifies 'compare these options' as analysis", () => {
      expect(classifyTask("compare these options")).toBe("analysis");
    });

    it("classifies 'what is the difference' as analysis", () => {
      expect(classifyTask("what is the difference")).toBe("analysis");
    });
  });

  describe("creative classification", () => {
    it("classifies 'write a story' as creative", () => {
      expect(classifyTask("write a story")).toBe("creative");
    });

    it("classifies 'create a design' as creative", () => {
      expect(classifyTask("create a design")).toBe("creative");
    });

    it("classifies 'brainstorm ideas' as creative", () => {
      expect(classifyTask("brainstorm ideas")).toBe("creative");
    });

    it("classifies 'write a blog post' as creative", () => {
      expect(classifyTask("write a blog post")).toBe("creative");
    });

    it("classifies 'imagine a world' as creative", () => {
      expect(classifyTask("imagine a world")).toBe("creative");
    });
  });

  describe("fast classification", () => {
    it("classifies short messages without questions as fast", () => {
      expect(classifyTask("hello")).toBe("fast");
      expect(classifyTask("ok thanks")).toBe("fast");
      expect(classifyTask("yes")).toBe("fast");
      expect(classifyTask("no")).toBe("fast");
    });

    it("does not classify short questions as fast", () => {
      expect(classifyTask("what?")).toBe("general");
      expect(classifyTask("how are you?")).toBe("general");
    });
  });

  describe("general classification", () => {
    it("classifies questions as general", () => {
      expect(
        classifyTask(
          "Tell me about the current weather conditions today in detail?"
        )
      ).toBe("general");
    });

    it("classifies help requests with questions as general", () => {
      // Note: Short messages without "?" are classified as "fast"
      expect(classifyTask("Can you help me with something?")).toBe("general");
    });

    it("classifies polite requests as general", () => {
      expect(
        classifyTask(
          "Please tell me more about this topic that I am interested in?"
        )
      ).toBe("general");
    });

    it("falls back to general for unclassified input", () => {
      // Note: Must avoid keywords from other categories (code, analysis, creative, etc.)
      // "history" contains "story" which triggers creative, so use different wording
      expect(
        classifyTask("What are the current market trends for technology products?")
      ).toBe("general");
    });
  });

  describe("priority order", () => {
    it("code takes priority over other categories", () => {
      expect(classifyTask("debug and write a story about fixing bugs")).toBe(
        "code"
      );
    });

    it("analysis takes priority over creative", () => {
      expect(
        classifyTask("summarize and explain the creative writing process")
      ).toBe("analysis");
    });
  });
});

describe("getClassificationReason", () => {
  it("returns correct reason for code", () => {
    expect(getClassificationReason("debug this", "code")).toBe(
      "Detected programming-related keywords"
    );
  });

  it("returns correct reason for analysis", () => {
    expect(getClassificationReason("explain", "analysis")).toBe(
      "Detected analysis or explanation request"
    );
  });

  it("returns correct reason for creative", () => {
    expect(getClassificationReason("write a story", "creative")).toBe(
      "Detected creative writing request"
    );
  });

  it("returns correct reason for fast", () => {
    expect(getClassificationReason("hello", "fast")).toBe(
      "Short message without complex question"
    );
  });

  it("returns correct reason for general", () => {
    expect(getClassificationReason("what is", "general")).toBe(
      "General conversation query"
    );
  });
});

describe("TASK_DISPLAY_NAMES", () => {
  it("has display names for all task types", () => {
    const taskTypes = ["code", "general", "fast", "creative", "analysis", "other"];
    for (const type of taskTypes) {
      expect(TASK_DISPLAY_NAMES[type as keyof typeof TASK_DISPLAY_NAMES]).toBeDefined();
      expect(TASK_DISPLAY_NAMES[type as keyof typeof TASK_DISPLAY_NAMES].length).toBeGreaterThan(0);
    }
  });
});

describe("TASK_ICONS", () => {
  it("has icons for all task types", () => {
    const taskTypes = ["code", "general", "fast", "creative", "analysis", "other"];
    for (const type of taskTypes) {
      expect(TASK_ICONS[type as keyof typeof TASK_ICONS]).toBeDefined();
    }
  });
});
