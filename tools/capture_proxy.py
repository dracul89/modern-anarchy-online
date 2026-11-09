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
connection too, so the whole session is captured. The game servers' hand-offs
to other game servers (0x3c) and to the chat server (0x43) are rewritten the
same way, inside the compressed game stream.

The log's lines are `<unix time> <tag> <bytes> <hex> c<connection>`: tags
`OPEN` and `CLOSE` (no bytes) bracket a connection, `C>S` and `S>C` are what
each recv returned. What the proxy does with the hand-offs, and anything that
goes wrong (a server that cannot be reached, a port that is taken), goes to
stdout; capture_session.ps1 keeps that in the capture folder too.
"""
import socket, struct, sys, threading, time, traceback, zlib

listen = int(sys.argv[1]) if len(sys.argv) > 1 else 7505
host, _, port = (sys.argv[2] if len(sys.argv) > 2 else "cm.d1.funcom.com:7505").partition(":")
out = open(sys.argv[3] if len(sys.argv) > 3 else time.strftime("capture-%H%M%S.log"), "a", buffering=1)
lock = threading.Lock()
conn_ids = iter(range(1, 1000))
listening = set()
closed = set()
ports = {}


def log(tag, cid, data):
    with lock:
        out.write(f"{time.time():.3f} {tag} {len(data)} {data.hex()} c{cid}\n")


def say(text):
    print(f"{time.strftime('%H:%M:%S')} {text}", flush=True)


def local_port(ip, port, chat=False):
    """The loopback port that relays to (ip, port): the same number when it is free, else 1000 higher
    (a game server can use the login port: sending the client there would reach the login server;
    another program on this machine may hold the port too)."""
    key = (ip, port)
    if key not in ports:
        local = port
        while local in listening or local == listen or not serve(local, ip, port, chat=chat):
            local += 1000
            if local > 65535:
                raise OSError(f"no free loopback port for {ip}:{port}")
        ports[key] = local
    return ports[key]


class LoginRewriter:
    """The server's side of the login connection. Its frames (two-byte size at offset 6, padded to
    four bytes like the plain game frames) are parsed one by one from wherever a frame starts in
    what recv returned, so the game-server hand-off (system message 0x17: four bytes, the IPv4
    address and the port after the 16-byte header) is rewritten to 127.0.0.1 and the port of a
    proxy to that server whether it comes alone, glued to the message before it, or split over
    two reads. Nothing is held back except a hand-off that is not whole yet (and the first few
    bytes of a frame, until its kind is known); a stream that does not look like frames is passed
    through unchanged from then on, so a surprise can never stall the login."""

    def __init__(self):
        self.buf = b""
        self.skip = 0       # the rest of a frame already forwarded unparsed
        self.raw = False    # not frames: pass everything through
        self.target = None  # the (ip, port) rewritten by the last feed

    def feed(self, data):
        self.target = None
        if self.raw:
            return data
        self.buf += data
        out = b""
        while self.buf:
            if self.skip:
                n = min(self.skip, len(self.buf))
                out += self.buf[:n]
                self.buf = self.buf[n:]
                self.skip -= n
                continue
            if len(self.buf) < 20:
                break  # too little to tell what this frame is: wait for the rest
            size = struct.unpack(">H", self.buf[6:8])[0]
            if size < 16:
                self.raw = True
                out += self.buf
                self.buf = b""
                break
            take = (size + 3) & ~3
            handoff = size >= 30 and self.buf[16:20] == b"\x00\x00\x00\x17"
            if len(self.buf) < take:
                if handoff:
                    break  # hold it until it is whole: it has to be rewritten
                out += self.buf
                self.skip = take - len(self.buf)
                self.buf = b""
                break
            frame, self.buf = self.buf[:take], self.buf[take:]
            if handoff:
                ip = socket.inet_ntoa(frame[24:28])
                game_port = struct.unpack(">H", frame[28:30])[0]
                self.target = (ip, game_port)
                frame = frame[:24] + bytes([127, 0, 0, 1]) + struct.pack(">H", local_port(ip, game_port)) + frame[30:]
            out += frame
        return out

    def flush(self):
        """What is still held back when the connection ends."""
        rest, self.buf = self.buf, b""
        return rest


class GameRewriter:
    """The server's side of a game connection: after the control packet that
    switches compression on, the frames are inflated, a system message 0x3c
    (go to another game server: type, an IPv4 address, a port) or 0x43 (the chat server) has its
    address replaced by 127.0.0.1 (and a proxy started for that server), and
    the frames are deflated again for the client."""

    def __init__(self):
        self.head = b""
        self.dec = None
        self.comp = None
        self.plain = b""
        self.target = None
        self.chat = False

    def feed(self, data):
        if self.dec is None:
            self.head += data
            if len(self.head) < 16:
                return b""
            data, self.head = self.head, b""
            if data[:2] == b"\xdf\xdf" and data[8] | data[9] << 8:
                self.dec = zlib.decompressobj()
                self.comp = zlib.compressobj()
                return data[:16] + self.feed(data[16:])
            self.dec = False
        # Plain frames (before compression starts, or when it never does) are parsed too; they are
        # padded to a multiple of four bytes, compressed ones are not.
        plain_mode = self.dec is False
        self.plain += data if plain_mode else self.dec.decompress(data)
        out = b""
        self.target = None
        self.chat = False
        while len(self.plain) >= 16:
            size = struct.unpack(">H", self.plain[6:8])[0]
            take = (size + 3) & ~3 if plain_mode else size
            if size < 16 or len(self.plain) < take:
                break
            frame, self.plain = self.plain[:take], self.plain[take:]
            if struct.unpack(">H", frame[2:4])[0] == 1 and frame[16:20] == b"\x00\x00\x00<" and size >= 26:
                ip = socket.inet_ntoa(frame[20:24])
                port = struct.unpack(">H", frame[24:26])[0]
                self.target = (ip, port)
                frame = frame[:20] + bytes([127, 0, 0, 1]) + struct.pack(">H", local_port(ip, port)) + frame[26:]
            elif struct.unpack(">H", frame[2:4])[0] == 1 and frame[16:20] == b"\x00\x00\x00C" and size >= 32:
                # ChatServerInfo: the chat server the client opens a second connection to: (u32 1,
                # a length-prefixed address string, a u32 port). The address becomes 127.0.0.1 padded
                # with NULs to the same length (the message keeps its size) and a proxy is started
                # for that server, so the chat traffic is captured too.
                n = struct.unpack(">I", frame[24:28])[0]
                if 9 <= n <= 40 and 28 + n + 4 <= size:
                    ip = frame[28:28 + n].rstrip(b"\x00").decode("ascii", "replace")
                    port = struct.unpack(">I", frame[28 + n:32 + n])[0]
                    self.target = (ip, port)
                    self.chat = True
                    frame = frame[:28] + b"127.0.0.1".ljust(n, b"\x00") + struct.pack(">I", local_port(ip, port, chat=True)) + frame[32 + n:]
            out += frame
        if plain_mode:
            return out
        return self.comp.compress(out) + self.comp.flush(zlib.Z_SYNC_FLUSH) if out else b""


def pump(src, dst, tag, cid, rewriter=None):
    """Relays src to dst, logging every read; `rewriter` (the server's side only) edits the hand-offs."""
    try:
        while d := src.recv(65536):
            log(tag, cid, d)
            if rewriter:
                try:
                    d = rewriter.feed(d)
                except Exception:
                    say(f"c{cid}: the rewriter failed, the connection is dropped\n{traceback.format_exc()}")
                    return
                if rewriter.target:
                    kind = "chat" if getattr(rewriter, "chat", False) else "game"
                    say(f"c{cid}: {kind} server {rewriter.target[0]}:{rewriter.target[1]} -> local {ports[rewriter.target]}")
                if not d:
                    continue
            dst.sendall(d)
        if rewriter and hasattr(rewriter, "flush"):
            rest = rewriter.flush()
            if rest:
                dst.sendall(rest)
    except OSError as e:
        # A reset from the other pump's shutdown is the normal way a closed connection ends here.
        if not (isinstance(e, ConnectionError) and cid in closed):
            say(f"c{cid} {tag}: {e}")
    finally:
        with lock:
            first = cid not in closed
            closed.add(cid)
        if first:
            log("CLOSE", cid, b"")
            say(f"c{cid}: closed ({tag} ended first)")
        for s in (src, dst):
            try:
                s.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass


def serve(local_port, remote_host, remote_port, chat=False):
    """Listens on 127.0.0.1:local_port and relays each connection to remote_host:remote_port.
    Returns False when the port cannot be bound (another program has it)."""
    if local_port in listening:
        return True
    srv = socket.socket()
    if hasattr(socket, "SO_EXCLUSIVEADDRUSE"):
        # On Windows SO_REUSEADDR would bind even while another socket listens on the port, and then
        # which one gets the connections is anyone's guess: fail instead, and pick another port.
        srv.setsockopt(socket.SOL_SOCKET, socket.SO_EXCLUSIVEADDRUSE, 1)
    else:
        srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    try:
        srv.bind(("127.0.0.1", local_port))
        srv.listen(4)
    except OSError as e:
        say(f"port {local_port} cannot be used ({e})")
        srv.close()
        return False
    listening.add(local_port)
    say(f"proxy on 127.0.0.1:{local_port} -> {remote_host}:{remote_port}")

    def accept():
        while True:
            try:
                c, _ = srv.accept()
            except OSError as e:
                say(f"port {local_port}: accept failed ({e}); still listening")
                time.sleep(1)
                continue
            cid = next(conn_ids)
            try:
                r = socket.create_connection((remote_host, remote_port), timeout=20)
                r.settimeout(None)
            except OSError as e:
                # The client's connection is closed, so it reports the failure itself instead of
                # hanging; the listener stays up for the next attempt.
                say(f"c{cid}: cannot reach {remote_host}:{remote_port} ({e}); the client's connection is closed")
                c.close()
                continue
            log("OPEN", cid, b"")
            say(f"c{cid}: open on {local_port} -> {remote_host}:{remote_port}")
            s2c = LoginRewriter() if local_port == listen else (None if chat else GameRewriter())
            threading.Thread(target=pump, args=(c, r, "C>S", cid), daemon=True).start()
            threading.Thread(target=pump, args=(r, c, "S>C", cid, s2c), daemon=True).start()

    threading.Thread(target=accept, daemon=True).start()
    return True


if __name__ == "__main__":
    if not serve(listen, host, int(port or 7505)):
        say(f"cannot listen on 127.0.0.1:{listen}: is an old proxy still running?")
        sys.exit(1)
    while True:
        time.sleep(3600)
