import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PageHeader } from "../PageHeader";

describe("PageHeader", () => {
  it("renders title", () => {
    render(<PageHeader title="AI Chat" />);
    expect(screen.getByRole("heading", { name: "AI Chat" })).toBeInTheDocument();
  });

  it("renders description when provided", () => {
    render(<PageHeader title="Settings" description="Configure your AI" />);
    expect(screen.getByText("Configure your AI")).toBeInTheDocument();
  });

  it("renders children as actions", () => {
    render(
      <PageHeader title="Chat">
        <button type="button">New Chat</button>
      </PageHeader>
    );
    expect(screen.getByRole("button", { name: "New Chat" })).toBeInTheDocument();
  });

  it("renders without description gracefully", () => {
    const { container } = render(<PageHeader title="Memory" />);
    expect(container.querySelector("p")).not.toBeInTheDocument();
  });
});
