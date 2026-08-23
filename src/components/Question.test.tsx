import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import Question from "./Question";

describe("Question", () => {
  it("sits on the right, opposite the answer", () => {
    render(<Question text="what is a spectral class" />);
    const question = screen.getByText("what is a spectral class");
    expect(question.parentElement?.className).toContain("justify-end");
  });

  // A pasted snippet is the common case, and collapsing it would rewrite what
  // the user actually asked.
  it("keeps the line breaks the user typed", () => {
    render(<Question text={"one\ntwo"} />);
    expect(screen.getByText(/one/).className).toContain("whitespace-pre-wrap");
  });
});
