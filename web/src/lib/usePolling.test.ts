import { describe, it, expect } from "vitest";
import { pollDelay, MAX_POLL_BACKOFF_MS } from "./usePolling";

describe("pollDelay", () => {
  it("keeps the configured interval while polls succeed", () => {
    expect(pollDelay(2000, 0)).toBe(2000);
  });

  it("doubles the gap for each consecutive failure", () => {
    expect(pollDelay(2000, 1)).toBe(4000);
    expect(pollDelay(2000, 2)).toBe(8000);
    expect(pollDelay(2000, 3)).toBe(16000);
  });

  it("never waits longer than the cap, however long the source stays down", () => {
    expect(pollDelay(2000, 4)).toBe(MAX_POLL_BACKOFF_MS);
    expect(pollDelay(2000, 500)).toBe(MAX_POLL_BACKOFF_MS);
  });

  it("never backs off to something faster than the configured interval", () => {
    expect(pollDelay(60_000, 0)).toBe(60_000);
    expect(pollDelay(60_000, 1)).toBe(60_000);
  });
});
