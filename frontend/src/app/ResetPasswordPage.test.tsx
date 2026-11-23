import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ResetPasswordPage } from "./ResetPasswordPage";

describe("ResetPasswordPage", () => {
  beforeEach(() => {
    window.history.replaceState(null, "", "/reset-password");
    vi.restoreAllMocks();
  });

  it("rejects a missing reset token without rendering a password form", async () => {
    render(<ResetPasswordPage />);

    expect(
      await screen.findByText(/tautan reset tidak valid atau sudah kedaluwarsa/i),
    ).toBeInTheDocument();
    expect(screen.queryByLabelText("Password baru")).not.toBeInTheDocument();
  });

  it("submits a token once and confirms successful password reset", async () => {
    window.history.replaceState(null, "", "/reset-password#token=" + "a".repeat(64));
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ message: "Password berhasil diperbarui. Silakan masuk kembali." }),
    });
    vi.stubGlobal("fetch", fetchMock);

    render(<ResetPasswordPage />);

    fireEvent.change(await screen.findByLabelText("Password baru"), {
      target: { value: "a-strong-new-password" },
    });
    fireEvent.change(screen.getByLabelText("Konfirmasi password baru"), {
      target: { value: "a-strong-new-password" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Simpan password baru" }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/identity/recovery/password-reset",
      expect.objectContaining({
        method: "POST",
        credentials: "same-origin",
        body: JSON.stringify({
          token: "a".repeat(64),
          new_password: "a-strong-new-password",
          confirm_password: "a-strong-new-password",
        }),
      }),
    );
    expect(await screen.findByRole("status")).toHaveTextContent(
      "Password berhasil diperbarui. Silakan masuk kembali.",
    );
    expect(window.location.hash).toBe("");
  });

  it("does not send a request when the password confirmation differs", async () => {
    window.history.replaceState(null, "", "/reset-password#token=" + "b".repeat(64));
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);

    render(<ResetPasswordPage />);

    fireEvent.change(await screen.findByLabelText("Password baru"), {
      target: { value: "a-strong-new-password" },
    });
    fireEvent.change(screen.getByLabelText("Konfirmasi password baru"), {
      target: { value: "a-different-password" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Simpan password baru" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Konfirmasi password belum cocok.",
    );
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
