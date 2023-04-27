import { assert, test } from "vitest";
import { runScenario, pause } from "@holochain/tryorama";
import { Record } from "@holochain/client";
import { createMew } from "./common";
import { mewsfeedAppBundleSource } from "../../utils";

test("create a Mew and get latest mews", async () => {
  await runScenario(
    async (scenario) => {
      // Set up the app to be installed
      const appSource = { appBundleSource: mewsfeedAppBundleSource };

      // Add 2 players with the test app to the Scenario. The returned players
      // can be destructured.
      const [alice, bob] = await scenario.addPlayersWithApps([
        appSource,
        appSource,
      ]);

      // Shortcut peer discovery through gossip and register all agents in every
      // conductor of the scenario.
      await scenario.shareAllAgents();

      // Bob gets latest mews
      let collectionOutput: Record[] = await bob.cells[0].callZome({
        zome_name: "mews",
        fn_name: "get_latest_mews",
        payload: {
          limit: 5,
        },
      });
      assert.equal(collectionOutput.length, 0);

      // Alice creates a Mew
      const createdRecord: Record = await createMew(alice.cells[0]);
      assert.ok(createdRecord);

      await pause(1200);

      // Bob gets latest mews again
      collectionOutput = await bob.cells[0].callZome({
        zome_name: "mews",
        fn_name: "get_latest_mews",
        payload: {
          limit: 5,
        },
      });
      assert.equal(collectionOutput.length, 1);
      assert.deepEqual(createdRecord, collectionOutput[0]);

      // Alice creates 10 mews
      await createMew(alice.cells[0], { text: "i am a mew 1" });
      await createMew(alice.cells[0], { text: "i am a mew 2" });
      await createMew(alice.cells[0], { text: "i am a mew 3" });
      await createMew(alice.cells[0], { text: "i am a mew 4" });
      const nextBatchLatestMewRecord = await createMew(alice.cells[0], {
        text: "i am a mew 5",
      });
      await createMew(alice.cells[0], { text: "i am a mew 6" });
      await createMew(alice.cells[0], { text: "i am a mew 7" });
      await createMew(alice.cells[0], { text: "i am a mew 8" });
      await createMew(alice.cells[0], { text: "i am a mew 9" });
      const latestMewRecord = await createMew(alice.cells[0], {
        text: "i am a mew 10",
      });

      // Bob gets latest 5 mews
      collectionOutput = await bob.cells[0].callZome({
        zome_name: "mews",
        fn_name: "get_latest_mews",
        payload: {
          limit: 5,
        },
      });
      assert.equal(collectionOutput.length, 5);
      assert.deepEqual(latestMewRecord, collectionOutput[0]);

      // Bob gets latest 5 mews after the oldest previously recieved
      collectionOutput = await bob.cells[0].callZome({
        zome_name: "mews",
        fn_name: "get_latest_mews",
        payload: {
          limit: 5,
          before_mew_hash: collectionOutput[-1].signed_action.hashed.hash,
        },
      });
      assert.equal(collectionOutput.length, 5);
      assert.deepEqual(nextBatchLatestMewRecord, collectionOutput[0]);
    },
    true,
    { timeout: 100000 }
  );
});
