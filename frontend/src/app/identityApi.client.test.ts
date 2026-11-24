import { afterEach, describe, expect, it, vi } from "vitest";
import { getCurrentClient, IdentityApiError, loginClient } from "./identityApi";

afterEach(() => vi.unstubAllGlobals());

describe("Client identity API", () => {
  it("uses the Owner login endpoint with same-origin no-store credentials", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response("{}", { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);

    await loginClient("business_owner", "owner@example.test", "test-password");

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/identity/login/owner",
      expect.objectContaining({
        method: "POST",
        credentials: "same-origin",
        cache: "no-store",
        body: JSON.stringify({
          identifier: "owner@example.test",
          password: "test-password",
        }),
      }),
    );
  });

  it("uses the Investor login endpoint", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response("{}", { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);

    await loginClient("investor", "investor@example.test", "test-password");

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/identity/login/investor",
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("accepts only a session matching the selected client role", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        new Response(JSON.stringify({ user_id: "owner-1", role: "business_owner" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      ),
    );

    await expect(getCurrentClient("business_owner")).resolves.toEqual({
      user_id: "owner-1",
      role: "business_owner",
    });
    await expect(getCurrentClient("investor")).rejects.toBeInstanceOf(IdentityApiError);
  });
});
