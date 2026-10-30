"""Timestamped screenshots during a capture session, to line up what the original client showed with the network log.

    python tools/screen_logger.py <out_dir> [seconds=2] [scale=0.5]

Every few seconds it saves the whole desktop (all screens), scaled, as <out_dir>/<unix time in ms>.jpg: the same clock as
capture_proxy.py's log lines (seconds since 1970), so a line's time finds its picture. A shot identical to the one before
(nothing moved) is skipped. Stops on Ctrl+C or when <out_dir>/stop exists.
"""
import os, sys, time
from PIL import ImageGrab, ImageChops

out = sys.argv[1]
every = float(sys.argv[2]) if len(sys.argv) > 2 else 2.0
scale = float(sys.argv[3]) if len(sys.argv) > 3 else 0.5
os.makedirs(out, exist_ok=True)
last = None
saved = 0
try:
    while not os.path.exists(os.path.join(out, "stop")):
        started = time.time()
        try:
            shot = ImageGrab.grab(all_screens=True)
        except OSError:
            # (the desktop is locked or switching: try again next time)
            time.sleep(every)
            continue
        small = shot.resize((int(shot.width * scale), int(shot.height * scale))).convert("RGB")
        if last is None or ImageChops.difference(small, last).getbbox() is not None:
            small.save(os.path.join(out, f"{int(started * 1000)}.jpg"), quality=70)
            saved += 1
            last = small
        time.sleep(max(0.0, every - (time.time() - started)))
except KeyboardInterrupt:
    pass
print(f"{saved} screenshots in {out}")
