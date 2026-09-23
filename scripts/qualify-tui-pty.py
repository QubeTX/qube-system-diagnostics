"""Real terminal interaction qualification; writes assertions/timings, no screen content.

Development dependencies: pyte==0.8.2, psutil==7.2.2 and (Windows) pywinpty==3.0.5.
No companion execution, installation, speed test, repair or privileged read occurs.
This is an interaction observer; do not run during CPU/memory qualification.
"""
import argparse
import codecs
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import signal
import tempfile
import threading
import time

import psutil
import pyte


class InteractiveScreen(pyte.Screen):
    def __init__(self, reply):
        super().__init__(80, 24)
        self.reply = reply

    def write_process_input(self, data):
        # A real terminal answers device-status/cursor queries. Ignoring them
        # manufactures startup timeouts (and can break Unix terminal setup).
        self.reply(data)


class Terminal:
    def __init__(self, binary, env):
        self.lock = threading.Lock()
        self.input_lock = threading.Lock()
        self.screen = InteractiveScreen(self.write)
        self.stream = pyte.Stream(self.screen)
        self.tail = ""
        self.reader_error = None
        self.closed = False
        self.job = None
        self.reaped = False
        self.characters_received = 0
        self.last_output_at = None
        self.actions = []
        if os.name == "nt":
            from winpty import Backend, PtyProcess
            spec = importlib.util.spec_from_file_location("sd300_measure", Path(__file__).with_name("measure-tui-windows.py"))
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            self.job = module.Job()
            self.process = PtyProcess.spawn([str(Path(os.environ["SystemRoot"]) / "System32/cmd.exe"), "/d", "/q"],
                dimensions=(24,80), env=env, backend=Backend.ConPTY)
            self.pid = self.process.pid
            self.job.assign(self.pid)
            self.launched_at = time.perf_counter()
            self.process.write(f'"{binary}"\r\n')
        else:
            import fcntl
            import pty
            import struct
            import termios
            self.launched_at = time.perf_counter()
            self.pid, self.fd = pty.fork()
            if self.pid == 0:
                fcntl.ioctl(1, termios.TIOCSWINSZ, struct.pack("HHHH",24,80,0,0))
                os.execve(binary, [str(binary)], env)
            self.decoder = codecs.getincrementaldecoder("utf-8")("replace")
        self.reader = threading.Thread(target=self.read, daemon=True)
        self.reader.start()

    def read(self):
        try:
            while not self.closed:
                if os.name == "nt":
                    chunk = self.process.read(4096)
                else:
                    chunk = self.decoder.decode(os.read(self.fd, 4096))
                if not chunk:
                    break
                with self.lock:
                    self.characters_received += len(chunk)
                    self.last_output_at = time.monotonic()
                    self.tail = (self.tail + chunk)[-16384:]
                    self.stream.feed(chunk)
        except (EOFError, OSError):
            pass  # PTYs signal their ordinary end with EOF or EIO.
        except Exception as error:
            self.reader_error = type(error).__name__

    def write(self, text):
        with self.input_lock:
            if os.name == "nt":
                self.process.write(text)
            else:
                os.write(self.fd, text.encode("utf-8"))

    def lines(self):
        with self.lock:
            return list(self.screen.display)

    def wait(self, predicate, description, timeout=5):
        start = time.perf_counter()
        while time.perf_counter() - start < timeout:
            if self.reader_error:
                raise RuntimeError("Terminal decoder failed: " + self.reader_error)
            if predicate(self.lines()):
                return (time.perf_counter() - start) * 1000
            time.sleep(.002)
        raise AssertionError("Timed out: " + description)

    def diagnostics(self):
        # Only fixed application labels and structural state may leave this
        # observer. Never publish arbitrary process/host/connection text.
        labels = ["Select a diagnostic mode", "User Mode", "Tech Mode", "Overview",
            "CPU", "Mem", "Disk", "GPU", "Net", "Procs", "Thermals", "Drivers",
            "Inspector", "Keybindings", "filter", "PAUSED at", "too small",
            "Network companion", "Export this session", "Sort: Memory descending"]
        with self.lock:
            lines = list(self.screen.display)
            state = {"dimensions":[self.screen.columns,self.screen.lines],
                "cursor":[self.screen.cursor.x,self.screen.cursor.y],
                "received_characters":self.characters_received,
                "last_output_age_ms":None if self.last_output_at is None else (time.monotonic()-self.last_output_at)*1000,
                "label_rows":{label:[row for row,line in enumerate(lines) if label in line] for label in labels},
                "restoration_in_tail":"\x1b[?1049l" in self.tail,
                "reader_error":self.reader_error,"reader_alive":self.reader.is_alive(),
                "recent_actions":self.actions[-8:]}
        try:
            process = psutil.Process(self.pid)
            state["process_status"] = process.status()
            state["descendant_count"] = len(process.children(recursive=True))
        except psutil.Error as error:
            state["process_query_error"] = type(error).__name__
        return state

    def action(self, keys, predicate, description, timeout=5):
        start = time.perf_counter()
        self.actions.append(description)
        self.actions = self.actions[-8:]
        self.write(keys)
        self.wait(predicate, description, timeout)
        return (time.perf_counter() - start) * 1000

    def resize(self, columns, rows):
        with self.lock:
            self.screen.resize(lines=rows, columns=columns)
        if os.name == "nt":
            self.process.setwinsize(rows, columns)
        else:
            import fcntl
            import struct
            import termios
            fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack("HHHH",rows,columns,0,0))
            os.kill(self.pid, signal.SIGWINCH)

    def close(self):
        self.closed = True
        if self.job:
            self.job.close()
            self.process.close(force=True)
        else:
            try:
                root = psutil.Process(self.pid)
                for child in reversed(root.children(recursive=True)):
                    try:
                        child.kill()
                    except psutil.NoSuchProcess:
                        pass
                root.kill()
            except psutil.NoSuchProcess:
                pass
            if not self.reaped:
                try:
                    os.waitpid(self.pid, 0)
                except ChildProcessError:
                    pass
            os.close(self.fd)
        self.reader.join(timeout=2)


def contains(text):
    return lambda lines: text in "\n".join(lines)


def main():
    if os.name != "nt":
        def interrupted(_signal, _frame):
            raise KeyboardInterrupt("Qualification interrupted")
        signal.signal(signal.SIGTERM, interrupted)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--ascii", action="store_true")
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    if any(char in str(binary) for char in '\"%&|<>^\r\n'):
        parser.error("Executable path contains command-shell metacharacters")
    with tempfile.TemporaryDirectory(prefix="sd300-pty-") as directory:
        root = Path(directory)
        env = dict(os.environ, HOME=directory, APPDATA=directory, LOCALAPPDATA=directory,
            XDG_CONFIG_HOME=directory, TERM="xterm-256color")
        if args.ascii:
            env.update(NO_COLOR="1", SD300_ASCII="1")
        config = root / ("Library/Application Support" if os.sys.platform == "darwin" else "") / ("sd300" if os.sys.platform.startswith("linux") else "SD-300")
        config.mkdir(parents=True, exist_ok=True)
        (config/"settings.json").write_text(json.dumps({"schema_version":1,"tui":{"mouse_enabled":True,"reduced_motion":True}}), encoding="utf-8")
        start = time.perf_counter()
        terminal = Terminal(binary, env)
        results = []
        try:
            terminal.wait(contains("Select a diagnostic mode"), "immediate chooser", 10)
            startup_ms = (time.perf_counter()-terminal.launched_at)*1000
            for mode, key in [("User Mode","1"),("Tech Mode","2")]:
                if mode == "Tech Mode":
                    terminal.action("m", contains("Select a diagnostic mode"), "mode chooser")
                terminal.action(key, contains(mode), mode)
                terminal.wait(contains("filter"), "progressive first sample", 20)
                for columns, rows in [(80,24),(140,40)]:
                    terminal.resize(columns,rows)
                    terminal.wait(lambda lines: len(lines)==rows and len(lines[0])==columns, "resize")
                    for number, label in enumerate(["Overview","CPU","Mem","Disk","GPU","Net","Procs","Thermals","Drivers"],1):
                        latency = terminal.action(str(number), lambda lines: label in lines[2], f"{mode} {columns}x{rows} section {label}")
                        if columns == 80:
                            terminal.action("\r", contains("Inspector"), "inspect " + label)
                            terminal.action("\r", lambda lines: "Inspector" not in "\n".join(lines), "close inspector")
                        results.append({"mode":mode,"size":[columns,rows],"section":label,"input_ms":latency})
                    terminal.action("7", lambda lines: "Procs" in lines[2], "processes")
                    terminal.action("M", contains("Sort: Memory descending"), "sort direction")
                    terminal.action("s", contains("Sort: Memory reversed"), "reverse direction")
                    terminal.action("/no-such-process-9471\r", contains("No matching rows"), "filter full inventory")
                    terminal.action("\x1b", lambda lines: "No matching rows" not in "\n".join(lines), "clear filter")
                    if not args.ascii:
                        terminal.action("/監視🛰\r", contains("No matching rows"), "Unicode filter")
                        terminal.action("\x1b", lambda lines: "No matching rows" not in "\n".join(lines), "clear Unicode filter")
                    terminal.write("j"*30 + "k"*30 + "\x1b[6~\x1b[5~")
                    terminal.action(" ", contains("PAUSED at"), "freeze presentation")
                    frozen = terminal.lines()[:2]
                    time.sleep(1.2)
                    assert frozen == terminal.lines()[:2], "Frozen capture changed while collection continued"
                    terminal.action(" ", lambda lines: "PAUSED at" not in "\n".join(lines), "resume latest")
            terminal.action("N", contains("Network companion"), "companion actions")
            terminal.action("b", contains("M-Lab consent: OFF"), "bandwidth consent defaults off")
            terminal.action("\x1b", lambda lines: "Network companion" not in "\n".join(lines), "decline speed test")
            terminal.action("E", contains("Export this session"), "export explanation")
            terminal.action("\x1b", lambda lines: "Export this session" not in "\n".join(lines), "close export without writing")
            latencies = []
            for _ in range(30):
                latencies.append(terminal.action("?", contains("Keybindings"), "help input"))
                latencies.append(terminal.action("?", lambda lines: "Keybindings" not in "\n".join(lines), "close help"))
            terminal.resize(60,18)
            terminal.wait(contains("too small"), "small terminal guidance")
            terminal.resize(80,24)
            terminal.wait(contains("filter"), "restored size")
            terminal.action("\x1b[<0;12;24M\x1b[<0;12;24m", lambda lines: "CPU" in lines[2], "optional mouse navigation")
            if args.ascii:
                assert all(line.isascii() for line in terminal.lines()), "ASCII fallback emitted non-ASCII cells"
                with terminal.lock:
                    assert all(cell.fg == "default" and cell.bg == "default" for row in terminal.screen.buffer.values() for cell in row.values()), "NO_COLOR retained a color"
            terminal.write("q")
            deadline = time.monotonic()+15
            while time.monotonic()<deadline:
                with terminal.lock:
                    restored = "\x1b[?1049l" in terminal.tail
                if restored:
                    break
                time.sleep(.01)
            assert restored, "Alternate screen was not restored after quit"
            if os.name != "nt":
                while time.monotonic() < deadline:
                    pid, status = os.waitpid(terminal.pid, os.WNOHANG)
                    if pid:
                        terminal.reaped = True
                        assert os.waitstatus_to_exitcode(status) == 0, "Monitor exit was not successful"
                        break
                    time.sleep(.01)
                assert terminal.reaped, "Monitor did not finish bounded worker shutdown"
            latencies.sort()
            report = {"schema":1,"sha256":hashlib.sha256(binary.read_bytes()).hexdigest(),"platform":os.sys.platform,
                "startup_chooser_ms":startup_ms,"ascii":args.ascii,"sections":results,"input_samples":len(latencies),
                "terminal_setup_ms":(terminal.launched_at-start)*1000,
                "input_p95_ms":latencies[int(.95*(len(latencies)-1))],"input_max_ms":max(latencies),"terminal_restored":restored,
                "input_method":"Key write to decoded terminal state, includes PTY transport and 2 ms observer polling",
                "no_optional_action_executed":True}
            args.output.parent.mkdir(parents=True,exist_ok=True)
            args.output.write_text(json.dumps(report,indent=2)+"\n",encoding="utf-8")
            print(json.dumps(report,indent=2))
        except Exception as error:
            failure = {"schema":1,"passed":False,"platform":os.sys.platform,"ascii":args.ascii,
                "sha256":hashlib.sha256(binary.read_bytes()).hexdigest(),
                "failure_type":type(error).__name__,"failure":str(error),
                "completed_sections":results,"terminal":terminal.diagnostics()}
            args.output.parent.mkdir(parents=True,exist_ok=True)
            args.output.write_text(json.dumps(failure,indent=2)+"\n",encoding="utf-8")
            print(json.dumps(failure),file=os.sys.stderr)
            raise
        finally:
            terminal.close()


if __name__ == "__main__":
    main()
