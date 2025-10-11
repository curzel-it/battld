
```
░█▀▄░█▀█░▀█▀░▀█▀░█░░░█▀▄
░█▀▄░█▀█░░█░░░█░░█░░░█░█
░▀▀░░▀░▀░░▀░░░▀░░▀▀▀░▀▀░
```

Battld is a hub of for turn-based multiplayer games you can play in the terminal.

Join the Beta!

Just need to run the client, a server is running at [battld.curzel.it](https://battld.curzel.it)

## Run the client
You'll need rust, cargo, etc, then:
```bash
git clone https://github.com/curzel-it/battld
cd battld
cargo run --bin client
```

By default a `config.json` is automatically created at runtime, pointed to `battld.curzel.it`, where I am running a public beta.

You will be prompted to create a ssh keys pair and provide a username. 
There is no account recovery whatsoever, so be sure to keep your keys around if you like the game.

## Games
### Coming Soon
* 5-Cards poker
* Something something Tower Defense
* If you have ideas please send them over!

### Tic-Tac-Toe
```
==================================================
  Tic-Tac-Toe
==================================================

  You are: X

   X | · | · 
  ---+---+---
   · | O | · 
  ---+---+---
   · | · | · 

  YOUR TURN

  Enter move as 'row col' (0-indexed, e.g., '1 2'):
```

### Rock-Paper-Scissors
```
==================================================
  Rock-Paper-Scissors
==================================================

  Previous Rounds:

    Round 1: Paper vs Rock - WIN

  Current Round:

    Opponent is choosing...
    You haven't selected yet

  SELECT YOUR MOVE

  Enter your choice (rock/paper/scissors):
  > 
```

## BYOC - Bring Your Own Client
Custom clients will always be allowed for all games on the platform, so, if you want to develop something cool, go ahead!

There is also no reason in 2025 to even attempt to fight aganist bots, so yeah, bots allowed.

There will be (when out of beta), however, VERY stringent rate limiting for both API and WebSockets, making the use of Bots a double edged sword.

As for now, there's no documentation of the API or the WebSockets messages whatsoever, so I suggest you run a local http server and use a proxy to see how everything works.

## Run the server
Feel free to run your own server, just note this is still in active development:
```bash
cargo run --bin server
```
Good luck!