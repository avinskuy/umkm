import { afterEach, describe, expect, it, vi } from "vitest";
import {
  beginAdminLogin,
  completeAdminLogin,
  getCurrentAdmin,
  IdentityApiError,
  logoutAdmin,
} from "./identityApi";

afterEach(() => vi.unstubAllGlobals());

describe("Admin identity API", () => {
  it("uses same-origin, no-store requests for the first login step", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response("{}", {
        status: 202,
        headers: { "Content-Type": "application/json" },
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await beginAdminLogin("admin@example.test", "demo-value");

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/identity/admin/login",
      expect.objectContaining({
        method: "POST",
        credentials: "same-origin",
        cache: "no-store",
        body: JSON.stringify({
          identifier: "admin@example.test",
          password: "demo-value",
        }),
      }),
    );
  });

  it("sends the authenticator code to the TOTP endpoint", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response("{}", {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await completeAdminLogin("123456");

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/identity/admin/login/totp",
      expect.objectContaining({
        method: "POST",
        credentials: "same-origin",
        cache: "no-store",
        body: JSON.stringify({ code: "123456" }),
      }),
    );
  });

  it("accepts an Admin session and rejects a different role", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ user_id: "id", role: "admin" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ user_id: "id", role: "owner" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );
    vi.stubGlobal("fetch", fetchMock);

    await expect(getCurrentAdmin()).resolves.toEqual({
      user_id: "id",
      role: "admin",
    });
    await expect(getCurrentAdmin()).rejects.toMatchObject({ status: 502 });
  });

  it("handles a 204 logout response without parsing JSON", async () => {
    const response = new Response(null, { status: 204 });
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(response));

    await expect(logoutAdmin()).resolves.toBeUndefined();
  });

  it("converts network errors into a typed connection error", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));

    await expect(getCurrentAdmin()).rejects.toBeInstanceOf(IdentityApiError);
  });
});
