import { Deltachat } from "../dist/deltachat.js";

run().catch(console.error);

async function run() {
  const delta = new Deltachat();
  delta.on("event", (event) => {
    console.log("event", event.data);
  });

  const accounts = await delta.rpc.getAllAccounts();
  console.log("accounts", accounts);
  console.log("waiting for events...")
}
