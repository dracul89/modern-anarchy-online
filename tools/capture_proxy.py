"""Logging TCP proxy for studying the login protocol with the ORIGINAL client.

    python tools/capture_proxy.py [listen_port=7505] [remote=cm.d1.funcom.com:7505] [out=capture-<time>.log]

Point the original client at 127.0.0.1:<listen_port> (AnarchyOnline.exe with
arguments IA16777343 IP<port>), then log in yourself in its window. The proxy
relays bytes and writes them (hex, timestamped, per direction, per connection)
to the log. The log holds the login exchange, so keep it out of the repository
(*.log is ignored) and delete it when you're done.

When the server answers a character login (system message 0x17) with the
address of the game server, the proxy rewrites that address to 127.0.0.1 (the
message keeps its length), listens on the game server's port and relays that
connection too, so the whole session is captured.
"""
import socket, struct, sys, threading, time, zlib

listen = int(sys.argv[1]) if len(sys.argv) > 1 else 7505
host, _, port = (sys.argv[2] if len(sys.argv) > 2 else "cm.d1.funcom.com:7505").partition(":")
out = open(sys.argv[3] if len(sys.argv) > 3 else time.strftime("capture-%H%M%S.log"), "a", buffering=1)
lock = threading.Lock()
conn_ids = iter(range(1, 1000))
listening = set()


def log(tag, cid, data):
    with lock:
        out.write(f"{time.time():.3f} {tag} {len(data)} {data.hex()} c{cid}\n")


def rewrite(data):
    """The game-server hand-off (system message 0x17) with 127.0.0.1 for its
    address. Returns (data, (ip, port) or None)."""
    if len(data) >= 32 and data[6:8] == b"\x00\x2e" and data[16:20] == b"\x00\x00\x00\x17":
        ip = socket.inet_ntoa(data[24:28])
        game_port = struct.unpack(">H", data[28:30])[0]
        return data[:24] + bytes([127, 0, 0, 1]) + data[28:], (ip, game_port)
    return data, None


class GameRewriter:
    """The server's side of a game connection: after the control packet that
    switches compression on, the frames are inflated, a system message 0x3c
    (go to another game server: type, an IPv4 address, a port) has its
    address replaced by 127.0.0.1 (and a proxy started for that server), and
    the frames are deflated again for the client."""

    def __init__(self):
        self.head = b""
        self.dec = None
        self.comp = None
        self.plain = b""

    def feed(self, data):
        if self.dec is None:
            self.head += data
            if len(self.head) < 16:
                return b"", None
            data, self.head = self.head, b""
            if data[:2] == b"\xdf\xdf" and data[8] | data[9] << 8:
                self.dec = zlib.decompressobj()
                self.comp = zlib.compressobj()
                return data[:16] + self.feed(data[16:])[0], self.target
            self.dec = False
        if self.dec is False:
            return data, None
        self.plain += self.dec.decompress(data)
        out = b""
        self.target = None
        while len(self.plain) >= 16:
            size = struct.unpack(">H", self.plain[6:8])[0]
            if size < 16 or len(self.plain) < size:
                break
            frame, self.plain = self.plain[:size], self.plain[size:]
            if struct.unpack(">H", frame[2:4])[0] == 1 and frame[16:20] == b"\x00\x00\x00\x3c" and size >= 26:
                ip = socket.inet_ntoa(frame[20:24])
                port = struct.unpack(">H", frame[24:26])[0]
                self.target = (ip, port)
                frame = frame[:20] + bytes([127, 0, 0, 1]) + frame[24:]
            out += frame
        return (self.comp.compress(out) + self.comp.flush(zlib.Z_SYNC_FLUSH) if out else b""), self.target

    target = None


def pump(src, dst, tag, cid, intercept, game=False):
    rewriter = GameRewriter() if game and tag == "S>C" else None
    try:
        while d := src.recv(65536):
            log(tag, cid, d)
            if intercept:
                d, target = rewrite(d)
                if target:
                    print(f"c{cid}: game server {target[0]}:{target[1]}", flush=True)
                    serve(target[1], target[0], target[1])
            elif rewriter:
                d, target = rewriter.feed(d)
                if target:
                    print(f"c{cid}: another game server {target[0]}:{target[1]}", flush=True)
                    serve(target[1], target[0], target[1])
                if not d:
                    continue
            dst.sendall(d)
    except OSError:
        pass
    finally:
        for s in (src, dst):
            try:
                s.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass


def serve(local_port, remote_host, remote_port):
    if local_port in listening:
        return
    listening.add(local_port)
    srv = socket.socket()
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind(("127.0.0.1", local_port))
    srv.listen(4)
    print(f"proxy on 127.0.0.1:{local_port} -> {remote_host}:{remote_port}", flush=True)

    def accept():
        while True:
            c, _ = srv.accept()
            cid = next(conn_ids)
            r = socket.create_connection((remote_host, remote_port))
            log("OPEN", cid, b"")
            threading.Thread(target=pump, args=(c, r, "C>S", cid, False), daemon=True).start()
            threading.Thread(target=pump, args=(r, c, "S>C", cid, local_port == listen, local_port != listen), daemon=True).start()

    threading.Thread(target=accept, daemon=True).start()


serve(listen, host, int(port or 7505))
while True:
    time.sleep(3600)
