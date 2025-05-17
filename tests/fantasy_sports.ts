
import * as anchor from "@coral-xyz/anchor";
import { Program, Idl, AnchorProvider } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import idlJson from "../target/idl/fantasy_sports.json";

import {
  createMint,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountInstruction,
} from "@solana/spl-token";
import { assert } from "chai";

describe("Fantasy Sports Tests (Safe Cast Fix)", () => {
  const provider = AnchorProvider.env();
  anchor.setProvider(provider);
const programId = new PublicKey("Ewc1AMWHGyXZysx8LcRCtYv2UfviYS2DYAtMr5n16cvt");

if (!idlJson || typeof idlJson !== "object") throw new Error("IDL is invalid");
if (!programId) throw new Error("programId is invalid");

// 🛠 PATCH MISSING SIZE INTO IDL
if (Array.isArray(idlJson.accounts)) {
  for (const acc of idlJson.accounts) {
    if (acc.name === "BetPool") (acc as any).size = 256;
    if (acc.name === "UserPick") (acc as any).size = 128;
    // Add fallback to log and stub any missing size
    if ((acc as any).size === undefined) {
      console.warn(`[PATCH] Adding default size=128 for account ${acc.name}`);
      (acc as any).size = 128;
    }
  }
}


const idl = {
  ...(idlJson as any),
  address: programId.toString(), // ✅ Ensure it's a base58 string
} as unknown as Idl;

console.log("programId:", programId.toBase58?.() ?? programId);
console.log("idl address:", (idl as any).address);
console.log("idl.accounts[0]:", (idl as any).accounts?.[0]);

try {
  const program = new Program(idl, programId, provider);
} catch (e) {
  console.error("Program construction failed", e);
}
  const admin = provider.wallet as anchor.Wallet;
  const mintSeed = Buffer.from("mint_authority");
  let mintAuthorityPda: anchor.web3.PublicKey;

  let betPoolPda: anchor.web3.PublicKey;
  let feeVault: anchor.web3.Keypair;

  it("Single-user flow: create, bet, result, claim, withdraw", async () => {
    const fixtureId = new anchor.BN(1234);
    const sportName = "NBA";
    const playerId = anchor.web3.Keypair.generate().publicKey;
    const statLine = 25;
    const deadline = new anchor.BN(Math.floor(Date.now() / 1000) + 60);
    const amount = new anchor.BN(1_000_000);

    [betPoolPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("bet_pool"), fixtureId.toArrayLike(Buffer, "le", 8)],
      program.programId
    );
    [mintAuthorityPda] = anchor.web3.PublicKey.findProgramAddressSync([mintSeed], program.programId);

    feeVault = anchor.web3.Keypair.generate();
    await provider.connection.requestAirdrop(feeVault.publicKey, 2e9);
console.log("betPoolPda", betPoolPda?.toBase58?.());
console.log("admin.publicKey", admin?.publicKey?.toBase58?.());
console.log("systemProgram", anchor.web3.SystemProgram.programId.toBase58());
    await program.methods.initializeBetPool(fixtureId, sportName, playerId, statLine, deadline)
      .accounts({
        betPool: betPoolPda,
        admin: admin.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      }).rpc();

    const nftMint = await createMint(provider.connection, admin.payer, mintAuthorityPda, mintAuthorityPda, 0);
    const userAta = await getAssociatedTokenAddress(nftMint, admin.publicKey);
    const ataIx = createAssociatedTokenAccountInstruction(admin.publicKey, userAta, admin.publicKey, nftMint);
    await provider.sendAndConfirm(new anchor.web3.Transaction().add(ataIx));

    const [userPickPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("user_pick"), admin.publicKey.toBuffer(), betPoolPda.toBuffer()],
      program.programId
    );
console.log("betPoolPda", betPoolPda?.toBase58?.());
console.log("admin.publicKey", admin?.publicKey?.toBase58?.());
console.log("systemProgram", anchor.web3.SystemProgram.programId.toBase58());
    await program.methods.placeBet(amount, true).accounts({
      bettor: admin.publicKey,
      betPool: betPoolPda,
      userPick: userPickPda,
      feeVault: feeVault.publicKey,
      nftMint,
      userAta,
      mintAuthority: mintAuthorityPda,
      metadataAccount: nftMint,
      metadataProgram: anchor.web3.SystemProgram.programId,
      tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    }).rpc();
console.log("betPoolPda", betPoolPda?.toBase58?.());
console.log("admin.publicKey", admin?.publicKey?.toBase58?.());
console.log("systemProgram", anchor.web3.SystemProgram.programId.toBase58());
    await program.methods.publishResult({ overWins: {} }).accounts({
      betPool: betPoolPda,
      authority: admin.publicKey,
    }).rpc();

    const before = await provider.connection.getBalance(admin.publicKey);
    console.log("betPoolPda", betPoolPda?.toBase58?.());
console.log("admin.publicKey", admin?.publicKey?.toBase58?.());
console.log("systemProgram", anchor.web3.SystemProgram.programId.toBase58());
    await program.methods.settleClaim().accounts({
      userPick: userPickPda,
      betPool: betPoolPda,
      recipient: admin.publicKey,
      owner: admin.publicKey,
    }).rpc();
    const after = await provider.connection.getBalance(admin.publicKey);
    assert.isAbove(after, before);

    const withdrawAmount = new anchor.BN(500_000);
    console.log("betPoolPda", betPoolPda?.toBase58?.());
console.log("admin.publicKey", admin?.publicKey?.toBase58?.());
console.log("systemProgram", anchor.web3.SystemProgram.programId.toBase58());
    await program.methods.withdrawFees(withdrawAmount).accounts({
      admin: admin.publicKey,
      feeVault: feeVault.publicKey,
      recipient: admin.publicKey,
    }).signers([feeVault]).rpc();
  });

  it("Multi-user flow: Over vs Under, Under wins", async () => {
    const fixtureId = new anchor.BN(4242);
    const sportName = "NFL";
    const playerId = anchor.web3.Keypair.generate().publicKey;
    const statLine = 100;
    const deadline = new anchor.BN(Math.floor(Date.now() / 1000) + 60);
    const amount = new anchor.BN(1_000_000);

    const bettor2 = anchor.web3.Keypair.generate();
    await provider.connection.requestAirdrop(bettor2.publicKey, 2e9);

    [betPoolPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("bet_pool"), fixtureId.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    [mintAuthorityPda] = anchor.web3.PublicKey.findProgramAddressSync([mintSeed], program.programId);

    feeVault = anchor.web3.Keypair.generate();
    await provider.connection.requestAirdrop(feeVault.publicKey, 2e9);
console.log("betPoolPda", betPoolPda?.toBase58?.());
console.log("admin.publicKey", admin?.publicKey?.toBase58?.());
console.log("systemProgram", anchor.web3.SystemProgram.programId.toBase58());

    await program.methods.initializeBetPool(fixtureId, sportName, playerId, statLine, deadline)
      .accounts({
        betPool: betPoolPda,
        admin: admin.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      }).rpc();

    async function place(user: anchor.Wallet | anchor.web3.Keypair, side: boolean) {
      const payer = "payer" in user ? user.payer : user;

      const mint = await createMint(provider.connection, payer, mintAuthorityPda, mintAuthorityPda, 0);
      const ata = await getAssociatedTokenAddress(mint, user.publicKey);
      const ataIx = createAssociatedTokenAccountInstruction(user.publicKey, ata, user.publicKey, mint);
      await provider.sendAndConfirm(new anchor.web3.Transaction().add(ataIx), "payer" in user ? [] : [user]);

      const [pickPda] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("user_pick"), user.publicKey.toBuffer(), betPoolPda.toBuffer()],
        program.programId
      );
console.log("betPoolPda", betPoolPda?.toBase58?.());
console.log("admin.publicKey", admin?.publicKey?.toBase58?.());
console.log("systemProgram", anchor.web3.SystemProgram.programId.toBase58());
      await program.methods.placeBet(amount, side).accounts({
        bettor: user.publicKey,
        betPool: betPoolPda,
        userPick: pickPda,
        feeVault: feeVault.publicKey,
        nftMint: mint,
        userAta: ata,
        mintAuthority: mintAuthorityPda,
        metadataAccount: mint,
        metadataProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      }).signers("payer" in user ? [] : [user]).rpc();

      return pickPda;
    }

    const pick1 = await place(admin, true);
    const pick2 = await place(bettor2, false);
console.log("betPoolPda", betPoolPda?.toBase58?.());
console.log("admin.publicKey", admin?.publicKey?.toBase58?.());
console.log("systemProgram", anchor.web3.SystemProgram.programId.toBase58());

    await program.methods.publishResult({ underWins: {} }).accounts({
      betPool: betPoolPda,
      authority: admin.publicKey,
    }).rpc();

    const before = await provider.connection.getBalance(bettor2.publicKey);
    console.log("betPoolPda", betPoolPda?.toBase58?.());
console.log("admin.publicKey", admin?.publicKey?.toBase58?.());
console.log("systemProgram", anchor.web3.SystemProgram.programId.toBase58());
    await program.methods.settleClaim().accounts({
      userPick: pick2,
      betPool: betPoolPda,
      recipient: bettor2.publicKey,
      owner: bettor2.publicKey,
    }).signers([bettor2]).rpc();
    const after = await provider.connection.getBalance(bettor2.publicKey);
    assert.isAbove(after, before);

    try {
      console.log("betPoolPda", betPoolPda?.toBase58?.());
console.log("admin.publicKey", admin?.publicKey?.toBase58?.());
console.log("systemProgram", anchor.web3.SystemProgram.programId.toBase58());
      await program.methods.settleClaim().accounts({
        userPick: pick1,
        betPool: betPoolPda,
        recipient: admin.publicKey,
        owner: admin.publicKey,
      }).rpc();
      throw new Error("Over bettor should not receive payout");
    } catch (err) {
  if (err instanceof Error) {
    assert.include(err.message, "This pick has already been claimed");
  } else {
    throw err; // re-throw if it's not an Error instance
  }
}
  });
});
