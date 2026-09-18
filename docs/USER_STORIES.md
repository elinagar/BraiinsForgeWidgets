# User Stories

One section per widget, in the format of bmc-main's `docs/stories/widgets/`, so a widget can be proposed upstream by
copying its section into that folder.

## Party Quiz Widget

The Party Quiz widget turns the Deck into a multiplayer trivia board. Players join from their phones by scanning a QR
code on the Deck's screen; the phone becomes the buzzer and the Deck shows the question, the countdown, the reveal, and
the standings. Nothing is installed on the phone and no traffic leaves the local network during a game. Questions come
in packs from the Braiins Forge Nexus, cached on the device so a game works offline once a pack is present.

### User stories

#### Join from a phone

> As a player, I want to join the quiz from my phone by scanning the Deck's screen so that I do not install anything or
> create an account.

- The lobby shows a QR code and, beneath it, the same address in plain text (`http://<deck-ip>:<port>/`) for phones
  that cannot scan.
- Opening the address serves a controller page from the Deck itself. It works in any current mobile browser and needs
  no internet connection.
- The player enters a name (1 to 12 characters) and picks one of six colours; the Deck adds their avatar to the lobby
  within a second.
- A colour already taken by another player is greyed out on the phone while a free colour remains; from the seventh
  player on, colours repeat. A duplicate name is suffixed with a number.
- A game holds up to 12 players. A thirteenth phone is told the lobby is full.
- The lobby shows the count of joined players out of the maximum, and one LED per player lights in that player's colour
  from the left end of the strip.
- Closing and reopening the phone page during a game returns the player to their seat with their score intact, keyed on
  a token the page keeps in the browser's local storage.

#### Host the game

> As the host, I want to start, pause, and end the game from my phone so that the Deck stays a board and nobody has to
> reach it.

- The first player to join becomes the host and sees Start, Skip question, and End game controls on their phone. A host
  crown marks them on the Deck.
- The game cannot start with fewer than two players. Start is disabled and says why.
- When the host leaves, the next player in join order becomes host.
- Ending a game shows the podium and then returns the Deck to a fresh lobby after 30 seconds, or immediately when the
  host taps New game.
- The host can tap **Restart** at any time to kick everyone, including themselves; the Deck returns to an empty lobby
  and every phone goes back to the join screen on its next poll. The Deck's own Reset control in the top bar does the
  same from the device, in every phase, for the case where the host's phone is the thing that is stuck.

#### Answer on the phone, watch the Deck

> As a player, I want the question and the answers on the Deck and only the four buttons on my phone so that everyone
> looks at the same screen and the phone never spoils the answer.

- Each question shows on the Deck with its category chip, difficulty, question number of total, and four answer cards
  in fixed colours with fixed shapes (triangle, diamond, circle, square), so the phone buttons match by colour and shape
  and colour-blind players can still play.
- The phone shows four large buttons of the same colours and shapes, the player's name and colour, the running score,
  and a thin progress bar for the countdown. It never shows the answer text.
- Tapping a button locks the answer. The button stays highlighted and the other three dim; a tap after locking does
  nothing.
- The Deck shows how many players have answered out of the total and a small dot per answered player in their colour.
- The countdown is a ring on the Deck counting down the *Answer time* in whole seconds. When every player has answered
  the countdown ends early after a one-second grace.
- Scoring is time-weighted: a correct answer in the first second earns 1 000 points, decaying linearly to 500 at the
  deadline. Three correct answers in a row add a 200-point streak bonus per further correct answer. A wrong or missing
  answer earns nothing and resets the streak.

#### See the reveal

> As a player, I want to see the right answer, how the room voted, and a one-line explanation so that a wrong answer
> still teaches something.

- The reveal highlights the correct card with a light ring and dims the other three.
- Beside the cards, one bar per answer shows how many players chose it, with the names of those players.
- The fastest correct player is named with the points they earned.
- A one-line explanation from the question pack appears under the correct answer. When the pack has none, the line is
  omitted rather than left blank.
- Each phone shows Correct or Not this time, the answer the player picked, the points earned, the new rank with its
  change, and the streak count.
- The reveal lasts 4 seconds, then the standings show for 5 seconds, then the next question begins. The host can skip
  ahead.

#### Follow the standings

> As a player, I want the standings after each question and a podium at the end so that the room can cheer.

- The standings show a podium for the top three and a ranked list for the rest, each with points earned in the last
  question.
- The final screen shows the podium with the winner's colour on the LEDs.
- The Deck keeps a best-ever list of the top ten scores with player name and date across games, shown on the final
  screen when the current game enters it, and on the lobby as a single line.

#### Feel it on the LEDs and hear it

> As a player, I want the Deck's lights and sounds to react so that the game feels like a game show and not a form.

- Lobby: the strip flashes in the newest player's colour for a moment as they join, with a soft chime.
- Game start: a rising three-note sting on the first question; a two-note "attention" cue on every later question.
- Countdown: an amber chase runs during the last 5 seconds, with a quiet tick each second during the last 3, and a
  quiet tap on the Deck each time a player locks an answer.
- Reveal: the strip breathes green with a chime when at least half the players were right, red with a low tone
  otherwise.
- Standings: a short riser as the standings appear.
- Podium: a chase in the winner's colour for 3 seconds with a brass fanfare.
- All LED effects are scene-local. When the widget is not on the active scene nothing lights.
- Sounds follow the device volume, including the night mode volume, and can be turned off with the *Sounds* parameter.

#### Get questions

> As a user, I want fresh questions in the categories my room likes so that the game does not repeat itself.

- A *Category pack* parameter chooses Mixed or one of the packs the Nexus offers. *Difficulty* chooses Easy, Medium,
  Hard, or Mixed. *Questions per game* chooses 5 to 30 in steps of 5. *Answer time* chooses 10 to 40 seconds.
- The widget fetches a pack of 200 questions for the chosen category and difficulty from the Braiins Forge Nexus when a
  scene with the widget becomes active and the cached pack is older than a day or has fewer than 50 unplayed questions.
- Packs are stored in the widget's blob cache, so a game starts without internet when a pack is cached. Without any
  cached pack and without internet, the lobby says the Deck needs to be online once to download questions.
- The widget remembers the IDs of played questions per pack and never repeats one until every question in the pack has
  been played, then it starts over.
- Each question carries its source pack ID and version so a withdrawn question can be dropped on the next refresh.

#### Stay safe on the local network

> As a user, I want the game to be harmless to expose on my Wi-Fi so that guests' phones can talk to the Deck without
> a second thought.

- The controller endpoint accepts only the game's own small set of requests. Anything else returns 404. Bodies over
  1 KiB are rejected.
- Player names are trimmed and rendered as plain text. Nothing a phone sends is interpreted as markup or code.
- A phone can only act on its own seat, identified by its token. It cannot answer for or rename another player.
- The listener runs only while the widget's scene is active or a game is in progress, and closes when the widget
  unloads. While the Deck shows another scene the widget is dormant and phones get no answer; the controller page
  names that state and asks the player to bring the scene back or turn off cycling.

### Constraints

- The widget renders full-screen on the BMC100 Deck at 1280x480 only. The board needs the whole panel, and the phone
  controller assumes a Deck with an LED strip for the intended experience. Smaller combined-scene sizes and the miner
  displays are out of scope.
- *Category pack*, *Difficulty*, *Questions per game*, *Answer time*, *Show player names on Deck*, *Sounds*, and
  *Sound level* are manifest-driven parameters configurable from the web UI. Scores follow the device number format.
- The LED strip API drives the whole strip with one effect at a time (solid, breathe, chase), not individual LEDs, so
  per-player LEDs are out of reach; the lobby flash above is the substitute.
- The widget uses one HTTP listener on an ephemeral port and registers it over mDNS as `_deck-quiz._tcp` so a
  companion page could find it later. Phones poll the game state every 400 ms during a question and every second
  otherwise; a poll that names the version it already has gets an empty reply, so the common case is cheap.
- The controller page is a single HTML file under 20 KiB bundled as a widget asset and served with a long cache header.
  It uses no external resources.
- Question packs come from the Braiins Forge Nexus at `/api/v1/data/party-quiz/…`. The Nexus corpus, its generation and
  review pipeline, and the pack endpoint are a separate deliverable tracked in the implementation plan. Until it exists,
  the widget ships a bundled starter pack in its assets: 460 hand-reviewed questions, 46 per pack, growing toward 1 000.
- The widget shows `Loading…` in the lobby until network info and a question pack are available, and `Offline` with an
  explanation when neither a cached pack nor the Nexus is reachable.
- Touch on the Deck itself is limited to a Reset control in the lobby's top bar, shown once someone has joined. It
  clears every seat; phones return to the join screen on their next poll. The Deck screen is a board, not a
  controller.
