import { act } from "react";
import { beforeEach, describe, expect, it } from "vitest";
import { useContextPanelStore } from "../contextPanelStore";

describe("contextPanelStore", () => {
  beforeEach(() => {
    act(() => {
      useContextPanelStore.getState().clearContent();
    });
  });

  it("starts with null content", () => {
    expect(useContextPanelStore.getState().content).toBeNull();
  });

  it("setContent stores a ReactNode", () => {
    const node = <div>hello</div>;
    act(() => {
      useContextPanelStore.getState().setContent(node);
    });
    expect(useContextPanelStore.getState().content).toBe(node);
  });

  it("clearContent resets to null", () => {
    act(() => {
      useContextPanelStore.getState().setContent(<div>hello</div>);
      useContextPanelStore.getState().clearContent();
    });
    expect(useContextPanelStore.getState().content).toBeNull();
  });
});
