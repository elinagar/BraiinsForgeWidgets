# Promo video

`render.py` produces a 1080p, 30 fps, 72-second MP4 with motion graphics, captions, the game's own cues, and a
synthesized backing track from `music.py`. Everything is generated from the repo's screenshots and mockups, so it can be
re-rendered after any change.

```shell
python3 promo/render.py --preview /tmp/preview      # one PNG per scene, quick look
python3 promo/render.py party-quiz-promo.mp4        # full render, a few minutes
```

Needs Python 3 with numpy and Pillow, and `ffmpeg` on PATH. The Ubuntu font is used when present, DejaVu otherwise.

## Real footage

Landscape phone clips, 10 to 15 seconds each, dropped into `promo/clips/` replace the Deck mockups in the matching
scene. Names:

| File | Scene | What to film |
| --- | --- | --- |
| `lobby.mp4` | 5.5 s to 13.5 s | the Deck lobby with the QR code, someone scanning and appearing as an avatar |
| `question.mp4` | 13.5 s to 21.5 s | a question with the countdown ring and the amber lights in the last seconds |
| `reveal.mp4` | 21.5 s to 29.5 s | the reveal with the green or red lights and people reacting |
| `podium.mp4` | 29.5 s to 37.5 s | the podium with the fanfare |

Tips: turn the room lights down so the Deck and its LED strip carry the frame, hold the phone still or on a small
tripod, and include hands and phones in the shot when you can. The captions and the phone screenshots are still
overlaid on top of your footage; edit `SCENES` and the `scene_*` functions in `render.py` to change timings or copy.

## Music

`music.py` writes a 122 BPM electronic track: pads for the intro and outro, kick, snare, hats, bass, and a plucked
arpeggio in the verses. It is generated, so it is free to use anywhere. `render.py` ducks it under the game cues
(join chime, start sting, ticks, correct, standings riser, fanfare) placed at the moments the corresponding scene
shows them.
