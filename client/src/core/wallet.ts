import { PublicKey } from "@solana/web3.js";
import { getProvider } from "./conn";
import { lamportsToSol } from "./utils";
import idl from "../slots.json";
import { Program } from "@project-serum/anchor";

const programID = new PublicKey(idl.metadata.address);

export const checkIfWalletConnected = async () => {
  const { solana } = window;
  return new Promise(async (resolve, reject) => {
    try {
      // setLoading(true);
      if (solana && solana.isPhantom) {
        const response = await solana.connect({
          onlyIfTrusted: true,
        });
        resolve(response.publicKey.toString());
        console.log("public key", response.publicKey.toString());
      } else {
        resolve("");
      }
    } catch (err) {
      console.log("errror");
      resolve("");
    }
  });
};

export const connectWallet = async () => {
  const { solana } = window;
  return new Promise(async (resolve, reject) => {
    try {
      // setLoading(true);
      if (solana) {
        const response = await solana.connect(); //to disconnect use "solana.disconnect()"
        resolve(response.publicKey.toString());
        // setWalletAdresss(response.publicKey.toString());

        // initUserAccount();
      } else {
        alert("Please Install Solana's Phantom Wallet");
        resolve("");
      }
    } catch (err) {
      resolve("");
      console.log(err);
    }
  });
};

export const getBalance = async () => {
  const provider = getProvider();
  //@ts-ignore
  const program = new Program(idl, programID, provider);
  if (!provider.publicKey) {
    return { balance: "", winBalance: "", vaultBalance: "" };
  }

  const [vault] = await PublicKey.findProgramAddressSync(
    [Buffer.from("treasury")],
    program.programId
  );

  let vaultBalance = await provider.connection
    .getBalance(vault)
    .then(function (data) {
      return lamportsToSol(data).toFixed(2);
    })
    .catch(function (error) {
      console.log(error);
      return "";
    });

  let balance = await provider.connection
    .getBalance(provider.publicKey)
    .then(function (data) {
      console.log("Wallet balance: " + lamportsToSol(data).toFixed(2));
      return lamportsToSol(data).toFixed(2);
    })
    .catch(function (error) {
      console.log(error);
      return "";
    });

  let winBalance = "0.00";
  let userVault: PublicKey | undefined;

  try {
    if (provider.wallet && provider.wallet.publicKey) {
      [userVault] = await PublicKey.findProgramAddressSync(
        [Buffer.from("uvault"), provider.wallet.publicKey.toBuffer()],
        program.programId
      );

      if (userVault) {
        const userVaultBalance = await provider.connection.getBalance(userVault);
        winBalance = lamportsToSol(userVaultBalance).toFixed(2);
        console.log("Winning balance: " + winBalance);
      }
    }
  } catch (error) {
    console.error("Error getting user vault:", error);
  }

  return { balance, winBalance, vaultBalance };
};
