#!/usr/bin/env python3
"""Render the Party Quiz promo: 1920x1080 at 30 fps, about 72 s, with music.

    python3 promo/render.py out.mp4              # full render
    python3 promo/render.py --preview out_dir    # one PNG per scene, no encode

Real footage: drop landscape clips into promo/clips/ named lobby.mp4,
question.mp4, reveal.mp4, podium.mp4 and they replace the Deck mockups in
those scenes (frames are extracted with ffmpeg and cover-fitted).
"""
import math, os, subprocess, sys, wave, shutil, tempfile
import numpy as np
from PIL import Image, ImageDraw, ImageFont, ImageFilter

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
IMG = os.path.join(ROOT, "docs", "images")
SND = os.path.join(ROOT, "party-quiz", "assets", "sounds")
CLIPS = os.path.join(ROOT, "promo", "clips")
W, H, FPS = 1920, 1080, 30
BG = (14, 15, 18); CARD = (21, 23, 28); TEXT = (244, 241, 234); MUTED = (168, 164, 154)
ACCENT = (245, 165, 36); CORAL = (229, 83, 61); BLUE = (59, 130, 246); LIME = (132, 204, 22)
VIOLET = (168, 85, 247); TEAL = (20, 184, 166)
PLAYER_COLORS = [CORAL, BLUE, LIME, VIOLET, ACCENT, TEAL]

def font(size, bold=True):
    for p in ["/usr/share/fonts/truetype/ubuntu/Ubuntu-B.ttf" if bold else "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
              "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf" if bold else "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"]:
        if os.path.exists(p):
            return ImageFont.truetype(p, size)
    return ImageFont.load_default()

F_TITLE, F_H1, F_H2, F_BODY, F_SMALL, F_MONO = font(150), font(84), font(56), font(40, False), font(30, False), None
for p in ["/usr/share/fonts/truetype/ubuntu/UbuntuMono-B.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf"]:
    if os.path.exists(p): F_MONO = ImageFont.truetype(p, 44); break
F_MONO = F_MONO or F_BODY

# ── easing ───────────────────────────────────────────────────────────────
def clamp(x, a=0.0, b=1.0): return max(a, min(b, x))
def ease_out(x): x = clamp(x); return 1 - (1 - x) ** 3
def ease_in_out(x): x = clamp(x); return x * x * (3 - 2 * x)
def seg(t, start, dur): return clamp((t - start) / dur)

# ── assets ───────────────────────────────────────────────────────────────
_cache = {}
def img(name):
    if name not in _cache:
        _cache[name] = Image.open(os.path.join(IMG, name)).convert("RGBA")
    return _cache[name]

def rounded(im, radius):
    mask = Image.new("L", im.size, 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, im.size[0]-1, im.size[1]-1], radius, fill=255)
    out = im.copy(); out.putalpha(mask); return out

def shadowed(canvas, im, xy, blur=28, offset=(0, 18), alpha=140):
    sh = Image.new("RGBA", (im.size[0]+blur*4, im.size[1]+blur*4), (0,0,0,0))
    a = im.split()[3].point(lambda v: int(v * alpha / 255))
    sh.paste((0,0,0,255), (blur*2, blur*2), a)
    sh = sh.filter(ImageFilter.GaussianBlur(blur))
    canvas.alpha_composite(sh, (xy[0]-blur*2+offset[0], xy[1]-blur*2+offset[1]))
    canvas.alpha_composite(im, xy)

def fit(im, w, h):
    s = min(w / im.size[0], h / im.size[1])
    return im.resize((max(1, int(im.size[0]*s)), max(1, int(im.size[1]*s))), Image.LANCZOS)

def cover(im, w, h):
    s = max(w / im.size[0], h / im.size[1])
    r = im.resize((int(im.size[0]*s)+1, int(im.size[1]*s)+1), Image.LANCZOS)
    x = (r.size[0]-w)//2; y = (r.size[1]-h)//2
    return r.crop((x, y, x+w, y+h))

def text(draw, xy, s, f, fill=TEXT, anchor="la", glow=None, canvas=None):
    if glow and canvas is not None:
        layer = Image.new("RGBA", canvas.size, (0,0,0,0))
        ImageDraw.Draw(layer).text(xy, s, font=f, fill=glow + (200,), anchor=anchor)
        canvas.alpha_composite(layer.filter(ImageFilter.GaussianBlur(18)))
        draw = ImageDraw.Draw(canvas)
    draw.text(xy, s, font=f, fill=fill, anchor=anchor)

def fade_layer(canvas, layer, alpha):
    if alpha <= 0: return
    if alpha >= 1: canvas.alpha_composite(layer); return
    a = layer.split()[3].point(lambda v: int(v * alpha)); l2 = layer.copy(); l2.putalpha(a); canvas.alpha_composite(l2)

def led_strip(canvas, t, y, colors, mode="chase", intensity=1.0):
    """Ten glowing dots across the width, like the Deck's strip."""
    layer = Image.new("RGBA", canvas.size, (0,0,0,0)); d = ImageDraw.Draw(layer)
    for i in range(10):
        x = 360 + i * 133
        if mode == "chase":
            ph = (t * 1.6 - i * 0.12) % 1.0; a = max(0.0, 1 - ph * 2.2)
            col = colors[0]
        elif mode == "breathe":
            a = 0.45 + 0.55 * (0.5 + 0.5 * math.sin(t * 2.2)); col = colors[0]
        elif mode == "players":
            col = colors[i % len(colors)]; a = 1.0 if i < len(colors) else 0.0
        else:
            col = colors[0]; a = 1.0
        a *= intensity
        if a <= 0.02:
            d.ellipse([x-14, y-14, x+14, y+14], fill=(38, 40, 46, 255)); continue
        d.ellipse([x-30, y-30, x+30, y+30], fill=col + (int(90*a),))
        d.ellipse([x-14, y-14, x+14, y+14], fill=col + (int(255*a),))
    layer = layer.filter(ImageFilter.GaussianBlur(3))
    canvas.alpha_composite(layer)

def caption(canvas, t, start, title, sub=None, y=930):
    a = ease_out(seg(t, start, 0.6))
    if a <= 0: return
    layer = Image.new("RGBA", canvas.size, (0,0,0,0)); d = ImageDraw.Draw(layer)
    dy = int((1 - a) * 40)
    text(d, (W//2, y + dy), title, F_H2, TEXT, "mm")
    if sub: text(d, (W//2, y + 58 + dy), sub, F_BODY, MUTED, "mm")
    fade_layer(canvas, layer, a)

# ── optional real clips ─────────────────────────────────────────────────
_clip_frames = {}
def clip_frame(name, t_local):
    """Frame from promo/clips/<name>.mp4 at local time, or None."""
    path = os.path.join(CLIPS, name + ".mp4")
    if not os.path.exists(path): return None
    if name not in _clip_frames:
        d = tempfile.mkdtemp(prefix="pq-clip-")
        subprocess.run(["ffmpeg", "-v", "error", "-i", path, "-vf", f"fps={FPS},scale={W}:{H}:force_original_aspect_ratio=increase,crop={W}:{H}",
                        os.path.join(d, "f%05d.png")], check=True)
        _clip_frames[name] = sorted(os.listdir(d)); _clip_frames[name+"_dir"] = d
    frames = _clip_frames[name]
    if not frames: return None
    i = min(len(frames)-1, int(t_local * FPS))
    return Image.open(os.path.join(_clip_frames[name+"_dir"], frames[i])).convert("RGBA")

# ── scenes ───────────────────────────────────────────────────────────────
SCENES = [(0, 5.5, "title"), (5.5, 13.5, "lobby"), (13.5, 21.5, "question"), (21.5, 29.5, "reveal"),
          (29.5, 37.5, "podium"), (37.5, 47.5, "features"), (47.5, 57.5, "hood"), (57.5, 67.5, "cta"), (67.5, 72.0, "end")]
TOTAL = 72.0

def deck_and_phone(canvas, t, start, deck_name, phone_name, clip=None, phone_enter=1.2, zoom=(1.0, 1.06)):
    """Deck mockup (or clip) large on the left, a phone sliding in on the right."""
    tl = t - start
    frame = clip_frame(clip, tl) if clip else None
    if frame is not None:
        canvas.alpha_composite(frame)
        dark = Image.new("RGBA", (W, H), (0, 0, 0, 90)); canvas.alpha_composite(dark)
    else:
        deck = img(deck_name)
        z = zoom[0] + (zoom[1]-zoom[0]) * ease_in_out(tl / 8.0)
        dw = int(1330 * z); dh = int(dw * deck.size[1] / deck.size[0])
        dimg = rounded(deck.resize((dw, dh), Image.LANCZOS), 22)
        a = ease_out(seg(t, start, 0.7))
        x = 90 - int((1-a) * 60); y = 150 + (560 - dh)//2
        layer = Image.new("RGBA", canvas.size, (0,0,0,0)); shadowed(layer, dimg, (x, y)); fade_layer(canvas, layer, a)
    if phone_name:
        ph = img(phone_name); pw = 330; phh = int(pw * ph.size[1] / ph.size[0])
        pimg = rounded(ph.resize((pw, phh), Image.LANCZOS), 40)
        a = ease_out(seg(t, start + phone_enter, 0.8))
        if a > 0:
            x = W - 120 - pw + int((1-a) * 220); y = 140
            layer = Image.new("RGBA", canvas.size, (0,0,0,0)); shadowed(layer, pimg, (x, y), blur=36); fade_layer(canvas, layer, a)

def scene_title(canvas, d, t):
    a1 = ease_out(seg(t, 0.3, 1.0)); a2 = ease_out(seg(t, 1.2, 0.8)); a3 = ease_out(seg(t, 2.0, 0.8))
    layer = Image.new("RGBA", canvas.size, (0,0,0,0)); ld = ImageDraw.Draw(layer)
    text(ld, (W//2, 420 + int((1-a1)*30)), "PARTY QUIZ", F_TITLE, TEXT, "mm", glow=ACCENT, canvas=layer)
    fade_layer(canvas, layer, a1)
    layer = Image.new("RGBA", canvas.size, (0,0,0,0)); ld = ImageDraw.Draw(layer)
    text(ld, (W//2, 540), "for the Braiins Deck", F_H2, ACCENT, "mm"); fade_layer(canvas, layer, a2)
    layer = Image.new("RGBA", canvas.size, (0,0,0,0)); ld = ImageDraw.Draw(layer)
    text(ld, (W//2, 620), "Your Deck is the game show. Your phones are the buzzers.", F_BODY, MUTED, "mm"); fade_layer(canvas, layer, a3)
    led_strip(canvas, t, 800, [ACCENT], "chase", intensity=ease_out(seg(t, 0.8, 1.0)))

def scene_lobby(canvas, d, t):
    deck_and_phone(canvas, t, 5.5, "deck-lobby-mockup.png", "phone-join.png", clip="lobby")
    caption(canvas, t, 6.2, "Scan the Deck to join. No app, no account.", "The Deck serves the controller page itself, over your Wi-Fi.")

def scene_question(canvas, d, t):
    deck_and_phone(canvas, t, 13.5, "deck-question-mockup.png", "phone-answer.png", clip="question")
    caption(canvas, t, 14.2, "Four colours, four shapes. Tap the one that matches.", "Faster answers score more. Streaks pay a bonus.")

def scene_reveal(canvas, d, t):
    deck_and_phone(canvas, t, 21.5, "deck-reveal-mockup.png", "phone-correct.png", clip="reveal")
    caption(canvas, t, 22.2, "The reveal: who voted what, who was fastest, and why.", "Lights breathe green or red. Sounds follow every stage.")

def scene_podium(canvas, d, t):
    deck_and_phone(canvas, t, 29.5, "deck-standings-mockup.png", "phone-podium.png", clip="podium")
    caption(canvas, t, 30.2, "Standings after every question. A podium at the end.", "The Deck keeps its best-ever record.")

FEATURES = [("460", "questions in 10 categories"), ("12", "players on their phones"), ("0", "apps, accounts or cloud"),
            ("Offline", "runs inside the Deck itself"), ("Host", "start, skip, restart by phone"), ("Kids", "a pack of their own")]
def scene_features(canvas, d, t):
    a = ease_out(seg(t, 37.7, 0.6))
    layer = Image.new("RGBA", canvas.size, (0,0,0,0)); ld = ImageDraw.Draw(layer)
    text(ld, (W//2, 130), "Built for a living room", F_H1, TEXT, "mm"); fade_layer(canvas, layer, a)
    cw, ch, gap = 540, 300, 40
    x0 = (W - 3*cw - 2*gap)//2; y0 = 260
    for i, (big, small) in enumerate(FEATURES):
        ai = ease_out(seg(t, 38.2 + i*0.22, 0.6))
        if ai <= 0: continue
        cx = x0 + (i % 3) * (cw + gap); cy = y0 + (i // 3) * (ch + gap) + int((1-ai)*40)
        card = Image.new("RGBA", (cw, ch), (0,0,0,0)); cd = ImageDraw.Draw(card)
        cd.rounded_rectangle([0,0,cw-1,ch-1], 26, fill=CARD + (255,))
        cd.text((40, 70), big, font=F_H1, fill=ACCENT if i % 2 == 0 else TEXT, anchor="lm")
        cd.text((40, 190), small, font=F_BODY, fill=MUTED, anchor="lm")
        l2 = Image.new("RGBA", canvas.size, (0,0,0,0)); shadowed(l2, card, (cx, cy), blur=20, alpha=110); fade_layer(canvas, l2, ai)

def scene_hood(canvas, d, t):
    a = ease_out(seg(t, 47.7, 0.6))
    layer = Image.new("RGBA", canvas.size, (0,0,0,0)); ld = ImageDraw.Draw(layer)
    text(ld, (W//2, 130), "Under the hood", F_H1, TEXT, "mm"); fade_layer(canvas, layer, a)
    # Deck box centre
    a2 = ease_out(seg(t, 48.2, 0.7))
    layer = Image.new("RGBA", canvas.size, (0,0,0,0)); ld = ImageDraw.Draw(layer)
    bx, by, bw, bh = 660, 380, 600, 260
    ld.rounded_rectangle([bx, by, bx+bw, by+bh], 30, fill=CARD + (255,), outline=ACCENT + (255,), width=4)
    text(ld, (bx+bw//2, by+70), "Braiins Deck", F_H2, TEXT, "mm")
    text(ld, (bx+bw//2, by+140), "Rust, compiled to WebAssembly", F_BODY, ACCENT, "mm")
    text(ld, (bx+bw//2, by+195), "HTTP server, game logic, LEDs, sound", F_SMALL, MUTED, "mm")
    fade_layer(canvas, layer, a2)
    # phones around with arrows
    spots = [(250, 300), (250, 720), (1670, 300), (1670, 720)]
    for i, (px, py) in enumerate(spots):
        ai = ease_out(seg(t, 49.0 + i*0.25, 0.6))
        if ai <= 0: continue
        layer = Image.new("RGBA", canvas.size, (0,0,0,0)); ld = ImageDraw.Draw(layer)
        ld.rounded_rectangle([px-70, py-130, px+70, py+130], 28, fill=CARD + (255,), outline=PLAYER_COLORS[i] + (255,), width=4)
        ld.ellipse([px-26, py-26, px+26, py+26], fill=PLAYER_COLORS[i] + (255,))
        ex = bx if px < W//2 else bx + bw; ey = by + bh//2
        ld.line([(px + (70 if px < W//2 else -70), py), (ex, ey)], fill=MUTED + (200,), width=5)
        fade_layer(canvas, layer, ai)
    a3 = ease_out(seg(t, 50.5, 0.6))
    layer = Image.new("RGBA", canvas.size, (0,0,0,0)); ld = ImageDraw.Draw(layer)
    text(ld, (W//2, 760), "Phones join over your Wi-Fi and poll the Deck. Nothing leaves the room.", F_BODY, MUTED, "mm")
    text(ld, (W//2, 830), "Open source, GPL-3.0", F_BODY, TEXT, "mm")
    fade_layer(canvas, layer, a3)

def scene_cta(canvas, d, t):
    a = ease_out(seg(t, 57.7, 0.6))
    layer = Image.new("RGBA", canvas.size, (0,0,0,0)); ld = ImageDraw.Draw(layer)
    text(ld, (W//2, 260), "Install in one command", F_H1, TEXT, "mm")
    fade_layer(canvas, layer, a)
    a2 = ease_out(seg(t, 58.4, 0.7))
    layer = Image.new("RGBA", canvas.size, (0,0,0,0)); ld = ImageDraw.Draw(layer)
    ld.rounded_rectangle([260, 400, W-260, 560], 24, fill=CARD + (255,))
    text(ld, (W//2, 480), "scripts/install-prebuilt.sh dist/party-quiz-0.1.0.nar <deck-ip>", F_MONO, LIME, "mm")
    fade_layer(canvas, layer, a2)
    a3 = ease_out(seg(t, 59.2, 0.7))
    layer = Image.new("RGBA", canvas.size, (0,0,0,0)); ld = ImageDraw.Draw(layer)
    text(ld, (W//2, 680), "github.com/elinagar/BraiinsForgeWidgets", F_H2, ACCENT, "mm", glow=ACCENT, canvas=layer)
    text(ld, (W//2, 760), "Prebuilt package, screenshots, docs, and the source.", F_BODY, MUTED, "mm")
    fade_layer(canvas, layer, a3)
    led_strip(canvas, t, 940, PLAYER_COLORS, "players", intensity=ease_out(seg(t, 60.0, 1.0)))

def scene_end(canvas, d, t):
    a = ease_out(seg(t, 67.7, 0.8))
    layer = Image.new("RGBA", canvas.size, (0,0,0,0)); ld = ImageDraw.Draw(layer)
    text(ld, (W//2, 480), "PARTY QUIZ", F_TITLE, TEXT, "mm", glow=ACCENT, canvas=layer)
    text(ld, (W//2, 600), "a widget for the Braiins Deck", F_H2, ACCENT, "mm")
    fade_layer(canvas, layer, a)
    led_strip(canvas, t, 800, [ACCENT], "breathe", intensity=0.8)

RENDER = {"title": scene_title, "lobby": scene_lobby, "question": scene_question, "reveal": scene_reveal,
          "podium": scene_podium, "features": scene_features, "hood": scene_hood, "cta": scene_cta, "end": scene_end}

def frame_at(t):
    canvas = Image.new("RGBA", (W, H), BG + (255,))
    d = ImageDraw.Draw(canvas)
    for start, end, name in SCENES:
        if start - 0.5 <= t < end + 0.5:
            layer = Image.new("RGBA", (W, H), (0,0,0,0))
            RENDER[name](layer, ImageDraw.Draw(layer), t)
            # cross-fade at scene edges
            a = min(seg(t, start, 0.5), 1 - seg(t, end - 0.5, 0.5)) if name != "end" else min(seg(t, start, 0.5), 1 - seg(t, TOTAL - 1.5, 1.5))
            fade_layer(canvas, layer, a)
    return canvas.convert("RGB")

# ── audio ────────────────────────────────────────────────────────────────
def load_wav(path, sr=44100):
    with wave.open(path, "rb") as w:
        n, ch, rate = w.getnframes(), w.getnchannels(), w.getframerate()
        data = np.frombuffer(w.readframes(n), dtype="<i2").astype(np.float32) / 32767
    if ch == 2: data = data.reshape(-1, 2)
    else: data = np.stack([data, data], axis=1)
    if rate != sr:
        idx = np.arange(0, len(data), rate / sr)
        data = np.stack([np.interp(idx, np.arange(len(data)), data[:, c]) for c in range(2)], axis=1)
    return data

CUES = [("join", 8.3), ("join", 9.1), ("start", 13.6), ("lock", 16.0), ("tick", 18.6), ("tick", 19.6), ("tick", 20.6),
        ("correct", 21.6), ("standings", 29.6), ("fanfare", 31.0), ("join", 60.2), ("join", 60.6), ("join", 61.0)]
def build_audio(music_path, out_path):
    sr = 44100; n = int(TOTAL * sr)
    music = load_wav(music_path, sr)[:n]
    if len(music) < n: music = np.vstack([music, np.zeros((n - len(music), 2))])
    duck = np.ones(n)
    cues = np.zeros((n, 2))
    for name, at in CUES:
        p = os.path.join(SND, name + ".wav")
        if not os.path.exists(p): continue
        s = load_wav(p, sr) * 0.9
        i = int(at * sr); j = min(n, i + len(s)); cues[i:j] += s[:j-i]
        a, b = max(0, i - int(0.15*sr)), min(n, j + int(0.4*sr))
        duck[a:b] = np.minimum(duck[a:b], 0.45)
    # smooth the ducking curve
    k = int(0.12 * sr); kernel = np.ones(k) / k
    duck = np.convolve(duck, kernel, mode="same")
    mix = music * duck[:, None] * 0.8 + cues
    mix = np.tanh(mix * 1.1); mix *= 0.9 / max(1e-9, np.max(np.abs(mix)))
    with wave.open(out_path, "wb") as w:
        w.setnchannels(2); w.setsampwidth(2); w.setframerate(sr)
        w.writeframes((mix * 32767).astype("<i2").tobytes())

# ── main ─────────────────────────────────────────────────────────────────
def main():
    if len(sys.argv) >= 3 and sys.argv[1] == "--preview":
        out = sys.argv[2]; os.makedirs(out, exist_ok=True)
        for start, end, name in SCENES:
            frame_at(start + min(3.0, (end - start) * 0.6)).save(os.path.join(out, f"{name}.png"))
        print("preview frames in", out); return
    out = sys.argv[1] if len(sys.argv) > 1 else "party-quiz-promo.mp4"
    tmp = tempfile.mkdtemp(prefix="pq-promo-")
    music = os.path.join(tmp, "music.wav")
    subprocess.run([sys.executable, os.path.join(ROOT, "promo", "music.py"), music], check=True)
    audio = os.path.join(tmp, "mix.wav"); build_audio(music, audio)
    cmd = ["ffmpeg", "-y", "-v", "error", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{W}x{H}", "-r", str(FPS), "-i", "-",
           "-i", audio, "-c:v", "libx264", "-preset", "medium", "-crf", "20", "-pix_fmt", "yuv420p", "-movflags", "+faststart",
           "-c:a", "aac", "-b:a", "192k", "-shortest", out]
    proc = subprocess.Popen(cmd, stdin=subprocess.PIPE)
    total = int(TOTAL * FPS)
    for i in range(total):
        proc.stdin.write(frame_at(i / FPS).tobytes())
        if i % (FPS * 6) == 0: print(f"  {i/FPS:5.1f} s / {TOTAL:.0f} s", flush=True)
    proc.stdin.close(); proc.wait()
    shutil.rmtree(tmp, ignore_errors=True)
    for d in [v for k, v in _clip_frames.items() if k.endswith("_dir")]: shutil.rmtree(d, ignore_errors=True)
    print("wrote", out)

if __name__ == "__main__":
    main()
