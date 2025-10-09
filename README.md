
```
░█▀▄░█▀█░▀█▀░▀█▀░█░░░█▀▄
░█▀▄░█▀█░░█░░░█░░█░░░█░█
░▀▀░░▀░▀░░▀░░░▀░░▀▀▀░▀▀░
```

Battld is a hub of for turn-based multiplayer games you can play in the terminal.

## ⚠️ Security Notice

This project uses a **simplified authentication mechanism** intended for learning and fun, not production use.

**Current authentication:**
- RSA public key signatures for player authentication
- Challenge string rotates hourly based on a time-based seed
- Only ~8,760 possible challenges per year (one per hour)
- Predictable if you know the algorithm

**What this means:**
- ✅ Good enough for a fun hobby project
- ✅ Prevents casual impersonation
- ❌ Not suitable for storing sensitive data
- ❌ Not suitable for production environments
- ❌ No protection against determined attackers

If you're running a public instance, understand that accounts can be compromised by sophisticated attackers. Don't use this for anything requiring real security!

For v2, we're considering: JWT tokens with secure server-side secrets, proper session management, or OAuth integration.

## Run the client
You'll need rust, cargo, etc, then:
```bash
git clone https://github.com/curzel-it/battld
cd battld
cargo run --bin client
```

You will be prompted to create a ssh keys pair and provide a username.

There is no account recovery whatsoever, so be sure to keep your keys around if you like the game.

## Run the server
```bash
cargo run --bin server
```
Good luck!

## Games
|Game|Status|
|---|---|
|Tic-Tac-Toe|Released|
|Rock-Paper-Scissor|In progress|
|5-Cards Poker|---|
