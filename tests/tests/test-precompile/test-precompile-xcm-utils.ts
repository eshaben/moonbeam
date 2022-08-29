import "@moonbeam-network/api-augment";

import { u8aToHex } from "@polkadot/util";
import { expect } from "chai";
import { ethers } from "ethers";
import { GLMR, PRECOMPILE_XCM_UTILS_ADDRESS } from "../../util/constants";
import { getCompiled } from "../../util/contracts";
import { web3EthCall } from "../../util/providers";
import { describeDevMoonbeamAllEthTxTypes } from "../../util/setup-dev-tests";
import { generateKeyringPair } from "../../util/accounts";
import { BN } from "@polkadot/util";
import type { XcmVersionedXcm } from "@polkadot/types/lookup";
import { descendOriginFromAddress } from "../../util/xcm";
import {
  ALITH_TRANSACTION_TEMPLATE,
  createTransaction,
} from "../../util/transactions";

const XCM_UTILS_CONTRACT = getCompiled("XcmUtils");
const XCM_UTILSTRANSACTOR_INTERFACE = new ethers.utils.Interface(XCM_UTILS_CONTRACT.contract.abi);

describeDevMoonbeamAllEthTxTypes("Precompiles - xcm utils", (context) => {
  it("allows to retrieve parent-based ML account", async function () {
    const multilocation: [number, {}[]] =
      // Destination as multilocation
      [
        // one parent
        1,
        [],
      ];
    const result = await web3EthCall(context.web3, {
      to: PRECOMPILE_XCM_UTILS_ADDRESS,
      data: XCM_UTILSTRANSACTOR_INTERFACE.encodeFunctionData("multilocationToAddress", [
        multilocation,
      ]),
    });

    const expectedAddress = u8aToHex(new Uint8Array([...new TextEncoder().encode("Parent")]))
      .slice(2)
      .padEnd(40, "0");

    expect(result.result).to.equal(`0x${expectedAddress.padStart(64, "0")}`);
  });

  it("allows to retrieve parent-based ML account", async function () {
    const multilocation: [number, {}[]] =
      // Destination as multilocation
      [
        // one parent
        1,
        [],
      ];
    const result = await web3EthCall(context.web3, {
      to: PRECOMPILE_XCM_UTILS_ADDRESS,
      data: XCM_UTILSTRANSACTOR_INTERFACE.encodeFunctionData("multilocationToAddress", [
        multilocation,
      ]),
    });

    const expectedAddress = u8aToHex(new Uint8Array([...new TextEncoder().encode("Parent")]))
      .slice(2)
      .padEnd(40, "0");

    expect(result.result).to.equal(`0x${expectedAddress.padStart(64, "0")}`);
  });

  it("allows to retrieve parachain-based ML account", async function () {
    const x2_parachain_asset_enum_selector = "0x00";
    const x2_parachain_id = "000007D0";
    const paraId = context.polkadotApi.createType("ParaId", 2000);

    const multilocation: [number, {}[]] =
      // Destination as multilocation
      [
        // one parent
        1,
        // Parachain(2000)
        [x2_parachain_asset_enum_selector + x2_parachain_id],
      ];

    const result = await web3EthCall(context.web3, {
      to: PRECOMPILE_XCM_UTILS_ADDRESS,
      data: XCM_UTILSTRANSACTOR_INTERFACE.encodeFunctionData("multilocationToAddress", [
        multilocation,
      ]),
    });

    const expectedAddress = u8aToHex(
      new Uint8Array([...new TextEncoder().encode("sibl"), ...paraId.toU8a()])
    )
      .slice(2)
      .padEnd(40, "0");

    expect(result.result).to.equal(`0x${expectedAddress.padStart(64, "0")}`);
  });

  it("allows to retrieve generic ML-based derivated account", async function () {
    const x2_parachain_asset_enum_selector = "0x00";
    const x2_parachain_id = "00000001";

    // Junction::AccountKey20
    const account20EnumSelector = "0x03";
    // [0x01; 20]
    const account20Address = "0101010101010101010101010101010101010101";
    // NetworkId::Any
    const account20NetworkId = "00";

    const multilocation: [number, {}[]] =
      // Destination as multilocation
      [
        // one parent
        1,
        // X2(Parachain(2000), AccountId32(account32Address))
        [
          x2_parachain_asset_enum_selector + x2_parachain_id,
          account20EnumSelector + account20Address + account20NetworkId,
        ],
      ];

    const result = await web3EthCall(context.web3, {
      to: PRECOMPILE_XCM_UTILS_ADDRESS,
      data: XCM_UTILSTRANSACTOR_INTERFACE.encodeFunctionData("multilocationToAddress", [
        multilocation,
      ]),
    });

    const { originAddress, descendOriginAddress } = descendOriginFromAddress(context);
    expect(result.result).to.equal(`0x${descendOriginAddress.slice(2).padStart(64, "0")}`);
  });
});

describeDevMoonbeamAllEthTxTypes("Precompiles - xcm utils", (context) => {
  it("allows to execute a custom xcm message", async function () {
    let random = generateKeyringPair();

    const transferCall = context.polkadotApi.tx.balances.transfer(
      random.address,
      1n * GLMR
    );
    const transferCallEncoded = transferCall?.method.toHex();
    
    const xcmMessage = {
      V2: [
        {
          Transact: {
            originType: "SovereignAccount",
            requireWeightAtMost: new BN(525_000_000), // 21_000 gas limit
            call: {
              encoded: transferCallEncoded,
            },
          },
        }
      ],
    };

    const receivedMessage: XcmVersionedXcm = context.polkadotApi.createType(
      "XcmVersionedXcm",
      xcmMessage
    ) as any;

    await context.createBlock(
      createTransaction(context, {
        ...ALITH_TRANSACTION_TEMPLATE,
        to: PRECOMPILE_XCM_UTILS_ADDRESS,
        data:  XCM_UTILSTRANSACTOR_INTERFACE.encodeFunctionData("execute", [
          receivedMessage.toU8a(),
          2_000_000_000
        ]),
      })
    );

     // Tokens transferred
     const testAccountBalance = (
      await context.polkadotApi.query.system.account(random.address)
    ).data.free.toBigInt();

    expect(testAccountBalance).to.eq(1n * GLMR);
  });
});

describeDevMoonbeamAllEthTxTypes("Precompiles - xcm utils", (context) => {
  it("allows to execute a custom xcm message evm and evvm", async function () {
    let random = generateKeyringPair();

    const ethTx =  {
      V1: {
        gas_limit: 21000,
        fee_payment: {
          Auto: {
            Low: null,
          },
        },
        action: {
          Call: random.address,
        },
        value: 1n * GLMR,
        input: [],
        access_list: null,
      }
    };
    const transferCall = context.polkadotApi.tx.ethereumXcm.transact(ethTx as any);
    const transferCallEncoded = transferCall?.method.toHex();
    
    const xcmMessage = {
      V2: [
        {
          Transact: {
            originType: "SovereignAccount",
            requireWeightAtMost: new BN(525_000_000), // 21_000 gas limit
            call: {
              encoded: transferCallEncoded,
            },
          },
        }
      ],
    };

    const receivedMessage: XcmVersionedXcm = context.polkadotApi.createType(
      "XcmVersionedXcm",
      xcmMessage
    ) as any;

    await context.createBlock(
      createTransaction(context, {
        ...ALITH_TRANSACTION_TEMPLATE,
        to: PRECOMPILE_XCM_UTILS_ADDRESS,
        data:  XCM_UTILSTRANSACTOR_INTERFACE.encodeFunctionData("execute", [
          receivedMessage.toU8a(),
          2_000_000_000
        ]),
      })
    );

     // Tokens transferred
     const testAccountBalance = (
      await context.polkadotApi.query.system.account(random.address)
    ).data.free.toBigInt();

    expect(testAccountBalance).to.eq(1n * GLMR);
  });
});