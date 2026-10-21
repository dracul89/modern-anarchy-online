"""Logging TCP proxy for studying the login protocol with the ORIGINAL client.

    python tools/capture_proxy.py [listen_port=7505] [remote=cm.d1.funcom.com:7505] [out=capture.log]

Point the original client at 127.0.0.1:<listen_port> (AnarchyOnline.exe with
arguments IA<ip as uint> IP<port>), then log in yourself in its window. The
proxy only relays bytes and writes them (hex, timestamped, per direction) to
the log. The log holds the login exchange, so keep it out of the repository
(*.log is ignored) and delete it when you're done.
"""
import socket, sys, threading, time

listen = int(sys.argv[1]) if len(sys.argv) > 1 else 7505
host, _, port = (sys.argv[2] if len(sys.argv) > 2 else "cm.d1.funcom.com:7505").partition(":")
out = open(sys.argv[3] if len(sys.argv) > 3 else "capture.log", "a", buffering=1)
lock = threading.Lock()

def log(tag, data):
    with lock:
        out.write(f"{time.time():.3f} {tag} {len(data)} {data.hex()}\n")

def pump(src, dst, tag):
    try:
        while (d := src.recv(65536)):
            log(tag, d)
            dst.sendall(d)
    except OSError:
        pass
    finally:
        for s in (src, dst):
            try: s.shutdown(socket.SHUT_RDWR)
            except OSError: pass

srv = socket.socket(); srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
srv.bind(("127.0.0.1", listen)); srv.listen(4)
print(f"proxy on 127.0.0.1:{listen} -> {host}:{port}", flush=True)
while True:
    c, _ = srv.accept()
    r = socket.create_connection((host, int(port or 7505)))
    log("OPEN", b"")
    threading.Thread(target=pump, args=(c, r, "C>S"), daemon=True).start()
    threading.Thread(target=pump, args=(r, c, "S>C"), daemon=True).start()
