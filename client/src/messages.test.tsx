import { describe, expect, test } from "vitest";

import { render, fireEvent } from "solid-testing-library";

import { Messages } from "./messages";

describe("<Messages />", () => {
  test("it will render a text input", () => {
    const { getByRole, unmount } = render(() => <Messages />);
    expect(getByRole("textbox")).toBeInTheDocument();
    unmount();
  });

  test("it will add a new message", async () => {
    const { getByRole, getByText, unmount } = render(() => <Messages />);
    const input = getByRole("textbox") as HTMLInputElement;
    input.value = "test new todo";
    fireEvent.keyDown(input, { key: "Enter", code: "Enter", charCode: 13 });
    expect(input.value).toBe("");
    expect(getByText(/test new todo/)).toBeInTheDocument();
    unmount();
  });

  test("it will clear messages", async () => {
    const { getByRole, getByText, queryByText, unmount } = render(() => (
      <Messages />
    ));
    const input = getByRole("textbox") as HTMLInputElement;
    input.value = "test new todo";
    fireEvent.keyDown(input, { key: "Enter", code: "Enter", charCode: 13 });
    expect(input.value).toBe("");
    expect(getByText(/test new todo/)).toBeInTheDocument();
    input.value = "clear";
    fireEvent.keyDown(input, { key: "Enter", code: "Enter", charCode: 13 });
    expect(input.value).toBe("");
    expect(queryByText(/test new todo/)).not.toBeInTheDocument();
    unmount();
  });

  test("re-focuses when click outside input", () => {
    const { getByRole, getByText, unmount } = render(() => <Messages />);
    const input = getByRole("textbox") as HTMLInputElement;
    input.value = "test new todo";
    fireEvent.keyDown(input, { key: "Enter", code: "Enter", charCode: 13 });
    expect(input.value).toBe("");
    expect(getByText(/test new todo/)).toBeInTheDocument();
    const newMessage = getByText("test new todo") as HTMLSpanElement;
    input.blur();
    newMessage.click();
    expect(input).toHaveFocus();
    unmount();
  });
});
