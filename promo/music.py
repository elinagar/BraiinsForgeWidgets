#!/usr/bin/env python3
"""Synthesize the promo's background track: 122 BPM electronic, ~72 s.

Sections: intro (pads), verse (kick + bass + plucks), lift (hats, chords),
outro (pads). Pure numpy; no samples. Output: 44.1 kHz stereo WAV.
"""
import sys, struct, wave, math
import numpy as np

SR = 44100
BPM = 122
BEAT = 60.0 / BPM
BAR = 4 * BEAT
LENGTH_S = 72.0
N = int(SR * LENGTH_S)
t = np.arange(N) / SR

def note(name):
    names = {"C":0,"C#":1,"D":2,"D#":3,"E":4,"F":5,"F#":6,"G":7,"G#":8,"A":9,"A#":10,"B":11}
    return 440.0 * 2 ** ((names[name[:-1]] - 9) / 12 + (int(name[-1]) - 4))

def env(length, a, d, s_level, r, total):
    """ADSR over `total` samples."""
    out = np.zeros(total)
    a_n, d_n, r_n = int(a*SR), int(d*SR), int(r*SR)
    s_n = max(0, total - a_n - d_n - r_n)
    idx = 0
    out[idx:idx+a_n] = np.linspace(0, 1, a_n, endpoint=False); idx += a_n
    out[idx:idx+d_n] = np.linspace(1, s_level, d_n, endpoint=False); idx += d_n
    out[idx:idx+s_n] = s_level; idx += s_n
    out[idx:idx+r_n] = np.linspace(s_level, 0, min(r_n, total-idx))
    return out

def place(buf, sig, at_s, gain=1.0):
    i = int(at_s * SR)
    j = min(N, i + len(sig))
    if i < N:
        buf[i:j] += gain * sig[:j-i]

def saw(f, n):
    x = np.arange(n) / SR
    return 2 * ((x * f) % 1.0) - 1

def sine(f, n):
    x = np.arange(n) / SR
    return np.sin(2 * np.pi * f * x)

def lowpass(x, cutoff):
    rc = 1 / (2 * math.pi * cutoff); a = (1/SR) / (rc + 1/SR)
    y = np.empty_like(x); acc = 0.0
    for i in range(len(x)):
        acc += a * (x[i] - acc); y[i] = acc
    return y

def kick(n=int(0.35*SR)):
    x = np.arange(n) / SR
    f = 150 * np.exp(-x * 25) + 45
    ph = np.cumsum(2 * np.pi * f / SR)
    return np.sin(ph) * np.exp(-x * 9) * 0.9

def hat(n=int(0.06*SR), open_=False):
    rng = np.random.default_rng(1)
    x = rng.uniform(-1, 1, n)
    x = x - lowpass(x, 5000)  # crude high-pass
    e = np.exp(-np.arange(n) / SR * (18 if not open_ else 8))
    return x * e * 0.25

def snare(n=int(0.18*SR)):
    rng = np.random.default_rng(2)
    x = rng.uniform(-1, 1, n) * np.exp(-np.arange(n)/SR*22)
    body = sine(190, n) * np.exp(-np.arange(n)/SR*30)
    return (x * 0.5 + body * 0.6) * 0.7

def pluck(f, dur):
    n = int(dur * SR)
    s = saw(f, n) * 0.5 + sine(f, n) * 0.5
    s = lowpass(s, 1800 + 4*f)
    return s * env(n, 0.005, 0.12, 0.35, 0.15, n)

def bass(f, dur):
    n = int(dur * SR)
    s = saw(f, n) * 0.6 + sine(f, n) * 0.6
    s = lowpass(s, 380)
    return s * env(n, 0.005, 0.08, 0.6, 0.06, n)

def pad(chord, dur):
    n = int(dur * SR)
    s = np.zeros(n)
    for f in chord:
        for det in (-0.004, 0.0, 0.004):
            s += saw(f * (1 + det), n) / 6
    s = lowpass(s, 900)
    return s * env(n, 0.6, 0.3, 0.8, 1.0, n)

# --- arrangement -----------------------------------------------------------
L = np.zeros(N); R = np.zeros(N)
prog = [  # (bass root, chord) per bar, a 4-bar loop in A minor / C
    ("A2", ["A3","C4","E4"]), ("F2", ["F3","A3","C4"]),
    ("C3", ["C4","E4","G4"]), ("G2", ["G3","B3","D4"]),
]
bars = int(LENGTH_S / BAR)
for b in range(bars):
    t0 = b * BAR
    root, chord = prog[b % 4]
    section = "intro" if b < 2 else "verse" if b < 6 else "lift" if b < 14 else "verse2" if b < 18 else "outro"
    # pads: intro, lift, outro
    if section in ("intro", "lift", "outro"):
        p = pad([note(c) for c in chord], BAR * 1.02)
        place(L, p, t0, 0.22); place(R, p, t0 + 0.01, 0.22)
    if section in ("verse", "lift", "verse2"):
        for beat in range(4):
            place(L, kick(), t0 + beat*BEAT, 0.9); place(R, kick(), t0 + beat*BEAT, 0.9)
            if beat in (1, 3):
                place(L, snare(), t0 + beat*BEAT, 0.5); place(R, snare(), t0 + beat*BEAT, 0.5)
            for eighth in range(2):
                at = t0 + beat*BEAT + eighth*BEAT/2
                h = hat(open_=(eighth == 1 and beat == 3))
                place(L, h, at, 0.6 if eighth else 0.8); place(R, h, at + 0.002, 0.8 if eighth else 0.6)
        # bass: root on 1 and 3, octave on the "and" of 2 and 4
        bf = note(root)
        for beat in range(4):
            place(L, bass(bf if beat % 2 == 0 else bf*2, BEAT*0.45), t0 + beat*BEAT, 0.8)
            place(R, bass(bf if beat % 2 == 0 else bf*2, BEAT*0.45), t0 + beat*BEAT, 0.8)
            place(L, bass(bf, BEAT*0.2), t0 + beat*BEAT + BEAT*0.5, 0.5)
            place(R, bass(bf, BEAT*0.2), t0 + beat*BEAT + BEAT*0.5, 0.5)
    if section in ("verse", "lift", "verse2"):
        # plucked arpeggio, 16ths, alternating sides
        seq = [note(c) for c in chord] + [note(chord[0]) * 2]
        for k in range(16):
            at = t0 + k * BEAT / 4
            f = seq[k % 4] * (2 if section == "lift" and k % 8 >= 4 else 1)
            s = pluck(f, BEAT * 0.5)
            (L if k % 2 else R).__iadd__(0)  # no-op keeps flake8 quiet
            place(L if k % 2 == 0 else R, s, at, 0.28)
            place(R if k % 2 == 0 else L, s, at, 0.14)

# master: gentle fade in/out, soft clip, normalize
fade = np.ones(N)
fi, fo = int(1.5*SR), int(4.0*SR)
fade[:fi] = np.linspace(0, 1, fi); fade[-fo:] = np.linspace(1, 0, fo)
mix = np.stack([L, R], axis=1) * fade[:, None]
mix = np.tanh(mix * 1.2)
mix *= 0.85 / np.max(np.abs(mix))
out = sys.argv[1] if len(sys.argv) > 1 else "music.wav"
with wave.open(out, "wb") as w:
    w.setnchannels(2); w.setsampwidth(2); w.setframerate(SR)
    w.writeframes((mix * 32767).astype("<i2").tobytes())
print(f"wrote {out}: {LENGTH_S:.0f} s, {bars} bars at {BPM} BPM")
