import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { RegistrationPage } from "./RegistrationPage";

afterEach(() => cleanup());

describe("RegistrationPage", () => {
  it("lets the visitor choose one client role", () => {
    render(<RegistrationPage />);

    const owner = screen.getByRole("button", { name: /Pemilik Usaha/i });
    const investor = screen.getByRole("button", { name: /Investor/i });

    expect(owner).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(investor);
    expect(investor).toHaveAttribute("aria-pressed", "true");
    expect(owner).toHaveAttribute("aria-pressed", "false");
  });

  it("switches the contact field between email and WhatsApp", () => {
    render(<RegistrationPage />);

    expect(screen.getByLabelText("Alamat email")).toHaveAttribute("type", "email");
    fireEvent.click(screen.getByRole("button", { name: "WhatsApp" }));

    expect(screen.getByLabelText("Nomor WhatsApp")).toHaveAttribute("type", "tel");
    expect(screen.getByLabelText("Nomor WhatsApp")).toHaveAttribute(
      "placeholder",
      "Contoh: +6281234567890",
    );
  });

  it("does not submit or send data while public registration is unavailable", () => {
    render(<RegistrationPage />);

    expect(
      screen.getByRole("button", { name: "Pendaftaran segera tersedia" }),
    ).toBeDisabled();
    expect(screen.getByRole("status")).toHaveTextContent(
      "Form ini merupakan rancangan antarmuka dan belum mengirim data.",
    );
  });
});
