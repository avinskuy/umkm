import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AdminLoginPage } from "./AdminLoginPage";
import { beginAdminLogin, completeAdminLogin, IdentityApiError } from "./identityApi";
import { navigateTo } from "./navigation";

vi.mock("./identityApi", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./identityApi")>();
  return {
    ...actual,
    beginAdminLogin: vi.fn(),
    completeAdminLogin: vi.fn(),
  };
});

vi.mock("./navigation", () => ({ navigateTo: vi.fn() }));

describe("AdminLoginPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(beginAdminLogin).mockResolvedValue(undefined);
    vi.mocked(completeAdminLogin).mockResolvedValue(undefined);
  });

  afterEach(() => {
    cleanup();
  });

  it("completes password and TOTP steps before navigating to the Admin workspace", async () => {
    render(<AdminLoginPage />);

    fireEvent.change(screen.getByLabelText("Email atau nomor terdaftar"), {
      target: { value: " admin@example.test " },
    });
    fireEvent.change(screen.getByLabelText("Password"), {
      target: { value: "demo-value" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Lanjutkan" }));

    await waitFor(() => {
      expect(beginAdminLogin).toHaveBeenCalledWith("admin@example.test", "demo-value");
    });
    expect(await screen.findByLabelText("Kode authenticator")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Kode authenticator"), {
      target: { value: "12a34567" },
    });
    expect(screen.getByLabelText("Kode authenticator")).toHaveValue("123456");
    fireEvent.click(screen.getByRole("button", { name: "Verifikasi dan masuk" }));

    await waitFor(() => {
      expect(completeAdminLogin).toHaveBeenCalledWith("123456");
      expect(navigateTo).toHaveBeenCalledWith("/admin");
    });
  });

  it("shows a safe authentication error without advancing the login phase", async () => {
    vi.mocked(beginAdminLogin).mockRejectedValueOnce(
      new IdentityApiError(401, "Login tidak berhasil."),
    );
    render(<AdminLoginPage />);

    fireEvent.change(screen.getByLabelText("Email atau nomor terdaftar"), {
      target: { value: "admin@example.test" },
    });
    fireEvent.change(screen.getByLabelText("Password"), {
      target: { value: "incorrect-value" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Lanjutkan" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("Login tidak berhasil.");
    expect(screen.getByLabelText("Password")).toBeInTheDocument();
    expect(completeAdminLogin).not.toHaveBeenCalled();
  });
});
