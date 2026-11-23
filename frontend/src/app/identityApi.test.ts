import { afterEach, describe, expect, it, vi } from "vitest";
import {
  beginAdminLogin,
  completeAdminLogin,
  getCurrentAdmin,
  IdentityApiError,
  logoutAdmin,
} from "./identityApi";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("Admin identity API", () => {
  it("starts password login with same-origin credentials and no-store caching", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ message: "challenge issued" }), {
        status: 202,
        headers: { "Content-Type": "application/json" },
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await beginAdminLogin("admin@example.test", "test-password");

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/identity/admin/login",
      expect.objectContaining({
        method: "POST",
        credentials: "same-origin",
        cache: "no-store",
        body: JSON.stringify({
          identifier: "admin@example.test",
          password: "test-password",
        }),
        headers: expect.objectContaining({
          Accept: "application/json",
          "Content-Type": "application/json",
        }),
      }),
    );
  });

  it("submits only the authenticator code to the TOTP endpoint", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ message: "authenticated" }), {
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

  it("accepts only an Admin session response", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        new Response(JSON.stringify({ user_id: "admin-id", role: "admin" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      ),
    );

    await expect(getCurrentAdmin()).resolves.toEqual({
      user_id: "admin-id",
      role: "admin",
    });
  });

  it("rejects a non-Admin role in the current-session response", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        new Response(JSON.stringify({ user_id: "user-id", role: "owner" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      ),
    );

    await expect(getCurrentAdmin()).rejects.toMatchObject({
      name: "IdentityApiError",
      status: 502,
    });
  });

  it("logs out without trying to parse a 204 response body", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(null, { status: 204 }));
    vi.stubGlobal("fetch", fetchMock);

    await expect(logoutAdmin()).resolves.toBeUndefined();

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/identity/admin/logout",
      expect.objectContaining({
        method: "POST",
        credentials: "same-origin",
        cache: "no-store",
      }),
    );
  });

  it("preserves safe API errors for the login form", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        new Response(JSON.stringify({ error: "Login tidak berhasil." }), {
          status: 401,
          headers: { "Content-Type": "application/json" },
        }),
      ),
    );

    await expect(beginAdminLogin("admin@example.test", "wrong-password")).rejects.toEqual(
      expect.objectContaining({
        name: "IdentityApiError",
        status: 401,
        message: "Login tidak berhasil.",
      }),
    );
  });

  it("converts network failures into a typed connection error", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("network down")));

    await expect(getCurrentAdmin()).rejects.toBeInstanceOf(IdentityApiError);
    await expect(getCurrentAdmin()).rejects.toMatchObject({ status: 0 });
  });
});
