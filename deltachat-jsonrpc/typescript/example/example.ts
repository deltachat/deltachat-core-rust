import { DeltaChat, DeltaChatEvent } from "../deltachat.js";

var SELECTED_ACCOUNT = 0;

window.addEventListener("DOMContentLoaded", (_event) => {
  (window as any).selectDeltaAccount = (id: string) => {
    SELECTED_ACCOUNT = Number(id);
    window.dispatchEvent(new Event("account-changed"));
  };
  console.log('launch run script...')
  run().catch((err) => console.error("run failed", err));
});

async function run() {
  const $main = document.getElementById("main")!;
  const $side = document.getElementById("side")!;
  const $head = document.getElementById("header")!;

  const client = new DeltaChat('ws://localhost:20808/ws')

  ;(window as any).client = client.rpc;

  client.on("ALL", event => {
    onIncomingEvent(event)
  })

  window.addEventListener("account-changed", async (_event: Event) => {
    listChatsForSelectedAccount();
  });

  await Promise.all([loadAccountsInHeader(), listChatsForSelectedAccount()]);

  async function loadAccountsInHeader() {
    console.log('load accounts')
    const accounts = await client.rpc.getAllAccounts();
    console.log('accounts loaded', accounts)
    for (const account of accounts) {
      if (account.type === "Configured") {
        write(
          $head,
          `<a href="#" onclick="selectDeltaAccount(${account.id})">
          ${account.id}: ${account.addr!}
          </a>&nbsp;`
        );
      } else {
        write(
          $head,
          `<a href="#">
          ${account.id}: (unconfigured)
          </a>&nbsp;`
        )
      }
    }
  }

  async function listChatsForSelectedAccount() {
    clear($main);
    const selectedAccount = SELECTED_ACCOUNT
    const info = await client.rpc.getAccountInfo(selectedAccount);
    if (info.type !== "Configured") {
      return write($main, "Account is not configured");
    }
    write($main, `<h2>${info.addr!}</h2>`);
    const chats = await client.rpc.getChatlistEntries(
      selectedAccount,
      0,
      null,
      null
    );
    for (const [chatId, _messageId] of chats) {
      const chat = await client.rpc.chatlistGetFullChatById(
        selectedAccount,
        chatId
      );
      write($main, `<h3>${chat.name}</h3>`);
      const messageIds = await client.rpc.messageListGetMessageIds(
        selectedAccount,
        chatId,
        0
      );
      const messages = await client.rpc.messageGetMessages(
        selectedAccount,
        messageIds
      );
      for (const [_messageId, message] of Object.entries(messages)) {
        write($main, `<p>${message.text}</p>`);
      }
    }
  }

  function onIncomingEvent(event: DeltaChatEvent) {
    write(
      $side,
      `
        <p class="message">
          [<strong>${event.id}</strong> on account ${event.contextId}]<br>
          <em>f1:</em> ${JSON.stringify(event.field1)}<br>
          <em>f2:</em> ${JSON.stringify(event.field2)}
        </p>`
    );
  }
}

function write(el: HTMLElement, html: string) {
  el.innerHTML += html;
}
function clear(el: HTMLElement) {
  el.innerHTML = "";
}
