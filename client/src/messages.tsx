import { For, createSignal, createEffect } from "solid-js";
import { useApollo } from "./apollo";
import { gql } from "@apollo/client/core";

import styles from "./messages.module.css";

type Message = { text: string };

type QResult = {
  data: { numberIncremented: number };
};

export const Messages = () => {
  const client = useApollo();

  createEffect(() => {
    const subscription = client
      .subscribe({
        query: gql`
          subscription {
            messages {
              text
            }
          }
        `,
      })
      .subscribe(
        ({
          data: {
            messages: { text },
          },
        }) => {
          setMessages((m) => [...m, { text }]);
          scrollToBottom();
          // setNumber(data);
        }
      );

    return () => {
      subscription.unsubscribe();
    };
  }, []);

  let input!: HTMLInputElement;
  let messagesList!: HTMLUListElement;

  const [messages, setMessages] = createSignal<Message[]>([]);
  const [focused, setFocused] = createSignal(false);

  const focus = () => {
    if (!focused()) {
      input.focus();
    }
  };

  createEffect(() => {
    focus();
  }, [focus]);

  const onKeyDown = async (e: KeyboardEvent) => {
    const text = input.value;

    switch (e.key) {
      case "Enter":
        // Send message if non-empty
        if (input.value.length > 0) {
          try {
            await client.mutate({
              variables: { text },
              mutation: gql`
                mutation Message($text: String) {
                  message(text: $text)
                }
              `,
            });
          } catch (err) {
            console.error("UNABLE TO MUTATE: ", err);
          }

          scrollToBottom();
          input.value = "";
        }

        // Naive clear command
        if (text.toLowerCase() === "clear") {
          setMessages([]);
        }

        break;
    }
  };

  const scrollToBottom = () => {
    if (messagesList) {
      const scrollHeight = messagesList.scrollHeight;
      const height = messagesList.clientHeight;
      const maxScrollTop = scrollHeight - height;
      /* istanbul ignore next */
      messagesList.scrollTop = maxScrollTop > 0 ? maxScrollTop : 0;
    }
  };

  return (
    <main class={styles.container} onClick={focus}>
      <ul class={styles.messagesContainer} ref={messagesList}>
        <For each={messages()}>
          {(message) => (
            <li class={styles.line}>
              <span>~</span>
              <span>{message.text}</span>
            </li>
          )}
        </For>
      </ul>

      <div class={styles.inputContainer}>
        <div class={styles.line}>
          <span>~</span>
          <input
            role="textbox"
            class={styles.input}
            ref={input}
            onFocus={() => setFocused(true)}
            onBlur={() => setFocused(false)}
            onKeyDown={onKeyDown}
          />
        </div>
      </div>
    </main>
  );
};
