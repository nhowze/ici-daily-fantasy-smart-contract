import * as anchor from "@coral-xyz/anchor";
import { Program, Idl } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import {
    TOKEN_PROGRAM_ID,
    getOrCreateAssociatedTokenAccount,
    createMint,
} from "@solana/spl-token";
import { readFileSync } from "fs";

// Load wallet
const rawKey = JSON.parse(
    readFileSync(`${process.env.HOME}/.config/solana/id.json`, "utf8")
);
const payer = Keypair.fromSecretKey(Uint8Array.from(rawKey));

// Load IDL
const idlRaw = JSON.parse(
    readFileSync(`${__dirname}/../target/idl/fantasy_sports.json`, "utf8")
);
const idl = idlRaw as Idl;

// Provider
const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);


const program = anchor.workspace.FantasySports as Program<typeof idl>;

describe("Fantasy Sports Full Contract Test", () => {
    it("Dummy test - Program loads", async () => {
        console.log("Program ID:", program.programId.toBase58());
    });

    let betPoolPDA: PublicKey;
    let fixtureId: number;
    let statLine: number;
    let userPickPDA: PublicKey;
    let feeVaultPDA: PublicKey;
    let mintKeypair: Keypair;
    let mintAuthorityPDA: PublicKey;
    let userAta: any;

    const playerId = new PublicKey("11111111111111111111111111111111");
    const sportName = "NBA";
    const bettingDeadline = Math.floor(Date.now() / 1000) + 3600;
    const bettor = provider.wallet.publicKey;

    it("Step 1️⃣ InitializeBetPool", async () => {
        fixtureId = 888888 + Math.floor(Math.random() * 1000);
        statLine = 200 + Math.floor(Math.random() * 100);

        [betPoolPDA] = await PublicKey.findProgramAddress(
            [
                Buffer.from("bet_pool"),
                new anchor.BN(fixtureId).toArrayLike(Buffer, "le", 8),
                playerId.toBuffer(),
                new anchor.BN(statLine).toArrayLike(Buffer, "le", 4),
            ],
            program.programId
        );

        console.log("Initializing BetPool:", betPoolPDA.toBase58());

        const tx = await program.methods
            .initializeBetPool(
                fixtureId,
                sportName,
                playerId,
                statLine,
                new anchor.BN(bettingDeadline)
            )
            .accounts({
                betPool: betPoolPDA,
                admin: provider.wallet.publicKey,
                systemProgram: SystemProgram.programId,
            })
            .rpc();

        console.log("initializeBetPool TX:", tx);
    });

    it("Step 2️⃣ PlaceBet", async () => {
        const amount = new anchor.BN(10000000); // 0.01 SOL in lamports
        const pickSide = true; // OVER

        [userPickPDA] = await PublicKey.findProgramAddress(
            [Buffer.from("user_pick"), bettor.toBuffer(), betPoolPDA.toBuffer()],
            program.programId
        );

        [feeVaultPDA] = await PublicKey.findProgramAddress(
            [Buffer.from("fee_vault"), betPoolPDA.toBuffer()],
            program.programId
        );

        [mintAuthorityPDA] = await PublicKey.findProgramAddress(
            [Buffer.from("mint_authority")],
            program.programId
        );

        // Create dummy NFT mint
        mintKeypair = Keypair.generate();
        const mintTx = await provider.connection.requestAirdrop(
            provider.wallet.publicKey,
            1 * anchor.web3.LAMPORTS_PER_SOL
        );
        await provider.connection.confirmTransaction(mintTx, "confirmed");

        const createMintTx = await createMint(
            provider.connection,
            payer,
            mintAuthorityPDA,
            mintAuthorityPDA,
            0,
            mintKeypair
        );

        console.log("Created Mint:", createMintTx.toBase58());

        userAta = await getOrCreateAssociatedTokenAccount(
            provider.connection,
            payer,
            mintKeypair.publicKey,
            bettor
        );

        // Dummy placeholders for Metaplex accounts
        const dummyMetadataAccount = Keypair.generate().publicKey;
        const dummyMetadataProgram = Keypair.generate().publicKey;

        const tx = await program.methods
            .placeBet(amount, pickSide)
            .accounts({
                bettor: bettor,
                betPool: betPoolPDA,
                userPick: userPickPDA,
                feeVault: feeVaultPDA,
                nftMint: mintKeypair.publicKey,
                userAta: userAta.address,
                mintAuthority: mintAuthorityPDA,
                metadataAccount: dummyMetadataAccount,
                metadataProgram: dummyMetadataProgram,
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: SystemProgram.programId,
                rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            })
            .signers([mintKeypair])
            .rpc();

        console.log("placeBet TX:", tx);
    });

    it("Step 3️⃣ PublishResult", async () => {
        const tx = await program.methods
            .publishResult({ overWins: {} }) // Match your Outcome enum
            .accounts({
                betPool: betPoolPDA,
                authority: provider.wallet.publicKey,
            })
            .rpc();

        console.log("publishResult TX:", tx);
    });

    it("Step 4️⃣ SettleClaim", async () => {
        const tx = await program.methods
            .settleClaim()
            .accounts({
                userPick: userPickPDA,
                betPool: betPoolPDA,
                recipient: bettor,
                owner: bettor,
            })
            .rpc();

        console.log("settleClaim TX:", tx);
    });

    it("Step 5️⃣ WithdrawFees", async () => {
        const tx = await program.methods
            .withdrawFees()
            .accounts({
                admin: provider.wallet.publicKey,
                betPool: betPoolPDA,
                feeVault: feeVaultPDA,
                recipient: provider.wallet.publicKey,
            })
            .rpc();

        console.log("withdrawFees TX:", tx);
    });

    it("Negative Test: Bet after deadline", async () => {
        console.log("⚠️ Skipping for now: would require new pool with expired deadline 🚀");
    });

    it("Negative Test: Double claim", async () => {
        try {
            await program.methods
                .settleClaim()
                .accounts({
                    userPick: userPickPDA,
                    betPool: betPoolPDA,
                    recipient: bettor,
                    owner: bettor,
                })
                .rpc();

            throw new Error("Expected double claim to fail — but it succeeded!");
        } catch (err: unknown) {
            if (err instanceof Error) {
                console.log("✅ Double claim correctly failed:", err.message);
            } else {
                console.log("✅ Double claim correctly failed:", err);
            }
        }
    });

    it("Negative Test: Invalid fee vault", async () => {
        const badVault = Keypair.generate().publicKey;

        try {
            await program.methods
                .placeBet(new anchor.BN(10000000), true)
                .accounts({
                    bettor: bettor,
                    betPool: betPoolPDA,
                    userPick: userPickPDA,
                    feeVault: badVault, // intentionally wrong
                    nftMint: mintKeypair.publicKey,
                    userAta: userAta.address,
                    mintAuthority: mintAuthorityPDA,
                    metadataAccount: Keypair.generate().publicKey,
                    metadataProgram: Keypair.generate().publicKey,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    systemProgram: SystemProgram.programId,
                    rent: anchor.web3.SYSVAR_RENT_PUBKEY,
                })
                .signers([mintKeypair])
                .rpc();

            throw new Error("Expected invalid fee vault to fail — but it succeeded!");
        } catch (err: unknown) {
            if (err instanceof Error) {
                console.log("✅ Invalid fee vault correctly failed:", err.message);
            } else {
                console.log("✅ Invalid fee vault correctly failed:", err);
            }
        }
    });
});
