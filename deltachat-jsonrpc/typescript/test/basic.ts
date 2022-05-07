import { strictEqual } from "assert";
import chai, { assert, expect } from "chai";
import chaiAsPromised from "chai-as-promised";
chai.use(chaiAsPromised);
import { Deltachat } from "../dist/deltachat.js";

import {
  CMD_API_Server_Handle,
  CMD_API_SERVER_PORT,
  startCMD_API_Server,
} from "./test_base.js";

describe("basic tests", () => {
  let server_handle: CMD_API_Server_Handle;
  let dc: Deltachat;

  before(async () => {
    server_handle = await startCMD_API_Server(CMD_API_SERVER_PORT);
    // make sure server is up by the time we continue
    await new Promise((res) => setTimeout(res, 100));

    dc = new Deltachat({
      url: "ws://localhost:" + CMD_API_SERVER_PORT + "/ws",
    });
    dc.on("ALL", (event) => {
      //console.log("event", event);
    });
  });

  after(async () => {
    dc && dc.close();
    await server_handle.close();
  });

  it("check email", async () => {
    const positive_test_cases = [
      "email@example.com",
      "36aa165ae3406424e0c61af17700f397cad3fe8ab83d682d0bddf3338a5dd52e@yggmail@yggmail",
    ];
    const negative_test_cases = ["email@", "example.com", "emai221"];

    expect(
      await Promise.all(
        positive_test_cases.map((email) => dc.rpc.checkEmailValidity(email))
      )
    ).to.not.contain(false);

    expect(
      await Promise.all(
        negative_test_cases.map((email) => dc.rpc.checkEmailValidity(email))
      )
    ).to.not.contain(true);
  });

  it("system info", async () => {
    const system_info = await dc.rpc.getSystemInfo();
    expect(system_info).to.contain.keys([
      "arch",
      "num_cpus",
      "deltachat_core_version",
      "sqlite_version",
    ]);
  });

  describe("account managment", () => {
    it("should create account", async () => {
      await dc.rpc.addAccount();
      assert((await dc.rpc.getAllAccountIds()).length === 1);
    });

    it("should remove the account again", async () => {
      await dc.rpc.removeAccount((await dc.rpc.getAllAccountIds())[0]);
      assert((await dc.rpc.getAllAccountIds()).length === 0);
    });

    it("should create multiple accounts", async () => {
      await dc.rpc.addAccount();
      await dc.rpc.addAccount();
      await dc.rpc.addAccount();
      await dc.rpc.addAccount();
      assert((await dc.rpc.getAllAccountIds()).length === 4);
    });
  });

  describe("contact managment", function () {
    let acc: number;
    before(async () => {
      acc = await dc.rpc.addAccount();
    });
    it("block and unblock contact", async function () {
      const contactId = await dc.rpc.contactsCreateContact(
        acc,
        "example@delta.chat",
        null
      );
      expect((await dc.rpc.contactsGetContact(acc, contactId)).is_blocked).to.be
        .false;
      await dc.rpc.contactsBlock(acc, contactId);
      expect((await dc.rpc.contactsGetContact(acc, contactId)).is_blocked).to.be
        .true;
      expect(await dc.rpc.contactsGetBlocked(acc)).to.have.length(1);
      await dc.rpc.contactsUnblock(acc, contactId);
      expect((await dc.rpc.contactsGetContact(acc, contactId)).is_blocked).to.be
        .false;
      expect(await dc.rpc.contactsGetBlocked(acc)).to.have.length(0);
    });
  });

  describe("configuration", function () {
    let acc: number;
    before(async () => {
      acc = await dc.rpc.addAccount();
    });

    it("set and retrive", async function () {
      await dc.rpc.setConfig(acc, "addr", "valid@email");
      assert((await dc.rpc.getConfig(acc, "addr")) == "valid@email");
    });
    it("set invalid key should throw", async function () {
      await expect(dc.rpc.setConfig(acc, "invalid_key", "some value")).to.be
        .eventually.rejected;
    });
    it("get invalid key should throw", async function () {
      await expect(dc.rpc.getConfig(acc, "invalid_key")).to.be.eventually
        .rejected;
    });
    it("set and retrive ui.*", async function () {
      await dc.rpc.setConfig(acc, "ui.chat_bg", "color:red");
      assert((await dc.rpc.getConfig(acc, "ui.chat_bg")) == "color:red");
    });
    it("set and retrive (batch)", async function () {
      const config = { addr: "valid@email", mail_pw: "1234" };
      await dc.rpc.batchSetConfig(acc, config);
      const retrieved = await dc.rpc.batchGetConfig(acc, Object.keys(config));
      expect(retrieved).to.deep.equal(config);
    });
    it("set and retrive ui.* (batch)", async function () {
      const config = {
        "ui.chat_bg": "color:green",
        "ui.enter_key_sends": "true",
      };
      await dc.rpc.batchSetConfig(acc, config);
      const retrieved = await dc.rpc.batchGetConfig(acc, Object.keys(config));
      expect(retrieved).to.deep.equal(config);
    });
    it("set and retrive mixed(ui and core) (batch)", async function () {
      const config = {
        "ui.chat_bg": "color:yellow",
        "ui.enter_key_sends": "false",
        addr: "valid2@email",
        mail_pw: "123456",
      };
      await dc.rpc.batchSetConfig(acc, config);
      const retrieved = await dc.rpc.batchGetConfig(acc, Object.keys(config));
      expect(retrieved).to.deep.equal(config);
    });
  });
});
