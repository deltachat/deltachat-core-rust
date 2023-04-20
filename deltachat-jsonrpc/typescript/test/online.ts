import { assert, expect } from "chai";
import { StdioDeltaChat as DeltaChat, DcEvent, T as X } from "../deltachat.js";
import { RpcServerHandle, createTempUser, startServer } from "./test_base.js";

const EVENT_TIMEOUT = 20000;

describe("online tests", function () {
  let serverHandle: RpcServerHandle;
  let dc: DeltaChat;
  let account1: { email: string; password: string };
  let account2: { email: string; password: string };
  let accountId1: number, accountId2: number;

  before(async function () {
    this.timeout(60000);
    if (!process.env.DCC_NEW_TMP_EMAIL) {
      if (process.env.COVERAGE && !process.env.COVERAGE_OFFLINE) {
        console.error(
          "CAN NOT RUN COVERAGE correctly: Missing DCC_NEW_TMP_EMAIL environment variable!\n\n",
          "You can set COVERAGE_OFFLINE=1 to circumvent this check and skip the online tests, but those coverage results will be wrong, because some functions can only be tested in the online test"
        );
        process.exit(1);
      }
      console.log(
        "Missing DCC_NEW_TMP_EMAIL environment variable!, skip integration tests"
      );
      this.skip();
    }
    serverHandle = await startServer();
    dc = new DeltaChat(serverHandle.stdin, serverHandle.stdout);

    account1 = await createTempUser(process.env.DCC_NEW_TMP_EMAIL);
    if (!account1 || !account1.email || !account1.password) {
      console.log(
        "We didn't got back an account from the api, skip integration tests"
      );
      this.skip();
    }

    account2 = await createTempUser(process.env.DCC_NEW_TMP_EMAIL);
    if (!account2 || !account2.email || !account2.password) {
      console.log(
        "We didn't got back an account2 from the api, skip integration tests"
      );
      this.skip();
    }
  });

  after(async () => {
    dc && dc.close();
    serverHandle && (await serverHandle.close());
  });

  let accountsConfigured = false;

  it("configure test accounts", async function () {
    this.timeout(40000);

    accountId1 = await dc.rpc.addAccount();
    await dc.rpc.setConfig(accountId1, "addr", account1.email);
    await dc.rpc.setConfig(accountId1, "mail_pw", account1.password);
    await dc.rpc.configure(accountId1);

    accountId2 = await dc.rpc.addAccount();
    await dc.rpc.batchSetConfig(accountId2, {
      addr: account2.email,
      mail_pw: account2.password,
    });
    await dc.rpc.configure(accountId2);
    accountsConfigured = true;
  });

  it("send and receive text message", async function () {
    if (!accountsConfigured) {
      this.skip();
    }
    this.timeout(15000);

    const contactId = await dc.rpc.createContact(
      accountId1,
      account2.email,
      null
    );
    const chatId = await dc.rpc.createChatByContactId(accountId1, contactId);

    await dc.rpc.miscSendTextMessage(accountId1, chatId, "Hello");
    const { chatId: chatIdOnAccountB } = await waitForMessageEvent(
      dc,
      accountId2
    );
    await dc.rpc.acceptChat(accountId2, chatIdOnAccountB);
    const messageList = await dc.rpc.getMessageIds(
      accountId2,
      chatIdOnAccountB,
      false,
      false
    );

    expect(messageList).have.length(1);
    const message = await dc.rpc.getMessage(accountId2, messageList[0]);
    expect(message.text).equal("Hello");
  });

  it("send and receive text message roundtrip, encrypted on answer onwards", async function () {
    if (!accountsConfigured) {
      this.skip();
    }
    this.timeout(10000);

    // send message from A to B
    const contactId = await dc.rpc.createContact(
      accountId1,
      account2.email,
      null
    );
    const chatId = await dc.rpc.createChatByContactId(accountId1, contactId);
    dc.rpc.miscSendTextMessage(accountId1, chatId, "Hello2");
    // wait for message from A
    console.log("wait for message from A");

    const event = await waitForMessageEvent(dc, accountId2);
    const { chatId: chatIdOnAccountB } = event;

    await dc.rpc.acceptChat(accountId2, chatIdOnAccountB);
    const messageList = await dc.rpc.getMessageIds(
      accountId2,
      chatIdOnAccountB,
      false,
      false
    );
    const message = await dc.rpc.getMessage(
      accountId2,
      messageList.reverse()[0]
    );
    expect(message.text).equal("Hello2");
    // Send message back from B to A
    dc.rpc.miscSendTextMessage(accountId2, chatId, "super secret message");
    // Check if answer arives at A and if it is encrypted
    await waitForMessageEvent(dc, accountId1);

    const messageId = (
      await dc.rpc.getMessageIds(accountId1, chatId, false, false)
    ).reverse()[0];
    const message2 = await dc.rpc.getMessage(accountId1, messageId);
    expect(message2.text).equal("super secret message");
    expect(message2.showPadlock).equal(true);
  });

  it("get provider info for example.com", async () => {
    const acc = await dc.rpc.addAccount();
    const info = await dc.rpc.getProviderInfo(acc, "example.com");
    expect(info).to.be.not.null;
    expect(info?.overviewPage).to.equal(
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

async function waitForMessageEvent<T extends DcEvent["type"]>(
  dc: DeltaChat,
  accountId: number
): Promise<Extract<X.EventType, { type: T; chatId: number }>> {
  while (true) {
    const event: X.Event = await dc.rpc.getNextEvent();
    if (event.event.type == "IncomingMsg" && event.context_id == accountId) {
      return event.event as any;
    }
  }
}
