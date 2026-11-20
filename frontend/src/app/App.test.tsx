import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App";

describe("App", () => {
  it("renders the USahapintar project identity", () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "USahapintar" })).toBeInTheDocument();
    expect(screen.getByText(/business owners and investors/i)).toBeInTheDocument();
  });
});
