import { afterEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ClientLoginPage } from "./ClientLoginPage";

afterEach(() => vi.unstubAllGlobals());

describe("ClientLoginPage", () => {
  it("defaults to Business Owner and sends credentials to the owner endpoint", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(new Response("{}", { status: 200 }))
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({ user_id: "owner-1", role: "business_owner" }),
          {
            status: 200,
            headers: { "Content-Type": "application/json" },
          },
        ),
      );
    vi.stubGlobal("fetch", fetchMock);

    render(<ClientLoginPage />);
    fireEvent.change(screen.getByLabelText(/email atau nomor whatsapp/i), {
      target: { value: "owner@example.test" },
    });
    fireEvent.change(screen.getByLabelText("Password"), {
      target: { value: "not-a-real-password" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Masuk" }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "/api/v1/identity/login/owner",
      expect.objectContaining({
        method: "POST",
        credentials: "same-origin",
        cache: "no-store",
        body: JSON.stringify({
          identifier: "owner@example.test",
          password: "not-a-real-password",
        }),
      }),
    );
    expect(await screen.findByRole("status")).toHaveTextContent(
      "Berhasil masuk sebagai Pemilik Usaha.",
    );
  });

  it("uses the investor endpoint when Investor is selected", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(new Response("{}", { status: 200 }))
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({ user_id: "investor-1", role: "investor" }),
          {
            status: 200,
            headers: { "Content-Type": "application/json" },
          },
        ),
      );
    vi.stubGlobal("fetch", fetchMock);

    render(<ClientLoginPage />);
    fireEvent.click(screen.getByRole("button", { name: /Investor/i }));
    fireEvent.change(screen.getByLabelText(/email atau nomor whatsapp/i), {
      target: { value: "investor@example.test" },
    });
    fireEvent.change(screen.getByLabelText("Password"), {
      target: { value: "not-a-real-password" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Masuk" }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "/api/v1/identity/login/investor",
      expect.objectContaining({ method: "POST" }),
    );
    expect(await screen.findByRole("status")).toHaveTextContent(
      "Berhasil masuk sebagai Investor.",
    );
  });

  it("shows API errors without exposing credentials", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        new Response(
          JSON.stringify({ error: "Email/nomor telepon atau password salah." }),
          {
            status: 401,
            headers: { "Content-Type": "application/json" },
          },
        ),
      ),
    );

    render(<ClientLoginPage />);
    fireEvent.change(screen.getByLabelText(/email atau nomor whatsapp/i), {
      target: { value: "owner@example.test" },
    });
    fireEvent.change(screen.getByLabelText("Password"), {
      target: { value: "not-a-real-password" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Masuk" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Email/nomor telepon atau password salah.",
    );
  });
});
