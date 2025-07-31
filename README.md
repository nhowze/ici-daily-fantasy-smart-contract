<!-- Badges -->

![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)
![Solana Devnet](https://img.shields.io/badge/network-solana-blue)

# 🌟 Fantasy Sports Anchor Program — Decentralized DFS on Solana

A Solana-based decentralized daily fantasy sports protocol built with Anchor.

Users can:

* Place bets on player stat lines (e.g. "Lamar Jackson over 200 yards")
* Receive a unique NFT representing their pick
* Trade picks on a secondary market before results are finalized
* Claim winnings after results are published

This program powers [NextManUp.io](https://nextmanup.io) — an NFT-based DFS platform.

> ⚠️ **Security Disclaimer:** This smart contract is **not audited**. Do **not** deploy to mainnet with real funds unless fully reviewed. It is tested and deployed safely on Devnet only.

---

## === DEV SETUP 🚀 ===

## 1️⃣ One-time Host Setup

👉 On **host machine** (not in Docker), edit this:

```bash
nano ~/.config/solana/cli/config.yml
```

Change:

```yaml
keypair_path: /home/nhowze/.config/solana/id.json
```

To:

```yaml
keypair_path: /root/.config/solana/id.json
```

This ensures Docker can use the wallet.

---

## 2️⃣ Scripts / Project Workflow

### Build Docker image (after Dockerfile changes):

```bash
./docker-build.sh
```

Uses:

```bash
COMPOSE_BAKE=true BUILDKIT_PROGRESS=plain docker compose build
```

---

### Build Anchor program:

```bash
./build.sh
```

---

### Deploy program to Devnet:

```bash
./deploy.sh
```

* Sets `ANCHOR_PROVIDER_URL`
* Sets `ANCHOR_WALLET`
* Runs `anchor build && anchor deploy`

**IMPORTANT:**
👉 After deploying, update `Anchor.toml` with new Program ID:

```toml
[programs.devnet]
fantasy_sports = "NEW_PROGRAM_ID_HERE"
```

👉 And update tests with new Program ID if needed:

```typescript
const programId = new PublicKey("NEW_PROGRAM_ID_HERE");
```

---

### Run tests:

```bash
./test.sh
```

---

### Clean build artifacts:

```bash
./clean.sh
```

---

### Interactive shell in container:

```bash
./run.sh
```

---

## 🚀 Script Usage — Host vs Container

### Host Scripts

| Script            | Purpose                          |
| ----------------- | -------------------------------- |
| `docker-build.sh` | Build Docker image               |
| `build.sh`        | Build Anchor program             |
| `deploy.sh`       | Build + Deploy to Devnet         |
| `test.sh`         | Run tests                        |
| `clean.sh`        | Clean artifacts                  |
| `run.sh`          | Open interactive container shell |

### Container Scripts (run inside container)

Once inside container:

```bash
./run.sh
```

Then run:

```bash
docker/build.sh
docker/deploy.sh
docker/test.sh
docker/clean.sh
```

| Script             | Purpose                  |
| ------------------ | ------------------------ |
| `docker/build.sh`  | Build Anchor program     |
| `docker/deploy.sh` | Build + Deploy to Devnet |
| `docker/test.sh`   | Run tests                |
| `docker/clean.sh`  | Clean artifacts          |

---

### Example Workflows

#### Host

```bash
./docker-build.sh        # Only when Dockerfile changes
./clean.sh
./build.sh
./deploy.sh              # New deploy → update Anchor.toml & tests
./test.sh
```

#### Container

```bash
./run.sh                 # Open container shell
# Inside container:
docker/clean.sh
docker/build.sh
docker/deploy.sh
docker/test.sh
```

---

## 3️⃣ Notes / Gotchas

### Pin `bn.js` version (for Anchor borsh compatibility)

In `package.json`, add:

```json
"resolutions": {
  "bn.js": "5.2.1"
}
```

Then:

```bash
yarn install
```

---

## 4️⃣ Summary of Scripts

| Script             | Purpose                                     |
| ------------------ | ------------------------------------------- |
| `docker-build.sh`  | Build Docker image                          |
| `build.sh`         | Build Anchor program                        |
| `deploy.sh`        | Build + Deploy to Devnet                    |
| `test.sh`          | Run tests                                   |
| `clean.sh`         | Clean artifacts                             |
| `run.sh`           | Interactive shell                           |
| `docker/build.sh`  | Build Anchor program (inside container)     |
| `docker/deploy.sh` | Build + Deploy to Devnet (inside container) |
| `docker/test.sh`   | Run tests (inside container)                |
| `docker/clean.sh`  | Clean artifacts (inside container)          |

---

## 5️⃣ Final Notes

✅ Always **redeploy** after major Anchor / Solana changes
✅ Always update Program IDs in both `Anchor.toml` + tests
✅ If you see `DeclaredProgramIdMismatch` → your program ID is stale
✅ Use either **host** or **container** workflow — both work

---

## 👥 Contributing

PRs and issues welcome. Feel free to fork and improve this contract or its tooling.

## 👮‍♂️ Maintainer

Created by [@NicHowze](https://github.com/nhowze)
Project: [https://nextmanup.io](https://nextmanup.io)

## 📄 License

This project is licensed under the [MIT License](./LICENSE).

---

🎉 That’s it! You now have a fully modern, clean workflow with flexible host & container scripts 🚀
