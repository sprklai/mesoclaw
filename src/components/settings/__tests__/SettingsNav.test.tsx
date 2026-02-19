import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SettingsNav } from "../SettingsNav";

const sections = [
  { id: "ai", label: "AI Provider" },
  { id: "skills", label: "Skills" },
  { id: "app", label: "App Settings" },
];

describe("SettingsNav", () => {
  it("renders all section labels", () => {
    render(
      <SettingsNav sections={sections} activeSection="ai" onSectionChange={vi.fn()} />
    );
    expect(screen.getByRole("button", { name: "AI Provider" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Skills" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "App Settings" })).toBeInTheDocument();
  });

  it("marks active section with aria-current", () => {
    render(
      <SettingsNav sections={sections} activeSection="skills" onSectionChange={vi.fn()} />
    );
    expect(screen.getByRole("button", { name: "Skills" })).toHaveAttribute(
      "aria-current",
      "page"
    );
  });

  it("calls onSectionChange when a section is clicked", () => {
    const onChange = vi.fn();
    render(
      <SettingsNav sections={sections} activeSection="ai" onSectionChange={onChange} />
    );
    fireEvent.click(screen.getByRole("button", { name: "Skills" }));
    expect(onChange).toHaveBeenCalledWith("skills");
  });
});
