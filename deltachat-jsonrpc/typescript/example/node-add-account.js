import { DeltaChat } from "../dist/deltachat.js";

run().catch(console.error);

async function run() {
  const delta = new DeltaChat("ws://localhost:20808/ws");
  delta.on("event", (event) => {
    console.log("event", event.data);
  });

  const email = process.argv[2];
  const password = process.argv[3];
  if (!email || !password)
    throw new Error(
      "USAGE: node node-add-account.js <EMAILADDRESS> <PASSWORD>"
    );
  console.log(`creating acccount for ${email}`);
  const id = await delta.rpc.addAccount();
  console.log(`created account id ${id}`);
  await delta.rpc.setConfig(id, "addr", email);
  await delta.rpc.setConfig(id, "mail_pw", password);
  console.log("configuration updated");
  await delta.rpc.configure(id);
  console.log("account configured!");

  const accounts = await delta.rpc.getAllAccounts();
  console.log("accounts", accounts);
  console.log("waiting for events...");
}
