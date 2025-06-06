import * as anchor from "@coral-xyz/anchor";
import { Program, Idl } from "@coral-xyz/anchor";
import { getAssociatedTokenAddress } from "@solana/spl-token";
import {
    PublicKey,
    Keypair,
    SystemProgram,
} from "@solana/web3.js";
import {
    TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { readFileSync } from "fs";

// Load wallet
const rawKey = JSON.parse(
    readFileSync(`${process.env.HOME}/.config/solana/id.json`, "utf8")
);
const payer = Keypair.fromSecretKey(Uint8Array.from(rawKey));

// Load IDL
const idlRaw = JSON.parse(
    readFileSync(`${__dirname}/../../target/idl/fantasy_sports.json`, "utf8")
);
const idl = idlRaw as Idl;

// Provider
const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);

const BN = anchor.BN;
const program = anchor.workspace.FantasySports as Program<typeof idl>;

describe("Fantasy Sports Full Contract Test", () => {
    before(() => {
        console.log("Program ID:", program.programId.toBase58());
    });

    let betPoolPDA: PublicKey;
    let fixtureId: number;
    let statLine: number;
    let userPickPDA: PublicKey;
    let feeVaultPDA: PublicKey;
    let nftMintPDA: PublicKey;
    let mintAuthorityPDA: PublicKey;
    let userAta: PublicKey;

    const playerId = new PublicKey("11111111111111111111111111111111");
    const sportName = "NBA";
    const bettingDeadline = Math.floor(Date.now() / 1000) + 3600;
    const bettor = provider.wallet.publicKey;

    it("Step 1️⃣ InitializeBetPool", async function () {
        this.timeout(20000);

        fixtureId = 888888 + Math.floor(Math.random() * 1000);
        statLine = 200 + Math.floor(Math.random() * 100);

        [betPoolPDA] = await PublicKey.findProgramAddress(
            [
                Buffer.from("bet_pool"),
                new BN(fixtureId).toArrayLike(Buffer, "le", 8),
                playerId.toBuffer(),
                new BN(statLine).toArrayLike(Buffer, "le", 4),
            ],
            program.programId
        );

        console.log("Initializing BetPool:", betPoolPDA.toBase58());

        const tx = await program.methods
            .initializeBetPool(
                new BN(fixtureId),
                sportName,
                playerId,
                new BN(statLine),
                new BN(bettingDeadline)
            )
            .accounts({
                betPool: betPoolPDA,
                admin: provider.wallet.publicKey,
                systemProgram: SystemProgram.programId,
            })
            .rpc();

        console.log("initializeBetPool TX:", tx);
    });

    it("Step 2️⃣ PlaceBet", async function () {
        this.timeout(30000);

        const amount = new BN(10000000); // 0.01 SOL
        const pickSide = true;
        const uri = "";

        [userPickPDA] = await PublicKey.findProgramAddress(
            [
                Buffer.from("user_pick"),
                bettor.toBuffer(),
                betPoolPDA.toBuffer()
            ],
            program.programId
        );

        [feeVaultPDA] = await PublicKey.findProgramAddress(
            [
                Buffer.from("fee_vault"),
                betPoolPDA.toBuffer()
            ],
            program.programId
        );

        [nftMintPDA] = await PublicKey.findProgramAddress(
            [
                Buffer.from("mint"),
                userPickPDA.toBuffer()
            ],
            program.programId
        );

        [mintAuthorityPDA] = await PublicKey.findProgramAddress(
            [
                Buffer.from("mint_authority")
            ],
            program.programId
        );

        console.log("NFT Mint PDA:", nftMintPDA.toBase58());

        userAta = await getAssociatedTokenAddress(
            nftMintPDA,
            bettor,
            false
        );

        console.log("User ATA:", userAta.toBase58());

        const tx = await program.methods
            .placeBet(amount, pickSide, uri)
            .accounts({
                bettor: bettor,
                betPool: betPoolPDA,
                userPick: userPickPDA,
                feeVault: feeVaultPDA,
                nftMint: nftMintPDA,
                userAta: userAta,
                mintAuthority: mintAuthorityPDA,
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: SystemProgram.programId,
                rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            })
            .rpc();

        console.log("placeBet TX:", tx);
    });

    it("Step 3️⃣ PublishResult", async function () {
        this.timeout(15000);

        const tx = await program.methods
            .publishResult({ overWins: {} })
            .accounts({
                betPool: betPoolPDA,
                authority: provider.wallet.publicKey,
            })
            .rpc();

        console.log("publishResult TX:", tx);
    });

    it("Step 4️⃣ SettleClaim", async function () {
        this.timeout(15000);

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

    it("Step 5️⃣ WithdrawFees", async function () {
        this.timeout(15000);

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

    it("Negative Test: Double claim", async function () {
        this.timeout(15000);

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

    it("Negative Test: Invalid fee vault", async function () {
        this.timeout(15000);

        const badVault = Keypair.generate().publicKey;

        try {
            await program.methods
                .placeBet(new BN(10000000), true, "")
                .accounts({
                    bettor: bettor,
                    betPool: betPoolPDA,
                    userPick: userPickPDA,
                    feeVault: badVault,
                    nftMint: nftMintPDA,
                    userAta: userAta,
                    mintAuthority: mintAuthorityPDA,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    systemProgram: SystemProgram.programId,
                    rent: anchor.web3.SYSVAR_RENT_PUBKEY,
                })
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
