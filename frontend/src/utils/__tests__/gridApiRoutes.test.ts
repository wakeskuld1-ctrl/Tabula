// ### Change Log
// - 2026-03-15: Reason=TDD for API route alignment; Purpose=lock fetch paths and payloads

import { describe, it, expect, vi, afterEach } from "vitest";
// ### Change Log
// - 2026-03-15: Reason=Route alignment needs shared helpers; Purpose=verify new GridAPI calls
import { fetchVersions, updateStyleRange, ensureColumns } from "../GridAPI";

// ### Change Log
// - 2026-03-15: Reason=Global fetch must be isolated; Purpose=avoid cross-test leaks
const stubFetch = (impl: ReturnType<typeof vi.fn>) => {
  vi.stubGlobal("fetch", impl);
};

// ### Change Log
// - 2026-03-15: Reason=Reset stubbed globals; Purpose=keep test environment clean
afterEach(() => {
  vi.unstubAllGlobals();
});

// ### Change Log
// - 2026-03-15: Reason=Time machine relies on table_name; Purpose=ensure encoding is consistent
it("fetchVersions uses encoded table_name and returns json", async () => {
  const mockFetch = vi.fn().mockResolvedValue({
    ok: true,
    status: 200,
    json: async () => ({ status: "ok", versions: [] })
  } as Response);

  stubFetch(mockFetch);

  const result = await fetchVersions("汇总数据");
  expect(mockFetch).toHaveBeenCalledWith(`/api/versions?table_name=${encodeURIComponent("汇总数据")}`);
  expect(result.status).toBe("ok");
});

// ### Change Log
// - 2026-03-15: Reason=Style updates must follow existing POST pattern; Purpose=verify payload + route
it("updateStyleRange posts to update_style_range with json body", async () => {
  const payload = { table_name: "t", session_id: "s", range: { start_row: 0 }, style: { bold: true } };
  const mockFetch = vi.fn().mockResolvedValue({
    ok: true,
    status: 200,
    json: async () => ({ status: "ok" })
  } as Response);

  stubFetch(mockFetch);

  await updateStyleRange(payload);
  expect(mockFetch).toHaveBeenCalledWith(
    "/api/update_style_range",
    expect.objectContaining({
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload)
    })
  );
});

// ### Change Log
// - 2026-03-15: Reason=Pivot writes need schema expansion; Purpose=surface status on ensure_columns failure
it("ensureColumns throws with status when backend rejects", async () => {
  const mockFetch = vi.fn().mockResolvedValue({
    ok: false,
    status: 400,
    text: async () => "bad request"
  } as Response);

  stubFetch(mockFetch);

  await expect(ensureColumns({ table_name: "t", session_id: "s", columns: [] }))
    .rejects
    .toMatchObject({ status: 400 });
});
