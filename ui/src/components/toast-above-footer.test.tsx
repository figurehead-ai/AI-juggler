import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { resetToasts, showToast, ToastHost } from "./Toast";

afterEach(() => {
  cleanup();
  resetToasts();
});

describe("toasts never cover a footer at the window's foot", () => {
  it("sits above a footer marked data-toast-avoid, wherever it's drawn (New pool's, from its portal, round 6 #6)", () => {
    Object.defineProperty(window, "innerHeight", { configurable: true, value: 800 });
    const foot = document.createElement("div");
    foot.setAttribute("data-toast-avoid", "");
    foot.getBoundingClientRect = () => ({ top: 740, bottom: 800, height: 60, left: 0, right: 1280, width: 1280, x: 0, y: 740, toJSON: () => ({}) });
    document.body.appendChild(foot);
    render(<ToastHost />);
    act(() => {
      showToast({ message: "Pick a project first" });
    });
    // 800 − 740 + 8: above the footer's top, with a little room.
    expect(screen.getByTestId("toast-stack").style.bottom).toBe("68px");
    foot.remove();
  });

  it("sits at its usual place with no such footer", () => {
    Object.defineProperty(window, "innerHeight", { configurable: true, value: 800 });
    render(<ToastHost />);
    act(() => {
      showToast({ message: "Saved" });
    });
    expect(screen.getByTestId("toast-stack").style.bottom).toBe("16px");
  });
});
