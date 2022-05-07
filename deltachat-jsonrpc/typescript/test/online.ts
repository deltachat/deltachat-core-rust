import { assert, expect } from "chai";
import {
  Deltachat,
  DeltachatEvent,
  eventIdToName,
  Event_TypeID,
} from "../dist/deltachat.js";
import {
  CMD_API_Server_Handle,
  CMD_API_SERVER_PORT,
  createTempUser,
  startCMD_API_Server,
} from "./test_base.js";

describe("online tests", function () {
  let server_handle: CMD_API_Server_Handle;
  let dc: Deltachat;
  let account: { email: string; password: string };
  let account2: { email: string; password: string };
  let acc1: number, acc2: number;

  before(async function () {
    if (!process.env.DCC_NEW_TMP_EMAIL) {
      if (process.env.COVERAGE && !process.env.COVERAGE_OFFLINE) {
        console.error(
          "CAN NOT RUN COVERAGE correctly: Missing DCC_NEW_TMP_EMAIL environment variable!\n\n",
          "You can set COVERAGE_OFFLINE=1 to circumvent this check and skip the online tests, but those coverage results will be wrong, because some functions can only be tested in the online test"
        );
        process.exit(1);
      }
      console.log(
        "Missing DCC_NEW_TMP_EMAIL environment variable!, skip intergration tests"
      );
      this.skip();
    }
    server_handle = await startCMD_API_Server(CMD_API_SERVER_PORT);
    dc = new Deltachat({
      url: "ws://localhost:" + CMD_API_SERVER_PORT + "/ws",
    });

    account = await createTempUser(process.env.DCC_NEW_TMP_EMAIL);
    if (!account || !account.email || !account.password) {
      console.log(
        "We didn't got back an account from the api, skip intergration tests"
      );
      this.skip();
    }

    account2 = await createTempUser(process.env.DCC_NEW_TMP_EMAIL);
    if (!account2 || !account2.email || !account2.password) {
      console.log(
        "We didn't got back an account2 from the api, skip intergration tests"
      );
      this.skip();
    }
  });

  after(async () => {
    dc && dc.close();
    server_handle && (await server_handle.close());
  });

  let are_configured = false;

  it("configure test accounts", async function () {
    this.timeout(20000);

    acc1 = await dc.rpc.addAccount();
    await dc.rpc.setConfig(acc1, "addr", account.email);
    await dc.rpc.setConfig(acc1, "mail_pw", account.password);
    let configure_promise = dc.rpc.configure(acc1);

    acc2 = await dc.rpc.addAccount();
    await dc.rpc.batchSetConfig(acc2, {
      addr: account2.email,
      mail_pw: account2.password,
    });

    await Promise.all([configure_promise, dc.rpc.configure(acc2)]);
    are_configured = true;
  });

  it("send and recieve text message", async function () {
    if (!are_configured) {
      this.skip();
    }
    this.timeout(15000);

    const contactId = await dc.rpc.contactsCreateContact(
      acc1,
      account2.email,
      null
    );
    const chatId = await dc.rpc.contactsCreateChatByContactId(acc1, contactId);
    const eventPromise = waitForEvent(dc, "INCOMING_MSG", acc2);
    dc.rpc.miscSendTextMessage(acc1, "Hello", chatId);
    const { field1: chatIdOnAccountB } = await eventPromise;
    await dc.rpc.acceptChat(acc2, chatIdOnAccountB);
    const messageList = await dc.rpc.messageListGetMessageIds(
      acc2,
      chatIdOnAccountB,
      0
    );

    expect(messageList).have.length(1);
    const message = await dc.rpc.messageGetMessage(acc2, messageList[0]);
    expect(message.text).equal("Hello");
  });

  it("send and recieve text message roundtrip, encrypted on answer onwards", async function () {
    if (!are_configured) {
      this.skip();
    }
    this.timeout(7000);

    // send message from A to B
    const contactId = await dc.rpc.contactsCreateContact(
      acc1,
      account2.email,
      null
    );
    const chatId = await dc.rpc.contactsCreateChatByContactId(acc1, contactId);
    dc.rpc.miscSendTextMessage(acc1, "Hello2", chatId);
    // wait for message from A
    const event = await waitForEvent(dc, "INCOMING_MSG", acc2);
    const { field1: chatIdOnAccountB } = event;

    await dc.rpc.acceptChat(acc2, chatIdOnAccountB);
    const messageList = await dc.rpc.messageListGetMessageIds(
      acc2,
      chatIdOnAccountB,
      0
    );
    const message = await dc.rpc.messageGetMessage(
      acc2,
      messageList.reverse()[0]
    );
    expect(message.text).equal("Hello2");
    // Send message back from B to A
    dc.rpc.miscSendTextMessage(acc2, "super secret message", chatId);
    // Check if answer arives at A and if it is encrypted
    await waitForEvent(dc, "INCOMING_MSG", acc1);

    const messageId = (
      await dc.rpc.messageListGetMessageIds(acc1, chatId, 0)
    ).reverse()[0];
    const message2 = await dc.rpc.messageGetMessage(acc1, messageId);
    expect(message2.text).equal("super secret message");
    expect(message2.show_padlock).equal(true);
  });

  it("get provider info for example.com", async () => {
    const acc = await dc.rpc.addAccount();
    const info = await dc.rpc.getProviderInfo(acc, "example.com");
    expect(info).to.be.not.null;
    expect(info?.overview_page).to.equal(
      "https://providers.delta.chat/example-com"
    );
    expect(info?.status).to.equal(3);
  });

  it("get provider info - domain and email should give same result", async () => {
    const acc = await dc.rpc.addAccount();
    const info_domain = await dc.rpc.getProviderInfo(acc, "example.com");
    const info_email = await dc.rpc.getProviderInfo(acc, "hi@example.com");
    expect(info_email).to.deep.equal(info_domain);
  });
});

type event_data = {
  contextId: number;
  id: Event_TypeID;
  [key: string]: any;
};
async function waitForEvent(
  dc: Deltachat,
  event: ReturnType<typeof eventIdToName>,
  accountId: number
): Promise<event_data> {
  return new Promise((res, rej) => {
    const callback = (ev: DeltachatEvent) => {
      if (ev.contextId == accountId) {
        dc.off(event, callback);
        res(ev);
      }
    };
    dc.on(event, callback);
  });
}
