"""A fake Anarchy Online server that replays a login captured with
capture_proxy.py, so a client can be tested (and debugged) on the same world
load again and again without the real server or a real account.

    python tools/fake_server.py <capture.log> <chat connection> <game connection>
    python tools/fake_server.py capture-180803.log c16 c17

The chat connection is the log-in (messages 0x22/0x24/0x25/0x0e/0x16/0x17);
the game connection is the one that carries the world. Start the client with
    AnarchyOnline.exe IA16777343 IP7505 DU127.0.0.1:7505
type ANY name and password in its login window (the fake server accepts
them), pick the character and enter the world: the client then receives the
captured world load. The log holds the original account's character list, so
keep it out of the repository (*.log is ignored).
"""
import os, socket, struct, sys, threading, time

log, chat_id, game_id = sys.argv[1], sys.argv[2], sys.argv[3]
CHAT_PORT = int(os.environ.get("FAKE_CHAT_PORT", 7505))
GAME_PORT = int(os.environ.get("FAKE_GAME_PORT", 7506))

server_chat, server_game, client_game = [], [], b""
for line in open(log):
    p = line.split()
    if len(p) > 4 and p[1] == "S>C" and p[-1] == chat_id:
        server_chat.append(bytes.fromhex(p[3]))
    if len(p) > 4 and p[1] == "S>C" and p[-1] == game_id:
        server_game.append((float(p[0]), bytes.fromhex(p[3])))

# The chat connection's server frames, one per client request, in order.
chat_stream = b"".join(server_chat)
frames, i = [], 0
while i + 16 <= len(chat_stream):
    size = struct.unpack(">H", chat_stream[i + 6:i + 8])[0]
    padded = (size + 3) & ~3
    frames.append(bytearray(chat_stream[i:i + padded]))
    i += padded
print(f"{len(frames)} chat frames, {len(server_game)} game chunks", flush=True)
# The last chat frame is the ticket (0x17): point it at this machine.
ticket = frames[-1]
assert ticket[16:20] == b"\x00\x00\x00\x17", "the last chat frame is not a ticket"
ticket[24:28] = bytes([127, 0, 0, 1])
ticket[28:30] = struct.pack(">H", GAME_PORT)


def read_frame(sock):
    head = b""
    while len(head) < 16:
        d = sock.recv(16 - len(head))
        if not d:
            return None
        head += d
    size = struct.unpack(">H", head[6:8])[0]
    rest = ((size + 3) & ~3) - 16
    body = b""
    while len(body) < rest:
        d = sock.recv(rest - len(body))
        if not d:
            return None
        body += d
    return head + body


def chat(sock):
    sent = 0
    while sent < len(frames) and read_frame(sock):
        sock.sendall(bytes(frames[sent]))
        print(f"chat: sent frame {sent + 1}/{len(frames)}", flush=True)
        sent += 1
    time.sleep(1)
    sock.close()


def game(sock):
    first = read_frame(sock)
    print("game: client says hello" if first else "game: client left", flush=True)
    if not first:
        return
    for _, chunk in server_game:
        sock.sendall(chunk)
    print("game: the captured world load sent; holding the connection open", flush=True)
    try:
        while sock.recv(65536):
            pass
    except OSError:
        pass


def serve(port, handler):
    srv = socket.socket()
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind(("127.0.0.1", port))
    srv.listen(4)
    print(f"listening on 127.0.0.1:{port}", flush=True)
    while True:
        c, _ = srv.accept()
        threading.Thread(target=handler, args=(c,), daemon=True).start()


threading.Thread(target=serve, args=(GAME_PORT, game), daemon=True).start()
serve(CHAT_PORT, chat)
