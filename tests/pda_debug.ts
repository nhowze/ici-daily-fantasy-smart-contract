import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { Program } from "@coral-xyz/anchor";
import { readFileSync } from "fs";
const idlRaw = JSON.parse(readFileSync("./target/idl/fantasy_sports.json", "utf-8"));
const idl = idlRaw as anchor.Idl;


(async () => {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

const program = anchor.workspace.FantasySports as Program<typeof idl>;

    it("Dummy test to confirm program loads", async () => {
        console.log("Program ID:", program.programId.toBase58());
    });


    console.log("🔍 Scanning BetPool PDAs...");

    const betPoolAccounts = await provider.connection.getProgramAccounts(
        program.programId,
        {
            filters: [
                { dataSize: 8 + 8 + 32 + 32 + 4 + 8 + 8 + 8 + 8 + 1 + 1 + 32 + 1 }, // adjust if you changed size
            ],
        }
    );

    console.log(`Found ${betPoolAccounts.length} BetPool PDAs:`);
    betPoolAccounts.forEach((acc, idx) => {
        console.log(`BetPool ${idx + 1}: ${acc.pubkey.toBase58()}`);
    });

    console.log("\n🔍 Scanning UserPick PDAs...");

    const userPickAccounts = await provider.connection.getProgramAccounts(
        program.programId,
        {
            filters: [
                { dataSize: 8 + 32 + 8 + 1 + 32 + 1 + 32 + 1 }, // adjust if needed
            ],
        }
    );

    console.log(`Found ${userPickAccounts.length} UserPick PDAs:`);
    userPickAccounts.forEach((acc, idx) => {
        console.log(`UserPick ${idx + 1}: ${acc.pubkey.toBase58()}`);
    });

    console.log("\n🔍 Scanning FeeVault PDAs (Derived Only)...");

    console.log("⚠️ FeeVault is UncheckedAccount → not directly searchable by size.");
    console.log("Use PDA derivation to check specific vaults (example below).");

    // Example: Derive FeeVault for known BetPool
    if (betPoolAccounts.length > 0) {
        const exampleBetPool = betPoolAccounts[0].pubkey;
        const [feeVaultPDA] = await PublicKey.findProgramAddress(
            [Buffer.from("fee_vault"), exampleBetPool.toBuffer()],
            program.programId
        );

        console.log(`Example FeeVault PDA for first BetPool: ${feeVaultPDA.toBase58()}`);
    }

    console.log("\n✅ PDA Debug Complete.");
})();
