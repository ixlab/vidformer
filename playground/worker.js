// Pyodide Web Worker for Vidformer Playground
// Runs Python code in a separate thread so UI stays responsive

importScripts('https://cdn.jsdelivr.net/pyodide/v0.26.4/full/pyodide.js');

let pyodide = null;
let interruptBuffer = null;

function send(type, data) {
    self.postMessage({ type, ...data });
}

async function initPyodide(buffer) {
    send('log', { message: 'Loading Pyodide...', level: 'info' });
    pyodide = await loadPyodide();

    // Set up interrupt buffer if provided (allows cancellation without terminating worker)
    if (buffer) {
        interruptBuffer = buffer;
        pyodide.setInterruptBuffer(interruptBuffer);
        send('log', { message: 'Interrupt support enabled', level: 'info' });
    }

    send('log', { message: 'Loading core packages...', level: 'info' });
    await pyodide.loadPackage(['micropip', 'numpy']);

    send('log', { message: 'Installing vidformer...', level: 'info' });
    const micropip = pyodide.pyimport('micropip');
    await micropip.install('vidformer');

    send('log', { message: 'Packages installed!', level: 'success' });

    const setupCode = `
import sys
import io
import math
import json
import re
import pickle
import urllib.request

import vidformer as vf
import vidformer.cv2 as cv2

def _on_writer_init(writer):
    spec = writer.spec()
    if spec and hasattr(spec, '_vod_endpoint'):
        video_url = spec._vod_endpoint + "playlist.m3u8"
        status_url = spec._vod_endpoint + "status"
        from js import _notifyVideoReady
        _notifyVideoReady(video_url, status_url)

server = vf.Server(
    "https://api.vidformer.org",
    api_key="VF_GUEST",
    vod_only=True,
    cv2_writer_init_callback=_on_writer_init
)
cv2.set_server(server)

_PROVIDED_MODULES = {"cv2", "vidformer", "vf", "math", "json", "re", "pickle", "urllib", "requests"}

def _filter_imports(code):
    lines = code.strip().split("\\n")
    filtered = []
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("import "):
            modules = [m.strip().split(".")[0].split(" ")[0] for m in stripped[7:].split(",")]
            remaining = [m.strip() for m in stripped[7:].split(",")
                        if m.strip().split(".")[0].split(" ")[0] not in _PROVIDED_MODULES]
            if not remaining:
                continue
            elif len(remaining) < len(modules):
                filtered.append(line[:len(line) - len(stripped)] + "import " + ", ".join(remaining))
                continue
        elif stripped.startswith("from "):
            parts = stripped[5:].split()
            if parts:
                module = parts[0].split(".")[0]
                if module in _PROVIDED_MODULES:
                    continue
        filtered.append(line)
    return "\\n".join(filtered)

print("Vidformer environment ready!")
print(f"Connected to: https://api.vidformer.org")
`;

    self._notifyVideoReady = function(hlsUrl, statusUrl) {
        send('videoReady', { hlsUrl, statusUrl });
    };

    await pyodide.runPythonAsync(setupCode);
    send('log', { message: 'Vidformer environment ready!', level: 'success' });
    send('ready', {});
}

async function runCode(code) {
    if (!pyodide) {
        send('error', { message: 'Pyodide not initialized' });
        return;
    }

    try {
        const filteredCode = await pyodide.runPythonAsync(`_filter_imports(${JSON.stringify(code)})`);

        await pyodide.runPythonAsync(`
import sys
import io
_stdout_capture = io.StringIO()
_stderr_capture = io.StringIO()
_old_stdout = sys.stdout
_old_stderr = sys.stderr
sys.stdout = _stdout_capture
sys.stderr = _stderr_capture
`);

        await pyodide.runPythonAsync(filteredCode);

        const output = await pyodide.runPythonAsync(`
sys.stdout = _old_stdout
sys.stderr = _old_stderr
_stdout_capture.getvalue() + _stderr_capture.getvalue()
`);

        if (output) {
            send('stdout', { output: output });
        }
        send('done', {});

    } catch (error) {
        // Check if this was a KeyboardInterrupt (cancellation)
        const isInterrupt = error && (
            (error.type && error.type.name === 'KeyboardInterrupt') ||
            (error.message && error.message.includes('KeyboardInterrupt'))
        );

        // Try to restore stdout/stderr
        try {
            await pyodide.runPythonAsync(`
sys.stdout = _old_stdout
sys.stderr = _old_stderr
`);
        } catch (e) {}

        if (isInterrupt) {
            // Clean cancellation - report as cancelled, not error
            send('cancelled', {});
            return;
        }

        // Try to get any captured stderr before restoring
        let stderrOutput = '';
        try {
            stderrOutput = await pyodide.runPythonAsync(`_stderr_capture.getvalue()`);
        } catch (e) {}

        // Get the full error message from Pyodide PythonError
        let errorMsg = 'Unknown error';
        if (error) {
            // For Pyodide PythonError, try to format the traceback
            if (error.type && error.type.name) {
                // It's a Python exception
                errorMsg = error.type.name;
                if (error.message) {
                    errorMsg += ': ' + error.message;
                }
            } else if (error.message) {
                errorMsg = error.message;
            } else if (typeof error.toString === 'function') {
                const str = error.toString();
                if (str && str !== '[object Object]') {
                    errorMsg = str;
                }
            }

            // Try to get the Python traceback if available
            try {
                const tb = await pyodide.runPythonAsync(`
import traceback
import sys
if sys.last_traceback:
    ''.join(traceback.format_exception(sys.last_type, sys.last_value, sys.last_traceback))
else:
    ''
`);
                if (tb) {
                    errorMsg = tb;
                }
            } catch (e) {}

            console.error('Full error:', error);
        }

        // Include stderr if there was any
        if (stderrOutput) {
            errorMsg = stderrOutput + '\n' + errorMsg;
        }

        send('error', { message: errorMsg });
    }
}

self.onmessage = async function(e) {
    const { type, code, buffer } = e.data;
    if (type === 'init') {
        try {
            await initPyodide(buffer);
        } catch (error) {
            send('error', { message: 'Failed to initialize: ' + error.message });
        }
    } else if (type === 'run') {
        // Reset interrupt buffer before running
        if (interruptBuffer) {
            interruptBuffer[0] = 0;
        }
        await runCode(code);
    }
};
