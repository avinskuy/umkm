import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AdminDashboardPage } from "./AdminDashboardPage";
import { getCurrentAdmin, IdentityApiError, logoutAdmin } from "./identityApi";
import { navigateTo } from "./navigation";

vi.mock("./identityApi", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./identityApi")>();
  return {
    ...actual,
    getCurrentAdmin: vi.fn(),
    logoutAdmin: vi.fn(),
  };
});

vi.mock("./navigation", () => ({ navigateTo: vi.fn() }));

describe("AdminDashboardPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getCurrentAdmin).mockResolvedValue({
      user_id: "admin-user-id",
      role: "admin",
    });
    vi.mocked(logoutAdmin).mockResolvedValue(undefined);
  });

  afterEach(() => {
    cleanup();
  });

  it("renders only after the server confirms an Admin session and supports logout", async () => {
    render(<AdminDashboardPage />);

    expect(
      await screen.findByRole("heading", { name: "Dashboard Admin" }),
    ).toBeInTheDocument();
    expect(screen.getByText("admin-user-id")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Logout" }));

    await waitFor(() => {
      expect(logoutAdmin).toHaveBeenCalledOnce();
      expect(navigateTo).toHaveBeenCalledWith("/admin/login");
    });
  });

  it("redirects to login when the server rejects the Admin session", async () => {
    vi.mocked(getCurrentAdmin).mockRejectedValueOnce(
      new IdentityApiError(401, "Sesi Admin tidak valid."),
    );
    render(<AdminDashboardPage />);

    await waitFor(() => {
      expect(navigateTo).toHaveBeenCalledWith("/admin/login");
    });
  });
});
