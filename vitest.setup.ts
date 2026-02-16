import "@testing-library/jest-dom";
import { cleanup } from "@testing-library/react";
import { expect, afterEach } from "vitest";

// Cleanup after each test
afterEach(() => {
  cleanup();
});

// Extend Vitest's expect with jest-dom matchers
expect.extend({});
