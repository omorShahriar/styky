import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", () => ({
  api: { listItems: vi.fn(), getSettings: vi.fn() },
}));

import { api } from "./api";
import { QUIET_ATTEMPTS, RETRY_STEPS_MS, retryDelay } from "./retry";
import { useStore } from "./store";

const listItems = vi.mocked(api.listItems);
const getSettings = vi.mocked(api.getSettings);

describe("retryDelay", () => {
  it("backs off and then holds at the last step", () => {
    expect([1, 2, 3, 4, 5, 6, 40].map(retryDelay)).toEqual([250, 500, 1000, 2000, 3000, 3000, 3000]);
    expect(RETRY_STEPS_MS[RETRY_STEPS_MS.length - 1]).toBeLessThanOrEqual(5000);
  });
});

describe("store.refresh at startup", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useStore.setState({ loaded: false, items: [], toast: null });
    listItems.mockReset();
    getSettings.mockReset();
    getSettings.mockResolvedValue({});
  });
  afterEach(() => vi.useRealTimers());

  it("retries quickly and silently until the backend answers", async () => {
    listItems.mockRejectedValueOnce("state not managed").mockRejectedValueOnce("state not managed").mockResolvedValue([]);
    await useStore.getState().refresh();
    expect(useStore.getState().loaded).toBe(false);
    await vi.advanceTimersByTimeAsync(250);
    expect(useStore.getState().loaded).toBe(false);
    await vi.advanceTimersByTimeAsync(500);
    expect(useStore.getState().loaded).toBe(true);
    expect(useStore.getState().toast).toBeNull();
    expect(listItems).toHaveBeenCalledTimes(3);
  });

  it("tells the user once it keeps failing", async () => {
    listItems.mockRejectedValue("boom");
    await useStore.getState().refresh();
    for (let i = 1; i <= QUIET_ATTEMPTS - 1; i++) await vi.advanceTimersByTimeAsync(retryDelay(i));
    expect(useStore.getState().toast).toBeNull();
    await vi.advanceTimersByTimeAsync(retryDelay(QUIET_ATTEMPTS));
    expect(useStore.getState().toast?.message).toContain("boom");
  });

  it("toasts straight away once already loaded", async () => {
    useStore.setState({ loaded: true });
    listItems.mockRejectedValue("boom");
    await useStore.getState().refresh();
    expect(useStore.getState().toast?.message).toContain("boom");
    await vi.advanceTimersByTimeAsync(10_000);
    expect(listItems).toHaveBeenCalledTimes(1);
  });
});
