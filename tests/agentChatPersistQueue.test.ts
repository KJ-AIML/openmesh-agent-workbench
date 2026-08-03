import { afterEach, describe, expect, it, vi } from "vitest";
import { createPersistQueue } from "../src/lib/agentChat/persistQueue";

afterEach(() => {
  vi.useRealTimers();
});

describe("createPersistQueue", () => {
  it("debounces writes and coalesces to the latest payload", () => {
    vi.useFakeTimers();
    const persist = vi.fn();
    const queue = createPersistQueue(persist, 400);

    queue.schedule("/p", [{ id: "a" }]);
    queue.schedule("/p", [{ id: "b" }]);
    expect(persist).not.toHaveBeenCalled();

    vi.advanceTimersByTime(399);
    expect(persist).not.toHaveBeenCalled();

    // Debounce window ends → deferred setTimeout(0) write is queued.
    vi.advanceTimersByTime(1);
    expect(persist).not.toHaveBeenCalled();

    // Flush the deferred macrotask.
    vi.runOnlyPendingTimers();
    expect(persist).toHaveBeenCalledTimes(1);
    expect(persist).toHaveBeenCalledWith("/p", [{ id: "b" }]);
  });

  it("flush writes immediately and cancels the timer", () => {
    vi.useFakeTimers();
    const persist = vi.fn();
    const queue = createPersistQueue(persist, 400);

    queue.schedule("/p", [{ id: "a" }]);
    queue.flush();
    expect(persist).toHaveBeenCalledTimes(1);
    expect(persist).toHaveBeenCalledWith("/p", [{ id: "a" }]);

    vi.advanceTimersByTime(500);
    expect(persist).toHaveBeenCalledTimes(1);
  });

  it("flush still writes after debounce when idle write is pending", () => {
    vi.useFakeTimers();
    const persist = vi.fn();
    const queue = createPersistQueue(persist, 400);

    queue.schedule("/p", [{ id: "a" }]);
    vi.advanceTimersByTime(400);
    // Deferred write queued, not yet run.
    expect(persist).not.toHaveBeenCalled();
    expect(queue.pending).toBe(true);

    queue.flush();
    expect(persist).toHaveBeenCalledTimes(1);
    expect(persist).toHaveBeenCalledWith("/p", [{ id: "a" }]);

    vi.runOnlyPendingTimers();
    expect(persist).toHaveBeenCalledTimes(1);
  });

  it("cancel drops pending work without writing", () => {
    vi.useFakeTimers();
    const persist = vi.fn();
    const queue = createPersistQueue(persist, 400);

    queue.schedule("/p", [{ id: "a" }]);
    expect(queue.pending).toBe(true);
    queue.cancel();
    expect(queue.pending).toBe(false);

    vi.advanceTimersByTime(500);
    expect(persist).not.toHaveBeenCalled();
  });

  it("flush is a no-op when nothing is pending", () => {
    const persist = vi.fn();
    const queue = createPersistQueue(persist, 400);
    queue.flush();
    expect(persist).not.toHaveBeenCalled();
  });
});
