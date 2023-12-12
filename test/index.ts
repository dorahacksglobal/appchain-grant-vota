import { Secp256k1HdWallet, SigningCosmosClient } from "@cosmjs/launchpad";
import {
  DirectSecp256k1HdWallet,
  OfflineDirectSigner,
} from "@cosmjs/proto-signing";
import {
  StargateClient,
  SigningStargateClient,
  IndexedTx,
  GasPrice,
} from "@cosmjs/stargate";
import { MsgSend } from "cosmjs-types/cosmos/bank/v1beta1/tx";
import { Tx } from "cosmjs-types/cosmos/tx/v1beta1/tx";
import {
  AppchainGrantVotaClient,
} from "../ts/AppchainGrantVota.client";
import {
  CosmWasmClient,
  SigningCosmWasmClient,
} from "@cosmjs/cosmwasm-stargate";
import { Decimal } from "@cosmjs/math";
import dotenv from "dotenv";
import fs from 'fs';
import * as base64js from "base64-js";

dotenv.config();

const is_mainnet = process.env.MAINNET === 'true';
const rpcEndpoint = is_mainnet ?
  "https://vota-rpc.dorafactory.org"
  : "https://vota-testnet-rpc.dorafactory.org";
const mnemonic = process.env.MNEMONIC as string;
const chain_id = is_mainnet ? "vota-ash" : "cvota-testnet";
const prefix = "dora";
const denom = is_mainnet ? "peaka" : 'peaka';
const decimals = 18;

let admin_address: string;

const check_balance = async (client: StargateClient, address: string) => {
  // 查询余额
  console.log(
    "With client, chain id:",
    await client.getChainId(),
    ", height:",
    await client.getHeight()
  );
  const res = await client.getAllBalances(address);
  console.log("Admin balances:", res);
  return res
}

const send_coins = async (signingClient: SigningStargateClient, fromAddress: string, toAddress: string) => {
  // 发送币
  const msg: MsgSend = {
    fromAddress,
    toAddress,
    amount: [
      {
        denom: denom,
        amount: "100000",
      }]
  };
  const fee = {
    amount: [
      {
        denom: denom,
        amount: "100000",
      },
    ],
    gas: "200000",
  };
  const memo = "test send coin";
  
  const result = await signingClient.sendTokens(fromAddress, toAddress, [
    {
      denom: denom,
      amount: "100000",
    }], fee, memo);
  // .signAndBroadcast(fromAddress, [msg], fee, memo);
}

// 上传合约
const upload_contract = async (signingCosmWasmClient: SigningCosmWasmClient) => {
  const wasmCode = fs.readFileSync('./artifacts/appchain_grant_vota.wasm');
  const encoded = Buffer.from(wasmCode).toString('base64');
  const contractData = base64js.toByteArray(encoded);
  const uploadResult = await signingCosmWasmClient.upload(
    admin_address,
    contractData,
    'auto',
    '',
  );
  console.log("Storage Contract:", uploadResult);
  return uploadResult
};

// 初始化合约
const init_contract = async (signingCosmWasmClient: SigningCosmWasmClient, codeId: number) => {
  const instantiateOptions = {
    memo: "QuadraticGrantTestInstance",
    funds: [],
    admin: admin_address
  };

  const instantiateResult = await signingCosmWasmClient.instantiate(
    admin_address,
    codeId,
    {
      admins: ["dora1kw5qfnrxk9sw5gcyk3emktwtca94e5a4dau8y3", "dora1pntxsj79xkjm9q096fj9ry9wvtexmtk6ms6fag", "dora1apfd8sm69x9prca2rranp32pdagh9s9um2fplu"],
    },
    'QuadraticGrantTestInstance',
    'auto',
    instantiateOptions
  );
  console.log("instantiateResult:", instantiateResult);
  return instantiateResult
}

const vote = async (contract: AppchainGrantVotaClient, projectId: number, amount: string) => {
  // 投票
  const res = await contract.batchVote({ amounts: [amount], projectIds: [projectId] }, 'auto', '', [
    {
      denom: denom,
      amount: amount,
    }
  ])
  console.log("batchVote result:", res)
  console.log(res.events[res.events.length - 4])
  return res
}

const end_round = async (contract: AppchainGrantVotaClient) => {
  // 结束轮次
  const res = await contract.endRound()
  console.log("endRound result:", res)
  console.log(res.events[res.events.length - 1])
  return res
}

(async () => {
  const wallet = await Secp256k1HdWallet.fromMnemonic(mnemonic, {
    prefix,
  });

  const [{ address: address }] = await wallet.getAccounts();
  admin_address = address;
  console.log("Address:", address);

  const client = await StargateClient.connect(rpcEndpoint);
  const signingClient = await SigningStargateClient.connectWithSigner(
    rpcEndpoint,
    wallet,
    {
      gasPrice: new GasPrice(Decimal.fromUserInput('100000000000', 2), denom)
    }
  );

  const cosmWasmClient = await CosmWasmClient.connect(rpcEndpoint);
  const signingCosmWasmClient = await SigningCosmWasmClient.connectWithSigner(
    rpcEndpoint,
    wallet,
    {
      gasPrice: new GasPrice(Decimal.fromUserInput('100000000000', 2), denom)
    }
  );

  await check_balance(client, address);

  let codeId = parseInt(process.env.CODEID || ''); // 如果已经上传过了，可以直接使用已经上传的合约
  let contract_address = process.env.CONTRACT_ADDRESS;
  if (!codeId) {
    let res = await upload_contract(signingCosmWasmClient);
    codeId = res.codeId;
  }
  if (!contract_address) {
    let res = await init_contract(signingCosmWasmClient, codeId);
    contract_address = res.contractAddress;
  }

  console.log("codeId:", codeId);
  console.log("contract_address:", contract_address);
  const contract = new AppchainGrantVotaClient(
    signingCosmWasmClient,
    address,
    contract_address
  );

  // contract.project({projectId:1}).then((res) => {
  //   console.log("project:", res);
  // })
  await contract.setBeneficiary({ address: 'dora1apfd8sm69x9prca2rranp32pdagh9s9um2fplu' }).then((res) => console.log("setBeneficiary:", res));
  // contract.roundId().then((res) => console.log("roundId:", res));
  // await vote(contract, 1, '4' + '0'.repeat(decimals - 1));
  // contract.projectVoter({ projectId: 1, roundId: 1, voter: address }).then((res) => console.log("projectVoter:", res));
  // await vote(contract, 1, '1' + '0'.repeat(decimals - 1));
  // let res = await end_round(contract);
  // console.log(res)
  // contract.projectVoter({ projectId: 1, roundId: 1, voter: address }).then((res) => console.log("projectVoter:", res));
  // contract.project({ projectId: 1, roundId: 2 }).then((res) => console.log("project:", res));
})();

