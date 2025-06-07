import * as anchor from "@coral-xyz/anchor";
import * as fs from "fs";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import {
    TOKEN_PROGRAM_ID,
    getAccount,
    getAssociatedTokenAddress,
    getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";

const adminKeypair = Keypair.fromSecretKey(
    new Uint8Array(
        JSON.parse(fs.readFileSync(process.env.ANCHOR_WALLET!, "utf-8"))
    )
);

describe("Fantasy Sports Full Contract Test", function () {
    this.timeout(30000);  // Increase timeout — Devnet slow sometimes!

    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.FantasySports;

    const admin = provider.wallet;
    const bettor = Keypair.generate();

    const fixtureId = new anchor.BN(12345);
    const sportName = "NFL";
    const playerId = Keypair.generate().publicKey;
    const statLine = 200;
    const bettingDeadline = new anchor.BN(Date.now() / 1000 + 3600);
    const betAmount = new anchor.BN(1_000_000_000);

    let betPoolPda: PublicKey;
    let userPickPda: PublicKey;
    let feeVaultPda: PublicKey;
    let nftMintPda: PublicKey;
    let userAta: PublicKey;

    it("Step 1️⃣ Initialize Bet Pool", async () => {
        [betPoolPda] = await PublicKey.findProgramAddress(
            [
                Buffer.from("bet_pool"),
                fixtureId.toArrayLike(Buffer, "le", 8),
                playerId.toBuffer(),
                Buffer.from(Uint8Array.of(...new anchor.BN(statLine).toArray("le", 4))),
            ],
            program.programId
        );

        console.log("📍 Bet Pool PDA:", betPoolPda.toBase58());

        await program.methods
            .initializeBetPool(
                fixtureId,
                sportName,
                playerId,
                statLine,
                bettingDeadline
            )
            .accounts({
                betPool: betPoolPda,
                admin: admin.publicKey,
                systemProgram: SystemProgram.programId,
            })
            .rpc();

        console.log("✅ Bet Pool initialized");
    });

    it("Step 2️⃣ Place Bet (mint NFT)", async () => {
        // Fund bettor with more SOL (5 SOL safe)
        const transferTx = new anchor.web3.Transaction().add(
            anchor.web3.SystemProgram.transfer({
                fromPubkey: admin.publicKey,
                toPubkey: bettor.publicKey,
                lamports: 10_000_000_000,
            })
        );
        await provider.sendAndConfirm(transferTx);
        console.log("💰 Bettor funded via manual transfer:", bettor.publicKey.toBase58());

        // Derive PDAs
        [userPickPda] = await PublicKey.findProgramAddress(
            [
                Buffer.from("user_pick"),
                bettor.publicKey.toBuffer(),
                betPoolPda.toBuffer(),
            ],
            program.programId
        );
        console.log("🎟️ User Pick PDA:", userPickPda.toBase58());

        [feeVaultPda] = await PublicKey.findProgramAddress(
            [Buffer.from("fee_vault"), betPoolPda.toBuffer()],
            program.programId
        );
        console.log("💰 Fee Vault PDA:", feeVaultPda.toBase58());

        [nftMintPda] = await PublicKey.findProgramAddress(
            [Buffer.from("mint"), userPickPda.toBuffer()],
            program.programId
        );
        console.log("🪙 NFT Mint PDA:", nftMintPda.toBase58());

        // PRE-CREATE user ATA → required for Anchor 0.31.1
        userAta = await getAssociatedTokenAddress(nftMintPda, bettor.publicKey);

        await getOrCreateAssociatedTokenAccount(
            provider.connection,
            adminKeypair,    // payer (admin funds the creation)
            nftMintPda,
            bettor.publicKey
        );

        console.log("🎁 User ATA:", userAta.toBase58());

        // Call placeBet
        await program.methods
            .placeBet(betAmount, true, "https://dummy.uri/metadata.json")
            .accounts({
                bettor: bettor.publicKey,
                betPool: betPoolPda,
                userPick: userPickPda,
                feeVault: feeVaultPda,
                nftMint: nftMintPda,
                userAta: userAta,
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: SystemProgram.programId,
                rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            })
            .signers([bettor])
            .rpc();

        console.log("✅ Bet placed + NFT minted");

        // Wait a bit to ensure consistency
        await new Promise(resolve => setTimeout(resolve, 500));

        // Verify ATA now exists and holds the NFT
        const nftAccount = await getAccount(provider.connection, userAta);
        console.log("NFT token amount:", Number(nftAccount.amount));
        if (Number(nftAccount.amount) !== 1) {
            throw new Error("NFT was not minted correctly!");
        }
    });

    it("Step 3️⃣ Publish Result", async () => {
        await program.methods
            .publishResult({ overWins: {} })
            .accounts({
                betPool: betPoolPda,
                authority: admin.publicKey,
            })
            .rpc();

        console.log("✅ Result published: OverWins");
    });

    it("Step 4️⃣ Settle Claim", async () => {
        const bettorProvider = new anchor.AnchorProvider(
            provider.connection,
            new anchor.Wallet(bettor),
            provider.opts
        );

        // Switch to bettor provider
        anchor.setProvider(bettorProvider);

        // ⚠️ Force workspace client to use new provider!
        (program as any).provider = bettorProvider;

        await program.methods
            .settleClaim()
            .accounts({
                userPick: userPickPda,
                betPool: betPoolPda,
                recipient: bettor.publicKey,
                owner: bettor.publicKey,
            })
            .signers([bettor])
            .rpc();

        console.log("✅ Claim settled and payout sent to bettor");

        // Switch back to admin provider
        anchor.setProvider(provider);
        (program as any).provider = provider;
    });

    it("Step 5️⃣ Withdraw Fees", async () => {
        await program.methods
            .withdrawFees()
            .accounts({
                admin: admin.publicKey,
                betPool: betPoolPda,
                feeVault: feeVaultPda,
                recipient: admin.publicKey,
            })
            .rpc();

        console.log("✅ Fees withdrawn to admin");
    });
});
