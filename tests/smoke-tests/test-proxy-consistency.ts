import "@moonbeam-network/api-augment";
import { ApiDecoration } from "@polkadot/api/types";
import chalk from "chalk";
import { expect } from "chai";
import { printTokens } from "../util/logging";
import { describeSmokeSuite } from "../util/setup-smoke-tests";

// TEMPLATE: Remove useless types at the end
import type { PalletProxyProxyDefinition } from "@polkadot/types/lookup";

// TEMPLATE: Replace debug name
const debug = require("debug")("smoke:proxy");

const wssUrl = process.env.WSS_URL || null;
const relayWssUrl = process.env.RELAY_WSS_URL || null;

// TEMPLATE: Give suitable name
describeSmokeSuite(`Verify number of proxies per account`, { wssUrl, relayWssUrl }, (context) => {
  // TEMPLATE: Declare variables representing the state to inspect
  //           To know the type of the variable, type the query and the highlight
  //           it to see
  //           Ex: context.polkadotApi.query.proxy.proxies.entries()
  //             Displays PalletProxyProxyDefinition
  //           Then add the type in the import from "@polkadot/types/lookup"
  const proxiesPerAccount: { [account: string]: PalletProxyProxyDefinition[] } = {};

  let atBlockNumber: number = 0;
  let apiAt: ApiDecoration<"promise"> = null;

  // If the state structure has changed at a specific version, it should get included in the test
  let specVersion: number = 0;

  // TEMPLATE: Describe the data you are retrieving
  before("Retrieve all proxies", async function () {
    // It takes time to load all the proxies.
    // TEMPLATE: Adapt the timeout to be an over-estimate
    this.timeout(30_000); // 30s

    // How many entries to query at a time
    const limit = 1000;

    // Last key to query in a loop
    let last_key = "";

    // current number of queried items
    let count = 0;

    // Configure the api at a specific block
    // (to avoid inconsistency querying over multiple block when the test takes a long time to
    // query data and blocks are being produced)
    atBlockNumber = process.env.BLOCK_NUMBER
      ? parseInt(process.env.BLOCK_NUMBER)
      : (await context.polkadotApi.rpc.chain.getHeader()).number.toNumber();
    apiAt = await context.polkadotApi.at(
      await context.polkadotApi.rpc.chain.getBlockHash(atBlockNumber)
    );
    specVersion = (await apiAt.query.system.lastRuntimeUpgrade()).unwrap().specVersion.toNumber();

    // TEMPLATE: query the data
    while (true) {
      let query = await apiAt.query.proxy.proxies.entriesPaged({
        args: [],
        pageSize: limit,
        startKey: last_key,
      });

      if (query.length == 0) {
        break;
      }
      count += query.length;

      // TEMPLATE: convert the data into the format you want (usually a dictionary per account)
      for (const proxyData of query) {
        let accountId = `0x${proxyData[0].toHex().slice(-40)}`;
        last_key = proxyData[0].toString();
        proxiesPerAccount[accountId] = proxyData[1][0].toArray();
      }

      // Debug logs to make sure it keeps progressing
      // TEMPLATE: Adapt log line
      if (count % (10 * limit) == 0) {
        debug(`Retrieved ${count} proxies`);
      }
    }

    // TEMPLATE: Adapt proxies
    debug(`Retrieved ${count} total proxies`);
  });

  // TEMPLATE: Give details about what you are testing
  it("should have no more than the maximum allowed proxies", async function () {
    this.timeout(240000);

    // TEMPLATE: Retrieve additional information
    const maxProxies = (await context.polkadotApi.consts.proxy.maxProxies).toNumber();

    // Instead of putting an expect in the loop. We track all failed entries instead
    const failedProxies: { accountId: string; proxiesCount: number }[] = [];

    // TEMPLATE: Adapt variables
    for (const accountId of Object.keys(proxiesPerAccount)) {
      const proxiesCount = proxiesPerAccount[accountId].length;
      if (proxiesCount > maxProxies) {
        failedProxies.push({ accountId, proxiesCount });
      }
    }

    // TEMPLATE: Write nice logging for your test if it fails :)
    console.log("Failed accounts with too many proxies:");
    console.log(
      failedProxies
        .map(({ accountId, proxiesCount }) => {
          return `accountId: ${accountId} - ${chalk.red(
            proxiesCount.toString().padStart(4, " ")
          )} proxies (expected max: ${maxProxies})`;
        })
        .join(`\n`)
    );

    // Make sure the test fails after we print the errors
    // TEMPLATE: Adapt variable & text
    expect(failedProxies.length, "Failed max proxies").to.equal(0);

    // Additional debug logs
    debug(
      `Verified ${Object.keys(proxiesPerAccount).length} total accounts (at #${atBlockNumber})`
    );
  });

  // TEMPLATE: Exemple of test verifying a constant in the runtime
  it("should have a maximum allowed proxies of 32", async function () {
    // TEMPLATE: Remove if the value is the same for each runtime
    const runtimeName = context.polkadotApi.runtimeVersion.specName.toString();
    const networkName = context.polkadotApi.runtimeChain.toString();

    // TEMPLATE: Retrieve additional information
    const maxProxies = (await context.polkadotApi.consts.proxy.maxProxies).toNumber();

    switch (runtimeName) {
      case "moonbase":
        expect(maxProxies).to.equal(32);
        break;
      case "moonriver":
        expect(maxProxies).to.equal(32);
        break;
      case "moonbeam":
        expect(maxProxies).to.equal(32);
        break;
      default:
        expect(maxProxies).to.equal(32);
        break;
    }

    // TEMPLATE: This is redundant but is used to show how to check based on the network
    switch (networkName) {
      case "Moonbase Alpha":
        expect(maxProxies).to.equal(32);
        break;
      case "Moonbeam":
        expect(maxProxies).to.equal(32);
        break;
      default:
        expect(maxProxies).to.equal(32);
        break;
    }

    // TEMPLATE: Updates the log line
    debug(`Verified maximum allowed proxies constant`);
  });
});

// TEMPLATE: Running the smoke test on stagenet
//
// DEBUG=test*,smoke* \
// WSS_URL=wss://wss.api.moondev.network \
// RELAY_WSS_URL=wss://frag-stagenet-relay-rpc-ws.g.moondev.network \
// ./node_modules/.bin/mocha -r ts-node/register smoke-tests/test-proxy-consistency.ts
//
// Running all tests
// DEBUG=test*,smoke* \
// WSS_URL=wss://wss.api.moondev.network \
// RELAY_WSS_URL=wss://frag-stagenet-relay-rpc-ws.g.moondev.network \
// npm run smoke-test
