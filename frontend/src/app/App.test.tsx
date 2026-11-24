import { render } from "@testing-library/react";
import { screen } from "@testing-library/dom";
import { beforeEach, describe, expect, it } from "vitest";
import { App } from "./App";

describe("App", () => {
  beforeEach(() => {
    window.history.replaceState(null, "", "/");
  });

  it("renders the USahapintar project identity", () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "USahapintar" })).toBeInTheDocument();
    expect(screen.getByText(/business owners and investors/i)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Masuk" })).toHaveAttribute(
      "href",
      "/login",
    );
  });

  it("routes the client login path to the Owner and Investor login page", () => {
    window.history.replaceState(null, "", "/login");

    render(<App />);

    expect(
      screen.getByRole("heading", { name: "Selamat datang kembali" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("group", { name: "Jenis akun" })).toBeInTheDocument();
  });

  it("routes the registration path to the registration page", () => {
    window.history.replaceState(null, "", "/register");

    render(<App />);

    expect(screen.getByRole("heading", { name: "Buat akun baru" })).toBeInTheDocument();
    expect(screen.getByRole("group", { name: "Jenis akun" })).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Pendaftaran belum dibuka.");
  });

  it("routes the Admin login path to the dedicated login page", () => {
    window.history.replaceState(null, "", "/admin/login");

    render(<App />);

    expect(screen.getByRole("heading", { name: "Admin sign in" })).toBeInTheDocument();
  });

  it("routes the Admin workspace path to the protected dashboard shell", () => {
    window.history.replaceState(null, "", "/admin");

    render(<App />);

    expect(
      screen.getByRole("heading", { name: "Memeriksa sesi administrator" }),
    ).toBeInTheDocument();
  });
});
