# Fantasy Sports Anchor Project — Dev Setup 🚀

---

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

---

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

Runs `anchor build` inside container.

---

### Deploy program to Devnet:

```bash
./deploy.sh
```

- Sets `ANCHOR_PROVIDER_URL`
- Sets `ANCHOR_WALLET`
- Runs `anchor build && anchor deploy`

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

- Runs tests on Devnet
- Automatically sets `ANCHOR_PROVIDER_URL` and `ANCHOR_WALLET`

---

### Clean build artifacts:

```bash
./clean.sh
```

- Deletes `/target`  
- Runs `anchor clean`

---

### Interactive shell in container:

```bash
./run.sh
```

---

## 🚀 Script Usage — Host vs Container

---

### Host Scripts (run from your host terminal)

| Script | Purpose |
|--------|---------|
| `docker-build.sh` | Build Docker image |
| `build.sh` | Build Anchor program |
| `deploy.sh` | Build + Deploy to Devnet |
| `test.sh` | Run tests |
| `clean.sh` | Clean artifacts |
| `run.sh` | Open interactive container shell |

---

### Container Scripts (run when inside container)

Once you are inside:

```bash
./run.sh
```

Then inside:

```bash
docker/build.sh
docker/deploy.sh
docker/test.sh
docker/clean.sh
```

| Script | Purpose |
|--------|---------|
| `docker/build.sh` | Build Anchor program |
| `docker/deploy.sh` | Build + Deploy to Devnet |
| `docker/test.sh` | Run tests |
| `docker/clean.sh` | Clean artifacts |

---

### Example Host Workflow:

```bash
./docker-build.sh        # Only when Dockerfile changes
./clean.sh
./build.sh
./deploy.sh              # New deploy → update Anchor.toml & tests
./test.sh
```

---

### Example Container Workflow:

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

---

### Pin `bn.js` version (required for Anchor borsh compatibility)

In `package.json`, add:

```json
"resolutions": {
  "bn.js": "5.2.1"
}
```

Then run:

```bash
yarn install
```

---

## 4️⃣ Summary of Scripts

| Script | Purpose |
|--------|---------|
| `docker-build.sh` | Build Docker image |
| `build.sh` | Build Anchor program |
| `deploy.sh` | Build + Deploy to Devnet |
| `test.sh` | Run tests |
| `clean.sh` | Clean artifacts |
| `run.sh` | Interactive shell |
| `docker/build.sh` | Build Anchor program (inside container) |
| `docker/deploy.sh` | Build + Deploy to Devnet (inside container) |
| `docker/test.sh` | Run tests (inside container) |
| `docker/clean.sh` | Clean artifacts (inside container) |

---

## 5️⃣ Final Notes

✅ Always **redeploy** after major Anchor / Solana changes  
✅ Always update Program IDs in both `Anchor.toml` + tests  
✅ If you see `DeclaredProgramIdMismatch` → this is usually because your program ID is stale.  
✅ You can use either **host workflow** or **container workflow** — both are supported!

---

🎉 That’s it! You now have a fully modern, clean workflow with flexible host & container scripts 🚀
