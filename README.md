
```
░█▀▄░█▀█░▀█▀░▀█▀░█░░░█▀▄
░█▀▄░█▀█░░█░░░█░░█░░░█░█
░▀▀░░▀░▀░░▀░░░▀░░▀▀▀░▀▀░
```

Battld is a hub for turn-based multiplayer games you can play in the terminal.

I had this idea, seemed cool, got bored.

Maybe I'll come back to it, some day... 💀

## Run the thing
You'll need rust, cargo, etc, then:
```bash
git clone https://github.com/curzel-it/battld
cd battld
cargo run --bin server &
cargo run --bin client
```

A `config.json` is automatically created at runtime, pointed to `localhost:3000`.

You will be prompted to create a ssh keys pair and provide a username. 
There is no account recovery whatsoever, so be sure to keep your keys around if you like the game.

## Games

### Chess
There is a chess prototype, unfinished, unpolished, not selectable in the ui.

### Briscola
Briscola is an italian card game, more info [here](https://en.wikipedia.org/wiki/Briscola):
```
  Briscola:   Deck:          Opponent played:   
  ╭───────╮                  ╭───────╮
  │     K │   33             │     A │
  │╰─┼─╮S │   cards          │       │
  │ ╭┴╮   │   left           │   S   │
  │ │ │   │                  │       │
  ╰───────╯                  ╰───────╯

  Your hand:
  ╭───────╮  ╭───────╮  ╭───────╮  
  │     4 │  │     2 │  │     7 │  
  │  S S  │  │       │  │  D D  │  
  │       │  │  C C  │  │ D D D │  
  │  S S  │  │       │  │  D D  │  
  ╰───────╯  ╰───────╯  ╰───────╯  
     [0]        [1]        [2]        

  Your turn! Enter card index:
  > 
```

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